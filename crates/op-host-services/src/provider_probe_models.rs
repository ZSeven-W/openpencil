//! Live model-list queries for Claude + Gemini, plus the Codex
//! `latest-model.md` fallback parser.
//!
//! These are the "no hardcoded lists" upgrades shared by startup
//! discovery (`model_discovery.rs`) and the connect-time probe
//! (`provider_probe.rs`). Each mirrors the TS implementation in
//! `apps/web/server/api/ai/connect-agent.ts`:
//!
//! - Claude: the Agent SDK's `query().supportedModels()` /
//!   `accountInfo()` resolve from the CLI's `initialize` control
//!   response over stream-json stdio (connect-agent.ts:147-262 via
//!   `@anthropic-ai/claude-agent-sdk`). We speak the same wire
//!   protocol directly: spawn `claude --output-format stream-json
//!   --verbose --input-format stream-json …`, write one
//!   `control_request {subtype:"initialize"}` line, read the
//!   `control_response` carrying `models` + `account`.
//! - Gemini: the `generativelanguage.googleapis.com/v1beta/models`
//!   listing authenticated by `GEMINI_API_KEY` / `GOOGLE_API_KEY`
//!   or the CLI's `~/.gemini/oauth_creds.json` access token
//!   (connect-agent.ts:904-973), filtered to `generateContent`
//!   models and sorted gemini-3 first.
//! - Codex: `~/.codex/skills/.system/openai-docs/references/
//!   latest-model.md` markdown-table rows when `models_cache.json`
//!   is missing (connect-agent.ts:378-413).

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use op_ai::agent_settings_state::AgentProvider;
use op_ai::chat_models::ModelEntry;

use crate::model_discovery::{extract_json_object, resolve_cli};

/// Budget for the `claude` CLI to answer the `initialize` control
/// request. CLI startup (Node boot + auth check) takes a few
/// seconds; both callers run off the UI thread.
const CLAUDE_INIT_TIMEOUT: Duration = Duration::from_secs(15);

/// Account fields the `initialize` control response carries —
/// the TS Agent SDK's `accountInfo()` shape used by
/// `buildClaudeConnectionInfo` (connect-agent.ts:183-197, 270-295).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClaudeAccount {
    pub email: Option<String>,
    pub subscription_type: Option<String>,
}

/// Result of the live Claude initialize query.
pub enum ClaudeInitResult {
    /// The CLI answered — model list (may be empty) + account info.
    Answered(Vec<ModelEntry>, Option<ClaudeAccount>),
    /// The CLI exited with a failure code before answering.
    ExitedWithError(i32),
    /// Spawn failure, timeout, or clean exit without a response —
    /// the TS "query closed before response" bucket that falls back
    /// to the default model list (connect-agent.ts:216-258).
    NoAnswer,
}

/// Query the Claude CLI for its supported models + account info via
/// the stream-json `initialize` control request — the same wire
/// call the TS Agent SDK's `supportedModels()` awaits.
pub fn claude_initialize_query() -> ClaudeInitResult {
    let Some(exe) = resolve_cli("claude") else {
        return ClaudeInitResult::NoAnswer;
    };
    // Arg set mirrors the TS connect options `{maxTurns: 1, tools:
    // [], permissionMode: 'plan', persistSession: false}` after the
    // SDK's arg mapping (sdk.mjs: `--tools ""` for an empty array,
    // `--no-session-persistence` for persistSession=false).
    let mut cmd = Command::new(exe);
    cmd.args([
        "--output-format",
        "stream-json",
        "--verbose",
        "--input-format",
        "stream-json",
        "--max-turns",
        "1",
        "--tools",
        "",
        "--permission-mode",
        "plan",
        "--no-session-persistence",
    ])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::null());
    // The SDK sets the entrypoint marker and strips NODE_OPTIONS.
    if std::env::var_os("CLAUDE_CODE_ENTRYPOINT").is_none() {
        cmd.env("CLAUDE_CODE_ENTRYPOINT", "sdk-ts");
    }
    cmd.env_remove("NODE_OPTIONS");
    crate::chat_spawn::hide_console_window(&mut cmd);
    let Ok(mut child) = cmd.spawn() else {
        return ClaudeInitResult::NoAnswer;
    };
    let Some(mut stdin) = child.stdin.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return ClaudeInitResult::NoAnswer;
    };
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return ClaudeInitResult::NoAnswer;
    };

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    let init = r#"{"request_id":"op-connect-1","type":"control_request","request":{"subtype":"initialize"}}"#;
    if writeln!(stdin, "{init}").is_err() {
        let _ = child.kill();
        let _ = child.wait();
        return ClaudeInitResult::NoAnswer;
    }
    let _ = stdin.flush();

    let deadline = Instant::now() + CLAUDE_INIT_TIMEOUT;
    let mut answered = None;
    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        match rx.recv_timeout(remaining) {
            Ok(line) => {
                if let Some(parsed) = parse_claude_init_line(&line) {
                    answered = Some(parsed);
                    break;
                }
            }
            // Timeout, or the reader closed because the CLI exited
            // without answering — stop waiting either way.
            Err(_) => break,
        }
    }
    drop(stdin);
    let status = if answered.is_some() {
        let _ = child.kill();
        child.wait().ok()
    } else {
        // Give the exited process a moment to report its code so a
        // login failure maps to the friendly-error path.
        child.try_wait().ok().flatten().or_else(|| {
            let _ = child.kill();
            child.wait().ok()
        })
    };
    match answered {
        Some((models, account)) => ClaudeInitResult::Answered(models, account),
        None => match status {
            Some(st) if !st.success() => ClaudeInitResult::ExitedWithError(st.code().unwrap_or(1)),
            _ => ClaudeInitResult::NoAnswer,
        },
    }
}

/// Parse one stream-json line from the Claude CLI; yields models +
/// account only for the successful `initialize` control response.
pub fn parse_claude_init_line(line: &str) -> Option<(Vec<ModelEntry>, Option<ClaudeAccount>)> {
    let json: serde_json::Value = serde_json::from_str(extract_json_object(line)?).ok()?;
    if json.get("type")?.as_str()? != "control_response" {
        return None;
    }
    let response = json.get("response")?;
    if response.get("subtype").and_then(|v| v.as_str()) != Some("success") {
        return None;
    }
    let inner = response.get("response")?;
    let models = inner
        .get("models")?
        .as_array()?
        .iter()
        .filter_map(|m| {
            let value = m.get("value")?.as_str()?;
            let name = m
                .get("displayName")
                .and_then(|v| v.as_str())
                .unwrap_or(value);
            Some(ModelEntry::new(AgentProvider::ClaudeCode, value, name))
        })
        .collect();
    let account = inner.get("account").map(|a| ClaudeAccount {
        email: a.get("email").and_then(|v| v.as_str()).map(str::to_string),
        subscription_type: a
            .get("subscriptionType")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    });
    Some((models, account))
}

/// TS `FALLBACK_CLAUDE_MODELS` (connect-agent.ts:102-145) — used
/// when `supportedModels()` doesn't answer (e.g. third-party API
/// proxies that don't support the listing endpoint).
pub fn fallback_claude_models() -> Vec<ModelEntry> {
    [
        ("claude-sonnet-4-6", "Claude Sonnet 4.6"),
        ("claude-opus-4-6", "Claude Opus 4.6"),
        ("claude-sonnet-4-5-20250514", "Claude Sonnet 4.5"),
        ("claude-haiku-4-5-20251001", "Claude Haiku 4.5"),
        ("claude-3-7-sonnet-20250219", "Claude 3.7 Sonnet"),
        ("claude-3-5-sonnet-20241022", "Claude 3.5 Sonnet"),
        ("claude-3-5-haiku-20241022", "Claude 3.5 Haiku"),
    ]
    .iter()
    .map(|(value, name)| ModelEntry::new(AgentProvider::ClaudeCode, *value, *name))
    .collect()
}

/// TS `FALLBACK_GEMINI_MODELS` (connect-agent.ts:870-901).
pub fn fallback_gemini_models() -> Vec<ModelEntry> {
    [
        ("gemini-3-pro-preview", "Gemini 3 Pro"),
        ("gemini-3-flash-preview", "Gemini 3 Flash"),
        ("gemini-2.5-pro", "Gemini 2.5 Pro"),
        ("gemini-2.5-flash", "Gemini 2.5 Flash"),
        ("gemini-2.0-flash", "Gemini 2.0 Flash"),
    ]
    .iter()
    .map(|(value, name)| ModelEntry::new(AgentProvider::GeminiCli, *value, *name))
    .collect()
}

/// Fetch the Gemini model list from the generativelanguage API
/// using the same local credentials the TS route reads
/// (connect-agent.ts:904-973). `None` = no usable credentials or
/// the request failed (callers fall back silently, like the TS
/// `catch`); `Some(vec![])` = the API answered with nothing usable
/// (TS surfaces a warning).
pub fn gemini_models_from_api() -> Option<Vec<ModelEntry>> {
    const BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";
    let env_key = std::env::var("GEMINI_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
        .or_else(|| {
            std::env::var("GOOGLE_API_KEY")
                .ok()
                .filter(|k| !k.is_empty())
        });
    let (url, bearer) = match env_key {
        Some(key) => (format!("{BASE}?key={key}"), None),
        None => {
            // OAuth token from the Gemini CLI login, with the same
            // 60 s expiry guard the TS route applies.
            let creds_path = dirs::home_dir()?.join(".gemini").join("oauth_creds.json");
            let raw = std::fs::read_to_string(creds_path).ok()?;
            let creds: serde_json::Value = serde_json::from_str(&raw).ok()?;
            let token = creds.get("access_token")?.as_str()?.to_string();
            if let Some(expiry) = creds.get("expiry_date").and_then(|v| v.as_f64()) {
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .ok()?
                    .as_millis() as f64;
                if now_ms > expiry - 60_000.0 {
                    return None; // Token expired.
                }
            }
            (BASE.to_string(), Some(token))
        }
    };

    let body = http_get_json(&url, bearer.as_deref())?;
    Some(parse_gemini_models_json(&body))
}

/// Blocking GET returning the response body. reqwest is async-only
/// in this workspace, so spin a current-thread runtime — the same
/// shape `iconify_host.rs` uses.
fn http_get_json(url: &str, bearer: Option<&str>) -> Option<String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?;
    runtime.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent(concat!("openpencil-desktop/", env!("CARGO_PKG_VERSION")))
            .build()
            .ok()?;
        let mut req = client.get(url);
        if let Some(token) = bearer {
            req = req.header("Authorization", format!("Bearer {token}"));
        }
        let res = req.send().await.ok()?.error_for_status().ok()?;
        res.text().await.ok()
    })
}

/// Parse + filter + sort the generativelanguage models listing —
/// `generateContent` models only, embedding/AQA/legacy ids skipped,
/// gemini-3 sorted first (connect-agent.ts:933-972).
pub fn parse_gemini_models_json(body: &str) -> Vec<ModelEntry> {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    let Some(arr) = json.get("models").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    let mut models = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for m in arr {
        let supports_generate = m
            .get("supportedGenerationMethods")
            .and_then(|v| v.as_array())
            .is_some_and(|methods| {
                methods
                    .iter()
                    .any(|v| v.as_str() == Some("generateContent"))
            });
        if !supports_generate {
            continue;
        }
        let id = m
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim_start_matches("models/")
            .to_string();
        if id.is_empty() || seen.contains(&id) || gemini_id_is_skipped(&id) {
            continue;
        }
        seen.insert(id.clone());
        let display = m
            .get("displayName")
            .and_then(|v| v.as_str())
            .unwrap_or(&id)
            .to_string();
        models.push(ModelEntry::new(AgentProvider::GeminiCli, id, display));
    }
    // Stable sort keeps the API order within each tier.
    models.sort_by_key(|m| gemini_tier(&m.value));
    models
}

/// TS skip regex `/embed|aqa|^chat-bison|^text-bison|^gemini-1\.0/i`.
fn gemini_id_is_skipped(id: &str) -> bool {
    let lower = id.to_ascii_lowercase();
    lower.contains("embed")
        || lower.contains("aqa")
        || lower.starts_with("chat-bison")
        || lower.starts_with("text-bison")
        || lower.starts_with("gemini-1.0")
}

/// TS sort order fn (connect-agent.ts:961-969).
fn gemini_tier(value: &str) -> u8 {
    if value.contains("gemini-3") {
        0
    } else if value.contains("gemini-2.5-pro") {
        1
    } else if value.contains("gemini-2.5-flash") {
        2
    } else if value.contains("gemini-2.0") {
        3
    } else {
        4
    }
}

/// `CODEX_HOME` env override or `~/.codex` — the same resolution
/// the TS route applies (connect-agent.ts:526).
pub fn codex_home() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("CODEX_HOME") {
        if !home.is_empty() {
            return Some(PathBuf::from(home));
        }
    }
    Some(dirs::home_dir()?.join(".codex"))
}

/// Parse model ids out of Codex's bundled `latest-model.md`
/// reference — the TS fallback when `models_cache.json` is missing
/// (connect-agent.ts:373-413). Text/reasoning models only.
pub fn codex_models_from_latest_md(codex_home: &Path) -> Vec<ModelEntry> {
    let md_path = codex_home
        .join("skills")
        .join(".system")
        .join("openai-docs")
        .join("references")
        .join("latest-model.md");
    let Ok(content) = std::fs::read_to_string(md_path) else {
        return Vec::new();
    };
    parse_codex_latest_model_md(&content)
}

/// Markdown-table row parser for `latest-model.md` — matches the TS
/// row regex ``^\|\s*`([^`]+)`\s*\|\s*(.+?)\s*\|`` and skip regex
/// `/image|audio|tts|transcribe|realtime|sora|video|embedding|moderation/i`.
pub fn parse_codex_latest_model_md(content: &str) -> Vec<ModelEntry> {
    let mut models = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for line in content.lines() {
        let Some(rest) = line.trim_start().strip_prefix('|') else {
            continue;
        };
        let mut cells = rest.split('|');
        let Some(first) = cells.next() else { continue };
        let first = first.trim();
        let Some(slug) = first
            .strip_prefix('`')
            .and_then(|s| s.strip_suffix('`'))
            .filter(|s| !s.is_empty())
        else {
            continue;
        };
        let desc = cells.next().unwrap_or("").trim();
        if codex_md_is_skipped(slug) || codex_md_is_skipped(desc) || seen.contains(slug) {
            continue;
        }
        seen.insert(slug.to_string());
        // TS maps `displayName: slug` (the description is carried in
        // a field ModelEntry doesn't have; display stays the slug).
        models.push(ModelEntry::new(AgentProvider::CodexCli, slug, slug));
    }
    models
}

fn codex_md_is_skipped(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "image",
        "audio",
        "tts",
        "transcribe",
        "realtime",
        "sora",
        "video",
        "embedding",
        "moderation",
    ]
    .iter()
    .any(|w| lower.contains(w))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_init_response_parser_extracts_models_and_account() {
        // Mirrors the CLI's control_response envelope: the SDK reads
        // `$.response.response.models` / `.account`.
        let line = r#"{"type":"control_response","response":{"subtype":"success","request_id":"op-connect-1","response":{"models":[{"value":"claude-sonnet-4-6","displayName":"Claude Sonnet 4.6","description":"Latest"},{"value":"claude-haiku-4-5"}],"account":{"email":"a@b.c","subscriptionType":"pro","apiKeySource":"none"}}}}"#;
        let (models, account) = parse_claude_init_line(line).expect("parses");
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].display_name, "Claude Sonnet 4.6");
        // Missing displayName falls back to the value.
        assert_eq!(models[1].display_name, "claude-haiku-4-5");
        let account = account.expect("account present");
        assert_eq!(account.email.as_deref(), Some("a@b.c"));
        assert_eq!(account.subscription_type.as_deref(), Some("pro"));
    }

    #[test]
    fn claude_init_parser_skips_non_init_lines() {
        // System/init chatter and error responses must not match.
        assert!(parse_claude_init_line(r#"{"type":"system","subtype":"init"}"#).is_none());
        assert!(parse_claude_init_line(
            r#"{"type":"control_response","response":{"subtype":"error","request_id":"x","error":"nope"}}"#
        )
        .is_none());
        assert!(parse_claude_init_line("not json").is_none());
    }

    #[test]
    fn fallback_model_lists_mirror_ts_constants() {
        let claude = fallback_claude_models();
        assert_eq!(claude.len(), 7);
        assert_eq!(claude[0].value, "claude-sonnet-4-6");
        assert_eq!(claude[6].display_name, "Claude 3.5 Haiku");
        let gemini = fallback_gemini_models();
        assert_eq!(gemini.len(), 5);
        assert_eq!(gemini[0].value, "gemini-3-pro-preview");
        assert_eq!(gemini[4].display_name, "Gemini 2.0 Flash");
    }

    #[test]
    fn gemini_listing_filters_sorts_and_dedupes_like_ts() {
        let body = r#"{"models":[
            {"name":"models/gemini-2.0-flash","displayName":"Gemini 2.0 Flash","supportedGenerationMethods":["generateContent"]},
            {"name":"models/text-embedding-004","displayName":"Embedding","supportedGenerationMethods":["embedContent"]},
            {"name":"models/gemini-embedding-001","displayName":"Gemini Embedding","supportedGenerationMethods":["generateContent"]},
            {"name":"models/aqa","displayName":"AQA","supportedGenerationMethods":["generateContent"]},
            {"name":"models/gemini-1.0-pro","displayName":"Legacy","supportedGenerationMethods":["generateContent"]},
            {"name":"models/gemini-3-pro-preview","displayName":"Gemini 3 Pro","supportedGenerationMethods":["generateContent"]},
            {"name":"models/gemini-2.5-pro","displayName":"Gemini 2.5 Pro","supportedGenerationMethods":["generateContent"]},
            {"name":"models/gemini-2.5-pro","displayName":"Duplicate","supportedGenerationMethods":["generateContent"]}
        ]}"#;
        let models = parse_gemini_models_json(body);
        let values: Vec<&str> = models.iter().map(|m| m.value.as_str()).collect();
        // gemini-3 first, then 2.5-pro, then 2.0; embed/aqa/legacy
        // skipped; duplicate deduped.
        assert_eq!(
            values,
            ["gemini-3-pro-preview", "gemini-2.5-pro", "gemini-2.0-flash"]
        );
        assert_eq!(models[0].display_name, "Gemini 3 Pro");
    }

    #[test]
    fn codex_latest_md_parser_keeps_text_models_only() {
        let md = "\
| Model | Description |\n\
| --- | --- |\n\
| `gpt-5.5` | Latest flagship reasoning model |\n\
| `gpt-5.5` | duplicate row |\n\
| `gpt-image-2` | Image generation model |\n\
| `sora-3` | Video generation |\n\
| `gpt-audio-mini` | Audio model |\n\
| no-backticks | not a model row |\n\
| `gpt-5.4-codex` | Coding model |\n";
        let models = parse_codex_latest_model_md(md);
        let values: Vec<&str> = models.iter().map(|m| m.value.as_str()).collect();
        assert_eq!(values, ["gpt-5.5", "gpt-5.4-codex"]);
        // displayName mirrors the slug like the TS mapper.
        assert_eq!(models[0].display_name, "gpt-5.5");
    }

    #[test]
    fn gemini_skip_filter_matches_ts_regex() {
        assert!(gemini_id_is_skipped("text-embedding-004"));
        assert!(gemini_id_is_skipped("aqa"));
        assert!(gemini_id_is_skipped("chat-bison-001"));
        assert!(gemini_id_is_skipped("gemini-1.0-pro"));
        assert!(!gemini_id_is_skipped("gemini-2.5-flash"));
        // `^` anchors: bison/legacy markers only skip at the start.
        assert!(!gemini_id_is_skipped("my-chat-bison"));
    }
}
