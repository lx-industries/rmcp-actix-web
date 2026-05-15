//! Integration tests for unknown `Mcp-Session-Id` handling.
//!
//! Per the MCP 2025-03-26 Streamable HTTP Session Management spec, when a
//! request carries an `Mcp-Session-Id` header whose value the server does not
//! recognize, the server must respond with `404 Not Found` so the client can
//! recover by starting a new session via an `InitializeRequest` without a
//! session id. These tests pin that contract for both `POST` and `GET`.

mod common;

use actix_web::{App, HttpServer};
use common::calculator::Calculator;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp_actix_web::transport::StreamableHttpService;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

#[actix_web::test]
async fn post_with_unknown_session_id_returns_404() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("rmcp_actix_web=debug")
        .with_test_writer()
        .try_init();

    let service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(Calculator::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(true)
        .build();

    let server = HttpServer::new(move || {
        App::new().service(actix_web::web::scope("/").service(service.clone().scope()))
    })
    .bind("127.0.0.1:0")
    .expect("Failed to bind server");

    let addr = *server.addrs().first().unwrap();
    let server_handle = server.run();
    let server_task = tokio::spawn(async move {
        let _ = server_handle.await;
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let url = format!("http://{}", addr);

    let tools_list_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/list",
        "id": 1
    });

    let response = client
        .post(&url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .header("Mcp-Session-Id", "definitely-not-a-real-session")
        .json(&tools_list_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        reqwest::StatusCode::NOT_FOUND,
        "POST with unknown Mcp-Session-Id must return 404 Not Found"
    );

    let body = response.text().await.expect("Failed to read response body");
    assert_eq!(
        body, "Session not found",
        "Response body must be the spec-compliant short diagnostic"
    );
    assert!(
        !body.starts_with("Unauthorized:"),
        "Response body must not retain the legacy `Unauthorized:` prefix"
    );

    server_task.abort();
}

#[actix_web::test]
async fn get_with_unknown_session_id_returns_404() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("rmcp_actix_web=debug")
        .with_test_writer()
        .try_init();

    let service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(Calculator::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(true)
        .build();

    let server = HttpServer::new(move || {
        App::new().service(actix_web::web::scope("/").service(service.clone().scope()))
    })
    .bind("127.0.0.1:0")
    .expect("Failed to bind server");

    let addr = *server.addrs().first().unwrap();
    let server_handle = server.run();
    let server_task = tokio::spawn(async move {
        let _ = server_handle.await;
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let url = format!("http://{}", addr);

    let response = client
        .get(&url)
        .header("Accept", "text/event-stream")
        .header("Mcp-Session-Id", "definitely-not-a-real-session")
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        reqwest::StatusCode::NOT_FOUND,
        "GET with unknown Mcp-Session-Id must return 404 Not Found"
    );

    let body = response.text().await.expect("Failed to read response body");
    assert_eq!(
        body, "Session not found",
        "Response body must be the spec-compliant short diagnostic"
    );
    assert!(
        !body.starts_with("Unauthorized:"),
        "Response body must not retain the legacy `Unauthorized:` prefix"
    );

    server_task.abort();
}
