//! SDK MCP Server support via rmcp
//!
//! This module provides re-exports from the official rmcp crate for creating
//! in-process MCP servers with custom tools.
//!
//! # Example
//!
//! ```ignore
//! use anthropic_agent_sdk::mcp::{tool, tool_router, tool_handler};
//! use anthropic_agent_sdk::mcp::{Parameters, CallToolResult, Content, ToolRouter, ServerHandler};
//! use schemars::JsonSchema;
//! use serde::Deserialize;
//!
//! #[derive(Deserialize, JsonSchema)]
//! struct GreetParams {
//!     name: String,
//! }
//!
//! #[derive(Clone)]
//! struct Greeter {
//!     tool_router: ToolRouter<Self>,
//! }
//!
//! #[tool_router]
//! impl Greeter {
//!     fn new() -> Self {
//!         Self { tool_router: Self::tool_router() }
//!     }
//!
//!     #[tool(description = "Greet someone by name")]
//!     async fn greet(&self, params: Parameters<GreetParams>) -> Result<CallToolResult, String> {
//!         Ok(CallToolResult::success(vec![Content::text(
//!             format!("Hello, {}!", params.name)
//!         )]))
//!     }
//! }
//!
//! #[tool_handler]
//! impl ServerHandler for Greeter {
//!     fn get_info(&self) -> rmcp::model::ServerInfo {
//!         rmcp::model::ServerInfo::new("greeter", "1.0.0")
//!     }
//! }
//! ```

// Re-export rmcp macros for tool definition
pub use rmcp::{tool, tool_handler, tool_router};

// Re-export core types from rmcp
pub use rmcp::ServerHandler;

// Re-export model types
pub use rmcp::model::{
    // Tool types
    CallToolResult,
    // Content types
    Content,
    CustomNotification,
    // Custom protocol extensions (rmcp 0.12.0+)
    CustomRequest,
    CustomResult,
    // Server info and capabilities
    ServerCapabilities,
    ServerInfo,
    Tool,
};

// Re-export handler types
pub use rmcp::handler::server::tool::ToolRouter;
pub use rmcp::handler::server::wrapper::{Json, Parameters};

// Re-export service types for handler signatures
pub use rmcp::service::{RequestContext, RoleServer};

// Re-export schemars for schema derivation
pub use rmcp::schemars;

// Re-export ErrorData for tool error handling
pub use rmcp::ErrorData as McpError;

// Re-export transport for stdio server
pub use rmcp::transport::io::stdio;

// Re-export service traits for serving
pub use rmcp::ServiceExt;

/// Marker trait for SDK MCP servers
///
/// Any type implementing `rmcp::ServerHandler` automatically implements this trait,
/// making it usable as an SDK MCP server.
pub trait SdkMcpServer: rmcp::ServerHandler {}
impl<T: rmcp::ServerHandler> SdkMcpServer for T {}
