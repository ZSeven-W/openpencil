# Claude Agent SDK for Rust

Rust SDK for building AI agents powered by Claude Code. Mirrors the [TypeScript Claude Agent SDK](https://platform.claude.com/docs/en/agent-sdk/typescript) with idiomatic Rust patterns.

## Installation

**Prerequisites:**

- Rust 1.85.0+ (edition 2024)
- Node.js
- Claude Code 2.0.75+: `npm install -g @anthropic-ai/claude-code`

```toml
[dependencies]
anthropic-agent-sdk = "0.2"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

## Quick Start

```rust
use anthropic_agent_sdk::{query, Message, ContentBlock, StreamExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream = query("What is 2 + 2?", None).await?;
    let mut stream = Box::pin(stream);

    while let Some(message) = stream.next().await {
        if let Message::Assistant { message, .. } = message? {
            for block in &message.content {
                if let ContentBlock::Text { text } = block {
                    println!("{}", text);
                }
            }
        }
    }
    Ok(())
}
```

## Examples

```bash
# Core functionality
cargo run --example simple_query
cargo run --example convenience_methods
cargo run --example bidirectional_demo
cargo run --example message_queue_demo
cargo run --example session_binding_demo
cargo run --example interactive_client

# Hooks and permissions
cargo run --example hooks_demo
cargo run --example permissions_demo
cargo run --example hooks_lifecycle_test

# Introspection and runtime
cargo run --example introspection_demo
cargo run --example result_fields_demo
cargo run --example runtime_setters_demo

# Security
cargo run --example security_demo

# OAuth authentication
cargo run --example oauth_demo
cargo run --example oauth_demo -- status
cargo run --example oauth_demo -- logout

# Structured output
cargo run --example structured_output_demo

# MCP (requires --features rmcp for mcp_server)
cargo run --example mcp_integration
cargo run --example mcp_server --features rmcp
```

## TypeScript SDK Parity

### Hook Events

| Event              | TypeScript | Rust SDK |
| ------------------ | ---------- | -------- |
| PreToolUse         | ✓          | ✓        |
| PostToolUse        | ✓          | ✓        |
| PostToolUseFailure | ✓          | ✓        |
| Notification       | ✓          | ✓        |
| UserPromptSubmit   | ✓          | ✓        |
| SessionStart       | ✓          | ✓        |
| SessionEnd         | ✓          | ✓        |
| Stop               | ✓          | ✓        |
| SubagentStart      | ✓          | ✓        |
| SubagentStop       | ✓          | ✓        |
| PreCompact         | ✓          | ✓        |
| PermissionRequest  | ✓          | ✓        |

### Query/Client Methods

| Method                 | TypeScript | Rust SDK |
| ---------------------- | ---------- | -------- |
| interrupt()            | ✓          | ✓        |
| setPermissionMode()    | ✓          | ✓        |
| setModel()             | ✓          | ✓        |
| setMaxThinkingTokens() | ✓          | ✓        |
| supportedCommands()    | ✓          | ✓        |
| supportedModels()      | ✓          | ✓        |
| mcpServerStatus()      | ✓          | ✓        |
| accountInfo()          | ✓          | ✓        |

### Options

| Option                          | TypeScript | Rust SDK |
| ------------------------------- | ---------- | -------- |
| allowedTools                    | ✓          | ✓        |
| disallowedTools                 | ✓          | ✓        |
| systemPrompt                    | ✓          | ✓        |
| mcpServers                      | ✓          | ✓        |
| permissionMode                  | ✓          | ✓        |
| canUseTool                      | ✓          | ✓        |
| hooks                           | ✓          | ✓        |
| agents                          | ✓          | ✓        |
| maxTurns                        | ✓          | ✓        |
| model                           | ✓          | ✓        |
| cwd                             | ✓          | ✓        |
| env                             | ✓          | ✓        |
| resume                          | ✓          | ✓        |
| forkSession                     | ✓          | ✓        |
| settingSources                  | ✓          | ✓        |
| maxBudgetUsd                    | ✓          | ✓        |
| maxThinkingTokens               | ✓          | ✓        |
| fallbackModel                   | ✓          | ✓        |
| outputFormat                    | ✓          | ✓        |
| sandbox                         | ✓          | ✓        |
| plugins                         | ✓          | ✓        |
| betas                           | ✓          | ✓        |
| strictMcpConfig                 | ✓          | ✓        |
| resumeSessionAt                 | ✓          | ✓        |
| allowDangerouslySkipPermissions | ✓          | ✓        |
| pathToClaudeCodeExecutable      | ✓          | ✓        |
| stderr                          | ✓          | ✓        |
| tools (preset)                  | ✓          | ✓        |
| enableFileCheckpointing         | ✓          | ✓        |
| sessionId                       | ✓          | ✓        |

### MCP Server Types

| Type             | TypeScript | Rust SDK |
| ---------------- | ---------- | -------- |
| stdio            | ✓          | ✓        |
| sse              | ✓          | ✓        |
| http             | ✓          | ✓        |
| sdk (in-process) | ✓          | ✓        |

### Result Message Fields

| Field              | TypeScript | Rust SDK |
| ------------------ | ---------- | -------- |
| modelUsage         | ✓          | ✓        |
| permission_denials | ✓          | ✓        |
| structured_output  | ✓          | ✓        |
| errors             | ✓          | ✓        |

## Development

```bash
cargo build
cargo test
cargo clippy
cargo doc --open
```

## Security

The SDK implements strict security measures:

- **Environment variables**: Dangerous vars (LD_PRELOAD, PATH, etc.) cause errors
- **CLI arguments**: Only allowlisted flags permitted, others rejected
- **Session binding**: Auto-binds on first Result, validates on send
- **Buffer limits**: Configurable max buffer size (default 1MB)
- **100% safe Rust**: No unsafe code

See [SECURITY.md](SECURITY.md) for full documentation.

## OAuth Authentication

For Claude Max/Pro subscribers, authenticate without API keys:

```rust
use anthropic_agent_sdk::auth::OAuthClient;

let client = OAuthClient::new()?;
let token = client.authenticate().await?;
// Token is cached in platform-specific config directory
```

See `oauth_demo` example for full usage including status check and logout.

## Documentation

- [README_API.md](README_API.md) - Full API reference
- [SECURITY.md](SECURITY.md) - Security documentation and threat model

## License

MIT

## Related

- [TypeScript Claude Agent SDK](https://platform.claude.com/docs/en/agent-sdk/typescript)
- [Claude Code](https://code.claude.com/docs)
