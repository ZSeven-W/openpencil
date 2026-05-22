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
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use op_ai::agent_settings_state::AgentProvider;
use op_ai::chat_models::ModelEntry;

/// Background model-discovery probe. [`discover_models`] reads a
/// cache file and spawns a subprocess (`opencode models`, ~1 s),
/// so it must not block the event loop — the probe runs it on a
/// worker thread and the host drains the result on a later frame.
pub struct ModelProbe {
    rx: Option<Receiver<Vec<ModelEntry>>>,
}

impl ModelProbe {
    /// Spawn the discovery worker. Returns immediately.
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(discover_models());
        });
        Self { rx: Some(rx) }
    }

    /// If discovery has finished, move its models into the host's
    /// `EditorState.chat` and return `true`. Idempotent — the
    /// receiver is dropped after the first drain so later calls are
    /// cheap no-ops.
    pub fn poll_into(&mut self, host: &mut op_host_native::WidgetHostNative) -> bool {
        let Some(rx) = self.rx.as_ref() else {
            return false;
        };
        match rx.try_recv() {
            Ok(models) => {
                // The discovery worker emits shell-core `ModelEntry`s;
                // translate each into the op-editor-core type the
                // host's `EditorState.chat` carries. The translated
                // list is the *full* catalog (every installed CLI);
                // `rebuild_chat_models` then narrows it to the
                // providers the user has connected in Settings.
                let es = host.editor_state_mut();
                es.chat.discovered_models = models.into_iter().map(model_entry_to_ec).collect();
                es.rebuild_chat_models();
                host.mark_editor_state_dirty();
                self.rx = None;
                true
            }
            Err(TryRecvError::Empty) => false,
            Err(TryRecvError::Disconnected) => {
                self.rx = None;
                false
            }
        }
    }
}

/// Translate a shell-core `ModelEntry` into op-editor-core's.
fn model_entry_to_ec(m: ModelEntry) -> op_editor_core::ModelEntry {
    use op_ai::agent_settings_state::AgentProvider as ScP;
    let provider = match m.provider {
        ScP::ClaudeCode => op_editor_core::AgentProvider::ClaudeCode,
        ScP::CodexCli => op_editor_core::AgentProvider::CodexCli,
        ScP::OpenCode => op_editor_core::AgentProvider::OpenCode,
        ScP::GithubCopilot => op_editor_core::AgentProvider::GithubCopilot,
        ScP::GeminiCli => op_editor_core::AgentProvider::GeminiCli,
    };
    op_editor_core::ModelEntry::new(provider, m.value, m.display_name)
}

/// Probe every CLI we know how to query and return a flat,
/// provider-grouped model list. Safe to call off the UI thread —
/// it only reads files and spawns short-lived subprocesses.
pub fn discover_models() -> Vec<ModelEntry> {
    let mut out = Vec::new();
    out.extend(discover_claude());
    out.extend(discover_codex());
    out.extend(discover_gemini());
    out.extend(discover_copilot());
    out.extend(discover_opencode());
    out
}

/// Resolve `name` to an executable on `PATH`. On Windows this also
/// tries the `.exe` / `.cmd` / `.bat` suffixes so npm-installed CLI
/// shims resolve. Returns `None` when the CLI is not installed.
fn resolve_cli(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let exts: &[&str] = if cfg!(windows) {
        &["", ".exe", ".cmd", ".bat"]
    } else {
        &[""]
    };
    for dir in std::env::split_paths(&path) {
        for ext in exts {
            let candidate = dir.join(format!("{name}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Claude Code — the `claude` CLI exposes no model-listing command;
/// `--model` takes documented aliases that always resolve to the
/// current latest model. We surface those aliases (better than a
/// dated model-id list) when the CLI is installed.
fn discover_claude() -> Vec<ModelEntry> {
    if resolve_cli("claude").is_none() {
        return Vec::new();
    }
    [
        ("default", "Default"),
        ("sonnet", "Sonnet"),
        ("opus", "Opus"),
        ("haiku", "Haiku"),
    ]
    .iter()
    .map(|(value, name)| ModelEntry::new(AgentProvider::ClaudeCode, *value, *name))
    .collect()
}

/// How long to wait for `codex app-server` to answer `model/list`.
/// The server refreshes its model list on demand (a few seconds),
/// so the budget is generous; discovery runs off the UI thread.
const CODEX_APP_SERVER_TIMEOUT: Duration = Duration::from_secs(12);

/// Codex CLI — query models via the official App Server protocol
/// first (most accurate + stable), then the on-disk cache, then a
/// minimal placeholder so the provider is still selectable when
/// `codex` is installed but neither source answered.
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
fn codex_models_from_app_server() -> Option<Vec<ModelEntry>> {
    let exe = resolve_cli("codex")?;
    let mut child = Command::new(exe)
        .arg("app-server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdin = child.stdin.take()?;
    let stdout = child.stdout.take()?;

    // Child has no timed read — a reader thread funnels stdout
    // lines through a channel so the main path can apply a
    // deadline and walk away from a hung server.
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
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"openpencil","version":"0.8.0"},"capabilities":{"experimentalApi":true}}}"#;
    let initialized = r#"{"jsonrpc":"2.0","method":"initialized"}"#;
    let list = r#"{"jsonrpc":"2.0","id":2,"method":"model/list","params":{}}"#;
    writeln!(stdin, "{init}").ok()?;
    writeln!(stdin, "{initialized}").ok()?;
    writeln!(stdin, "{list}").ok()?;
    stdin.flush().ok();

    let deadline = Instant::now() + CODEX_APP_SERVER_TIMEOUT;
    let mut models = None;
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

fn codex_models_from_cache() -> Option<Vec<ModelEntry>> {
    let path = dirs::home_dir()?.join(".codex").join("models_cache.json");
    let raw = std::fs::read_to_string(path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let arr = json.get("models")?.as_array()?;
    Some(
        arr.iter()
            .filter_map(|m| {
                let slug = m.get("slug")?.as_str()?;
                let name = m
                    .get("display_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(slug);
                Some(ModelEntry::new(AgentProvider::CodexCli, slug, name))
            })
            .collect(),
    )
}

/// Gemini CLI — the `gemini` CLI exposes only `-m/--model`, no
/// listing command, so we surface its documented model names when
/// the CLI is installed.
fn discover_gemini() -> Vec<ModelEntry> {
    if resolve_cli("gemini").is_none() {
        return Vec::new();
    }
    [
        ("gemini-2.5-pro", "Gemini 2.5 Pro"),
        ("gemini-2.5-flash", "Gemini 2.5 Flash"),
    ]
    .iter()
    .map(|(value, name)| ModelEntry::new(AgentProvider::GeminiCli, *value, *name))
    .collect()
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

    // `connect` (id 1) establishes the session; `models.list`
    // (id 2) needs no params when the server runs without a
    // connection token (we spawned it ourselves). Both go out as
    // `Content-Length`-framed frames — the wire framing the
    // official copilot-sdk writes on `copilot --stdio`.
    let connect = r#"{"jsonrpc":"2.0","id":1,"method":"connect","params":{}}"#;
    let list = r#"{"jsonrpc":"2.0","id":2,"method":"models.list","params":{}}"#;
    write_lsp_frame(&mut stdin, connect).ok()?;
    write_lsp_frame(&mut stdin, list).ok()?;
    stdin.flush().ok();

    let deadline = Instant::now() + COPILOT_STDIO_TIMEOUT;
    let mut models = None;
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
    let _ = child.kill();
    let _ = child.wait();
    models
}

/// Write one `Content-Length`-framed JSON-RPC message — the wire
/// framing the official copilot-sdk writes on `copilot --stdio`
/// (`Content-Length: <byte len>\r\n\r\n<body>`, no trailing
/// newline). `body` is ASCII JSON, so `str::len` is the byte
/// count the header must report.
fn write_lsp_frame(w: &mut impl Write, body: &str) -> std::io::Result<()> {
    write!(w, "Content-Length: {}\r\n\r\n{}", body.len(), body)
}

/// Extract the first complete top-level JSON object from `s` —
/// the slice from the first `{` to its matching `}`, brace-counted
/// with string/escape awareness. This lets the response parsers
/// cope with a `Content-Length:` header prefix or a glued
/// next-frame header suffix on the same line, so discovery is
/// robust whether the CLI replies newline-delimited or
/// Content-Length-framed.
fn extract_json_object(s: &str) -> Option<&str> {
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
fn parse_copilot_model_list(line: &str) -> Option<Vec<ModelEntry>> {
    let json: serde_json::Value = serde_json::from_str(extract_json_object(line)?).ok()?;
    if json.get("id")?.as_i64()? != 2 {
        return None;
    }
    let models = json.get("result")?.get("models")?.as_array()?;
    Some(
        models
            .iter()
            .filter_map(|m| {
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
fn discover_opencode() -> Vec<ModelEntry> {
    let Some(exe) = resolve_cli("opencode") else {
        return Vec::new();
    };
    let Ok(output) = Command::new(exe).arg("models").output() else {
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
    fn codex_cache_parser_extracts_slug_and_display_name() {
        // Mirrors the real `models_cache.json` shape; the parser
        // must skip entries missing a slug and not abort the rest.
        let json: serde_json::Value = serde_json::from_str(
            r#"{"models":[
                {"slug":"gpt-5.5","display_name":"GPT-5.5"},
                {"display_name":"no slug — skipped"},
                {"slug":"gpt-5.5-codex"}
            ]}"#,
        )
        .unwrap();
        let arr = json.get("models").unwrap().as_array().unwrap();
        let parsed: Vec<ModelEntry> = arr
            .iter()
            .filter_map(|m| {
                let slug = m.get("slug")?.as_str()?;
                let name = m
                    .get("display_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(slug);
                Some(ModelEntry::new(AgentProvider::CodexCli, slug, name))
            })
            .collect();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].display_name, "GPT-5.5");
        // Missing display_name falls back to the slug.
        assert_eq!(parsed[1].value, "gpt-5.5-codex");
        assert_eq!(parsed[1].display_name, "gpt-5.5-codex");
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
    fn discover_models_never_panics() {
        // Whatever is or isn't installed on the test machine, the
        // probe must return cleanly.
        let _ = discover_models();
    }
}
