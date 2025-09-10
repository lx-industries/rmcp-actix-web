//! Example demonstrating Authorization header forwarding with SSE transport.
//!
//! This example requires the `transport-sse` feature to be enabled.
//!
//! **DEPRECATED**: The SSE transport is deprecated in favor of StreamableHttp transport.
//! Please see `authorization_proxy_example.rs` for the recommended approach using StreamableHttp.
//!
//! This example shows how Authorization headers sent with POST requests to the
//! message endpoint are forwarded to MCP services, enabling proxy scenarios
//! where different tools can use different authentication tokens.
//!
//! Run with:
//! ```bash
//! cargo run --example authorization_proxy_sse_example
//! ```
//!
//! Then test with curl:
//! ```bash
//! # 1. Connect to SSE endpoint and get the message URL
//! curl -N http://localhost:8080/sse
//! # Note the endpoint URL from the first event (e.g., /message?sessionId=xxx)
//!
//! # 2. Send initialize (can include Authorization if needed)
//! curl -X POST "http://localhost:8080/message?sessionId=xxx" \
//!   -H "Content-Type: application/json" \
//!   -H "Authorization: Bearer init-token" \
//!   -d '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}'
//!
//! # 3. Call a tool that needs authentication
//! curl -X POST "http://localhost:8080/message?sessionId=xxx" \
//!   -H "Content-Type: application/json" \
//!   -H "Authorization: Bearer api-token-12345" \
//!   -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"call_api"},"id":2}'
//!
//! # 4. Call a public tool (no auth needed)
//! curl -X POST "http://localhost:8080/message?sessionId=xxx" \
//!   -H "Content-Type: application/json" \
//!   -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"public_api"},"id":3}'
//!
//! # 5. Call a different API with different token
//! curl -X POST "http://localhost:8080/message?sessionId=xxx" \
//!   -H "Content-Type: application/json" \
//!   -H "Authorization: Bearer github-token-xyz" \
//!   -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"github_api"},"id":4}'
//! ```

#[cfg(feature = "transport-sse")]
mod sse_example {
    #![allow(deprecated)]

    use actix_web::{App, HttpServer};
    use rmcp::{
        ErrorData as McpError, RoleServer, ServerHandler,
        handler::server::router::tool::ToolRouter, model::*, service::RequestContext, tool,
        tool_handler, tool_router,
    };
    use rmcp_actix_web::transport::{AuthorizationHeader, SseService};
    use serde_json::json;
    use std::sync::Arc;

    /// SSE proxy service that demonstrates per-request Authorization.
    ///
    /// Unlike the StreamableHttp example which stores Authorization at initialize,
    /// this example shows the more realistic proxy pattern where each tool call
    /// can have its own Authorization token for different backend APIs.
    #[derive(Clone)]
    struct SseProxyService {
        tool_router: ToolRouter<SseProxyService>,
    }

    #[tool_router]
    impl SseProxyService {
        fn new() -> Self {
            Self {
                tool_router: Self::tool_router(),
            }
        }

        /// Make an authenticated API call using the Authorization from THIS request
        #[tool(description = "Make authenticated API call using provided token")]
        async fn call_api(
            &self,
            context: RequestContext<RoleServer>,
        ) -> Result<CallToolResult, McpError> {
            // Get Authorization for THIS specific request
            if let Some(auth) = context.extensions.get::<AuthorizationHeader>() {
                // In production, you would use this token to call a real API:
                // let client = reqwest::Client::new();
                // let response = client.get("https://api.example.com/data")
                //     .header("Authorization", &auth.0)
                //     .send()
                //     .await?;

                let result = json!({
                    "success": true,
                    "auth_used": auth.0,
                    "api": "generic-api",
                    "note": "Would call backend API with this token"
                });

                Ok(CallToolResult::success(vec![Content::text(
                    result.to_string(),
                )]))
            } else {
                Err(McpError::invalid_request(
                    "No authorization token provided for this request. Include Authorization: Bearer <token>",
                    None,
                ))
            }
        }

        /// Call GitHub API with the provided token
        #[tool(description = "Call GitHub API using provided GitHub token")]
        async fn github_api(
            &self,
            context: RequestContext<RoleServer>,
        ) -> Result<CallToolResult, McpError> {
            if let Some(auth) = context.extensions.get::<AuthorizationHeader>() {
                // This would call GitHub API with the provided token
                let result = json!({
                    "success": true,
                    "auth_used": auth.0,
                    "api": "github",
                    "note": "Would fetch GitHub data with this token"
                });

                Ok(CallToolResult::success(vec![Content::text(
                    result.to_string(),
                )]))
            } else {
                Err(McpError::invalid_request(
                    "GitHub token required. Include Authorization: Bearer <github-token>",
                    None,
                ))
            }
        }

        /// Public API call that doesn't need authentication
        #[tool(description = "Public API call that doesn't need auth")]
        async fn public_api(
            &self,
            _context: RequestContext<RoleServer>,
        ) -> Result<CallToolResult, McpError> {
            // No auth needed - this could call a public API
            let result = json!({
                "success": true,
                "data": "Public data accessible without authentication",
                "api": "public",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });

            Ok(CallToolResult::success(vec![Content::text(
                result.to_string(),
            )]))
        }

        /// Check what authorization was provided with this request
        #[tool(description = "Check authorization token for this request")]
        async fn check_request_auth(
            &self,
            context: RequestContext<RoleServer>,
        ) -> Result<CallToolResult, McpError> {
            let status = if let Some(auth) = context.extensions.get::<AuthorizationHeader>() {
                json!({
                    "has_auth": true,
                    "auth_header": auth.0,
                    "type": "Bearer",
                    "note": "This authorization is for THIS request only"
                })
            } else {
                json!({
                    "has_auth": false,
                    "note": "No Authorization header in this request"
                })
            };

            Ok(CallToolResult::success(vec![Content::text(
                status.to_string(),
            )]))
        }
    }

    #[tool_handler]
    impl ServerHandler for SseProxyService {
        fn get_info(&self) -> ServerInfo {
            ServerInfo {
                protocol_version: ProtocolVersion::V_2024_11_05,
                capabilities: ServerCapabilities::builder().enable_tools().build(),
                server_info: Implementation::from_build_env(),
                instructions: Some(
                    "SSE proxy service demonstrating per-request Authorization. \
                     Each tool call can include its own Bearer token for different APIs. \
                     Some tools require auth, others don't."
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

            // Note: We don't store Authorization here because in SSE,
            // each request can have its own Authorization header.
            // This is different from the StreamableHttp example.

            if let Some(auth) = context.extensions.get::<AuthorizationHeader>() {
                tracing::info!("Initialize request included Authorization: {}", auth.0);
            } else {
                tracing::info!("Initialize request without Authorization (normal for SSE)");
            }

            Ok(self.get_info())
        }
    }

    pub async fn run() -> std::io::Result<()> {
        // Initialize logging
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive("rmcp_actix_web=info".parse().unwrap())
                    .add_directive("authorization_proxy_sse_example=info".parse().unwrap()),
            )
            .init();

        println!("🚀 Starting SSE Authorization Proxy Example Server");
        println!("   Server: http://localhost:8080");
        println!();
        println!("📝 Test flow:");
        println!();
        println!("1. Connect to SSE endpoint:");
        println!("   curl -N http://localhost:8080/sse");
        println!("   (Note the endpoint URL from the first event)");
        println!();
        println!("2. Initialize (auth optional):");
        println!("   curl -X POST \"http://localhost:8080/message?sessionId=YOUR_SESSION_ID\" \\");
        println!("     -H \"Content-Type: application/json\" \\");
        println!(
            "     -d '{{\"jsonrpc\":\"2.0\",\"method\":\"initialize\",\"params\":{{\"protocolVersion\":\"2024-11-05\",\"capabilities\":{{}},\"clientInfo\":{{\"name\":\"test\",\"version\":\"1.0\"}}}},\"id\":1}}'"
        );
        println!();
        println!("3. Call API with token:");
        println!("   curl -X POST \"http://localhost:8080/message?sessionId=YOUR_SESSION_ID\" \\");
        println!("     -H \"Content-Type: application/json\" \\");
        println!("     -H \"Authorization: Bearer your-api-token\" \\");
        println!(
            "     -d '{{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"params\":{{\"name\":\"call_api\"}},\"id\":2}}'"
        );
        println!();
        println!("4. Call public API (no auth):");
        println!("   curl -X POST \"http://localhost:8080/message?sessionId=YOUR_SESSION_ID\" \\");
        println!("     -H \"Content-Type: application/json\" \\");
        println!(
            "     -d '{{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"params\":{{\"name\":\"public_api\"}},\"id\":3}}'"
        );
        println!();
        println!("5. Check request auth:");
        println!("   curl -X POST \"http://localhost:8080/message?sessionId=YOUR_SESSION_ID\" \\");
        println!("     -H \"Content-Type: application/json\" \\");
        println!("     -H \"Authorization: Bearer check-this-token\" \\");
        println!(
            "     -d '{{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"params\":{{\"name\":\"check_request_auth\"}},\"id\":4}}'"
        );
        println!();
        println!("ℹ️  Key points:");
        println!("   - Each request can have different (or no) Authorization");
        println!("   - Only Bearer tokens are forwarded");
        println!("   - Perfect for proxy scenarios with multiple backends");
        println!();

        let service = SseService::builder()
            .service_factory(Arc::new(|| Ok(SseProxyService::new())))
            .build();

        HttpServer::new(move || App::new().service(service.clone().scope()))
            .bind("127.0.0.1:8080")?
            .run()
            .await
    }
}

#[cfg(feature = "transport-sse")]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    sse_example::run().await
}

#[cfg(not(feature = "transport-sse"))]
fn main() {
    eprintln!("This example requires the 'transport-sse' feature to be enabled.");
    eprintln!(
        "Run with: cargo run --example authorization_proxy_sse_example --features transport-sse"
    );
    std::process::exit(1);
}
