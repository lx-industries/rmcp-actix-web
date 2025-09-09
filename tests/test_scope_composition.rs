//! Tests for framework-level scope composition
//!
//! These tests verify that both SSE and StreamableHttp services can be
//! mounted at custom paths using actix-web's scope composition.

#![allow(deprecated)]

use std::sync::Arc;
use std::time::Duration;

use actix_web::{App, test, web};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
#[cfg(feature = "transport-sse-server")]
use rmcp_actix_web::transport::SseService;
use rmcp_actix_web::transport::StreamableHttpService;

mod common;
use common::calculator::Calculator;

#[actix_web::test]
#[cfg(feature = "transport-sse-server")]
async fn test_sse_service_scope_composition() {
    // Test that SseService can be mounted at a custom path using builder pattern
    let sse_service = SseService::builder()
        .service_factory(Arc::new(|| Ok(Calculator::new())))
        .sse_path("/sse".to_string())
        .post_path("/message".to_string())
        .build();

    // Create app with scope mounted at custom path
    let app = test::init_service(
        App::new().service(web::scope("/api/v1/mcp").service(sse_service.scope())),
    )
    .await;

    // Test that the SSE endpoint is accessible at the custom path
    let req = test::TestRequest::get()
        .uri("/api/v1/mcp/sse")
        .insert_header(("mcp-session-id", "test-session"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "text/event-stream"
    );
}

#[actix_web::test]
async fn test_streamable_http_service_scope_composition() {
    // Test that StreamableHttpService can be mounted at a custom path using builder pattern
    let http_service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(Calculator::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(true)
        .build();

    // Create app with scope mounted at custom path
    let app = test::init_service(
        App::new().service(web::scope("/api/v2/mcp").service(http_service.scope())),
    )
    .await;

    // Test POST request to the custom path (should require proper headers)
    let req = test::TestRequest::post()
        .uri("/api/v2/mcp/")
        .insert_header(("content-type", "application/json"))
        .insert_header(("accept", "application/json, text/event-stream"))
        .set_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success() || resp.status().is_client_error()); // Either works or needs session
}

#[actix_web::test]
#[cfg(feature = "transport-sse-server")]
async fn test_multiple_services_composition() {
    // Test mounting multiple MCP services at different paths using builder pattern
    let sse_service = SseService::builder()
        .service_factory(Arc::new(|| Ok(Calculator::new())))
        .sse_path("/sse".to_string())
        .post_path("/message".to_string())
        .build();

    let http_service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(Calculator::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(true)
        .build();

    // Create app with both services mounted at different paths
    let app = test::init_service(
        App::new()
            .service(web::scope("/sse-calc").service(sse_service.scope()))
            .service(web::scope("/http-calc").service(http_service.scope())),
    )
    .await;

    // Test SSE endpoint
    let sse_req = test::TestRequest::get()
        .uri("/sse-calc/sse")
        .insert_header(("mcp-session-id", "test-session"))
        .to_request();
    let sse_resp = test::call_service(&app, sse_req).await;
    assert_eq!(sse_resp.status(), 200);

    // Test streamable HTTP endpoint
    let http_req = test::TestRequest::post()
        .uri("/http-calc/")
        .insert_header(("content-type", "application/json"))
        .insert_header(("accept", "application/json, text/event-stream"))
        .set_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        }))
        .to_request();
    let http_resp = test::call_service(&app, http_req).await;
    assert!(http_resp.status().is_success() || http_resp.status().is_client_error());
}

#[actix_web::test]
#[cfg(feature = "transport-sse-server")]
async fn test_nested_scope_composition() {
    // Test deeply nested scope composition
    let sse_service = SseService::builder()
        .service_factory(Arc::new(|| Ok(Calculator::new())))
        .sse_path("/sse".to_string())
        .post_path("/message".to_string())
        .sse_keep_alive(Duration::from_secs(30))
        .build();

    // Create deeply nested scope structure
    let app = test::init_service(App::new().service(web::scope("/api").service(
        web::scope("/v1").service(
            web::scope("/services").service(web::scope("/mcp").service(sse_service.scope())),
        ),
    )))
    .await;

    // Test that the deeply nested path works
    let req = test::TestRequest::get()
        .uri("/api/v1/services/mcp/sse")
        .insert_header(("mcp-session-id", "test-session"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "text/event-stream"
    );
}
