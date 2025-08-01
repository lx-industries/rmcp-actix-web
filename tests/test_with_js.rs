use rmcp::transport::{
    StreamableHttpServerConfig, streamable_http_server::session::local::LocalSessionManager,
};
use rmcp_actix_web::{SseServer, StreamableHttpService};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
mod common;
use common::calculator::Calculator;

const SSE_BIND_ADDRESS: &str = "127.0.0.1:8000";
const STREAMABLE_HTTP_BIND_ADDRESS: &str = "127.0.0.1:8001";

#[actix_web::test]
async fn test_with_js_client() -> anyhow::Result<()> {
    let _ = tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .try_init();
    tokio::process::Command::new("npm")
        .arg("install")
        .current_dir("tests/test_with_js")
        .spawn()?
        .wait()
        .await?;

    let ct = SseServer::serve(SSE_BIND_ADDRESS.parse()?)
        .await?
        .with_service(Calculator::default);

    let output = tokio::process::Command::new("node")
        .arg("tests/test_with_js/client.js")
        .output()
        .await?;
    assert!(output.status.success());

    // Capture and validate the actual MCP responses
    let stdout = String::from_utf8(output.stdout)?;
    let mut responses: Vec<serde_json::Value> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(serde_json::from_str)
        .collect::<Result<Vec<_>, _>>()?;

    // Sort arrays for deterministic snapshots (preserve_order handles object properties)
    for response in &mut responses {
        if let Some(tools) = response.get_mut("tools").and_then(|t| t.as_array_mut()) {
            tools.sort_by(|a, b| {
                let name_a = a.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let name_b = b.get("name").and_then(|n| n.as_str()).unwrap_or("");
                name_a.cmp(name_b)
            });
        }
    }

    insta::assert_json_snapshot!("js_sse_client_responses", responses);
    ct.cancel();
    Ok(())
}

#[actix_web::test]
async fn test_with_js_streamable_http_client() -> anyhow::Result<()> {
    let _ = tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .try_init();
    tokio::process::Command::new("npm")
        .arg("install")
        .current_dir("tests/test_with_js")
        .spawn()?
        .wait()
        .await?;

    let service = std::sync::Arc::new(
        StreamableHttpService::<Calculator, LocalSessionManager>::new(
            || Ok(Calculator::new()),
            Default::default(),
            StreamableHttpServerConfig {
                stateful_mode: true,
                sse_keep_alive: None,
            },
        ),
    );

    let server = actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .wrap(actix_web::middleware::Logger::default())
            .service(
                actix_web::web::scope("/mcp")
                    .configure(StreamableHttpService::configure(service.clone())),
            )
    })
    .bind(STREAMABLE_HTTP_BIND_ADDRESS)?
    .run();

    let server_handle = server.handle();
    let server_task = tokio::spawn(server);

    // Give the server a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let output = tokio::process::Command::new("node")
        .arg("tests/test_with_js/streamable_client.js")
        .output()
        .await?;
    assert!(output.status.success());

    // Capture and validate the actual MCP responses
    let stdout = String::from_utf8(output.stdout)?;
    let mut responses: Vec<serde_json::Value> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(serde_json::from_str)
        .collect::<Result<Vec<_>, _>>()?;

    // Sort arrays for deterministic snapshots (preserve_order handles object properties)
    for response in &mut responses {
        if let Some(tools) = response.get_mut("tools").and_then(|t| t.as_array_mut()) {
            tools.sort_by(|a, b| {
                let name_a = a.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let name_b = b.get("name").and_then(|n| n.as_str()).unwrap_or("");
                name_a.cmp(name_b)
            });
        }
    }

    insta::assert_json_snapshot!("js_streamable_http_client_responses", responses);

    server_handle.stop(true).await;
    let _ = server_task.await;
    Ok(())
}
