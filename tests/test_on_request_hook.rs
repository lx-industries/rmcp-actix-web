// tests/test_on_request_hook.rs
//! Integration tests for the on_request hook functionality.
//!
//! These tests verify that the on_request hook is invoked in all three code paths:
//! - Stateless mode
//! - Stateful mode with existing session
//! - Stateful mode with new session (initialization)

mod common;

use actix_web::{App, HttpMessage, HttpRequest, HttpServer, dev::Service};
use futures::StreamExt;
use reqwest::Response;
use rmcp::model::Extensions;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp_actix_web::transport::StreamableHttpService;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

/// Custom extension type for testing - simulates JWT claims from auth middleware
#[derive(Clone, Debug, PartialEq)]
pub struct TestClaims {
    pub user_id: String,
    pub role: String,
}

/// Test service that can report what extensions it received
mod extension_test_service {
    use rmcp::{
        ErrorData as McpError, RoleServer, ServerHandler,
        handler::server::router::tool::ToolRouter, model::*, service::RequestContext, tool,
        tool_handler, tool_router,
    };
    use serde_json::json;

    use super::TestClaims;

    #[derive(Clone)]
    pub struct ExtensionTestService {
        tool_router: ToolRouter<ExtensionTestService>,
    }

    #[tool_router]
    impl ExtensionTestService {
        pub fn new() -> Self {
            Self {
                tool_router: Self::tool_router(),
            }
        }

        /// Returns claims from the request context if present
        #[tool(description = "Get claims from request context")]
        async fn get_claims(
            &self,
            context: RequestContext<RoleServer>,
        ) -> Result<CallToolResult, McpError> {
            let claims = context.extensions.get::<TestClaims>().cloned();

            let result = if let Some(c) = claims {
                json!({ "user_id": c.user_id, "role": c.role })
            } else {
                json!({ "claims": null })
            };

            Ok(CallToolResult::success(vec![Content::text(
                result.to_string(),
            )]))
        }

        /// Simple echo test to verify service is working
        #[tool(description = "Simple echo test")]
        fn echo(&self) -> Result<CallToolResult, McpError> {
            Ok(CallToolResult::success(vec![Content::text("echo")]))
        }
    }

    #[tool_handler]
    impl ServerHandler for ExtensionTestService {
        fn get_info(&self) -> ServerInfo {
            ServerInfo {
                protocol_version: ProtocolVersion::V_2024_11_05,
                capabilities: ServerCapabilities::builder().enable_tools().build(),
                server_info: Implementation::from_build_env(),
                instructions: None,
            }
        }

        async fn initialize(
            &self,
            _request: InitializeRequestParam,
            context: RequestContext<RoleServer>,
        ) -> Result<InitializeResult, McpError> {
            // Log whether claims were received during initialization
            if let Some(claims) = context.extensions.get::<TestClaims>() {
                tracing::info!(
                    "Received claims during initialization: user_id={}, role={}",
                    claims.user_id,
                    claims.role
                );
            } else {
                tracing::info!("No claims received during initialization");
            }
            Ok(self.get_info())
        }
    }
}

use extension_test_service::ExtensionTestService;

/// Helper function to extract claims from SSE response
async fn extract_claims_from_sse_response(response: Response) -> Option<Value> {
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();

    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(Ok(bytes)) = stream.next().await {
            body.extend_from_slice(&bytes);
            if body.ends_with(b"\n\n") || body.len() > 4096 {
                break;
            }
        }
    })
    .await;

    // Parse SSE and extract claims
    let body_str = String::from_utf8_lossy(&body);
    for line in body_str.lines() {
        if let Some(json_str) = line.strip_prefix("data: ")
            && let Ok(response_json) = serde_json::from_str::<Value>(json_str)
            && let Some(text_value) = response_json.pointer("/result/content/0/text")
            && let Some(text_str) = text_value.as_str()
            && let Ok(claims_response) = serde_json::from_str::<Value>(text_str)
        {
            return Some(claims_response);
        }
    }
    None
}

/// Test that on_request hook is called in stateless mode
#[actix_web::test]
async fn test_on_request_hook_stateless_mode() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("rmcp_actix_web=debug,test_on_request_hook=debug")
        .with_test_writer()
        .try_init();

    // Create service with on_request hook that propagates TestClaims
    let service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(ExtensionTestService::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(false)
        .on_request(Arc::new(|http_req: &HttpRequest, ext: &mut Extensions| {
            // Propagate TestClaims from HttpRequest to rmcp Extensions
            if let Some(claims) = http_req.extensions().get::<TestClaims>() {
                tracing::debug!(
                    "on_request hook: propagating claims for user {}",
                    claims.user_id
                );
                ext.insert(claims.clone());
            }
        }))
        .build();

    let server = HttpServer::new(move || {
        App::new()
            // Middleware that adds claims to every request
            .wrap_fn(|req, srv| {
                // Simulate auth middleware adding claims
                req.extensions_mut().insert(TestClaims {
                    user_id: "stateless-user-123".to_string(),
                    role: "admin".to_string(),
                });
                srv.call(req)
            })
            .service(actix_web::web::scope("/mcp").service(service.clone().scope()))
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
    let url = format!("http://{}/mcp", addr);

    // Send initialize request first (stateless mode processes each request independently)
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
        .expect("Failed to send init request");

    assert_eq!(response.status(), 200);

    // Consume the init response
    let mut stream = response.bytes_stream();
    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(Ok(_)) = stream.next().await {}
    })
    .await;

    // Now call get_claims tool to verify hook was invoked
    let tool_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_claims"
        },
        "id": 2
    });

    let tool_response = client
        .post(&url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&tool_request)
        .send()
        .await
        .expect("Failed to send tool request");

    assert_eq!(tool_response.status(), 200);

    let claims = extract_claims_from_sse_response(tool_response).await;
    assert!(claims.is_some(), "Should have received claims response");

    let claims = claims.unwrap();
    assert_eq!(
        claims.get("user_id").and_then(|v| v.as_str()),
        Some("stateless-user-123"),
        "User ID should be propagated via on_request hook in stateless mode"
    );
    assert_eq!(
        claims.get("role").and_then(|v| v.as_str()),
        Some("admin"),
        "Role should be propagated via on_request hook in stateless mode"
    );

    server_task.abort();
}

/// Test that on_request hook is called for existing sessions in stateful mode
#[actix_web::test]
async fn test_on_request_hook_stateful_mode_existing_session() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("rmcp_actix_web=debug,test_on_request_hook=debug")
        .with_test_writer()
        .try_init();

    // Create service with on_request hook
    let service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(ExtensionTestService::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(true)
        .on_request(Arc::new(|http_req: &HttpRequest, ext: &mut Extensions| {
            if let Some(claims) = http_req.extensions().get::<TestClaims>() {
                tracing::debug!(
                    "on_request hook: propagating claims for user {}",
                    claims.user_id
                );
                ext.insert(claims.clone());
            }
        }))
        .build();

    let server = HttpServer::new(move || {
        App::new()
            .wrap_fn(|req, srv| {
                // Use different claims for existing session test
                req.extensions_mut().insert(TestClaims {
                    user_id: "existing-session-user".to_string(),
                    role: "editor".to_string(),
                });
                srv.call(req)
            })
            .service(actix_web::web::scope("/mcp").service(service.clone().scope()))
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
    let url = format!("http://{}/mcp", addr);

    // Step 1: Initialize session
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
        .expect("Failed to send init request");

    assert_eq!(response.status(), 200);

    // Extract session ID
    let session_id = response
        .headers()
        .get("Mcp-Session-Id")
        .and_then(|v| v.to_str().ok())
        .expect("Should have session ID")
        .to_string();

    // Consume the init response
    let mut stream = response.bytes_stream();
    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(Ok(_)) = stream.next().await {}
    })
    .await;

    // Step 2: Send initialized notification
    let initialized_notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });

    let notif_response = client
        .post(&url)
        .header("Mcp-Session-Id", &session_id)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&initialized_notification)
        .send()
        .await
        .expect("Failed to send initialized notification");

    assert_eq!(notif_response.status(), 202);

    // Step 3: Call tool on existing session - this tests Task 4
    let tool_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_claims"
        },
        "id": 2
    });

    let tool_response = client
        .post(&url)
        .header("Mcp-Session-Id", &session_id)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&tool_request)
        .send()
        .await
        .expect("Failed to send tool request");

    assert_eq!(tool_response.status(), 200);

    let claims = extract_claims_from_sse_response(tool_response).await;
    assert!(claims.is_some(), "Should have received claims response");

    let claims = claims.unwrap();
    assert_eq!(
        claims.get("user_id").and_then(|v| v.as_str()),
        Some("existing-session-user"),
        "User ID should be propagated via on_request hook for existing session"
    );
    assert_eq!(
        claims.get("role").and_then(|v| v.as_str()),
        Some("editor"),
        "Role should be propagated via on_request hook for existing session"
    );

    server_task.abort();
}

/// Test that on_request hook is called during new session initialization in stateful mode
#[actix_web::test]
async fn test_on_request_hook_stateful_mode_new_session() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("rmcp_actix_web=debug,test_on_request_hook=debug")
        .with_test_writer()
        .try_init();

    // Use an Arc<Mutex> to capture whether the hook was called during initialization
    let hook_called = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let hook_called_clone = hook_called.clone();

    // Create service with on_request hook
    let service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(ExtensionTestService::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(true)
        .on_request(Arc::new(
            move |http_req: &HttpRequest, ext: &mut Extensions| {
                if let Some(claims) = http_req.extensions().get::<TestClaims>() {
                    tracing::debug!(
                        "on_request hook: propagating claims for user {} during session init",
                        claims.user_id
                    );
                    hook_called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                    ext.insert(claims.clone());
                }
            },
        ))
        .build();

    let server = HttpServer::new(move || {
        App::new()
            .wrap_fn(|req, srv| {
                req.extensions_mut().insert(TestClaims {
                    user_id: "new-session-user".to_string(),
                    role: "viewer".to_string(),
                });
                srv.call(req)
            })
            .service(actix_web::web::scope("/mcp").service(service.clone().scope()))
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
    let url = format!("http://{}/mcp", addr);

    // Send initialize request - this creates a new session and tests Task 5
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
        .expect("Failed to send init request");

    assert_eq!(response.status(), 200);

    // Verify session was created
    let session_id = response
        .headers()
        .get("Mcp-Session-Id")
        .and_then(|v| v.to_str().ok())
        .expect("Should have session ID");

    assert!(!session_id.is_empty(), "Session ID should not be empty");

    // Consume the init response
    let mut stream = response.bytes_stream();
    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(Ok(_)) = stream.next().await {}
    })
    .await;

    // Verify the hook was called
    assert!(
        hook_called.load(std::sync::atomic::Ordering::SeqCst),
        "on_request hook should have been called during new session initialization"
    );

    server_task.abort();
}

/// Test that on_request hook works without panicking when no claims are present
#[actix_web::test]
async fn test_on_request_hook_no_claims() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("rmcp_actix_web=debug")
        .with_test_writer()
        .try_init();

    // Create service with on_request hook that won't find any claims
    let service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(ExtensionTestService::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(false)
        .on_request(Arc::new(|http_req: &HttpRequest, ext: &mut Extensions| {
            // This will not find claims since middleware doesn't add them
            if let Some(claims) = http_req.extensions().get::<TestClaims>() {
                ext.insert(claims.clone());
            }
        }))
        .build();

    let server = HttpServer::new(move || {
        // No middleware adding claims
        App::new().service(actix_web::web::scope("/mcp").service(service.clone().scope()))
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
    let url = format!("http://{}/mcp", addr);

    // Initialize
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
        .expect("Failed to send init request");

    assert_eq!(response.status(), 200);

    // Consume the init response
    let mut stream = response.bytes_stream();
    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(Ok(_)) = stream.next().await {}
    })
    .await;

    // Call get_claims - should return null claims
    let tool_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_claims"
        },
        "id": 2
    });

    let tool_response = client
        .post(&url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&tool_request)
        .send()
        .await
        .expect("Failed to send tool request");

    assert_eq!(tool_response.status(), 200);

    let claims = extract_claims_from_sse_response(tool_response).await;
    assert!(claims.is_some(), "Should have received response");

    let claims = claims.unwrap();
    // Should have null claims since no middleware added them
    assert!(
        claims.get("claims").is_some()
            || (claims.get("user_id").is_none() && claims.get("role").is_none()),
        "Should indicate no claims were found"
    );

    server_task.abort();
}

/// Test that service works without on_request hook configured
#[actix_web::test]
async fn test_service_without_on_request_hook() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("rmcp_actix_web=debug")
        .with_test_writer()
        .try_init();

    // Create service WITHOUT on_request hook
    let service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(ExtensionTestService::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(false)
        .build();

    let server = HttpServer::new(move || {
        App::new()
            .wrap_fn(|req, srv| {
                // Even with claims in HttpRequest, they won't be propagated without hook
                req.extensions_mut().insert(TestClaims {
                    user_id: "ignored-user".to_string(),
                    role: "ignored".to_string(),
                });
                srv.call(req)
            })
            .service(actix_web::web::scope("/mcp").service(service.clone().scope()))
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
    let url = format!("http://{}/mcp", addr);

    // Initialize
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
        .expect("Failed to send init request");

    assert_eq!(
        response.status(),
        200,
        "Service should work without on_request hook"
    );

    // Consume the init response
    let mut stream = response.bytes_stream();
    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(Ok(_)) = stream.next().await {}
    })
    .await;

    // Call tool - should work but not have claims
    let tool_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_claims"
        },
        "id": 2
    });

    let tool_response = client
        .post(&url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&tool_request)
        .send()
        .await
        .expect("Failed to send tool request");

    assert_eq!(tool_response.status(), 200);

    let claims = extract_claims_from_sse_response(tool_response).await;
    assert!(claims.is_some(), "Should have received response");

    let claims = claims.unwrap();
    // Should NOT have the claims since no hook was configured
    assert!(
        claims.get("user_id").is_none()
            || claims.get("claims").map(|v| v.is_null()).unwrap_or(false),
        "Claims should not be propagated without on_request hook"
    );

    server_task.abort();
}

/// Test that the builder accepts a closure directly (not requiring Arc)
/// via the on_request_fn convenience method.
#[actix_web::test]
async fn test_on_request_builder_ergonomics() {
    // Test that the builder accepts a closure directly (not requiring Arc)
    let _service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(ExtensionTestService::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .on_request_fn(|_req, _ext| {
            // Simple closure should work without Arc wrapping
        })
        .build();

    // If this compiles, the test passes
}
