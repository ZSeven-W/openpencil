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
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use op_ai::agent_settings_state::AgentProvider;
use op_ai::chat_models::ModelEntry;
use op_ai::chat_provider::CliName;

use crate::cli_probe_error::CliProbeError;
use crate::cli_probe_support::{bounded_cli_output, diagnose_timeout, BoundedProbe};
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

/// Resolve `name` through the same executable search used by live chat:
/// the current/login-shell PATH merge first, then standard package-manager
/// install directories. Keeping one resolver prevents Settings from checking
/// a different CLI than the transport later starts.
pub fn resolve_cli(name: &str) -> Option<PathBuf> {
    crate::chat_spawn::resolve_binary(name)
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
    codex_models_from_app_server_with_exe(&exe)
}

/// App-server discovery pinned to a previously resolved Codex executable.
/// The Settings connection gate uses this so its version check and model
/// handshake cannot silently select different installations.
pub(crate) fn codex_models_from_app_server_with_exe(exe: &Path) -> Option<Vec<ModelEntry>> {
    let mut cmd = crate::chat_spawn::build_blocking_command(exe, &["app-server"]);
    cmd.env_clear()
        .envs(crate::chat_subprocess_quirks::codex_child_env())
        .env("PATH", crate::chat_spawn::runtime_path_for_binary(exe))
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

/// GitHub Copilot CLI — query the live catalog through the official SDK.
/// The shared probe applies server-mode argv and delegates framing to the SDK.
fn discover_copilot() -> Vec<ModelEntry> {
    let Some(exe) = resolve_cli("copilot") else {
        return Vec::new();
    };
    crate::copilot_sdk_probe::probe_copilot_cli(&exe)
        .map(|probe| probe.models)
        .unwrap_or_default()
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

/// Model listing is a networked CLI operation and can pause for first-run
/// authentication. Keep startup/settings refresh bounded like the other CLI
/// catalogs rather than letting one OpenCode process block every provider.
const OPENCODE_MODEL_QUERY_TIMEOUT: Duration = Duration::from_secs(20);

/// OpenCode — `opencode models` prints one `provider/model` slug per line.
pub fn discover_opencode() -> Vec<ModelEntry> {
    query_opencode_models().unwrap_or_default()
}

pub(crate) fn query_opencode_models() -> Result<Vec<ModelEntry>, CliProbeError> {
    let exe = resolve_cli("opencode").ok_or(CliProbeError::NotFound {
        provider: "OpenCode",
    })?;
    opencode_models_from_exe(&exe, OPENCODE_MODEL_QUERY_TIMEOUT)
}

fn opencode_models_from_exe(
    exe: &std::path::Path,
    timeout: Duration,
) -> Result<Vec<ModelEntry>, CliProbeError> {
    let output = match bounded_cli_output(CliName::OpenCode, exe, &["models"], timeout) {
        BoundedProbe::Completed(output) => output,
        BoundedProbe::TimedOut { stdout, stderr } => {
            return Err(CliProbeError::Timeout(diagnose_timeout(
                CliName::OpenCode,
                "OpenCode",
                "`opencode`",
                timeout,
                &stdout,
                &stderr,
            )))
        }
        BoundedProbe::Failed => {
            return Err(CliProbeError::NotResponding {
                provider: "OpenCode",
            })
        }
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            CliProbeError::QueryFailed {
                provider: "OpenCode",
                login_command: Some("`opencode`"),
            }
        } else {
            CliProbeError::CliReported(stderr)
        });
    }
    let models = parse_opencode_models(&String::from_utf8_lossy(&output.stdout));
    if models.is_empty() {
        Err(CliProbeError::NoCatalog {
            provider: "OpenCode",
        })
    } else {
        Ok(models)
    }
}

fn parse_opencode_models(stdout: &str) -> Vec<ModelEntry> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|slug| valid_opencode_model_slug(slug))
        .map(|slug| ModelEntry::new(AgentProvider::OpenCode, slug, slug))
        .collect()
}

fn valid_opencode_model_slug(slug: &str) -> bool {
    let Some((provider, model)) = slug.split_once('/') else {
        return false;
    };
    !provider.is_empty()
        && !model.is_empty()
        && slug.len() <= 256
        && slug
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/' | ':' | '@'))
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
    fn opencode_catalog_accepts_only_bounded_provider_model_slugs() {
        let oversized = format!("provider/{}", "x".repeat(300));
        let raw = format!(
            "anthropic/claude-sonnet-4-6\nopenrouter/meta/llama-3\nwarning: sign in\n/no-provider\nprovider/\n{oversized}\n"
        );
        let values: Vec<_> = parse_opencode_models(&raw)
            .into_iter()
            .map(|model| model.value)
            .collect();
        assert_eq!(
            values,
            ["anthropic/claude-sonnet-4-6", "openrouter/meta/llama-3"]
        );
    }

    #[cfg(unix)]
    #[test]
    fn opencode_catalog_query_kills_hung_process_tree_at_deadline() {
        use std::os::unix::fs::PermissionsExt;

        let script = std::env::temp_dir().join(format!(
            "openpencil-opencode-models-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::write(
            &script,
            "#!/bin/sh\nprintf 'anthropic/claude-test\\n'\nsleep 30\n",
        )
        .expect("write fake opencode");
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("chmod");

        let started = Instant::now();
        let result = opencode_models_from_exe(&script, Duration::from_millis(100));
        let elapsed = started.elapsed();
        let _ = std::fs::remove_file(&script);

        assert!(
            matches!(result, Err(CliProbeError::Timeout(_))),
            "{result:?}"
        );
        assert!(
            elapsed < Duration::from_secs(2),
            "hung wrapper/descendant escaped the deadline: {elapsed:?}"
        );
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
}
