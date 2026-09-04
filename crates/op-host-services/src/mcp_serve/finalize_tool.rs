//! MCP tool `finalize_design` — run OpenPencil's post-generation repair
//! passes over the document from a plain MCP call.
//!
//! External models that drive the editor through the bare MCP `batch_design`
//! surface never get the deterministic cleanup/repair backstop the built-in
//! design agent applies after a generation. This tool closes that gap: it runs
//! the exact App whole-document finalizer (`op_orchestrator::
//! record_loop_finalize_counted`) and reports the `RepairSummary` that path
//! surfaces as its quality credential.
//!
//! ## How the mutation reaches the host
//!
//! MCP tools are snapshots: they cannot mutate the live `EditorState`, they
//! return `EditorCommand`s the host applies (and, in file-backed mode, saves).
//! The orchestrator records sink-driven repairs and converts direct semantic
//! passes to same-id shallow patches in App order. It atomically replays and
//! compares the canonical document before this tool returns the proven command
//! sequence as ONE `EditorCommand::Batch`.
//!
//! ## Idempotence
//!
//! The passes are idempotent; running the tool twice over an already-finalized
//! document lands (near) zero repairs on the second call.
//!
//! ## Echo-only advisories
//!
//! - `section-structure-drift`: sibling nodes have inconsistent structure.
//! - `board-trailing-void`: a card/deck board has significant empty space.
//! - `board-format-drift`: a card board's aspect ratio drifted from 3:4/1:1.
//! - `shader-invalid`: a shader fill is invalid and will degrade to flat colour.
//! - `shader-budget`: a shader fill is expensive (exceeds GPU budget for the design form).

use std::collections::{BTreeMap, BTreeSet};

use op_design_lint::design_form::classify_root_form_node;
use op_design_lint::detectors::detect_shader_budget;
use op_editor_core::{EditorCommand, EditorState, PenNodeExt};
use op_mcp::{McpTool, ToolErrorCode, ToolOutcome};
use op_orchestrator::repair_summary::RepairSummary;

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
        let recorded = match op_orchestrator::record_loop_finalize_counted(&self.state) {
            Ok(recorded) => recorded,
            Err(error) => {
                return ToolOutcome::Err(
                    ToolErrorCode::Internal,
                    format!("finalize_design could not prove command replay parity: {error}"),
                )
            }
        };
        let op_orchestrator::RecordedLoopFinalize {
            state,
            summary,
            commands,
        } = recorded;
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
        // DS P2-d item ②: card format drift rides the same channel too. A
        // Card board whose authored aspect passed the 3:4 / 1:1 regular band
        // (e.g. the text-wrap reflow growing 1440 → 2116) is an
        // informational finding — whether long-form output is acceptable is
        // a product decision, so the advisory names the ratio and both
        // directions and repairs nothing.
        let format_drift = op_orchestrator::board_trailing_void::collect_board_format_drift(&state);
        // Shader-budget findings: GPU cost of shader fills (blocking if the
        // shader is invalid and degrades to a flat colour; informational if
        // just expensive). Each root form is classified once to determine the
        // budget tier, then all issues are partitioned by severity.
        let mut shader_blocking = Vec::new();
        let mut shader_informational = Vec::new();
        for root in state.active_children() {
            let form = classify_root_form_node(root);
            let issues = detect_shader_budget(root, form);
            for issue in issues {
                if issue.severity == op_design_lint::IssueSeverity::Warning {
                    shader_blocking.push(issue);
                } else {
                    shader_informational.push(issue);
                }
            }
        }
        let json = finalize_result_json(
            &summary,
            root_ids.len(),
            &advisories,
            &void_advisories,
            &format_drift,
            &shader_blocking,
            &shader_informational,
        );
        if commands.is_empty() {
            ToolOutcome::OkJson(json)
        } else {
            ToolOutcome::OkJsonWithCommand(json, EditorCommand::Batch { commands })
        }
    }
}

/// `root_ids`: optional JSON array string, comma-separated string, or omitted
/// for the required whole-document scope (every top-level root on the active
/// page). Blank input is treated as omitted. An explicit subset is rejected:
/// the App-equivalent finalizer is intentionally not a per-root transform.
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
    let requested: BTreeSet<String> = ids.into_iter().collect();
    let all_roots = default_root_ids(state);
    let all_root_set: BTreeSet<String> = all_roots.iter().cloned().collect();
    if requested.len() != all_roots.len() || requested != all_root_set {
        return Err(
            "root_ids must include every top-level root on the active page; the whole-document finalizer requires every active root"
                .to_string(),
        );
    }
    // Preserve active-page document order for reporting and deterministic
    // diagnostics. Duplicate explicit ids have already been harmlessly
    // collapsed by the set equality check above.
    Ok(all_roots)
}

/// Every top-level root on the active page — the tool's required scope.
fn default_root_ids(state: &EditorState) -> Vec<String> {
    state
        .active_children()
        .iter()
        .map(|node| node.id_str().to_string())
        .collect()
}

/// The human-readable repair summary, rendered through the same
/// `quality_credential` surface the built-in agent loop shows its users, plus
/// the structured per-category tally so an MCP client can reason about it.
///
/// `advisories` (DS P2-a item ③) are the echo-only structure-drift findings,
/// `void_advisories` (DS P2-b item C) the board-trailing-void ones,
/// `format_drift` (DS P2-d item ②) the card format-drift ones, and
/// `shader_blocking` / `shader_informational` are shader fill diagnostics:
/// the blocking ones (shader-invalid) are added to advisories and keep
/// `complete=false`; the informational ones (shader-budget) ride a separate
/// echo-only `informational` array that never affects `complete`. None are
/// edits and therefore never inflate the repair tally.
fn finalize_result_json(
    summary: &RepairSummary,
    roots: usize,
    advisories: &[op_orchestrator::orchestration_self_check::SectionStructureDriftAdvisory],
    void_advisories: &[op_orchestrator::board_trailing_void::BoardTrailingVoidAdvisory],
    format_drift: &[op_orchestrator::board_trailing_void::BoardFormatDriftAdvisory],
    shader_blocking: &[op_design_lint::Issue],
    shader_informational: &[op_design_lint::Issue],
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
    advisories_json.extend(
        format_drift
            .iter()
            .map(|advisory| render(advisory.code, &advisory.node_ids, &advisory.message)),
    );
    // Shader-invalid (blocking) advisories are appended BEFORE complete/count are
    // computed so they gate completion like other blocking advisories.
    advisories_json.extend(shader_blocking.iter().map(|issue| {
        render(
            "shader-invalid",
            std::slice::from_ref(&issue.node_id),
            &issue.reason,
        )
    }));
    let complete = advisories_json.is_empty();
    let blocking_advisory_count = advisories_json.len();
    // Shader-budget (informational) advisories ride a separate always-present
    // array that never affects complete or blockingAdvisoryCount.
    let informational_json: Vec<serde_json::Value> = shader_informational
        .iter()
        .map(|issue| {
            render(
                "shader-budget",
                std::slice::from_ref(&issue.node_id),
                &issue.reason,
            )
        })
        .collect();
    serde_json::json!({
        "roots": roots,
        "complete": complete,
        "blockingAdvisoryCount": blocking_advisory_count,
        "checkedCategories": categories,
        "repairs": summary.total_repairs(),
        "repairRecords": records,
        "advisories": advisories_json,
        "informational": informational_json,
        "notes": summary.notes(),
        "summary": credential.trim().to_string(),
    })
    .to_string()
}
