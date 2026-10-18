//! Original full-pipeline responses confused insert handles with node objects.

use super::*;
use op_mcp::script_runner::run_script_to_program;

#[test]
fn retained_handle_mutations_get_actionable_safe_retry_feedback() {
    for response in [
        include_str!("test_fixtures/product_handle_1018_attempt1.js"),
        include_str!("test_fixtures/product_handle_1018_attempt2.js"),
    ] {
        let error = run_script_to_program(response).unwrap_err().to_string();
        assert_eq!(error, "script error: not an object");
        let original = RetryFeedback::Completeness("Deliver every requested specification.".into());
        let combined = combine_retry_feedback(None, Some(&original), Some(&error)).unwrap();
        let RetryFeedback::Completeness(reason) = combined else {
            panic!("the original content rejection must keep its repair path");
        };
        assert!(reason.starts_with("Deliver every requested specification."));
        assert!(reason.contains("opaque binding strings"));
        assert!(reason.contains("separate local variable"));
        assert!(reason.contains("put events/state/bindings inside props"));
        assert!(reason.contains("do not replace required events/state/bindings"));
        assert!(reason.len() < MAX_SCRIPT_FEEDBACK_BYTES + 100);
        assert!(
            !reason.contains("轻氧"),
            "do not echo arbitrary response content"
        );
    }

    // An inline handler and an explicit parent handle preserve the interaction
    // without mutating a string. This proves protocol execution, not visual QA.
    let corrected = r#"
const section = I(null, {type:"frame", layout:"vertical"});
const row = I(section, {type:"frame", layout:"horizontal"});
for (const label of ["450ml", "600ml"]) {
  const option = I(row, {type:"frame", events:{onTap:[{set:{"$app.capacity":JSON.stringify(label)}}]}});
  I(option, {type:"text", content:label});
}
"#;
    let program = run_script_to_program(corrected).unwrap();
    assert_eq!(program.lines().count(), 6);
    assert!(program.contains(r#""onTap":[{"set":{"$app.capacity":"\"600ml\""}}]"#));
    assert!(program.contains(r#""content":"600ml""#));
}

#[test]
fn object_feedback_does_not_interpret_provider_text_or_copy_thrown_payloads() {
    let error = "script error: not an object";
    let feedback = script_error_feedback(Some(error)).unwrap();
    assert!(feedback.len() <= MAX_SCRIPT_FEEDBACK_BYTES);
    for other in [
        "HTTP 500: script error: not an object",
        "provider stream failed: script error: not an object",
    ] {
        assert!(script_error_feedback(Some(other)).is_none());
    }
    let arbitrary =
        script_error_feedback(Some("script error: not an object. PRIVATE_EXCEPTION_BODY")).unwrap();
    assert!(!arbitrary.contains("PRIVATE_EXCEPTION_BODY"));
    assert!(!arbitrary.contains("opaque binding strings"));
}
