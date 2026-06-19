//! Connect-time provider probe — the Rust port of
//! `apps/web/server/api/ai/connect-agent.ts`.
//!
//! Pressing Connect on a provider card runs a real probe: binary
//! resolution (installed?), a responsiveness gate where TS runs one
//! (`codex --version` / `gemini --version`), a live model query, and
//! an auth-state read that yields the human-readable
//! `connectionInfo` string ("Connected via pro (a@b.c)"). Failures
//! map through the same friendly-error tables the TS route uses.
//!
//! Everything here is blocking and runs on the probe worker thread
//! (`provider_probe_host.rs`); nothing touches the UI thread.

use std::io::Read;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use base64::Engine;
use op_ai::agent_settings_state::AgentProvider;
use op_ai::chat_models::ModelEntry;

use crate::model_discovery::{
    codex_models_from_app_server, codex_models_from_cache, discover_opencode, extract_json_object,
    parse_copilot_model_list, resolve_cli, write_lsp_frame,
};
use crate::provider_probe_models::{
    claude_initialize_query, codex_home, codex_models_from_latest_md, gemini_models_from_api,
    ClaudeAccount, ClaudeInitResult,
};

/// Probe result — the TS `ConnectResult` shape plus the install
/// guidance the not-installed path surfaces.
#[derive(Debug, Clone, Default)]
pub struct ProbeOutcome {
    pub connected: bool,
    pub models: Vec<ModelEntry>,
    pub error: Option<String>,
    pub warning: Option<String>,
    pub not_installed: bool,
    pub install_command: Option<String>,
    pub connection_info: Option<String>,
    pub hint_path: Option<String>,
    pub version: Option<String>,
}

impl ProbeOutcome {
    fn not_installed(provider: AgentProvider, error: &str) -> Self {
        Self {
            not_installed: true,
            error: Some(error.to_string()),
            install_command: Some(install_command(provider).to_string()),
            ..Self::default()
        }
    }

    fn failed(error: String) -> Self {
        Self {
            error: Some(error),
            ..Self::default()
        }
    }
}

fn no_models_error(provider: AgentProvider) -> String {
    match provider {
        AgentProvider::ClaudeCode => {
            "No models found. Claude Code did not return a model list. Run \"claude login\" to authenticate, or set ANTHROPIC_API_KEY in ~/.claude/settings.json.".to_string()
        }
        AgentProvider::CodexCli => {
            "No models found. Run \"codex login\" or set OPENAI_API_KEY, then run codex once to populate the model cache.".to_string()
        }
        AgentProvider::OpenCode => {
            "No models configured in OpenCode. Run \"opencode\" to set up providers.".to_string()
        }
        AgentProvider::GithubCopilot => {
            "No models found. Run \"copilot login\" to authenticate first.".to_string()
        }
        AgentProvider::GeminiCli => {
            "No models found. Run \"gemini\" once to authenticate, or set GEMINI_API_KEY.".to_string()
        }
    }
}

fn connected_probe_outcome(
    provider: AgentProvider,
    models: Vec<ModelEntry>,
    connection_info: Option<String>,
    warning: Option<String>,
    hint_path: Option<String>,
    version: Option<String>,
) -> ProbeOutcome {
    if models.is_empty() {
        return ProbeOutcome::failed(no_models_error(provider));
    }
    ProbeOutcome {
        connected: true,
        models,
        connection_info,
        warning,
        hint_path,
        version,
        ..ProbeOutcome::default()
    }
}

/// Run the connect probe for one provider. Blocking.
pub fn connect_provider(provider: AgentProvider) -> ProbeOutcome {
    match provider {
        AgentProvider::ClaudeCode => connect_claude_code(),
        AgentProvider::CodexCli => connect_codex_cli(),
        AgentProvider::OpenCode => connect_opencode(),
        AgentProvider::GithubCopilot => connect_copilot(),
        AgentProvider::GeminiCli => connect_gemini_cli(),
    }
}

/// Manual install command per provider — TS
/// `install-agent.ts::getInstallInfo` (lines 37-76). The TS
/// auto-install execution (`npm install -g` from the app) is NOT
/// ported; the command is surfaced as guidance instead.
fn install_command(provider: AgentProvider) -> &'static str {
    match provider {
        AgentProvider::ClaudeCode => "npm install -g @anthropic-ai/claude-code",
        AgentProvider::CodexCli => "npm install -g @openai/codex",
        AgentProvider::OpenCode => {
            if cfg!(windows) {
                "npm install -g opencode-ai"
            } else {
                "curl -fsSL https://opencode.ai/install | bash"
            }
        }
        AgentProvider::GithubCopilot => {
            if cfg!(target_os = "macos") {
                "brew install github/copilot/copilot"
            } else if cfg!(windows) {
                "winget install GitHub.CopilotCLI"
            } else {
                "See documentation"
            }
        }
        AgentProvider::GeminiCli => "npm install -g @anthropic-ai/gemini-cli",
    }
}

/// Cross-platform config-path hint (TS `configPath`).
fn config_path(unix: &str, win: &str) -> String {
    if cfg!(windows) {
        win.to_string()
    } else {
        unix.to_string()
    }
}

/// `key.slice(0, 8) + "..."` masking (TS connect-agent.ts:291).
fn mask_key(key: &str) -> String {
    if key.len() > 12 {
        let head: String = key.chars().take(8).collect();
        format!("{head}...")
    } else {
        "***".to_string()
    }
}

/// Decode a JWT payload — base64url-decode the middle part, no
/// signature verification (TS `decodeJwtPayload`).
fn decode_jwt_payload(token: &str) -> Option<serde_json::Value> {
    let mut parts = token.split('.');
    let _header = parts.next()?;
    let payload = parts.next()?;
    if parts.next().is_none() || payload.is_empty() {
        return None;
    }
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Run `exe --version`, capturing stdout+stderr (the TS gates run
/// with `2>&1`). `None` on spawn failure, non-zero exit, or
/// timeout — the "CLI not responding" path.
fn cli_version(exe: &Path, timeout: Duration) -> Option<String> {
    let mut child = Command::new(exe)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                // --version output is tiny — far below the pipe
                // buffer — so reading after exit cannot block.
                let mut out = String::new();
                if let Some(mut stdout) = child.stdout.take() {
                    let _ = stdout.read_to_string(&mut out);
                }
                if out.trim().is_empty() {
                    if let Some(mut stderr) = child.stderr.take() {
                        let _ = stderr.read_to_string(&mut out);
                    }
                }
                return Some(out.trim().to_string());
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return None,
        }
    }
}

// ---------------------------------------------------------------
// Claude Code (connect-agent.ts:147-262)
// ---------------------------------------------------------------

fn connect_claude_code() -> ProbeOutcome {
    if resolve_cli("claude").is_none() {
        return ProbeOutcome::not_installed(AgentProvider::ClaudeCode, "Claude Code CLI not found");
    }
    match claude_initialize_query() {
        ClaudeInitResult::Answered(models, account) => {
            let (info, hint) = build_claude_connection_info(account.as_ref());
            connected_probe_outcome(
                AgentProvider::ClaudeCode,
                models,
                Some(info),
                None,
                Some(hint),
                None,
            )
        }
        ClaudeInitResult::ExitedWithError(code) => ProbeOutcome::failed(friendly_claude_error(
            &format!("process exited with code {code}"),
        )),
        ClaudeInitResult::NoAnswer => {
            ProbeOutcome::failed(no_models_error(AgentProvider::ClaudeCode))
        }
    }
}

/// `ANTHROPIC_API_KEY` / `ANTHROPIC_BASE_URL` the way the TS env
/// builder resolves them (resolve-claude-agent-env.ts:110-136),
/// including the `ANTHROPIC_AUTH_TOKEN` → `ANTHROPIC_API_KEY`
/// compat rule applied when no API key is set.
fn claude_env_var(name: &str) -> Option<String> {
    if let Some(value) = claude_env_var_resolved(name) {
        return Some(value);
    }
    if name == "ANTHROPIC_API_KEY" {
        return claude_env_var_resolved("ANTHROPIC_AUTH_TOKEN");
    }
    None
}

/// One env name through the TS precedence chain — the
/// `{...fromSettings, ...fromProcess}` spread: process env wins
/// over the `~/.claude/settings.local.json` env block, which wins
/// over `settings.json`. A key present at a higher tier shadows the
/// lower tiers even when its value is blank (a blank value reads as
/// absent, like the TS falsy check at the use site).
fn claude_env_var_resolved(name: &str) -> Option<String> {
    if let Some(value) = std::env::var_os(name) {
        let value = value.to_string_lossy().into_owned();
        return (!value.trim().is_empty()).then_some(value);
    }
    let dir = dirs::home_dir()?.join(".claude");
    for file in ["settings.local.json", "settings.json"] {
        let Ok(raw) = std::fs::read_to_string(dir.join(file)) else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
            continue;
        };
        let Some(value) = json.get("env").and_then(|e| e.get(name)) else {
            continue;
        };
        let text = match value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            _ => continue,
        };
        return (!text.trim().is_empty()).then_some(text);
    }
    None
}

/// TS `buildClaudeConnectionInfo` (connect-agent.ts:270-295).
fn build_claude_connection_info(account: Option<&ClaudeAccount>) -> (String, String) {
    let hint = config_path(
        "~/.claude/settings.json",
        "%USERPROFILE%\\.claude\\settings.json",
    );
    if let Some(email) = account.and_then(|a| a.email.as_deref()) {
        let sub = account
            .and_then(|a| a.subscription_type.as_deref())
            .unwrap_or("subscription");
        return (format!("Connected via {sub} ({email})"), hint);
    }
    let api_key = claude_env_var("ANTHROPIC_API_KEY");
    let base_url = claude_env_var("ANTHROPIC_BASE_URL");
    match (api_key, base_url) {
        (Some(_), Some(_)) => ("Connected via API key (custom base URL)".to_string(), hint),
        (Some(key), None) => (format!("Connected via API key ({})", mask_key(&key)), hint),
        _ => ("Connected via subscription".to_string(), hint),
    }
}

/// TS `friendlyClaudeError` (connect-agent.ts:357-371).
fn friendly_claude_error(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("process exited with code 1")
        || lower.contains("invalid model")
        || lower.contains("unknown model")
    {
        return "Claude Code exited with code 1. Run \"claude login\" to authenticate, or set ANTHROPIC_API_KEY in ~/.claude/settings.json.".to_string();
    }
    if lower.contains("exited with code") {
        return "Unable to connect. Claude Code process exited unexpectedly.".to_string();
    }
    if lower.contains("not found") || lower.contains("enoent") {
        return "Claude Code CLI not found. Please install it first.".to_string();
    }
    if lower.contains("timed out") || lower.contains("timedout") {
        return "Connection timed out. Please try again.".to_string();
    }
    raw.to_string()
}

// ---------------------------------------------------------------
// Codex CLI (connect-agent.ts:416-576)
// ---------------------------------------------------------------

fn connect_codex_cli() -> ProbeOutcome {
    let Some(exe) = resolve_cli("codex") else {
        return ProbeOutcome::not_installed(AgentProvider::CodexCli, "Codex CLI not found");
    };
    // Responsiveness gate — TS runs `codex --version` with a 5 s
    // budget (connect-agent.ts:512-522).
    let Some(version) = cli_version(&exe, Duration::from_secs(5)) else {
        return ProbeOutcome::failed("Codex CLI not responding".to_string());
    };
    // Model list: app-server protocol (the approved no-hardcoded-
    // lists design; a superset of TS, which reads only the cache),
    // then the on-disk cache, then latest-model.md (both TS paths).
    let mut models = codex_models_from_app_server().unwrap_or_default();
    if models.is_empty() {
        models = codex_models_from_cache().unwrap_or_default();
    }
    if models.is_empty() {
        if let Some(home) = codex_home() {
            models = codex_models_from_latest_md(&home);
        }
    }
    if !codex_has_auth() {
        return ProbeOutcome::failed(
            "Not authenticated. Run \"codex login\" or set OPENAI_API_KEY first.".to_string(),
        );
    }
    let (info, hint) = build_codex_connection_info();
    connected_probe_outcome(
        AgentProvider::CodexCli,
        models,
        Some(info),
        None,
        Some(hint),
        Some(version),
    )
}

/// TS `buildCodexConnectionInfo` (connect-agent.ts:312-354) —
/// `OPENAI_API_KEY` first, then the `auth.json` JWT.
fn build_codex_connection_info() -> (String, String) {
    let hint = config_path("~/.codex/config.toml", "%USERPROFILE%\\.codex\\config.toml");
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        if !key.is_empty() {
            return (format!("Connected via API key ({})", mask_key(&key)), hint);
        }
    }
    let auth_raw = codex_home()
        .map(|home| home.join("auth.json"))
        .and_then(|path| std::fs::read_to_string(path).ok());
    (codex_connection_info_from_auth(auth_raw.as_deref()), hint)
}

/// Pure half of the Codex auth-info builder so the JWT/auth.json
/// parsing is unit-testable.
fn codex_connection_info_from_auth(auth_raw: Option<&str>) -> String {
    let fallback = "Connected via Codex CLI".to_string();
    let Some(raw) = auth_raw else { return fallback };
    let Ok(auth) = serde_json::from_str::<serde_json::Value>(raw) else {
        return fallback;
    };
    let auth_mode = auth.get("auth_mode").and_then(|v| v.as_str());
    if let Some(id_token) = auth
        .get("tokens")
        .and_then(|t| t.get("id_token"))
        .and_then(|v| v.as_str())
    {
        if let Some(payload) = decode_jwt_payload(id_token) {
            let email = payload.get("email").and_then(|v| v.as_str());
            let plan = payload
                .get("https://api.openai.com/auth")
                .and_then(|a| a.get("chatgpt_plan_type"))
                .and_then(|v| v.as_str());
            if let Some(email) = email {
                let label = plan.or(auth_mode).unwrap_or("subscription");
                return format!("Connected via {label} ({email})");
            }
        }
    }
    if let Some(mode) = auth_mode {
        return format!("Connected via {mode}");
    }
    fallback
}

fn codex_has_auth() -> bool {
    if std::env::var("OPENAI_API_KEY")
        .ok()
        .is_some_and(|key| !key.trim().is_empty())
    {
        return true;
    }
    let auth_raw = codex_home()
        .map(|home| home.join("auth.json"))
        .and_then(|path| std::fs::read_to_string(path).ok());
    codex_auth_present(auth_raw.as_deref())
}

fn codex_auth_present(auth_raw: Option<&str>) -> bool {
    let Some(raw) = auth_raw else { return false };
    let Ok(auth) = serde_json::from_str::<serde_json::Value>(raw) else {
        return false;
    };
    auth.get("tokens")
        .and_then(|t| t.get("id_token"))
        .and_then(|v| v.as_str())
        .is_some_and(|token| !token.trim().is_empty())
        || auth
            .get("auth_mode")
            .and_then(|v| v.as_str())
            .is_some_and(|mode| !mode.trim().is_empty())
}

// ---------------------------------------------------------------
// OpenCode (connect-agent.ts:673-740)
// ---------------------------------------------------------------

fn connect_opencode() -> ProbeOutcome {
    if resolve_cli("opencode").is_none() {
        return ProbeOutcome::not_installed(AgentProvider::OpenCode, "OpenCode CLI not found");
    }
    // Model query via `opencode models` — Rust's sanctioned
    // transport divergence from the TS `opencode serve` SDK round
    // trip (the listing is equivalent: `provider/model` slugs).
    let models = discover_opencode();
    if models.is_empty() {
        return ProbeOutcome::failed(
            "No models configured in OpenCode. Run \"opencode\" to set up providers.".to_string(),
        );
    }
    let info = opencode_provider_summary(&models);
    ProbeOutcome {
        connected: true,
        models,
        connection_info: Some(info),
        hint_path: Some(config_path(
            "~/.opencode/config.json",
            "%USERPROFILE%\\.opencode\\config.json",
        )),
        ..ProbeOutcome::default()
    }
}

/// "Connected (anthropic, openai, google +2)" — the TS provider
/// summary (connect-agent.ts:723-727), derived here from the
/// distinct `provider/` slug prefixes instead of the serve-API's
/// provider display names.
fn opencode_provider_summary(models: &[ModelEntry]) -> String {
    let mut names: Vec<&str> = Vec::new();
    for m in models {
        if let Some((provider, _)) = m.value.split_once('/') {
            if !provider.is_empty() && !names.contains(&provider) {
                names.push(provider);
            }
        }
    }
    if names.is_empty() {
        return "Connected via OpenCode server".to_string();
    }
    let shown = names[..names.len().min(3)].join(", ");
    if names.len() > 3 {
        format!("Connected ({shown} +{})", names.len() - 3)
    } else {
        format!("Connected ({shown})")
    }
}

// ---------------------------------------------------------------
// GitHub Copilot (connect-agent.ts:742-839)
// ---------------------------------------------------------------

/// Auth status from `auth.getStatus` — the wire method the official
/// copilot-sdk's `getAuthStatus()` sends (client.js:549-555).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct CopilotAuth {
    login: Option<String>,
    auth_type: Option<String>,
    status_message: Option<String>,
}

const COPILOT_PROBE_TIMEOUT: Duration = Duration::from_secs(8);

fn connect_copilot() -> ProbeOutcome {
    let Some(exe) = resolve_cli("copilot") else {
        return ProbeOutcome::not_installed(
            AgentProvider::GithubCopilot,
            "GitHub Copilot CLI not found",
        );
    };
    let Some((models, auth)) = copilot_probe_stdio(&exe) else {
        return ProbeOutcome::failed(friendly_copilot_error("Connection timed out"));
    };
    if models.is_empty() {
        return ProbeOutcome::failed(
            "No models found. Run \"copilot login\" to authenticate first.".to_string(),
        );
    }
    let hint = config_path(
        "~/.config/github-copilot/config.json",
        "%USERPROFILE%\\.config\\github-copilot\\config.json",
    );
    let info = copilot_connection_info(auth.as_ref());
    ProbeOutcome {
        connected: true,
        models,
        connection_info: Some(info),
        hint_path: Some(hint),
        ..ProbeOutcome::default()
    }
}

/// TS connectCopilot's status mapping (connect-agent.ts:799-819).
fn copilot_connection_info(auth: Option<&CopilotAuth>) -> String {
    if let Some(auth) = auth {
        if let Some(login) = auth.login.as_deref() {
            let method = auth
                .auth_type
                .as_deref()
                .map(|t| format!(" ({t})"))
                .unwrap_or_default();
            return format!("Connected as @{login}{method}");
        }
        if let Some(message) = auth.status_message.as_deref() {
            return message.to_string();
        }
    }
    "Connected via GitHub".to_string()
}

/// One `copilot --stdio` session: `connect` (id 1) → `models.list`
/// (id 2) → `auth.getStatus` (id 3), all pipelined. Returns `None`
/// when the model list never answered; auth is best-effort (TS
/// logs and continues when `getAuthStatus` fails).
fn copilot_probe_stdio(exe: &Path) -> Option<(Vec<ModelEntry>, Option<CopilotAuth>)> {
    let mut child = Command::new(exe)
        .arg("--stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdin = child.stdin.take()?;
    let stdout = child.stdout.take()?;

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    let connect = r#"{"jsonrpc":"2.0","id":1,"method":"connect","params":{}}"#;
    let list = r#"{"jsonrpc":"2.0","id":2,"method":"models.list","params":{}}"#;
    let auth = r#"{"jsonrpc":"2.0","id":3,"method":"auth.getStatus","params":{}}"#;
    write_lsp_frame(&mut stdin, connect).ok()?;
    write_lsp_frame(&mut stdin, list).ok()?;
    write_lsp_frame(&mut stdin, auth).ok()?;
    use std::io::Write as _;
    stdin.flush().ok();

    let deadline = Instant::now() + COPILOT_PROBE_TIMEOUT;
    let mut models: Option<Vec<ModelEntry>> = None;
    let mut auth_status: Option<CopilotAuth> = None;
    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        if models.is_some() && auth_status.is_some() {
            break;
        }
        match rx.recv_timeout(remaining) {
            Ok(line) => {
                if models.is_none() {
                    if let Some(parsed) = parse_copilot_model_list(&line) {
                        models = Some(parsed);
                        continue;
                    }
                }
                if auth_status.is_none() {
                    if let Some(parsed) = parse_copilot_auth_status(&line) {
                        auth_status = Some(parsed);
                    }
                }
            }
            Err(_) => break,
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    models.map(|m| (m, auth_status))
}

/// Parse the `id:3` (`auth.getStatus`) response.
fn parse_copilot_auth_status(line: &str) -> Option<CopilotAuth> {
    let json: serde_json::Value = serde_json::from_str(extract_json_object(line)?).ok()?;
    if json.get("id")?.as_i64()? != 3 {
        return None;
    }
    let result = json.get("result")?;
    Some(CopilotAuth {
        login: result
            .get("login")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        auth_type: result
            .get("authType")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        status_message: result
            .get("statusMessage")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    })
}

/// TS `friendlyCopilotError` (connect-agent.ts:842-853).
fn friendly_copilot_error(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("not found") || lower.contains("enoent") {
        return "GitHub Copilot CLI not found. Install it from https://docs.github.com/copilot/how-tos/copilot-cli".to_string();
    }
    if lower.contains("not authenticated")
        || lower.contains("authenticate first")
        || lower.contains("auth")
        || lower.contains("unauthenticated")
        || lower.contains("login")
    {
        return "Not authenticated. Run \"copilot login\" in your terminal first.".to_string();
    }
    if lower.contains("timed out") || lower.contains("timedout") {
        return "Connection timed out. Please try again.".to_string();
    }
    raw.to_string()
}

// ---------------------------------------------------------------
// Gemini CLI (connect-agent.ts:975-1063)
// ---------------------------------------------------------------

fn connect_gemini_cli() -> ProbeOutcome {
    let Some(exe) = resolve_cli("gemini") else {
        return ProbeOutcome::not_installed(AgentProvider::GeminiCli, "Gemini CLI not found");
    };
    // Responsiveness gate — TS runs `gemini --version` with a 10 s
    // budget (connect-agent.ts:986-997).
    let Some(version) = cli_version(&exe, Duration::from_secs(10)) else {
        return ProbeOutcome::failed("Gemini CLI not responding".to_string());
    };
    let models = match gemini_models_from_api() {
        Some(models) if !models.is_empty() => models,
        Some(_) => return ProbeOutcome::failed(no_models_error(AgentProvider::GeminiCli)),
        None => {
            return ProbeOutcome::failed(
                "Unable to fetch Gemini models. Run \"gemini\" once to authenticate, or set GEMINI_API_KEY.".to_string(),
            )
        }
    };
    let (info, hint) = build_gemini_connection_info();
    connected_probe_outcome(
        AgentProvider::GeminiCli,
        models,
        Some(info),
        None,
        Some(hint),
        Some(version),
    )
}

/// TS `buildGeminiConnectionInfo` (connect-agent.ts:1027-1063).
fn build_gemini_connection_info() -> (String, String) {
    let hint = config_path(
        "~/.gemini/settings.json",
        "%USERPROFILE%\\.gemini\\settings.json",
    );
    let env_key = std::env::var("GEMINI_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
        .or_else(|| {
            std::env::var("GOOGLE_API_KEY")
                .ok()
                .filter(|k| !k.is_empty())
        });
    if let Some(key) = env_key {
        return (format!("Connected via API key ({})", mask_key(&key)), hint);
    }
    let gemini_dir = dirs::home_dir().map(|h| h.join(".gemini"));
    if let Some(dir) = gemini_dir {
        if dir.join("oauth_creds.json").is_file() {
            // Try the active Google account email.
            if let Ok(raw) = std::fs::read_to_string(dir.join("google_accounts.json")) {
                if let Ok(accounts) = serde_json::from_str::<serde_json::Value>(&raw) {
                    if let Some(active) = accounts.get("active").and_then(|v| v.as_str()) {
                        return (format!("Connected via Google ({active})"), hint);
                    }
                }
            }
            return ("Connected via Google OAuth".to_string(), hint);
        }
    }
    ("Connected via Gemini CLI".to_string(), hint)
}

#[cfg(test)]
#[path = "provider_probe_tests.rs"]
mod tests;
