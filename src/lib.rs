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
//! ### SSE Service Example
//!
//! ```rust,no_run
//! use rmcp_actix_web::SseService;
//! use actix_web::{App, HttpServer};
//! use std::time::Duration;
//!
//! # #[derive(Clone)]
//! # struct MyMcpService;
//! # impl rmcp::ServerHandler for MyMcpService {
//! #     fn get_info(&self) -> rmcp::model::ServerInfo { rmcp::model::ServerInfo::default() }
//! # }
//! # impl MyMcpService {
//! #     fn new() -> Self { Self }
//! # }
//! #[actix_web::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let sse_service = SseService::builder()
//!         .service_factory(std::sync::Arc::new(|| Ok(MyMcpService::new())))
//!         .sse_path("/sse".to_string())
//!         .post_path("/message".to_string())
//!         .sse_keep_alive(Duration::from_secs(30))
//!         .build();
//!
//!     HttpServer::new(move || {
//!         App::new()
//!             .service(sse_service.clone().scope())
//!     })
//!     .bind("127.0.0.1:8080")?
//!     .run()
//!     .await?;
//!     
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
//! use std::{sync::Arc, time::Duration};
//!
//! # #[derive(Clone)]
//! # struct MyMcpService;
//! # impl ServerHandler for MyMcpService {
//! #     fn get_info(&self) -> ServerInfo { ServerInfo::default() }
//! # }
//! # impl MyMcpService {
//! #     fn new() -> Self { Self }
//! # }
//! #[actix_web::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     HttpServer::new(|| {
//!         let http_service = StreamableHttpService::builder()
//!             .service_factory(Arc::new(|| Ok(MyMcpService::new())))
//!             .session_manager(Arc::new(LocalSessionManager::default()))
//!             .stateful_mode(true)
//!             .sse_keep_alive(Duration::from_secs(30))
//!             .build();
//!
//!         App::new()
//!             .service(http_service.scope())
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
//! - `composition_sse_example.rs` - SSE server with framework-level composition
//! - `composition_streamable_http_example.rs` - StreamableHttp with custom mounting
//! - `multi_service_example.rs` - Multiple MCP services with different transports
//!
//! ## Framework-Level Composition
//!
//! Both transports support framework-level composition aligned with RMCP patterns,
//! allowing you to mount MCP services at custom paths within existing actix-web applications.
//!
//! ### SSE Service Composition
//!
//! ```rust,no_run
//! use rmcp_actix_web::SseService;
//! use actix_web::{App, web};
//! use std::time::Duration;
//!
//! # #[derive(Clone)]
//! # struct MyService;
//! # impl rmcp::ServerHandler for MyService {
//! #     fn get_info(&self) -> rmcp::model::ServerInfo { rmcp::model::ServerInfo::default() }
//! # }
//! # impl MyService { fn new() -> Self { Self } }
//! let sse_service = SseService::builder()
//!     .service_factory(std::sync::Arc::new(|| Ok(MyService::new())))
//!     .sse_path("/events".to_string())
//!     .post_path("/messages".to_string())
//!     .sse_keep_alive(Duration::from_secs(30))
//!     .build();
//!
//! // Mount at custom path using scope()
//! let app = App::new()
//!     .service(web::scope("/api/v1/calculator").service(sse_service.scope()));
//! ```
//!
//! ### StreamableHttp Service Composition
//!
//! ```rust,no_run
//! use rmcp_actix_web::StreamableHttpService;
//! use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
//! use actix_web::{App, web};
//! use std::{sync::Arc, time::Duration};
//!
//! # use rmcp::{ServerHandler, model::ServerInfo};
//! # #[derive(Clone)]
//! # struct MyService;
//! # impl ServerHandler for MyService {
//! #     fn get_info(&self) -> ServerInfo { ServerInfo::default() }
//! # }
//! # impl MyService { fn new() -> Self { Self } }
//! # use actix_web::HttpServer;
//! # #[actix_web::main]
//! # async fn main() -> std::io::Result<()> {
//! HttpServer::new(|| {
//!     let http_service = StreamableHttpService::builder()
//!         .service_factory(Arc::new(|| Ok(MyService::new())))
//!         .session_manager(Arc::new(LocalSessionManager::default()))
//!         .stateful_mode(true)
//!         .sse_keep_alive(Duration::from_secs(30))
//!         .build();
//!
//!     // Mount at custom path using scope()
//!     App::new()
//!         .service(web::scope("/api/v1/calculator").service(http_service.scope()))
//! }).bind("127.0.0.1:8080")?.run().await
//! # }
//! ```
//!
//! ### Multi-Service Composition
//!
//! ```rust,no_run
//! use rmcp_actix_web::{SseService, StreamableHttpService};
//! use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
//! use actix_web::{App, web};
//! use std::{sync::Arc, time::Duration};
//!
//! # #[derive(Clone)]
//! # struct MyService;
//! # impl rmcp::ServerHandler for MyService {
//! #     fn get_info(&self) -> rmcp::model::ServerInfo { rmcp::model::ServerInfo::default() }
//! # }
//! # impl MyService { fn new() -> Self { Self } }
//! # use actix_web::HttpServer;
//! # #[actix_web::main]
//! # async fn main() -> std::io::Result<()> {
//! HttpServer::new(|| {
//!     // Both services use identical builder pattern
//!     let sse_service = SseService::builder()
//!         .service_factory(Arc::new(|| Ok(MyService::new())))
//!         .sse_path("/events".to_string())
//!         .post_path("/messages".to_string())
//!         .build();
//!
//!     let http_service = StreamableHttpService::builder()
//!         .service_factory(Arc::new(|| Ok(MyService::new())))
//!         .session_manager(Arc::new(LocalSessionManager::default()))
//!         .stateful_mode(true)
//!         .build();
//!
//!     // Both services mount identically via scope()
//!     App::new()
//!         .service(web::scope("/api/sse").service(sse_service.scope()))
//!         .service(web::scope("/api/http").service(http_service.scope()))
//! }).bind("127.0.0.1:8080")?.run().await
//! # }
//! ```
//!
//! See the `examples/` directory for complete working examples of composition patterns.
//!
//! ## Performance Considerations
//!
//! - SSE transport has lower overhead for unidirectional communication
//! - Streamable HTTP maintains persistent sessions which may use more memory
//! - Both transports use efficient async I/O through actix-web's actor system
//! - Framework-level composition adds minimal overhead
//!
//! ## Feature Flags
//!
//! This crate supports selective compilation of transport types:
//!
//! - `transport-sse-server` (default): Enables SSE transport
//! - `transport-streamable-http-server` (default): Enables StreamableHttp transport
//!
//! To use only specific transports, disable default features:
//!
//! ```toml
//! [dependencies]
//! rmcp-actix-web = { version = "0.1", default-features = false, features = ["transport-sse-server"] }
//! ```

pub mod transport;

// Direct exports of main types

/// Server-Sent Events (SSE) transport service implementation.
///
/// Provides real-time, unidirectional communication from server to client using
/// the SSE protocol. Ideal for streaming updates, notifications, and real-time data.
///
/// Uses a builder pattern for configuration and integrates seamlessly with actix-web.
///
/// See the [module documentation](transport::sse_server) for more details.
#[cfg(feature = "transport-sse-server")]
pub use transport::sse_server::SseService;

/// Streamable HTTP transport service for actix-web integration.
///
/// Provides bidirectional communication with session management using a custom
/// HTTP streaming protocol. Supports both request/response and streaming patterns.
///
/// See the [module documentation](transport::streamable_http_server) for more details.
#[cfg(feature = "transport-streamable-http-server")]
pub use transport::streamable_http_server::StreamableHttpService;

// Re-exports of configuration types from rmcp

/// Unique identifier for client sessions in server-side HTTP transports.
///
/// Used by both SSE and Streamable HTTP transports to track individual client connections.
pub use rmcp::transport::common::server_side_http::SessionId;

// Note: SseServerConfig removed in favor of builder pattern

/// Configuration for the streamable HTTP server transport.
///
/// Currently uses default configuration. Future versions may add customization options.
#[cfg(feature = "transport-streamable-http-server")]
pub use rmcp::transport::streamable_http_server::StreamableHttpServerConfig;
