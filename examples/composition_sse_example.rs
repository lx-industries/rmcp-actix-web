//! SSE Server Composition Example
//!
//! This example demonstrates how to use framework-level composition to mount
//! SSE MCP services at custom paths within an existing actix-web application.
//!
//! ## Key Features Demonstrated
//!
//! - Using `SseServer::new()` to get a composable scope
//! - Mounting MCP services at custom paths
//! - Integration with existing actix-web middleware and routes
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example composition_sse_example
//! ```
//!
//! Then test with curl:
//! ```bash
//! # Connect to SSE endpoint at custom path
//! curl -N http://127.0.0.1:8080/api/v1/mcp/sse
//!
//! # Send a message (in another terminal)
//! curl -X POST http://127.0.0.1:8080/api/v1/mcp/message?sessionId=<session_id> \
//!      -H "Content-Type: application/json" \
//!      -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}'
//! ```

use actix_web::{App, HttpResponse, HttpServer, Result, web};
use rmcp_actix_web::{SseServer, SseServerConfig};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod common;
use common::calculator::Calculator;

/// A simple health check endpoint to demonstrate integration with existing routes
async fn health_check() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "mcp-calculator",
        "version": "1.0.0"
    })))
}

/// Root endpoint that shows available services
async fn root() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "MCP Calculator Service",
        "endpoints": {
            "health": "/health",
            "mcp_sse": "/api/v1/mcp/sse",
            "mcp_post": "/api/v1/mcp/message"
        },
        "documentation": "https://modelcontextprotocol.io/"
    })))
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for better debugging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let bind_addr = "127.0.0.1:8080";
    tracing::info!("Starting SSE composition example server on {}", bind_addr);

    // Create SSE server configuration with custom paths
    let _sse_config = SseServerConfig {
        bind: bind_addr.parse()?,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: CancellationToken::new(),
        sse_keep_alive: Some(std::time::Duration::from_secs(30)),
    };

    // Start the SSE server directly since we need custom composition
    let server = HttpServer::new(move || {
        // Create a new SSE server scope for each worker
        let sse_config = SseServerConfig {
            bind: "127.0.0.1:0".parse().unwrap(), // Will be ignored since we're using manual binding
            sse_path: "/sse".to_string(),
            post_path: "/message".to_string(),
            ct: CancellationToken::new(),
            sse_keep_alive: Some(std::time::Duration::from_secs(30)),
        };

        let (sse_server, mcp_scope) = SseServer::new(sse_config);

        // Start the MCP service for this worker
        let _ct = sse_server.with_service(Calculator::new);

        App::new()
            // Add logging middleware
            .wrap(actix_web::middleware::Logger::default())
            // Add custom application routes
            .route("/", web::get().to(root))
            .route("/health", web::get().to(health_check))
            // Mount the MCP service at a custom API path
            .service(
                web::scope("/api")
                    .service(web::scope("/v1").service(web::scope("/mcp").service(mcp_scope))),
            )
        // You could add more API versions here
        // .service(web::scope("/api/v2").service(...))
    })
    .bind(bind_addr)?
    .run();

    tracing::info!("🚀 Server started successfully!");
    tracing::info!("📊 Health check: http://{}/health", bind_addr);
    tracing::info!("🔌 MCP SSE endpoint: http://{}/api/v1/mcp/sse", bind_addr);
    tracing::info!(
        "📨 MCP POST endpoint: http://{}/api/v1/mcp/message",
        bind_addr
    );
    tracing::info!("Press Ctrl+C to stop the server");

    // Run server until cancelled
    tokio::select! {
        result = server => {
            if let Err(e) = result {
                tracing::error!("HTTP server error: {}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Received Ctrl+C, shutting down gracefully");
        }
    }

    Ok(())
}
