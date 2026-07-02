//! `emit_elements` — the design loop's element-builder tool.
//!
//! ## Why this exists
//!
//! Data-proven (ab-v8/ab-v9): the orchestrator beats the bare agentic loop
//! on weak models (≈50% vs 0% on M3) NOT because of its planner/scaffold,
//! but because its MANIFEST mode lets the model emit high-level
//! `{"el":"<kind>"}` element lines that Rust element-builders expand into
//! role-tagged subtrees (`stat-card`, `profile-header`, …). The loop only had
//! raw `batch_design` (primitive frames/text, no element kinds, no roles) → 0%.
//!
//! `emit_elements` closes that gap by giving the loop the SAME element-builder
//! surface the orchestrator's MANIFEST path uses — without exposing the 188
//! `add_*_v0/v1` element tools (too heavy for a loop prompt's tool list).
//!
//! ## Reuse, not reimplementation
//!
//! The tool runs `op_orchestrator::manifest::parse_manifest` — the EXACT
//! assembler the orchestrator MANIFEST mode calls. `parse_manifest` invokes
//! `op_mcp::element_manifest::build_element` per `el` line (semantic builders +
//! the repairing argument layer + `el:"ref"` component instances), then
//! assembles the el-line forest by `in:` references into role-tagged
//! `PenNode`s. So `emit_elements`' output is byte-equivalent in shape to the
//! orchestrator's manifest path: same builders, same role tags, same
//! post-processing surface. The result rides `EditorCommand::InsertSubtree`,
//! the same command `batch_design` emits.
//!
//! ## Gating
//!
//! This tool is wired ONLY into the design agent loop's tool set
//! (`design_agent_tools::DESIGN_TOOLS`). It is NOT registered with the MCP
//! server (`TOOL_SCHEMAS` is untouched), so the external MCP surface and its
//! advertised catalog are unchanged.

use std::collections::BTreeMap;

use op_editor_core::{EditorCommand, NodeId};
use op_mcp::{McpTool, ToolErrorCode, ToolOutcome};
use serde_json::Value;

/// Loop-only tool name.
pub const EMIT_ELEMENTS_TOOL: &str = "emit_elements";

/// The `emit_elements` tool schema, kept self-contained here (NOT sourced from
/// `mcp_serve::schemas::TOOL_SCHEMAS`) precisely so the tool stays loop-only and
/// the MCP server's advertised catalog count is unaffected.
pub const EMIT_ELEMENTS_SCHEMA: &str = r#"{"name":"emit_elements","description":"PREFERRED design tool. Emit a high-level element manifest — a JSON array of element lines like {\"el\":\"stat_card\",\"label\":\"MRR\",\"value\":\"$48k\"} — and the host expands each into a polished, role-tagged subtree (stat-card, profile-header, nav-item, …) and inserts it. Use {\"el\":\"section\",\"role\":\"hero\",\"direction\":\"vertical\",\"gap\":16} as a 1-based container, then nest later lines into it with \"in\": <line number>. NEVER write ids; nesting is by \"in\" only. Prefer this over hand-building primitives with batch_design.","inputSchema":{"type":"object","properties":{"elements":{"type":"string","description":"JSON array of element-line objects. Each object has an \"el\" kind plus that kind's params; \"section\" lines are containers other lines nest under via \"in\". e.g. [{\"el\":\"section\",\"role\":\"stats\",\"direction\":\"horizontal\",\"gap\":16},{\"el\":\"stat_card\",\"in\":1,\"label\":\"MRR\",\"value\":\"$48k\",\"trend\":\"up\"}]"},"parent_id":{"type":"string","description":"Optional existing container node id to insert under; omit/empty/0/root = active page root."}},"required":["elements"]}}"#;

/// The `emit_elements` `McpTool`. Stateless: it depends only on its args (the
/// manifest assembler is pure), so it carries no snapshot — `parent_id` is a
/// caller-supplied existing node id, resolved by the host's `InsertSubtree`
/// apply against the live document.
pub struct EmitElements;

impl McpTool for EmitElements {
    fn name(&self) -> &str {
        EMIT_ELEMENTS_TOOL
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw) = args
            .get("elements")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "elements is required: a JSON array of element-line objects".into(),
            );
        };

        // Parse the array of el-line objects, then re-serialize each object on
        // its own line — `parse_manifest` scans balanced `{…}` spans, so a
        // newline-joined JSONL feed is what it expects.
        let value: Value = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(e) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("elements must be a JSON array of objects: {e}"),
                )
            }
        };
        let Value::Array(items) = value else {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "elements must be a JSON array (e.g. [{\"el\":\"stat_card\",...}])".into(),
            );
        };
        if items.is_empty() {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "elements must contain at least one element line".into(),
            );
        }
        let mut jsonl = String::new();
        for item in &items {
            if !item.is_object() {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    "every elements entry must be an object with an \"el\" kind".into(),
                );
            }
            // Compact form keeps each object on one line so `balanced_spans`
            // sees exactly one span per element line, matching the line-number
            // semantics `in:` references rely on.
            jsonl.push_str(&item.to_string());
            jsonl.push('\n');
        }

        // Run the SAME assembler the orchestrator MANIFEST path uses:
        // el lines → `op_mcp::element_manifest::build_element` per line →
        // role-tagged `PenNode` forest assembled by `in:` references.
        let Some(outcome) = op_orchestrator::manifest::parse_manifest(&jsonl) else {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "no element lines found: every entry needs an \"el\" kind (e.g. \"stat_card\", \"section\")".into(),
            );
        };
        if outcome.nodes.is_empty() {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!(
                    "element manifest produced no nodes ({} dropped). warnings: {}",
                    outcome.dropped_lines,
                    outcome.warnings.join("; ")
                ),
            );
        }

        let parent_id = parse_parent_id(args.get("parent_id"));
        let mut result = BTreeMap::new();
        result.insert("wrote".into(), "true".into());
        result.insert("count".into(), count_forest(&outcome.nodes).to_string());
        result.insert("elementLines".into(), outcome.element_lines.to_string());
        result.insert("rawNodeLines".into(), outcome.raw_node_lines.to_string());
        result.insert("droppedLines".into(), outcome.dropped_lines.to_string());
        if !outcome.warnings.is_empty() {
            // Surface the repair/degrade trail so weak-model misses stay
            // observable in the loop's tool-result stream.
            result.insert("warnings".into(), outcome.warnings.join("; "));
        }

        ToolOutcome::OkWithCommand(
            result,
            EditorCommand::InsertSubtree {
                nodes: outcome.nodes,
                parent_id,
                page_id: None,
            },
        )
    }
}

/// Resolve the optional `parent_id` arg: empty / `0` / `root` / `document` /
/// `null` all mean the active page root (`NodeId::NONE`).
fn parse_parent_id(raw: Option<&String>) -> NodeId {
    match raw.map(|s| s.trim()) {
        None | Some("") | Some("0") | Some("root") | Some("document") | Some("null") => {
            NodeId::NONE
        }
        Some(id) => NodeId::new(id),
    }
}

/// Count every node in a forest (subtree-inclusive).
fn count_forest(nodes: &[jian_ops_schema::node::PenNode]) -> usize {
    fn count_subtree(node: &jian_ops_schema::node::PenNode) -> usize {
        use op_editor_core::PenNodeExt;
        1 + node
            .children()
            .map(|c| c.iter().map(count_subtree).sum::<usize>())
            .unwrap_or(0)
    }
    nodes.iter().map(count_subtree).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use jian_ops_schema::node::PenNode;
    use op_editor_core::PenNodeExt;

    fn args(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    /// Recursively collect every node `role` (`base.role`) in a forest. The
    /// element builders stamp semantic roles the orchestrator's MANIFEST path
    /// relies on; `emit_elements` must produce the same.
    fn collect_roles(node: &PenNode, out: &mut Vec<String>) {
        if let Some(role) = node.base().role.as_deref() {
            if !role.is_empty() {
                out.push(role.to_string());
            }
        }
        if let Some(children) = node.children() {
            for child in children {
                collect_roles(child, out);
            }
        }
    }

    fn all_roles(nodes: &[PenNode]) -> Vec<String> {
        let mut out = Vec::new();
        for node in nodes {
            collect_roles(node, &mut out);
        }
        out
    }

    #[test]
    fn emit_elements_builds_role_tagged_nodes() {
        // A couple of el lines (the task's example) must expand into the SAME
        // role-tagged structure the orchestrator MANIFEST path produces —
        // proving the loop reuses the element builders, not raw primitives.
        let tool = EmitElements;
        let elements = r#"[{"el":"stat_card","label":"MRR","value":"$48.2k","trend":"up"},{"el":"stat_card","label":"Users","value":"12.4k"}]"#;
        let outcome = tool.call(&args(&[("elements", elements)]));
        let (result, command) = match outcome {
            ToolOutcome::OkWithCommand(result, command) => (result, command),
            other => panic!("expected OkWithCommand, got {other:?}"),
        };
        assert_eq!(result.get("elementLines").map(String::as_str), Some("2"));
        assert_eq!(result.get("droppedLines").map(String::as_str), Some("0"));

        let nodes = match command {
            EditorCommand::InsertSubtree { nodes, .. } => nodes,
            other => panic!("expected InsertSubtree, got {other:?}"),
        };
        assert_eq!(nodes.len(), 2, "two top-level stat cards");
        let roles = all_roles(&nodes);
        assert!(
            roles.iter().any(|r| r == "stat-card"),
            "stat-card role must be present (proves element-builder expansion), got {roles:?}"
        );
    }

    #[test]
    fn emit_elements_nests_under_a_section_via_in() {
        // `{"el":"section",...}` is line 1; later lines nest via `in:1`.
        let tool = EmitElements;
        let elements = r#"[{"el":"section","role":"stats","direction":"horizontal","gap":16},{"el":"stat_card","in":1,"label":"MRR","value":"$48k"},{"el":"stat_card","in":1,"label":"DAU","value":"3.1k"}]"#;
        let outcome = tool.call(&args(&[("elements", elements)]));
        let command = match outcome {
            ToolOutcome::OkWithCommand(_, command) => command,
            other => panic!("expected OkWithCommand, got {other:?}"),
        };
        let nodes = match command {
            EditorCommand::InsertSubtree { nodes, .. } => nodes,
            other => panic!("expected InsertSubtree, got {other:?}"),
        };
        assert_eq!(nodes.len(), 1, "one section root");
        let section_kids = nodes[0].children().map(|c| c.len()).unwrap_or(0);
        assert_eq!(section_kids, 2, "both stat cards nested in the section");
    }

    #[test]
    fn emit_elements_resolves_parent_id_to_active_root() {
        let tool = EmitElements;
        let elements = r#"[{"el":"badge","label":"New"}]"#;
        // Empty / sentinel parent_id values resolve to the active page root.
        for sentinel in ["", "0", "root", "document"] {
            let outcome = tool.call(&args(&[("elements", elements), ("parent_id", sentinel)]));
            match outcome {
                ToolOutcome::OkWithCommand(_, EditorCommand::InsertSubtree { parent_id, .. }) => {
                    assert_eq!(parent_id, NodeId::NONE, "{sentinel:?} → active page root");
                }
                other => panic!("expected InsertSubtree for {sentinel:?}, got {other:?}"),
            }
        }
        // A real id is preserved as the insert target.
        let outcome = tool.call(&args(&[("elements", elements), ("parent_id", "42")]));
        match outcome {
            ToolOutcome::OkWithCommand(_, EditorCommand::InsertSubtree { parent_id, .. }) => {
                assert_eq!(parent_id, NodeId::new("42"));
            }
            other => panic!("expected InsertSubtree, got {other:?}"),
        }
    }

    #[test]
    fn emit_elements_rejects_missing_or_malformed_elements() {
        let tool = EmitElements;
        assert!(matches!(
            tool.call(&BTreeMap::new()),
            ToolOutcome::Err(ToolErrorCode::MissingArgument, _)
        ));
        assert!(matches!(
            tool.call(&args(&[("elements", "not json")])),
            ToolOutcome::Err(ToolErrorCode::InvalidArgument, _)
        ));
        assert!(matches!(
            tool.call(&args(&[("elements", "{\"el\":\"badge\"}")])),
            ToolOutcome::Err(ToolErrorCode::InvalidArgument, _) // object, not array
        ));
        assert!(matches!(
            tool.call(&args(&[("elements", "[]")])),
            ToolOutcome::Err(ToolErrorCode::InvalidArgument, _)
        ));
    }

    #[test]
    fn emit_elements_schema_is_valid_json_with_required_elements() {
        let schema: Value =
            serde_json::from_str(EMIT_ELEMENTS_SCHEMA).expect("schema must be valid JSON");
        assert_eq!(schema["name"], "emit_elements");
        let required = schema["inputSchema"]["required"]
            .as_array()
            .expect("required array");
        assert!(required.iter().any(|v| v == "elements"));
    }
}
