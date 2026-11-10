//! Prompt-cache breakpoints for the Anthropic-wire tool loop.
//!
//! A design run re-sends the same ~50 KB of tools + system prompt on every
//! one of its up-to-28 turns, plus a history that only ever grows at the
//! end. Marking the end of the system prompt and the end of the latest
//! message as cache breakpoints lets the provider reuse the processed prefix
//! from the previous turn instead of reading it again. The model sees
//! exactly the same tokens either way; only latency and cost change.
//!
//! Third-party "Anthropic-compatible" endpoints do not all accept the block
//! form of `system` or the `cache_control` field, so the breakpoints are
//! only added for Anthropic's own host unless `OPENPENCIL_PROMPT_CACHE`
//! forces them on (`1`) or off (`0`).

use serde_json::{json, Value};

/// Environment override: `1` forces breakpoints on, `0` forces them off.
pub const PROMPT_CACHE_ENV: &str = "OPENPENCIL_PROMPT_CACHE";

/// Whether to add breakpoints for a request to `url`.
pub(crate) fn enabled_for(url: &str) -> bool {
    decide(url, std::env::var(PROMPT_CACHE_ENV).ok().as_deref())
}

fn decide(url: &str, env: Option<&str>) -> bool {
    match env.map(str::trim) {
        Some("1") => true,
        Some("0") => false,
        _ => reqwest::Url::parse(url)
            .ok()
            .and_then(|parsed| parsed.host_str().map(str::to_ascii_lowercase))
            .is_some_and(|host| host == "anthropic.com" || host.ends_with(".anthropic.com")),
    }
}

fn breakpoint() -> Value {
    json!({ "type": "ephemeral" })
}

/// Add the two breakpoints to an Anthropic Messages request body in place.
///
/// The body's `messages` are the request's own copy, so the loop's running
/// history never accumulates markers (the API allows four per request).
pub(crate) fn apply(body: &mut Value) {
    // Tools render before `system`, so this one breakpoint covers both.
    if let Some(text) = body.get("system").and_then(Value::as_str) {
        body["system"] = json!([{
            "type": "text",
            "text": text,
            "cache_control": breakpoint(),
        }]);
    }
    let Some(last) = body
        .get_mut("messages")
        .and_then(Value::as_array_mut)
        .and_then(|messages| messages.last_mut())
    else {
        return;
    };
    match last.get_mut("content") {
        Some(Value::String(text)) => {
            let text = std::mem::take(text);
            last["content"] = json!([{
                "type": "text",
                "text": text,
                "cache_control": breakpoint(),
            }]);
        }
        Some(Value::Array(blocks)) => {
            if let Some(block) = blocks.last_mut().filter(|block| block.is_object()) {
                block["cache_control"] = breakpoint();
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_anthropic_hosts_opt_in_by_default() {
        assert!(decide("https://api.anthropic.com/v1/messages", None));
        assert!(!decide(
            "https://open.bigmodel.cn/api/anthropic/v1/messages",
            None
        ));
        assert!(!decide(
            "https://anthropic.com.evil.example/v1/messages",
            None
        ));
        assert!(!decide("not a url", None));
        assert!(decide("https://proxy.example/v1/messages", Some("1")));
        assert!(!decide(
            "https://api.anthropic.com/v1/messages",
            Some(" 0 ")
        ));
        assert!(decide("https://api.anthropic.com/v1/messages", Some("yes")));
    }

    #[test]
    fn system_and_latest_message_get_a_breakpoint_and_text_is_unchanged() {
        let mut body = json!({
            "system": "You design.",
            "tools": [{"name": "batch_design"}],
            "messages": [
                {"role": "user", "content": "music app"},
                {"role": "assistant", "content": [{"type": "tool_use", "id": "t1"}]},
                {"role": "user", "content": [
                    {"type": "tool_result", "tool_use_id": "t1", "content": "ok"},
                    {"type": "text", "text": "nudge"},
                ]},
            ],
        });
        apply(&mut body);
        assert_eq!(body["system"][0]["text"], "You design.");
        assert_eq!(body["system"][0]["cache_control"]["type"], "ephemeral");
        let last = &body["messages"][2]["content"];
        assert!(last[0].get("cache_control").is_none());
        assert_eq!(last[1]["cache_control"]["type"], "ephemeral");
        assert_eq!(last[1]["text"], "nudge");
        // Earlier turns and the tools stay untouched.
        assert_eq!(body["messages"][0]["content"], "music app");
        assert!(body["tools"][0].get("cache_control").is_none());
        assert_eq!(body.to_string().matches("cache_control").count(), 2);
    }

    #[test]
    fn a_plain_string_first_message_becomes_one_marked_text_block() {
        let mut body = json!({
            "messages": [{"role": "user", "content": "hello"}],
        });
        apply(&mut body);
        assert!(body.get("system").is_none());
        assert_eq!(
            body["messages"][0]["content"],
            json!([{"type": "text", "text": "hello", "cache_control": {"type": "ephemeral"}}])
        );
    }
}
