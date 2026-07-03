//! Per-CLI quirks for the Codex / Gemini subprocess transports —
//! verbatim ports of the TS reference clients:
//!
//! - `apps/web/server/utils/codex-client.ts` — env allowlist (+
//!   `~/.codex/config.toml` `env_key` opt-ins), stderr error
//!   extraction with auth-hint rewrites, JSONL line parsing.
//! - `apps/web/server/utils/gemini-client.ts` — env allowlist,
//!   `-o stream-json` line parsing, user-friendly API error mapping.
//!
//! Both also share the TS `buildPrompt` GUIDELINES / TASK framing so
//! the wire prompt matches the TS server byte-for-byte.

use op_ai::chat_provider::{ChatDelta, StopReason};

/// Allowlist-based env filter for the Codex CLI subprocess. Only
/// passes through safe system vars and provider-specific prefixes —
/// prevents leaking secrets like ANTHROPIC_API_KEY, AWS_SECRET_KEY,
/// GITHUB_TOKEN, etc. (TS `codex-client.ts::CODEX_ENV_ALLOWLIST`).
const CODEX_ENV_ALLOWLIST: &[&str] = &[
    "PATH",
    "HOME",
    "TERM",
    "LANG",
    "SHELL",
    "TMPDIR",
    // HTTP(S) proxy pass-through: without these the CLI falls back to the
    // macOS SYSTEM proxy, whose stream-reconnect path codex rejects
    // ("Invalid proxy configuration: http://127.0.0.1:7897" — measured; a
    // clash hiccup mid-stream then kills the whole turn). The env proxy is
    // the path users actually validate in their terminal. `all_proxy`
    // (socks5) is deliberately NOT forwarded — that scheme is what the
    // reconnect client chokes on.
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "NO_PROXY",
    "http_proxy",
    "https_proxy",
    "no_proxy",
    // Windows-essential vars
    "SYSTEMROOT",
    "COMSPEC",
    "USERPROFILE",
    "APPDATA",
    "LOCALAPPDATA",
    "PATHEXT",
    "SYSTEMDRIVE",
    "TEMP",
    "TMP",
    "HOMEDRIVE",
    "HOMEPATH",
];

/// Allowlist-based env filter for the Gemini CLI subprocess (TS
/// `gemini-client.ts::GEMINI_ENV_ALLOWLIST` — same system core, no
/// extra config-file opt-ins).
const GEMINI_ENV_ALLOWLIST: &[&str] = CODEX_ENV_ALLOWLIST;

/// Extract provider-declared `env_key` entries from
/// `~/.codex/config.toml` content. Preserves the default safety
/// boundary of not forwarding sensitive environment variables
/// automatically, while still letting user-defined Codex providers
/// opt in through config.toml as the single source of truth (TS
/// `extractCodexConfigEnvKeys`, regex `^\s*env_key\s*=\s*"([^"]+)"\s*$`).
pub fn extract_codex_config_env_keys(config_toml: &str) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    for line in config_toml.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("env_key") else {
            continue;
        };
        let rest = rest.trim_start();
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        let rest = rest.trim();
        // Value must be a full double-quoted string with nothing after.
        let Some(inner) = rest
            .strip_prefix('"')
            .and_then(|r| r.strip_suffix('"'))
            .filter(|inner| !inner.contains('"'))
        else {
            continue;
        };
        if !inner.is_empty() && !keys.iter().any(|k| k == inner) {
            keys.push(inner.to_string());
        }
    }
    keys
}

/// Load the Codex config env-key opt-ins from disk (TS
/// `loadCodexConfigEnvKeys`: `$CODEX_HOME` or `~/.codex`, file
/// `config.toml`; missing / unreadable file = no extra keys).
fn load_codex_config_env_keys() -> Vec<String> {
    let codex_home = std::env::var("CODEX_HOME")
        .ok()
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".codex")));
    let Some(home) = codex_home else {
        return Vec::new();
    };
    match std::fs::read_to_string(home.join("config.toml")) {
        Ok(content) => extract_codex_config_env_keys(&content),
        Err(_) => Vec::new(),
    }
}

/// Codex child env: allowlisted system vars + `OPENAI_*` / `CODEX_*`
/// prefixes + config.toml `env_key` opt-ins (TS `filterCodexEnv`).
pub fn codex_child_env() -> Vec<(String, String)> {
    let extra = load_codex_config_env_keys();
    std::env::vars()
        .filter(|(k, _)| {
            CODEX_ENV_ALLOWLIST.contains(&k.as_str())
                || extra.iter().any(|e| e == k)
                || k.starts_with("OPENAI_")
                || k.starts_with("CODEX_")
        })
        .collect()
}

/// Gemini child env: allowlisted system vars + `GOOGLE_*` /
/// `GEMINI_*` / `GCLOUD_*` prefixes (TS `filterGeminiEnv`).
pub fn gemini_child_env() -> Vec<(String, String)> {
    std::env::vars()
        .filter(|(k, _)| {
            GEMINI_ENV_ALLOWLIST.contains(&k.as_str())
                || k.starts_with("GOOGLE_")
                || k.starts_with("GEMINI_")
                || k.starts_with("GCLOUD_")
        })
        .collect()
}

/// TS `codex-client.ts::buildPrompt` / `gemini-client.ts::buildPrompt`
/// framing: an empty system prompt passes the task through; otherwise
/// the system prompt rides a GUIDELINES block ahead of the TASK block.
pub fn guidelines_task_prompt(system_prompt: &str, task: &str) -> String {
    let task = task.trim();
    let system = system_prompt.trim();
    if system.is_empty() {
        return task.to_string();
    }
    format!(
        "You are a design generation assistant. Follow the guidelines below to produce the requested output.\n\
         \n\
         --- GUIDELINES ---\n\
         {system}\n\
         \n\
         --- TASK ---\n\
         {task}"
    )
}

/// Parse one `codex exec --json` stdout line (TS
/// `parseCodexJsonLine` + the Rust streaming superset). `None` =
/// skip the line silently — TS drops non-JSON lines and unknown
/// events without surfacing them in the transcript.
///
/// Beyond-TS-baseline (documented divergence): TS buffers the whole
/// turn and reads the final text from `--output-last-message`; Rust
/// streams `item.completed` agent messages live instead, so the same
/// final text arrives incrementally without the temp file.
pub fn parse_codex_line(line: &str) -> Option<ChatDelta> {
    let trimmed = line.trim();
    if !trimmed.starts_with('{') {
        return None; // non-JSON (warnings, banners) — TS skips
    }
    let val: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    let ty = val.get("type").and_then(|v| v.as_str()).unwrap_or("");
    match ty {
        // TS: error events surface `message`, with a fixed fallback.
        "error" => {
            let msg = val
                .get("message")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .unwrap_or("Codex returned an unknown error.");
            Some(ChatDelta::Error(msg.to_string()))
        }
        // Rust streaming superset: the completed agent message is the
        // turn's answer text.
        "item.completed" => {
            let item = val.get("item");
            let item_ty = item.and_then(|i| i.get("type")).and_then(|v| v.as_str());
            let text = item.and_then(|i| i.get("text")).and_then(|v| v.as_str());
            match (item_ty, text) {
                (Some("agent_message"), Some(text)) => Some(ChatDelta::TextDelta(text.to_string())),
                _ => None,
            }
        }
        "turn.completed" => Some(ChatDelta::Done {
            stop_reason: StopReason::EndTurn,
        }),
        // Lifecycle events carry no renderable payload — skip (TS
        // finds no delta/text/content field on them either).
        "thread.started" | "turn.started" | "item.started" | "item.updated" => None,
        // TS: common Codex JSONL stream events include deltas in
        // "delta" or "text" (or "content"); take the first non-empty.
        _ => {
            let text = ["delta", "text", "content"].iter().find_map(|k| {
                val.get(*k)
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
            });
            text.map(|t| ChatDelta::TextDelta(t.to_string()))
        }
    }
}

/// Extract a human-readable error from Codex stderr (TS
/// `extractCodexCliError`). `None` when stderr is blank.
pub fn extract_codex_cli_error(stderr: &str) -> Option<String> {
    let trimmed = stderr.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lines: Vec<&str> = trimmed
        .split('\n')
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();

    // 1. Look for "error: ..." lines (simple CLI errors), last first.
    for line in lines.iter().rev() {
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("error:") {
            return Some(line["error:".len()..].trim().to_string());
        }
    }

    // 2. Look for Codex structured log errors:
    //    "<timestamp> ERROR <module>: <message>" — these contain the
    //    real error (auth failures, API errors, etc.).
    for line in lines.iter().rev() {
        if let Some(msg) = match_structured_error(line) {
            // For auth errors, provide actionable guidance.
            if is_codex_auth_error(&msg) {
                return Some(
                    "Codex authentication expired. Run \"codex logout && codex login\" to re-authenticate."
                        .to_string(),
                );
            }
            return Some(msg);
        }
    }

    // 3. Skip unhelpful "Warning: no last agent message" — surface it
    //    only as a friendlier fallback.
    let last_line = lines.last().copied();
    if let Some(last) = last_line {
        if last
            .to_ascii_lowercase()
            .starts_with("warning: no last agent message")
        {
            return Some(
                "Codex returned no output. Check \"codex login\" status or try a different model."
                    .to_string(),
            );
        }
    }

    last_line.map(str::to_string)
}

/// TS regex `\bERROR\s+\S+:\s*(.+)` — a structured Codex log line.
fn match_structured_error(line: &str) -> Option<String> {
    let pos = find_word(line, "ERROR")?;
    let rest = line[pos + "ERROR".len()..].trim_start();
    // `\S+:` — a non-space module token ending in a colon.
    let colon = rest.find(':')?;
    let module = &rest[..colon];
    if module.is_empty() || module.contains(char::is_whitespace) {
        return None;
    }
    let msg = rest[colon + 1..].trim();
    if msg.is_empty() {
        None
    } else {
        Some(msg.to_string())
    }
}

/// Find `word` at a word boundary (preceded by start-of-string or a
/// non-alphanumeric char) — mirrors the TS `\b` anchor.
fn find_word(haystack: &str, word: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(rel) = haystack[from..].find(word) {
        let pos = from + rel;
        let boundary_ok = pos == 0
            || haystack[..pos]
                .chars()
                .next_back()
                .map(|c| !c.is_alphanumeric() && c != '_')
                .unwrap_or(true);
        if boundary_ok {
            return Some(pos);
        }
        from = pos + word.len();
    }
    None
}

/// TS auth-error sniff: `/refresh token|sign in again|token.*expired|401 Unauthorized/i`.
fn is_codex_auth_error(msg: &str) -> bool {
    let lower = msg.to_ascii_lowercase();
    if lower.contains("refresh token") || lower.contains("sign in again") {
        return true;
    }
    if lower.contains("401 unauthorized") {
        return true;
    }
    // `token.*expired` — "token" followed (anywhere later) by "expired".
    if let Some(pos) = lower.find("token") {
        if lower[pos..].contains("expired") {
            return true;
        }
    }
    false
}

/// Parse one `gemini -o stream-json` stdout line (TS
/// `parseStreamJsonLine`). `None` = skip silently — non-JSON noise
/// like "Loaded cached credentials." never reaches the transcript.
pub fn parse_gemini_stream_line(line: &str) -> Option<ChatDelta> {
    let trimmed = line.trim();
    if !trimmed.starts_with('{') {
        return None;
    }
    let val: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    let ty = val.get("type").and_then(|v| v.as_str()).unwrap_or("");

    if ty == "message" && val.get("role").and_then(|v| v.as_str()) == Some("assistant") {
        let content = val.get("content").and_then(|v| v.as_str()).unwrap_or("");
        if !content.is_empty() {
            return Some(ChatDelta::TextDelta(content.to_string()));
        }
        return None;
    }

    if ty == "result" {
        // Check for error in result event; a success result carries
        // no renderable delta (the text already streamed).
        if val.get("status").and_then(|v| v.as_str()) == Some("error") {
            if let Some(err) = val.get("error") {
                let msg = err
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error");
                return Some(ChatDelta::Error(friendly_gemini_api_error(msg)));
            }
        }
        return None;
    }

    if ty == "error" {
        let msg = val
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        return Some(ChatDelta::Error(friendly_gemini_api_error(msg)));
    }

    None
}

/// Map raw Gemini API errors to user-friendly messages (TS
/// `friendlyGeminiApiError`).
pub fn friendly_gemini_api_error(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("quota")
        || lower.contains("exhausted")
        || lower.contains("429")
        || lower.contains("capacity")
    {
        if let Some(reset) = match_reset_after(raw) {
            return format!("Gemini quota exhausted. Resets after {reset}.");
        }
        return "Gemini quota exhausted. Please wait and try again.".to_string();
    }
    if lower.contains("401") || lower.contains("unauthenticated") || lower.contains("auth") {
        return "Gemini auth expired. Run \"gemini\" in your terminal to re-authenticate."
            .to_string();
    }
    if raw.contains("[object Object]") {
        return "Gemini API error. Check your quota or try a different model.".to_string();
    }
    raw.to_string()
}

/// TS regex `/reset after (\S+)/i` — the quota-reset timestamp token.
fn match_reset_after(raw: &str) -> Option<&str> {
    let lower = raw.to_ascii_lowercase();
    let pos = lower.find("reset after ")?;
    let tail = &raw[pos + "reset after ".len()..];
    let token: &str = tail.split_whitespace().next()?;
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_config_env_keys_extracts_quoted_values() {
        let toml = r#"
[model_providers.custom]
name = "Custom"
env_key = "CUSTOM_API_KEY"
  env_key = "SECOND_KEY"
env_key = "CUSTOM_API_KEY"
env_key = not_quoted
"#;
        assert_eq!(
            extract_codex_config_env_keys(toml),
            vec!["CUSTOM_API_KEY".to_string(), "SECOND_KEY".to_string()],
            "dedupe + quoted-only, like the TS regex"
        );
    }

    #[test]
    fn guidelines_task_prompt_matches_ts_framing() {
        let p = guidelines_task_prompt("Be terse.", "make a button");
        assert!(p.starts_with("You are a design generation assistant."));
        assert!(p.contains("--- GUIDELINES ---\nBe terse.\n"));
        assert!(p.ends_with("--- TASK ---\nmake a button"));
        // Empty system → bare task (TS early return).
        assert_eq!(guidelines_task_prompt("  ", " hi "), "hi");
    }

    #[test]
    fn codex_line_parser_skips_noise_and_lifecycle() {
        assert_eq!(parse_codex_line("Reading prompt from stdin..."), None);
        assert_eq!(
            parse_codex_line(r#"{"type":"thread.started","thread_id":"t"}"#),
            None
        );
        assert_eq!(parse_codex_line(r#"{"type":"item.started"}"#), None);
        assert_eq!(
            parse_codex_line(r#"{"type":"item.completed","item":{"type":"reasoning"}}"#),
            None
        );
        assert_eq!(parse_codex_line("{not json"), None);
    }

    #[test]
    fn codex_line_parser_streams_agent_message_and_done() {
        assert_eq!(
            parse_codex_line(
                r#"{"type":"item.completed","item":{"type":"agent_message","text":"hi"}}"#
            ),
            Some(ChatDelta::TextDelta("hi".into()))
        );
        assert_eq!(
            parse_codex_line(r#"{"type":"turn.completed","usage":{}}"#),
            Some(ChatDelta::Done {
                stop_reason: StopReason::EndTurn
            })
        );
    }

    #[test]
    fn codex_line_parser_error_and_generic_delta_fields() {
        assert_eq!(
            parse_codex_line(r#"{"type":"error","message":"rate limited"}"#),
            Some(ChatDelta::Error("rate limited".into()))
        );
        // TS fallback when the error event has no message.
        assert_eq!(
            parse_codex_line(r#"{"type":"error"}"#),
            Some(ChatDelta::Error("Codex returned an unknown error.".into()))
        );
        // TS: delta / text / content extraction on unknown shapes.
        assert_eq!(
            parse_codex_line(r#"{"type":"whatever","delta":"abc"}"#),
            Some(ChatDelta::TextDelta("abc".into()))
        );
        assert_eq!(
            parse_codex_line(r#"{"type":"whatever","content":"xyz"}"#),
            Some(ChatDelta::TextDelta("xyz".into()))
        );
        assert_eq!(parse_codex_line(r#"{"type":"whatever"}"#), None);
    }

    #[test]
    fn codex_stderr_error_prefers_error_lines() {
        let stderr = "warming up\nerror: missing config\n";
        assert_eq!(
            extract_codex_cli_error(stderr).as_deref(),
            Some("missing config")
        );
        assert_eq!(extract_codex_cli_error("   "), None);
    }

    #[test]
    fn codex_stderr_error_rewrites_auth_failures() {
        let stderr =
            "2026-06-10T00:00:00 ERROR codex_core::auth: failed to refresh token, sign in again";
        assert_eq!(
            extract_codex_cli_error(stderr).as_deref(),
            Some(
                "Codex authentication expired. Run \"codex logout && codex login\" to re-authenticate."
            )
        );
        let stderr2 = "ts ERROR api: request failed with 502";
        assert_eq!(
            extract_codex_cli_error(stderr2).as_deref(),
            Some("request failed with 502")
        );
    }

    #[test]
    fn codex_stderr_error_no_last_agent_message_fallback() {
        let stderr = "Warning: no last agent message; check stderr";
        assert_eq!(
            extract_codex_cli_error(stderr).as_deref(),
            Some(
                "Codex returned no output. Check \"codex login\" status or try a different model."
            )
        );
        // Plain last-line fallback.
        assert_eq!(
            extract_codex_cli_error("something odd happened").as_deref(),
            Some("something odd happened")
        );
    }

    #[test]
    fn gemini_line_parser_assistant_messages_and_noise() {
        assert_eq!(parse_gemini_stream_line("Loaded cached credentials."), None);
        assert_eq!(
            parse_gemini_stream_line(r#"{"type":"message","role":"assistant","content":"Hello"}"#),
            Some(ChatDelta::TextDelta("Hello".into()))
        );
        // Empty content / non-assistant roles are skipped.
        assert_eq!(
            parse_gemini_stream_line(r#"{"type":"message","role":"assistant","content":""}"#),
            None
        );
        assert_eq!(
            parse_gemini_stream_line(r#"{"type":"message","role":"user","content":"x"}"#),
            None
        );
    }

    #[test]
    fn gemini_line_parser_result_and_error_events() {
        // Success result carries no delta.
        assert_eq!(
            parse_gemini_stream_line(r#"{"type":"result","status":"ok","response":"done"}"#),
            None
        );
        assert_eq!(
            parse_gemini_stream_line(
                r#"{"type":"result","status":"error","error":{"message":"401 unauthenticated"}}"#
            ),
            Some(ChatDelta::Error(
                "Gemini auth expired. Run \"gemini\" in your terminal to re-authenticate.".into()
            ))
        );
        assert_eq!(
            parse_gemini_stream_line(r#"{"type":"error","message":"plain failure"}"#),
            Some(ChatDelta::Error("plain failure".into()))
        );
    }

    #[test]
    fn gemini_friendly_error_table_matches_ts() {
        assert_eq!(
            friendly_gemini_api_error("Quota will reset after 3:00PM details"),
            "Gemini quota exhausted. Resets after 3:00PM."
        );
        assert_eq!(
            friendly_gemini_api_error("429 too many requests"),
            "Gemini quota exhausted. Please wait and try again."
        );
        assert_eq!(
            friendly_gemini_api_error("[object Object]"),
            "Gemini API error. Check your quota or try a different model."
        );
        assert_eq!(friendly_gemini_api_error("boring"), "boring");
    }
}
