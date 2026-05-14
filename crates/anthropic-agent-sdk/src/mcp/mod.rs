//! MCP (Model Context Protocol) integration for Claude Agent SDK
//!
//! This module provides MCP server configuration types and optional rmcp integration
//! for creating in-process MCP servers with custom tools.
//!
//! # Configuration Types (always available)
//!
//! MCP server configuration types are always available for configuring external servers:
//!
//! - [`McpStdioServerConfig`] - Spawn MCP server as subprocess
//! - [`McpSseServerConfig`] - Connect via Server-Sent Events
//! - [`McpHttpServerConfig`] - Connect via HTTP
//! - [`SdkMcpServerConfig`] - In-process SDK server (requires `rmcp` feature)
//!
//! # SDK MCP Servers (requires `rmcp` feature)
//!
//! Enable the `rmcp` feature to create in-process MCP servers using the official
//! [rmcp](https://crates.io/crates/rmcp) crate:
//!
//! ```toml
//! [dependencies]
//! anthropic-agent-sdk = { version = "0.2", features = ["rmcp"] }
//! ```
//!
//! ## Quick Start with rmcp
//!
//! ```ignore
//! use anthropic_agent_sdk::mcp::{tool, tool_router, tool_handler};
//! use anthropic_agent_sdk::mcp::{Parameters, CallToolResult, Content, ToolRouter};
//! use rmcp::ServerHandler;
//! use schemars::JsonSchema;
//! use serde::Deserialize;
//!
//! #[derive(Deserialize, JsonSchema)]
//! struct AddParams {
//!     a: f64,
//!     b: f64,
//! }
//!
//! #[derive(Clone)]
//! struct Calculator {
//!     tool_router: ToolRouter<Self>,
//! }
//!
//! #[tool_router]
//! impl Calculator {
//!     fn new() -> Self {
//!         Self { tool_router: Self::tool_router() }
//!     }
//!
//!     #[tool(description = "Add two numbers")]
//!     async fn add(&self, params: Parameters<AddParams>) -> Result<CallToolResult, String> {
//!         Ok(CallToolResult::success(vec![Content::text(
//!             format!("{} + {} = {}", params.a, params.b, params.a + params.b)
//!         )]))
//!     }
//! }
//!
//! #[tool_handler]
//! impl ServerHandler for Calculator {
//!     fn get_info(&self) -> rmcp::model::ServerInfo {
//!         rmcp::model::ServerInfo::new("calculator", "1.0.0")
//!     }
//! }
//! ```
//!
//! # Benefits of SDK MCP Servers
//!
//! - **No subprocess overhead** - Tools run in the same process
//! - **Type safety** - Compile-time schema validation via schemars
//! - **Ergonomic macros** - `#[tool]`, `#[tool_router]`, `#[tool_handler]`
//! - **Full MCP support** - Resources, prompts, sampling, and more via rmcp

// Re-export configuration types (always available)
pub use crate::types::mcp::{
    McpHttpServerConfig, McpServerConfig, McpServers, McpSseServerConfig, McpStdioServerConfig,
    SdkMcpServerConfig,
};

// SDK MCP server support via rmcp (optional)
#[cfg(feature = "rmcp")]
mod sdk;
#[cfg(feature = "rmcp")]
pub use sdk::*;
