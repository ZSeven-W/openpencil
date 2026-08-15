//! MCP tool `finalize_design` — run OpenPencil's post-generation repair
//! passes over the document from a plain MCP call.
//!
//! External models that drive the editor through the bare MCP `batch_design`
//! surface never get the deterministic cleanup/repair backstop the built-in
//! design agent applies after a generation. This tool closes that gap: it runs
//! the exact whole-root cleanup family (`op_orchestrator::cleanup::
//! run_cleanup_passes_with_summary` — the same driver the orchestrator and the
//! agentic loop's `apply_loop_finalize` route through) and reports the
//! `RepairSummary` those paths surface as their quality credential.
//!
//! ## How the mutation reaches the host
//!
//! MCP tools are snapshots: they cannot mutate the live `EditorState`, they
//! return `EditorCommand`s the host applies (and, in file-backed mode, saves).
//! The cleanup passes speak `DocSink`, so the tool drives them over a
//! [`RecordingDocSink`] — a borrowed-state sink modelled on
//! `op_orchestrator::loop_finalize::StateDocSink` that records every accepted
//! apply — and returns the recorded commands as ONE `EditorCommand::Batch`.
//! Replay is deterministic because the commands were generated against a
//! clone of the exact state the host will apply them to, so the repair count
//! the summary reports is precisely the edit count the batch lands.
//!
//! ## Idempotence
//!
//! The passes are idempotent; running the tool twice over an already-finalized
//! document lands (near) zero repairs on the second call.

use std::collections::BTreeMap;

use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, EditorState, NodeId, PenNodeExt};
use op_mcp::{McpTool, ToolErrorCode, ToolOutcome};
use op_orchestrator::repair_summary::RepairSummary;

/// Default canvas width when the document has no measurable top-level frame.
/// Mirrors the desktop/web default design width.
const DEFAULT_CANVAS_WIDTH: f64 = 1200.0;

/// The `finalize_design` tool: a snapshot of the document at registration
/// time plus nothing else — the cleanup runs against a clone inside `call`.
pub struct FinalizeDesignTool {
    state: EditorState,
}

/// Snapshot constructor, same shape as the other read/write tool snapshots.
pub fn finalize_design_snapshot(doc: &EditorState) -> FinalizeDesignTool {
    FinalizeDesignTool { state: doc.clone() }
}

impl McpTool for FinalizeDesignTool {
    fn name(&self) -> &str {
        "finalize_design"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let root_ids: Vec<String> = match parse_root_ids(args, &self.state) {
            Ok(ids) => ids,
            Err(message) => return ToolOutcome::Err(ToolErrorCode::InvalidArgument, message),
        };
        let canvas_width = self
            .state
            .active_children()
            .first()
            .and_then(PenNodeExt::width_px)
            .filter(|w| *w > 0.0)
            .unwrap_or(DEFAULT_CANVAS_WIDTH);
        // Minimal plan from the document itself — the same helper the agentic
        // loop uses (only root name + width/fill are read by the passes; see
        // `synthesize_plan` for the field contract).
        let plan = op_orchestrator::loop_finalize::synthesize_plan(
            self.state.active_children(),
            canvas_width,
        );

        let mut state = self.state.clone();
        let root_id_refs: Vec<&str> = root_ids.iter().map(String::as_str).collect();
        let mut summary = RepairSummary::default();
        let mut sink = RecordingDocSink {
            state: &mut state,
            commands: Vec::new(),
        };
        op_orchestrator::cleanup::run_cleanup_passes_with_summary(
            &mut sink,
            &plan,
            &root_id_refs,
            &mut summary,
        );
        // Take the recorded commands first so the sink's `&mut state` borrow
        // ends, then run the read-only advisory scan over the final document.
        let commands = sink.commands;
        // Post-cleanup echo-only structural advisories (DS P2-a item ③):
        // every parent node of the FINAL document runs the same
        // sibling-structure-drift detector the pre-insertion self-check
        // uses, and hits are reported — never repaired, never counted in
        // the repair tally, never written back as commands. The
        // model-in-the-loop fixes them via batch_design / update_node and
        // calls finalize_design again (see the schema description).
        let advisories = op_orchestrator::orchestration_self_check::collect_section_structure_drift(
            state.active_children(),
        );
        // DS P2-b item C: board-trailing-void findings ride the SAME
        // echo-only advisory channel — reported, never repaired, never
        // counted in the repair tally. A fixed Card/Deck board whose void is
        // still >= 25% after the cleanup passes (incl. the centre repair)
        // holds too little content for any repair to rescue; the advisory
        // tells the caller to add density.
        let void_advisories =
            op_orchestrator::board_trailing_void::collect_board_trailing_void(&state);
        let json = finalize_result_json(&summary, root_ids.len(), &advisories, &void_advisories);
        if commands.is_empty() {
            ToolOutcome::OkJson(json)
        } else {
            ToolOutcome::OkJsonWithCommand(json, EditorCommand::Batch { commands })
        }
    }
}

/// Minimal borrowed-state [`op_orchestrator::types::DocSink`] that records
/// every ACCEPTED apply so the whole cleanup run can be replayed by the host
/// as one atomic `Batch`. Modelled on `loop_finalize::StateDocSink` (which
/// applies straight through) and the orchestrator tests' `VecDocSink` (which
/// records); recording only accepted commands keeps the replay from ever
/// tripping the batch rollback on a deterministic clone-vs-live mismatch.
struct RecordingDocSink<'a> {
    state: &'a mut EditorState,
    commands: Vec<EditorCommand>,
}

impl op_orchestrator::types::DocSink for RecordingDocSink<'_> {
    fn state(&self) -> &EditorState {
        self.state
    }

    fn apply(&mut self, cmd: EditorCommand) -> bool {
        if self.state.apply(cmd.clone()) {
            self.commands.push(cmd);
            true
        } else {
            false
        }
    }

    fn insert_subtree_returning_root_ids(
        &mut self,
        nodes: Vec<PenNode>,
        parent_id: &NodeId,
    ) -> Option<Vec<String>> {
        // Record the command before the live apply (VecDocSink's order), but
        // keep it only when the insert was accepted.
        if let Some(ids) = self
            .state
            .insert_subtree_returning_root_ids(nodes.clone(), parent_id)
        {
            self.commands.push(EditorCommand::InsertSubtree {
                nodes,
                parent_id: parent_id.clone(),
                page_id: None,
            });
            Some(ids)
        } else {
            None
        }
    }

    fn begin_undo_batch(&mut self) {}

    fn end_undo_batch(&mut self) {}
}

/// `root_ids`: optional JSON array string, comma-separated string, or omitted
/// for the default ("every top-level frame on the active page"). Blank input
/// is treated as omitted.
fn parse_root_ids(
    args: &BTreeMap<String, String>,
    state: &EditorState,
) -> Result<Vec<String>, String> {
    let raw = args
        .get("root_ids")
        .map(|v| v.trim())
        .filter(|v| !v.is_empty());
    let Some(raw) = raw else {
        return Ok(default_root_ids(state));
    };
    let ids: Vec<String> = if raw.starts_with('[') {
        match serde_json::from_str::<serde_json::Value>(raw) {
            Ok(serde_json::Value::Array(items)) => items
                .into_iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect(),
            Ok(_) => {
                return Err("root_ids must be a JSON array of node id strings".to_string());
            }
            Err(_) => {
                return Err("root_ids must be a JSON array of node id strings".to_string());
            }
        }
    } else {
        raw.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
    };
    if ids.is_empty() {
        return Err("root_ids must name at least one node".to_string());
    }
    Ok(ids)
}

/// Every top-level frame on the active page — the tool's default scope.
fn default_root_ids(state: &EditorState) -> Vec<String> {
    state
        .active_children()
        .iter()
        .filter(|node| matches!(node, PenNode::Frame(_)))
        .map(|node| node.id_str().to_string())
        .collect()
}

/// The human-readable repair summary, rendered through the same
/// `quality_credential` surface the built-in agent loop shows its users, plus
/// the structured per-category tally so an MCP client can reason about it.
///
/// `advisories` (DS P2-a item ③) are the echo-only structure-drift findings
/// and `void_advisories` (DS P2-b item C) the board-trailing-void ones: both
/// informational, NOT part of the repair tally, and the document is never
/// changed because of them.
fn finalize_result_json(
    summary: &RepairSummary,
    roots: usize,
    advisories: &[op_orchestrator::orchestration_self_check::SectionStructureDriftAdvisory],
    void_advisories: &[op_orchestrator::board_trailing_void::BoardTrailingVoidAdvisory],
) -> String {
    let quality = crate::quality_credential::quality_summary_from_repairs(summary);
    let credential =
        crate::quality_credential::quality_credential_line_with_records(&quality, None)
            .unwrap_or_else(|| "nothing checked".to_string());
    let categories: Vec<serde_json::Value> = op_orchestrator::repair_summary::CheckCategory::ALL
        .iter()
        .filter_map(|category| {
            let checked = summary.checked().contains(category);
            if !checked {
                return None;
            }
            Some(serde_json::json!({
                "category": category.key(),
                "checked": true,
                "repairs": summary.repairs_for(*category),
            }))
        })
        .collect();
    let records: Vec<serde_json::Value> = summary
        .records()
        .iter()
        .map(|record| {
            serde_json::json!({
                "pass": record.pass,
                "category": record.category.key(),
                "nodeId": record.node_id,
                "nodeName": record.node_name,
                "detail": record.detail,
            })
        })
        .collect();
    let render = |code: &str, node_ids: &[String], message: &str| {
        serde_json::json!({
            "code": code,
            "nodeIds": node_ids,
            "message": message,
        })
    };
    let mut advisories_json: Vec<serde_json::Value> = advisories
        .iter()
        .map(|advisory| render(advisory.code, &advisory.node_ids, &advisory.message))
        .collect();
    advisories_json.extend(
        void_advisories
            .iter()
            .map(|advisory| render(advisory.code, &advisory.node_ids, &advisory.message)),
    );
    serde_json::json!({
        "roots": roots,
        "checkedCategories": categories,
        "repairs": summary.total_repairs(),
        "repairRecords": records,
        "advisories": advisories_json,
        "notes": summary.notes(),
        "summary": credential.trim().to_string(),
    })
    .to_string()
}
