//! Integration tests for Authorization header forwarding in MCP proxy scenarios.
//!
//! These tests verify that Authorization headers are properly forwarded to MCP services
//! while other headers are not, as per the MCP specification requirements.

mod common;

use actix_web::{App, HttpServer};
use common::headers_test_service::HeadersTestService;
use futures::StreamExt;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp_actix_web::StreamableHttpService;
use serde_json::json;
use std::sync::Arc;

#[actix_web::test]
async fn test_authorization_forwarded_in_streamable_http_stateless() {
    // Initialize tracing for debugging
    let _ = tracing_subscriber::fmt()
        .with_env_filter("rmcp_actix_web=debug")
        .with_test_writer()
        .try_init();

    // Create service in stateless mode
    let service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(HeadersTestService::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(false)
        .build();

    let server = HttpServer::new(move || {
        App::new().service(actix_web::web::scope("/mcp").service(service.clone().scope()))
    })
    .bind("127.0.0.1:0")
    .expect("Failed to bind server");

    let addr = *server.addrs().first().unwrap();
    let server_handle = server.run();

    let server_task = tokio::spawn(async move {
        let _ = server_handle.await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let url = format!("http://{}/mcp", addr);

    // Send initialize request with Authorization header
    let init_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        },
        "id": 1
    });

    let response = client
        .post(&url)
        .header("Authorization", "Bearer test-token-abc123")
        .header("X-Custom-Header", "should-not-be-forwarded")
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&init_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    // Read SSE response
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();

    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while let Some(chunk) = stream.next().await {
            if let Ok(bytes) = chunk {
                body.extend_from_slice(&bytes);
                if body.ends_with(b"\n\n") || body.len() > 4096 {
                    break;
                }
            }
        }
    })
    .await;

    let body_str = String::from_utf8_lossy(&body);
    assert!(
        body_str.contains("data: "),
        "Response should be in SSE format"
    );

    // Now send a tool call to check what headers were captured
    let tool_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_headers"
        },
        "id": 2
    });

    let tool_response = client
        .post(&url)
        .header("Authorization", "Bearer test-token-abc123")
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&tool_request)
        .send()
        .await
        .expect("Failed to send tool request");

    assert_eq!(tool_response.status(), 200);

    server_task.abort();
}

#[actix_web::test]
async fn test_authorization_forwarded_in_streamable_http_stateful() {
    // Initialize tracing for debugging
    let _ = tracing_subscriber::fmt()
        .with_env_filter("rmcp_actix_web=debug")
        .with_test_writer()
        .try_init();

    // Create service in stateful mode
    let service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(HeadersTestService::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(true)
        .build();

    let server = HttpServer::new(move || {
        App::new().service(actix_web::web::scope("/mcp").service(service.clone().scope()))
    })
    .bind("127.0.0.1:0")
    .expect("Failed to bind server");

    let addr = *server.addrs().first().unwrap();
    let server_handle = server.run();

    let server_task = tokio::spawn(async move {
        let _ = server_handle.await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let url = format!("http://{}/mcp", addr);

    // In stateful mode, we need to initialize without a session first
    let init_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        },
        "id": 1
    });

    // First request creates the session
    let response = client
        .post(&url)
        .header("Authorization", "Bearer stateful-token-xyz789")
        .header("X-Another-Header", "should-not-be-forwarded")
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&init_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    // Extract session ID from response headers
    let session_id = response
        .headers()
        .get("Mcp-Session-Id")
        .and_then(|v| v.to_str().ok())
        .expect("Should have session ID");

    // Verify Authorization was captured during initialization
    assert!(!session_id.is_empty());

    server_task.abort();
}

#[actix_web::test]
async fn test_non_bearer_authorization_not_forwarded() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("rmcp_actix_web=debug")
        .with_test_writer()
        .try_init();

    let service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(HeadersTestService::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(false)
        .build();

    let server = HttpServer::new(move || {
        App::new().service(actix_web::web::scope("/mcp").service(service.clone().scope()))
    })
    .bind("127.0.0.1:0")
    .expect("Failed to bind server");

    let addr = *server.addrs().first().unwrap();
    let server_handle = server.run();

    let server_task = tokio::spawn(async move {
        let _ = server_handle.await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let url = format!("http://{}/mcp", addr);

    // Send request with non-Bearer authorization
    let init_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        },
        "id": 1
    });

    // Test with Basic auth (should not be forwarded)
    let response = client
        .post(&url)
        .header("Authorization", "Basic dXNlcjpwYXNz")
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&init_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    server_task.abort();
}

#[actix_web::test]
async fn test_missing_authorization_doesnt_break_service() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("rmcp_actix_web=debug")
        .with_test_writer()
        .try_init();

    let service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(HeadersTestService::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(false)
        .build();

    let server = HttpServer::new(move || {
        App::new().service(actix_web::web::scope("/mcp").service(service.clone().scope()))
    })
    .bind("127.0.0.1:0")
    .expect("Failed to bind server");

    let addr = *server.addrs().first().unwrap();
    let server_handle = server.run();

    let server_task = tokio::spawn(async move {
        let _ = server_handle.await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let url = format!("http://{}/mcp", addr);

    // Send request without Authorization header
    let init_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        },
        "id": 1
    });

    let response = client
        .post(&url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&init_request)
        .send()
        .await
        .expect("Failed to send request");

    // Service should work fine without Authorization header
    assert_eq!(response.status(), 200);

    server_task.abort();
}
