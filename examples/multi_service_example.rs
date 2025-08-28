//! Multi-Service Composition Example
//!
//! This example demonstrates how to compose multiple MCP services using both
//! SSE and StreamableHttp transports within a single actix-web application
//! using the unified builder pattern.
//!
//! ## Key Features Demonstrated
//!
//! - Multiple MCP services running simultaneously
//! - Different transport types (SSE and StreamableHttp) in one app
//! - Unified builder pattern for both service types
//! - API versioning with scope composition
//! - Service discovery endpoints
//! - Middleware integration and CORS handling
//!
//! ## Services Provided
//!
//! - Calculator (SSE) at `/api/v1/sse/calculator/`
//! - Calculator (StreamableHttp) at `/api/v1/http/calculator/`
//! - Service discovery at `/api/services`
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example multi_service_example
//! ```
//!
//! Then explore the services:
//! ```bash
//! # Get service discovery info
//! curl http://127.0.0.1:8080/api/services
//!
//! # Test SSE calculator
//! curl -N -H "Mcp-Session-Id: test-session" \
//!      http://127.0.0.1:8080/api/v1/sse/calculator/sse
//!
//! # Test StreamableHttp calculator (initialize session)
//! curl -X POST http://127.0.0.1:8080/api/v1/http/calculator/ \
//!      -H "Content-Type: application/json" \
//!      -H "Accept: application/json, text/event-stream" \
//!      -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"curl","version":"1.0"}}}'
//! ```

use actix_web::{App, HttpResponse, HttpServer, Result, middleware, web};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp_actix_web::{SseService, StreamableHttpService};
use std::{sync::Arc, time::Duration};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod common;
use common::calculator::Calculator;

/// Service discovery endpoint that lists all available MCP services
async fn service_discovery() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "services": {
            "calculator_sse": {
                "transport": "sse",
                "version": "1.0.0",
                "endpoints": {
                    "sse": "/api/v1/sse/calculator/sse",
                    "post": "/api/v1/sse/calculator/message"
                },
                "description": "Calculator service using Server-Sent Events",
                "capabilities": ["tools/list", "tools/call"],
                "tools": ["add", "subtract", "multiply", "divide"]
            },
            "calculator_http": {
                "transport": "streamable-http",
                "version": "1.0.0",
                "endpoints": {
                    "base": "/api/v1/http/calculator/"
                },
                "description": "Calculator service using StreamableHttp with sessions",
                "capabilities": ["tools/list", "tools/call"],
                "tools": ["add", "subtract", "multiply", "divide"],
                "features": ["stateful_sessions", "session_management"]
            }
        },
        "meta": {
            "total_services": 2,
            "transport_types": ["sse", "streamable-http"],
            "api_version": "v1",
            "protocol": "Model Context Protocol (MCP)"
        },
        "usage": {
            "sse": "Connect to SSE endpoint for real-time streaming, POST messages to post endpoint",
            "streamable_http": "POST initialize request to create session, then use Mcp-Session-Id header"
        }
    })))
}

/// Health check endpoint that validates all services
async fn health_check() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "services": {
            "calculator_sse": "running",
            "calculator_http": "running"
        },
        "version": "1.0.0"
    })))
}

/// Root endpoint with navigation
async fn root() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Multi-Service MCP Server",
        "description": "Demonstrates composition of multiple MCP services with different transports",
        "endpoints": {
            "health": "/health",
            "services": "/api/services",
            "calculator_sse": "/api/v1/sse/calculator/",
            "calculator_http": "/api/v1/http/calculator/"
        },
        "transports": ["sse", "streamable-http"],
        "documentation": "https://modelcontextprotocol.io/"
    })))
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize comprehensive tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,rmcp_actix_web=debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let bind_addr = "127.0.0.1:8080";
    tracing::info!("🚀 Starting Multi-Service MCP server on {}", bind_addr);

    // === Create Services using Unified Builder Pattern ===

    // SSE Calculator Service
    let sse_service = SseService::builder()
        .service_factory(Arc::new(|| {
            tracing::debug!("Creating new Calculator for SSE transport");
            Ok(Calculator::new())
        }))
        .sse_path("/sse".to_string()) // Custom SSE endpoint
        .post_path("/message".to_string()) // Custom message endpoint
        .sse_keep_alive(Duration::from_secs(30)) // Keep-alive pings
        .build();

    // StreamableHttp Calculator Service - wrap in Arc since it can't be cloned
    let http_service = Arc::new(
        StreamableHttpService::builder()
            .service_factory(Arc::new(|| {
                tracing::debug!("Creating new Calculator for StreamableHttp transport");
                Ok(Calculator::new())
            }))
            .session_manager(Arc::new(LocalSessionManager::default())) // Session management
            .stateful_mode(true) // Enable sessions
            .sse_keep_alive(Duration::from_secs(30)) // Keep-alive pings
            .build(),
    );

    // === Main HTTP Server with All Services ===
    let server = HttpServer::new(move || {
        App::new()
            // === Middleware Stack ===
            .wrap(middleware::Logger::default())
            .wrap(middleware::NormalizePath::trim())
            .wrap(
                middleware::DefaultHeaders::new()
                    .add(("Access-Control-Allow-Origin", "*"))
                    .add(("Access-Control-Allow-Methods", "GET, POST, DELETE, OPTIONS"))
                    .add((
                        "Access-Control-Allow-Headers",
                        "Content-Type, Accept, Mcp-Session-Id, Last-Event-ID",
                    ))
                    .add(("X-Service-Type", "multi-mcp")),
            )
            // === Application Routes ===
            .route("/", web::get().to(root))
            .route("/health", web::get().to(health_check))
            // === API Structure ===
            .service(
                web::scope("/api")
                    // Service discovery
                    .route("/services", web::get().to(service_discovery))
                    // API v1 with different transport services
                    .service(
                        web::scope("/v1")
                            // SSE-based calculator using scope()
                            .service(web::scope("/sse").service(
                                web::scope("/calculator").service(sse_service.clone().scope()),
                            ))
                            // StreamableHttp-based calculator using scope()
                            .service(web::scope("/http").service(
                                web::scope("/calculator").service(http_service.clone().scope()),
                            )),
                    ),
            )
    })
    .bind(bind_addr)?
    .run();

    // === Startup Information ===
    tracing::info!("✅ Multi-Service MCP Server started successfully!");
    tracing::info!("");
    tracing::info!("📊 Service Discovery: http://{}/api/services", bind_addr);
    tracing::info!("🏥 Health Check: http://{}/health", bind_addr);
    tracing::info!("");
    tracing::info!("🔥 SSE Calculator:");
    tracing::info!(
        "   • SSE Stream: http://{}/api/v1/sse/calculator/sse",
        bind_addr
    );
    tracing::info!(
        "   • POST Endpoint: http://{}/api/v1/sse/calculator/message",
        bind_addr
    );
    tracing::info!("");
    tracing::info!("💻 StreamableHttp Calculator:");
    tracing::info!(
        "   • Base URL: http://{}/api/v1/http/calculator/",
        bind_addr
    );
    tracing::info!("   • Supports: Sessions, Streaming, Request/Response");
    tracing::info!("");
    tracing::info!("💡 Tip: Check /api/services for detailed usage instructions");
    tracing::info!("🛑 Press Ctrl+C to stop all services");

    // === Graceful Shutdown ===
    tokio::select! {
        result = server => {
            if let Err(e) = result {
                tracing::error!("HTTP server error: {}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Received Ctrl+C, shutting down all services gracefully");
        }
    }

    tracing::info!("🔚 All services stopped successfully");
    Ok(())
}
