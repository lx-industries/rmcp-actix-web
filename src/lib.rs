//! # rmcp-actix-web
//!
#![warn(missing_docs)]
//! actix-web transport implementations for RMCP (Rust Model Context Protocol).
//!
//! This crate provides HTTP-based transport layers for the [Model Context Protocol (MCP)][mcp],
//! offering a complete alternative to the default Axum-based transports in the main [RMCP crate][rmcp].
//! If you're already using actix-web in your application or prefer its API, this crate allows
//! you to integrate MCP services seamlessly without introducing additional web frameworks.
//!
//! [mcp]: https://modelcontextprotocol.io/
//! [rmcp]: https://crates.io/crates/rmcp
//!
//! ## Features
//!
//! - **[SSE (Server-Sent Events) Transport][SseServer]**: Real-time, unidirectional communication from server to client
//! - **[Streamable HTTP Transport][StreamableHttpService]**: Bidirectional communication with session management
//! - **Full MCP Compatibility**: Implements the complete MCP specification
//! - **Drop-in Replacement**: Same service implementations work with either Axum or actix-web transports
//! - **Production Ready**: Built on battle-tested actix-web framework
//!
//! ## Quick Start
//!
//! ### SSE Server Example
//!
//! ```rust,no_run
//! use rmcp_actix_web::SseServer;
//! use rmcp::{ServerHandler, model::ServerInfo};
//!
//! # struct MyMcpService;
//! # impl ServerHandler for MyMcpService {
//! #     fn get_info(&self) -> ServerInfo { ServerInfo::default() }
//! # }
//! # impl MyMcpService {
//! #     fn new() -> Self { Self }
//! # }
//! #[actix_web::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create SSE server with default configuration
//!     let server = SseServer::serve("127.0.0.1:8080".parse()?).await?;
//!     
//!     // Attach your MCP service implementation
//!     let ct = server.with_service(|| MyMcpService::new());
//!     
//!     // Server runs until cancellation token is triggered
//!     ct.cancelled().await;
//!     Ok(())
//! }
//! ```
//!
//! ### Streamable HTTP Server Example
//!
//! ```rust,no_run
//! use rmcp_actix_web::StreamableHttpService;
//! use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
//! use rmcp::{ServerHandler, model::ServerInfo};
//! use actix_web::{App, HttpServer};
//! use std::sync::Arc;
//!
//! # struct MyMcpService;
//! # impl ServerHandler for MyMcpService {
//! #     fn get_info(&self) -> ServerInfo { ServerInfo::default() }
//! # }
//! # impl MyMcpService {
//! #     fn new() -> Self { Self }
//! # }
//! #[actix_web::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let service = Arc::new(StreamableHttpService::new(
//!         || Ok(MyMcpService::new()),
//!         LocalSessionManager::default().into(),
//!         Default::default(),
//!     ));
//!
//!     HttpServer::new(move || {
//!         App::new()
//!             .configure(StreamableHttpService::configure(service.clone()))
//!     })
//!     .bind("127.0.0.1:8080")?
//!     .run()
//!     .await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Transport Selection
//!
//! Choose between the two transport types based on your needs:
//!
//! - **[SSE Transport][transport::sse_server]**: Best for server-to-client streaming, simpler protocol, works through proxies
//! - **[Streamable HTTP][transport::streamable_http_server]**: Full bidirectional communication, session management, more complex protocol
//!
//! ## Examples
//!
//! See the `examples/` directory for complete working examples:
//! - `counter_sse.rs` - SSE server with a simple counter service
//! - `counter_streamable_http.rs` - Streamable HTTP server example
//!
//! ## Integration with Existing actix-web Applications
//!
//! Both transports can be integrated into existing actix-web applications using the
//! standard actix-web configuration patterns. The [`SseServer::new()`] method returns
//! a [`Scope`][actix_web::Scope] that can be mounted in your existing [`App`][actix_web::App].
//!
//! ## Performance Considerations
//!
//! - SSE transport has lower overhead for unidirectional communication
//! - Streamable HTTP maintains persistent sessions which may use more memory
//! - Both transports use efficient async I/O through actix-web's actor system
//!
//! ## Feature Flags
//!
//! This crate currently has no feature flags. All transports are included by default.

pub mod transport;

// Direct exports of main types

/// Server-Sent Events (SSE) transport server implementation.
///
/// Provides real-time, unidirectional communication from server to client using
/// the SSE protocol. Ideal for streaming updates, notifications, and real-time data.
///
/// See the [module documentation](transport::sse_server) for more details.
pub use transport::sse_server::SseServer;

/// Streamable HTTP transport service for actix-web integration.
///
/// Provides bidirectional communication with session management using a custom
/// HTTP streaming protocol. Supports both request/response and streaming patterns.
///
/// See the [module documentation](transport::streamable_http_server) for more details.
pub use transport::streamable_http_server::StreamableHttpService;

// Re-exports of configuration types from rmcp

/// Unique identifier for client sessions in server-side HTTP transports.
///
/// Used by both SSE and Streamable HTTP transports to track individual client connections.
pub use rmcp::transport::common::server_side_http::SessionId;

/// Configuration for the SSE server transport.
///
/// Allows customization of bind address, endpoints, keep-alive intervals, and cancellation.
///
/// # Example
/// ```rust,no_run
/// use rmcp_actix_web::SseServerConfig;
/// use tokio_util::sync::CancellationToken;
///
/// let config = SseServerConfig {
///     bind: "127.0.0.1:8080".parse().unwrap(),
///     sse_path: "/sse".to_string(),
///     post_path: "/message".to_string(),
///     ct: CancellationToken::new(),
///     sse_keep_alive: Some(std::time::Duration::from_secs(30)),
/// };
/// ```
pub use rmcp::transport::sse_server::SseServerConfig;

/// Configuration for the streamable HTTP server transport.
///
/// Currently uses default configuration. Future versions may add customization options.
pub use rmcp::transport::streamable_http_server::StreamableHttpServerConfig;
