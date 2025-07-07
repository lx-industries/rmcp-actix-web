//! Transport implementations for the Model Context Protocol using actix-web.
//!
//! This module provides HTTP-based transport layers that enable MCP services
//! to communicate with clients over standard web protocols. Both transports
//! are built on actix-web and provide different trade-offs between simplicity
//! and functionality.
//!
//! ## Available Transports
//!
//! ### SSE (Server-Sent Events)
//!
//! The [`sse_server`] module provides a unidirectional transport using the
//! [Server-Sent Events][sse-spec] protocol. This is ideal for:
//! - Real-time notifications and updates
//! - Streaming data from server to client
//! - Simple integration with existing web infrastructure
//! - Compatibility with browsers and HTTP/1.1 proxies
//!
//! See [`SseServer`][crate::SseServer] for the main implementation.
//!
//! ### Streamable HTTP
//!
//! The [`streamable_http_server`] module provides a bidirectional transport
//! with session management. This is ideal for:
//! - Full request/response communication patterns
//! - Maintaining client state across requests
//! - Complex interaction patterns
//! - Higher performance for bidirectional communication
//!
//! See [`StreamableHttpService`][crate::StreamableHttpService] for the main implementation.
//!
//! [sse-spec]: https://html.spec.whatwg.org/multipage/server-sent-events.html
//!
//! ## Framework-Level Composition
//!
//! Both transports support framework-level composition for mounting at custom paths:
//!
//! ```rust,no_run
//! use actix_web::{App, HttpServer, web};
//! use rmcp_actix_web::{SseServer, SseServerConfig, StreamableHttpService};
//! use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
//! use tokio_util::sync::CancellationToken;
//! use std::sync::Arc;
//!
//! # use rmcp::{ServerHandler, model::ServerInfo};
//! # struct MyService;
//! # impl ServerHandler for MyService {
//! #     fn get_info(&self) -> ServerInfo { ServerInfo::default() }
//! # }
//! # impl MyService { fn new() -> Self { Self } }
//! #[actix_web::main]
//! async fn main() -> std::io::Result<()> {
//!     // SSE server composition
//!     let sse_config = SseServerConfig {
//!         bind: "127.0.0.1:0".parse().unwrap(),
//!         sse_path: "/sse".to_string(),
//!         post_path: "/message".to_string(),
//!         ct: CancellationToken::new(),
//!         sse_keep_alive: None,
//!     };
//!     let (sse_server, sse_scope) = SseServer::new(sse_config);
//!     let _ct = sse_server.with_service(|| MyService::new());
//!     
//!     // StreamableHttp service composition
//!     let http_service = Arc::new(StreamableHttpService::new(
//!         || Ok(MyService::new()),
//!         LocalSessionManager::default().into(),
//!         Default::default(),
//!     ));
//!     let http_scope = StreamableHttpService::scope(http_service);
//!     
//!     // Compose both in one application
//!     HttpServer::new(move || {
//!         // Create new scopes for each worker
//!         let sse_config = SseServerConfig {
//!             bind: "127.0.0.1:0".parse().unwrap(),
//!             sse_path: "/sse".to_string(),
//!             post_path: "/message".to_string(),
//!             ct: CancellationToken::new(),
//!             sse_keep_alive: None,
//!         };
//!         let (sse_server, sse_scope) = SseServer::new(sse_config);
//!         let _ct = sse_server.with_service(|| MyService::new());
//!         
//!         let http_service = Arc::new(StreamableHttpService::new(
//!             || Ok(MyService::new()),
//!             LocalSessionManager::default().into(),
//!             Default::default(),
//!         ));
//!         let http_scope = StreamableHttpService::scope(http_service);
//!         
//!         App::new()
//!             .service(web::scope("/api/v1/sse").service(sse_scope))
//!             .service(web::scope("/api/v1/http").service(http_scope))
//!     })
//!     .bind("127.0.0.1:8080")?
//!     .run()
//!     .await
//! }
//! ```
//!
//! ## Protocol Compatibility
//!
//! Both transports implement the [MCP protocol specification][mcp] and are compatible
//! with all MCP clients that support HTTP transports. The wire protocol is
//! identical to the Axum-based transports in the main [RMCP crate][rmcp].
//!
//! [mcp]: https://modelcontextprotocol.io/
//! [rmcp]: https://docs.rs/rmcp/

/// Server-Sent Events transport implementation.
///
/// Provides unidirectional streaming from server to client using the SSE protocol.
#[cfg(feature = "transport-sse-server")]
pub mod sse_server;

/// Streamable HTTP transport implementation.
///
/// Provides bidirectional communication with session management.
#[cfg(feature = "transport-streamable-http-server")]
pub mod streamable_http_server;
