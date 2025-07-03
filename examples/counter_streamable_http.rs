//! Streamable HTTP transport example.
//!
//! This example demonstrates how to use the bidirectional streamable HTTP transport
//! with session management. The example implements the same counter service as the
//! SSE example but using the more feature-rich streamable HTTP transport.
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run --example counter_streamable_http
//! ```
//!
//! ## Testing with curl
//!
//! Send a JSON-RPC request (creates a new session):
//! ```bash
//! curl -X POST http://localhost:8080/ \
//!   -H "Content-Type: application/json" \
//!   -H "X-Session-Id: test-session" \
//!   -d '{"jsonrpc":"2.0","method":"counter/current","params":{},"id":1}'
//! ```
//!
//! Resume the SSE stream for a session:
//! ```bash
//! curl -N -H "X-Session-Id: test-session" \
//!   -H "Accept: text/event-stream" \
//!   http://localhost:8080/
//! ```
//!
//! Close a session:
//! ```bash
//! curl -X DELETE -H "X-Session-Id: test-session" http://localhost:8080/
//! ```
//!
//! ## Architecture
//!
//! The streamable HTTP transport provides:
//! - Bidirectional communication
//! - Session persistence
//! - Support for resuming connections
//! - Efficient routing of messages

mod common;
use std::sync::Arc;

use actix_web::{App, HttpServer, middleware};
use common::counter::Counter;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp_actix_web::StreamableHttpService;

/// Example streamable HTTP server using rmcp-actix-web.
///
/// Important: This uses `#[actix_web::main]` instead of `#[tokio::main]`
/// because actix-web requires its own runtime configuration.
#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let bind_addr = "127.0.0.1:8080";

    // Create the streamable HTTP service using rmcp-actix-web
    // This service provides bidirectional communication with session management:
    // - Factory function creates a new Counter instance for each session
    // - LocalSessionManager tracks client sessions in memory
    // - Default config enables stateful mode with standard settings
    let service = Arc::new(StreamableHttpService::new(
        || Ok(Counter::new()),
        LocalSessionManager::default().into(),
        Default::default(),
    ));

    println!("Starting actix-web streamable HTTP server on {bind_addr}");
    println!("POST / - Send JSON-RPC requests");
    println!("GET / - Resume SSE stream with session ID");
    println!("DELETE / - Close session");

    // Use actix-web's HttpServer and App to host the service
    // The StreamableHttpService::configure method sets up the routes:
    // - GET / : SSE endpoint for resuming event streams
    // - POST / : Message endpoint for JSON-RPC requests
    // - DELETE / : Session management endpoint
    HttpServer::new(move || {
        App::new()
            // Add request logging middleware
            .wrap(middleware::Logger::default())
            // Configure MCP routes - mounts the service at root path
            .configure(StreamableHttpService::configure(service.clone()))
    })
    .bind(bind_addr)?
    .run()
    .await?;

    Ok(())
}
