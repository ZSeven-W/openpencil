//! First-party MCP debug tools.
//!
//! `debug_validation_report` restores, on the Rust side, the TS
//! `debug_validation_report` debug tool (`packages/pen-mcp/src/tools/
//! debug-validation-report.ts`). It runs `op_design_lint::detect_all`
//! over the active page of the open document and serializes the result.
//!
//! ## Deliberately narrower than the TS tool (spec §7)
//!
//! The TS tool accepts `filePath` / `pageId` / `rootNodeId` /
//! `categories` / `maxIssues`. This Rust tool ships a no-parameter
//! contract: it reports `detect_all` over the active page of the
//! currently-open document. The per-issue JSON shape (`Issue`) is
//! byte-identical to TS (`#[serde(rename_all = "camelCase")]`), so a
//! client reading individual issues is unaffected.
//!
//! ## Read-only + env-gated
//!
//! The tool emits no `EditorCommand` and mutates nothing — it only
//! runs `detect_all`. It is gated behind `OPENPENCIL_DEBUG_TOOLS=1`
//! (the existing debug-tool isolation env flag); when the flag is
//! unset the tool surfaces a clean `ToolFailed` error at call time.

use std::collections::BTreeMap;

use jian_ops_schema::node::PenNode;
use op_design_lint::{detect_all, Issue};
use op_editor_core::EditorState;

use super::{McpTool, ToolErrorCode, ToolOutcome};

/// The env flag that gates the debug tools — matches the isolation
/// flag the TS debug tooling uses (`OPENPENCIL_DEBUG_TOOLS=1`).
const DEBUG_TOOLS_ENV: &str = "OPENPENCIL_DEBUG_TOOLS";

/// Returns whether the debug tools are enabled (`OPENPENCIL_DEBUG_TOOLS=1`).
///
/// Public so the host MCP server can keep the debug tool out of the
/// production catalog entirely — both `rebuild_registry` and
/// `tools/list` consult this, so a client with the flag unset never
/// sees `debug_validation_report` at all (not just `ToolFailed` on call).
pub fn debug_tools_enabled() -> bool {
    std::env::var(DEBUG_TOOLS_ENV).is_ok_and(|v| v == "1")
}

/// First-party `debug_validation_report` tool — runs the
/// `op-design-lint` detectors over the active page and returns the
/// `Issue` list. Read-only: snapshots the active page at registration,
/// emits no command.
pub struct DebugValidationReport {
    /// The detected issues, snapshotted at registration time.
    pub issues: Vec<Issue>,
}

impl McpTool for DebugValidationReport {
    fn name(&self) -> &str {
        "debug_validation_report"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        if !debug_tools_enabled() {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("debug tools are disabled — set {DEBUG_TOOLS_ENV}=1 to enable"),
            );
        }
        // Per-category breakdown — `;`-separated `category|count`
        // records, mirroring `list_node_kinds` / `count_nodes`.
        let mut by_category: BTreeMap<String, u32> = BTreeMap::new();
        for issue in &self.issues {
            let key = serde_json::to_value(issue.category)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default();
            *by_category.entry(key).or_insert(0) += 1;
        }
        let categories: Vec<String> = by_category
            .iter()
            .map(|(cat, count)| format!("{cat}|{count}"))
            .collect();
        // The full issue list — serde keeps the `Issue` wire shape
        // byte-identical to the TS `debug_validation_report` output.
        let issues_json = match serde_json::to_string(&self.issues) {
            Ok(j) => j,
            Err(e) => {
                return ToolOutcome::Err(
                    ToolErrorCode::Internal,
                    format!("failed to serialize issues: {e}"),
                )
            }
        };
        let mut out = BTreeMap::new();
        out.insert("count".into(), self.issues.len().to_string());
        out.insert("categories".into(), categories.join(";"));
        out.insert("issues".into(), issues_json);
        ToolOutcome::Ok(out)
    }
}

/// Snapshot the active page + run `op_design_lint::detect_all` over
/// every top-level node of the active page.
///
/// `detect_all` walks a single root `PenNode`; a page can hold several
/// top-level nodes, so we run the detectors once per top-level node and
/// concatenate — the same multi-root discipline `apply_fixes` uses.
pub fn debug_validation_report_snapshot(state: &EditorState) -> DebugValidationReport {
    let roots: &[PenNode] = state.active_children();
    let mut issues = Vec::new();
    for root in roots {
        issues.extend(detect_all(root, &state.doc));
    }
    DebugValidationReport { issues }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{frame, state_with};
    use jian_ops_schema::node::PenNodeBase;

    /// Build a frame whose `base.rotation` is set — triggers the
    /// `unexpected-rotation` detector (UI nodes rarely tilt on purpose).
    fn rotated_frame() -> PenNode {
        let mut node = frame("f1", "tilted", 0.0, 0.0, 100.0, 100.0, vec![]);
        if let PenNode::Frame(f) = &mut node {
            f.base = PenNodeBase {
                rotation: Some(12.0),
                ..f.base.clone()
            };
        }
        node
    }

    /// One test covers the gate + the report shape: the three cases all
    /// flip the same process-wide env var, so they MUST run as a single
    /// `#[test]` — separate tests would race under cargo's parallel
    /// runner. The snapshot (`detect_all`) is env-independent; only the
    /// `call()` gate reads `OPENPENCIL_DEBUG_TOOLS`.
    #[test]
    fn report_respects_the_env_gate_and_serializes_issues() {
        let bad = debug_validation_report_snapshot(&state_with(vec![rotated_frame()]));
        let clean_frame = frame("f1", "clean", 0.0, 0.0, 100.0, 100.0, vec![]);
        let clean = debug_validation_report_snapshot(&state_with(vec![clean_frame]));
        let empty = BTreeMap::new();

        // Gate closed — the tool rejects regardless of document state.
        std::env::remove_var(DEBUG_TOOLS_ENV);
        match bad.call(&empty) {
            ToolOutcome::Err(code, msg) => {
                assert_eq!(code, ToolErrorCode::ToolFailed);
                assert!(msg.contains(DEBUG_TOOLS_ENV));
            }
            other => panic!("expected ToolFailed when gate is closed, got {other:?}"),
        }

        // Gate open — a known-bad document reports its issue.
        std::env::set_var(DEBUG_TOOLS_ENV, "1");
        match bad.call(&empty) {
            ToolOutcome::Ok(out) => {
                assert_eq!(out.get("count"), Some(&"1".to_string()));
                // Per-category breakdown carries the rotation detector.
                assert_eq!(
                    out.get("categories"),
                    Some(&"unexpected-rotation|1".to_string())
                );
                // The serialized issue list round-trips back to `Issue`.
                let issues_json = out.get("issues").expect("issues field");
                let issues: Vec<Issue> = serde_json::from_str(issues_json).unwrap();
                assert_eq!(issues.len(), 1);
                assert_eq!(issues[0].node_id, "f1");
            }
            other => panic!("expected Ok for the bad document, got {other:?}"),
        }

        // Gate open — a clean document reports zero issues.
        match clean.call(&empty) {
            ToolOutcome::Ok(out) => {
                assert_eq!(out.get("count"), Some(&"0".to_string()));
                assert_eq!(out.get("categories"), Some(&String::new()));
                assert_eq!(out.get("issues"), Some(&"[]".to_string()));
            }
            other => panic!("expected Ok for the clean document, got {other:?}"),
        }
        std::env::remove_var(DEBUG_TOOLS_ENV);
    }
}
