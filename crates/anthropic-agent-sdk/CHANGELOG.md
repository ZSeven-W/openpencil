# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.75] - 2025-12-22

### Added

- OAuth 2.0 authentication module with PKCE support for Claude Max/Pro subscribers
- `OAuthClient` for browser-based authentication without API keys
- `TokenStorage` for persistent token caching (platform-specific config directory)
- `TokenInfo` with expiration tracking and auto-refresh support
- `oauth_demo` example with status, logout, and help commands
- File checkpointing support via `enable_file_checkpointing` option (CLI 2.0.75+) [experimental]
- `uuid` field on `Message::User` for message tracking
- `RewindFiles` control request for checkpoint restoration [experimental]
- `rewind_files()` method on `ClaudeSDKClient` [experimental]
- `checkpointing_demo` and `checkpointing_rewind_test` examples [experimental]
- Hook input types: `PreCompactHookInput`, `CompactTrigger`, `PermissionRequestHookInput`
- `resume_session_at` option for resuming at specific message UUID

### Changed

- Token exchange uses JSON body format (not form-urlencoded) per Anthropic API
- Hook input types now use `Option<String>` for `tool_use_id` field (CLI 2.0.75+)

### Dependencies

- Added `sha2` 0.10.9 for PKCE challenge generation
- Added `base64` 0.22.1 for base64url encoding
- Added `reqwest` 0.12.28 for HTTP requests
- Added `dirs` 6.0.0 for config directory resolution

## [0.2.67] - 2025-12-12

### Added

- Interactive TUI demo crate at `demos/tui-demo` with Goose-style UI
- Spinner with cycling random thinking messages (updates every 2 seconds)
- Styled prompts: green `>>` for user input, cyan `claude>` for responses
- Tool use headers showing operation and file path
- Markdown rendering with syntax highlighting via bat
- REPL input with rustyline (history, multi-line, Ctrl+C/D handling)
- Slash commands: `/help`, `/quit`, `/clear`, `/test-spinner`
- Context usage visualization with colored dot progress indicators
- Goose TUI analysis documentation at `docs/goose-tui-analysis.md`

### Changed

- Converted project to workspace with SDK at root and demos as members
- Workspace resolver set to "2" for consistent dependency resolution

### Fixed

- Clippy error in `src/transport/mod.rs` (items after test module)
- Clippy errors in `examples/mcp_integration.rs` (collapsible match, single match)

## [0.2.60] - 2025-12-07

### Added

- Rust SDK for Claude Code CLI with full TypeScript SDK parity
- Async streaming queries via `query()` function
- Bidirectional client with `ClaudeSDKClient` for multi-turn conversations
- Message buffering via `queue_message()` and `next_buffered()` for CLI timing
- Session binding with auto-bind on first Result message
- Session resume support for model switching mid-conversation
- Interrupt support via `interrupt()` for stopping streaming responses
- Hook system with 11 event types (PreToolUse, PostToolUse, SessionStart, etc.)
- Permission system with `PermissionCallback` trait and `PermissionManager`
- MCP server integration (stdio, SSE, HTTP, SDK in-process via rmcp feature)
- Introspection methods: `session_info()`, `supported_models()`, `supported_commands()`, `mcp_server_status()`
- Result message fields: `model_usage`, `permission_denials`, `structured_output`, `errors`
- Runtime setters for model, max_thinking_tokens, permission_mode
- Security hardening: env var blocking, CLI arg allowlist, session validation
- UTF-8 safe string utilities: `safe_truncate()`, `truncate_for_display()`, `safe_window()`
- 26 working examples covering all SDK features
- 3 integration test suites (client, control protocol, security)
- Comprehensive API documentation (README_API.md)
- Security documentation with threat model (SECURITY.md)

### Features

- Full TypeScript SDK parity across all options, methods, and hook events
- Trait-based callbacks for hooks and permissions
- Type-safe identifiers (SessionId, ToolName, RequestId)
- Builder pattern via typed-builder for ergonomic configuration
- 100% safe Rust with zero unsafe blocks
- Async-first design with tokio runtime
- Streaming message parsing with futures::Stream
- Configurable timeouts and buffer limits
- CLI version validation (requires Claude Code 2.0.60+)
- Optional tracing support for structured logging
- Optional rmcp feature for in-process MCP SDK servers

### Security

- Environment variable filtering blocks LD_PRELOAD, PATH, NODE_OPTIONS, etc.
- CLI argument allowlist prevents injection attacks
- Session binding validates session IDs on send
- 30-second default I/O timeouts
- 1MB default buffer limits
- Strict error handling on security violations
