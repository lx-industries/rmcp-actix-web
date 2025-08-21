//! Server-Sent Events (SSE) transport implementation for MCP.
//!
//! This module provides a unidirectional transport using the SSE protocol,
//! allowing servers to push real-time updates to clients over a standard HTTP connection.
//!
//! ## Architecture
//!
//! The SSE transport consists of two HTTP endpoints:
//! - **SSE endpoint** (`/sse` by default): Clients connect here to receive server-sent events
//! - **POST endpoint** (`/message` by default): Clients send JSON-RPC messages here
//!
//! ## Connection Flow
//!
//! 1. Client connects to the SSE endpoint with a session ID
//! 2. Server establishes an event stream for real-time messages
//! 3. Client sends requests to the POST endpoint with the same session ID
//! 4. Server processes requests and sends responses via the SSE stream
//!
//! ## Features
//!
//! - Automatic keep-alive pings to maintain connections
//! - Session management for multiple concurrent clients
//! - Graceful shutdown via cancellation tokens
//! - Compatible with proxies and firewalls
//!
//! ## Example
//!
//! ```rust,no_run
//! use rmcp_actix_web::{SseServer, SseServerConfig};
//! use tokio_util::sync::CancellationToken;
//!
//! # struct MyService;
//! # use rmcp::{ServerHandler, model::ServerInfo};
//! # impl ServerHandler for MyService {
//! #     fn get_info(&self) -> ServerInfo { ServerInfo::default() }
//! # }
//! # impl MyService {
//! #     fn new() -> Self { Self }
//! # }
//! #[actix_web::main]
//! async fn main() -> std::io::Result<()> {
//!     // Simple server with defaults
//!     let server = SseServer::serve("127.0.0.1:8080".parse().unwrap()).await?;
//!     
//!     // Attach service and get cancellation token
//!     let ct = server.with_service(|| MyService::new());
//!     
//!     // Server runs until cancelled
//!     ct.cancelled().await;
//!     Ok(())
//! }
//! ```

use std::{collections::HashMap, io, sync::Arc, time::Duration};

use actix_web::{
    HttpRequest, HttpResponse, Result, Scope,
    error::ErrorInternalServerError,
    http::header::{CACHE_CONTROL, CONTENT_TYPE},
    middleware,
    web::{self, Bytes, Data, Json, Query},
};
use futures::{Sink, SinkExt, Stream, StreamExt};
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::{CancellationToken, PollSender};
use tracing::Instrument;

use rmcp::{
    RoleServer,
    model::ClientJsonRpcMessage,
    service::{RxJsonRpcMessage, TxJsonRpcMessage, serve_directly_with_ct},
    transport::{
        common::server_side_http::{DEFAULT_AUTO_PING_INTERVAL, SessionId, session_id},
        sse_server::SseServerConfig,
    },
};

const HEADER_X_ACCEL_BUFFERING: &str = "X-Accel-Buffering";

type TxStore =
    Arc<tokio::sync::RwLock<HashMap<SessionId, tokio::sync::mpsc::Sender<ClientJsonRpcMessage>>>>;

#[derive(Clone, Debug)]
struct AppData {
    txs: TxStore,
    transport_tx: tokio::sync::mpsc::UnboundedSender<SseServerTransport>,
    post_path: Arc<str>,
    sse_ping_interval: Duration,
}

impl AppData {
    pub fn new(
        post_path: String,
        sse_ping_interval: Duration,
    ) -> (
        Self,
        tokio::sync::mpsc::UnboundedReceiver<SseServerTransport>,
    ) {
        let (transport_tx, transport_rx) = tokio::sync::mpsc::unbounded_channel();
        (
            Self {
                txs: Default::default(),
                transport_tx,
                post_path: post_path.into(),
                sse_ping_interval,
            },
            transport_rx,
        )
    }
}

#[doc(hidden)]
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostEventQuery {
    /// The session ID from the query string
    pub session_id: String,
}

async fn post_event_handler(
    app_data: Data<AppData>,
    query: Query<PostEventQuery>,
    _req: HttpRequest,
    message: Json<ClientJsonRpcMessage>,
) -> Result<HttpResponse> {
    let session_id = &query.session_id;
    tracing::debug!(session_id, ?message, "new client message");

    let tx = {
        let rg = app_data.txs.read().await;
        rg.get(session_id.as_str())
            .ok_or_else(|| actix_web::error::ErrorNotFound("Session not found"))?
            .clone()
    };

    // Note: In actix-web, we don't have direct access to modify extensions
    // This would need a different approach for passing HTTP request context

    if tx.send(message.0).await.is_err() {
        tracing::error!("send message error");
        return Err(actix_web::error::ErrorGone("Session closed"));
    }

    Ok(HttpResponse::Accepted().finish())
}

async fn sse_handler(app_data: Data<AppData>, _req: HttpRequest) -> Result<HttpResponse> {
    let session = session_id();
    tracing::info!(%session, "sse connection");

    let (from_client_tx, from_client_rx) = tokio::sync::mpsc::channel(64);
    let (to_client_tx, to_client_rx) = tokio::sync::mpsc::channel(64);
    let to_client_tx_clone = to_client_tx.clone();

    app_data
        .txs
        .write()
        .await
        .insert(session.clone(), from_client_tx);

    let stream = ReceiverStream::new(from_client_rx);
    let sink = PollSender::new(to_client_tx);
    let transport = SseServerTransport {
        stream,
        sink,
        session_id: session.clone(),
        tx_store: app_data.txs.clone(),
    };

    let transport_send_result = app_data.transport_tx.send(transport);
    if transport_send_result.is_err() {
        tracing::warn!("send transport out error");
        return Err(ErrorInternalServerError(
            "Failed to send transport, server is closed",
        ));
    }

    let post_path = app_data.post_path.clone();
    let ping_interval = app_data.sse_ping_interval;
    let session_for_stream = session.clone();

    // Create SSE response stream
    let sse_stream = async_stream::stream! {
        // Send initial endpoint message
        yield Ok::<_, actix_web::Error>(Bytes::from(format!(
            "event: endpoint\ndata: {post_path}?sessionId={session_for_stream}\n\n"
        )));

        // Set up ping interval
        let mut ping_interval = tokio::time::interval(ping_interval);
        ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let mut rx = ReceiverStream::new(to_client_rx);

        loop {
            tokio::select! {
                Some(message) = rx.next() => {
                    match serde_json::to_string(&message) {
                        Ok(json) => {
                            yield Ok(Bytes::from(format!("event: message\ndata: {json}\n\n")));
                        }
                        Err(e) => {
                            tracing::error!("Failed to serialize message: {}", e);
                        }
                    }
                }
                _ = ping_interval.tick() => {
                    yield Ok(Bytes::from(": ping\n\n"));
                }
                else => break,
            }
        }
    };

    // Clean up on disconnect
    let app_data_clone = app_data.clone();
    let session_for_cleanup = session.clone();
    actix_rt::spawn(async move {
        to_client_tx_clone.closed().await;

        let mut txs = app_data_clone.txs.write().await;
        txs.remove(&session_for_cleanup);
        tracing::debug!(%session_for_cleanup, "Closed session and cleaned up resources");
    });

    Ok(HttpResponse::Ok()
        .insert_header((CONTENT_TYPE, "text/event-stream"))
        .insert_header((CACHE_CONTROL, "no-cache"))
        .insert_header((HEADER_X_ACCEL_BUFFERING, "no"))
        .streaming(sse_stream))
}

/// Transport handle for an individual SSE client connection.
///
/// Implements both `Sink` and `Stream` traits to provide bidirectional communication
/// for a single client session. This is created internally for each client connection
/// and passed to the MCP service.
pub struct SseServerTransport {
    stream: ReceiverStream<RxJsonRpcMessage<RoleServer>>,
    sink: PollSender<TxJsonRpcMessage<RoleServer>>,
    session_id: SessionId,
    tx_store: TxStore,
}

impl Sink<TxJsonRpcMessage<RoleServer>> for SseServerTransport {
    type Error = io::Error;

    fn poll_ready(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.sink
            .poll_ready_unpin(cx)
            .map_err(std::io::Error::other)
    }

    fn start_send(
        mut self: std::pin::Pin<&mut Self>,
        item: TxJsonRpcMessage<RoleServer>,
    ) -> Result<(), Self::Error> {
        self.sink
            .start_send_unpin(item)
            .map_err(std::io::Error::other)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.sink
            .poll_flush_unpin(cx)
            .map_err(std::io::Error::other)
    }

    fn poll_close(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let inner_close_result = self
            .sink
            .poll_close_unpin(cx)
            .map_err(std::io::Error::other);
        if inner_close_result.is_ready() {
            let session_id = self.session_id.clone();
            let tx_store = self.tx_store.clone();
            tokio::spawn(async move {
                tx_store.write().await.remove(&session_id);
            });
        }
        inner_close_result
    }
}

impl Stream for SseServerTransport {
    type Item = RxJsonRpcMessage<RoleServer>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.stream.poll_next_unpin(cx)
    }
}

/// Server-Sent Events transport server for MCP.
///
/// Provides a unidirectional streaming transport from server to client using the SSE protocol.
/// Clients connect to the SSE endpoint to receive events and send requests via a separate POST endpoint.
///
/// # Architecture
///
/// The server manages two endpoints:
/// - SSE endpoint for server-to-client streaming
/// - POST endpoint for client-to-server messages
///
/// Each client connection is identified by a unique session ID that must be provided
/// in both the SSE connection and POST requests.
///
/// # Example
///
/// ```rust,no_run
/// use rmcp_actix_web::SseServer;
/// # use rmcp::{ServerHandler, model::ServerInfo};
/// # struct MyService;
/// # impl ServerHandler for MyService {
/// #     fn get_info(&self) -> ServerInfo { ServerInfo::default() }
/// # }
/// # impl MyService { fn new() -> Self { Self } }
///
/// #[actix_web::main]
/// async fn main() -> std::io::Result<()> {
///     // Start server with default configuration
///     let server = SseServer::serve("127.0.0.1:8080".parse().unwrap()).await?;
///     
///     // Attach MCP service
///     let ct = server.with_service(|| MyService::new());
///     
///     // Run until cancelled
///     ct.cancelled().await;
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct SseServer {
    transport_rx: Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<SseServerTransport>>>,
    /// The configuration used by this server instance.
    pub config: SseServerConfig,
    app_data: Data<AppData>,
}

impl SseServer {
    /// Creates and starts an SSE server with default configuration.
    ///
    /// This is the simplest way to start an SSE server. It uses default paths
    /// (`/sse` for events, `/message` for POST) and creates a new cancellation token.
    ///
    /// # Arguments
    ///
    /// * `bind` - The socket address to bind to
    ///
    /// # Returns
    ///
    /// Returns the server instance on success, or an I/O error if binding fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rmcp_actix_web::SseServer;
    ///
    /// #[actix_web::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let server = SseServer::serve("127.0.0.1:8080".parse().unwrap()).await?;
    ///     // Server is now running
    ///     Ok(())
    /// }
    /// ```
    pub async fn serve(bind: std::net::SocketAddr) -> io::Result<Self> {
        Self::serve_with_config(SseServerConfig {
            bind,
            sse_path: "/sse".to_string(),
            post_path: "/message".to_string(),
            ct: CancellationToken::new(),
            sse_keep_alive: None,
        })
        .await
    }

    /// Creates and starts an SSE server with custom configuration.
    ///
    /// Allows full control over server configuration including paths, bind address,
    /// keep-alive intervals, and cancellation token.
    ///
    /// # Arguments
    ///
    /// * `config` - The server configuration
    ///
    /// # Returns
    ///
    /// Returns the configured server instance on success, or an I/O error if binding fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rmcp_actix_web::{SseServer, SseServerConfig};
    /// use tokio_util::sync::CancellationToken;
    /// use std::time::Duration;
    ///
    /// #[actix_web::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let config = SseServerConfig {
    ///         bind: "127.0.0.1:8080".parse().unwrap(),
    ///         sse_path: "/events".to_string(),
    ///         post_path: "/rpc".to_string(),
    ///         ct: CancellationToken::new(),
    ///         sse_keep_alive: Some(Duration::from_secs(30)),
    ///     };
    ///     
    ///     let server = SseServer::serve_with_config(config).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn serve_with_config(mut config: SseServerConfig) -> io::Result<Self> {
        let bind_addr = config.bind;
        let ct = config.ct.clone();

        // First bind to get the actual address
        let listener = std::net::TcpListener::bind(bind_addr)?;
        let actual_addr = listener.local_addr()?;
        listener.set_nonblocking(true)?;

        // Update config with actual address
        config.bind = actual_addr;
        let (sse_server, _) = Self::new(config);
        let app_data = sse_server.app_data.clone();
        let sse_path = sse_server.config.sse_path.clone();
        let post_path = sse_server.config.post_path.clone();

        let server = actix_web::HttpServer::new(move || {
            actix_web::App::new()
                .app_data(app_data.clone())
                .wrap(middleware::NormalizePath::trim())
                .route(&sse_path, web::get().to(sse_handler))
                .route(&post_path, web::post().to(post_event_handler))
        })
        .listen(listener)?
        .run();

        let ct_child = ct.child_token();
        let server_handle = server.handle();

        actix_rt::spawn(async move {
            ct_child.cancelled().await;
            tracing::info!("sse server cancelled");
            server_handle.stop(true).await;
        });

        actix_rt::spawn(
            async move {
                if let Err(e) = server.await {
                    tracing::error!(error = %e, "sse server shutdown with error");
                }
            }
            .instrument(tracing::info_span!("sse-server", bind_address = %actual_addr)),
        );

        Ok(sse_server)
    }

    /// Creates a new SSE server without starting the HTTP server.
    ///
    /// This method returns both the server instance and an actix-web `Scope` that can be
    /// mounted into an existing actix-web application. This allows integration with
    /// existing web services and follows the RMCP pattern where the framework router
    /// is returned directly for composition.
    ///
    /// # Arguments
    ///
    /// * `config` - The server configuration
    ///
    /// # Returns
    ///
    /// Returns a tuple of:
    /// - The `SseServer` instance
    /// - An actix-web `Scope` configured with the SSE routes
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rmcp_actix_web::{SseServer, SseServerConfig};
    /// use actix_web::{App, HttpServer, web};
    /// use tokio_util::sync::CancellationToken;
    ///
    /// #[actix_web::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let config = SseServerConfig {
    ///         bind: "127.0.0.1:8080".parse().unwrap(),
    ///         sse_path: "/sse".to_string(),
    ///         post_path: "/message".to_string(),
    ///         ct: CancellationToken::new(),
    ///         sse_keep_alive: None,
    ///     };
    ///     let (server, scope) = SseServer::new(config);
    ///     
    ///     // Mount into existing app at a custom path
    ///     let app = App::new()
    ///         .service(web::scope("/api/v1").service(scope));
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub fn new(config: SseServerConfig) -> (SseServer, Scope) {
        let (app_data, transport_rx) = AppData::new(
            config.post_path.clone(),
            config.sse_keep_alive.unwrap_or(DEFAULT_AUTO_PING_INTERVAL),
        );

        let sse_path = config.sse_path.clone();
        let post_path = config.post_path.clone();

        let app_data = Data::new(app_data);

        let scope = web::scope("")
            .app_data(app_data.clone())
            .route(&sse_path, web::get().to(sse_handler))
            .route(&post_path, web::post().to(post_event_handler));

        let server = SseServer {
            transport_rx: Arc::new(Mutex::new(transport_rx)),
            config,
            app_data,
        };

        (server, scope)
    }

    /// Attaches an MCP service to the server and starts processing connections.
    ///
    /// This method spawns a background task that creates a new service instance
    /// for each incoming client connection. The service provider function is called
    /// once per connection to allow for per-connection state.
    ///
    /// # Arguments
    ///
    /// * `service_provider` - A function that creates new service instances
    ///
    /// # Returns
    ///
    /// Returns the server's cancellation token. The server will run until this token is cancelled.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rmcp_actix_web::SseServer;
    /// # use rmcp::{ServerHandler, model::ServerInfo};
    /// # struct MyService;
    /// # impl ServerHandler for MyService {
    /// #     fn get_info(&self) -> ServerInfo { ServerInfo::default() }
    /// # }
    /// # impl MyService { fn new() -> Self { Self } }
    ///
    /// #[actix_web::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let server = SseServer::serve("127.0.0.1:8080".parse().unwrap()).await?;
    ///     
    ///     // Attach service - new instance per connection
    ///     let ct = server.with_service(|| MyService::new());
    ///     
    ///     // Wait for shutdown
    ///     ct.cancelled().await;
    ///     Ok(())
    /// }
    /// ```
    pub fn with_service<S, F>(self, service_provider: F) -> CancellationToken
    where
        S: rmcp::ServerHandler,
        F: Fn() -> S + Send + 'static,
    {
        use rmcp::service::ServiceExt;
        let ct = self.config.ct.clone();
        let transport_rx = self.transport_rx.clone();

        actix_rt::spawn(async move {
            while let Some(transport) = transport_rx.lock().await.recv().await {
                let service = service_provider();
                let ct_child = ct.child_token();
                tokio::spawn(async move {
                    let server = service
                        .serve_with_ct(transport, ct_child)
                        .await
                        .map_err(std::io::Error::other)?;
                    server.waiting().await?;
                    tokio::io::Result::Ok(())
                });
            }
        });
        self.config.ct.clone()
    }

    /// Attaches an MCP service using direct initialization.
    ///
    /// Similar to [`with_service`](Self::with_service) but skips the standard MCP
    /// initialization handshake. This is useful when the client doesn't require
    /// the initialization phase or when implementing custom initialization logic.
    ///
    /// # Arguments
    ///
    /// * `service_provider` - A function that creates new service instances
    ///
    /// # Returns
    ///
    /// Returns the server's cancellation token.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rmcp_actix_web::SseServer;
    /// # use rmcp::{ServerHandler, model::ServerInfo};
    /// # struct MyService;
    /// # impl ServerHandler for MyService {
    /// #     fn get_info(&self) -> ServerInfo { ServerInfo::default() }
    /// # }
    /// # impl MyService { fn new() -> Self { Self } }
    ///
    /// #[actix_web::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let server = SseServer::serve("127.0.0.1:8080".parse().unwrap()).await?;
    ///     
    ///     // Skip initialization handshake
    ///     let ct = server.with_service_directly(|| MyService::new());
    ///     
    ///     ct.cancelled().await;
    ///     Ok(())
    /// }
    /// ```
    pub fn with_service_directly<S, F>(self, service_provider: F) -> CancellationToken
    where
        S: rmcp::ServerHandler,
        F: Fn() -> S + Send + 'static,
    {
        let ct = self.config.ct.clone();
        let transport_rx = self.transport_rx.clone();

        actix_rt::spawn(async move {
            while let Some(transport) = transport_rx.lock().await.recv().await {
                let service = service_provider();
                let ct_child = ct.child_token();
                tokio::spawn(async move {
                    let server = serve_directly_with_ct(service, transport, None, ct_child);
                    server.waiting().await?;
                    tokio::io::Result::Ok(())
                });
            }
        });
        self.config.ct.clone()
    }

    /// Cancels the server by triggering its cancellation token.
    ///
    /// This will shut down the HTTP server and any active client connections.
    pub fn cancel(&self) {
        self.config.ct.cancel();
    }

    /// Waits for and returns the next client transport connection.
    ///
    /// This method is primarily used internally but can be useful for
    /// advanced use cases where you want to handle transports manually.
    ///
    /// Returns `None` when the server is shut down.
    pub async fn next_transport(&self) -> Option<SseServerTransport> {
        self.transport_rx.lock().await.recv().await
    }
}

impl Stream for SseServer {
    type Item = SseServerTransport;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let mut rx = match self.transport_rx.try_lock() {
            Ok(rx) => rx,
            Err(_) => {
                cx.waker().wake_by_ref();
                return std::task::Poll::Pending;
            }
        };
        rx.poll_recv(cx)
    }
}
