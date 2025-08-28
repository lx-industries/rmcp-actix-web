# rmcp-actix-web

actix-web transport implementations for RMCP (Rust Model Context Protocol)

This crate provides actix-web-based transport implementations for the Model Context Protocol, offering a complete alternative to the default Axum-based transports in the main RMCP crate.

## Overview

`rmcp-actix-web` provides:
- **SSE (Server-Sent Events) transport**: Real-time, unidirectional communication
- **Streamable HTTP transport**: Bidirectional communication with session management
- **Framework-level composition**: Mount MCP services at custom paths using actix-web Scope
- **Full MCP compatibility**: Implements the complete MCP protocol specification
- **RMCP ecosystem alignment**: APIs that follow RMCP patterns for maximum consistency

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rmcp-actix-web = "0.2"
rmcp = "0.3"
actix-web = "4"
```

### Feature Flags

Control which transports are compiled:

```toml
# Default: both transports enabled
rmcp-actix-web = "0.2"

# Only SSE transport
rmcp-actix-web = { version = "0.2", default-features = false, features = ["transport-sse-server"] }

# Only StreamableHttp transport
rmcp-actix-web = { version = "0.2", default-features = false, features = ["transport-streamable-http-server"] }
```

## Compatibility Matrix

| rmcp-actix-web | rmcp |
|----------------|------|
| 0.2.2          | 0.3.0|
| 0.2.x          | 0.2.x|
| 0.1.x          | 0.2.x|

## Quick Start

### Simple SSE Server

```rust
use rmcp_actix_web::SseService;
use actix_web::{App, HttpServer};
use std::sync::Arc;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sse_service = SseService::builder()
        .service_factory(Arc::new(|| Ok(MyMcpService::new())))
        .build();

    HttpServer::new(move || {
        App::new()
            .service(sse_service.clone().scope())
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await?;
    
    Ok(())
}
```

### Framework-Level Composition

Mount MCP services at custom paths within existing actix-web applications:

```rust
use rmcp_actix_web::{SseService, StreamableHttpService};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use actix_web::{App, HttpServer, web};
use std::sync::Arc;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // SSE service with builder pattern
    let sse_service = SseService::builder()
        .service_factory(Arc::new(|| Ok(MyMcpService::new())))
        .sse_path("/events".to_string())
        .post_path("/messages".to_string())
        .build();

    // StreamableHttp service with builder pattern (shared across workers)
    let http_service = Arc::new(
        StreamableHttpService::builder()
            .service_factory(Arc::new(|| Ok(MyMcpService::new())))
            .session_manager(Arc::new(LocalSessionManager::default()))
            .stateful_mode(true)
            .build(),
    );

    HttpServer::new(move || {
        App::new()
            // Your existing routes
            .route("/health", web::get().to(|| async { "OK" }))
            // Mount MCP services at custom paths
            .service(web::scope("/api/v1/sse-calc").service(sse_service.clone().scope()))
            .service(web::scope("/api/v1/http-calc").service(http_service.clone().scope()))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await?;

    Ok(())
}
```

## Examples

See the `examples/` directory for complete working examples:

### Basic Examples
- `counter_sse.rs` - SSE server with a simple counter service
- `counter_streamable_http.rs` - Streamable HTTP server example

### Composition Examples
- `composition_sse_example.rs` - SSE server with framework-level composition
- `composition_streamable_http_example.rs` - StreamableHttp with custom mounting
- `multi_service_example.rs` - Multiple MCP services with different transports

### Running Examples

```bash
# Basic SSE server
cargo run --example counter_sse

# Framework composition with SSE
cargo run --example composition_sse_example

# Multi-service example with both transports
cargo run --example multi_service_example
```

Each example includes detailed documentation and curl commands for testing.

## Key Features

### Framework-Level Composition
- **SSE Service**: `SseService::builder().build()` with `.scope()` method for mounting at custom paths
- **StreamableHttp**: `StreamableHttpService::builder().build()` with `.scope()` for composition
- **Multi-Service**: Compose multiple MCP services with different transports in one app
- **Unified Builder API**: Consistent builder pattern across all transport services

### Protocol Support
- **Full MCP Compatibility**: Implements complete MCP protocol specification
- **Bidirectional Communication**: Both request/response and streaming patterns
- **Session Management**: Stateful and stateless modes for StreamableHttp
- **Keep-Alive**: Configurable keep-alive intervals for connection health

### Integration
- **Drop-in Replacement**: Same service implementations work with Axum or actix-web
- **Middleware Support**: Full integration with actix-web middleware stack
- **Custom Paths**: Mount services at any path using actix-web's Scope system
- **Built on actix-web**: Leverages the mature actix-web framework

## License

MIT License - see LICENSE file for details.

## Contributing

This project is part of the Model Context Protocol ecosystem. Contributions are welcome!

## References

- [Model Context Protocol Specification](https://modelcontextprotocol.io/)
- [RMCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [Original PR #294](https://github.com/modelcontextprotocol/rust-sdk/pull/294)
