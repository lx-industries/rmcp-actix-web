//! SSE Service Composition Example
//!
//! This example requires the `transport-sse-server` feature to be enabled.
//!
//! **DEPRECATED**: The SSE transport is deprecated in favor of StreamableHttp transport.
//! Please see `composition_streamable_http_example.rs` for the recommended approach.
//!
//! This example demonstrates how to use framework-level composition to mount
//! SSE MCP services at custom paths within an existing actix-web application
//! using the unified builder pattern.
//!
//! ## Key Features Demonstrated
//!
//! - Using `SseService::builder()` to configure the service
//! - Mounting MCP services at custom paths using `.scope()`
//! - Integration with existing actix-web middleware and routes
//! - Unified builder pattern consistent with StreamableHttpService
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
//! curl -N -H "Mcp-Session-Id: test-session" \
//!      http://127.0.0.1:8080/api/v1/mcp/sse
//!
//! # Send a message (in another terminal)
//! curl -X POST http://127.0.0.1:8080/api/v1/mcp/message \
//!      -H "Content-Type: application/json" \
//!      -H "Mcp-Session-Id: test-session" \
//!      -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}'
//! ```

#[cfg(feature = "transport-sse-server")]
use actix_web::{App, HttpResponse, HttpServer, Result, middleware, web};
#[cfg(feature = "transport-sse-server")]
#[allow(deprecated)]
use rmcp_actix_web::transport::SseService;
#[cfg(feature = "transport-sse-server")]
use std::{sync::Arc, time::Duration};
#[cfg(feature = "transport-sse-server")]
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(feature = "transport-sse-server")]
mod common;
#[cfg(feature = "transport-sse-server")]
use common::calculator::Calculator;

/// A simple health check endpoint to demonstrate integration with existing routes
#[cfg(feature = "transport-sse-server")]
async fn health_check() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "mcp-calculator",
        "version": "1.0.0"
    })))
}

/// Root endpoint that shows available services
#[cfg(feature = "transport-sse-server")]
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

#[cfg(feature = "transport-sse-server")]
#[allow(deprecated)]
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

    // Create SSE service using builder pattern with custom paths
    let sse_service = SseService::builder()
        .service_factory(Arc::new(|| Ok(Calculator::new()))) // Create new Calculator for each session
        .sse_path("/sse".to_string()) // Custom SSE endpoint path
        .post_path("/message".to_string()) // Custom message endpoint path
        .sse_keep_alive(Duration::from_secs(30)) // Keep-alive pings every 30 seconds
        .build();

    // Start the HTTP server with framework-level composition
    let server = HttpServer::new(move || {
        App::new()
            // Add logging middleware
            .wrap(middleware::Logger::default())
            .wrap(middleware::NormalizePath::trim())
            // Add custom application routes
            .route("/", web::get().to(root))
            .route("/health", web::get().to(health_check))
            // Mount the MCP service at a custom API path using scope()
            .service(web::scope("/api").service(
                web::scope("/v1").service(web::scope("/mcp").service(sse_service.clone().scope())),
            ))
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

#[cfg(not(feature = "transport-sse-server"))]
fn main() {
    eprintln!("This example requires the 'transport-sse-server' feature to be enabled.");
    eprintln!(
        "Run with: cargo run --example composition_sse_example --features transport-sse-server"
    );
    std::process::exit(1);
}
