# Agent SDK reference - Rust

API reference for the Rust Agent SDK, including all functions, types, and traits.

---

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
anthropic-agent-sdk = "0.2"
```

With MCP SDK server support:

```toml
[dependencies]
anthropic-agent-sdk = { version = "0.2", features = ["rmcp"] }
```

## Functions

### `query()`

One-shot query function for stateless interactions. For multi-turn conversations, use [`ClaudeSDKClient`](#claudesdkclient) instead.

```rust
pub async fn query(
    prompt: impl Into<String>,
    options: Option<ClaudeAgentOptions>,
) -> Result<impl Stream<Item = Result<Message>>>
```

#### Parameters

| Parameter | Type                         | Description                                                                   |
| :-------- | :--------------------------- | :---------------------------------------------------------------------------- |
| `prompt`  | `impl Into<String>`          | The input prompt                                                              |
| `options` | `Option<ClaudeAgentOptions>` | Configuration options (defaults to `ClaudeAgentOptions::default()` if `None`) |

#### Returns

Returns a `Stream<Item = Result<Message>>` that yields messages from the conversation.

#### Example

```rust
use anthropic_agent_sdk::query;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream = query("What is the capital of France?", None).await?;
    let mut stream = Box::pin(stream);

    while let Some(message) = stream.next().await {
        println!("{:?}", message?);
    }
    Ok(())
}
```

With options:

```rust
use anthropic_agent_sdk::{query, ClaudeAgentOptions};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = ClaudeAgentOptions::builder()
        .system_prompt("You are a helpful coding assistant")
        .max_turns(1)
        .build();

    let stream = query("Write a hello world in Python", Some(options)).await?;
    let mut stream = Box::pin(stream);

    while let Some(message) = stream.next().await {
        println!("{:?}", message?);
    }
    Ok(())
}
```

### `ClaudeSDKClient`

Client for bidirectional communication with Claude Code. Supports interactive conversations, interrupts, hooks, and permission callbacks.

```rust
pub struct ClaudeSDKClient { /* fields omitted */ }
```

#### Constructor

```rust
impl ClaudeSDKClient {
    pub async fn new(
        options: ClaudeAgentOptions,
        cli_path: Option<PathBuf>,
    ) -> Result<Self>
}
```

| Parameter  | Type                 | Description                                       |
| :--------- | :------------------- | :------------------------------------------------ |
| `options`  | `ClaudeAgentOptions` | Configuration options                             |
| `cli_path` | `Option<PathBuf>`    | Path to Claude Code CLI (auto-detected if `None`) |

#### Methods

| Method                                      | Description                                         |
| :------------------------------------------ | :-------------------------------------------------- |
| `send_message(content)`                     | Send a message to Claude                            |
| `next_message()`                            | Get the next message from the stream                |
| `receive_response()`                        | Stream messages until a Result message              |
| `interrupt()`                               | Send an interrupt signal                            |
| `close()`                                   | Close the client and clean up resources             |
| `is_connected()`                            | Check if the client is connected                    |
| `get_session_id()`                          | Get the current session ID                          |
| `queue_message(content)`                    | Queue a message to send after current turn          |
| `next_buffered()`                           | Get next message, auto-send queued after Result     |
| `queued_count()`                            | Number of messages in queue                         |
| `has_queued()`                              | Check if queue has messages                         |
| `send_queued()`                             | Manually send next queued message                   |
| `clear_queue()`                             | Clear all queued messages                           |
| `bind_session(session_id)`                  | Bind client to session, enable validation           |
| `bound_session()`                           | Get bound session ID                                |
| `unbind_session()`                          | Clear session binding                               |
| `validate_session()`                        | Validate current session matches bound              |
| `session_info()`                            | Get session information (model, tools, MCP servers) |
| `current_model()`                           | Get the current model being used                    |
| `available_tools()`                         | Get available tools in this session                 |
| `mcp_server_status()`                       | Get status of MCP servers                           |
| `supported_models()`                        | Get list of known Claude models (static)            |
| `supported_commands()`                      | Get available slash commands                        |
| `account_info()`                            | Get account information                             |
| `set_model(model)`                          | Store model preference locally                      |
| `get_runtime_model()`                       | Get runtime model override                          |
| `set_permission_mode(mode)`                 | Store permission mode locally                       |
| `get_runtime_permission_mode()`             | Get runtime permission mode override                |
| `set_max_thinking_tokens(tokens)`           | Store thinking tokens preference locally            |
| `get_runtime_max_thinking_tokens()`         | Get runtime thinking tokens override                |
| `clear_runtime_overrides()`                 | Clear all runtime overrides                         |
| `cancellation_token()`                      | Get a child cancellation token                      |
| `cancel()`                                  | Cancel all ongoing operations                       |
| `is_cancelled()`                            | Check if cancellation was requested                 |
| `take_hook_receiver()`                      | Take the hook event receiver for manual handling    |
| `take_permission_receiver()`                | Take the permission request receiver                |
| `respond_to_hook(hook_id, response)`        | Respond to a hook event                             |
| `respond_to_permission(request_id, result)` | Respond to a permission request                     |

#### Basic Usage

```rust
use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = ClaudeAgentOptions::default();
    let mut client = ClaudeSDKClient::new(options, None).await?;

    client.send_message("Hello, Claude!").await?;

    while let Some(message) = client.next_message().await {
        match message? {
            Message::Assistant { message, .. } => {
                println!("Response: {:?}", message.content);
            }
            Message::Result { .. } => break,
            _ => {}
        }
    }

    client.close().await?;
    Ok(())
}
```

#### Using `receive_response()`

```rust
use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions, Message};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = ClaudeAgentOptions::default();
    let mut client = ClaudeSDKClient::new(options, None).await?;

    client.send_message("Hello").await?;

    let mut messages = Box::pin(client.receive_response());
    while let Some(msg) = messages.next().await {
        match msg? {
            Message::Assistant { message, .. } => println!("{:?}", message),
            Message::Result { .. } => println!("Done!"),
            _ => {}
        }
    }

    Ok(())
}
```

#### Interrupt

```rust
use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
use tokio::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = ClaudeAgentOptions::default();
    let mut client = ClaudeSDKClient::new(options, None).await?;

    client.send_message("Write a long essay").await?;

    // After some time, interrupt
    tokio::time::sleep(Duration::from_millis(500)).await;
    client.interrupt().await?;

    Ok(())
}
```

#### Cancellation

```rust
use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = ClaudeAgentOptions::default();
    let mut client = ClaudeSDKClient::new(options, None).await?;

    let cancel_token = client.cancellation_token();

    // Use in a spawned task
    let token = cancel_token.clone();
    tokio::spawn(async move {
        tokio::select! {
            _ = token.cancelled() => {
                println!("Operation cancelled");
            }
            _ = async { /* long operation */ } => {
                println!("Operation completed");
            }
        }
    });

    // Later, cancel all operations
    client.cancel();
    client.close().await?;

    Ok(())
}
```

#### Message Buffering

The CLI only reads stdin between turns (after Result, before next user message). Messages sent during streaming are ignored. The SDK provides built-in buffering to handle multi-turn conversations:

```rust
use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = ClaudeAgentOptions::builder()
        .max_turns(10)
        .build();
    let mut client = ClaudeSDKClient::new(options, None).await?;

    // Send first message
    client.send_message("What is Python?").await?;

    // Queue follow-up messages (sent automatically after each Result)
    client.queue_message("What is TypeScript?");
    client.queue_message("Compare Rust to both.");

    let total = 1 + client.queued_count(); // Track expected turns
    let mut turn = 0;

    // next_buffered() auto-sends queued messages after each Result
    while let Some(msg) = client.next_buffered().await {
        match msg? {
            Message::Assistant { message, .. } => {
                println!("Claude: {:?}", message.content);
            }
            Message::Result { .. } => {
                turn += 1;
                if turn >= total {
                    break;
                }
            }
            _ => {}
        }
    }

    client.close().await?;
    Ok(())
}
```

**Security**: Each queued message is associated with the session_id at queue time. When sending, the SDK verifies the session hasn't changed - if it has, the message is discarded and the queue is cleared to prevent messages from being sent to an unintended conversation context.

| Method                   | Returns                   | Description                                     |
| :----------------------- | :------------------------ | :---------------------------------------------- |
| `queue_message(content)` | `()`                      | Add message to buffer with current session_id   |
| `next_buffered()`        | `Option<Result<Message>>` | Get next message, auto-send queued after Result |
| `queued_count()`         | `usize`                   | Number of messages waiting in queue             |
| `has_queued()`           | `bool`                    | Check if queue has pending messages             |
| `send_queued()`          | `Result<bool>`            | Manually send next queued message               |
| `clear_queue()`          | `()`                      | Clear all queued messages                       |

#### Session Binding (Secure by Default)

Sessions are **automatically bound** on first Result message for security:

```rust
use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = ClaudeAgentOptions::default();
    let mut client = ClaudeSDKClient::new(options, None).await?;

    client.send_message("Hello").await?;

    // First Result auto-binds the session
    while let Some(msg) = client.next_message().await {
        if let Message::Result { .. } = msg? {
            // Session is now auto-bound!
            break;
        }
    }

    // All subsequent sends are validated automatically
    // Returns ClaudeError::SessionMismatch if session changed
    client.send_message("Follow-up").await?;

    client.close().await?;
    Ok(())
}
```

| Method                     | Returns             | Description                                                    |
| :------------------------- | :------------------ | :------------------------------------------------------------- |
| `bind_session(session_id)` | `()`                | Override auto-bound session                                    |
| `bound_session()`          | `Option<SessionId>` | Get bound session ID                                           |
| `unbind_session()`         | `()`                | Clear binding (for multi-session scenarios)                    |
| `validate_session()`       | `Result<()>`        | Validate current matches bound (auto-called by `send_message`) |

**Behavior:**

- Auto-binds on first Result (secure by default)
- Returns `Ok(())` if sessions match or either is None (early in conversation)
- Returns `Err(ClaudeError::SessionMismatch)` if bound ≠ current

#### Introspection

```rust
use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = ClaudeAgentOptions::default();
    let client = ClaudeSDKClient::new(options, None).await?;

    // Session information (available after init message)
    if let Some(info) = client.session_info() {
        println!("Model: {:?}", info.model);
        println!("Tools: {:?}", info.tool_names());

        for server in &info.mcp_servers {
            println!("MCP: {} ({})", server.name, server.status);
        }
    }

    // Static model list
    for model in ClaudeSDKClient::supported_models() {
        println!("{}: {:?}", model.id, model.name);
    }

    // Slash commands
    for cmd in client.supported_commands() {
        println!("/{} - {}", cmd.name, cmd.description);
    }

    Ok(())
}
```

## Types

### `ClaudeAgentOptions`

Configuration options for the SDK. Uses the builder pattern via `typed-builder`.

```rust
let options = ClaudeAgentOptions::builder()
    .system_prompt("You are a helpful assistant")
    .model("sonnet")
    .max_turns(10)
    .build();
```

| Property                             | Type                                           | Default            | Description                                                  |
| :----------------------------------- | :--------------------------------------------- | :----------------- | :----------------------------------------------------------- |
| `allowed_tools`                      | `Vec<ToolName>`                                | `[]`               | Tools Claude is allowed to use                               |
| `system_prompt`                      | `Option<SystemPrompt>`                         | `None`             | System prompt configuration                                  |
| `mcp_servers`                        | `McpServers`                                   | `McpServers::None` | MCP server configurations                                    |
| `permission_mode`                    | `Option<PermissionMode>`                       | `None`             | Permission mode for tool execution                           |
| `continue_conversation`              | `bool`                                         | `false`            | Continue from previous conversation                          |
| `resume`                             | `Option<SessionId>`                            | `None`             | Session ID to resume                                         |
| `max_turns`                          | `Option<u32>`                                  | `None`             | Maximum conversation turns                                   |
| `disallowed_tools`                   | `Vec<ToolName>`                                | `[]`               | Tools Claude cannot use                                      |
| `model`                              | `Option<String>`                               | `None`             | Claude model to use                                          |
| `permission_prompt_tool_name`        | `Option<String>`                               | `None`             | MCP tool name for permission prompts                         |
| `cwd`                                | `Option<PathBuf>`                              | `None`             | Working directory                                            |
| `settings`                           | `Option<PathBuf>`                              | `None`             | Path to settings file                                        |
| `add_dirs`                           | `Vec<PathBuf>`                                 | `[]`               | Additional directories Claude can access                     |
| `env`                                | `HashMap<String, String>`                      | `{}`               | Environment variables (dangerous vars blocked, see Security) |
| `extra_args`                         | `HashMap<String, Option<String>>`              | `{}`               | Additional CLI arguments (allowlist enforced, see Security)  |
| `max_buffer_size`                    | `Option<usize>`                                | `None`             | Maximum buffer size (default: 1MB)                           |
| `read_timeout_secs`                  | `Option<u64>`                                  | `None`             | Read timeout in seconds (default: 120)                       |
| `can_use_tool`                       | `Option<CanUseToolCallback>`                   | `None`             | Custom permission callback                                   |
| `hooks`                              | `Option<HashMap<HookEvent, Vec<HookMatcher>>>` | `None`             | Hook configurations                                          |
| `user`                               | `Option<String>`                               | `None`             | User identifier                                              |
| `include_partial_messages`           | `bool`                                         | `false`            | Include partial message events                               |
| `fork_session`                       | `bool`                                         | `false`            | Fork session when resuming                                   |
| `session_id`                         | `Option<String>`                               | `None`             | Custom session ID (must be valid UUID)                       |
| `agents`                             | `Option<HashMap<String, AgentDefinition>>`     | `None`             | Custom agent definitions                                     |
| `setting_sources`                    | `Option<Vec<SettingSource>>`                   | `None`             | Settings sources to load                                     |
| `max_budget_usd`                     | `Option<f64>`                                  | `None`             | Maximum budget in USD                                        |
| `max_thinking_tokens`                | `Option<u32>`                                  | `None`             | Maximum tokens for thinking                                  |
| `fallback_model`                     | `Option<String>`                               | `None`             | Model to use if primary fails                                |
| `output_format`                      | `Option<OutputFormat>`                         | `None`             | Structured output format                                     |
| `sandbox`                            | `Option<SandboxSettings>`                      | `None`             | Sandbox configuration                                        |
| `plugins`                            | `Option<Vec<SdkPluginConfig>>`                 | `None`             | Plugins to load                                              |
| `betas`                              | `Option<Vec<SdkBeta>>`                         | `None`             | Beta features to enable                                      |
| `strict_mcp_config`                  | `bool`                                         | `false`            | Enforce strict MCP validation                                |
| `resume_session_at`                  | `Option<String>`                               | `None`             | Resume at specific message UUID                              |
| `allow_dangerously_skip_permissions` | `bool`                                         | `false`            | Allow bypassing permissions                                  |
| `path_to_claude_code_executable`     | `Option<PathBuf>`                              | `None`             | Custom CLI path                                              |
| `stderr`                             | `Option<StderrCallback>`                       | `None`             | Callback for stderr output                                   |
| `tools`                              | `Option<ToolsConfig>`                          | `None`             | Tools configuration                                          |

### `SystemPrompt`

System prompt configuration.

```rust
pub enum SystemPrompt {
    String(String),
    Preset(SystemPromptPreset),
}
```

#### `SystemPromptPreset`

```rust
pub struct SystemPromptPreset {
    pub prompt_type: String,  // Always "preset"
    pub preset: String,       // e.g., "claude_code"
    pub append: Option<String>,
}
```

### `ToolsConfig`

Tools configuration - list or preset.

```rust
pub enum ToolsConfig {
    List(Vec<ToolName>),
    Preset(ToolsPreset),
}
```

```rust
// Use Claude Code's default tools
let config = ToolsConfig::claude_code_preset();

// Or specify explicit tools
let config = ToolsConfig::from_list(vec![
    ToolName::from("Read"),
    ToolName::from("Write"),
]);
```

### `OutputFormat`

Structured output format configuration.

```rust
pub struct OutputFormat {
    pub format_type: String,        // Always "json_schema"
    pub schema: serde_json::Value,
}
```

```rust
let format = OutputFormat::json_schema(serde_json::json!({
    "type": "object",
    "properties": {
        "answer": { "type": "string" }
    }
}));
```

### `SandboxSettings`

Configuration for command sandboxing.

```rust
pub struct SandboxSettings {
    pub enabled: Option<bool>,
    pub auto_allow_bash_if_sandboxed: Option<bool>,
    pub excluded_commands: Option<Vec<String>>,
    pub allow_unsandboxed_commands: Option<bool>,
    pub network: Option<NetworkSandboxSettings>,
    pub ignore_violations: Option<SandboxIgnoreViolations>,
    pub enable_weaker_nested_sandbox: Option<bool>,
}
```

| Property                       | Type                              | Default | Description                                  |
| :----------------------------- | :-------------------------------- | :------ | :------------------------------------------- |
| `enabled`                      | `Option<bool>`                    | `None`  | Enable sandbox mode                          |
| `auto_allow_bash_if_sandboxed` | `Option<bool>`                    | `None`  | Auto-approve bash when sandboxed             |
| `excluded_commands`            | `Option<Vec<String>>`             | `None`  | Commands that bypass sandbox                 |
| `allow_unsandboxed_commands`   | `Option<bool>`                    | `None`  | Allow model to request unsandboxed execution |
| `network`                      | `Option<NetworkSandboxSettings>`  | `None`  | Network restrictions                         |
| `ignore_violations`            | `Option<SandboxIgnoreViolations>` | `None`  | Violations to ignore                         |
| `enable_weaker_nested_sandbox` | `Option<bool>`                    | `None`  | Weaker nested sandbox for compatibility      |

### `NetworkSandboxSettings`

Network-specific sandbox configuration.

```rust
pub struct NetworkSandboxSettings {
    pub allow_local_binding: Option<bool>,
    pub allow_unix_sockets: Option<Vec<String>>,
    pub allow_all_unix_sockets: Option<bool>,
    pub http_proxy_port: Option<u16>,
    pub socks_proxy_port: Option<u16>,
}
```

### `SandboxIgnoreViolations`

Violations to ignore in sandbox mode.

```rust
pub struct SandboxIgnoreViolations {
    pub file: Option<Vec<String>>,
    pub network: Option<Vec<String>>,
}
```

### `SdkPluginConfig`

Plugin configuration.

```rust
pub enum SdkPluginConfig {
    Local { path: String },
}
```

### `SdkBeta`

Available beta features.

```rust
pub enum SdkBeta {
    Context1M,  // 1 million token context window
}
```

| Value       | Description                    | Compatible Models                  |
| :---------- | :----------------------------- | :--------------------------------- |
| `Context1M` | 1 million token context window | Claude Sonnet 4, Claude Sonnet 4.5 |

### `AgentDefinition`

Configuration for custom subagents.

```rust
pub struct AgentDefinition {
    pub description: String,
    pub prompt: String,
    pub tools: Option<Vec<String>>,
    pub model: Option<String>,
}
```

| Field         | Required | Description                                      |
| :------------ | :------- | :----------------------------------------------- |
| `description` | Yes      | When to use this agent                           |
| `prompt`      | Yes      | Agent's system prompt                            |
| `tools`       | No       | Allowed tools (inherits all if omitted)          |
| `model`       | No       | Model override (`"sonnet"`, `"opus"`, `"haiku"`) |

### `SettingSource`

Configuration sources to load.

```rust
pub enum SettingSource {
    User,     // ~/.claude/settings.json
    Project,  // .claude/settings.json
    Local,    // .claude/settings.local.json
}
```

When `setting_sources` is omitted, no filesystem settings are loaded.

```rust
// Load project settings only
let options = ClaudeAgentOptions::builder()
    .setting_sources(vec![SettingSource::Project])
    .build();

// Load all settings
let options = ClaudeAgentOptions::builder()
    .setting_sources(vec![
        SettingSource::User,
        SettingSource::Project,
        SettingSource::Local,
    ])
    .build();
```

### `PermissionMode`

Permission modes for tool execution.

```rust
pub enum PermissionMode {
    Default,           // Prompt for dangerous tools
    AcceptEdits,       // Auto-accept file edits
    Plan,              // Plan mode - no execution
    BypassPermissions, // Allow all (requires allow_dangerously_skip_permissions)
}
```

### `CanUseToolCallback`

Type alias for permission callback.

```rust
pub type CanUseToolCallback = Arc<dyn PermissionCallback>;
```

### `PermissionResult`

Result of a permission check.

```rust
pub enum PermissionResult {
    Allow(PermissionResultAllow),
    Deny(PermissionResultDeny),
}
```

#### `PermissionResultAllow`

```rust
pub struct PermissionResultAllow {
    pub updated_input: Option<serde_json::Value>,
    pub updated_permissions: Option<Vec<PermissionUpdate>>,
}
```

#### `PermissionResultDeny`

```rust
pub struct PermissionResultDeny {
    pub message: String,
    pub interrupt: bool,
}
```

### `McpServerConfig`

MCP server configuration.

```rust
pub enum McpServerConfig {
    Stdio(McpStdioServerConfig),
    Sse(McpSseServerConfig),
    Http(McpHttpServerConfig),
    Sdk(SdkMcpServerConfig),
}
```

#### `McpStdioServerConfig`

```rust
pub struct McpStdioServerConfig {
    pub server_type: Option<String>,  // "stdio"
    pub command: String,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
}
```

#### `McpSseServerConfig`

```rust
pub struct McpSseServerConfig {
    pub server_type: String,  // "sse"
    pub url: String,
    pub headers: Option<HashMap<String, String>>,
}
```

#### `McpHttpServerConfig`

```rust
pub struct McpHttpServerConfig {
    pub server_type: String,  // "http"
    pub url: String,
    pub headers: Option<HashMap<String, String>>,
}
```

#### `SdkMcpServerConfig`

```rust
pub struct SdkMcpServerConfig {
    pub name: String,
    pub version: Option<String>,
}
```

### `McpServers`

MCP servers container.

```rust
pub enum McpServers {
    None,
    Dict(HashMap<String, McpServerConfig>),
    Path(PathBuf),
}
```

```rust
use std::collections::HashMap;

let mut servers = HashMap::new();
servers.insert("my-server".to_string(), McpServerConfig::Stdio(
    McpStdioServerConfig {
        server_type: Some("stdio".to_string()),
        command: "npx".to_string(),
        args: Some(vec!["-y".to_string(), "@my/mcp-server".to_string()]),
        env: None,
    }
));

let options = ClaudeAgentOptions::builder()
    .mcp_servers(McpServers::Dict(servers))
    .build();
```

## Message Types

### `Message`

Message types returned from Claude Code.

```rust
pub enum Message {
    User { ... },
    Assistant { ... },
    System { ... },
    Result { ... },
    StreamEvent { ... },
}
```

#### `Message::User`

User message sent to Claude.

| Field                | Type                 | Description                                 |
| :------------------- | :------------------- | :------------------------------------------ |
| `parent_tool_use_id` | `Option<String>`     | Parent tool use ID for nested conversations |
| `message`            | `UserMessageContent` | Message content                             |
| `session_id`         | `Option<SessionId>`  | Session identifier                          |

#### `Message::Assistant`

Response from Claude.

| Field                | Type                      | Description                                 |
| :------------------- | :------------------------ | :------------------------------------------ |
| `parent_tool_use_id` | `Option<String>`          | Parent tool use ID for nested conversations |
| `message`            | `AssistantMessageContent` | Message content                             |
| `session_id`         | `Option<SessionId>`       | Session identifier                          |

#### `Message::System`

System messages (init, status updates).

| Field     | Type                | Description                      |
| :-------- | :------------------ | :------------------------------- |
| `subtype` | `String`            | Message subtype (e.g., `"init"`) |
| `data`    | `serde_json::Value` | System message data              |

#### `Message::Result`

Conversation result with metrics.

| Field                | Type                          | Description                                                                   |
| :------------------- | :---------------------------- | :---------------------------------------------------------------------------- |
| `subtype`            | `String`                      | Result subtype (`"success"`, `"error_max_turns"`, `"error_during_execution"`) |
| `duration_ms`        | `u64`                         | Total duration in milliseconds                                                |
| `duration_api_ms`    | `u64`                         | API call duration in milliseconds                                             |
| `is_error`           | `bool`                        | Whether this is an error result                                               |
| `num_turns`          | `u32`                         | Number of conversation turns                                                  |
| `session_id`         | `SessionId`                   | Session identifier                                                            |
| `total_cost_usd`     | `Option<f64>`                 | Total cost in USD                                                             |
| `usage`              | `Option<serde_json::Value>`   | Aggregate token usage                                                         |
| `result`             | `Option<String>`              | Result message (for success)                                                  |
| `model_usage`        | `HashMap<String, ModelUsage>` | Per-model usage statistics                                                    |
| `permission_denials` | `Vec<SDKPermissionDenial>`    | Denied tool uses                                                              |
| `structured_output`  | `Option<serde_json::Value>`   | Structured output (when `output_format` specified)                            |
| `errors`             | `Vec<String>`                 | Error messages                                                                |

#### `Message::StreamEvent`

Partial message events (requires `include_partial_messages: true`).

| Field                | Type                | Description           |
| :------------------- | :------------------ | :-------------------- |
| `uuid`               | `String`            | Event UUID            |
| `session_id`         | `SessionId`         | Session identifier    |
| `event`              | `serde_json::Value` | Raw stream event data |
| `parent_tool_use_id` | `Option<String>`    | Parent tool use ID    |

### `ContentBlock`

Content blocks in assistant messages.

```rust
pub enum ContentBlock {
    Text { text: String },
    Thinking { thinking: String, signature: String },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_use_id: String, content: Option<ContentValue>, is_error: Option<bool> },
}
```

| Variant      | Description                                              |
| :----------- | :------------------------------------------------------- |
| `Text`       | Text content from Claude                                 |
| `Thinking`   | Extended thinking block (when `max_thinking_tokens` set) |
| `ToolUse`    | Tool invocation request                                  |
| `ToolResult` | Result from tool execution                               |

### `ContentValue`

Content value for tool results.

```rust
pub enum ContentValue {
    String(String),
    Blocks(Vec<serde_json::Value>),
}
```

### AskUserQuestion Tool Types

Types for the `AskUserQuestion` tool which allows agents to ask users multiple-choice questions.

#### `QuestionOption`

```rust
pub struct QuestionOption {
    pub label: String,
    pub description: String,
}
```

| Field         | Type     | Description                           |
| :------------ | :------- | :------------------------------------ |
| `label`       | `String` | Display text for this option          |
| `description` | `String` | Explanation of what this option means |

#### `QuestionSpec`

```rust
pub struct QuestionSpec {
    pub question: String,
    pub header: String,
    pub options: Vec<QuestionOption>,
    pub multi_select: bool,
}
```

| Field          | Type                  | Description                                        |
| :------------- | :-------------------- | :------------------------------------------------- |
| `question`     | `String`              | The complete question to ask the user              |
| `header`       | `String`              | Short label displayed as a chip/tag (max 12 chars) |
| `options`      | `Vec<QuestionOption>` | Available choices (2-4 options)                    |
| `multi_select` | `bool`                | Whether multiple options can be selected           |

#### `AskUserQuestionInput`

```rust
pub struct AskUserQuestionInput {
    pub questions: Vec<QuestionSpec>,
    pub answers: Option<HashMap<String, String>>,
}
```

| Field       | Type                              | Description                             |
| :---------- | :-------------------------------- | :-------------------------------------- |
| `questions` | `Vec<QuestionSpec>`               | Questions to ask (1-4 questions)        |
| `answers`   | `Option<HashMap<String, String>>` | User answers (populated in tool result) |

#### `AskUserQuestionOutput`

```rust
pub struct AskUserQuestionOutput {
    pub answers: HashMap<String, String>,
}
```

| Field     | Type                      | Description                             |
| :-------- | :------------------------ | :-------------------------------------- |
| `answers` | `HashMap<String, String>` | User's answers keyed by question header |

#### Example: Parsing Tool Input

```rust
use anthropic_agent_sdk::types::{AskUserQuestionInput, ContentBlock};

// When you receive a ToolUse content block
if let ContentBlock::ToolUse { name, input, .. } = block {
    if name == "AskUserQuestion" {
        let question_input: AskUserQuestionInput = serde_json::from_value(input)?;
        for q in &question_input.questions {
            println!("Q: {} (header: {})", q.question, q.header);
            for opt in &q.options {
                println!("  - {}: {}", opt.label, opt.description);
            }
        }
    }
}
```

### `UserMessageContent`

User message content wrapper.

```rust
pub struct UserMessageContent {
    pub role: String,  // Always "user"
    pub content: Option<UserContent>,
}
```

### `UserContent`

User content variants.

```rust
pub enum UserContent {
    String(String),
    Blocks(Vec<ContentBlock>),
}
```

### `AssistantMessageContent`

Assistant message content.

```rust
pub struct AssistantMessageContent {
    pub model: String,
    pub content: Vec<ContentBlock>,
}
```

### `SDKPermissionDenial`

Information about denied tool use.

```rust
pub struct SDKPermissionDenial {
    pub tool_name: String,
    pub tool_use_id: String,
    pub tool_input: serde_json::Value,
}
```

### `ModelUsage`

Per-model usage statistics from Result messages.

```rust
pub struct ModelUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub web_search_requests: u64,
    pub cost_usd: f64,
    pub context_window: u64,
}
```

| Method                     | Returns | Description                                |
| :------------------------- | :------ | :----------------------------------------- |
| `total_tokens()`           | `u64`   | Input + output tokens                      |
| `effective_input_tokens()` | `u64`   | Input + cache read + cache creation tokens |

## Working with Tools

Claude uses tools to interact with the filesystem, run commands, and perform actions.
The SDK provides options, hooks, and callbacks to control tool usage.

### Tool Flow

1. Claude decides to use a tool → `ContentBlock::ToolUse` appears in message stream
2. (Optional) `PreToolUse` hook fires → can block or modify input
3. (Optional) `PermissionCallback` fires → can allow/deny
4. CLI executes the tool
5. (Optional) `PostToolUse` hook fires → can observe result
6. `ContentBlock::ToolResult` appears in message stream

### Restricting Tools via Options

Use `allowed_tools` and `disallowed_tools` to control which tools Claude can use:

```rust
// Allow only read operations
let options = ClaudeAgentOptions::builder()
    .allowed_tools(vec!["Read".into(), "Glob".into(), "Grep".into()])
    .build();

// Allow all except dangerous tools
let options = ClaudeAgentOptions::builder()
    .disallowed_tools(vec!["Bash".into(), "Write".into()])
    .build();
```

| Option             | Type            | Description                              |
| :----------------- | :-------------- | :--------------------------------------- |
| `allowed_tools`    | `Vec<ToolName>` | Whitelist - only these tools can be used |
| `disallowed_tools` | `Vec<ToolName>` | Blacklist - these tools cannot be used   |

When both are specified, `allowed_tools` takes precedence.

### Detecting Tool Use

```rust
while let Some(msg) = client.next_message().await? {
    if let Message::Assistant { content, .. } = &msg {
        for block in content {
            match block {
                ContentBlock::ToolUse { id, name, input } => {
                    println!("Tool: {} ({})", name, id);
                    println!("Input: {}", input);
                }
                ContentBlock::ToolResult { tool_use_id, content, is_error } => {
                    println!("Result for {}: {:?}", tool_use_id, content);
                    if is_error.unwrap_or(false) {
                        println!("Tool execution failed");
                    }
                }
                _ => {}
            }
        }
    }
}
```

### Intercepting Tool Use

Use hooks to intercept, modify, or block tool execution:

```rust
// Block all Bash commands containing "rm"
struct BlockDangerousBash;

#[async_trait]
impl HookCallback for BlockDangerousBash {
    async fn call(
        &self,
        input: serde_json::Value,
        _tool_use_id: Option<String>,
        _ctx: HookContext,
    ) -> Result<HookOutput> {
        if let Some(cmd) = input.get("command").and_then(|c| c.as_str()) {
            if cmd.contains("rm ") {
                return Ok(HookOutput {
                    decision: Some(HookDecision::Block),
                    reason: Some("Dangerous command blocked".into()),
                    ..Default::default()
                });
            }
        }
        Ok(HookOutput::default())
    }
}

// Register for Bash tool only
let hook = HookMatcher::builder()
    .event(HookEvent::PreToolUse)
    .tool_name("Bash")
    .build();

hook_manager.register(hook, Arc::new(BlockDangerousBash));
```

### Dynamic Permission Control

Use `PermissionCallback` for runtime decisions:

```rust
struct AuditingPermissionHandler;

#[async_trait]
impl PermissionCallback for AuditingPermissionHandler {
    async fn call(
        &self,
        tool_name: String,
        input: serde_json::Value,
        _ctx: ToolPermissionContext,
    ) -> Result<PermissionResult> {
        // Log all tool usage
        println!("[AUDIT] Tool: {}, Input: {}", tool_name, input);

        // Allow read tools, deny write tools
        if matches!(tool_name.as_str(), "Read" | "Glob" | "Grep") {
            Ok(PermissionResult::Allow(PermissionResultAllow {
                updated_input: None,
                updated_permissions: None,
            }))
        } else {
            Ok(PermissionResult::Deny(PermissionResultDeny {
                message: format!("Tool '{}' not allowed", tool_name),
                interrupt: false,
            }))
        }
    }
}

let options = ClaudeAgentOptions::builder()
    .can_use_tool(Arc::new(AuditingPermissionHandler))
    .build();
```

See [Hook Types](#hook-types) for `PreToolUse`/`PostToolUse` details.
See [Permission Types](#permission-types) for `PermissionCallback` trait.
See [Claude Code CLI Reference](https://docs.anthropic.com/en/docs/claude-code/cli) for tool schemas.

## Hook Types

### `HookEvent`

Hook event types for intercepting agent actions.

```rust
pub enum HookEvent {
    PreToolUse,        // Before a tool is used
    PostToolUse,       // After a tool is used
    PostToolUseFailure,// After a tool use fails
    Notification,      // When a notification is received
    UserPromptSubmit,  // When user submits a prompt
    SessionStart,      // When a session starts
    SessionEnd,        // When a session ends
    Stop,              // When conversation stops
    SubagentStart,     // When a subagent starts
    SubagentStop,      // When a subagent stops
    PreCompact,        // Before compacting the conversation
    PermissionRequest, // When a permission is requested
}
```

### `HookCallback`

Trait for hook callbacks.

```rust
#[async_trait]
pub trait HookCallback: Send + Sync {
    async fn call(
        &self,
        input: serde_json::Value,
        tool_use_id: Option<String>,
        context: HookContext,
    ) -> Result<HookOutput>;
}
```

#### Implementing with a struct

```rust
use anthropic_agent_sdk::callbacks::HookCallback;
use anthropic_agent_sdk::types::{HookOutput, HookContext};
use async_trait::async_trait;

struct LoggingHook;

#[async_trait]
impl HookCallback for LoggingHook {
    async fn call(
        &self,
        input: serde_json::Value,
        tool_use_id: Option<String>,
        _context: HookContext,
    ) -> Result<HookOutput> {
        println!("Tool called: {:?}", tool_use_id);
        Ok(HookOutput::default())
    }
}
```

#### Implementing with a closure

```rust
use anthropic_agent_sdk::hooks::HookManager;

let hook = HookManager::callback(|input, tool_name, ctx| async move {
    println!("Tool: {:?}, Session: {:?}", tool_name, ctx.session_id);
    Ok(HookOutput::default())
});
```

### `HookMatcher`

Hook configuration with pattern matching.

```rust
pub struct HookMatcher {
    pub matcher: Option<String>,
    pub hooks: Vec<Arc<dyn HookCallback>>,
    pub timeout: Option<Duration>,  // Default: 60 seconds
}
```

| Field     | Type                         | Description                                                                   |
| :-------- | :--------------------------- | :---------------------------------------------------------------------------- |
| `matcher` | `Option<String>`             | Pattern (`None` for all, `"Bash"` for specific, `"Write\|Edit"` for multiple) |
| `hooks`   | `Vec<Arc<dyn HookCallback>>` | Hook callbacks to invoke                                                      |
| `timeout` | `Option<Duration>`           | Timeout per hook (default: 60 seconds)                                        |

### `HookMatcherBuilder`

Builder for creating `HookMatcher` instances.

```rust
use anthropic_agent_sdk::hooks::HookMatcherBuilder;
use std::time::Duration;

let matcher = HookMatcherBuilder::new(Some("Bash"))
    .add_hook(my_hook)
    .timeout(Duration::from_secs(30))
    .build();
```

| Method              | Description                          |
| :------------------ | :----------------------------------- |
| `new(pattern)`      | Create builder with optional pattern |
| `add_hook(hook)`    | Add a hook callback                  |
| `timeout(duration)` | Set timeout for hooks                |
| `build()`           | Build the `HookMatcher`              |

### `HookContext`

Context passed to hook callbacks.

```rust
pub struct HookContext {
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    pub cancellation_token: Option<CancellationToken>,
}
```

| Field                | Type                        | Description                    |
| :------------------- | :-------------------------- | :----------------------------- |
| `session_id`         | `Option<String>`            | Session identifier             |
| `cwd`                | `Option<String>`            | Current working directory      |
| `cancellation_token` | `Option<CancellationToken>` | Token for cancellation support |

| Method           | Returns | Description                         |
| :--------------- | :------ | :---------------------------------- |
| `is_cancelled()` | `bool`  | Check if cancellation was requested |

### `HookOutput`

Output from a hook callback.

```rust
pub struct HookOutput {
    pub decision: Option<HookDecision>,
    pub system_message: Option<String>,
    pub hook_specific_output: Option<serde_json::Value>,
}
```

| Field                  | Type                        | Description                      |
| :--------------------- | :-------------------------- | :------------------------------- |
| `decision`             | `Option<HookDecision>`      | Block or allow the action        |
| `system_message`       | `Option<String>`            | Message to add to system context |
| `hook_specific_output` | `Option<serde_json::Value>` | Hook-specific data               |

### `HookDecision`

Decision for hook callbacks.

```rust
pub enum HookDecision {
    Block,  // Block the action
}
```

Omit `decision` or set to `None` to allow the action.

### Hook Input Types

All hook inputs include `BaseHookInput`:

```rust
pub struct BaseHookInput {
    pub session_id: String,
    pub transcript_path: String,
    pub cwd: String,
    pub permission_mode: Option<String>,
}
```

| Hook Event           | Input Type                    | Additional Fields                                                   |
| :------------------- | :---------------------------- | :------------------------------------------------------------------ |
| `PreToolUse`         | `PreToolUseHookInput`         | `tool_name`, `tool_input`                                           |
| `PostToolUse`        | `PostToolUseHookInput`        | `tool_name`, `tool_input`, `tool_response`, `tool_use_id?`          |
| `PostToolUseFailure` | `PostToolUseFailureHookInput` | `tool_name`, `tool_input`, `tool_use_id?`, `error`, `is_interrupt?` |
| `SessionStart`       | `SessionStartHookInput`       | `source: SessionStartSource`                                        |
| `SessionEnd`         | `SessionEndHookInput`         | `reason: SessionEndReason`                                          |
| `SubagentStart`      | `SubagentStartHookInput`      | `agent_id`, `agent_type`                                            |
| `SubagentStop`       | `SubagentStopHookInput`       | `agent_id?`, `agent_transcript_path?`, `stop_hook_active`           |
| `UserPromptSubmit`   | `UserPromptSubmitHookInput`   | `prompt`                                                            |
| `Notification`       | `NotificationHookInput`       | `message`, `title?`                                                 |
| `PreCompact`         | `PreCompactHookInput`         | `trigger: CompactTrigger`, `custom_instructions?`                   |
| `PermissionRequest`  | `PermissionRequestHookInput`  | `tool_name`, `tool_input`, `permission_suggestions?`                |
| `Stop`               | `StopHookInput`               | `stop_hook_active`                                                  |

**Note:** Fields marked with `?` are optional (may be absent in CLI 2.0.75+).

### `SessionStartSource`

```rust
pub enum SessionStartSource {
    Startup,  // Fresh startup
    Resume,   // Resumed session
    Clear,    // After clear
    Compact,  // After compact
}
```

### `SessionEndReason`

```rust
pub enum SessionEndReason {
    Clear,           // Session cleared
    Logout,          // User logged out
    PromptInputExit, // User exited prompt input
    Other,           // Other reason
}
```

### `CompactTrigger`

```rust
pub enum CompactTrigger {
    Manual,  // Manually triggered
    Auto,    // Automatically triggered
}
```

### `HookManager`

Manager for registering and invoking hooks.

```rust
use anthropic_agent_sdk::hooks::{HookManager, HookMatcherBuilder};
use anthropic_agent_sdk::types::HookEvent;

let mut manager = HookManager::new();

// Register a hook for PreToolUse events on Bash
let hook = HookManager::callback(|input, tool_name, ctx| async move {
    println!("Bash called: {:?}", input);
    Ok(HookOutput::default())
});

manager.register_for_event(
    HookEvent::PreToolUse,
    HookMatcherBuilder::new(Some("Bash")).add_hook(hook).build(),
);
```

| Method                                 | Description                                   |
| :------------------------------------- | :-------------------------------------------- |
| `new()`                                | Create a new hook manager                     |
| `from_hooks_config(config)`            | Create from a hooks configuration HashMap     |
| `register_for_event(event, matcher)`   | Register a matcher for an event               |
| `has_hooks_for(event)`                 | Check if hooks are registered for an event    |
| `invoke(event, data, tool_name, ctx)`  | Invoke hooks for an event                     |
| `set_session_context(session_id, cwd)` | Set session context                           |
| `set_cancellation_token(token)`        | Set cancellation token                        |
| `build_context()`                      | Build a HookContext with current session info |
| `callback(f)`                          | Create a hook callback from a closure         |

## Permission Types

### `PermissionCallback`

Trait for permission callbacks.

```rust
#[async_trait]
pub trait PermissionCallback: Send + Sync {
    async fn call(
        &self,
        tool_name: String,
        input: serde_json::Value,
        context: ToolPermissionContext,
    ) -> Result<PermissionResult>;
}
```

#### Implementing with a struct

```rust
use anthropic_agent_sdk::callbacks::PermissionCallback;
use anthropic_agent_sdk::types::{PermissionResult, PermissionResultAllow, PermissionResultDeny, ToolPermissionContext};
use async_trait::async_trait;

struct AllowReadOnly;

#[async_trait]
impl PermissionCallback for AllowReadOnly {
    async fn call(
        &self,
        tool_name: String,
        _input: serde_json::Value,
        _context: ToolPermissionContext,
    ) -> Result<PermissionResult> {
        if tool_name == "Read" || tool_name == "Glob" {
            Ok(PermissionResult::Allow(PermissionResultAllow {
                updated_input: None,
                updated_permissions: None,
            }))
        } else {
            Ok(PermissionResult::Deny(PermissionResultDeny {
                message: "Only read operations allowed".to_string(),
                interrupt: false,
            }))
        }
    }
}
```

#### Implementing with a closure

```rust
use anthropic_agent_sdk::permissions::PermissionManager;

let callback = PermissionManager::callback(|tool_name, input, ctx| async move {
    // ctx.suggestions contains permission suggestions from CLI
    // ctx.is_cancelled() checks if operation was cancelled
    Ok(PermissionResult::Allow(PermissionResultAllow::default()))
});
```

### `ToolPermissionContext`

Context passed to permission callbacks.

```rust
pub struct ToolPermissionContext {
    pub suggestions: Vec<PermissionUpdate>,
    pub cancellation_token: Option<CancellationToken>,
}
```

| Field                | Type                        | Description                     |
| :------------------- | :-------------------------- | :------------------------------ |
| `suggestions`        | `Vec<PermissionUpdate>`     | Permission suggestions from CLI |
| `cancellation_token` | `Option<CancellationToken>` | Token for cancellation support  |

| Method                                  | Returns | Description                         |
| :-------------------------------------- | :------ | :---------------------------------- |
| `new(suggestions)`                      | `Self`  | Create with suggestions             |
| `with_cancellation(suggestions, token)` | `Self`  | Create with cancellation token      |
| `is_cancelled()`                        | `bool`  | Check if cancellation was requested |

### `PermissionUpdate`

Permission update operations.

```rust
pub enum PermissionUpdate {
    AddRules { rules: Option<Vec<PermissionRuleValue>>, destination: Option<PermissionUpdateDestination> },
    ReplaceRules { rules: Option<Vec<PermissionRuleValue>>, destination: Option<PermissionUpdateDestination> },
    RemoveRules { rules: Option<Vec<PermissionRuleValue>>, destination: Option<PermissionUpdateDestination> },
    SetMode { mode: PermissionMode, destination: Option<PermissionUpdateDestination> },
    AddDirectories { directories: Option<Vec<String>>, destination: Option<PermissionUpdateDestination> },
    RemoveDirectories { directories: Option<Vec<String>>, destination: Option<PermissionUpdateDestination> },
}
```

### `PermissionUpdateDestination`

Where to save permission updates.

```rust
pub enum PermissionUpdateDestination {
    UserSettings,    // ~/.claude/settings.json
    ProjectSettings, // .claude/settings.json
    LocalSettings,   // .claude/settings.local.json
    Session,         // Session only (temporary)
}
```

### `PermissionBehavior`

Permission behavior options.

```rust
pub enum PermissionBehavior {
    Allow,
    Deny,
    Ask,
}
```

### `PermissionRuleValue`

Permission rule definition.

```rust
pub struct PermissionRuleValue {
    pub tool_name: String,
    pub rule_content: Option<String>,
}
```

### `PermissionRequest`

Permission request from CLI.

```rust
pub struct PermissionRequest {
    pub tool_name: ToolName,
    pub tool_input: serde_json::Value,
    pub context: ToolPermissionContext,
}
```

### `PermissionManager`

Manager for permission callbacks.

```rust
use anthropic_agent_sdk::permissions::PermissionManager;
use anthropic_agent_sdk::types::{PermissionResult, PermissionResultAllow, ToolName};

let mut manager = PermissionManager::new();

// Set allowed tools
manager.set_allowed_tools(Some(vec![
    ToolName::from("Read"),
    ToolName::from("Glob"),
]));

// Set disallowed tools
manager.set_disallowed_tools(vec![ToolName::from("Bash")]);

// Set custom callback
let callback = PermissionManager::callback(|tool_name, input, ctx| async move {
    Ok(PermissionResult::Allow(PermissionResultAllow::default()))
});
manager.set_callback(callback);
```

| Method                           | Description                              |
| :------------------------------- | :--------------------------------------- |
| `new()`                          | Create a new permission manager          |
| `set_callback(callback)`         | Set permission callback                  |
| `set_allowed_tools(tools)`       | Set allowed tools (`None` = all allowed) |
| `set_disallowed_tools(tools)`    | Set disallowed tools                     |
| `can_use_tool(name, input, ctx)` | Check if tool can be used                |
| `callback(f)`                    | Create a callback from a closure         |

## Introspection Types

### `SessionInfo`

Session information from init message.

```rust
pub struct SessionInfo {
    pub model: Option<String>,
    pub tools: Vec<ToolInfo>,
    pub cwd: Option<String>,
    pub mcp_servers: Vec<McpServerStatus>,
    pub extra: HashMap<String, serde_json::Value>,
}
```

| Method                 | Returns                    | Description                        |
| :--------------------- | :------------------------- | :--------------------------------- |
| `from_init_data(data)` | `Self`                     | Create from init message data      |
| `tool_names()`         | `Vec<&str>`                | Get tool names                     |
| `has_tool(name)`       | `bool`                     | Check if tool is available         |
| `mcp_server(name)`     | `Option<&McpServerStatus>` | Get MCP server by name             |
| `has_mcp_errors()`     | `bool`                     | Check if any MCP server has errors |
| `permission_mode()`    | `Option<&str>`             | Get current permission mode        |
| `is_plan_mode()`       | `bool`                     | Check if session is in plan mode   |

### `ToolInfo`

Tool information.

```rust
pub struct ToolInfo {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Option<serde_json::Value>,
}
```

### `McpServerStatus`

MCP server connection status.

```rust
pub struct McpServerStatus {
    pub name: String,
    pub status: String,  // "connected", "failed", "needs-auth", "pending"
    pub error: Option<String>,
    pub tools: Vec<String>,
}
```

| Method           | Returns | Description                  |
| :--------------- | :------ | :--------------------------- |
| `is_connected()` | `bool`  | Check if server is connected |

### `ModelInfo`

Claude model information.

```rust
pub struct ModelInfo {
    pub id: String,
    pub name: Option<String>,
    pub max_tokens: Option<u32>,
    pub supports_thinking: bool,
}
```

```rust
// Get known models
for model in ModelInfo::known_models() {
    println!("{}: {:?}", model.id, model.name);
}
```

### `SlashCommand`

Custom slash command information.

```rust
pub struct SlashCommand {
    pub name: String,
    pub description: String,
    pub argument_hint: String,
}
```

### `AccountInfo`

OAuth account information.

```rust
pub struct AccountInfo {
    pub email: Option<String>,
    pub account_id: Option<String>,
    pub is_oauth: bool,
    pub organization_id: Option<String>,
}
```

## Error Types

### `ClaudeError`

Main error type for the SDK.

```rust
pub enum ClaudeError {
    CliNotFound(String),
    Connection(String),
    Process { message: String, exit_code: i32, stderr: Option<String> },
    JsonDecode(serde_json::Error),
    JsonEncode(String),
    MessageParse { message: String, data: Option<serde_json::Value> },
    Transport(String),
    ControlProtocol(String),
    Hook(String),
    Mcp(String),
    Io(std::io::Error),
    Timeout(String),
    InvalidConfig(String),
    CliVersionTooOld { found: String, minimum: String },
    ControlTimeout { timeout_secs: u64, request_type: String },
    NotConnected,
    AlreadyConnected,
    AuthenticationError(String),
    NetworkError(String),
}
```

| Variant               | Description                            |
| :-------------------- | :------------------------------------- |
| `CliNotFound`         | Claude Code CLI not found              |
| `Connection`          | Connection error                       |
| `Process`             | Process execution error with exit code |
| `JsonDecode`          | JSON parsing error                     |
| `JsonEncode`          | JSON encoding error                    |
| `MessageParse`        | Message parsing error                  |
| `Transport`           | Transport layer error                  |
| `ControlProtocol`     | Control protocol error                 |
| `Hook`                | Hook execution error                   |
| `Mcp`                 | MCP error                              |
| `Io`                  | I/O error                              |
| `Timeout`             | Timeout error                          |
| `InvalidConfig`       | Invalid configuration                  |
| `CliVersionTooOld`    | CLI version below minimum              |
| `ControlTimeout`      | Control request timed out              |
| `NotConnected`        | Client not connected                   |
| `AlreadyConnected`    | Client already connected               |
| `AuthenticationError` | Authentication error                   |
| `NetworkError`        | Network error                          |

#### Helper Constructors

```rust
ClaudeError::cli_not_found()
ClaudeError::connection("Failed to connect")
ClaudeError::process("Failed", 1, Some("stderr output".to_string()))
ClaudeError::message_parse("Invalid message", Some(data))
ClaudeError::transport("Connection lost")
ClaudeError::hook("Hook failed")
ClaudeError::mcp("MCP server error")
ClaudeError::timeout("Request timed out")
ClaudeError::invalid_config("Missing required field")
ClaudeError::cli_version_too_old("1.0.0", "2.0.0")
ClaudeError::not_connected()
ClaudeError::authentication("Invalid API key")
ClaudeError::network("DNS resolution failed")
```

### `Result<T>`

Type alias for SDK results.

```rust
pub type Result<T> = std::result::Result<T, ClaudeError>;
```

## MCP Configuration

MCP (Model Context Protocol) configuration for external and in-process servers.

### External Server Configuration

Already documented in [Types > McpServerConfig](#mcpserverconfig).

### In-Process MCP Servers (feature: `rmcp`)

The `rmcp` feature enables creating in-process MCP servers with custom tools.

Enable in `Cargo.toml`:

```toml
[dependencies]
anthropic-agent-sdk = { version = "0.2", features = ["rmcp"] }
```

#### Re-exported Macros

| Macro             | Description                                       |
| :---------------- | :------------------------------------------------ |
| `#[tool_router]`  | Mark an impl block as containing tool definitions |
| `#[tool]`         | Mark a method as an MCP tool                      |
| `#[tool_handler]` | Implement ServerHandler for a type                |

#### Re-exported Types

| Type                 | Description                           |
| :------------------- | :------------------------------------ |
| `ServerHandler`      | Trait for MCP server implementations  |
| `ToolRouter<T>`      | Router for tool invocations           |
| `Parameters<T>`      | Typed parameter wrapper               |
| `Json`               | JSON response wrapper                 |
| `CallToolResult`     | Tool result type                      |
| `Tool`               | Tool definition                       |
| `Content`            | Content type for responses            |
| `ServerCapabilities` | Server capability declaration         |
| `ServerInfo`         | Server information                    |
| `McpError`           | Error type (alias for `ErrorData`)    |
| `schemars`           | Re-exported for JsonSchema derivation |

#### `SdkMcpServer`

Marker trait automatically implemented for any `ServerHandler`.

```rust
pub trait SdkMcpServer: ServerHandler {}
```

#### Example: Creating an MCP Server

```rust
use anthropic_agent_sdk::mcp::{
    tool, tool_router, tool_handler,
    Parameters, CallToolResult, Content, ToolRouter, ServerHandler,
};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
struct GreetParams {
    name: String,
}

#[derive(Clone)]
struct Greeter {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl Greeter {
    fn new() -> Self {
        Self { tool_router: Self::tool_router() }
    }

    #[tool(description = "Greet someone by name")]
    async fn greet(&self, params: Parameters<GreetParams>) -> Result<CallToolResult, String> {
        Ok(CallToolResult::success(vec![Content::text(
            format!("Hello, {}!", params.name)
        )]))
    }
}

#[tool_handler]
impl ServerHandler for Greeter {
    fn get_info(&self) -> rmcp::model::ServerInfo {
        rmcp::model::ServerInfo::new("greeter", "1.0.0")
    }
}
```

## Type-Safe Identifiers

Newtype wrappers for type safety. All implement `Display`, `AsRef<str>`, `Deref<Target=str>`, and `Borrow<str>`.

### `SessionId`

Session identifier.

```rust
pub struct SessionId(String);
```

| Method     | Returns | Description         |
| :--------- | :------ | :------------------ |
| `new(id)`  | `Self`  | Create from string  |
| `as_str()` | `&str`  | Get as string slice |

### `ToolName`

Tool name identifier.

```rust
pub struct ToolName(String);
```

| Method      | Returns | Description         |
| :---------- | :------ | :------------------ |
| `new(name)` | `Self`  | Create from string  |
| `as_str()`  | `&str`  | Get as string slice |

### `RequestId`

Control protocol request identifier.

```rust
pub struct RequestId(String);
```

| Method     | Returns | Description         |
| :--------- | :------ | :------------------ |
| `new(id)`  | `Self`  | Create from string  |
| `as_str()` | `&str`  | Get as string slice |

## Utility Functions

### Safe String Handling (`utils` module)

Safe UTF-8 string truncation utilities that prevent panics when handling multi-byte characters.

```rust
use anthropic_agent_sdk::utils::{safe_truncate, truncate_for_display, safe_window};
```

#### `safe_truncate()`

Safely truncate a string at a UTF-8 character boundary.

```rust
pub fn safe_truncate(s: &str, max_bytes: usize) -> &str
```

| Parameter   | Type    | Description                           |
| :---------- | :------ | :------------------------------------ |
| `s`         | `&str`  | The string to truncate                |
| `max_bytes` | `usize` | Maximum number of bytes in the result |

Returns a string slice that is at most `max_bytes` long and valid UTF-8.

```rust
// Emoji 🔍 is 4 bytes - truncating at byte 10 would cut it in half
let text = "Status: 🔍 Active";
let result = safe_truncate(text, 10);
assert_eq!(result, "Status: "); // Stops before the emoji
```

#### `truncate_for_display()`

Truncate a string with ellipsis for display.

```rust
pub fn truncate_for_display(s: &str, max_bytes: usize) -> String
```

| Parameter   | Type    | Description                                    |
| :---------- | :------ | :--------------------------------------------- |
| `s`         | `&str`  | The string to truncate                         |
| `max_bytes` | `usize` | Maximum number of bytes before adding ellipsis |

Returns a String truncated with "..." appended if truncation occurred.

```rust
let text = "This is a long message";
let result = truncate_for_display(text, 10);
assert_eq!(result, "This is a ...");
```

#### `safe_window()`

Extract a safe substring window ending at a byte position.

```rust
pub fn safe_window(s: &str, end_byte: usize, window_size: usize) -> &str
```

| Parameter     | Type    | Description                                            |
| :------------ | :------ | :----------------------------------------------------- |
| `s`           | `&str`  | The source string                                      |
| `end_byte`    | `usize` | The byte position to end at (clamped to string length) |
| `window_size` | `usize` | Maximum window size in bytes                           |

Returns a string slice of at most `window_size` bytes ending at `end_byte`, respecting UTF-8 boundaries on both ends.

```rust
let code = "export class 🔍 Scanner";
let window = safe_window(code, 20, 10);
// Returns a safe slice without cutting multi-byte characters
```

### `check_claude_version()`

Validate Claude Code CLI version.

```rust
pub async fn check_claude_version(cli_path: &Path) -> Result<String>
```

Returns the version string if the CLI is at least `MIN_CLI_VERSION`, otherwise returns `ClaudeError::CliVersionTooOld`.

### `MIN_CLI_VERSION`

Minimum required CLI version.

```rust
pub const MIN_CLI_VERSION: &str = "1.0.0";
```

---

## Security

The SDK implements strict security measures to prevent injection attacks:

### Environment Variables

Dangerous environment variables are **blocked** (not silently filtered):

```rust
// These will cause an error
options.env.insert("LD_PRELOAD".to_string(), "/tmp/evil.so".to_string());
options.env.insert("NODE_OPTIONS".to_string(), "--require=/tmp/bad.js".to_string());

let client = ClaudeSDKClient::new(options, None).await;
// Error: "Dangerous environment variables detected: [LD_PRELOAD, NODE_OPTIONS]..."
```

Blocked variables: `LD_PRELOAD`, `LD_LIBRARY_PATH`, `DYLD_INSERT_LIBRARIES`, `DYLD_LIBRARY_PATH`, `PATH`, `NODE_OPTIONS`, `PYTHONPATH`, `PERL5LIB`, `RUBYLIB`

### CLI Arguments

Only allowlisted flags are permitted in `extra_args`:

```rust
// This will cause an error
options.extra_args.insert("dangerous-flag".to_string(), None);

let client = ClaudeSDKClient::new(options, None).await;
// Error: "Disallowed CLI flags in extra_args: [dangerous-flag]..."
```

Allowed flags: `timeout`, `retries`, `log-level`, `cache-dir`

### Session Binding

Sessions are automatically bound on first Result message. See [Session Binding](#session-binding-secure-by-default).

For full security documentation, see [SECURITY.md](SECURITY.md).

---

## OAuth Authentication

The SDK provides OAuth 2.0 authentication with PKCE for Claude Max/Pro subscribers. This allows users to authenticate without API keys.

### `OAuthClient`

OAuth client for Claude authentication.

```rust
use anthropic_agent_sdk::auth::{OAuthClient, TokenStorage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create OAuth client with default configuration
    let client = OAuthClient::new()?;

    // Authenticate - tries cached token first, then OAuth flow
    let token = client.authenticate().await?;

    println!("Access token: {}...", &token.access_token[..20]);
    Ok(())
}
```

#### Constructor

```rust
impl OAuthClient {
    pub fn new() -> AuthResult<Self>
    pub fn builder() -> OAuthClientBuilder
}
```

#### Methods

| Method               | Returns                 | Description                                    |
| :------------------- | :---------------------- | :--------------------------------------------- |
| `authenticate()`     | `AuthResult<TokenInfo>` | Try cached token, refresh, or start OAuth flow |
| `start_oauth_flow()` | `AuthResult<TokenInfo>` | Start full OAuth authorization flow            |
| `config()`           | `&OAuthConfig`          | Get OAuth configuration                        |
| `storage()`          | `&TokenStorage`         | Get token storage                              |
| `logout()`           | `AuthResult<()>`        | Delete cached token                            |
| `is_authenticated()` | `bool`                  | Check if valid token exists                    |
| `current_token()`    | `Option<TokenInfo>`     | Get current token without refresh              |

### `OAuthClientBuilder`

Builder for custom OAuth configuration.

```rust
let client = OAuthClient::builder()
    .auto_open_browser(false)  // Disable auto browser open
    .storage(TokenStorage::with_path("custom/path/token.json".into()))
    .build();
```

| Method                    | Description                                              |
| :------------------------ | :------------------------------------------------------- |
| `config(config)`          | Set custom OAuth configuration                           |
| `storage(storage)`        | Set custom token storage                                 |
| `auto_open_browser(bool)` | Enable/disable automatic browser opening (default: true) |
| `build()`                 | Build the OAuth client                                   |

### `OAuthConfig`

OAuth endpoint configuration.

```rust
pub struct OAuthConfig {
    pub client_id: String,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_uri: String,
    pub scopes: String,
}
```

Default configuration uses Claude Code's official OAuth endpoints.

### `TokenInfo`

OAuth token information.

```rust
pub struct TokenInfo {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub scope: Option<String>,
    pub expires_at: Option<u64>,
}
```

| Method                   | Returns            | Description                                 |
| :----------------------- | :----------------- | :------------------------------------------ |
| `is_expired()`           | `bool`             | Check if token is expired (with 60s buffer) |
| `authorization_header()` | `String`           | Get "Bearer {token}" header value           |
| `remaining_validity()`   | `Option<Duration>` | Get remaining validity duration             |

### `TokenStorage`

Persistent token storage.

```rust
use anthropic_agent_sdk::auth::TokenStorage;

// Default path: platform-specific (macOS: ~/Library/Application Support/claude-sdk/)
let storage = TokenStorage::new();

// Custom path
let storage = TokenStorage::with_path("/custom/path/token.json".into());

// Operations
let token = storage.load()?;        // Load token
let token = storage.load_valid()?;  // Load only if not expired
storage.save(&token)?;              // Save token
storage.delete()?;                  // Delete token
```

| Method              | Returns             | Description                    |
| :------------------ | :------------------ | :----------------------------- |
| `new()`             | `Self`              | Create with default path       |
| `with_path(path)`   | `Self`              | Create with custom path        |
| `path()`            | `&PathBuf`          | Get storage path               |
| `load()`            | `Result<TokenInfo>` | Load token from storage        |
| `load_valid()`      | `Result<TokenInfo>` | Load token only if not expired |
| `save(token)`       | `Result<()>`        | Save token to storage          |
| `delete()`          | `Result<()>`        | Delete stored token            |
| `has_valid_token()` | `bool`              | Check if valid token exists    |

### `OAuthError`

Errors that can occur during OAuth operations.

```rust
pub enum OAuthError {
    Http(String),
    TokenExchange(String),
    InvalidResponse(String),
    Cancelled,
    Storage(TokenError),
    Io(std::io::Error),
    Json(serde_json::Error),
    BrowserOpen(String),
    Reqwest(reqwest::Error),
}
```

### `TokenError`

Errors that can occur during token operations.

```rust
pub enum TokenError {
    Expired,
    NotFound,
    Io(std::io::Error),
    Json(serde_json::Error),
}
```

### OAuth Flow

1. Check for cached valid token
2. If expired, attempt refresh using refresh token
3. If no valid token, start browser-based OAuth flow with PKCE
4. User authorizes in browser, copies authorization code
5. Exchange code for access token
6. Cache token for future use

### Example: Full OAuth Flow

```rust
use anthropic_agent_sdk::auth::OAuthClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OAuthClient::new()?;

    // Check if already authenticated
    if client.is_authenticated() {
        let token = client.current_token().unwrap();
        if let Some(remaining) = token.remaining_validity() {
            println!("Token valid for: {:?}", remaining);
        }
    } else {
        // Start OAuth flow
        let token = client.authenticate().await?;
        println!("Authenticated! Scopes: {:?}", token.scope);
    }

    // Log out when done
    // client.logout()?;

    Ok(())
}
```

---

## See also

- [SECURITY.md](SECURITY.md) - Full security documentation and threat model
- [Claude Code CLI reference](https://docs.anthropic.com/en/docs/claude-code/cli-reference) - Command-line interface and tool input/output schemas
- [TypeScript SDK reference](https://docs.anthropic.com/en/docs/agent-sdk/typescript) - TypeScript SDK documentation
- [Python SDK reference](https://docs.anthropic.com/en/docs/agent-sdk/python) - Python SDK documentation
- [Common workflows](https://docs.anthropic.com/en/docs/claude-code/common-workflows) - Step-by-step guides
- [MCP (Model Context Protocol)](https://modelcontextprotocol.io/) - Protocol specification
