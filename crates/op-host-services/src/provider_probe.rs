//! Connect-time provider probe — the Rust port of
//! `apps/web/server/api/ai/connect-agent.ts`.
//!
//! Pressing Connect on a provider card runs a real probe: binary
//! resolution (installed?), a responsiveness gate where TS runs one
//! (`codex --version`), a live model query, and
//! an auth-state read that yields the human-readable
//! `connectionInfo` string ("Connected via pro (a@b.c)"). Failures
//! map through the same friendly-error tables the TS route uses.
//!
//! Everything here is blocking and runs on the probe worker thread
//! (`provider_probe_host.rs`); nothing touches the UI thread.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use base64::Engine;
use op_ai::agent_settings_state::AgentProvider;
use op_ai::chat_models::ModelEntry;
use op_i18n::Locale;

use crate::model_discovery::{
    codex_models_from_app_server, codex_models_from_cache, discover_opencode, resolve_cli,
};
use crate::provider_probe_models::{
    claude_initialize_query, codex_home, codex_models_from_latest_md, ClaudeAccount,
    ClaudeInitResult,
};

#[path = "provider_probe_copilot.rs"]
mod copilot;
use copilot::connect_copilot;
#[cfg(test)]
use copilot::{
    copilot_connection_info, friendly_copilot_error, parse_copilot_auth_status, CopilotAuth,
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
    fn not_installed(provider: AgentProvider, error: String) -> Self {
        Self {
            not_installed: true,
            error: Some(error),
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

/// Resolve the UI locale for probe messages the same way app startup
/// does: OS locale first, overridden by the persisted settings file.
/// The probe worker has no handle on the live editor state, and locale
/// changes are persisted on change, so this stays in sync in practice.
pub fn resolved_ui_locale() -> Locale {
    let mut state = op_editor_core::EditorState::new();
    crate::settings_io::load(&mut state);
    state.editor_ui.locale
}

fn t(locale: Locale, key: &'static str) -> String {
    op_i18n::translate(locale, key).to_string()
}

fn tw(locale: Locale, key: &'static str, vars: &[(&str, &str)]) -> String {
    op_i18n::translate_with(locale, key, vars)
}

fn no_models_error(locale: Locale, provider: AgentProvider) -> String {
    let key = match provider {
        AgentProvider::ClaudeCode => "providerProbe.noModelsClaude",
        AgentProvider::CodexCli => "providerProbe.noModelsCodex",
        AgentProvider::OpenCode => "providerProbe.noModelsOpenCode",
        AgentProvider::GithubCopilot => "providerProbe.noModelsCopilot",
        AgentProvider::Antigravity => "providerProbe.noModelsAntigravity",
        AgentProvider::GrokBuild => "providerProbe.noModelsGrok",
        // dsh exposes no verified model-listing command, so the probe
        // never queries one — this arm only covers a hypothetical
        // empty-catalog outcome, reusing the shared key + name var.
        AgentProvider::DeepSeekHarness => {
            return tw(
                locale,
                "providerProbe.noModelList",
                &[("name", "DeepSeek Harness")],
            );
        }
    };
    t(locale, key)
}

fn connected_probe_outcome(
    locale: Locale,
    provider: AgentProvider,
    models: Vec<ModelEntry>,
    connection_info: Option<String>,
    warning: Option<String>,
    hint_path: Option<String>,
    version: Option<String>,
) -> ProbeOutcome {
    if models.is_empty() {
        return ProbeOutcome::failed(no_models_error(locale, provider));
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

/// Run the connect probe for one provider. Blocking. Status/error
/// strings are produced in `locale` at construction time — the display
/// layer shows them verbatim.
pub fn connect_provider(provider: AgentProvider, locale: Locale) -> ProbeOutcome {
    match provider {
        AgentProvider::ClaudeCode => connect_claude_code(locale),
        AgentProvider::CodexCli => connect_codex_cli(locale),
        AgentProvider::OpenCode => connect_opencode(locale),
        AgentProvider::GithubCopilot => connect_copilot(locale),
        AgentProvider::Antigravity => crate::cli_provider_probe::connect_antigravity(),
        AgentProvider::GrokBuild => crate::cli_provider_probe::connect_grok_build(),
        AgentProvider::DeepSeekHarness => crate::cli_provider_probe::connect_deepseek_harness(),
    }
}

/// Manual install command per provider — TS
/// `install-agent.ts::getInstallInfo` (lines 37-76). The TS
/// auto-install execution (`npm install -g` from the app) is NOT
/// ported; the command is surfaced as guidance instead.
pub(crate) fn install_command(provider: AgentProvider) -> &'static str {
    install_command_for_platform(provider, cfg!(windows), cfg!(target_os = "macos"))
}

fn install_command_for_platform(
    provider: AgentProvider,
    windows: bool,
    macos: bool,
) -> &'static str {
    match provider {
        AgentProvider::ClaudeCode => "npm install -g @anthropic-ai/claude-code",
        AgentProvider::CodexCli => "npm install -g @openai/codex",
        AgentProvider::OpenCode => {
            if windows {
                "npm install -g opencode-ai"
            } else {
                "curl -fsSL https://opencode.ai/install | bash"
            }
        }
        AgentProvider::GithubCopilot => {
            if macos {
                "brew install github/copilot/copilot"
            } else if windows {
                "winget install GitHub.Copilot"
            } else {
                "See documentation"
            }
        }
        AgentProvider::Antigravity if windows => {
            "irm https://antigravity.google/cli/install.ps1 | iex"
        }
        AgentProvider::Antigravity => "curl -fsSL https://antigravity.google/cli/install.sh | bash",
        AgentProvider::GrokBuild if windows => "irm https://x.ai/cli/install.ps1 | iex",
        AgentProvider::GrokBuild => "curl -fsSL https://x.ai/cli/install.sh | bash",
        AgentProvider::DeepSeekHarness => "npm install -g @deepseek-ai/dsh",
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

/// Why a `--version` responsiveness gate failed. Structured (rather
/// than a bare `None`) so the card's error text can name what the CLI
/// itself reported: a GUI launch missing the user's PATH makes
/// `codex` — a `#!/usr/bin/env node` script — die with
/// `env: node: No such file or directory` on stderr and exit 127,
/// which read as an indistinguishable "CLI not responding" while the
/// actual fix was in the message all along.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CliVersionFailure {
    /// The process never started (missing binary, bad permissions).
    Spawn(String),
    /// It ran and exited non-zero; `tail` is its own output.
    Exited { status: String, tail: String },
    /// Still running at the deadline; `tail` is whatever it printed.
    TimedOut { seconds: u64, tail: String },
}

/// Human-readable card text for a failed version gate. `provider` is
/// the CLI's display name so each provider keeps its own wording.
pub(crate) fn cli_version_failure_message(provider: &str, failure: &CliVersionFailure) -> String {
    match failure {
        CliVersionFailure::Spawn(error) => format!("{provider} CLI failed to start: {error}"),
        CliVersionFailure::Exited { status, tail } if tail.is_empty() => {
            format!("{provider} CLI exited with status {status} and no output")
        }
        CliVersionFailure::Exited { tail, .. } => format!("{provider} CLI failed: {tail}"),
        CliVersionFailure::TimedOut { seconds, tail } if tail.is_empty() => {
            format!("{provider} CLI did not respond within {seconds}s")
        }
        CliVersionFailure::TimedOut { seconds, tail } => {
            format!("{provider} CLI did not respond within {seconds}s. Last output: {tail}")
        }
    }
}

/// Run `exe --version`, capturing stdout+stderr (the TS gates run
/// with `2>&1`). Both streams are drained on background threads so a
/// chatty or hung CLI can neither fill a pipe nor cost us its output
/// when the deadline kills it.
fn cli_version(exe: &Path, timeout: Duration) -> Result<String, CliVersionFailure> {
    let mut cmd = Command::new(exe);
    cmd.arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    crate::chat_spawn::hide_console_window(&mut cmd);
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(err) => return Err(CliVersionFailure::Spawn(err.to_string())),
    };
    let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(CliVersionFailure::Spawn("no output pipes".to_string()));
    };
    let stdout_reader = crate::cli_probe_support::PipeCapture::spawn(stdout);
    let stderr_reader = crate::cli_probe_support::PipeCapture::spawn(stderr);
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let out = String::from_utf8_lossy(&stdout_reader.finish()).into_owned();
                let err = String::from_utf8_lossy(&stderr_reader.finish()).into_owned();
                if !status.success() {
                    return Err(CliVersionFailure::Exited {
                        status: crate::chat_spawn::exit_status_label(&status),
                        tail: crate::cli_probe_support::tail_snippet(&out, &err),
                    });
                }
                // A CLI that prints its version to stderr is still a
                // healthy one — the TS gate's `2>&1` accepted either.
                let version = if out.trim().is_empty() {
                    err.trim()
                } else {
                    out.trim()
                };
                return Ok(version.to_string());
            }
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                let grace = crate::cli_probe_support::CAPTURE_GRACE;
                let out =
                    String::from_utf8_lossy(&stdout_reader.finish_bounded(grace)).into_owned();
                let err =
                    String::from_utf8_lossy(&stderr_reader.finish_bounded(grace)).into_owned();
                return Err(CliVersionFailure::TimedOut {
                    seconds: timeout.as_secs(),
                    tail: crate::cli_probe_support::tail_snippet(&out, &err),
                });
            }
        }
    }
}

// ---------------------------------------------------------------
// Claude Code (connect-agent.ts:147-262)
// ---------------------------------------------------------------

fn connect_claude_code(locale: Locale) -> ProbeOutcome {
    if resolve_cli("claude").is_none() {
        return ProbeOutcome::not_installed(
            AgentProvider::ClaudeCode,
            tw(
                locale,
                "providerProbe.cliNotFound",
                &[("name", "Claude Code")],
            ),
        );
    }
    match claude_initialize_query() {
        ClaudeInitResult::Answered(models, account) => {
            let (info, hint) = build_claude_connection_info(locale, account.as_ref());
            connected_probe_outcome(
                locale,
                AgentProvider::ClaudeCode,
                models,
                Some(info),
                None,
                Some(hint),
                None,
            )
        }
        ClaudeInitResult::ExitedWithError(code) => ProbeOutcome::failed(friendly_claude_error(
            locale,
            &format!("process exited with code {code}"),
        )),
        ClaudeInitResult::NoAnswer => {
            ProbeOutcome::failed(no_models_error(locale, AgentProvider::ClaudeCode))
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
    // Process env first, then the captured login-shell env — a GUI
    // (Dock) launch misses shell-rc exports like ANTHROPIC_API_KEY,
    // which silently flips the transport to the subscription OAuth
    // credential (→ 403 Request not allowed on non-Claude-Code-shaped
    // requests). See `chat_spawn::login_shell_env`.
    if let Some(value) = crate::chat_spawn::env_var_with_login_shell(name) {
        return Some(value);
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
fn build_claude_connection_info(
    locale: Locale,
    account: Option<&ClaudeAccount>,
) -> (String, String) {
    let hint = config_path(
        "~/.claude/settings.json",
        "%USERPROFILE%\\.claude\\settings.json",
    );
    if let Some(email) = account.and_then(|a| a.email.as_deref()) {
        let sub = account
            .and_then(|a| a.subscription_type.as_deref())
            .unwrap_or("subscription");
        return (
            tw(
                locale,
                "providerProbe.connectedViaPlan",
                &[("plan", sub), ("email", email)],
            ),
            hint,
        );
    }
    let api_key = claude_env_var("ANTHROPIC_API_KEY");
    let base_url = claude_env_var("ANTHROPIC_BASE_URL");
    match (api_key, base_url) {
        (Some(_), Some(_)) => (
            t(locale, "providerProbe.connectedViaApiKeyCustomBase"),
            hint,
        ),
        (Some(key), None) => (
            tw(
                locale,
                "providerProbe.connectedViaApiKey",
                &[("key", &mask_key(&key))],
            ),
            hint,
        ),
        _ => (t(locale, "providerProbe.connectedViaSubscription"), hint),
    }
}

/// TS `friendlyClaudeError` (connect-agent.ts:357-371).
fn friendly_claude_error(locale: Locale, raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("process exited with code 1")
        || lower.contains("invalid model")
        || lower.contains("unknown model")
    {
        return t(locale, "providerProbe.claudeExitCode1");
    }
    if lower.contains("exited with code") {
        return t(locale, "providerProbe.claudeExitedUnexpectedly");
    }
    if lower.contains("not found") || lower.contains("enoent") {
        return t(locale, "providerProbe.claudeNotFoundInstall");
    }
    if lower.contains("timed out") || lower.contains("timedout") {
        return t(locale, "providerProbe.timedOut");
    }
    raw.to_string()
}

// ---------------------------------------------------------------
// Codex CLI (connect-agent.ts:416-576)
// ---------------------------------------------------------------

fn connect_codex_cli(locale: Locale) -> ProbeOutcome {
    let Some(exe) = resolve_cli("codex") else {
        return ProbeOutcome::not_installed(
            AgentProvider::CodexCli,
            tw(locale, "providerProbe.cliNotFound", &[("name", "Codex")]),
        );
    };
    // Responsiveness gate — TS runs `codex --version` with a 5 s
    // budget (connect-agent.ts:512-522).
    let version = match cli_version(&exe, Duration::from_secs(5)) {
        Ok(version) => version,
        Err(failure) => {
            return ProbeOutcome::failed(cli_version_failure_message("Codex", &failure))
        }
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
        return ProbeOutcome::failed(t(locale, "providerProbe.notAuthenticatedCodex"));
    }
    let (info, hint) = build_codex_connection_info(locale);
    connected_probe_outcome(
        locale,
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
fn build_codex_connection_info(locale: Locale) -> (String, String) {
    let hint = config_path("~/.codex/config.toml", "%USERPROFILE%\\.codex\\config.toml");
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        if !key.is_empty() {
            return (
                tw(
                    locale,
                    "providerProbe.connectedViaApiKey",
                    &[("key", &mask_key(&key))],
                ),
                hint,
            );
        }
    }
    let auth_raw = codex_home()
        .map(|home| home.join("auth.json"))
        .and_then(|path| std::fs::read_to_string(path).ok());
    (
        codex_connection_info_from_auth(locale, auth_raw.as_deref()),
        hint,
    )
}

/// Pure half of the Codex auth-info builder so the JWT/auth.json
/// parsing is unit-testable.
fn codex_connection_info_from_auth(locale: Locale, auth_raw: Option<&str>) -> String {
    let fallback = t(locale, "providerProbe.connectedViaCodexCli");
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
                return tw(
                    locale,
                    "providerProbe.connectedViaPlan",
                    &[("plan", label), ("email", email)],
                );
            }
        }
    }
    if let Some(mode) = auth_mode {
        return tw(locale, "providerProbe.connectedViaMode", &[("mode", mode)]);
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

fn connect_opencode(locale: Locale) -> ProbeOutcome {
    if resolve_cli("opencode").is_none() {
        return ProbeOutcome::not_installed(
            AgentProvider::OpenCode,
            tw(locale, "providerProbe.cliNotFound", &[("name", "OpenCode")]),
        );
    }
    // Model query via `opencode models` — Rust's sanctioned
    // transport divergence from the TS `opencode serve` SDK round
    // trip (the listing is equivalent: `provider/model` slugs).
    let models = discover_opencode();
    if models.is_empty() {
        return ProbeOutcome::failed(t(locale, "providerProbe.noModelsOpenCode"));
    }
    let info = opencode_provider_summary(locale, &models);
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
fn opencode_provider_summary(locale: Locale, models: &[ModelEntry]) -> String {
    let mut names: Vec<&str> = Vec::new();
    for m in models {
        if let Some((provider, _)) = m.value.split_once('/') {
            if !provider.is_empty() && !names.contains(&provider) {
                names.push(provider);
            }
        }
    }
    if names.is_empty() {
        return t(locale, "providerProbe.connectedViaOpenCodeServer");
    }
    let shown = names[..names.len().min(3)].join(", ");
    if names.len() > 3 {
        tw(
            locale,
            "providerProbe.connectedProvidersMore",
            &[
                ("providers", &shown),
                ("count", &(names.len() - 3).to_string()),
            ],
        )
    } else {
        tw(
            locale,
            "providerProbe.connectedProviders",
            &[("providers", &shown)],
        )
    }
}

#[cfg(test)]
#[path = "provider_probe_tests.rs"]
mod tests;
