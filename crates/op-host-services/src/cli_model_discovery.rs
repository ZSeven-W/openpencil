//! Model catalogs for external coding CLIs whose integrations are newer than
//! the legacy provider set. Kept separate from `model_discovery.rs` so that
//! file stays below the repository's 800-line cap.

use std::collections::BTreeSet;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use op_ai::agent_settings_state::AgentProvider;
use op_ai::chat_models::ModelEntry;
use op_ai::chat_provider::CliName;

use crate::model_discovery::resolve_cli;

const MODEL_QUERY_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_COMMAND_OUTPUT_BYTES: usize = 1024 * 1024;

/// Query the installed Antigravity catalog. `agy models` currently prints a
/// human list of display names (the exact strings accepted by `--model`), so
/// parsing preserves spaces and parenthesized effort levels verbatim.
pub fn query_antigravity_models() -> Result<Vec<ModelEntry>, String> {
    let exe = resolve_cli("agy").ok_or_else(|| "Antigravity CLI not found".to_string())?;
    let output = command_output(CliName::Antigravity, &exe, &["models"], MODEL_QUERY_TIMEOUT)
        .ok_or_else(|| "Antigravity model query failed or timed out".to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "Antigravity model query failed. Run `agy` once to authenticate.".to_string()
        } else {
            stderr
        });
    }
    require_antigravity_models(
        &String::from_utf8_lossy(&output.stdout),
        &String::from_utf8_lossy(&output.stderr),
    )
}

pub fn discover_antigravity() -> Vec<ModelEntry> {
    if resolve_cli("agy").is_none() {
        return Vec::new();
    }
    query_antigravity_models().unwrap_or_else(|_| antigravity_default_model())
}

pub fn antigravity_default_model() -> Vec<ModelEntry> {
    vec![ModelEntry::new(
        AgentProvider::Antigravity,
        "default",
        "Antigravity default",
    )]
}

/// Parse `agy models`. Official examples are bullet lists such as
/// `Gemini 3.5 Flash (High)` and `Claude Opus 4.6 (Thinking)`; accept JSON
/// catalogs too so the integration survives a future machine-readable mode.
pub fn parse_antigravity_models(raw: &str) -> Vec<ModelEntry> {
    let mut names = BTreeSet::new();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) {
        // A catalog is either a top-level array or lives under one of the
        // documented catalog wrapper fields. Do not mine arbitrary top-level
        // `name` / `id` diagnostics for model-looking strings.
        collect_antigravity_names(&value, &mut names, value.is_array());
    }

    let mut in_catalog = false;
    let mut catalog_ended = false;
    let mut saw_catalog_entry = false;
    for line in raw.lines() {
        let clean = strip_ansi(line);
        let clean = clean.trim();
        if clean.is_empty() {
            if in_catalog && saw_catalog_entry {
                in_catalog = false;
                catalog_ended = true;
            }
            continue;
        }
        let lower = clean.to_ascii_lowercase();
        if lower.contains("available models") || lower == "models:" {
            in_catalog = true;
            catalog_ended = false;
            saw_catalog_entry = false;
            continue;
        }
        let (candidate, was_bullet) = trim_catalog_bullet(clean);
        if candidate.is_empty() || is_catalog_diagnostic(candidate) {
            if in_catalog && saw_catalog_entry {
                in_catalog = false;
                catalog_ended = true;
            }
            continue;
        }
        let unheaded_bullet = was_bullet && !catalog_ended;
        let unheaded_model = !catalog_ended && looks_like_antigravity_model(candidate);
        if (in_catalog || unheaded_bullet || unheaded_model)
            && looks_like_antigravity_model(candidate)
        {
            names.insert(candidate.to_string());
            if in_catalog || unheaded_bullet {
                saw_catalog_entry = true;
            }
        } else if in_catalog && saw_catalog_entry {
            in_catalog = false;
            catalog_ended = true;
        }
    }

    names
        .into_iter()
        .map(|name| ModelEntry::new(AgentProvider::Antigravity, name.clone(), name))
        .collect()
}

fn collect_antigravity_names(
    value: &serde_json::Value,
    out: &mut BTreeSet<String>,
    catalog_context: bool,
) {
    match value {
        serde_json::Value::String(name)
            if catalog_context && looks_like_antigravity_model(name) =>
        {
            out.insert(name.trim().to_string());
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_antigravity_names(value, out, true);
            }
        }
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                if catalog_context
                    && matches!(
                        key.as_str(),
                        "id" | "model" | "name" | "displayName" | "display_name"
                    )
                {
                    if let Some(name) = value
                        .as_str()
                        .map(str::trim)
                        .filter(|name| looks_like_antigravity_model(name))
                    {
                        out.insert(name.to_string());
                    }
                } else if matches!(key.as_str(), "models" | "data" | "result" | "catalog") {
                    collect_antigravity_names(value, out, true);
                }
            }
        }
        _ => {}
    }
}

fn trim_catalog_bullet(line: &str) -> (&str, bool) {
    let trimmed = line.trim_start();
    for bullet in ["* ", "- ", "• ", "● "] {
        if let Some(rest) = trimmed.strip_prefix(bullet) {
            return (rest.trim(), true);
        }
    }
    (trimmed.trim(), false)
}

fn looks_like_antigravity_model(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    !is_catalog_diagnostic(&lower)
        && [
            "gemini ",
            "claude ",
            "gpt-",
            "gpt ",
            "gemma ",
            "deepseek ",
            "grok ",
            "qwen ",
        ]
        .iter()
        .any(|prefix| lower.starts_with(prefix))
}

fn is_catalog_diagnostic(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    [
        "sign in",
        "signin",
        "log in",
        "login",
        "authenticate",
        "authentication",
        "unauthorized",
        "credential",
        "api key",
        "required",
        "unavailable",
        "failed",
        "failure",
        "error:",
        "no models",
        "loading",
        "checking",
        "timed out",
        "troubleshoot",
        "documentation",
        "release notes",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

/// Query the real Grok Build model catalog. The command is bounded because
/// model discovery runs during startup and must never strand its worker.
pub fn query_grok_models() -> Result<Vec<ModelEntry>, String> {
    let exe = resolve_cli("grok").ok_or_else(|| "Grok Build CLI not found".to_string())?;
    let output = command_output(CliName::GrokBuild, &exe, &["models"], MODEL_QUERY_TIMEOUT)
        .ok_or_else(|| "Grok Build model query failed or timed out".to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "Grok Build model query failed".to_string()
        } else {
            stderr
        });
    }
    require_grok_models(
        &String::from_utf8_lossy(&output.stdout),
        &String::from_utf8_lossy(&output.stderr),
    )
}

pub fn discover_grok() -> Vec<ModelEntry> {
    if resolve_cli("grok").is_none() {
        return Vec::new();
    }
    query_grok_models().unwrap_or_else(|_| grok_default_model())
}

fn grok_default_model() -> Vec<ModelEntry> {
    vec![ModelEntry::new(
        AgentProvider::GrokBuild,
        "default",
        "Grok Build default",
    )]
}

/// Parse `grok models`. Current builds print a human list, while some builds
/// expose JSON; accept both without mistaking headings or status prose for ids.
pub fn parse_grok_models(raw: &str) -> Vec<ModelEntry> {
    let mut ids = BTreeSet::new();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) {
        let allow_scalar = value.is_array();
        collect_grok_ids(&value, &mut ids, allow_scalar);
    }

    let mut in_catalog = false;
    let mut catalog_ended = false;
    let mut saw_catalog_entry = false;
    for line in raw.lines() {
        let clean = strip_ansi(line);
        let clean = clean.trim();
        if clean.is_empty() {
            if in_catalog && saw_catalog_entry {
                in_catalog = false;
                catalog_ended = true;
            }
            continue;
        }
        if is_grok_catalog_separator(clean) {
            continue;
        }
        if is_grok_catalog_heading(clean) {
            in_catalog = true;
            catalog_ended = false;
            saw_catalog_entry = false;
            continue;
        }
        if is_grok_catalog_table_header(clean) {
            in_catalog = true;
            catalog_ended = false;
            saw_catalog_entry = false;
            continue;
        }
        if looks_like_grok_catalog_prose(clean) {
            if in_catalog && saw_catalog_entry {
                in_catalog = false;
                catalog_ended = true;
            }
            continue;
        }

        let (row, was_bullet) = trim_catalog_bullet(clean);
        let unheaded_bullet = was_bullet && !catalog_ended;
        let candidate = grok_catalog_row_id(row, in_catalog || unheaded_bullet);
        if let Some(id) = candidate.filter(|id| {
            in_catalog || unheaded_bullet || (!catalog_ended && is_official_grok_model_id(id))
        }) {
            ids.insert(id.to_string());
            if in_catalog || unheaded_bullet {
                saw_catalog_entry = true;
            }
        } else if in_catalog && saw_catalog_entry {
            in_catalog = false;
            catalog_ended = true;
        }
    }
    ids.into_iter()
        .map(|id| ModelEntry::new(AgentProvider::GrokBuild, id.clone(), id))
        .collect()
}

fn collect_grok_ids(value: &serde_json::Value, out: &mut BTreeSet<String>, allow_scalar: bool) {
    match value {
        serde_json::Value::String(value) if allow_scalar && is_grok_model_id(value) => {
            out.insert(value.clone());
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_grok_ids(value, out, allow_scalar);
            }
        }
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                if allow_scalar && matches!(key.as_str(), "id" | "model" | "name" | "alias") {
                    if let Some(value) = value.as_str().filter(|value| is_grok_model_id(value)) {
                        out.insert(value.to_string());
                    }
                } else if matches!(key.as_str(), "models" | "data" | "result" | "catalog") {
                    collect_grok_ids(value, out, true);
                }
            }
        }
        _ => {}
    }
}

fn is_grok_model_id(value: &str) -> bool {
    let value = value.trim();
    let lower = value.to_ascii_lowercase();
    value.len() >= 2
        && value.len() <= 128
        && value
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
        && value
            .chars()
            .last()
            .is_some_and(|c| c.is_ascii_alphanumeric())
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | ':'))
        && !is_reserved_grok_catalog_word(&lower)
}

fn is_official_grok_model_id(value: &str) -> bool {
    value.to_ascii_lowercase().starts_with("grok-") && is_grok_model_id(value)
}

fn is_grok_catalog_heading(line: &str) -> bool {
    matches!(
        line.trim()
            .trim_end_matches(':')
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "models" | "available models" | "configured models" | "model catalog"
    )
}

fn is_grok_catalog_table_header(line: &str) -> bool {
    let Some(column) = first_grok_catalog_column(line) else {
        return false;
    };
    matches!(
        column.trim().to_ascii_lowercase().as_str(),
        "id" | "model" | "model id" | "name" | "alias"
    )
}

fn is_grok_catalog_separator(line: &str) -> bool {
    line.chars()
        .all(|c| c.is_ascii_whitespace() || matches!(c, '-' | '=' | '+' | '|'))
}

fn grok_catalog_row_id(row: &str, catalog_context: bool) -> Option<&str> {
    let row = row.trim().trim_matches('`');
    if row.is_empty() || looks_like_grok_catalog_prose(row) {
        return None;
    }

    if let Some(column) = first_grok_catalog_column(row) {
        return is_grok_model_id(column).then_some(column);
    }
    if is_grok_model_id(row) {
        return Some(row);
    }

    let (candidate, metadata) = row.split_once(char::is_whitespace)?;
    (catalog_context && is_grok_model_id(candidate) && is_grok_catalog_metadata(metadata))
        .then_some(candidate)
}

fn first_grok_catalog_column(row: &str) -> Option<&str> {
    if row.contains('|') {
        return row
            .split('|')
            .map(str::trim)
            .find(|column| !column.is_empty());
    }

    let bytes = row.as_bytes();
    let mut index = 0;
    while index + 1 < bytes.len() {
        if bytes[index].is_ascii_whitespace() && bytes[index + 1].is_ascii_whitespace() {
            return Some(row[..index].trim());
        }
        index += 1;
    }
    None
}

fn is_grok_catalog_metadata(value: &str) -> bool {
    let lower = value
        .trim()
        .trim_matches(|c| matches!(c, '(' | ')' | '[' | ']'))
        .trim()
        .to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "default"
            | "selected"
            | "current"
            | "custom"
            | "configured"
            | "builtin"
            | "built-in"
            | "active"
            | "ready"
            | "recommended"
    )
}

fn looks_like_grok_catalog_prose(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "sign in",
        "log in",
        "authenticate",
        "authentication",
        "unauthorized",
        "credential",
        "api key",
        "default model:",
        "status:",
        "error:",
        "failed to",
        "request failed",
        "timed out",
        "not available",
        "unavailable",
        "loading",
        "checking",
        "no models",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn is_reserved_grok_catalog_word(value: &str) -> bool {
    matches!(
        value,
        "active"
            | "automatic"
            | "available"
            | "checking"
            | "connected"
            | "configured"
            | "current"
            | "default"
            | "disconnected"
            | "error"
            | "failed"
            | "fetching"
            | "id"
            | "loading"
            | "model"
            | "models"
            | "name"
            | "none"
            | "ready"
            | "recommended"
            | "selected"
            | "status"
            | "unknown"
            | "update"
    )
}

fn require_antigravity_models(stdout: &str, stderr: &str) -> Result<Vec<ModelEntry>, String> {
    let models = parse_antigravity_models(stdout);
    if models.is_empty() {
        Err(catalog_error("Antigravity", "`agy`", stdout, stderr))
    } else {
        Ok(models)
    }
}

fn require_grok_models(stdout: &str, stderr: &str) -> Result<Vec<ModelEntry>, String> {
    let models = parse_grok_models(stdout);
    if models.is_empty() {
        Err(catalog_error("Grok Build", "`grok`", stdout, stderr))
    } else {
        Ok(models)
    }
}

fn catalog_error(provider: &str, command: &str, stdout: &str, stderr: &str) -> String {
    let diagnostics = format!("{stdout}\n{stderr}").to_ascii_lowercase();
    let auth_required = [
        "sign in",
        "signin",
        "log in",
        "login",
        "authenticate",
        "authentication",
        "unauthorized",
        "credential",
        "api key",
    ]
    .iter()
    .any(|marker| diagnostics.contains(marker));
    if auth_required {
        format!("{provider} model query requires authentication. Run {command} once to sign in.")
    } else if stdout.trim().is_empty() {
        format!("{provider} model query returned no model catalog")
    } else {
        format!("{provider} returned an unrecognized model catalog")
    }
}

fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            let _ = chars.next();
            for code in chars.by_ref() {
                if code.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

pub(crate) fn command_output(
    cli: CliName,
    exe: &Path,
    args: &[&str],
    timeout: Duration,
) -> Option<Output> {
    let mut cmd = Command::new(exe);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .envs(crate::chat_subprocess_safety::child_env(Some(cli))?);
    crate::chat_spawn::hide_console_window(&mut cmd);
    let mut child = cmd.spawn().ok()?;
    let stdout = child.stdout.take()?;
    let stderr = child.stderr.take()?;
    // Drain both pipes while the process is running. Waiting for exit before
    // reading can deadlock when a verbose CLI fills an OS pipe buffer.
    let stdout_reader = read_pipe(stdout);
    let stderr_reader = read_pipe(stderr);
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return Some(Output {
                    status,
                    stdout: stdout_reader.join().unwrap_or_default(),
                    stderr: stderr_reader.join().unwrap_or_default(),
                });
            }
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return None;
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return None;
            }
        }
    }
}

fn read_pipe<R>(mut pipe: R) -> JoinHandle<Vec<u8>>
where
    R: Read + Send + 'static,
{
    std::thread::spawn(move || {
        let mut output = Vec::new();
        let mut chunk = [0_u8; 8192];
        loop {
            let Ok(count) = pipe.read(&mut chunk) else {
                break;
            };
            if count == 0 {
                break;
            }
            let remaining = MAX_COMMAND_OUTPUT_BYTES.saturating_sub(output.len());
            output.extend_from_slice(&chunk[..count.min(remaining)]);
        }
        output
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_normal_grok_ids_and_custom_aliases_from_catalog_rows() {
        let text = "Available models:\n\
                    * grok-code-fast-1 (default)\n\
                    * my-model (custom)\n\
                    | grok-4.1-fast | ready |\n\
                    | company/sonnet:prod | configured |";
        let models = parse_grok_models(text);
        assert_eq!(
            models
                .iter()
                .map(|model| model.value.as_str())
                .collect::<Vec<_>>(),
            [
                "company/sonnet:prod",
                "grok-4.1-fast",
                "grok-code-fast-1",
                "my-model",
            ]
        );

        let models =
            parse_grok_models(r#"{"models":[{"id":"grok-code-fast-1"},{"alias":"my-model"}]}"#);
        assert_eq!(
            models
                .iter()
                .map(|model| model.value.as_str())
                .collect::<Vec<_>>(),
            ["grok-code-fast-1", "my-model"]
        );
    }

    #[test]
    fn parses_custom_aliases_from_headered_tables() {
        let text = "Model | Status\n------|-------\nmy-model | default\ngrok-4.5 | ready";
        let models = parse_grok_models(text);
        assert_eq!(
            models
                .iter()
                .map(|model| model.value.as_str())
                .collect::<Vec<_>>(),
            ["grok-4.5", "my-model"]
        );
    }

    #[test]
    fn parses_antigravity_display_names_without_losing_effort_suffixes() {
        let text = "Available models:\n* Gemini 3.5 Flash (Medium)\n\
                    * Claude Opus 4.6 (Thinking)\n* GPT-OSS 120B (Medium)";
        let models = parse_antigravity_models(text);
        assert_eq!(
            models
                .iter()
                .map(|model| model.value.as_str())
                .collect::<Vec<_>>(),
            [
                "Claude Opus 4.6 (Thinking)",
                "GPT-OSS 120B (Medium)",
                "Gemini 3.5 Flash (Medium)",
            ]
        );
        assert!(models.iter().all(|model| model.value == model.display_name));
    }

    #[test]
    fn parses_antigravity_json_and_ignores_auth_prose() {
        let models = parse_antigravity_models(
            r#"{"models":[{"displayName":"Gemini 3.1 Pro (High)"},{"name":"Claude Sonnet 4.6 (Thinking)"}]}"#,
        );
        assert_eq!(models.len(), 2);
        assert!(parse_antigravity_models("Please sign in to view available models").is_empty());
        assert!(parse_antigravity_models(
            "Available models:\n* Gemini authentication required\n* Claude login failed"
        )
        .is_empty());
        assert!(
            parse_antigravity_models(r#"{"name":"Gemini authentication required"}"#).is_empty()
        );
    }

    #[test]
    fn human_catalog_does_not_resume_after_its_blank_terminator() {
        let antigravity = parse_antigravity_models(
            "Available models:\n* Gemini 3.5 Flash (High)\n\n* Claude CLI troubleshooting",
        );
        assert_eq!(antigravity.len(), 1);
        assert_eq!(antigravity[0].value, "Gemini 3.5 Flash (High)");

        let grok =
            parse_grok_models("Available models:\n* grok-code-fast-1\n\n* release-notes-model");
        assert_eq!(grok.len(), 1);
        assert_eq!(grok[0].value, "grok-code-fast-1");
    }

    #[test]
    fn ignores_catalog_headings_and_unrelated_prose() {
        assert!(parse_grok_models("Available models:\nDefault model: automatic").is_empty());
        assert!(parse_grok_models(
            "Available models:\nStatus: ready\nconnected\nAuthentication required"
        )
        .is_empty());
        assert!(parse_grok_models("Please sign in to continue").is_empty());
        assert!(parse_grok_models(r#""connected""#).is_empty());
        assert!(parse_grok_models(r#"{"name":"grok-diagnostic"}"#).is_empty());
        assert!(
            parse_grok_models("Available models:\n* request failed\n* loading-models").is_empty()
        );
    }

    #[test]
    fn verified_catalogs_reject_empty_auth_and_unknown_output() {
        let empty = require_antigravity_models("", "").unwrap_err();
        assert!(empty.contains("no model catalog"));

        let auth = require_antigravity_models("", "Please sign in to continue").unwrap_err();
        assert!(auth.contains("requires authentication"));

        let unknown = require_grok_models("Available models:\nautomatic", "").unwrap_err();
        assert!(unknown.contains("unrecognized model catalog"));

        let auth = require_grok_models("", "Authentication required").unwrap_err();
        assert!(auth.contains("requires authentication"));
    }

    #[cfg(unix)]
    #[test]
    fn command_output_drains_large_stdout_and_stderr_before_exit() {
        let script = "i=0; while [ $i -lt 40000 ]; do \
                      printf '0123456789abcdef0123456789abcdef\\n'; \
                      printf 'fedcba9876543210fedcba9876543210\\n' >&2; \
                      i=$((i+1)); done";
        let output = command_output(
            CliName::GrokBuild,
            Path::new("/bin/sh"),
            &["-c", script],
            Duration::from_secs(5),
        )
        .expect("large piped output should not deadlock");
        assert!(output.status.success());
        assert_eq!(output.stdout.len(), MAX_COMMAND_OUTPUT_BYTES);
        assert_eq!(output.stderr.len(), MAX_COMMAND_OUTPUT_BYTES);
    }
}
