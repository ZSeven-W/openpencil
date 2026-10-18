//! Safe, bounded model feedback for failed script-generation attempts.

use crate::plan::RetryFeedback;
use op_mcp::script_runner::ScriptError;

pub(super) const MAX_SCRIPT_FEEDBACK_BYTES: usize = 1536;
const SCRIPT_MARKER: &str = "SCRIPT/PROTOCOL FIX REQUIRED:";

/// A script/transport failure does not resolve a prior quality rejection. Keep
/// its variant (which selects the prompt contract) and replace only the bounded
/// script appendix. A fresh quality gate remains authoritative.
pub(super) fn combine_retry_feedback(
    quality: Option<RetryFeedback>,
    previous: Option<&RetryFeedback>,
    error: Option<&str>,
) -> Option<RetryFeedback> {
    if quality.is_some() {
        return quality;
    }
    let Some(diagnostic) = script_error_feedback(error) else {
        return previous.cloned();
    };
    let Some(mut feedback) = previous.cloned() else {
        return Some(RetryFeedback::SelfCheck(diagnostic));
    };
    let reason = match &mut feedback {
        RetryFeedback::SelfCheck(reason)
        | RetryFeedback::Geometry(reason)
        | RetryFeedback::Completeness(reason)
        | RetryFeedback::Language(reason)
        | RetryFeedback::InitialBinding(reason) => reason,
    };
    let original_len = if reason.starts_with(SCRIPT_MARKER) {
        0
    } else {
        reason
            .find(&format!("\n\n{SCRIPT_MARKER}"))
            .unwrap_or(reason.len())
    };
    reason.truncate(original_len);
    if !reason.is_empty() {
        reason.push_str("\n\n");
    }
    reason.push_str(&diagnostic);
    Some(feedback)
}

/// The outcome boundary currently carries strings, including provider errors.
/// Recognize only runner diagnostics and reconstruct their safe explanation;
/// never copy a thrown value, provider response, or program envelope to a prompt.
fn script_error_feedback(error: Option<&str>) -> Option<String> {
    let error = error?;
    let runtime_reference = error
        .strip_prefix("script error: ")
        .and_then(|message| message.split_once(" is not defined"))
        .filter(|(_, suffix)| suffix.is_empty() || suffix.starts_with(". "))
        .and_then(|(name, _)| ScriptError::runtime_reference(name));
    let diagnostic = if let Some(runtime_reference) = runtime_reference {
        runtime_reference.to_string()
    } else if let Some(count) = error
        .strip_prefix("script response contains ")
        .and_then(|rest| rest.strip_suffix(" code blocks without a clear final program; return one complete JavaScript program"))
        .and_then(|count| count.parse::<usize>().ok())
    {
        ScriptError::AmbiguousCodeBlocks { count }.to_string()
    } else if error == "script is empty after stripping fences" {
        "script is empty after stripping fences. Emit executable JavaScript with I(parent, props) insertions.".into()
    } else if error == "script emitted no I(...), K(...), or U(...) operations" {
        "script emitted no I(...), K(...), or U(...) operations. Execute insertions; do not only define functions or return a JSON object.".into()
    } else if error.starts_with("script too large: ") {
        "script too large. Reduce repeated source with loops and local helpers while keeping the required content and behavior.".into()
    } else if error.starts_with("script error: ") {
        if error.contains(" is not defined") {
            "script error: undefined JavaScript variable. Declare local handles in this complete script. In fresh generation, I(parent, props) requires null or a parent handle returned in this script, never an existing-node id string. Use existing ids only in operations that explicitly support them, such as U. Do not reuse a variable from an earlier sandbox.".into()
        } else {
            "script error: JavaScript evaluation failed. Check syntax, quoting, variable declarations and supported I/K/U calls; fix the failing code before emitting it again.".into()
        }
    } else if error.starts_with("program produced no command:")
        || error.starts_with("unexpected batch_design outcome:")
        || error == "program command rejected by document"
        || error == "program produced no nodes"
    {
        "program failed to produce valid nodes. Use schema-supported node types and properties, and parent handles returned by I(...).".into()
    } else {
        return None;
    };
    let mut feedback = format!(
        "{SCRIPT_MARKER} {diagnostic}\n\
         Return one complete JavaScript program. Preserve the brief's requested behavior while \
         repairing the error: do not replace required events/state/bindings with static selected:true \
         styling, empty handlers, or no-op controls. Do not add interactions absent from the brief."
    );
    // Templates are short today; keep the boundary bounded if guidance grows.
    let mut end = feedback.len().min(MAX_SCRIPT_FEEDBACK_BYTES);
    while !feedback.is_char_boundary(end) {
        end -= 1;
    }
    feedback.truncate(end);
    Some(feedback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_templates_do_not_echo_exception_or_provider_payloads() {
        let private = "PRIVATE_RESPONSE_MARKER密".repeat(20_000);
        for prefix in ["script error: ", "program produced no command: "] {
            let feedback = script_error_feedback(Some(&format!("{prefix}{private}"))).unwrap();
            assert!(feedback.len() <= MAX_SCRIPT_FEEDBACK_BYTES);
            assert!(!feedback.contains("PRIVATE_RESPONSE_MARKER"));
            assert!(feedback.contains("one complete JavaScript program"));
        }
        for prefix in ["HTTP 401: ", "provider stream failed: ", "js runtime: "] {
            assert!(script_error_feedback(Some(&format!("{prefix}{private}"))).is_none());
        }
    }

    #[test]
    fn builtin_retry_feedback_reconstructs_the_shared_diagnostic_without_echoing_suffixes() {
        let diagnostic = ScriptError::runtime_reference("to_str")
            .unwrap()
            .to_string();
        for error in [
            "script error: to_str is not defined".to_string(),
            diagnostic,
            "script error: to_str is not defined. PRIVATE_EXCEPTION_BODY".to_string(),
        ] {
            let feedback = script_error_feedback(Some(&error)).unwrap();
            assert!(feedback.contains("registered Jian Preview builtin"));
            assert!(feedback.contains("quote the ENTIRE expression"));
            assert!(feedback.contains(r#"bindings: {content: "to_str($app.qty)"}"#));
            assert!(feedback.contains("do not replace required events/state/bindings"));
            assert!(feedback.len() <= MAX_SCRIPT_FEEDBACK_BYTES);
            assert!(!feedback.contains("PRIVATE_EXCEPTION_BODY"));
            assert!(!feedback.contains("Declare local handles"));
        }
        for error in [
            "script error: previousRow is not defined",
            "script error: to_str_fake is not defined. PRIVATE_EXCEPTION_BODY",
            "script error: to_str is not definedPRIVATE_EXCEPTION_BODY",
        ] {
            let feedback = script_error_feedback(Some(error)).unwrap();
            assert!(feedback.contains("undefined JavaScript variable"));
            assert!(!feedback.contains("registered Jian Preview builtin"));
            assert!(!feedback.contains("PRIVATE_EXCEPTION_BODY"));
        }
        for error in [
            "HTTP 500: to_str is not defined",
            "stream error: script error: to_str is not defined. PRIVATE_PROVIDER_BODY",
        ] {
            assert!(script_error_feedback(Some(error)).is_none());
        }
    }

    #[test]
    fn every_quality_variant_keeps_its_reason_and_only_the_latest_bounded_diagnostic() {
        let original = "Original quality requirement must survive.";
        for feedback in [
            RetryFeedback::SelfCheck(original.into()),
            RetryFeedback::Geometry(original.into()),
            RetryFeedback::Completeness(original.into()),
            RetryFeedback::Language(original.into()),
            RetryFeedback::InitialBinding(original.into()),
        ] {
            let mut combined = feedback.clone();
            for error in [
                "script is empty after stripping fences",
                "script error: bad syntax",
            ]
            .into_iter()
            .cycle()
            .take(10)
            {
                combined = combine_retry_feedback(None, Some(&combined), Some(error)).unwrap();
            }
            assert_eq!(
                std::mem::discriminant(&combined),
                std::mem::discriminant(&feedback)
            );
            let reason = match combined {
                RetryFeedback::SelfCheck(reason)
                | RetryFeedback::Geometry(reason)
                | RetryFeedback::Completeness(reason)
                | RetryFeedback::Language(reason)
                | RetryFeedback::InitialBinding(reason) => reason,
            };
            assert!(reason.starts_with(original));
            assert_eq!(reason.matches(SCRIPT_MARKER).count(), 1);
            assert!(reason.contains("JavaScript evaluation failed"));
            assert!(!reason.contains("script is empty"));
            assert!(reason.len() <= original.len() + 2 + MAX_SCRIPT_FEEDBACK_BYTES);
        }
    }

    #[test]
    fn new_quality_wins_and_transport_preserves_external_feedback_unchanged() {
        let previous = RetryFeedback::Language("Use the brief's language.".into());
        let quality = RetryFeedback::Completeness("All promised rows are required.".into());
        assert_eq!(
            combine_retry_feedback(
                Some(quality.clone()),
                Some(&previous),
                Some("script is empty after stripping fences")
            ),
            Some(quality)
        );
        assert_eq!(
            combine_retry_feedback(
                None,
                Some(&previous),
                Some("provider stream failed: PRIVATE")
            ),
            Some(previous)
        );
        let first =
            combine_retry_feedback(None, None, Some("script is empty after stripping fences"));
        let next = combine_retry_feedback(None, first.as_ref(), Some("script error: syntax"));
        assert!(
            matches!(next, Some(RetryFeedback::SelfCheck(reason)) if reason.starts_with(SCRIPT_MARKER) && reason.matches(SCRIPT_MARKER).count() == 1)
        );
    }
}
