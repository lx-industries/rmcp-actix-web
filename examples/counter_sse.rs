//! Server-Sent Events (SSE) transport example.
//!
//! This example demonstrates how to use the SSE transport to create an MCP server
//! that streams updates to clients. The example implements a simple counter service
//! that increments every second using the unified builder pattern.
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run --example counter_sse
//! ```
//!
//! ## Testing with curl
//!
//! In another terminal, connect to the SSE stream:
//! ```bash
//! curl -N -H "Mcp-Session-Id: test-session" http://localhost:8000/sse
//! ```
//!
//! Send a request to get the counter value:
//! ```bash
//! curl -X POST http://localhost:8000/message \
//!   -H "Content-Type: application/json" \
//!   -H "Mcp-Session-Id: test-session" \
//!   -d '{"jsonrpc":"2.0","method":"counter/current","params":{},"id":1}'
//! ```
//!
//! ## Architecture
//!
//! The SSE transport uses two endpoints:
//! - `/sse` - Server-to-client event stream
//! - `/message` - Client-to-server message endpoint
//!
//! Clients must provide the same session ID in both connections.

use actix_web::{App, HttpServer, middleware};
use rmcp_actix_web::transport::SseService;
use std::{sync::Arc, time::Duration};
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    {self},
};

mod common;
use common::counter::Counter;

const BIND_ADDRESS: &str = "127.0.0.1:8000";

/// Example SSE server using rmcp-actix-web with unified builder pattern.
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

    // Create SSE service using builder pattern
    let sse_service = SseService::builder()
        .service_factory(Arc::new(|| Ok(Counter::new()))
            as Arc<dyn Fn() -> Result<Counter, std::io::Error> + Send + Sync>) // Create new Counter for each session
        .sse_path("/sse".to_string()) // SSE endpoint path
        .post_path("/message".to_string()) // Message endpoint path
        .sse_keep_alive(Duration::from_secs(30)) // Keep-alive pings every 30 seconds
        .build();

    println!("\n🚀 SSE Server (actix-web) running at http://{BIND_ADDRESS}");
    println!("📡 SSE endpoint: http://{BIND_ADDRESS}/sse");
    println!("📮 Message endpoint: http://{BIND_ADDRESS}/message");
    println!("\nPress Ctrl+C to stop the server\n");

    // Start the HTTP server with the SSE service mounted
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(middleware::NormalizePath::trim())
            // Mount SSE service at root - endpoints will be /sse and /message
            .service(sse_service.clone().scope())
    })
    .bind(BIND_ADDRESS)?
    .run()
    .await?;

    println!("✅ Server stopped");
    Ok(())
}
