//! `OPENPENCIL_DEBUG_TIMING=1` timeline for the built-in tool loop.
//!
//! The orchestrator's `timing_log` wraps `Orchestrator::run`; the design-agent
//! loop never goes through it, so a loop-generated screen printed no timing
//! at all. This is the same switch and the same `[TIMING]` stderr channel for
//! the loop: one line per model request (payload size, time to response
//! headers, stream time, tools called), one per tool execution, the token
//! usage the provider reports (which is where a prompt-cache hit shows up),
//! and the moment the first write lands on the canvas.
//!
//! Disabled, every method returns immediately and nothing is measured.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use op_orchestrator::timing_log::secs;
use serde_json::Value;

fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(op_orchestrator::timing_log::enabled)
}

/// One loop run's timeline.
pub(crate) struct LoopTiming {
    on: bool,
    wire: &'static str,
    started: Instant,
    requests: usize,
    request_started: Instant,
    headers_after: Duration,
    first_write_seen: bool,
}

impl LoopTiming {
    pub(crate) fn from_env(wire: &'static str) -> Self {
        Self::new(wire, enabled())
    }

    fn new(wire: &'static str, on: bool) -> Self {
        let now = Instant::now();
        let timing = Self {
            on,
            wire,
            started: now,
            requests: 0,
            request_started: now,
            headers_after: Duration::ZERO,
            first_write_seen: false,
        };
        timing.line(format_args!("loop start wire={wire}"));
        timing
    }

    fn line(&self, message: std::fmt::Arguments<'_>) {
        if self.on {
            eprintln!("{}", self.render(message));
        }
    }

    fn render(&self, message: std::fmt::Arguments<'_>) -> String {
        format!("[TIMING] +{} {message}", secs(self.started.elapsed()))
    }

    /// A model request is about to be sent.
    pub(crate) fn request(&mut self, body: &Value) {
        if !self.on {
            return;
        }
        self.requests += 1;
        self.request_started = Instant::now();
        let bytes = serde_json::to_vec(body).map_or(0, |v| v.len());
        let messages = body
            .get("messages")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        self.line(format_args!(
            "req#{} start in={} messages={messages}",
            self.requests,
            kb(bytes)
        ));
    }

    /// Response headers arrived (throttle gap + connect + time to first byte).
    pub(crate) fn headers(&mut self) {
        if self.on {
            self.headers_after = self.request_started.elapsed();
        }
    }

    /// The response stream ended; `tools` are the calls it asked for.
    pub(crate) fn response<'a>(&self, tools: impl Iterator<Item = &'a str>) {
        if !self.on {
            return;
        }
        let total = self.request_started.elapsed();
        let names: Vec<&str> = tools.collect();
        self.line(format_args!(
            "req#{} end headers={} stream={} total={} tools=[{}]",
            self.requests,
            secs(self.headers_after),
            secs(total.saturating_sub(self.headers_after)),
            secs(total),
            names.join(",")
        ));
    }

    /// One tool finished. `wrote` = a write-level tool that succeeded.
    pub(crate) fn tool(&mut self, name: &str, took: Duration, wrote: bool, is_error: bool) {
        if !self.on {
            return;
        }
        let first = wrote && !self.first_write_seen;
        self.first_write_seen |= wrote;
        self.line(format_args!(
            "tool {name} took={}{}{}",
            secs(took),
            if is_error { " error" } else { "" },
            if first {
                " (first write on canvas)"
            } else {
                ""
            }
        ));
    }
}

impl Drop for LoopTiming {
    fn drop(&mut self) {
        self.line(format_args!(
            "loop end wire={} total={} requests={}",
            self.wire,
            secs(self.started.elapsed()),
            self.requests
        ));
    }
}

/// Print the token usage an SSE event carries, if any. Anthropic reports it
/// under `message.usage` (start) and `usage` (final delta), OpenAI-compatible
/// endpoints under `usage` on the last chunk.
pub(crate) fn note_usage(event_data: &str) {
    if !enabled() || !event_data.contains("\"usage\"") {
        return;
    }
    if let Some(usage) = usage_of(event_data) {
        eprintln!("[TIMING] usage {usage}");
    }
}

fn usage_of(event_data: &str) -> Option<String> {
    let value: Value = serde_json::from_str(event_data).ok()?;
    let usage = value
        .get("usage")
        .or_else(|| value.get("message").and_then(|m| m.get("usage")))
        .filter(|usage| usage.is_object())?;
    Some(usage.to_string())
}

fn kb(bytes: usize) -> String {
    format!("{:.1}KB", bytes as f64 / 1024.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn usage_is_found_in_both_wire_shapes() {
        let start = json!({"type":"message_start","message":{"usage":{"input_tokens":12,"cache_read_input_tokens":9000}}});
        assert!(usage_of(&start.to_string())
            .unwrap()
            .contains("cache_read_input_tokens"));
        let delta = json!({"type":"message_delta","usage":{"output_tokens":7}});
        assert_eq!(
            usage_of(&delta.to_string()).unwrap(),
            r#"{"output_tokens":7}"#
        );
        assert!(usage_of(r#"{"choices":[],"usage":null}"#).is_none());
        assert!(usage_of("not json").is_none());
    }

    #[test]
    fn disabled_timing_measures_nothing() {
        let mut timing = LoopTiming::new("test", false);
        timing.request(&json!({"messages":[1,2]}));
        timing.headers();
        timing.tool("batch_design", Duration::from_secs(1), true, false);
        assert_eq!(timing.requests, 0);
        assert_eq!(timing.headers_after, Duration::ZERO);
        assert!(!timing.first_write_seen);
    }

    #[test]
    fn enabled_timing_counts_requests_and_flags_the_first_write_once() {
        let mut timing = LoopTiming::new("test", true);
        timing.request(&json!({"messages":[1,2]}));
        timing.request(&json!({"messages":[1,2,3]}));
        assert_eq!(timing.requests, 2);
        timing.tool("get_editor_state", Duration::ZERO, false, false);
        assert!(!timing.first_write_seen);
        timing.tool("batch_design", Duration::ZERO, true, false);
        assert!(timing.first_write_seen);
        assert!(timing
            .render(format_args!("x"))
            .starts_with("[TIMING] +0.0s x"));
    }
}
