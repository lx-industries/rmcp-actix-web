//! Integration tests for `Mcp-Session-Id` handling.
//!
//! Per the MCP 2025-03-26 Streamable HTTP Session Management spec, when a
//! request carries an `Mcp-Session-Id` header whose value the server does not
//! recognize, the server must respond with `404 Not Found` so the client can
//! recover by starting a new session via an `InitializeRequest` without a
//! session id. When the header is missing or empty on a request that requires
//! a session id, the server must respond with `400 Bad Request`. In stateless
//! mode the header is ignored. These tests pin that contract for `POST`,
//! `GET`, and `DELETE`.

mod common;

use actix_web::{App, HttpServer};
use common::calculator::Calculator;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp_actix_web::transport::StreamableHttpService;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

const MISSING_SESSION_ID_BODY: &str = "Bad Request: Mcp-Session-Id header is required";
const SESSION_NOT_FOUND_BODY: &str = "Session not found";

struct TestServer {
    url: String,
    client: reqwest::Client,
    task: tokio::task::JoinHandle<()>,
}

impl TestServer {
    async fn spawn(stateful: bool) -> Self {
        let _ = tracing_subscriber::fmt()
            .with_env_filter("rmcp_actix_web=debug")
            .with_test_writer()
            .try_init();

        let service = StreamableHttpService::builder()
            .service_factory(Arc::new(|| Ok(Calculator::new())))
            .session_manager(Arc::new(LocalSessionManager::default()))
            .stateful_mode(stateful)
            .build();

        let server = HttpServer::new(move || {
            App::new().service(actix_web::web::scope("/").service(service.clone().scope()))
        })
        .bind("127.0.0.1:0")
        .expect("Failed to bind server");

        let addr = *server.addrs().first().unwrap();
        let server_handle = server.run();
        let task = tokio::spawn(async move {
            let _ = server_handle.await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        Self {
            url: format!("http://{addr}"),
            client: reqwest::Client::new(),
            task,
        }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[actix_web::test]
async fn post_with_unknown_session_id_returns_404() {
    let server = TestServer::spawn(true).await;

    let tools_list_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/list",
        "id": 1
    });

    let response = server
        .client
        .post(&server.url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .header("Mcp-Session-Id", "definitely-not-a-real-session")
        .json(&tools_list_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
    let body = response.text().await.expect("Failed to read response body");
    assert_eq!(body, SESSION_NOT_FOUND_BODY);
}

#[actix_web::test]
async fn get_with_unknown_session_id_returns_404() {
    let server = TestServer::spawn(true).await;

    let response = server
        .client
        .get(&server.url)
        .header("Accept", "text/event-stream")
        .header("Mcp-Session-Id", "definitely-not-a-real-session")
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
    let body = response.text().await.expect("Failed to read response body");
    assert_eq!(body, SESSION_NOT_FOUND_BODY);
}

#[actix_web::test]
async fn delete_with_unknown_session_id_returns_404() {
    let server = TestServer::spawn(true).await;

    let response = server
        .client
        .delete(&server.url)
        .header("Mcp-Session-Id", "definitely-not-a-real-session")
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
    let body = response.text().await.expect("Failed to read response body");
    assert_eq!(body, SESSION_NOT_FOUND_BODY);
}

#[actix_web::test]
async fn get_with_missing_session_id_returns_400() {
    let server = TestServer::spawn(true).await;

    let response = server
        .client
        .get(&server.url)
        .header("Accept", "text/event-stream")
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
    let body = response.text().await.expect("Failed to read response body");
    assert_eq!(body, MISSING_SESSION_ID_BODY);
}

#[actix_web::test]
async fn delete_with_missing_session_id_returns_400() {
    let server = TestServer::spawn(true).await;

    let response = server
        .client
        .delete(&server.url)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
    let body = response.text().await.expect("Failed to read response body");
    assert_eq!(body, MISSING_SESSION_ID_BODY);
}

#[actix_web::test]
async fn post_without_session_id_and_non_initialize_returns_400() {
    let server = TestServer::spawn(true).await;

    let tools_list_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/list",
        "id": 1
    });

    let response = server
        .client
        .post(&server.url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&tools_list_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
    let body = response.text().await.expect("Failed to read response body");
    assert_eq!(body, MISSING_SESSION_ID_BODY);
}

#[actix_web::test]
async fn post_with_empty_session_id_returns_400() {
    let server = TestServer::spawn(true).await;

    let tools_list_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/list",
        "id": 1
    });

    let response = server
        .client
        .post(&server.url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .header("Mcp-Session-Id", "")
        .json(&tools_list_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
    let body = response.text().await.expect("Failed to read response body");
    assert_eq!(body, MISSING_SESSION_ID_BODY);
}

#[actix_web::test]
async fn get_with_empty_session_id_returns_400() {
    let server = TestServer::spawn(true).await;

    let response = server
        .client
        .get(&server.url)
        .header("Accept", "text/event-stream")
        .header("Mcp-Session-Id", "")
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
    let body = response.text().await.expect("Failed to read response body");
    assert_eq!(body, MISSING_SESSION_ID_BODY);
}

#[actix_web::test]
async fn delete_with_empty_session_id_returns_400() {
    let server = TestServer::spawn(true).await;

    let response = server
        .client
        .delete(&server.url)
        .header("Mcp-Session-Id", "")
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
    let body = response.text().await.expect("Failed to read response body");
    assert_eq!(body, MISSING_SESSION_ID_BODY);
}

#[actix_web::test]
async fn stateless_post_with_session_id_header_is_ignored() {
    let server = TestServer::spawn(false).await;

    let initialize_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": {
                "name": "stateless-session-id-test",
                "version": "0.0.0"
            }
        },
        "id": 1
    });

    let response = server
        .client
        .post(&server.url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .header("Mcp-Session-Id", "stale-from-previous-deployment")
        .json(&initialize_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body = response.text().await.expect("Failed to read response body");
    let data = body
        .strip_prefix("data: ")
        .and_then(|rest| rest.split("\n\n").next())
        .expect("body must be a `data:` SSE frame");
    let payload: serde_json::Value =
        serde_json::from_str(data).expect("`data:` payload must be JSON");
    assert_eq!(
        payload["jsonrpc"], "2.0",
        "expected JSON-RPC frame: {payload:?}"
    );
    assert_eq!(payload["id"], 1, "id must echo the initialize request");
    assert!(
        payload["result"]["protocolVersion"].is_string(),
        "expected initialize result with protocolVersion, got: {payload:?}"
    );
}
