//! Example demonstrating the on_request hook for extension propagation.
//!
//! This example shows how to use the `on_request` hook to propagate typed data
//! from actix-web middleware to MCP service handlers. This is useful for:
//! - Passing decoded JWT claims to MCP handlers
//! - Forwarding user context from authentication middleware
//! - Sharing request metadata across the MCP service layer
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run --example on_request_hook_example
//! ```
//!
//! ## Testing with curl
//!
//! With claims header (simulated middleware):
//! ```bash
//! curl -X POST http://localhost:8080/ \
//!   -H "Content-Type: application/json" \
//!   -H "X-User-Id: alice123" \
//!   -H "X-User-Role: admin" \
//!   -d '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}'
//! ```
//!
//! Then call the whoami tool:
//! ```bash
//! curl -X POST http://localhost:8080/ \
//!   -H "Content-Type: application/json" \
//!   -H "X-User-Id: alice123" \
//!   -H "X-User-Role: admin" \
//!   -H "Mcp-Session-Id: <session-id-from-above>" \
//!   -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"whoami","arguments":{}},"id":2}'
//! ```

use actix_web::{App, HttpMessage, HttpServer, dev::ServiceRequest, middleware};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler, handler::server::router::tool::ToolRouter,
    model::*, service::RequestContext, tool, tool_handler, tool_router,
};
use rmcp_actix_web::transport::StreamableHttpService;
use serde_json::json;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// User claims that would typically come from JWT validation middleware.
/// In this example, we simulate them from custom headers.
#[derive(Clone, Debug)]
pub struct UserClaims {
    pub user_id: String,
    pub role: String,
}

/// Middleware that extracts user claims from headers (simulating JWT middleware).
/// In production, this would decode and validate actual JWT tokens.
fn extract_claims_from_headers(req: &ServiceRequest) {
    let user_id = req
        .headers()
        .get("X-User-Id")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let role = req
        .headers()
        .get("X-User-Role")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .unwrap_or_else(|| "guest".to_string());

    if let Some(user_id) = user_id {
        let claims = UserClaims { user_id, role };
        tracing::info!(?claims, "Extracted user claims from headers");
        req.extensions_mut().insert(claims);
    }
}

/// Custom middleware wrapper for claims extraction
use actix_web::dev::{Service, ServiceResponse, Transform};
use std::future::{Future, Ready, ready};
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct ClaimsExtractor;

impl<S, B> Transform<S, ServiceRequest> for ClaimsExtractor
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type InitError = ();
    type Transform = ClaimsExtractorMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ClaimsExtractorMiddleware { service }))
    }
}

pub struct ClaimsExtractorMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for ClaimsExtractorMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        extract_claims_from_headers(&req);
        Box::pin(self.service.call(req))
    }
}

/// MCP service that uses propagated user claims
#[derive(Clone)]
struct UserAwareService {
    tool_router: ToolRouter<UserAwareService>,
}

#[tool_router]
impl UserAwareService {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    /// Returns information about the current user based on propagated claims.
    #[tool(description = "Get current user info from propagated claims")]
    async fn whoami(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        if let Some(claims) = context.extensions.get::<UserClaims>() {
            let info = json!({
                "user_id": claims.user_id,
                "role": claims.role,
                "message": "Claims successfully propagated from middleware via on_request hook!"
            });
            Ok(CallToolResult::success(vec![Content::text(
                info.to_string(),
            )]))
        } else {
            let info = json!({
                "message": "No user claims found. Try adding X-User-Id header.",
                "hint": "curl -H 'X-User-Id: alice' -H 'X-User-Role: admin' ..."
            });
            Ok(CallToolResult::success(vec![Content::text(
                info.to_string(),
            )]))
        }
    }

    /// Example admin-only operation that checks user role.
    #[tool(description = "Admin-only operation - requires admin role")]
    async fn admin_action(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        if let Some(claims) = context.extensions.get::<UserClaims>() {
            if claims.role == "admin" {
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Admin action executed by user: {}",
                    claims.user_id
                ))]))
            } else {
                Err(McpError::invalid_request(
                    format!(
                        "Access denied. User '{}' has role '{}', but 'admin' is required.",
                        claims.user_id, claims.role
                    ),
                    None,
                ))
            }
        } else {
            Err(McpError::invalid_request(
                "Authentication required. No user claims found.",
                None,
            ))
        }
    }
}

#[tool_handler]
impl ServerHandler for UserAwareService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("User-aware service demonstrating on_request hook".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

const BIND_ADDRESS: &str = "127.0.0.1:8080";

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    println!("\n🚀 on_request Hook Example running at http://{BIND_ADDRESS}");
    println!(
        "\n📖 This example demonstrates propagating typed data from middleware to MCP handlers."
    );
    println!("\n🔧 Available tools:");
    println!("   - whoami: Returns current user info from propagated claims");
    println!("   - admin_action: Admin-only operation (requires role=admin)");
    println!("\n💡 Add headers to simulate authentication:");
    println!("   X-User-Id: <user-id>");
    println!("   X-User-Role: <role>");
    println!("\nPress Ctrl+C to stop the server\n");

    // Create service with on_request hook to propagate UserClaims from middleware
    let http_service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(UserAwareService::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(true)
        // The key feature: propagate UserClaims from actix-web to MCP handlers
        .on_request_fn(|http_req, mcp_ext| {
            // Copy UserClaims from HTTP request extensions to MCP extensions
            if let Some(claims) = http_req.extensions().get::<UserClaims>() {
                tracing::debug!(?claims, "Propagating claims to MCP context");
                mcp_ext.insert(claims.clone());
            }
        })
        .build();

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            // Middleware that extracts claims from headers (simulating JWT middleware)
            .wrap(ClaimsExtractor)
            .service(http_service.clone().scope())
    })
    .bind(BIND_ADDRESS)?
    .run()
    .await?;

    Ok(())
}
