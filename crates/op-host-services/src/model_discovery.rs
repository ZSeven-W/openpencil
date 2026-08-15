//! Cross-platform AI-model discovery.
//!
//! The chat panel's model picker lists whatever models the
//! *locally installed* CLIs actually expose. shell-core is
//! transport-free, so this desktop module does the probing and
//! writes the result into `Document.chat.available_models`.
//!
//! Each provider degrades independently — a missing CLI, absent
//! cache file or failed subprocess yields *that* provider's empty
//! contribution and never aborts the others. Where a CLI offers a
//! real listing interface we use it; where it offers none we fall
//! back to its documented model names, gated on the CLI actually
//! being installed so the picker never advertises a provider the
//! user doesn't have.
//!
//! Works on macOS / Linux / Windows: `resolve_cli` walks `PATH`
//! with the platform's executable extensions, and cache paths go
//! through `dirs::home_dir`.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use op_ai::agent_settings_state::AgentProvider;
use op_ai::chat_models::ModelEntry;

pub use crate::model_probe::ModelProbe;

/// Translate a shell-core `ModelEntry` into op-editor-core's.
pub fn model_entry_to_ec(m: ModelEntry) -> op_editor_core::ModelEntry {
    use op_ai::agent_settings_state::AgentProvider as ScP;
    let provider = match m.provider {
        ScP::ClaudeCode => op_editor_core::AgentProvider::ClaudeCode,
        ScP::CodexCli => op_editor_core::AgentProvider::CodexCli,
        ScP::OpenCode => op_editor_core::AgentProvider::OpenCode,
        ScP::GithubCopilot => op_editor_core::AgentProvider::GithubCopilot,
        ScP::Antigravity => op_editor_core::AgentProvider::Antigravity,
        ScP::GrokBuild => op_editor_core::AgentProvider::GrokBuild,
        ScP::DeepSeekHarness => op_editor_core::AgentProvider::DeepSeekHarness,
    };
    op_editor_core::ModelEntry::new(provider, m.value, m.display_name)
}

/// Probe every CLI we know how to query and return a flat,
/// provider-grouped model list. Safe to call off the UI thread —
/// it only reads files and spawns short-lived subprocesses.
pub fn discover_models() -> Vec<ModelEntry> {
    discover_models_for_connected([true; 7])
}

/// Discover a startup-selected provider set concurrently while preserving
/// the stable Settings/model-picker provider order in the returned catalog.
pub fn discover_models_for_connected(connected: [bool; 7]) -> Vec<ModelEntry> {
    let workers: Vec<_> = discovery_provider_order()
        .into_iter()
        .enumerate()
        .filter(|(index, _)| connected[*index])
        // Joined below — these workers are fanned out, not detached, so their
        // deadline-bounded probes all complete before the catalog is returned.
        .map(|(_, provider)| std::thread::spawn(move || discover_provider(provider)))
        .collect();
    workers
        .into_iter()
        .flat_map(|worker| worker.join().unwrap_or_default())
        .collect()
}

fn discover_provider(provider: AgentProvider) -> Vec<ModelEntry> {
    match provider {
        AgentProvider::ClaudeCode => discover_claude(),
        AgentProvider::CodexCli => discover_codex(),
        AgentProvider::OpenCode => discover_opencode(),
        AgentProvider::GithubCopilot => discover_copilot(),
        AgentProvider::Antigravity => crate::cli_model_discovery::discover_antigravity(),
        AgentProvider::GrokBuild => crate::cli_model_discovery::discover_grok(),
        AgentProvider::DeepSeekHarness => crate::cli_model_discovery::discover_deepseek_harness(),
    }
}

/// Provider probe order mirrors TS `DEFAULT_PROVIDERS`, which is
/// also the core `AgentProvider::ALL` order used by Settings.
fn discovery_provider_order() -> [AgentProvider; 7] {
    AgentProvider::ALL
}

/// Resolve `name` to an executable on `PATH`. On Windows this also
/// tries the `.exe` / `.cmd` / `.bat` suffixes so npm-installed CLI
/// shims resolve. Returns `None` when the CLI is not installed.
///
/// When the `PATH` walk misses (a Finder/dock launch inherits a
/// minimal PATH), the standard user-local install directories are
/// scanned next — the TS resolvers' `posixUserBinDirs()` candidate
/// list (`cli-resolver-helpers.ts:29-72`). The TS login-shell probe
/// (`probeViaLoginShell`) is intentionally NOT ported: spawning the
/// user's interactive shell from the GUI process is slow and
/// side-effectful.
pub fn resolve_cli(name: &str) -> Option<PathBuf> {
    let exts: &[&str] = if cfg!(windows) {
        &[".exe", ".cmd", ".bat", ""]
    } else {
        &[""]
    };
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            for ext in exts {
                let candidate = dir.join(format!("{name}{ext}"));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    for dir in user_bin_dirs() {
        for ext in exts {
            let candidate = dir.join(format!("{name}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Standard user-local / package-manager bin directories — the TS
/// `posixUserBinDirs()` list plus the opencode/npm-global extras the
/// per-CLI TS resolvers add, applied uniformly (a superset never
/// hides an install). Windows also covers npm-global plus the official
/// Antigravity and Grok Build installer directories.
fn user_bin_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if cfg!(windows) {
        return crate::cli_resolver_windows::user_bin_dirs();
    }
    let Some(home) = dirs::home_dir() else {
        return dirs;
    };
    // Antigravity's official Unix installer (`curl -fsSL
    // https://antigravity.google/cli/install.sh | bash`) writes `agy` to
    // `$HOME/.local/bin` by default (`TARGET_DIR="$HOME/.local/bin"` in
    // that script), mirroring the explicit `%LOCALAPPDATA%\agy\bin` entry
    // the Windows resolver carries (`cli_resolver_windows.rs`). Listed
    // first and by name — not left to coincidentally match the generic
    // dev-tool list below — so this candidate keeps resolving even if
    // that list's contents change. `.local/bin` is intentionally absent
    // from the loop below to avoid probing it twice.
    dirs.push(home.join(".local/bin"));
    for rel in [
        ".bun/bin",
        ".volta/bin",
        ".local/share/mise/shims",
        ".asdf/shims",
        "Library/pnpm",
        ".pnpm-global/bin",
        ".cargo/bin",
        ".opencode/bin",
        ".npm-global/bin",
    ] {
        dirs.push(home.join(rel));
    }
    dirs.push(PathBuf::from("/usr/local/bin"));
    dirs.push(PathBuf::from("/opt/homebrew/bin"));
    // nvm / fnm: enumerate installed node versions best-effort.
    if let Ok(rd) = std::fs::read_dir(home.join(".nvm/versions/node")) {
        for entry in rd.flatten() {
            dirs.push(entry.path().join("bin"));
        }
    }
    if let Ok(rd) = std::fs::read_dir(home.join(".fnm/node-versions")) {
        for entry in rd.flatten() {
            dirs.push(entry.path().join("installation/bin"));
        }
    }
    dirs
}

/// Claude Code — live `initialize` control-request query over the
/// CLI's stream-json stdio (the same wire the TS Agent SDK's
/// `supportedModels()` awaits, connect-agent.ts:147-209), falling
/// back to the TS `FALLBACK_CLAUDE_MODELS` list when the CLI is
/// installed but didn't answer (proxy/base-URL setups,
/// connect-agent.ts:216-258).
fn discover_claude() -> Vec<ModelEntry> {
    if resolve_cli("claude").is_none() {
        return Vec::new();
    }
    if let crate::provider_probe_models::ClaudeInitResult::Answered(models, _) =
        crate::provider_probe_models::claude_initialize_query()
    {
        if !models.is_empty() {
            return models;
        }
    }
    crate::provider_probe_models::fallback_claude_models()
}

/// How long to wait for `codex app-server` to answer `model/list`.
/// The server refreshes its model list on demand (a few seconds),
/// so the budget is generous; discovery runs off the UI thread.
const CODEX_APP_SERVER_TIMEOUT: Duration = Duration::from_secs(12);

/// Codex CLI — query models via the official App Server protocol
/// first (most accurate + stable), then the on-disk cache, then the
/// bundled `latest-model.md` reference (the TS fresh-install
/// fallback, connect-agent.ts:554-562), then a minimal placeholder
/// so the provider is still selectable when `codex` is installed
/// but no source answered.
fn discover_codex() -> Vec<ModelEntry> {
    if let Some(models) = codex_models_from_app_server() {
        if !models.is_empty() {
            return models;
        }
    }
    if let Some(models) = codex_models_from_cache() {
        if !models.is_empty() {
            return models;
        }
    }
    if let Some(home) = crate::provider_probe_models::codex_home() {
        let models = crate::provider_probe_models::codex_models_from_latest_md(&home);
        if !models.is_empty() {
            return models;
        }
    }
    if resolve_cli("codex").is_some() {
        return vec![ModelEntry::new(
            AgentProvider::CodexCli,
            "gpt-5.5",
            "GPT-5.5",
        )];
    }
    Vec::new()
}

/// Query Codex models through the official App Server protocol —
/// spawn `codex app-server` and drive JSON-RPC over stdio:
/// `initialize` → `initialized` → `model/list`. Returns `None` on
/// any failure (CLI missing, spawn error, timeout) so the caller
/// falls back to the on-disk cache.
pub fn codex_models_from_app_server() -> Option<Vec<ModelEntry>> {
    let exe = resolve_cli("codex")?;
    let mut cmd = Command::new(exe);
    cmd.arg("app-server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    crate::chat_spawn::hide_console_window(&mut cmd);
    let mut child = cmd.spawn().ok()?;
    let mut stdin = child.stdin.take()?;
    let stdout = child.stdout.take()?;

    // Child has no timed read — a reader thread funnels stdout
    // lines through a channel so the main path can apply a
    // deadline and walk away from a hung server.
    //
    // Detached on purpose: a thread parked in a blocking pipe read cannot be
    // signalled, so its ONLY shutdown path is the `child.kill()` below, which
    // closes the child's stdout and ends the iterator. Every exit from here
    // therefore has to reach that kill — see the `wrote` guard.
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    // `initialize` (id 1) → `initialized` notification →
    // `model/list` (id 2). stdin is held open until the response
    // lands so the server doesn't shut down mid-refresh.
    let init = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"clientInfo":{{"name":"openpencil","version":"{}"}},"capabilities":{{"experimentalApi":true}}}}}}"#,
        env!("CARGO_PKG_VERSION")
    );
    let initialized = r#"{"jsonrpc":"2.0","method":"initialized"}"#;
    let list = r#"{"jsonrpc":"2.0","id":2,"method":"model/list","params":{}}"#;
    // A failed handshake write must NOT `?` out of the function: that would
    // leave both the child process and its blocked reader thread alive for
    // the rest of the session. Fall through to the kill instead.
    let wrote = writeln!(stdin, "{init}")
        .and_then(|_| writeln!(stdin, "{initialized}"))
        .and_then(|_| writeln!(stdin, "{list}"))
        .and_then(|_| stdin.flush())
        .is_ok();

    let mut models = None;
    if wrote {
        let deadline = Instant::now() + CODEX_APP_SERVER_TIMEOUT;
        while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
            match rx.recv_timeout(remaining) {
                Ok(line) => {
                    if let Some(parsed) = parse_codex_model_list(&line) {
                        models = Some(parsed);
                        break;
                    }
                }
                // Timeout or the reader thread closed — give up.
                Err(_) => break,
            }
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    models
}

/// Parse one JSON-RPC line from `codex app-server`; yields the
/// model list only for the `id:2` (`model/list`) response, `None`
/// for the `initialize` reply / interleaved notifications.
fn parse_codex_model_list(line: &str) -> Option<Vec<ModelEntry>> {
    let json: serde_json::Value = serde_json::from_str(extract_json_object(line)?).ok()?;
    if json.get("id")?.as_i64()? != 2 {
        return None;
    }
    let data = json.get("result")?.get("data")?.as_array()?;
    Some(
        data.iter()
            .filter_map(|m| {
                // `hidden` is the app-server's twin of the cache's
                // `visibility` (which `parse_codex_models_cache` filters
                // on): entries the CLI lists for internal use only —
                // `codex-auto-review` and friends. Today the server
                // already withholds them from `model/list`, so this
                // filter is a no-op against the current build; it exists
                // so a server that starts sending them cannot leak an
                // unusable model into the picker, and so both parse paths
                // apply the same rule.
                if m.get("hidden").and_then(serde_json::Value::as_bool) == Some(true) {
                    return None;
                }
                let value = m.get("model").or_else(|| m.get("id"))?.as_str()?;
                let name = m
                    .get("displayName")
                    .and_then(|v| v.as_str())
                    .unwrap_or(value);
                Some(ModelEntry::new(AgentProvider::CodexCli, value, name))
            })
            .collect(),
    )
}

pub fn codex_models_from_cache() -> Option<Vec<ModelEntry>> {
    let path = crate::provider_probe_models::codex_home()?.join("models_cache.json");
    let raw = std::fs::read_to_string(path).ok()?;
    parse_codex_models_cache(&raw)
}

/// Parse Codex's `models_cache.json` — listed models only
/// (`visibility === "list"`), sorted by ascending `priority` with
/// missing priorities sinking to 999. The TS cache mapping
/// (connect-agent.ts:528-549).
pub fn parse_codex_models_cache(raw: &str) -> Option<Vec<ModelEntry>> {
    let json: serde_json::Value = serde_json::from_str(raw).ok()?;
    let arr = json.get("models")?.as_array()?;
    let mut keyed: Vec<(i64, ModelEntry)> = arr
        .iter()
        .filter_map(|m| {
            if m.get("visibility").and_then(|v| v.as_str()) != Some("list") {
                return None;
            }
            let slug = m.get("slug")?.as_str()?;
            let name = m
                .get("display_name")
                .and_then(|v| v.as_str())
                .unwrap_or(slug);
            let priority = m.get("priority").and_then(|v| v.as_i64()).unwrap_or(999);
            Some((
                priority,
                ModelEntry::new(AgentProvider::CodexCli, slug, name),
            ))
        })
        .collect();
    // Stable sort keeps the cache order within equal priorities,
    // like the TS Array.sort.
    keyed.sort_by_key(|(priority, _)| *priority);
    Some(keyed.into_iter().map(|(_, model)| model).collect())
}

/// How long to wait for `copilot --stdio` to answer `models.list`.
const COPILOT_STDIO_TIMEOUT: Duration = Duration::from_secs(8);

/// GitHub Copilot CLI — query models via the official CLI JSON-RPC
/// protocol (`connect` → `models.list`), then a documented-name
/// fallback when the CLI is installed but the query didn't answer.
fn discover_copilot() -> Vec<ModelEntry> {
    if let Some(models) = copilot_models_from_stdio() {
        if !models.is_empty() {
            return models;
        }
    }
    if resolve_cli("copilot").is_none() {
        return Vec::new();
    }
    [
        ("gpt-5", "GPT-5"),
        ("claude-sonnet-4.5", "Claude Sonnet 4.5"),
    ]
    .iter()
    .map(|(value, name)| ModelEntry::new(AgentProvider::GithubCopilot, *value, *name))
    .collect()
}

/// Query Copilot models through the official CLI JSON-RPC
/// protocol — the same `connect` → `models.list` wire calls the
/// `github-copilot-sdk` makes (protocol version 3), spoken
/// directly over `copilot --stdio`. Talking the wire protocol
/// rather than linking the SDK crate keeps the dep set + the
/// workspace's Rust 1.85 toolchain unchanged (the SDK crate needs
/// Rust 1.94). Returns `None` on any failure.
fn copilot_models_from_stdio() -> Option<Vec<ModelEntry>> {
    let exe = resolve_cli("copilot")?;
    let mut cmd = Command::new(exe);
    cmd.arg("--stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    crate::chat_spawn::hide_console_window(&mut cmd);
    let mut child = cmd.spawn().ok()?;
    let mut stdin = child.stdin.take()?;
    let stdout = child.stdout.take()?;

    // Detached reader thread; same contract as `codex_models_from_app_server`
    // — a blocking pipe read is uninterruptible, so `child.kill()` closing the
    // child's stdout is its only shutdown signal and every exit path below
    // must reach it.
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    // `connect` (id 1) establishes the session; `models.list`
    // (id 2) needs no params when the server runs without a
    // connection token (we spawned it ourselves). Both go out as
    // `Content-Length`-framed frames — the wire framing the
    // official copilot-sdk writes on `copilot --stdio`.
    let connect = r#"{"jsonrpc":"2.0","id":1,"method":"connect","params":{}}"#;
    let list = r#"{"jsonrpc":"2.0","id":2,"method":"models.list","params":{}}"#;
    // Never `?` out on a write failure — see the codex path: it would strand
    // the child process plus its blocked reader thread.
    let wrote = write_lsp_frame(&mut stdin, connect)
        .and_then(|_| write_lsp_frame(&mut stdin, list))
        .and_then(|_| stdin.flush())
        .is_ok();

    let mut models = None;
    if wrote {
        let deadline = Instant::now() + COPILOT_STDIO_TIMEOUT;
        while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
            match rx.recv_timeout(remaining) {
                Ok(line) => {
                    if let Some(parsed) = parse_copilot_model_list(&line) {
                        models = Some(parsed);
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    models
}

/// Write one `Content-Length`-framed JSON-RPC message — the wire
/// framing the official copilot-sdk writes on `copilot --stdio`
/// (`Content-Length: <byte len>\r\n\r\n<body>`, no trailing
/// newline). `body` is ASCII JSON, so `str::len` is the byte
/// count the header must report.
pub fn write_lsp_frame(w: &mut impl Write, body: &str) -> std::io::Result<()> {
    write!(w, "Content-Length: {}\r\n\r\n{}", body.len(), body)
}

/// Extract the first complete top-level JSON object from `s` —
/// the slice from the first `{` to its matching `}`, brace-counted
/// with string/escape awareness. This lets the response parsers
/// cope with a `Content-Length:` header prefix or a glued
/// next-frame header suffix on the same line, so discovery is
/// robust whether the CLI replies newline-delimited or
/// Content-Length-framed.
pub fn extract_json_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escaped = false;
    for i in start..bytes.len() {
        let c = bytes[i];
        if in_str {
            match c {
                _ if escaped => escaped = false,
                b'\\' => escaped = true,
                b'"' => in_str = false,
                _ => {}
            }
        } else {
            match c {
                b'"' => in_str = true,
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(&s[start..=i]);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

/// Parse one JSON-RPC line from `copilot --stdio`; yields the
/// model list only for the `id:2` (`models.list`) response.
/// Policy-disabled models are dropped — the TS route's
/// `!m.policy || m.policy.state === 'enabled'` filter
/// (connect-agent.ts:779-786).
pub fn parse_copilot_model_list(line: &str) -> Option<Vec<ModelEntry>> {
    let json: serde_json::Value = serde_json::from_str(extract_json_object(line)?).ok()?;
    if json.get("id")?.as_i64()? != 2 {
        return None;
    }
    let models = json.get("result")?.get("models")?.as_array()?;
    Some(
        models
            .iter()
            .filter_map(|m| {
                // TS: `!m.policy || m.policy.state === 'enabled'` —
                // a model carrying a policy object is kept only when
                // that policy's state is exactly "enabled".
                if let Some(policy) = m.get("policy").filter(|p| !p.is_null()) {
                    if policy.get("state").and_then(|s| s.as_str()) != Some("enabled") {
                        return None;
                    }
                }
                let id = m.get("id")?.as_str()?;
                let name = m.get("name").and_then(|v| v.as_str()).unwrap_or(id);
                Some(ModelEntry::new(AgentProvider::GithubCopilot, id, name))
            })
            .collect(),
    )
}

/// OpenCode — `opencode models` prints one `provider/model` slug
/// per line. A real query: parse stdout. Empty when the CLI is
/// missing or the command fails.
pub fn discover_opencode() -> Vec<ModelEntry> {
    let Some(exe) = resolve_cli("opencode") else {
        return Vec::new();
    };
    let mut cmd = Command::new(exe);
    cmd.arg("models");
    crate::chat_spawn::hide_console_window(&mut cmd);
    let Ok(output) = cmd.output() else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|slug| ModelEntry::new(AgentProvider::OpenCode, slug, slug))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_cache_parser_filters_visibility_and_sorts_by_priority() {
        // Mirrors the real `models_cache.json` shape and the TS
        // mapping (connect-agent.ts:528-549): `visibility === "list"`
        // only, ascending priority, missing priority sinks to 999,
        // missing display_name falls back to the slug, and an entry
        // without a slug is skipped without aborting the rest.
        let parsed = parse_codex_models_cache(
            r#"{"models":[
                {"slug":"gpt-5.5-codex","display_name":"GPT-5.5 Codex","visibility":"list","priority":2},
                {"slug":"gpt-internal","display_name":"Hidden","visibility":"hidden","priority":0},
                {"slug":"gpt-no-visibility","display_name":"No visibility"},
                {"display_name":"no slug — skipped","visibility":"list"},
                {"slug":"gpt-5.5","display_name":"GPT-5.5","visibility":"list","priority":1},
                {"slug":"gpt-unprioritized","visibility":"list"}
            ]}"#,
        )
        .expect("cache parses");
        let values: Vec<&str> = parsed.iter().map(|m| m.value.as_str()).collect();
        assert_eq!(values, ["gpt-5.5", "gpt-5.5-codex", "gpt-unprioritized"]);
        assert_eq!(parsed[0].display_name, "GPT-5.5");
        // Missing display_name falls back to the slug.
        assert_eq!(parsed[2].display_name, "gpt-unprioritized");
    }

    #[test]
    fn app_server_response_parser_picks_id2_and_skips_others() {
        // initialize reply (id 1) — not the model list.
        assert!(parse_codex_model_list(r#"{"id":1,"result":{"codexHome":"x"}}"#).is_none());
        // interleaved notification — no id.
        assert!(parse_codex_model_list(r#"{"method":"remoteControl/status/changed"}"#).is_none());
        // the model/list response (id 2).
        let models = parse_codex_model_list(
            r#"{"id":2,"result":{"data":[
                {"id":"gpt-5.5","model":"gpt-5.5","displayName":"GPT-5.5"},
                {"id":"gpt-5.4","model":"gpt-5.4"}
            ]}}"#,
        )
        .expect("id:2 parses");
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].display_name, "GPT-5.5");
        // Missing displayName falls back to the model value.
        assert_eq!(models[1].display_name, "gpt-5.4");
    }

    #[test]
    fn app_server_parser_drops_hidden_models_like_the_cache_parser() {
        // Same rule the cache parser applies via `visibility` — an
        // internal-use entry must never reach the picker.
        let models = parse_codex_model_list(
            r#"{"id":2,"result":{"data":[
                {"id":"gpt-5.6-sol","model":"gpt-5.6-sol","displayName":"GPT-5.6-Sol","hidden":false},
                {"id":"codex-auto-review","model":"codex-auto-review","displayName":"Codex Auto Review","hidden":true},
                {"id":"gpt-5.5","model":"gpt-5.5","displayName":"GPT-5.5"}
            ]}}"#,
        )
        .expect("id:2 parses");
        let values: Vec<&str> = models.iter().map(|m| m.value.as_str()).collect();
        assert_eq!(values, ["gpt-5.6-sol", "gpt-5.5"]);
    }

    #[test]
    fn copilot_response_parser_picks_id2_model_list() {
        // connect reply (id 1) — not the model list.
        assert!(parse_copilot_model_list(
            r#"{"jsonrpc":"2.0","id":1,"result":{"connected":true}}"#
        )
        .is_none());
        let models = parse_copilot_model_list(
            r#"{"jsonrpc":"2.0","id":2,"result":{"models":[
                {"id":"gpt-5-mini","name":"GPT-5 mini"},
                {"id":"claude-haiku-4.5"}
            ]}}"#,
        )
        .expect("id:2 parses");
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].display_name, "GPT-5 mini");
        // Missing name falls back to the id.
        assert_eq!(models[1].display_name, "claude-haiku-4.5");
    }

    #[test]
    fn copilot_parser_policy_filter_matches_ts() {
        // TS keeps `!m.policy || m.policy.state === 'enabled'`: no
        // policy → kept, null policy → kept, enabled → kept, any
        // other state (or a policy missing `state`) → dropped.
        let models = parse_copilot_model_list(
            r#"{"jsonrpc":"2.0","id":2,"result":{"models":[
                {"id":"no-policy"},
                {"id":"null-policy","policy":null},
                {"id":"enabled","policy":{"state":"enabled"}},
                {"id":"disabled","policy":{"state":"disabled"}},
                {"id":"stateless-policy","policy":{}}
            ]}}"#,
        )
        .expect("id:2 parses");
        let values: Vec<&str> = models.iter().map(|m| m.value.as_str()).collect();
        assert_eq!(values, ["no-policy", "null-policy", "enabled"]);
    }

    #[test]
    fn extract_json_object_handles_framing_prefix_and_suffix() {
        // Plain object.
        assert_eq!(extract_json_object(r#"{"a":1}"#), Some(r#"{"a":1}"#));
        // Content-Length header prefix + glued next-frame header suffix.
        let framed = "Content-Length: 7\r\n\r\n{\"a\":1}Content-Length: 9\r\n\r\n";
        assert_eq!(extract_json_object(framed), Some(r#"{"a":1}"#));
        // Nested braces + a brace inside a string must not end early.
        let nested = r#"prefix {"x":{"y":2},"s":"a}b{c"} trailing"#;
        assert_eq!(
            extract_json_object(nested),
            Some(r#"{"x":{"y":2},"s":"a}b{c"}"#)
        );
        // No object at all.
        assert_eq!(extract_json_object("Content-Length: 0\r\n"), None);
    }

    #[test]
    fn copilot_parser_tolerates_content_length_framed_line() {
        // A line carrying the CL header prefix + a glued next header
        // still yields the model list.
        let line = "Content-Length: 80\r\n\r\n{\"id\":2,\"result\":{\"models\":[{\"id\":\"gpt-5\",\"name\":\"GPT-5\"}]}}Content-Length: 5\r\n";
        let models = parse_copilot_model_list(line).expect("framed line parses");
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].display_name, "GPT-5");
    }

    #[test]
    fn discovery_order_matches_ts_default_provider_order() {
        assert_eq!(discovery_provider_order(), AgentProvider::ALL);
    }

    #[test]
    fn model_probe_reports_pending_state() {
        let idle = ModelProbe::idle();
        assert!(!idle.is_pending());

        let (pending, _tx) = ModelProbe::pending_for_test();
        assert!(pending.is_pending());
    }

    #[test]
    fn discover_models_never_panics() {
        // Whatever is or isn't installed on the test machine, the
        // probe must return cleanly.
        let _ = discover_models();
    }

    #[cfg(unix)]
    #[test]
    fn posix_candidates_list_antigravity_dir_exactly_once() {
        // `.local/bin` is both the generic dev-tool candidate and
        // Antigravity's official installer target; it must appear once,
        // not be probed twice.
        let dirs = user_bin_dirs();
        let Some(home) = dirs::home_dir() else {
            return;
        };
        let antigravity_dir = home.join(".local/bin");
        assert_eq!(
            dirs.iter().filter(|d| **d == antigravity_dir).count(),
            1,
            "expected exactly one .local/bin candidate, got {dirs:?}"
        );
        assert_eq!(
            dirs.first(),
            Some(&antigravity_dir),
            "Antigravity's official dir should be checked first"
        );
    }
}
