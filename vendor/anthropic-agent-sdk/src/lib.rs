//! # Claude Agent SDK for Rust
//!
//! Rust SDK for building AI agents powered by Claude Code.
//! Async/await, strong typing, tokio-based.
//!
//! ## Quick Start
//!
//! Basic usage with [`query()`]:
//!
//! ```no_run
//! use anthropic_agent_sdk::query;
//! use futures::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let stream = query("What is 2 + 2?", None).await?;
//!     let mut stream = Box::pin(stream);
//!
//!     while let Some(message) = stream.next().await {
//!         match message? {
//!             anthropic_agent_sdk::Message::Assistant { message, .. } => {
//!                 println!("Claude: {:?}", message);
//!             }
//!             _ => {}
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Core Features
//!
//! ### 1. Simple Queries with [`query()`]
//!
//! For one-shot interactions where you don't need bidirectional communication:
//!
//! ```no_run
//! # use anthropic_agent_sdk::{query, ClaudeAgentOptions};
//! # use futures::StreamExt;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let options = ClaudeAgentOptions::builder()
//!     .system_prompt("You are a helpful coding assistant")
//!     .max_turns(5)
//!     .build();
//!
//! let stream = query("Explain async/await in Rust", Some(options)).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### 2. Interactive Client with [`ClaudeSDKClient`]
//!
//! For stateful conversations with bidirectional communication:
//!
//! ```no_run
//! # use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let options = ClaudeAgentOptions::builder()
//!     .max_turns(10)
//!     .build();
//!
//! let mut client = ClaudeSDKClient::new(options, None).await?;
//! client.send_message("Hello, Claude!").await?;
//!
//! while let Some(message) = client.next_message().await {
//!     // Process messages...
//! }
//!
//! client.close().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### 3. MCP Tools with rmcp
//!
//! Create custom MCP tools that Claude can discover and use. Enable the `rmcp` feature
//! to use the official [rmcp](https://crates.io/crates/rmcp) crate:
//!
//! ```ignore
//! use anthropic_agent_sdk::mcp::{
//!     tool, tool_router, tool_handler, Parameters, ServerCapabilities, ServerHandler,
//!     ServerInfo, ToolRouter,
//! };
//! use schemars::JsonSchema;
//! use serde::Deserialize;
//!
//! #[derive(Deserialize, JsonSchema)]
//! struct AddParams {
//!     /// First number
//!     a: f64,
//!     /// Second number
//!     b: f64,
//! }
//!
//! #[derive(Clone)]
//! struct Calculator { tool_router: ToolRouter<Self> }
//!
//! #[tool_router]
//! impl Calculator {
//!     fn new() -> Self { Self { tool_router: Self::tool_router() } }
//!
//!     #[tool(description = "Add two numbers")]
//!     fn add(&self, Parameters(params): Parameters<AddParams>) -> String {
//!         format!("{} + {} = {}", params.a, params.b, params.a + params.b)
//!     }
//! }
//!
//! #[tool_handler]
//! impl ServerHandler for Calculator {
//!     fn get_info(&self) -> ServerInfo {
//!         ServerInfo {
//!             capabilities: ServerCapabilities::builder().enable_tools().build(),
//!             ..Default::default()
//!         }
//!     }
//! }
//! ```
//!
//! See `examples/mcp_server.rs` for a complete demo.
//!
//! ### 4. Hooks for Custom Behavior
//!
//! Intercept and modify tool execution:
//!
//! ```no_run
//! # use anthropic_agent_sdk::{ClaudeAgentOptions, HookManager, HookEvent, HookOutput};
//! # use anthropic_agent_sdk::hooks::HookMatcherBuilder;
//! # use std::collections::HashMap;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let hook = HookManager::callback(|event_data, tool_name, _context| async move {
//!     println!("Tool used: {:?}", tool_name);
//!     Ok(HookOutput::default())
//! });
//!
//! let matcher = HookMatcherBuilder::new(Some("*"))
//!     .add_hook(hook)
//!     .build();
//!
//! let mut hooks = HashMap::new();
//! hooks.insert(HookEvent::PreToolUse, vec![matcher]);
//!
//! let options = ClaudeAgentOptions::builder()
//!     .hooks(hooks)
//!     .build();
//! # Ok(())
//! # }
//! ```
//!
//! See the [`hooks`] module for more details.
//!
//! ### 5. Permission Control
//!
//! Control which tools Claude can use and how:
//!
//! ```no_run
//! # use anthropic_agent_sdk::{ClaudeAgentOptions, PermissionManager};
//! # use anthropic_agent_sdk::types::{PermissionResult, PermissionResultAllow, PermissionResultDeny};
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let permission_callback = PermissionManager::callback(
//!     |tool_name, _tool_input, _context| async move {
//!         match tool_name.as_str() {
//!             "Read" | "Glob" => Ok(PermissionResult::Allow(PermissionResultAllow {
//!                 updated_input: None,
//!                 updated_permissions: None,
//!             })),
//!             _ => Ok(PermissionResult::Deny(PermissionResultDeny {
//!                 message: "Tool not allowed".to_string(),
//!                 interrupt: false,
//!             }))
//!         }
//!     }
//! );
//!
//! let options = ClaudeAgentOptions::builder()
//!     .can_use_tool(permission_callback)
//!     .build();
//! # Ok(())
//! # }
//! ```
//!
//! See the [`permissions`] module for more details.
//!
//! ## Architecture
//!
//! The SDK is organized into several key modules:
//!
//! - [`types`]: Core type definitions, identifiers, and builders
//! - [`query()`]: Simple one-shot query function
//! - [`client`]: Interactive bidirectional client
//! - [`mcp`]: SDK MCP server for custom tools
//! - [`hooks`]: Hook system for intercepting events
//! - [`permissions`]: Permission control for tool usage
//! - [`transport`]: Communication layer with Claude Code CLI
//! - [`control`]: Control protocol handler
//! - [`message`]: Message parsing and types
//! - [`error`]: Error types and handling
//!
//! ## Feature Flags
//!
//! This crate supports the following feature flags:
//!
//! - `rmcp` - Enables SDK MCP server support via the official rmcp crate
//!
//! ## Logging
//!
//! This crate uses [`tracing`](https://crates.io/crates/tracing) for structured logging.
//! Tracing events are always emitted but are zero-cost when no subscriber is attached.
//! To see logs, attach a tracing subscriber in your application:
//!
//! ```rust,ignore
//! tracing_subscriber::fmt::init();
//! ```
//!
//! ## Examples
//!
//! - `simple_query.rs` - Basic query usage
//! - `interactive_client.rs` - Interactive REPL conversation
//! - `bidirectional_demo.rs` - Concurrent operations
//! - `hooks_demo.rs` - Hook system for tool interception
//! - `permissions_demo.rs` - Permission control for tools
//! - `mcp_server.rs` - MCP server with custom tools (requires `--features rmcp`)
//! - `mcp_integration.rs` - Full E2E with Claude using MCP tools
//! - `introspection_demo.rs` - Session info, models, commands, MCP status
//! - `plan_mode_demo.rs` - Plan mode with approval workflow
//! - `oauth_demo.rs` - OAuth authentication with PKCE
//!
//! Run examples with:
//! ```bash
//! cargo run --example simple_query
//! cargo run --example oauth_demo
//! cargo run --example mcp_server --features rmcp
//! ```
//!
//! ## Requirements
//!
//! - Rust 1.85.0 or later
//! - Node.js (for Claude Code CLI)
//! - Claude Code: `npm install -g @anthropic-ai/claude-code`
//!
//! ## Error Handling
//!
//! All fallible operations return [`Result<T, ClaudeError>`](Result):
//!
//! ```no_run
//! # use anthropic_agent_sdk::{query, ClaudeError};
//! # async fn example() {
//! match query("Hello", None).await {
//!     Ok(stream) => { /* ... */ }
//!     Err(ClaudeError::CliNotFound(msg)) => {
//!         eprintln!("Claude Code not installed: {}", msg);
//!     }
//!     Err(e) => {
//!         eprintln!("Error: {}", e);
//!     }
//! }
//! # }
//! ```
//!
//! ## Safety and Best Practices
//!
//! - **No unsafe code** - The SDK is 100% safe Rust
//! - **Type safety** - Newtypes prevent mixing incompatible values
//! - **Async/await** - Built on tokio for efficient concurrency
//! - **Resource management** - Proper cleanup via RAII and Drop
//! - **Error handling** - Typed errors with context
//!
//! ## Security
//!
//! - **Environment variable filtering** - Dangerous variables like `LD_PRELOAD`, `PATH`, `NODE_OPTIONS` are blocked
//! - **Callback timeouts** - Hook and permission callbacks have configurable timeouts (default 60 seconds)
//! - **Buffer limits** - Configurable max buffer size (default 1MB) prevents memory exhaustion
//! - **Cancellation support** - Callbacks receive cancellation tokens for graceful abort
//!
//! For complete security details, see `SECURITY.md` in the repository.
//!
//! ## Version History
//!
//! - **0.2.0** (Current) - TypeScript SDK parity release
//!   - MCP integration via official rmcp crate
//!   - Hooks, introspection, runtime setters
//!   - Plan mode, slash commands, skills support
//!   - Model usage tracking, permission denials
//!
//! - **0.1.0** - Initial release
//!   - `query()` function for simple queries
//!   - `ClaudeSDKClient` for bidirectional communication
//!   - Hook system for event interception
//!   - Permission control for tool usage

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod auth;
pub mod callbacks;
pub mod client;
pub mod control;
pub mod error;
pub mod hooks;
pub mod mcp;
pub mod message;
pub mod permissions;
pub mod query;
pub mod transport;
pub mod types;
pub mod utils;

// Re-export commonly used types
pub use callbacks::{
    FnHookCallback, FnPermissionCallback, HookCallback, PermissionCallback, SharedHookCallback,
    SharedPermissionCallback,
};
pub use client::ClaudeSDKClient;
pub use error::{ClaudeError, Result};
pub use futures::StreamExt;
pub use hooks::{HookManager, HookMatcherBuilder};
pub use message::parse_message;
pub use permissions::{PermissionManager, PermissionManagerBuilder};
pub use query::query;
pub use transport::{
    MIN_CLI_VERSION, PromptInput, SubprocessTransport, Transport, check_claude_version,
};
pub use types::{
    AgentDefinition, CanUseToolCallback, ClaudeAgentOptions, ClaudeAgentOptionsBuilder,
    ContentBlock, ContentValue, HookContext, HookDecision, HookEvent, HookMatcher, HookOutput,
    McpHttpServerConfig, McpServerConfig, McpServers, McpSseServerConfig, McpStdioServerConfig,
    Message, OutputFormat, PermissionBehavior, PermissionMode, PermissionRequest, PermissionResult,
    PermissionResultAllow, PermissionResultDeny, PermissionRuleValue, PermissionUpdate,
    PermissionUpdateDestination, RequestId, SdkMcpServerConfig, SessionId, SettingSource,
    SystemPrompt, SystemPromptPreset, ToolName, ToolPermissionContext, UsageData, UsageLimit,
    UserContent,
};

/// Version of the SDK
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
