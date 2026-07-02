//! MCP tool `spawn_agents` — request surface only (Phase 0).
//!
//! A design agent calls this to split a large design task into parallel
//! sub-tasks. Each config item carries a prompt, the container node(s) to
//! fill, and the styleguide + guideline NAMES to pass along (sub-agents
//! cannot search styleguides, so the parent resolves and passes them in).
//!
//! Phase 0: validates the config and returns the spawned-agent request
//! result (`{ spawned, agentIds }`). Actual parallel execution is wired
//! in Phase 3 (Task 3.1) when the sub-loop runtime exists.
//! The `enable_spawn_agents` flag / `ToolOutcome::SpawnAgents` emission
//! are therefore DEFERRED to Phase 3 — this tool ALWAYS registers and
//! always runs the request-surface path.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use super::{McpTool, ToolErrorCode, ToolOutcome};

/// Maximum number of sub-agents per call (Pencil recommends ≤ 8–10).
const MAX_AGENTS: usize = 12;

// ---------------------------------------------------------------------------
// Public types (consumed by Phase 3)
// ---------------------------------------------------------------------------

/// One spawned sub-agent specification — the caller's intent for one
/// parallel design subtask. Public so Phase 3 can consume it directly
/// after execution is wired.
#[derive(Debug, Clone, PartialEq)]
pub struct SpawnSpec {
    /// The design prompt for this sub-agent.
    pub prompt: String,
    /// Node ids of containers the sub-agent should fill.
    pub container_nodes: Vec<String>,
    /// Name of the styleguide to pass to the sub-agent.
    pub styleguide_name: String,
    /// Guideline topic names to pass to the sub-agent.
    pub guideline_names: Vec<String>,
}

// ---------------------------------------------------------------------------
// Parse helpers
// ---------------------------------------------------------------------------

/// Extract a required non-empty string field from a JSON object.
fn required_string(obj: &serde_json::Map<String, Value>, key: &str) -> Result<String, String> {
    match obj.get(key) {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(other) => Err(format!("{key} must be a string, got {other}")),
        None => Err(format!("{key} is required")),
    }
}

/// Extract an optional string-array field from a JSON object, defaulting to `[]`.
fn optional_string_array(
    obj: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Vec<String>, String> {
    match obj.get(key) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(arr)) => arr
            .iter()
            .enumerate()
            .map(|(i, v)| match v {
                Value::String(s) => Ok(s.clone()),
                other => Err(format!("{key}[{i}] must be a string, got {other}")),
            })
            .collect(),
        Some(other) => Err(format!("{key} must be an array, got {other}")),
    }
}

// ---------------------------------------------------------------------------
// Parse + validate
// ---------------------------------------------------------------------------

/// Parse and validate the `config` argument from the MCP args map.
///
/// Mirrors `batch_get::parse_patterns`: reads `args.get("config")` as a
/// JSON-string value and `serde_json::from_str`s it into a `Value` array.
/// Tests construct the args map directly (same convention as `batch_get_tests`):
/// `args.insert("config".into(), r#"[{"prompt":"...","styleguideName":"..."}]"#.into())`.
pub fn parse_spawn_config(args: &BTreeMap<String, String>) -> Result<Vec<SpawnSpec>, String> {
    let raw = args.get("config").map(String::as_str).unwrap_or("").trim();

    if raw.is_empty() {
        return Err("spawn_agents requires a non-empty config array".into());
    }

    let value: Value =
        serde_json::from_str(raw).map_err(|e| format!("config must be a JSON array: {e}"))?;

    let Value::Array(items) = value else {
        return Err("config must be a JSON array".into());
    };

    if items.is_empty() {
        return Err("spawn_agents requires a non-empty config array".into());
    }

    if items.len() > MAX_AGENTS {
        return Err(format!(
            "spawn_agents config exceeds the {MAX_AGENTS}-agent cap (got {}); Pencil recommends ≤ 8–10 agents per call",
            items.len()
        ));
    }

    let mut specs = Vec::with_capacity(items.len());
    for (i, item) in items.into_iter().enumerate() {
        let Value::Object(obj) = item else {
            return Err(format!("spawn_agents config[{i}] must be an object"));
        };

        let prompt = required_string(&obj, "prompt")
            .map_err(|e| format!("spawn_agents config[{i}]: {e}"))?;
        if prompt.trim().is_empty() {
            return Err(format!(
                "spawn_agents config[{i}]: prompt must be non-empty"
            ));
        }

        let styleguide_name = required_string(&obj, "styleguideName")
            .map_err(|e| format!("spawn_agents config[{i}]: {e}"))?;
        if styleguide_name.trim().is_empty() {
            return Err(format!(
                "spawn_agents config[{i}]: styleguideName must be non-empty (sub-agents cannot search styleguides — pass the name explicitly)"
            ));
        }

        let container_nodes = optional_string_array(&obj, "containerNodes")
            .map_err(|e| format!("spawn_agents config[{i}]: {e}"))?;

        let guideline_names = optional_string_array(&obj, "guidelineNames")
            .map_err(|e| format!("spawn_agents config[{i}]: {e}"))?;

        specs.push(SpawnSpec {
            prompt,
            container_nodes,
            styleguide_name,
            guideline_names,
        });
    }
    Ok(specs)
}

// ---------------------------------------------------------------------------
// Tool
// ---------------------------------------------------------------------------

/// Stateless MCP tool that validates and returns a spawn-agents request.
///
/// Phase 0 always registers this tool and returns the request result.
/// Actual parallel execution is deferred to Phase 3 (Task 3.1).
pub struct SpawnAgents;

impl McpTool for SpawnAgents {
    fn name(&self) -> &str {
        "spawn_agents"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let specs = match parse_spawn_config(args) {
            Ok(specs) => specs,
            Err(msg) => return ToolOutcome::Err(ToolErrorCode::InvalidArgument, msg),
        };
        let n = specs.len();
        let agent_ids: Vec<String> = (0..n).map(|i| format!("agent-{i}")).collect();
        match serde_json::to_string(&json!({ "spawned": n, "agentIds": agent_ids })) {
            Ok(json) => ToolOutcome::OkJson(json),
            Err(e) => ToolOutcome::Err(
                ToolErrorCode::Internal,
                format!("serialize spawn_agents result failed: {e}"),
            ),
        }
    }
}

/// Snapshot constructor — mirrors the `get_guidelines_snapshot` convention.
/// This tool is stateless; no snapshot of `EditorState` is needed.
pub fn spawn_agents_snapshot() -> SpawnAgents {
    SpawnAgents
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Build an args map with `config` set to a JSON string, mirroring
    /// the `batch_get_tests` convention of constructing the map directly.
    fn args_with_config(config_json: &str) -> BTreeMap<String, String> {
        let mut args = BTreeMap::new();
        args.insert("config".into(), config_json.into());
        args
    }

    // --- parse_spawn_config tests ---

    #[test]
    fn parse_valid_single_item_returns_one_spec_with_all_fields_populated() {
        let args = args_with_config(
            r#"[{"prompt":"Design the hero","containerNodes":["n1","n2"],"styleguideName":"brand","guidelineNames":["web-app","mobile"]}]"#,
        );
        let specs = parse_spawn_config(&args).expect("valid single item");
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].prompt, "Design the hero");
        assert_eq!(specs[0].container_nodes, vec!["n1", "n2"]);
        assert_eq!(specs[0].styleguide_name, "brand");
        assert_eq!(specs[0].guideline_names, vec!["web-app", "mobile"]);
    }

    #[test]
    fn parse_valid_two_item_config_returns_two_specs() {
        let args = args_with_config(
            r#"[
              {"prompt":"Hero section","styleguideName":"brand","containerNodes":["n10"],"guidelineNames":[]},
              {"prompt":"Footer section","styleguideName":"brand","containerNodes":["n20"],"guidelineNames":["web-app"]}
            ]"#,
        );
        let specs = parse_spawn_config(&args).expect("valid two items");
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].prompt, "Hero section");
        assert_eq!(specs[1].prompt, "Footer section");
    }

    #[test]
    fn parse_item_without_optional_arrays_defaults_to_empty_vecs() {
        // containerNodes and guidelineNames are optional (#[serde(default)]).
        let args = args_with_config(r#"[{"prompt":"Fill nav","styleguideName":"brand"}]"#);
        let specs = parse_spawn_config(&args).expect("optional arrays absent");
        assert!(specs[0].container_nodes.is_empty());
        assert!(specs[0].guideline_names.is_empty());
    }

    #[test]
    fn parse_empty_config_array_returns_invalid_argument_error() {
        let args = args_with_config("[]");
        let err = parse_spawn_config(&args).unwrap_err();
        assert!(
            err.contains("non-empty config array"),
            "expected non-empty message, got: {err}"
        );
    }

    #[test]
    fn parse_missing_config_arg_returns_invalid_argument_error() {
        let err = parse_spawn_config(&BTreeMap::new()).unwrap_err();
        assert!(
            err.contains("non-empty config array"),
            "expected non-empty message, got: {err}"
        );
    }

    #[test]
    fn parse_item_with_empty_prompt_returns_invalid_argument_error() {
        let args = args_with_config(r#"[{"prompt":"  ","styleguideName":"brand"}]"#);
        let err = parse_spawn_config(&args).unwrap_err();
        assert!(
            err.contains("prompt must be non-empty"),
            "expected prompt error, got: {err}"
        );
    }

    #[test]
    fn parse_item_missing_styleguide_name_returns_invalid_argument_error() {
        // JSON without styleguideName at all — serde deserialises to empty string
        // (no default), so the field is required.
        // We supply it as empty to trigger the trim check.
        let args = args_with_config(r#"[{"prompt":"Hero","styleguideName":""}]"#);
        let err = parse_spawn_config(&args).unwrap_err();
        assert!(
            err.contains("styleguideName must be non-empty"),
            "expected styleguide error, got: {err}"
        );
    }

    #[test]
    fn parse_over_cap_config_returns_invalid_argument_error() {
        // Build 13 items (> MAX_AGENTS = 12).
        let items: Vec<String> = (0..13)
            .map(|i| format!(r#"{{"prompt":"Task {i}","styleguideName":"brand"}}"#))
            .collect();
        let json = format!("[{}]", items.join(","));
        let args = args_with_config(&json);
        let err = parse_spawn_config(&args).unwrap_err();
        assert!(
            err.contains("12-agent cap"),
            "expected cap error, got: {err}"
        );
    }

    // --- McpTool::call tests ---

    #[test]
    fn call_valid_single_item_returns_ok_json_with_spawned_1_and_agent_0() {
        let tool = spawn_agents_snapshot();
        let args = args_with_config(
            r#"[{"prompt":"Hero","containerNodes":["n1"],"styleguideName":"brand","guidelineNames":["web-app"]}]"#,
        );
        match tool.call(&args) {
            ToolOutcome::OkJson(json) => {
                let v: serde_json::Value =
                    serde_json::from_str(&json).expect("OkJson is valid JSON");
                assert_eq!(v["spawned"], 1);
                assert_eq!(v["agentIds"], serde_json::json!(["agent-0"]));
            }
            other => panic!("expected OkJson, got {other:?}"),
        }
    }

    #[test]
    fn call_two_item_config_returns_spawned_2_and_correct_agent_ids() {
        let tool = spawn_agents_snapshot();
        let args = args_with_config(
            r#"[
              {"prompt":"Hero","styleguideName":"brand"},
              {"prompt":"Footer","styleguideName":"brand"}
            ]"#,
        );
        match tool.call(&args) {
            ToolOutcome::OkJson(json) => {
                let v: serde_json::Value =
                    serde_json::from_str(&json).expect("OkJson is valid JSON");
                assert_eq!(v["spawned"], 2);
                assert_eq!(v["agentIds"], serde_json::json!(["agent-0", "agent-1"]));
            }
            other => panic!("expected OkJson, got {other:?}"),
        }
    }

    #[test]
    fn call_empty_config_array_returns_invalid_argument() {
        let tool = spawn_agents_snapshot();
        let args = args_with_config("[]");
        match tool.call(&args) {
            ToolOutcome::Err(ToolErrorCode::InvalidArgument, msg) => {
                assert!(
                    msg.contains("non-empty config array"),
                    "error message: {msg}"
                );
            }
            other => panic!("expected InvalidArgument, got {other:?}"),
        }
    }

    #[test]
    fn call_item_with_empty_styleguide_name_returns_invalid_argument() {
        let tool = spawn_agents_snapshot();
        let args = args_with_config(r#"[{"prompt":"Hero","styleguideName":""}]"#);
        match tool.call(&args) {
            ToolOutcome::Err(ToolErrorCode::InvalidArgument, msg) => {
                assert!(
                    msg.contains("styleguideName must be non-empty"),
                    "error message: {msg}"
                );
            }
            other => panic!("expected InvalidArgument, got {other:?}"),
        }
    }
}
