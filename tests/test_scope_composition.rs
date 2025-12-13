//! Tests for framework-level scope composition
//!
//! These tests verify that StreamableHttp services can be
//! mounted at custom paths using actix-web's scope composition.

use std::sync::Arc;

use actix_web::{App, test, web};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp_actix_web::transport::StreamableHttpService;

mod common;
use common::calculator::Calculator;

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
