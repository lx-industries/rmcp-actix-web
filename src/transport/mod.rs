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
//! See [`SseService`][crate::SseService] for the main implementation.
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
//! Both transports support framework-level composition for mounting at custom paths
//! using a unified builder pattern:
//!
//! ```rust,no_run
//! use actix_web::{App, HttpServer, web};
//! use rmcp_actix_web::{SseService, StreamableHttpService};
//! use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
//! use std::{sync::Arc, time::Duration};
//!
//! # use rmcp::{ServerHandler, model::ServerInfo};
//! # #[derive(Clone)]
//! # struct MyService;
//! # impl ServerHandler for MyService {
//! #     fn get_info(&self) -> ServerInfo { ServerInfo::default() }
//! # }
//! # impl MyService { fn new() -> Self { Self } }
//! #[actix_web::main]
//! async fn main() -> std::io::Result<()> {
//!     // SSE service with builder pattern
//!     let sse_service = SseService::builder()
//!         .service_factory(Arc::new(|| Ok(MyService::new())))
//!         .sse_path("/events".to_string())
//!         .post_path("/messages".to_string())
//!         .sse_keep_alive(Duration::from_secs(30))
//!         .build();
//!     
//!     // StreamableHttp service with builder pattern
//!     let http_service = Arc::new(
//!         StreamableHttpService::builder()
//!             .service_factory(Arc::new(|| Ok(MyService::new())))
//!             .session_manager(Arc::new(LocalSessionManager::default()))
//!             .stateful_mode(true)
//!             .sse_keep_alive(Duration::from_secs(30))
//!             .build(),
//!     );
//!     
//!     // Both services mount identically via scope()
//!     HttpServer::new(move || {
//!         App::new()
//!             .service(web::scope("/api/v1/sse").service(sse_service.clone().scope()))
//!             .service(web::scope("/api/v1/http").service(http_service.clone().scope()))
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
