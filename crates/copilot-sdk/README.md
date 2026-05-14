# copilot-sdk (Rust)

Rust SDK for interacting with the GitHub Copilot CLI agent runtime (JSON-RPC over stdio or TCP).

This is a Rust port of the upstream SDKs and is currently in technical preview.

## Requirements

- Rust 1.85+ (Edition 2024)
- GitHub Copilot CLI installed and authenticated
- `copilot` available in `PATH`, or set `COPILOT_CLI_PATH` to the CLI executable/script

## Install

Once published, add:

```toml
[dependencies]
copilot-sdk = "0.1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

For development from this repository:

```toml
[dependencies]
copilot-sdk = { path = "." }
```

## Quick Start

```rust
use copilot_sdk::{Client, SessionConfig};

#[tokio::main]
async fn main() -> copilot_sdk::Result<()> {
    let client = Client::builder().build()?;
    client.start().await?;

    let session = client.create_session(SessionConfig::default()).await?;
    let response = session.send_and_collect("Hello!", None).await?;
    println!("{}", response);

    client.stop().await;
    Ok(())
}
```

## Features

### Session Management

Full session lifecycle with create, resume, list, delete, and foreground control:

```rust
let session = client.create_session(SessionConfig {
    model: Some("gpt-4.1".into()),
    streaming: true,
    client_name: Some("my-app".into()),
    ..Default::default()
}).await?;
```

### Model Management

Switch models and reasoning effort mid-session:

```rust
let model = session.get_model().await?;
session.set_model("claude-sonnet-4", Some(SetModelOptions {
    reasoning_effort: Some("high".into()),
})).await?;
```

### Mode Switching

Switch between interactive, plan, and autopilot modes:

```rust
session.set_mode(SessionMode::Plan).await?;
session.set_mode(SessionMode::Autopilot).await?;
session.set_mode(SessionMode::Interactive).await?;
```

### Plan Management

Read, update, and delete session plans:

```rust
session.update_plan(&PlanData {
    content: Some("Step 1: Implement\nStep 2: Test".into()),
    title: Some("Implementation Plan".into()),
}).await?;
let plan = session.read_plan().await?;
session.delete_plan().await?;
```

### Agent Management

List, select, and deselect custom agents:

```rust
let agents = session.list_agents().await?;
session.select_agent("code-reviewer").await?;
session.deselect_agent().await?;
```

### Custom Tools

Register tools that the assistant can invoke, with permission control:

```rust
let tool = Tool::new("get_weather")
    .description("Get current weather")
    .parameter("city", "string", "City name", true)
    .skip_permission(true);

session.register_tool_with_handler(tool, Some(handler)).await;
```

### Infinite Sessions

Automatic context window management with manual compaction support:

```rust
let config = SessionConfig {
    infinite_sessions: Some(InfiniteSessionConfig::enabled()),
    ..Default::default()
};
// Trigger manual compaction
session.compact().await?;
```

### Session Logging

Add log entries to sessions:

```rust
session.log("Processing step complete", Some(LogOptions {
    level: Some(SessionLogLevel::Info),
    ephemeral: Some(false),
})).await?;
```

### Shell Operations

Execute shell commands and manage processes:

```rust
let result = session.shell_exec(ShellExecOptions {
    command: "cargo test".into(),
    cwd: Some("/my/project".into()),
    env: None,
}).await?;
session.shell_kill(&result.process_id, ShellSignal::SIGTERM).await?;
```

### Workspace File Operations

List, read, and create files in the session workspace:

```rust
let files = session.workspace_list_files().await?;
let content = session.workspace_read_file("plan.md").await?;
session.workspace_create_file("notes.md", "# Notes").await?;
```

### Fleet Management

Start parallel agent fleets:

```rust
session.start_fleet(Some(FleetStartOptions {
    prompt: Some("Build and test the project".into()),
})).await?;
```

### Client Utilities

```rust
let status = client.get_status().await?;       // CLI version info
let auth = client.get_auth_status().await?;    // Authentication state
let models = client.list_models().await?;      // Available models
let tools = client.tools_list(None).await?;    // Available tools
let quota = client.get_quota().await?;         // Account quota
```

### OpenTelemetry Integration

Configure distributed tracing for the CLI process:

```rust
let client = Client::builder()
    .telemetry(TelemetryConfig {
        otlp_endpoint: Some("http://localhost:4318".into()),
        exporter_type: Some("otlp-http".into()),
        source_name: Some("my-app".into()),
        capture_content: Some(true),
        file_path: None,
    })
    .build()?;
```

### BYOK (Bring Your Own Key)

Use your own API keys with compatible providers, with custom model listing:

```rust
let client = Client::builder()
    .on_list_models(|| async {
        Ok(vec![ModelInfo { /* ... */ }])
    })
    .build()?;

let config = SessionConfig {
    provider: Some(ProviderConfig {
        base_url: "https://api.openai.com/v1".into(),
        api_key: Some("sk-...".into()),
        ..Default::default()
    }),
    auto_byok_from_env: true,
    ..Default::default()
};
```

### Hooks

Intercept session lifecycle at key points:

```rust
let config = SessionConfig {
    hooks: Some(SessionHooks {
        on_pre_tool_use: Some(Arc::new(|input| {
            println!("Tool: {}", input.tool_name);
            PreToolUseHookOutput::default()
        })),
        ..Default::default()
    }),
    ..Default::default()
};
```

## Examples

```bash
cargo run --example basic_chat          # Simple Q&A
cargo run --example streaming           # Streaming responses
cargo run --example tool_usage          # Custom tools
cargo run --example set_model           # Model switching
cargo run --example mode_switching      # Mode management
cargo run --example plan_ops            # Plan CRUD
cargo run --example agent_management    # Agent operations
cargo run --example telemetry           # OpenTelemetry setup
cargo run --example shell_exec          # Shell commands
cargo run --example hooks               # Session hooks
cargo run --example byok                # Bring Your Own Key
```

## Development

### Setup

Enable pre-commit hooks to catch formatting/linting issues before push:

```bash
git config core.hooksPath .githooks
```

### Commands

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

E2E tests (real Copilot CLI):

```bash
cargo test --features e2e -- --test-threads=1
```

Snapshot conformance tests (optional, against upstream YAML snapshots):

```bash
cargo test --features snapshots --test snapshot_conformance
```

Set `COPILOT_SDK_RUST_SNAPSHOT_DIR` or `UPSTREAM_SNAPSHOTS` to point at `copilot-sdk/test/snapshots` if it cannot be auto-detected.

## Protocol Compatibility

- **SDK Protocol Version**: 3 (minimum: 2)
- **Transport**: stdio (spawned CLI) and TCP (spawned or external server)
- **JSON-RPC**: v2.0 with Content-Length framing

## Feature Parity

This port targets feature parity with the official SDKs (Go, TypeScript, Python, .NET):

| Feature                                      | Status       |
| -------------------------------------------- | ------------ |
| Session CRUD (create/resume/list/delete)     | ✅           |
| Model management (get/switch)                | ✅           |
| Mode management (interactive/plan/autopilot) | ✅           |
| Plan management (read/update/delete)         | ✅           |
| Agent management (list/select/deselect)      | ✅           |
| Tool system (register/invoke/permissions)    | ✅           |
| Hook system (6 lifecycle hooks)              | ✅           |
| Permission handling                          | ✅           |
| User input handling                          | ✅           |
| Infinite sessions & compaction               | ✅           |
| Shell operations (exec/kill)                 | ✅           |
| Workspace file operations                    | ✅           |
| Fleet management                             | ✅           |
| Session logging                              | ✅           |
| BYOK (custom providers)                      | ✅           |
| OpenTelemetry configuration                  | ✅           |
| Custom model list callback                   | ✅           |
| MCP server integration                       | ✅           |
| Custom agent configuration                   | ✅           |
| Streaming events (40+ types)                 | ✅           |
| Protocol v2/v3 negotiation                   | ✅           |
| CLI bundling                                 | ❌ (planned) |

## License

MIT License - see [LICENSE](LICENSE).

## Related

- Upstream SDKs: https://github.com/github/copilot-sdk
