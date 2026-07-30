# Security Considerations

## Authorization Header Forwarding

This transport layer can forward Authorization headers from HTTP requests to MCP services. This enables two distinct architectural patterns:

### Pattern 1: MCP Service Authentication (MCP-Compliant)
- The Authorization header authenticates the client to the MCP service
- The MCP service validates tokens intended for itself
- The MCP service uses separate credentials for any upstream API calls
- **This follows MCP specification requirements**

### Pattern 2: Token Passthrough Proxy (Non-Compliant)
- The Authorization header is forwarded through the MCP service to backend APIs
- The MCP service acts as a transparent proxy for authentication
- Backend APIs receive and validate the original client tokens
- **This violates MCP specification: "MCP servers MUST NOT pass through the token it received from the MCP client"**

## The `authorization-token-passthrough` Feature Gate

Forwarding is off by default. Unless the `authorization-token-passthrough` feature is enabled, the
transport removes the `Authorization` header from the request the MCP service observes. Both
surfaces the transport controls are covered:

- the `AuthorizationHeader` extension is not inserted, and
- the header is absent from `http::request::Parts::headers` in the MCP request context.

This gate governs what the transport itself forwards to the MCP service, not what a
user-supplied `on_request` hook may do. The hook runs on the actix-web `HttpRequest`
before the `Authorization` header is stripped, so it still sees the raw header; a hook
that copies request headers into extensions of its own accord can forward the token past
this gate regardless of the feature setting. That is deployment-layer code with access to
the whole request by construction, not a defect in this transport.

With the feature enabled, a well-formed `Bearer` token is left on the request and additionally
surfaced as `AuthorizationHeader` in the extensions written by the `on_request` hook. Read it back
with `rmcp_actix_web::transport::on_request_extensions`:

```rust,ignore
use rmcp_actix_web::transport::{AuthorizationHeader, on_request_extensions};

let token = on_request_extensions(&context.extensions)
    .and_then(|extensions| extensions.get::<AuthorizationHeader>())
    .map(|auth| auth.0.as_str());
```

Non-`Bearer` schemes, empty tokens, and non-UTF-8 values are never forwarded. They are *removed
from the request* the MCP service observes, not merely left un-exposed as an extension: even with
the feature enabled, a `Basic` or malformed `Authorization` value is absent from
`http::request::Parts::headers`. Only a well-formed `Bearer` value survives, and only under the
feature.

## Security Implications

When using Pattern 2 (Token Passthrough):
- **Confused Deputy Risk**: Tokens meant for one service are used at another
- **Audience Validation**: Backend APIs MUST validate token audience claims
- **Token Scoping**: Clients MUST obtain tokens scoped for the backend API, not the MCP server

## Recommendations

1. **For MCP Services**: Validate tokens according to OAuth 2.1 Section 5.2
2. **For Proxy Implementations**: Document clearly that token passthrough violates MCP spec
3. **For Production Use**: Consider implementing proper OAuth delegation instead of passthrough

See [rmcp-openapi#67](https://gitlab.com/lx-industries/rmcp-openapi/-/issues/67) for detailed discussion.