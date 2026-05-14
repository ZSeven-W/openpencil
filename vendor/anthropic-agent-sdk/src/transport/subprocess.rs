//! Subprocess transport implementation using Claude Code CLI

use async_trait::async_trait;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{Mutex, mpsc};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::error::{ClaudeError, Result};
use crate::types::{ClaudeAgentOptions, SystemPrompt};
use crate::utils::truncate_for_display;
use crate::{Transport, VERSION};

const DEFAULT_MAX_BUFFER_SIZE: usize = 1024 * 1024; // 1MB

// Dangerous environment variables that should not be passed to subprocess
const DANGEROUS_ENV_VARS: &[&str] = &[
    "LD_PRELOAD",
    "LD_LIBRARY_PATH",
    "DYLD_INSERT_LIBRARIES",
    "DYLD_LIBRARY_PATH",
    "PATH",
    "NODE_OPTIONS",
    "PYTHONPATH",
    "PERL5LIB",
    "RUBYLIB",
];

// Allowed extra CLI flags (allowlist approach)
const ALLOWED_EXTRA_FLAGS: &[&str] = &["timeout", "retries", "log-level", "cache-dir"];

/// Prompt input type
#[derive(Debug)]
pub enum PromptInput {
    /// Single string prompt
    String(String),
    /// Stream of JSON messages
    Stream,
}

impl From<String> for PromptInput {
    fn from(s: String) -> Self {
        PromptInput::String(s)
    }
}

impl From<&str> for PromptInput {
    fn from(s: &str) -> Self {
        PromptInput::String(s.to_string())
    }
}

/// Subprocess transport for Claude Code CLI
pub struct SubprocessTransport {
    prompt: PromptInput,
    options: ClaudeAgentOptions,
    cli_path: PathBuf,
    cwd: Option<PathBuf>,
    process: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<BufReader<ChildStdout>>,
    ready: Arc<AtomicBool>,
    max_buffer_size: usize,
    reader_task: Option<JoinHandle<()>>,
    stderr_task: Option<JoinHandle<()>>,
    /// Cancellation token for aborting operations (like `AbortController` in JS)
    cancellation_token: CancellationToken,
}

impl SubprocessTransport {
    /// Create a new subprocess transport
    ///
    /// # Arguments
    /// * `prompt` - The prompt input (string or stream)
    /// * `options` - Configuration options
    /// * `cli_path` - Optional path to Claude Code CLI (will search if None)
    ///
    /// # Errors
    /// Returns error if CLI cannot be found
    pub fn new(
        prompt: PromptInput,
        options: ClaudeAgentOptions,
        cli_path: Option<PathBuf>,
    ) -> Result<Self> {
        Self::with_cancellation_token(prompt, options, cli_path, None)
    }

    /// Create a new subprocess transport with an optional parent cancellation token
    ///
    /// # Arguments
    /// * `prompt` - The prompt input (string or stream)
    /// * `options` - Configuration options
    /// * `cli_path` - Optional path to Claude Code CLI (will search if None)
    /// * `cancellation_token` - Optional parent cancellation token from client
    ///
    /// # Errors
    /// Returns error if CLI cannot be found
    pub fn with_cancellation_token(
        prompt: PromptInput,
        options: ClaudeAgentOptions,
        cli_path: Option<PathBuf>,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<Self> {
        tracing::debug!(
            "SubprocessTransport::new - output_format: {:?}",
            options.output_format.as_ref().map(|f| &f.format_type)
        );

        let cli_path = if let Some(path) = cli_path {
            path
        } else {
            Self::find_cli()?
        };

        let cwd = options.cwd.clone();
        let max_buffer_size = options.max_buffer_size.unwrap_or(DEFAULT_MAX_BUFFER_SIZE);

        // Use provided token or create a new one
        let token = cancellation_token.unwrap_or_default();

        Ok(Self {
            prompt,
            options,
            cli_path,
            cwd,
            process: None,
            stdin: None,
            stdout: None,
            ready: Arc::new(AtomicBool::new(false)),
            max_buffer_size,
            reader_task: None,
            stderr_task: None,
            cancellation_token: token,
        })
    }

    /// Find Claude Code CLI binary
    fn find_cli() -> Result<PathBuf> {
        // Try using 'which' crate first
        if let Ok(path) = which::which("claude") {
            return Ok(path);
        }

        // Manual search in common locations
        let home = env::var("HOME").unwrap_or_else(|_| String::from("/root"));
        let locations = vec![
            PathBuf::from(home.clone()).join(".npm-global/bin/claude"),
            PathBuf::from("/usr/local/bin/claude"),
            PathBuf::from(home.clone()).join(".local/bin/claude"),
            PathBuf::from(home.clone()).join("node_modules/.bin/claude"),
            PathBuf::from(home).join(".yarn/bin/claude"),
        ];

        for path in locations {
            if path.exists() && path.is_file() {
                return Ok(path);
            }
        }

        Err(ClaudeError::cli_not_found())
    }

    /// Build CLI command with all arguments
    #[allow(clippy::too_many_lines)]
    fn build_command(&self) -> Result<Command> {
        let mut cmd = Command::new(&self.cli_path);

        // Always use --print for non-interactive mode to avoid terminal manipulation
        cmd.arg("--print");

        cmd.arg("--output-format")
            .arg("stream-json")
            .arg("--verbose");

        // System prompt
        if let Some(ref system_prompt) = self.options.system_prompt {
            match system_prompt {
                SystemPrompt::String(s) => {
                    cmd.arg("--system-prompt").arg(s);
                }
                SystemPrompt::Preset(preset) => {
                    if let Some(ref append) = preset.append {
                        cmd.arg("--append-system-prompt").arg(append);
                    }
                }
                SystemPrompt::File(path) => {
                    cmd.arg("--system-prompt-file").arg(path);
                }
            }
        }

        // Append system prompt (additional text to append to any system prompt)
        if let Some(ref append) = self.options.append_system_prompt {
            cmd.arg("--append-system-prompt").arg(append);
        }

        // Allowed tools
        if !self.options.allowed_tools.is_empty() {
            let tools: Vec<String> = self
                .options
                .allowed_tools
                .iter()
                .map(|t| t.as_str().to_string())
                .collect();
            cmd.arg("--allowedTools").arg(tools.join(","));
        }

        // Max turns
        if let Some(max_turns) = self.options.max_turns {
            cmd.arg("--max-turns").arg(max_turns.to_string());
        }

        // Disallowed tools
        if !self.options.disallowed_tools.is_empty() {
            let tools: Vec<String> = self
                .options
                .disallowed_tools
                .iter()
                .map(|t| t.as_str().to_string())
                .collect();
            cmd.arg("--disallowedTools").arg(tools.join(","));
        }

        // Model
        if let Some(ref model) = self.options.model {
            cmd.arg("--model").arg(model);
        }

        // Permission prompt tool
        if let Some(ref tool) = self.options.permission_prompt_tool_name {
            cmd.arg("--permission-prompt-tool").arg(tool);
        }

        // Permission mode
        if let Some(ref mode) = self.options.permission_mode {
            let mode_str = match mode {
                crate::types::PermissionMode::Default => "default",
                crate::types::PermissionMode::AcceptEdits => "acceptEdits",
                crate::types::PermissionMode::Plan => "plan",
                crate::types::PermissionMode::BypassPermissions => "bypassPermissions",
            };
            cmd.arg("--permission-mode").arg(mode_str);

            // BypassPermissions requires the dangerous skip flag
            if matches!(mode, crate::types::PermissionMode::BypassPermissions) {
                cmd.arg("--dangerously-skip-permissions");
            }
        }

        // Continue conversation
        if self.options.continue_conversation {
            cmd.arg("--continue");
        }

        // Resume session
        if let Some(ref session_id) = self.options.resume {
            cmd.arg("--resume").arg(session_id.as_str());
        }

        // Settings file
        if let Some(ref settings) = self.options.settings {
            cmd.arg("--settings").arg(settings);
        }

        // Add directories
        for dir in &self.options.add_dirs {
            cmd.arg("--add-dir").arg(dir);
        }

        // MCP servers
        match &self.options.mcp_servers {
            crate::types::McpServers::Dict(servers) => {
                if !servers.is_empty() {
                    let mut config_map = HashMap::new();
                    for (name, config) in servers {
                        config_map.insert(name.clone(), Self::serialize_mcp_config(config));
                    }
                    let config_json = serde_json::json!({
                        "mcpServers": config_map
                    });
                    cmd.arg("--mcp-config").arg(config_json.to_string());
                }
            }
            crate::types::McpServers::Path(path) => {
                cmd.arg("--mcp-config").arg(path);
            }
            crate::types::McpServers::None => {}
        }

        // Include partial messages
        if self.options.include_partial_messages {
            cmd.arg("--include-partial-messages");
        }

        // Fork session
        if self.options.fork_session {
            cmd.arg("--fork-session");
        }

        // Custom session ID
        if let Some(ref session_id) = self.options.session_id {
            cmd.arg("--session-id").arg(session_id);
        }

        // Agents
        if let Some(ref agents) = self.options.agents {
            let agents_json = serde_json::to_string(agents).unwrap_or_default();
            cmd.arg("--agents").arg(agents_json);
        }

        // Setting sources
        if let Some(ref sources) = self.options.setting_sources {
            let sources_str: Vec<&str> = sources
                .iter()
                .map(|s| match s {
                    crate::types::SettingSource::User => "user",
                    crate::types::SettingSource::Project => "project",
                    crate::types::SettingSource::Local => "local",
                })
                .collect();
            cmd.arg("--setting-sources").arg(sources_str.join(","));
        } else {
            cmd.arg("--setting-sources").arg("");
        }

        // User identifier
        if let Some(ref user) = self.options.user {
            cmd.arg("--user").arg(user);
        }

        // ====================================================================
        // New options for TypeScript SDK parity
        // ====================================================================

        // Max budget in USD
        if let Some(max_budget) = self.options.max_budget_usd {
            cmd.arg("--max-budget-usd").arg(max_budget.to_string());
        }

        // Max thinking tokens
        if let Some(max_thinking) = self.options.max_thinking_tokens {
            cmd.arg("--max-thinking-tokens")
                .arg(max_thinking.to_string());
        }

        // Fallback model
        if let Some(ref fallback) = self.options.fallback_model {
            cmd.arg("--fallback-model").arg(fallback);
        }

        // JSON schema for structured outputs
        if let Some(ref output_format) = self.options.output_format {
            let schema_json = serde_json::to_string(&output_format.schema).unwrap_or_default();
            tracing::debug!("Adding --json-schema flag with schema: {}", schema_json);
            cmd.arg("--json-schema").arg(schema_json);
        }

        // Sandbox settings
        if let Some(ref sandbox) = self.options.sandbox {
            if sandbox.enabled == Some(true) {
                cmd.arg("--sandbox");

                if sandbox.auto_allow_bash_if_sandboxed == Some(true) {
                    cmd.arg("--sandbox-auto-allow-bash");
                }

                if let Some(ref excluded) = sandbox.excluded_commands {
                    cmd.arg("--sandbox-excluded-commands")
                        .arg(excluded.join(","));
                }

                if sandbox.allow_unsandboxed_commands == Some(true) {
                    cmd.arg("--sandbox-allow-unsandboxed");
                }

                if sandbox.enable_weaker_nested_sandbox == Some(true) {
                    cmd.arg("--sandbox-weaker-nested");
                }

                // Network settings
                if let Some(ref network) = sandbox.network {
                    if network.allow_local_binding == Some(true) {
                        cmd.arg("--sandbox-allow-local-binding");
                    }
                    if network.allow_all_unix_sockets == Some(true) {
                        cmd.arg("--sandbox-allow-all-unix-sockets");
                    }
                    if let Some(ref unix_sockets) = network.allow_unix_sockets {
                        cmd.arg("--sandbox-allow-unix-sockets")
                            .arg(unix_sockets.join(","));
                    }
                    if let Some(port) = network.http_proxy_port {
                        cmd.arg("--sandbox-http-proxy-port").arg(port.to_string());
                    }
                    if let Some(port) = network.socks_proxy_port {
                        cmd.arg("--sandbox-socks-proxy-port").arg(port.to_string());
                    }
                }

                // Ignore violations
                if let Some(ref ignore) = sandbox.ignore_violations {
                    if let Some(ref files) = ignore.file {
                        cmd.arg("--sandbox-ignore-file-violations")
                            .arg(files.join(","));
                    }
                    if let Some(ref networks) = ignore.network {
                        cmd.arg("--sandbox-ignore-network-violations")
                            .arg(networks.join(","));
                    }
                }
            }
        }

        // Plugins (local paths)
        if let Some(ref plugins) = self.options.plugins {
            for plugin in plugins {
                match plugin {
                    crate::types::SdkPluginConfig::Local { path } => {
                        cmd.arg("--plugin-dir").arg(path);
                    }
                }
            }
        }

        // Beta features
        if let Some(ref betas) = self.options.betas {
            for beta in betas {
                let beta_str = match beta {
                    crate::types::SdkBeta::Context1M => "context-1m-2025-08-07",
                };
                cmd.arg("--beta").arg(beta_str);
            }
        }

        // Strict MCP config validation
        if self.options.strict_mcp_config {
            cmd.arg("--strict-mcp-config");
        }

        // File checkpointing for rewind support
        // Enabled via environment variable, not CLI flag
        if self.options.enable_file_checkpointing {
            cmd.env("CLAUDE_CODE_ENABLE_SDK_FILE_CHECKPOINTING", "1");
        }

        // Resume session at specific message UUID
        if let Some(ref resume_at) = self.options.resume_session_at {
            cmd.arg("--resume-at").arg(resume_at);
        }

        // Extra args - strict allowlist enforcement
        // Reject any flags not in the allowlist to prevent CLI injection
        let disallowed: Vec<&String> = self
            .options
            .extra_args
            .keys()
            .filter(|flag| !ALLOWED_EXTRA_FLAGS.contains(&flag.as_str()))
            .collect();

        if !disallowed.is_empty() {
            let flags_str = disallowed
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            tracing::warn!(
                flags = %flags_str,
                allowed = ?ALLOWED_EXTRA_FLAGS,
                "Rejected disallowed CLI flags in extra_args"
            );
            return Err(ClaudeError::invalid_config(format!(
                "Disallowed CLI flags in extra_args: [{flags_str}]. Allowed flags: {ALLOWED_EXTRA_FLAGS:?}"
            )));
        }

        // All flags are allowed, add them
        for (flag, value) in &self.options.extra_args {
            if let Some(v) = value {
                cmd.arg(format!("--{flag}")).arg(v);
            } else {
                cmd.arg(format!("--{flag}"));
            }
        }

        // Prompt handling based on mode
        match &self.prompt {
            PromptInput::Stream => {
                // Streaming mode: use --input-format stream-json
                // --replay-user-messages enables CLI to read stdin during streaming
                cmd.arg("--input-format").arg("stream-json");
                cmd.arg("--replay-user-messages");
            }
            PromptInput::String(s) => {
                // String mode: pass the prompt as an argument after --
                cmd.arg("--").arg(s);
            }
        }

        Ok(cmd)
    }

    /// Get a child cancellation token for this transport
    /// Callers can use this to cancel ongoing operations
    #[must_use]
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.child_token()
    }

    /// Cancel all ongoing operations
    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    /// Serialize MCP config for CLI
    fn serialize_mcp_config(config: &crate::types::McpServerConfig) -> serde_json::Value {
        match config {
            crate::types::McpServerConfig::Stdio(stdio) => {
                let mut obj = serde_json::json!({
                    "command": stdio.command,
                });
                if let Some(ref args) = stdio.args {
                    obj["args"] = serde_json::json!(args);
                }
                if let Some(ref env) = stdio.env {
                    obj["env"] = serde_json::json!(env);
                }
                if let Some(ref server_type) = stdio.server_type {
                    obj["type"] = serde_json::json!(server_type);
                }
                obj
            }
            crate::types::McpServerConfig::Sse(sse) => {
                serde_json::json!({
                    "type": sse.server_type,
                    "url": sse.url,
                    "headers": sse.headers,
                })
            }
            crate::types::McpServerConfig::Http(http) => {
                serde_json::json!({
                    "type": http.server_type,
                    "url": http.url,
                    "headers": http.headers,
                })
            }
            crate::types::McpServerConfig::Sdk(sdk) => {
                let mut obj = serde_json::json!({
                    "type": "sdk",
                    "name": sdk.name,
                });
                if let Some(ref version) = sdk.version {
                    obj["version"] = serde_json::json!(version);
                }
                obj
            }
        }
    }
}

#[async_trait]
impl Transport for SubprocessTransport {
    async fn connect(&mut self) -> Result<()> {
        if self.process.is_some() {
            return Ok(());
        }

        tracing::debug!(
            "SubprocessTransport::connect - output_format is {:?}",
            self.options.output_format.as_ref().map(|f| &f.format_type)
        );

        let mut cmd = self.build_command()?;

        // Set up environment - strict enforcement of dangerous variable blocking
        let mut process_env = env::vars().collect::<HashMap<_, _>>();

        // Check for dangerous env vars in user-provided options (strict enforcement)
        let dangerous_found: Vec<&String> = self
            .options
            .env
            .keys()
            .filter(|key| DANGEROUS_ENV_VARS.contains(&key.as_str()))
            .collect();

        if !dangerous_found.is_empty() {
            let vars_str = dangerous_found
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            tracing::warn!(
                vars = %vars_str,
                "Rejected dangerous environment variables - possible injection attempt"
            );
            return Err(ClaudeError::invalid_config(format!(
                "Dangerous environment variables detected: [{vars_str}]. These are blocked to prevent injection attacks."
            )));
        }

        // All env vars are safe, add them
        for (key, value) in &self.options.env {
            process_env.insert(key.clone(), value.clone());
        }

        process_env.insert("CLAUDE_CODE_ENTRYPOINT".to_string(), "sdk-rust".to_string());
        process_env.insert("CLAUDE_AGENT_SDK_VERSION".to_string(), VERSION.to_string());

        if let Some(ref cwd) = self.cwd {
            process_env.insert("PWD".to_string(), cwd.to_string_lossy().to_string());
            cmd.current_dir(cwd);
        }

        cmd.envs(process_env);

        // Set up stdio
        // IMPORTANT: We pipe stderr instead of inheriting to prevent the child process
        // from manipulating the parent terminal state. Inheriting stderr gives the child
        // access to the terminal, which can leave it in a corrupted state.
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()); // Pipe stderr to prevent terminal manipulation

        // Log command for debugging
        tracing::debug!("CLI command: {:?}", cmd);

        // Spawn process
        let mut child = cmd.spawn().map_err(|e| {
            if let Some(ref cwd) = self.cwd {
                if !cwd.exists() {
                    #[cfg(debug_assertions)]
                    return ClaudeError::connection(format!(
                        "Working directory does not exist: {}",
                        cwd.display()
                    ));
                    #[cfg(not(debug_assertions))]
                    return ClaudeError::connection("Working directory does not exist".to_string());
                }
            }
            ClaudeError::connection(format!("Failed to start Claude Code: {e}"))
        })?;

        // Get stdin, stdout, and stderr
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ClaudeError::connection("Failed to get stdin handle"))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ClaudeError::connection("Failed to get stdout handle"))?;

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| ClaudeError::connection("Failed to get stderr handle"))?;

        // Spawn task to consume stderr to prevent blocking
        // We forward it to parent stderr for visibility
        let stderr_task = tokio::spawn(async move {
            use tokio::io::AsyncReadExt;
            let mut stderr = stderr;
            let mut buffer = vec![0u8; 4096];

            loop {
                match stderr.read(&mut buffer).await {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        // Forward stderr to parent's stderr
                        let _ = std::io::Write::write_all(&mut std::io::stderr(), &buffer[..n]);
                    }
                }
            }
        });

        // Store handles
        self.stdin = Some(stdin);
        self.stdout = Some(BufReader::new(stdout));
        self.process = Some(child);
        self.stderr_task = Some(stderr_task);
        self.ready.store(true, Ordering::SeqCst);

        // For string mode, close stdin immediately
        if matches!(self.prompt, PromptInput::String(_)) {
            if let Some(mut stdin) = self.stdin.take() {
                let _ = stdin.shutdown().await;
            }
        }

        Ok(())
    }

    async fn write(&mut self, data: &str) -> Result<()> {
        if !self.is_ready() {
            return Err(ClaudeError::transport("Transport is not ready for writing"));
        }

        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| ClaudeError::transport("stdin not available"))?;

        stdin
            .write_all(data.as_bytes())
            .await
            .map_err(|e| ClaudeError::transport(format!("Failed to write to stdin: {e}")))?;

        stdin
            .flush()
            .await
            .map_err(|e| ClaudeError::transport(format!("Failed to flush stdin: {e}")))?;

        Ok(())
    }

    async fn end_input(&mut self) -> Result<()> {
        if let Some(mut stdin) = self.stdin.take() {
            stdin
                .shutdown()
                .await
                .map_err(|e| ClaudeError::transport(format!("Failed to close stdin: {e}")))?;
        }
        Ok(())
    }

    fn read_messages(&mut self) -> mpsc::UnboundedReceiver<Result<serde_json::Value>> {
        let (tx, rx) = mpsc::unbounded_channel();

        // Take ownership of stdout and process
        let stdout = self.stdout.take();
        let process = Arc::new(Mutex::new(self.process.take()));
        let max_buffer_size = self.max_buffer_size;
        let cancel_token = self.cancellation_token.clone();

        // Spawn background task to read messages
        let task = tokio::spawn(async move {
            if stdout.is_none() {
                let _ = tx.send(Err(ClaudeError::connection(
                    "Not connected - stdout not available",
                )));
                return;
            }

            let mut stdout = stdout.unwrap();
            let mut json_buffer = String::new();

            loop {
                let mut line = String::new();

                // Use select! to allow cancellation - no hardcoded timeout
                // The caller controls cancellation via the CancellationToken
                tokio::select! {
                    // Check for cancellation
                    () = cancel_token.cancelled() => {
                        tracing::debug!("Read cancelled via CancellationToken");
                        break;
                    }
                    // Read next line
                    result = stdout.read_line(&mut line) => {
                        match result {
                            Ok(0) => break, // EOF
                            Ok(_) => {
                                let line = line.trim();
                                if line.is_empty() {
                                    continue;
                                }

                                // Accumulate partial JSON until we can parse it
                                json_buffer.push_str(line);

                                if json_buffer.len() > max_buffer_size {
                                    // Safe truncation for error preview (respects UTF-8 boundaries)
                                    let preview = truncate_for_display(&json_buffer, 100);
                                    let _ = tx.send(Err(ClaudeError::JsonDecode(
                                        serde_json::Error::io(std::io::Error::new(
                                            std::io::ErrorKind::InvalidData,
                                            format!(
                                                "JSON message exceeded maximum buffer size of {max_buffer_size} bytes. Preview: {preview}"
                                            ),
                                        )),
                                    )));
                                    json_buffer.clear();
                                    continue;
                                }

                                // Try to parse JSON
                                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&json_buffer) {
                                    let msg_type = data.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");
                                    tracing::trace!(msg_type, "Received message from CLI");

                                    // Debug log for result messages
                                    if msg_type == "result" {
                                        tracing::debug!(
                                            "Raw result JSON: {}",
                                            json_buffer.chars().take(500).collect::<String>()
                                        );
                                    }

                                    json_buffer.clear();
                                    if tx.send(Ok(data)).is_err() {
                                        // Receiver dropped, stop reading
                                        break;
                                    }
                                }
                                // else: Not complete yet, continue accumulating
                            }
                            Err(e) => {
                                let _ = tx.send(Err(ClaudeError::Io(e)));
                                break;
                            }
                        }
                    }
                }
            }

            // Check process exit code
            if let Ok(mut process_guard) = process.try_lock() {
                if let Some(mut child) = process_guard.take() {
                    match child.wait().await {
                        Ok(status) => {
                            if !status.success() {
                                if let Some(code) = status.code() {
                                    let _ = tx.send(Err(ClaudeError::process(
                                        "Command failed",
                                        code,
                                        Some("Check stderr output for details".to_string()),
                                    )));
                                }
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(Err(ClaudeError::Io(e)));
                        }
                    }
                }
            }
        });

        // Store task handle for cleanup
        self.reader_task = Some(task);

        rx
    }

    fn is_ready(&self) -> bool {
        self.ready.load(Ordering::SeqCst)
    }

    async fn close(&mut self) -> Result<()> {
        self.ready.store(false, Ordering::SeqCst);

        // Cancel any ongoing read operations via token
        self.cancellation_token.cancel();

        // Close stdin to signal the process to exit gracefully
        if let Some(mut stdin) = self.stdin.take() {
            let _ = stdin.shutdown().await;
        }

        // Wait for reader task to finish (it will exit due to cancellation)
        if let Some(task) = self.reader_task.take() {
            // Give a brief window for graceful exit before abort
            tokio::select! {
                _ = task => {}
                () = tokio::time::sleep(std::time::Duration::from_millis(100)) => {}
            }
        }
        if let Some(task) = self.stderr_task.take() {
            task.abort();
        }

        self.stdout = None;

        // Try to wait for the process to exit gracefully first
        if let Some(mut child) = self.process.take() {
            // Give the process a configurable timeout to exit gracefully
            let timeout_duration = std::time::Duration::from_secs(5);

            match tokio::time::timeout(timeout_duration, child.wait()).await {
                Ok(Ok(_status)) => {
                    // Process exited gracefully
                }
                Ok(Err(e)) => {
                    return Err(ClaudeError::Io(e));
                }
                Err(_) => {
                    // Timeout - kill the process
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                }
            }
        }

        Ok(())
    }
}

impl Drop for SubprocessTransport {
    fn drop(&mut self) {
        // Close stdin if still open to signal graceful shutdown
        if let Some(stdin) = self.stdin.take() {
            // Drop will close it
            drop(stdin);
        }

        // Abort reader task if running
        if let Some(task) = self.reader_task.take() {
            task.abort();
        }

        // Abort stderr task if running
        if let Some(task) = self.stderr_task.take() {
            task.abort();
        }

        // Try graceful shutdown first, then kill if needed
        if let Some(mut child) = self.process.take() {
            // Try to kill gracefully (SIGTERM on Unix)
            let _ = child.start_kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_cli() {
        // This will succeed if claude is installed
        let result = SubprocessTransport::find_cli();
        // We can't assert success because it depends on environment
        println!("CLI search result: {result:?}");
    }

    #[test]
    fn test_prompt_input_conversions() {
        let _prompt1: PromptInput = "hello".into();
        let _prompt2: PromptInput = String::from("world").into();
    }

    #[test]
    fn test_extra_args_allowlist_rejects_disallowed() {
        // Use same CLI discovery as production code
        let Ok(cli_path) = SubprocessTransport::find_cli() else {
            return; // Skip if CLI not installed
        };

        let mut options = ClaudeAgentOptions::default();
        options
            .extra_args
            .insert("dangerous-flag".to_string(), None);

        let transport = SubprocessTransport::new(PromptInput::Stream, options, Some(cli_path))
            .expect("Transport creation should succeed");

        let result = transport.build_command();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Disallowed CLI flags"));
        assert!(err.to_string().contains("dangerous-flag"));
    }

    #[test]
    fn test_extra_args_allowlist_accepts_allowed() {
        // Use same CLI discovery as production code
        let Ok(cli_path) = SubprocessTransport::find_cli() else {
            return; // Skip if CLI not installed
        };

        let mut options = ClaudeAgentOptions::default();
        options
            .extra_args
            .insert("timeout".to_string(), Some("30".to_string()));
        options
            .extra_args
            .insert("log-level".to_string(), Some("debug".to_string()));

        let transport = SubprocessTransport::new(PromptInput::Stream, options, Some(cli_path))
            .expect("Transport creation should succeed");

        let result = transport.build_command();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dangerous_env_vars_rejected() {
        // Use same CLI discovery as production code
        let Ok(cli_path) = SubprocessTransport::find_cli() else {
            return; // Skip if CLI not installed
        };

        let mut options = ClaudeAgentOptions::default();
        options
            .env
            .insert("LD_PRELOAD".to_string(), "/tmp/evil.so".to_string());

        let mut transport = SubprocessTransport::new(PromptInput::Stream, options, Some(cli_path))
            .expect("Transport creation should succeed");

        let result = transport.connect().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Dangerous environment variables"));
        assert!(err.to_string().contains("LD_PRELOAD"));
    }

    #[tokio::test]
    async fn test_safe_env_vars_accepted() {
        // Use same CLI discovery as production code
        let Ok(cli_path) = SubprocessTransport::find_cli() else {
            return; // Skip if CLI not installed
        };

        let mut options = ClaudeAgentOptions::default();
        options
            .env
            .insert("MY_SAFE_VAR".to_string(), "safe value".to_string());

        let mut transport = SubprocessTransport::new(PromptInput::Stream, options, Some(cli_path))
            .expect("Transport creation should succeed");

        // Connect should succeed (env vars are safe)
        let result = transport.connect().await;
        assert!(result.is_ok());

        // Cleanup
        transport.close().await.ok();
    }

    #[test]
    fn test_session_id_option_maps_to_cli_arg() {
        let Ok(cli_path) = SubprocessTransport::find_cli() else {
            return; // Skip if CLI not installed
        };

        let options = ClaudeAgentOptions::builder()
            .session_id("550e8400-e29b-41d4-a716-446655440000")
            .build();

        let transport = SubprocessTransport::new(PromptInput::Stream, options, Some(cli_path))
            .expect("Transport creation should succeed");

        let cmd = transport
            .build_command()
            .expect("build_command should succeed");
        // Convert tokio Command to std Command to access get_args()
        let std_cmd = cmd.as_std();
        let args: Vec<String> = std_cmd
            .get_args()
            .filter_map(|a| a.to_str().map(String::from))
            .collect();

        // Find --session-id flag
        let session_id_idx = args.iter().position(|a| a == "--session-id");
        assert!(
            session_id_idx.is_some(),
            "Expected --session-id flag in args"
        );

        // Verify the value follows the flag
        let value = &args[session_id_idx.unwrap() + 1];
        assert_eq!(value, "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_plugin_dir_flag_correct() {
        let Ok(cli_path) = SubprocessTransport::find_cli() else {
            return; // Skip if CLI not installed
        };

        let options = ClaudeAgentOptions::builder()
            .plugins(vec![crate::types::SdkPluginConfig::Local {
                path: "/path/to/plugin".to_string(),
            }])
            .build();

        let transport = SubprocessTransport::new(PromptInput::Stream, options, Some(cli_path))
            .expect("Transport creation should succeed");

        let cmd = transport
            .build_command()
            .expect("build_command should succeed");
        // Convert tokio Command to std Command to access get_args()
        let std_cmd = cmd.as_std();
        let args: Vec<String> = std_cmd
            .get_args()
            .filter_map(|a| a.to_str().map(String::from))
            .collect();

        // Should use --plugin-dir, not --plugin
        let plugin_dir_idx = args.iter().position(|a| a == "--plugin-dir");
        assert!(
            plugin_dir_idx.is_some(),
            "Expected --plugin-dir flag in args, got: {args:?}"
        );

        // Verify the value follows the flag
        let value = &args[plugin_dir_idx.unwrap() + 1];
        assert_eq!(value, "/path/to/plugin");
    }
}
