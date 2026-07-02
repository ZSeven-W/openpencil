//! MCP tool `get_guidelines` — returns product-design guideline text for a topic.
//!
//! Part of the Pencil-style agentic tool-loop (Phase 0 — purely additive). A
//! design agent calls this early in the loop to load the product-design
//! principles for the target surface before generating any nodes.

use std::collections::BTreeMap;

use op_ai_skills::guideline_for;

use super::{McpTool, ToolErrorCode, ToolOutcome};

/// Read tool that returns the product-design guidelines for `topic`.
///
/// Supported topics: `"web-app"`, `"mobile"`.
/// Unknown topics return the same error envelope shape as `get_style_guide`
/// on a bad argument — `ToolOutcome::Ok` with an `"error"` key in the result
/// map (not a JSON-RPC transport error).
pub struct GetGuidelines;

impl McpTool for GetGuidelines {
    fn name(&self) -> &str {
        "get_guidelines"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let topic = match args.get("topic").map(String::as_str) {
            Some(t) if !t.trim().is_empty() => t.trim(),
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::MissingArgument,
                    "topic is required (\"web-app\" or \"mobile\")".into(),
                )
            }
        };

        match guideline_for(topic) {
            Some(content) => {
                let mut out = BTreeMap::new();
                out.insert("topic".into(), topic.to_string());
                out.insert("content".into(), content);
                ToolOutcome::Ok(out)
            }
            None => {
                // Mirror the shape get_style_guide returns when no match is found:
                // ToolOutcome::Ok with an "error" key — not a transport-level error.
                let mut out = BTreeMap::new();
                out.insert(
                    "error".into(),
                    format!("Unknown topic: \"{topic}\". Supported topics: web-app, mobile."),
                );
                ToolOutcome::Ok(out)
            }
        }
    }
}

/// Snapshot constructor — mirrors `get_style_guide_snapshot` convention.
pub fn get_guidelines_snapshot() -> GetGuidelines {
    GetGuidelines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(topic: &str) -> ToolOutcome {
        let mut args = BTreeMap::new();
        args.insert("topic".into(), topic.into());
        get_guidelines_snapshot().call(&args)
    }

    #[test]
    fn web_app_returns_non_empty_content_with_purpose_first() {
        match call("web-app") {
            ToolOutcome::Ok(out) => {
                assert_eq!(out.get("topic").map(String::as_str), Some("web-app"));
                let content = out.get("content").expect("content field");
                assert!(!content.is_empty());
                // Verbatim phrase from product-principles.md.
                assert!(
                    content.contains("PURPOSE FIRST"),
                    "web-app guideline must contain PURPOSE FIRST: {content}"
                );
                // Verbatim phrase from design-principles.md.
                assert!(
                    content.contains("DESIGN CRAFT"),
                    "web-app guideline must contain DESIGN CRAFT: {content}"
                );
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn mobile_returns_three_section_architecture_content() {
        match call("mobile") {
            ToolOutcome::Ok(out) => {
                assert_eq!(out.get("topic").map(String::as_str), Some("mobile"));
                let content = out.get("content").expect("content field");
                assert!(!content.is_empty());
                // Verbatim phrase from mobile-app.md.
                assert!(
                    content.contains("THREE-SECTION ARCHITECTURE"),
                    "mobile guideline must contain THREE-SECTION ARCHITECTURE: {content}"
                );
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn unknown_topic_returns_ok_with_error_key_mirroring_style_guide_shape() {
        // get_style_guide returns ToolOutcome::Ok { "error": "..." } for unknown
        // inputs — not a transport-level ToolOutcome::Err. We must match that.
        match call("unknown-topic") {
            ToolOutcome::Ok(out) => {
                assert!(
                    out.contains_key("error"),
                    "unknown topic must return an error key in the result map: {out:?}"
                );
                assert!(
                    !out.contains_key("content"),
                    "unknown topic must not return content: {out:?}"
                );
                let msg = out.get("error").unwrap();
                assert!(
                    msg.contains("unknown-topic"),
                    "error message must name the bad topic: {msg}"
                );
            }
            other => panic!("expected Ok(error), got {other:?}"),
        }
    }

    #[test]
    fn missing_topic_arg_returns_err() {
        let out = get_guidelines_snapshot().call(&BTreeMap::new());
        assert!(
            matches!(out, ToolOutcome::Err(ToolErrorCode::MissingArgument, _)),
            "missing topic must return MissingArgument: {out:?}"
        );
    }
}
