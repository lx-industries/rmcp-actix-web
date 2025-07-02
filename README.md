# rmcp-actix-web

actix-web transport implementations for RMCP (Rust Model Context Protocol)

This crate provides actix-web-based transport implementations for the Model Context Protocol, offering a complete alternative to the default Axum-based transports in the main RMCP crate.

## Overview

`rmcp-actix-web` provides:
- SSE (Server-Sent Events) server transport
- Streamable HTTP server transport
- Full compatibility with the MCP protocol specification
- Drop-in replacement for RMCP's Axum transports

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rmcp-actix-web = "0.1"
rmcp = "0.1"
actix-web = "4"
```

## Quick Start

### SSE Server

```rust
use rmcp_actix_web::{SseServer, SseServerConfig};

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create SSE server configuration
    let config = SseServerConfig {
        bind: "127.0.0.1:8080".parse()?,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: tokio_util::sync::CancellationToken::new(),
        sse_keep_alive: None,
    };

    // Start the server
    let server = SseServer::serve_with_config(config).await?;
    
    // Attach your MCP service
    let ct = server.with_service(|| MyMcpService::new());
    
    // Wait for shutdown
    ct.cancelled().await;
    Ok(())
}
```

### Streamable HTTP Server

```rust
use rmcp_actix_web::StreamableHttpService;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use actix_web::{App, HttpServer};

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = StreamableHttpService::new(
        || Ok(MyMcpService::new()),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    HttpServer::new(move || {
        App::new()
            .configure(|cfg| service.config(cfg))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await?;
    
    Ok(())
}
```

## Examples

See the `examples/` directory for complete working examples:
- `counter_sse.rs` - SSE server with a simple counter service
- `counter_streamable_http.rs` - Streamable HTTP server example

Run an example:
```bash
cargo run --example counter_sse
```

## API Compatibility

This crate maintains full API compatibility with the Axum-based transports in RMCP. The same service implementations can be used with either transport layer.

## License

MIT License - see LICENSE file for details.

## Contributing

This project is part of the Model Context Protocol ecosystem. Contributions are welcome!

## References

- [Model Context Protocol Specification](https://modelcontextprotocol.io/)
- [RMCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [Original PR #294](https://github.com/modelcontextprotocol/rust-sdk/pull/294)