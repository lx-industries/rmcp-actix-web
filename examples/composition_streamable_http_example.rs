//! StreamableHttp Service Composition Example
//!
//! This example demonstrates how to use framework-level composition to mount
//! StreamableHttp MCP services at custom paths within an existing actix-web application.
//!
//! ## Key Features Demonstrated
//!
//! - Using `StreamableHttpService::scope()` to get a composable scope
//! - Mounting MCP services at custom paths
//! - Integration with existing actix-web middleware and routes
//! - Session management for stateful MCP communication
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example composition_streamable_http_example
//! ```
//!
//! Then test with curl:
//! ```bash
//! # Initialize a new session
//! curl -X POST http://127.0.0.1:8080/api/v1/calculator/ \
//!      -H "Content-Type: application/json" \
//!      -H "Accept: application/json, text/event-stream" \
//!      -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"curl-client","version":"1.0.0"}}}'
//!
//! # Use the returned session ID for subsequent requests
//! curl -X POST http://127.0.0.1:8080/api/v1/calculator/ \
//!      -H "Content-Type: application/json" \
//!      -H "Accept: application/json, text/event-stream" \
//!      -H "X-Session-Id: <session_id>" \
//!      -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
//! ```

use std::sync::Arc;

use actix_web::{App, HttpResponse, HttpServer, Result, web};
use rmcp::transport::{
    StreamableHttpServerConfig, streamable_http_server::session::local::LocalSessionManager,
};
use rmcp_actix_web::StreamableHttpService;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod common;
use common::calculator::Calculator;

/// A simple health check endpoint to demonstrate integration with existing routes
async fn health_check() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "mcp-calculator-streamable",
        "version": "1.0.0",
        "transport": "streamable-http"
    })))
}

/// API information endpoint
async fn api_info() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "api_version": "v1",
        "services": {
            "calculator": {
                "path": "/api/v1/calculator/",
                "transport": "streamable-http",
                "methods": ["GET", "POST", "DELETE"],
                "description": "MCP Calculator service with session management"
            }
        },
        "usage": {
            "initialize": "POST with initialize method to create session",
            "requests": "POST with X-Session-Id header for subsequent requests",
            "streaming": "GET with X-Session-Id header to receive streaming responses"
        }
    })))
}

/// Root endpoint that shows available services
async fn root() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "MCP Calculator Service (StreamableHttp)",
        "endpoints": {
            "health": "/health",
            "api_info": "/api/info",
            "calculator": "/api/v1/calculator/"
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
    tracing::info!(
        "Starting StreamableHttp composition example server on {}",
        bind_addr
    );

    // Create the StreamableHttp service OUTSIDE HttpServer::new() to share across workers
    let calculator_service = Arc::new(StreamableHttpService::new(
        || {
            tracing::debug!("Creating new Calculator instance for session");
            Ok(Calculator::new())
        },
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig {
            stateful_mode: true, // Enable session management
            sse_keep_alive: Some(std::time::Duration::from_secs(30)),
        },
    ));

    // Create the main HTTP server with framework-level composition
    let server = HttpServer::new(move || {
        // Get the composable scope for the calculator service (cloned for each worker)
        let calculator_scope = StreamableHttpService::scope(calculator_service.clone());

        App::new()
            // Add comprehensive logging middleware
            .wrap(actix_web::middleware::Logger::default())
            // Add CORS middleware for web clients
            .wrap(
                actix_web::middleware::DefaultHeaders::new()
                    .add(("Access-Control-Allow-Origin", "*"))
                    .add(("Access-Control-Allow-Methods", "GET, POST, DELETE, OPTIONS"))
                    .add((
                        "Access-Control-Allow-Headers",
                        "Content-Type, Accept, X-Session-Id",
                    )),
            )
            // Add custom application routes
            .route("/", web::get().to(root))
            .route("/health", web::get().to(health_check))
            .route("/api/info", web::get().to(api_info))
            // Mount the MCP calculator service at a custom API path
            .service(web::scope("/api").service(
                web::scope("/v1").service(web::scope("/calculator").service(calculator_scope)),
            ))
        // You could add more services here:
        // .service(
        //     web::scope("/api/v1/file-manager")
        //         .service(file_manager_scope)
        // )
        // .service(
        //     web::scope("/api/v2/calculator")
        //         .service(calculator_v2_scope)
        // )
    })
    .bind(bind_addr)?
    .run();

    tracing::info!("🚀 Server started successfully!");
    tracing::info!("📊 Health check: http://{}/health", bind_addr);
    tracing::info!("📋 API info: http://{}/api/info", bind_addr);
    tracing::info!(
        "🧮 Calculator service: http://{}/api/v1/calculator/",
        bind_addr
    );
    tracing::info!(
        "💡 Tip: Use stateful mode - create session with initialize, then use X-Session-Id header"
    );
    tracing::info!("Press Ctrl+C to stop the server");

    // Handle graceful shutdown
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
