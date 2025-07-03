//! Server-Sent Events (SSE) transport example.
//!
//! This example demonstrates how to use the SSE transport to create an MCP server
//! that streams updates to clients. The example implements a simple counter service
//! that increments every second.
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
//! curl -N -H "X-Session-Id: test-session" http://localhost:8000/sse
//! ```
//!
//! Send a request to get the counter value:
//! ```bash
//! curl -X POST http://localhost:8000/message \
//!   -H "Content-Type: application/json" \
//!   -H "X-Session-Id: test-session" \
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

use rmcp_actix_web::{SseServer, SseServerConfig};
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    {self},
};
mod common;
use common::counter::Counter;

const BIND_ADDRESS: &str = "127.0.0.1:8000";

/// Example SSE server using rmcp-actix-web.
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

    // Configure the SSE server
    let config = SseServerConfig {
        bind: BIND_ADDRESS.parse()?,
        sse_path: "/sse".to_string(), // Endpoint for SSE connections
        post_path: "/message".to_string(), // Endpoint for client messages
        ct: tokio_util::sync::CancellationToken::new(),
        sse_keep_alive: None, // Use default keep-alive interval
    };

    // Keep a reference to the cancellation token for shutdown
    let ct_signal = config.ct.clone();

    // Start the SSE server with our configuration
    let sse_server = SseServer::serve_with_config(config).await?;
    let bind_addr = sse_server.config.bind;

    // Attach the Counter service - a new instance is created for each client
    let ct = sse_server.with_service(Counter::new);

    println!("\n🚀 SSE Server (actix-web) running at http://{bind_addr}");
    println!("📡 SSE endpoint: http://{bind_addr}/sse");
    println!("📮 Message endpoint: http://{bind_addr}/message");
    println!("\nPress Ctrl+C to stop the server\n");

    // Set up Ctrl-C handler
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        println!("\n⏹️  Shutting down...");
        ct_signal.cancel();
    });

    // Wait for cancellation
    ct.cancelled().await;
    println!("✅ Server stopped");
    Ok(())
}
