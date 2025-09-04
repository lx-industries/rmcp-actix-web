//! Test service for verifying Authorization header forwarding.
//!
//! This service is used in tests to verify that Authorization headers sent by clients
//! are properly stored in RequestContext.extensions and accessible to MCP services.

#![allow(dead_code)]
use std::sync::Arc;

use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler, handler::server::router::tool::ToolRouter,
    model::*, service::RequestContext, tool, tool_handler, tool_router,
};
use rmcp_actix_web::transport::AuthorizationHeader;
use serde_json::json;
use tokio::sync::Mutex;

/// Test service that captures and returns the Authorization header.
///
/// This service is used to verify that Authorization headers are properly
/// passed through the transport layer and accessible via RequestContext.
#[derive(Clone)]
pub struct HeadersTestService {
    /// Stores Authorization header received during initialization
    captured_authorization: Arc<Mutex<Option<String>>>,
    /// Stores Authorization header from the most recent tool call
    last_tool_authorization: Arc<Mutex<Option<String>>>,
    /// Router for tool dispatch
    tool_router: ToolRouter<HeadersTestService>,
}

#[tool_router]
impl HeadersTestService {
    pub fn new() -> Self {
        Self {
            captured_authorization: Arc::new(Mutex::new(None)),
            last_tool_authorization: Arc::new(Mutex::new(None)),
            tool_router: Self::tool_router(),
        }
    }

    /// Returns the captured Authorization header
    #[tool(description = "Get the Authorization header that was captured during initialization")]
    async fn get_headers(&self) -> Result<CallToolResult, McpError> {
        let auth = self.captured_authorization.lock().await;

        if let Some(auth_value) = auth.as_ref() {
            Ok(CallToolResult::success(vec![Content::text(
                json!({
                    "authorization": auth_value
                })
                .to_string(),
            )]))
        } else {
            Ok(CallToolResult::success(vec![Content::text(
                "No Authorization header captured",
            )]))
        }
    }

    /// Returns the Authorization header from the current request
    #[tool(description = "Get the Authorization header from the current tool call request")]
    async fn get_current_auth(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        // Extract Authorization header from the current request's context
        if let Some(auth) = context.extensions.get::<AuthorizationHeader>() {
            // Store it for verification
            let mut last_auth = self.last_tool_authorization.lock().await;
            *last_auth = Some(auth.0.clone());

            Ok(CallToolResult::success(vec![Content::text(
                json!({
                    "authorization": auth.0
                })
                .to_string(),
            )]))
        } else {
            Ok(CallToolResult::success(vec![Content::text(
                json!({
                    "authorization": null,
                    "message": "No Authorization header in current request"
                })
                .to_string(),
            )]))
        }
    }

    /// Test tool to verify the service is working
    #[tool(description = "Simple echo test")]
    fn echo(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text("echo test")]))
    }
}

#[tool_handler]
impl ServerHandler for HeadersTestService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Test service for verifying Authorization header forwarding".to_string(),
            ),
        }
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        // Try to extract Authorization header from RequestContext extensions
        if let Some(auth) = context.extensions.get::<AuthorizationHeader>() {
            let mut captured = self.captured_authorization.lock().await;
            *captured = Some(auth.0.clone());
            tracing::info!(
                "Captured Authorization header during initialization: {}",
                auth.0
            );
        } else {
            tracing::info!("No Authorization header found in RequestContext extensions");
        }

        Ok(self.get_info())
    }
}
