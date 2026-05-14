// Copyright (c) 2026 Elias Bachaalany
// SPDX-License-Identifier: MIT

//! Tool definition utilities for the Copilot SDK.
//!
//! Provides convenience functions for defining tools with automatic
//! result normalization and error handling.

use crate::types::{Tool, ToolResultObject};
use serde_json::Value;

/// Normalize any result into a ToolResultObject.
///
/// - `None` / null → empty success
/// - `String` → success with text
/// - `ToolResultObject` (dict with resultType + textResultForLlm) → pass-through
/// - Everything else → JSON serialize
pub fn normalize_result(result: Value) -> ToolResultObject {
    match result {
        Value::Null => ToolResultObject {
            text_result_for_llm: String::new(),
            binary_results_for_llm: None,
            result_type: "success".to_string(),
            error: None,
            session_log: None,
            tool_telemetry: None,
        },
        Value::String(s) => ToolResultObject {
            text_result_for_llm: s,
            binary_results_for_llm: None,
            result_type: "success".to_string(),
            error: None,
            session_log: None,
            tool_telemetry: None,
        },
        Value::Object(ref map)
            if map.contains_key("resultType") && map.contains_key("textResultForLlm") =>
        {
            serde_json::from_value(result).unwrap_or_else(|_| ToolResultObject {
                text_result_for_llm: "Failed to parse tool result".to_string(),
                binary_results_for_llm: None,
                result_type: "failure".to_string(),
                error: None,
                session_log: None,
                tool_telemetry: None,
            })
        }
        other => ToolResultObject {
            text_result_for_llm: serde_json::to_string(&other).unwrap_or_default(),
            binary_results_for_llm: None,
            result_type: "success".to_string(),
            error: None,
            session_log: None,
            tool_telemetry: None,
        },
    }
}

/// Define a tool with metadata for registration on a session.
///
/// Returns a `Tool` struct with name, description, and parameters schema.
/// The handler must be registered separately on the session via
/// `session.register_tool_with_handler()`.
///
/// # Example
/// ```rust,no_run
/// use copilot_sdk::tools::define_tool;
/// use serde_json::json;
///
/// let tool = define_tool(
///     "my_tool",
///     "A description of my tool",
///     Some(json!({"type": "object", "properties": {"query": {"type": "string"}}})),
/// );
/// // Register on session: session.register_tool_with_handler(tool, Some(handler)).await;
/// ```
pub fn define_tool(name: &str, description: &str, parameters_schema: Option<Value>) -> Tool {
    Tool {
        name: name.to_string(),
        description: description.to_string(),
        parameters_schema: parameters_schema.unwrap_or(serde_json::json!({})),
        overrides_built_in_tool: false,
        skip_permission: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_normalize_null() {
        let result = normalize_result(Value::Null);
        assert_eq!(result.result_type, "success");
        assert_eq!(result.text_result_for_llm, "");
    }

    #[test]
    fn test_normalize_string() {
        let result = normalize_result(Value::String("hello".to_string()));
        assert_eq!(result.result_type, "success");
        assert_eq!(result.text_result_for_llm, "hello");
    }

    #[test]
    fn test_normalize_tool_result_passthrough() {
        let val = json!({
            "resultType": "success",
            "textResultForLlm": "tool output"
        });
        let result = normalize_result(val);
        assert_eq!(result.result_type, "success");
        assert_eq!(result.text_result_for_llm, "tool output");
    }

    #[test]
    fn test_normalize_other_value() {
        let val = json!({"key": "value"});
        let result = normalize_result(val);
        assert_eq!(result.result_type, "success");
        assert!(result.text_result_for_llm.contains("key"));
    }

    #[test]
    fn test_define_tool_basic() {
        let tool = define_tool("test_tool", "A test tool", None);
        assert_eq!(tool.name, "test_tool");
        assert_eq!(tool.description, "A test tool");
    }

    #[test]
    fn test_define_tool_with_schema() {
        let schema = json!({"type": "object", "properties": {"q": {"type": "string"}}});
        let tool = define_tool("search", "Search tool", Some(schema.clone()));
        assert_eq!(tool.name, "search");
        assert_eq!(tool.parameters_schema, schema);
    }
}
