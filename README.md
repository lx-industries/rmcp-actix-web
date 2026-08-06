# rmcp-actix-web

actix-web transport implementations for RMCP (Rust Model Context Protocol)

This crate provides actix-web-based transport implementations for the Model Context Protocol, offering a complete alternative to the default Axum-based transports in the main RMCP crate.

## Overview

`rmcp-actix-web` provides:
- **Streamable HTTP transport**: Bidirectional communication with session management
- **Framework-level composition**: Mount MCP services at custom paths using actix-web Scope
- **Full MCP compatibility**: Implements the complete MCP protocol specification
- **RMCP ecosystem alignment**: APIs that follow RMCP patterns for maximum consistency

## ⚠️ Security Notice

This transport forwards Authorization headers to MCP services. If your MCP service passes these tokens to upstream APIs (proxy pattern), be aware this violates MCP specifications. See [SECURITY.md](SECURITY.md) for details.

## Contributing

We welcome contributions to `rmcp-actix-web`! Please follow these guidelines:

### How to Contribute

1. **Fork the repository** on GitLab
2. **Create a feature branch** from `main`: `git checkout -b feature/my-new-feature`
3. **Make your changes** and ensure they follow the project's coding standards
4. **Add tests** for your changes if applicable and run examples to verify functionality
5. **Run the test suite** to ensure nothing is broken: `cargo test`
6. **Commit your changes** with clear, descriptive commit messages
7. **Push to your fork** and **create a merge request**

### Development Setup

```bash
# Clone your fork
git clone https://gitlab.com/your-username/rmcp-actix-web.git
cd rmcp-actix-web

# Build the project
cargo build --workspace

# Run tests
cargo test

# Run examples
cargo run --example counter_streamable_http
```

### Code Standards

- Follow Rust conventions and use `cargo fmt` to format code
- Run `cargo clippy --all-targets` to catch common mistakes
- Add documentation for public APIs
- Include tests for new functionality

### Reporting Issues

Found a bug or have a feature request? Please report it on our [GitLab issue tracker](https://gitlab.com/lx-industries/rmcp-actix-web/-/issues).

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rmcp-actix-web = "0.13"
rmcp = "3"
actix-web = "4"
```

### Feature Flags

| Feature | Default | Effect |
|---------|---------|--------|
| `transport-streamable-http` | yes | Enables the Streamable HTTP transport, which delegates every wire-protocol decision to rmcp's own `StreamableHttpService`. |
| `legacy-transport` | no | Additionally exposes the hand-written transport that predates that delegation, at `rmcp_actix_web::transport::legacy_streamable_http_server`. |
| `authorization-token-passthrough` | no | Forwards the `Authorization` header to the MCP service. Violates the MCP specification; see [SECURITY.md](SECURITY.md). |

```toml
# Default: Streamable HTTP transport enabled
rmcp-actix-web = "0.13"

# Streamable HTTP transport (explicit)
rmcp-actix-web = { version = "0.13", default-features = false, features = ["transport-streamable-http"] }

# Forward Authorization headers to your MCP service (see SECURITY.md first)
rmcp-actix-web = { version = "0.13", features = ["authorization-token-passthrough"] }
```

#### `legacy-transport`

`legacy-transport` keeps the hand-written actix-web transport available alongside the
delegating one. Enable it only if you depend on its wire behaviour or on its flat
`on_request` extension shape, where handlers read hook-written values straight from
`RequestContext::extensions`. It is frozen and receives no new features, and it does not
support MCP `2026-07-28`: its sessionless path serves every peer the legacy wire shape.

```toml
rmcp-actix-web = { version = "0.13", features = ["legacy-transport"] }
```

```rust,ignore
use rmcp_actix_web::transport::legacy_streamable_http_server::StreamableHttpService;
```

## Compatibility Matrix

| rmcp-actix-web | rmcp |
|----------------|------|
| 0.13.x         | 3.x  |
| 0.6.1          | 0.6.3|
| 0.4.2          | 0.6.1|
| 0.2.2          | 0.3.0|
| 0.2.x          | 0.2.x|
| 0.1.x          | 0.2.x|

## Upgrading to 0.13

0.13 replaces this crate's hand-written Streamable HTTP transport with delegation to
rmcp's own `StreamableHttpService`. rmcp now makes every wire-protocol decision, and this
crate contributes actix-web `Scope` composition and the builder API. That makes rmcp's
responses the contract, which changes observable behaviour in the four areas below. All
of these changes are adopted deliberately; none of them are bugs.

### Requests are now validated against `Host`

| Request | Before | Now |
|---------|--------|-----|
| Any request whose `Host` is outside rmcp's [loopback-only default allow-list](https://docs.rs/rmcp/3/rmcp/transport/streamable_http_server/tower/struct.StreamableHttpServerConfig.html#structfield.allowed_hosts) | served | `403 Forbidden`, body `Forbidden: Host header is not allowed` |
| Any request carrying no `Host` header at all | served | `400 Bad Request`, body `Bad Request: missing Host header` |

This is a DNS-rebinding defence inherited from rmcp. **Any deployment reachable under its
own hostname rejects every request until you configure `.allowed_hosts(...)`.** HTTP/1.1
clients always send `Host`, but actix's in-process test harness does not. See
[Host and Origin Validation](#host-and-origin-validation) for how to configure both.

### Session-handling status codes now follow rmcp

| Request | Before | Now | Why |
|---------|--------|-----|-----|
| `DELETE` with an unknown session id | `404 Not Found` | `202 Accepted`, empty body | `DELETE` is idempotent: deleting a session that does not exist reaches the same end state as deleting one that does. |
| `DELETE` with an empty `Mcp-Session-Id` | `400 Bad Request` | `202 Accepted`, empty body | Same idempotence, applied to an empty id: it is an id no session matches, not an absent one. |
| `POST` or `GET` with an empty `Mcp-Session-Id` | `400 Bad Request` | `404 Not Found`, body `Not Found: Session not found` | An empty header value is not a missing header. It is a session id that no session matches, so the client can recover by re-initializing. |
| `POST` with no session id that is not an `initialize` request | `400 Bad Request` | `422 Unprocessable Entity`, body `Unexpected message, expect initialize request` | The request is well-formed; it is the message that cannot be processed without a session. |

Two statuses are unchanged, but their bodies are rmcp's wording now: `POST` or `GET`
with an unknown non-empty session id still answers `404 Not Found`, with the body
`Not Found: Session not found` rather than `Session not found`; `GET` or `DELETE` with no
session id at all still answers `400 Bad Request`, with the body
`Bad Request: Session ID is required` rather than
`Bad Request: Mcp-Session-Id header is required`. Clients that match on body text rather
than status need updating.

### Hook-written extensions are nested

rmcp hands your MCP service the whole `http::request::Parts`, so values written by the
`on_request` hook are reached one hop further in than before. Read them with
`rmcp_actix_web::transport::on_request_extensions(&context.extensions)` instead of
indexing `context.extensions` directly — see
[Middleware Extension Propagation](#middleware-extension-propagation).

If you need the previous transport verbatim, including its flat extension shape, enable
the [`legacy-transport`](#legacy-transport) feature.

### SSE keep-alive follows rmcp's default

A builder knob left unset is not written to rmcp's config, so it keeps whatever default
rmcp chose. Leaving `sse_keep_alive` unset therefore sends rmcp's own keep-alive interval.

| Request | Before | Now |
|---------|--------|-----|
| An SSE stream from a service that never set `sse_keep_alive` | no keep-alive comments | an empty SSE comment (`:` alone on a line) at rmcp's default interval |
| An SSE stream from a service that called `.maybe_sse_keep_alive(None)` | no keep-alive comments | same as never setting it: rmcp's default interval |

Call `.sse_keep_alive(Duration::from_secs(...))` for a specific interval, or
`.disable_sse_keep_alive()` for the previous behaviour of no keep-alive at all.
`.maybe_sse_keep_alive(interval)` still takes an `Option<Duration>` for a value that is
optional at runtime, but its `None` now means "leave the knob unset" rather than "no
keep-alive"; `.disable_sse_keep_alive()` is what turns keep-alive off. The same three-way
distinction applies to `sse_retry`, which gains `.sse_retry(...)`,
`.maybe_sse_retry(...)` and `.disable_sse_retry()`.

## Quick Start

### Framework-Level Composition

Mount MCP services at custom paths within existing actix-web applications:

```rust
use rmcp_actix_web::transport::{LocalSessionManager, StreamableHttpService};
use actix_web::{App, HttpServer, web};
use std::sync::Arc;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // StreamableHttp service with builder pattern (shared across workers)
    let http_service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(MyMcpService::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(true)
        .build();

    HttpServer::new(move || {
        App::new()
            // Your existing routes
            .route("/health", web::get().to(|| async { "OK" }))
            // Mount MCP service at custom path
            .service(web::scope("/api/v1/mcp").service(http_service.clone().scope()))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await?;

    Ok(())
}
```

### Host and Origin Validation

By default, the transport accepts loopback hosts only — rmcp's
[`allowed_hosts` default](https://docs.rs/rmcp/3/rmcp/transport/streamable_http_server/tower/struct.StreamableHttpServerConfig.html#structfield.allowed_hosts) — and responds `403 Forbidden` to
every other `Host`. This is a DNS-rebinding defence inherited from rmcp and is safe for
servers running on loopback, but it means **any deployment reachable under its own
hostname rejects every request unless configured**. Set `.allowed_hosts(...)` to the
hostnames or `host:port` authorities your deployment is reachable under:

```rust,ignore
let http_service = StreamableHttpService::builder()
    .service_factory(Arc::new(|| Ok(MyMcpService::new())))
    .session_manager(Arc::new(LocalSessionManager::default()))
    .allowed_hosts(vec!["mcp.example.com".to_string()])
    .build();
```

`.allowed_origins(...)` similarly restricts the inbound `Origin` header. Leaving it unset
inherits rmcp's own default, which performs no `Origin` validation.

The same defence also rejects a request that carries **no** `Host` header at all, with
`400 Bad Request` and the body `Bad Request: missing Host header`. Setting
`allowed_hosts` does not exempt a request from this: the header must be present for
there to be an authority to check. HTTP/1.1 clients always send `Host`, so this does not
affect ordinary traffic, but actix's in-process test harness does not — tests written
with `actix_web::test::TestRequest` must set the header explicitly:

```rust,ignore
let req = test::TestRequest::post()
    .uri("/mcp/")
    .insert_header(("host", "localhost"))
    // ...
    .to_request();
```

Both behaviours are new in 0.13; see [Upgrading to 0.13](#upgrading-to-013) for the full
list of changes an upgrade brings.

## Examples

See the `examples/` directory for complete working examples:

### Basic Examples
- `counter_streamable_http.rs` - Streamable HTTP server example

### Composition Examples
- `composition_streamable_http_example.rs` - StreamableHttp with custom mounting

### Proxy Examples
- `authorization_proxy_example.rs` - MCP service acting as a proxy using Authorization headers

### Running Examples

```bash
# Basic StreamableHttp server
cargo run --example counter_streamable_http

# Framework composition with StreamableHttp
cargo run --example composition_streamable_http_example

# Authorization proxy example
cargo run --example authorization_proxy_example
```

Each example includes detailed documentation and curl commands for testing.

## Key Features

### Framework-Level Composition
- **StreamableHttp**: `StreamableHttpService::builder().build()` with `.scope()` for composition
- **Custom Paths**: Mount services at any path using actix-web's Scope system
- **Builder API**: Consistent builder pattern for service configuration

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

### Middleware Extension Propagation

Use the `on_request` hook to propagate typed data from actix-web middleware to MCP request handlers. This is useful for passing JWT claims, user context, or other authentication data:

```rust
use rmcp_actix_web::transport::{LocalSessionManager, StreamableHttpService};
use actix_web::HttpMessage;
use std::sync::Arc;

#[derive(Clone)]
struct JwtClaims { user_id: String }

let http_service = StreamableHttpService::builder()
    .service_factory(Arc::new(|| Ok(MyMcpService::new())))
    .session_manager(Arc::new(LocalSessionManager::default()))
    .on_request_fn(|http_req, ext| {
        // Access data populated by actix-web middleware
        if let Some(claims) = http_req.extensions().get::<JwtClaims>() {
            ext.insert(claims.clone());
        }
    })
    .build();
```

rmcp's transport nests the hook's extensions inside `http::request::Parts`, so read them back
in your MCP service handlers with `rmcp_actix_web::transport::on_request_extensions(&context.extensions)`
rather than reading `context.extensions` directly:

```rust,ignore
use rmcp_actix_web::transport::on_request_extensions;

async fn handle_request(
    &self,
    request: SomeRequest,
    context: RequestContext<RoleServer>,
) -> Result<Response, McpError> {
    if let Some(claims) = on_request_extensions(&context.extensions)
        .and_then(|extensions| extensions.get::<JwtClaims>())
    {
        // ...
    }
    // ...
}
```

### Proxy Support
- **Authorization Forwarding**: Bearer tokens from Authorization headers can be forwarded to MCP services (requires `authorization-token-passthrough` feature)
- **MCP Proxy Pattern**: Enable MCP services to act as proxies to backend APIs
- **Selective Header Forwarding**: Only forwards Authorization header when feature is enabled
- **Type-Safe Access**: Read the forwarded header as an `AuthorizationHeader` via `transport::on_request_extensions(&context.extensions)`
- **Security Notice**: Token passthrough violates MCP specifications - see [SECURITY.md](SECURITY.md) for important details

## License

MIT License - see LICENSE file for details.

## References

- [Model Context Protocol Specification](https://modelcontextprotocol.io/)
- [RMCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [Original PR #294](https://github.com/modelcontextprotocol/rust-sdk/pull/294)
