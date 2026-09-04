//! Bounded semantic retry state for failed design writes.
//!
//! The host never replays a failed mutation. Instead, it gives the model one
//! dedicated request in which to inspect the tool error and issue a corrected
//! call. Keeping this budget separate from the ordinary turn cap guarantees
//! the correction request is delivered even when the failed write consumed
//! the last normal turn, while the one-shot guard prevents retry loops.

use op_ai::chat_provider::ChatToolResult;

/// A design run gets at most one model-authored corrective round.
const MAX_CORRECTIVE_WRITE_ROUNDS: usize = 1;

/// `ChatDelta` has no structured retry/progress variant. A text delta is
/// intentional here: unlike transient thinking state, it remains visible in
/// the completed transcript and answers whether a correction was attempted.
pub(super) const CORRECTIVE_WRITE_PROGRESS: &str =
    "\n\n• Retrying failed design write · corrective attempt 1/1";
pub(super) const CORRECTIVE_WRITE_EXHAUSTED: &str =
    "\n\n• Design write still failed after corrective attempt 1/1";

/// Per-run one-shot guard. `pending` means the next provider request is the
/// dedicated correction round and therefore bypasses the ordinary turn cap.
#[derive(Default)]
pub(super) struct CorrectiveWriteRetry {
    rounds_scheduled: usize,
    pending: bool,
}

impl CorrectiveWriteRetry {
    /// Schedule the sole correction round. Returns the model-facing nudge when
    /// accepted, or `None` after the run has already spent its retry budget.
    pub(super) fn schedule(&mut self, failed_tools: &[String]) -> Option<String> {
        if failed_tools.is_empty()
            || self.pending
            || self.rounds_scheduled >= MAX_CORRECTIVE_WRITE_ROUNDS
        {
            return None;
        }
        self.rounds_scheduled += 1;
        self.pending = true;
        let tools = failed_tools.join(", ");
        Some(format!(
            "Design write tool call(s) failed: {tools}. This is your one corrective retry. \
             Inspect the tool error result above and issue a corrected write call now. \
             Do not repeat the same tool arguments unchanged. If you cannot correct the \
             write, explain the remaining failure explicitly."
        ))
    }

    /// Consume a pending correction at the start of its provider request.
    pub(super) fn begin_round(&mut self) -> bool {
        std::mem::take(&mut self.pending)
    }
}

/// Auth levels other than `read` represent a write/effectful tool. Unknown
/// tools are normalized to `read` by `AgentLoopConfig::level_for`, so this
/// fails closed instead of retrying an unclassified call.
pub(super) fn is_write_level(level: &str) -> bool {
    !level.eq_ignore_ascii_case("read")
}

/// Only a tool/domain error is useful model feedback. Provider HTTP failures
/// occur before execution and never reach this function; UI-bridge failures
/// and executor panics do, so exclude their stable envelopes rather than
/// spending the semantic correction budget on infrastructure the model cannot
/// repair by changing arguments.
pub(super) fn is_correctable_write_failure(level: &str, result: &ChatToolResult) -> bool {
    if !is_write_level(level) || !result.is_error {
        return false;
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&result.content) else {
        return false;
    };
    if value.get("success").and_then(serde_json::Value::as_bool) != Some(false) {
        return false;
    }
    let error = value
        .get("error")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    !error.starts_with("tool executor panicked:")
        && error != "tool execution timed out waiting for the editor"
        && error != "chat turn aborted before the tool ran"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedules_exactly_one_round() {
        let mut retry = CorrectiveWriteRetry::default();
        assert!(retry.schedule(&["batch_design".into()]).is_some());
        assert!(retry.begin_round());
        assert!(!retry.begin_round());
        assert!(retry.schedule(&["batch_design".into()]).is_none());
    }

    #[test]
    fn read_level_is_not_a_write() {
        assert!(!is_write_level("read"));
        assert!(is_write_level("create"));
        assert!(is_write_level("modify"));
        assert!(is_write_level("delete"));
    }

    #[test]
    fn infrastructure_errors_are_not_model_correctable() {
        let timeout = ChatToolResult {
            content:
                r#"{"success":false,"error":"tool execution timed out waiting for the editor"}"#
                    .into(),
            is_error: true,
        };
        let semantic = ChatToolResult {
            content: r#"{"success":false,"error":"height must be an integer"}"#.into(),
            is_error: true,
        };
        assert!(!is_correctable_write_failure("modify", &timeout));
        assert!(is_correctable_write_failure("modify", &semantic));
        assert!(!is_correctable_write_failure("read", &semantic));
    }
}
