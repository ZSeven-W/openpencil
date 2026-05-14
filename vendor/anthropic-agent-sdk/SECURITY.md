# Security Documentation

This document describes the security measures implemented in the Claude Agent SDK for Rust.

## Overview

The SDK implements multiple layers of defense to ensure secure operation when interfacing with the Claude Code CLI.

## Security Measures

### 1. Environment Variable Blocking (Strict Enforcement)

Dangerous environment variables that could be exploited for code injection cause an immediate error - they are **not** silently filtered:

```rust
const DANGEROUS_ENV_VARS: &[&str] = &[
    "LD_PRELOAD",           // Linux dynamic linker injection
    "LD_LIBRARY_PATH",      // Linux library path manipulation
    "DYLD_INSERT_LIBRARIES", // macOS dylib injection
    "DYLD_LIBRARY_PATH",    // macOS library path manipulation
    "PATH",                 // Command search path manipulation
    "NODE_OPTIONS",         // Node.js runtime injection
    "PYTHONPATH",           // Python module injection
    "PERL5LIB",             // Perl module injection
    "RUBYLIB",              // Ruby library injection
];
```

**Behavior:**

- Safe env vars: Passed to subprocess normally
- Dangerous env vars: Returns `ClaudeError::InvalidConfig` with details
- Logs warning (with `tracing-support` feature)

```rust
// This will fail with an error
options.env.insert("LD_PRELOAD".to_string(), "/tmp/evil.so".to_string());
let client = ClaudeSDKClient::new(options, None).await;
// Error: "Dangerous environment variables detected: [LD_PRELOAD]. These are blocked to prevent injection attacks."
```

**Location**: `src/transport/subprocess.rs:23-33`, enforced at `:549-572`

### 2. CLI Argument Allowlist (Strict Enforcement)

Only explicitly allowed CLI flags can be passed via `extra_args`. Disallowed flags cause an immediate error - they are **not** silently ignored:

```rust
const ALLOWED_EXTRA_FLAGS: &[&str] = &["timeout", "retries", "log-level", "cache-dir"];
```

**Behavior:**

- Allowed flags: Passed to CLI normally
- Disallowed flags: Returns `ClaudeError::InvalidConfig` with details
- Logs warning (with `tracing-support` feature)

```rust
// This will fail with an error
options.extra_args.insert("dangerous-flag".to_string(), None);
let client = ClaudeSDKClient::new(options, None).await;
// Error: "Disallowed CLI flags in extra_args: [dangerous-flag]. Allowed flags: [...]"
```

**Location**: `src/transport/subprocess.rs:36`, enforced at `:426-461`

### Validation Order and Precedence

Both checks run during `connect()`. CLI args are validated first, then env vars.

**CLI args** use an allowlist (whitelist):

- Only flags explicitly listed in `ALLOWED_EXTRA_FLAGS` are permitted
- Everything else is rejected

**Env vars** use a blocklist (blacklist):

- Only vars listed in `DANGEROUS_ENV_VARS` are rejected
- Everything else is permitted

There is no way to override these at runtime. The lists are compile-time constants. Attempting to pass a blocked item always fails - there is no "force" option by design.

### 3. Buffer Size Limits

Memory exhaustion attacks are prevented by limiting JSON message sizes:

- **Default limit**: 1MB (`DEFAULT_MAX_BUFFER_SIZE = 1024 * 1024`)
- **Configurable**: Via `ClaudeAgentOptions::max_buffer_size`
- Messages exceeding the limit generate an error and are discarded

**Location**: `src/transport/subprocess.rs:20`, enforced at `subprocess.rs:688-699`

### 4. Timeout Strategy (CancellationToken-based)

The SDK uses cooperative cancellation instead of hardcoded timeouts for I/O operations.

#### Design Rationale

Hardcoded timeouts (e.g., 30-second read timeouts) were considered but rejected because:

1. **Long-running operations are legitimate**: Claude Code queries involving subagent research, complex code analysis, or multi-step agentic workflows can take minutes
2. **False positive risk**: A timeout would interrupt valid operations, corrupting state
3. **User control is preferable**: Applications know their expected operation durations better than the SDK

#### Implementation

The SDK implements `CancellationToken` (analogous to JavaScript's `AbortController`):

```rust
// Client provides cancellation control
let token = client.cancellation_token();

// Application can cancel at any time
client.cancel();

// Background tasks respect cancellation
tokio::select! {
    _ = cancel_token.cancelled() => { /* cleanup */ }
    result = read_operation => { /* process */ }
}
```

**Locations**:

- `ClaudeSDKClient::cancellation_token()` - Get a child token
- `ClaudeSDKClient::cancel()` - Cancel all operations
- `SubprocessTransport::read_messages()` - Respects cancellation in read loop

#### Graceful Shutdown

The `close()` method implements a graceful shutdown with a 5-second timeout before forcing process termination:

```rust
// Try graceful exit first
let timeout_duration = Duration::from_secs(5);
match tokio::time::timeout(timeout_duration, child.wait()).await {
    Ok(_) => { /* graceful exit */ }
    Err(_) => {
        child.kill().await; // Force kill after timeout
    }
}
```

**Location**: `src/transport/subprocess.rs:792-807`

### 5. No Unsafe Code

The SDK is 100% safe Rust with no `unsafe` blocks. This ensures memory safety guarantees are enforced by the compiler.

### 6. Input Bounds Checking

Configurable values are bounded to prevent abuse:

- `max_turns`: Maximum 1000 turns (`ClaudeAgentOptions::MAX_ALLOWED_TURNS`)

### 7. Secure Debug Output

Sensitive data (callbacks, hooks) are redacted in Debug implementations:

```rust
impl Debug for ClaudeAgentOptions {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClaudeAgentOptions")
            .field("can_use_tool", &self.can_use_tool.as_ref().map(|_| "<callback>"))
            // ...
    }
}
```

**Location**: `src/types/options.rs:442-514`

### 8. Stderr Isolation

The subprocess stderr is piped (not inherited) to prevent the child process from manipulating the parent terminal state:

```rust
cmd.stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped()); // Prevents terminal manipulation
```

**Location**: `src/transport/subprocess.rs:538-540`

### 9. Session Binding (Secure by Default)

The SDK automatically binds to a session on the first `Result` message, preventing messages from being sent to unintended conversation contexts.

#### Design Rationale

Without session binding, a multi-turn application could accidentally send messages to a different session if:

- Session IDs get mixed up in application state
- Race conditions in concurrent applications
- Unexpected session restarts

#### Implementation

Session binding is **automatic and secure by default**:

```rust
// Auto-binds on first Result - no manual binding required
client.send_message("Hello").await?;
while let Some(msg) = client.next_message().await {
    if let Message::Result { .. } = msg? {
        // Session is now auto-bound!
        break;
    }
}

// All subsequent sends validate automatically
client.send_message("Follow-up").await?;  // Validates session first
```

If the current session doesn't match the bound session, `send_message()` returns `ClaudeError::SessionMismatch`:

```rust
#[error("Session mismatch: expected {expected}, got {actual}")]
SessionMismatch {
    expected: String,
    actual: String,
}
```

#### Override Methods

For advanced use cases (e.g., multi-session applications):

| Method               | Description                           |
| -------------------- | ------------------------------------- |
| `bind_session(id)`   | Override auto-bound session           |
| `unbind_session()`   | Disable validation (use with caution) |
| `validate_session()` | Manual validation check               |
| `bound_session()`    | Get currently bound session           |

#### Subagent Behavior

Subagents (via Task tool) share the parent session ID, so auto-bind works correctly:

- Parent binds to session on first Result
- Subagent hooks report same session ID
- No SessionMismatch conflicts

**Locations**:

- Auto-bind: `src/client.rs:449-454` (in `message_reader_task`)
- Validation: `src/client.rs` (`validate_session()`, `send_message()`)
- Error type: `src/error.rs:100-107`

### 10. OAuth Token Security

The OAuth module stores authentication tokens with appropriate security measures:

#### Token Storage

- **Location**: Platform-specific config directory
  - macOS: `~/Library/Application Support/claude-sdk/oauth_token.json`
  - Linux: `~/.config/claude-sdk/oauth_token.json`
  - Windows: `%APPDATA%\claude-sdk\oauth_token.json`
- **Permissions**: `0600` on Unix (owner read/write only)
- **Contents**: Access token, refresh token, expiration timestamp

```rust
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(0o600);
    std::fs::set_permissions(&storage_path, perms)?;
}
```

#### Token Handling

- **Expiration buffer**: Tokens considered expired 60 seconds before actual expiry
- **Auto-refresh**: Expired tokens trigger automatic refresh via refresh_token
- **PKCE**: Authorization uses PKCE (SHA-256 challenge) to prevent code interception
- **No secrets in logs**: Token values are truncated in display output

#### Logout

The `logout()` method deletes the cached token file entirely.

**Location**: `src/auth/token.rs:172-213`, `src/auth/oauth.rs`

## Threat Model

| Threat                      | Mitigation                                         |
| --------------------------- | -------------------------------------------------- |
| CLI command injection       | Argument allowlist, environment filtering          |
| Memory exhaustion           | Buffer size limits                                 |
| Infinite hangs              | CancellationToken, graceful shutdown timeout       |
| Terminal corruption         | Stderr isolation                                   |
| Memory unsafety             | 100% safe Rust                                     |
| Session confusion/hijacking | Auto-bind on first Result, validation on send      |
| OAuth token theft           | Restrictive file permissions (0600), PKCE for auth |
| OAuth code interception     | PKCE with SHA-256 challenge/verifier               |

## Reporting Security Issues

Please report security vulnerabilities to the maintainers via GitHub security advisories.
