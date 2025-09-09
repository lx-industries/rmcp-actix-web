//! Server-Sent Events (SSE) transport implementation for MCP.
//!
//! **DEPRECATED**: This transport is deprecated in favor of the StreamableHttp transport,
//! which provides better bidirectional communication and session management capabilities.
//! SSE transport will be removed in a future version. Please migrate to StreamableHttp.
//!
//! This module provides a unidirectional transport using the SSE protocol,
//! allowing servers to push real-time updates to clients over a standard HTTP connection.

#![allow(deprecated)]
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
//! - Builder pattern for configuration
//! - Compatible with proxies and firewalls
//!
//! ## Example
//!
//! ```rust,no_run
//! use rmcp_actix_web::transport::SseService;
//! use actix_web::{App, web};
//! use std::time::Duration;
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
//!     let sse_service = SseService::builder()
//!         .service_factory(std::sync::Arc::new(|| Ok(MyService::new())))
//!         .sse_path("/events".to_string())
//!         .post_path("/messages".to_string())
//!         .sse_keep_alive(Duration::from_secs(30))
//!         .build();
//!
//!     let app = App::new()
//!         .service(web::scope("/api").service(sse_service.scope()));
//!
//!     Ok(())
//! }
//! ```

use std::{collections::HashMap, sync::Arc, time::Duration};

use actix_web::{
    HttpRequest, HttpResponse, Result, Scope,
    error::ErrorInternalServerError,
    http::header::{self, CACHE_CONTROL, CONTENT_TYPE},
    middleware,
    web::{self, Bytes, Data, Json, Query},
};
use futures::{Sink, SinkExt, Stream, StreamExt};
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;

use crate::transport::AuthorizationHeader;
use rmcp::{
    RoleServer,
    model::{ClientJsonRpcMessage, GetExtensions},
    service::{RxJsonRpcMessage, TxJsonRpcMessage, serve_directly_with_ct},
    transport::common::server_side_http::{DEFAULT_AUTO_PING_INTERVAL, SessionId, session_id},
};

const HEADER_X_ACCEL_BUFFERING: &str = "X-Accel-Buffering";

type TxStore =
    Arc<tokio::sync::RwLock<HashMap<SessionId, tokio::sync::mpsc::Sender<ClientJsonRpcMessage>>>>;

#[derive(Clone, Debug)]
struct AppData {
    txs: TxStore,
    transport_tx: tokio::sync::mpsc::UnboundedSender<SseServerTransport>,
    post_path: Arc<str>,
    sse_path: Arc<str>,
    sse_ping_interval: Duration,
}

// AppData::new is no longer used since we create AppData directly
// in the scope method with shared session storage

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
    req: HttpRequest,
    mut message: Json<ClientJsonRpcMessage>,
) -> Result<HttpResponse> {
    let session_id = &query.session_id;
    tracing::debug!(session_id, ?message, "new client message");

    // Extract and inject Authorization header if present (Bearer tokens only)
    if let ClientJsonRpcMessage::Request(request_msg) = &mut message.0
        && let Some(auth_value) = req.headers().get(header::AUTHORIZATION)
        && let Ok(auth_str) = auth_value.to_str()
        && auth_str.starts_with("Bearer ")
    {
        request_msg
            .request
            .extensions_mut()
            .insert(AuthorizationHeader(auth_str.to_string()));
        tracing::debug!("Forwarding Authorization header for MCP proxy scenario");
    }

    let tx = {
        let rg = app_data.txs.read().await;
        rg.get(session_id.as_str())
            .ok_or_else(|| actix_web::error::ErrorNotFound("Session not found"))?
            .clone()
    };

    if tx.send(message.0).await.is_err() {
        tracing::error!("send message error");
        return Err(actix_web::error::ErrorGone("Session closed"));
    }

    Ok(HttpResponse::Accepted().finish())
}

async fn sse_handler(app_data: Data<AppData>, req: HttpRequest) -> Result<HttpResponse> {
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

    // Get the current path prefix from the request (remove the SSE endpoint part)
    let current_path = req.path();
    let sse_endpoint = &app_data.sse_path;
    let path_prefix = if current_path.ends_with(sse_endpoint.as_ref()) {
        &current_path[..current_path.len() - sse_endpoint.len()]
    } else {
        current_path
    };
    let relative_post_path = format!("{}{}", path_prefix, post_path);

    // Create SSE response stream
    let sse_stream = async_stream::stream! {
        // Send initial endpoint message
        yield Ok::<_, actix_web::Error>(Bytes::from(format!(
            "event: endpoint\ndata: {}?sessionId={}\n\n", relative_post_path, session_for_stream
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
#[deprecated(
    since = "0.7.0",
    note = "SSE transport is deprecated in favor of StreamableHttp transport which provides better bidirectional communication and session management"
)]
pub struct SseServerTransport {
    stream: ReceiverStream<RxJsonRpcMessage<RoleServer>>,
    sink: PollSender<TxJsonRpcMessage<RoleServer>>,
    session_id: SessionId,
    tx_store: TxStore,
}

impl Sink<TxJsonRpcMessage<RoleServer>> for SseServerTransport {
    type Error = std::io::Error;

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

/// Server-Sent Events transport service for MCP.
///
/// Provides a unidirectional streaming transport from server to client using the SSE protocol.
/// Clients connect to the SSE endpoint to receive events and send requests via a separate POST endpoint.
/// Uses a builder pattern for configuration.
///
/// **DEPRECATED**: Use `StreamableHttpService` instead for better bidirectional communication.
///
/// # Architecture
///
/// The service manages two endpoints:
/// - SSE endpoint for server-to-client streaming
/// - POST endpoint for client-to-server messages
///
/// Each client connection is identified by a unique session ID that must be provided
/// in both the SSE connection and POST requests.
///
/// # Example
///
/// ```rust,no_run
/// use rmcp_actix_web::transport::SseService;
/// use actix_web::{App, web};
/// use std::time::Duration;
///
/// # use rmcp::{ServerHandler, model::ServerInfo};
/// # struct MyService;
/// # impl ServerHandler for MyService {
/// #     fn get_info(&self) -> ServerInfo { ServerInfo::default() }
/// # }
/// # impl MyService { fn new() -> Self { Self } }
///
/// let sse_service = SseService::builder()
///     .service_factory(std::sync::Arc::new(|| Ok(MyService::new())))
///     .sse_path("/events".to_string())
///     .post_path("/messages".to_string())
///     .sse_keep_alive(Duration::from_secs(30))
///     .build();
///
/// let app = App::new()
///     .service(web::scope("/api").service(sse_service.scope()));
/// ```
#[deprecated(
    since = "0.7.0",
    note = "SSE transport is deprecated in favor of StreamableHttp transport which provides better bidirectional communication and session management"
)]
#[derive(Clone, bon::Builder)]
pub struct SseService<S> {
    /// The service factory function that creates new MCP service instances
    service_factory: Arc<dyn Fn() -> Result<S, std::io::Error> + Send + Sync>,

    /// The path for the SSE endpoint
    #[builder(default = "/sse".to_string())]
    sse_path: String,

    /// The path for the POST message endpoint
    #[builder(default = "/message".to_string())]
    post_path: String,

    /// Optional keep-alive interval for SSE connections
    sse_keep_alive: Option<Duration>,

    /// Shared session storage across workers
    #[builder(skip = Default::default())]
    shared_txs: TxStore,
}

impl<S> SseService<S>
where
    S: rmcp::ServerHandler + Send + 'static,
{
    /// Creates a new scope configured with this service for framework-level composition.
    ///
    /// This method provides framework-level composition aligned with RMCP patterns,
    /// similar to how `StreamableHttpService::scope()` works. This allows mounting the
    /// SSE service at custom paths using actix-web's routing.
    ///
    /// This method is similar to `scope` except that it allows specifying a custom path.
    ///
    /// # Returns
    ///
    /// Returns an actix-web `Scope` configured with the SSE routes
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rmcp_actix_web::transport::SseService;
    /// use actix_web::{App, HttpServer, web};
    /// use std::time::Duration;
    ///
    /// # use rmcp::{ServerHandler, model::ServerInfo};
    /// # struct MyService;
    /// # impl ServerHandler for MyService {
    /// #     fn get_info(&self) -> ServerInfo { ServerInfo::default() }
    /// # }
    /// # impl MyService { fn new() -> Self { Self } }
    /// let service = SseService::builder()
    ///     .service_factory(std::sync::Arc::new(|| Ok(MyService::new())))
    ///     .sse_path("/events".to_string())
    ///     .post_path("/messages".to_string())
    ///     .build();
    ///
    /// // Mount into existing app at a custom path
    /// let app = App::new()
    ///     .service(service.scope_with_path("/api/v1/mcp"));
    /// ```
    pub fn scope_with_path(
        self,
        path: &str,
    ) -> Scope<
        impl actix_web::dev::ServiceFactory<
            actix_web::dev::ServiceRequest,
            Config = (),
            Response = actix_web::dev::ServiceResponse,
            Error = actix_web::Error,
            InitError = (),
        >,
    > {
        let transport_rx = Arc::new(Mutex::new(None));
        let (transport_tx, rx) = tokio::sync::mpsc::unbounded_channel();
        *transport_rx
            .try_lock()
            .expect("Failed to acquire transport_rx lock") = Some(rx);

        // Create AppData with shared session storage
        let app_data = AppData {
            txs: self.shared_txs.clone(),
            transport_tx,
            post_path: self.post_path.clone().into(),
            sse_path: self.sse_path.clone().into(),
            sse_ping_interval: self.sse_keep_alive.unwrap_or(DEFAULT_AUTO_PING_INTERVAL),
        };

        let sse_path = self.sse_path.clone();
        let post_path = self.post_path.clone();

        let app_data = Data::new(app_data);
        let service_factory = self.service_factory.clone();
        let transport_rx_clone = transport_rx.clone();

        // Start the service handler task
        actix_rt::spawn(async move {
            let mut transport_rx = transport_rx_clone.lock().await.take();
            if let Some(mut rx) = transport_rx.take() {
                while let Some(transport) = rx.recv().await {
                    let service = match service_factory() {
                        Ok(service) => service,
                        Err(e) => {
                            tracing::error!("Failed to create service: {}", e);
                            continue;
                        }
                    };

                    tokio::spawn(async move {
                        let server = serve_directly_with_ct(
                            service,
                            transport,
                            None,
                            tokio_util::sync::CancellationToken::new(),
                        );
                        if let Err(e) = server.waiting().await {
                            tracing::error!("Service error: {}", e);
                        }
                    });
                }
            }
        });

        web::scope(path)
            .app_data(app_data.clone())
            .wrap(middleware::NormalizePath::trim())
            .route(&sse_path, web::get().to(sse_handler))
            .route(&post_path, web::post().to(post_event_handler))
    }

    /// Creates a new scope configured with this service for framework-level composition.
    ///
    /// This method provides framework-level composition aligned with RMCP patterns,
    /// similar to how `StreamableHttpService::scope()` works. This allows mounting the
    /// SSE service at custom paths using actix-web's routing.
    ///
    /// This method is equivalent to `scope_with_path("")`.
    ///
    /// # Returns
    ///
    /// Returns an actix-web `Scope` configured with the SSE routes
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rmcp_actix_web::transport::SseService;
    /// use actix_web::{App, HttpServer, web};
    /// use std::time::Duration;
    ///
    /// # use rmcp::{ServerHandler, model::ServerInfo};
    /// # struct MyService;
    /// # impl ServerHandler for MyService {
    /// #     fn get_info(&self) -> ServerInfo { ServerInfo::default() }
    /// # }
    /// # impl MyService { fn new() -> Self { Self } }
    /// let service = SseService::builder()
    ///     .service_factory(std::sync::Arc::new(|| Ok(MyService::new())))
    ///     .sse_path("/events".to_string())
    ///     .post_path("/messages".to_string())
    ///     .build();
    ///
    /// // Mount into existing app at a custom path
    /// let app = App::new()
    ///     .service(web::scope("/api/v1/mcp").service(service.scope()));
    /// ```
    pub fn scope(
        self,
    ) -> Scope<
        impl actix_web::dev::ServiceFactory<
            actix_web::dev::ServiceRequest,
            Config = (),
            Response = actix_web::dev::ServiceResponse,
            Error = actix_web::Error,
            InitError = (),
        >,
    > {
        self.scope_with_path("")
    }
}
