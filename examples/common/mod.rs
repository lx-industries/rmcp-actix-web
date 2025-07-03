//! Common service implementations for rmcp-actix-web examples.
//!
//! This module provides example MCP services that demonstrate different
//! aspects of the protocol and can be used with both SSE and streamable
//! HTTP transports.

/// Calculator service demonstrating basic tool implementation.
pub mod calculator;

/// Counter service demonstrating stateful operations and various MCP features.
pub mod counter;

/// Generic service template for creating custom MCP services.
pub mod generic_service;
