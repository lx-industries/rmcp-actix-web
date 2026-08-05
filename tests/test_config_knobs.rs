//! Runtime-observable coverage for the transport's rmcp config knobs.

use std::sync::Arc;

use actix_web::{App, test, web};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp_actix_web::transport::StreamableHttpService;

mod common;
use common::calculator::Calculator;

/// Builds a service whose accepted body size is capped at `limit` bytes.
fn service_with_body_limit(limit: usize) -> StreamableHttpService<Calculator> {
    StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(Calculator::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .max_request_body_bytes(limit)
        .build()
}

/// Builds a well-formed `initialize` request body.
fn initialize_body() -> serde_json::Value {
    serde_json::json!({
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
    })
}

/// Builds a `POST` addressed at the mounted transport.
///
/// The in-process test client sends no `Host` header, which the transport's
/// DNS-rebinding defence rejects outright, so one is supplied here.
fn post_request() -> test::TestRequest {
    test::TestRequest::post()
        .uri("/mcp")
        .insert_header(("host", "localhost"))
        .insert_header(("content-type", "application/json"))
        .insert_header(("accept", "application/json, text/event-stream"))
}

#[actix_web::test]
async fn oversized_request_body_is_rejected_with_413() {
    let service = service_with_body_limit(256);
    let app =
        test::init_service(App::new().service(web::scope("/mcp").service(service.scope()))).await;

    let mut oversized = initialize_body();
    oversized["params"]["clientInfo"]["version"] = serde_json::Value::String("x".repeat(4096));
    let request = post_request().set_json(&oversized).to_request();

    let response = test::call_service(&app, request).await;

    assert_eq!(response.status().as_u16(), 413);
}

/// The configured limit, not something unconditional, is what decides the `413`.
///
/// One body, two services: below the limit it is served, above it is rejected. A limit
/// that stopped reaching rmcp would leave both under rmcp's own default and collapse the
/// two outcomes into one.
#[actix_web::test]
async fn the_configured_limit_decides_whether_a_body_is_rejected_with_413() {
    let body = initialize_body();
    let body_len = serde_json::to_vec(&body)
        .expect("the body serializes")
        .len();

    let under = service_with_body_limit(body_len * 2);
    let over = service_with_body_limit(body_len / 2);

    let app_under =
        test::init_service(App::new().service(web::scope("/mcp").service(under.scope()))).await;
    let app_over =
        test::init_service(App::new().service(web::scope("/mcp").service(over.scope()))).await;

    let accepted =
        test::call_service(&app_under, post_request().set_json(&body).to_request()).await;
    let rejected = test::call_service(&app_over, post_request().set_json(&body).to_request()).await;

    assert_ne!(accepted.status().as_u16(), 413);
    assert_eq!(rejected.status().as_u16(), 413);
}
