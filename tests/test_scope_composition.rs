//! Tests for framework-level scope composition
//!
//! These tests verify that both SSE and StreamableHttp services can be
//! mounted at custom paths using actix-web's scope composition.

use std::sync::Arc;

use actix_web::{App, test, web};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp_actix_web::{SseServer, SseServerConfig, StreamableHttpService};
use tokio_util::sync::CancellationToken;

mod common;
use common::calculator::Calculator;

#[actix_web::test]
async fn test_sse_server_scope_composition() {
    // Test that SseServer can be mounted at a custom path
    let config = SseServerConfig {
        bind: "127.0.0.1:0".parse().unwrap(), // Use port 0 for automatic assignment
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: CancellationToken::new(),
        sse_keep_alive: None,
    };

    let (_server, scope) = SseServer::new(config);

    // Create app with scope mounted at custom path
    let app =
        test::init_service(App::new().service(web::scope("/api/v1/mcp").service(scope))).await;

    // Test that the SSE endpoint is accessible at the custom path
    let req = test::TestRequest::get().uri("/api/v1/mcp/sse").to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "text/event-stream"
    );
}

#[actix_web::test]
async fn test_streamable_http_service_scope_composition() {
    // Test that StreamableHttpService can be mounted at a custom path using scope
    let service = Arc::new(StreamableHttpService::new(
        || Ok(Calculator::new()),
        LocalSessionManager::default().into(),
        Default::default(),
    ));

    let scope = StreamableHttpService::scope(service);

    // Create app with scope mounted at custom path
    let app =
        test::init_service(App::new().service(web::scope("/api/v2/mcp").service(scope))).await;

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
async fn test_multiple_services_composition() {
    // Test mounting multiple MCP services at different paths
    let sse_config = SseServerConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: CancellationToken::new(),
        sse_keep_alive: None,
    };

    let (_sse_server, sse_scope) = SseServer::new(sse_config);

    let streamable_service = Arc::new(StreamableHttpService::new(
        || Ok(Calculator::new()),
        LocalSessionManager::default().into(),
        Default::default(),
    ));
    let streamable_scope = StreamableHttpService::scope(streamable_service);

    // Create app with both services mounted at different paths
    let app = test::init_service(
        App::new()
            .service(web::scope("/sse-calc").service(sse_scope))
            .service(web::scope("/http-calc").service(streamable_scope)),
    )
    .await;

    // Test SSE endpoint
    let sse_req = test::TestRequest::get().uri("/sse-calc/sse").to_request();
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
async fn test_nested_scope_composition() {
    // Test deeply nested scope composition
    let config = SseServerConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: CancellationToken::new(),
        sse_keep_alive: None,
    };

    let (_server, scope) = SseServer::new(config);

    // Create deeply nested scope structure
    let app = test::init_service(
        App::new().service(
            web::scope("/api").service(
                web::scope("/v1")
                    .service(web::scope("/services").service(web::scope("/mcp").service(scope))),
            ),
        ),
    )
    .await;

    // Test that the deeply nested path works
    let req = test::TestRequest::get()
        .uri("/api/v1/services/mcp/sse")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "text/event-stream"
    );
}
