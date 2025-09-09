//! Example demonstrating Authorization header forwarding for MCP proxy scenarios.
//!
//! This example shows how an MCP service can access Authorization headers sent by
//! clients and use them to proxy requests to backend APIs.
//!
//! Run with:
//! ```bash
//! cargo run --example authorization_proxy_example
//! ```
//!
//! Then test with curl:
//! ```bash
//! # Initialize with Bearer token
//! curl -X POST http://localhost:8080/mcp \
//!   -H "Content-Type: application/json" \
//!   -H "Accept: application/json, text/event-stream" \
//!   -H "Authorization: Bearer your-api-token-here" \
//!   -d '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}'
//!
//! # Call a tool that uses the token to proxy to a backend API
//! curl -X POST http://localhost:8080/mcp \
//!   -H "Content-Type: application/json" \
//!   -H "Accept: application/json, text/event-stream" \
//!   -H "Authorization: Bearer your-api-token-here" \
//!   -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"fetch_user_data"},"id":2}'
//! ```

use actix_web::{App, HttpServer};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler, handler::server::router::tool::ToolRouter,
    model::*, service::RequestContext, tool, tool_handler, tool_router,
};
use rmcp_actix_web::transport::{AuthorizationHeader, StreamableHttpService};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Example MCP service that acts as a proxy to backend APIs.
///
/// This service demonstrates how to:
/// 1. Capture Authorization headers from MCP clients
/// 2. Use those headers to authenticate with backend APIs
/// 3. Return data from backend APIs to MCP clients
#[derive(Clone)]
struct ProxyService {
    /// Stores the Authorization header for the session
    authorization: Arc<Mutex<Option<String>>>,
    /// Router for tool dispatch
    tool_router: ToolRouter<ProxyService>,
}

#[tool_router]
impl ProxyService {
    fn new() -> Self {
        Self {
            authorization: Arc::new(Mutex::new(None)),
            tool_router: Self::tool_router(),
        }
    }

    /// Simulates fetching user data from a backend API using the stored token
    #[tool(description = "Fetch user data from backend API using the provided authorization token")]
    async fn fetch_user_data(&self) -> Result<CallToolResult, McpError> {
        let auth = self.authorization.lock().await;

        if let Some(auth_header) = auth.as_ref() {
            // In a real implementation, you would use this token to call a backend API
            // For example:
            // let client = reqwest::Client::new();
            // let response = client.get("https://api.example.com/user")
            //     .header("Authorization", auth_header)
            //     .send()
            //     .await?;

            // For this example, we'll simulate a response
            let simulated_response = json!({
                "user_id": "12345",
                "name": "John Doe",
                "email": "john.doe@example.com",
                "auth_used": auth_header,
                "note": "This is simulated data. In production, this would come from your backend API."
            });

            Ok(CallToolResult::success(vec![Content::text(
                simulated_response.to_string(),
            )]))
        } else {
            Err(McpError::invalid_request(
                "No authorization token available. Please provide Authorization header.",
                None,
            ))
        }
    }

    /// Simulates posting data to a backend API
    #[tool(description = "Post data to backend API using the provided authorization token")]
    async fn post_to_backend(&self) -> Result<CallToolResult, McpError> {
        let auth = self.authorization.lock().await;

        if let Some(auth_header) = auth.as_ref() {
            // Simulate posting to backend
            let result = json!({
                "status": "success",
                "message": "Data posted successfully",
                "auth_used": auth_header,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });

            Ok(CallToolResult::success(vec![Content::text(
                result.to_string(),
            )]))
        } else {
            Err(McpError::invalid_request(
                "No authorization token available",
                None,
            ))
        }
    }

    /// Check if authorization is available
    #[tool(description = "Check if authorization token is available")]
    async fn check_auth(&self) -> Result<CallToolResult, McpError> {
        let auth = self.authorization.lock().await;

        let status = if auth.is_some() {
            json!({
                "authenticated": true,
                "token_type": "Bearer",
                "note": "Token is available for backend API calls"
            })
        } else {
            json!({
                "authenticated": false,
                "note": "No Authorization header provided"
            })
        };

        Ok(CallToolResult::success(vec![Content::text(
            status.to_string(),
        )]))
    }
}

#[tool_handler]
impl ServerHandler for ProxyService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "MCP proxy service that forwards Authorization headers to backend APIs. \
                 Provide a Bearer token in the Authorization header to authenticate."
                    .to_string(),
            ),
        }
    }

    async fn initialize(
        &self,
        request: InitializeRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        // Store peer info
        if context.peer.peer_info().is_none() {
            context.peer.set_peer_info(request);
        }

        // Extract and store Authorization header if present
        if let Some(auth) = context.extensions.get::<AuthorizationHeader>() {
            let mut stored_auth = self.authorization.lock().await;
            *stored_auth = Some(auth.0.clone());
            println!("✓ Authorization header captured: {}", auth.0);
            tracing::info!("Authorization header stored for proxy use: {}", auth.0);
        } else {
            println!("ℹ No Authorization header provided");
            tracing::info!("No Authorization header found - proxy calls will fail");
        }

        Ok(self.get_info())
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("rmcp_actix_web=info".parse().unwrap())
                .add_directive("authorization_proxy_example=info".parse().unwrap()),
        )
        .init();

    println!("🚀 Starting Authorization Proxy Example Server");
    println!("   Server: http://localhost:8080/mcp");
    println!();
    println!("📝 Test with curl:");
    println!();
    println!("1. Initialize with Bearer token:");
    println!("   curl -X POST http://localhost:8080/mcp \\");
    println!("     -H \"Content-Type: application/json\" \\");
    println!("     -H \"Accept: application/json, text/event-stream\" \\");
    println!("     -H \"Authorization: Bearer your-api-token-here\" \\");
    println!(
        "     -d '{{\"jsonrpc\":\"2.0\",\"method\":\"initialize\",\"params\":{{\"protocolVersion\":\"2024-11-05\",\"capabilities\":{{}},\"clientInfo\":{{\"name\":\"test\",\"version\":\"1.0\"}}}},\"id\":1}}'"
    );
    println!();
    println!("2. Check authentication status:");
    println!("   curl -X POST http://localhost:8080/mcp \\");
    println!("     -H \"Content-Type: application/json\" \\");
    println!("     -H \"Accept: application/json, text/event-stream\" \\");
    println!("     -H \"Authorization: Bearer your-api-token-here\" \\");
    println!(
        "     -d '{{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"params\":{{\"name\":\"check_auth\"}},\"id\":2}}'"
    );
    println!();
    println!("3. Fetch user data (simulated backend call):");
    println!("   curl -X POST http://localhost:8080/mcp \\");
    println!("     -H \"Content-Type: application/json\" \\");
    println!("     -H \"Accept: application/json, text/event-stream\" \\");
    println!("     -H \"Authorization: Bearer your-api-token-here\" \\");
    println!(
        "     -d '{{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"params\":{{\"name\":\"fetch_user_data\"}},\"id\":3}}'"
    );
    println!();
    println!(
        "ℹ️  Note: Only Bearer tokens are forwarded. Basic auth and other schemes are ignored."
    );
    println!();

    // Create the service in stateless mode for simplicity
    let service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(ProxyService::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(false)
        .build();

    HttpServer::new(move || {
        App::new().service(actix_web::web::scope("/mcp").service(service.clone().scope()))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
