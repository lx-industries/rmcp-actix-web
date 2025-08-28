//! Streamable HTTP transport example.
//!
//! This example demonstrates how to use the bidirectional streamable HTTP transport
//! with session management using the unified builder pattern. The example
//! implements the same counter service as the SSE example but using the more
//! feature-rich streamable HTTP transport.
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
//!   -H "Mcp-Session-Id: test-session" \
//!   -d '{"jsonrpc":"2.0","method":"counter/current","params":{},"id":1}'
//! ```
//!
//! Resume the SSE stream for a session:
//! ```bash
//! curl -N -H "Mcp-Session-Id: test-session" \
//!   -H "Accept: text/event-stream" \
//!   http://localhost:8080/
//! ```
//!
//! Close a session:
//! ```bash
//! curl -X DELETE -H "Mcp-Session-Id: test-session" http://localhost:8080/
//! ```
//!
//! ## Architecture
//!
//! The streamable HTTP transport provides:
//! - Bidirectional communication
//! - Session persistence
//! - Support for resuming connections
//! - Efficient routing of messages

use actix_web::{App, HttpServer, middleware};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp_actix_web::StreamableHttpService;
use std::{sync::Arc, time::Duration};
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    {self},
};

mod common;
use common::counter::Counter;

const BIND_ADDRESS: &str = "127.0.0.1:8080";

/// Example streamable HTTP server using rmcp-actix-web with unified builder pattern.
///
/// Important: This uses `#[actix_web::main]` instead of `#[tokio::main]`
/// because actix-web requires its own runtime configuration.
#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing for debug output
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    println!("\n🚀 Streamable HTTP Server (actix-web) running at http://{BIND_ADDRESS}");
    println!("📡 GET / - Resume SSE stream with session ID");
    println!("📮 POST / - Send JSON-RPC requests");
    println!("🗑️  DELETE / - Close session");
    println!("\nPress Ctrl+C to stop the server\n");

    // Start the HTTP server with the streamable HTTP service mounted
    HttpServer::new(|| {
        // Create streamable HTTP service using builder pattern
        let http_service = StreamableHttpService::builder()
            .service_factory(Arc::new(|| Ok(Counter::new()))) // Create new Counter for each session
            .session_manager(Arc::new(LocalSessionManager::default())) // Local session management
            .stateful_mode(true) // Enable stateful session management
            .sse_keep_alive(Duration::from_secs(30)) // Keep-alive pings every 30 seconds
            .build();

        App::new()
            .wrap(middleware::Logger::default())
            .wrap(middleware::NormalizePath::trim())
            // Mount streamable HTTP service at root - endpoints will be /, GET/POST/DELETE
            .service(http_service.scope())
    })
    .bind(BIND_ADDRESS)?
    .run()
    .await?;

    println!("✅ Server stopped");
    Ok(())
}
