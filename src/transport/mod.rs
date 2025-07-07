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
//! ## Integration
//!
//! Both transports can be integrated into existing actix-web applications:
//!
//! ```rust,no_run
//! use actix_web::{App, HttpServer};
//! use rmcp_actix_web::{SseServer, SseServerConfig};
//! use tokio_util::sync::CancellationToken;
//!
//! #[actix_web::main]
//! async fn main() -> std::io::Result<()> {
//!     // Standalone SSE server
//!     let server = SseServer::serve("127.0.0.1:8080".parse().unwrap()).await?;
//!     
//!     // Or integrate into existing app
//!     let config = SseServerConfig {
//!         bind: "127.0.0.1:8080".parse().unwrap(),
//!         sse_path: "/sse".to_string(),
//!         post_path: "/message".to_string(),
//!         ct: CancellationToken::new(),
//!         sse_keep_alive: None,
//!     };
//!     let (sse_server, scope) = SseServer::new(config);
//!     
//!     // Mount into existing app (scope usage would require Arc wrapping)
//!     // HttpServer would be configured here
//!     Ok(())
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
