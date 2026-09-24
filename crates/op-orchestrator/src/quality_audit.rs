//! End-of-run audit behind the user-facing quality report: run the
//! deterministic detectors once more on the FINAL document and report what
//! is still wrong, so "待关注 N 处" is a count of real findings rather than
//! an assumption.
//!
//! The repair passes already reported what they fixed
//! (`Progress::QualityChecked`); this is the other half. It is read-only:
//! it never edits the document, it only looks.
//!
//! Two detector families run, both already used elsewhere in the pipeline:
//! - `op_design_lint::detect_all` per board — only categories the report
//!   shows a user topic for (`op_editor_core::quality_report::topic_for_lint`);
//!   code-shape and taste ("slop") findings are not the user's problem to
//!   act on. Severity is deliberately NOT a filter: `Info` there means
//!   "detect-only, no safe auto-fix" (text contrast is one), which is
//!   exactly what the user must look at.
//! - `unfilled_screens::detect_unfilled_screens` — screens the plan
//!   promised that never received content.

use op_editor_core::quality_report::{lint_topics, topic_for_lint};
use op_editor_core::{EditorState, NodeId, PenNodeExt, QualityItem, QualityTopic};

/// What the audit covered and what it still found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QualityAudit {
    /// Topics whose detectors ran on at least one board.
    pub audited_topics: Vec<QualityTopic>,
    /// Every finding, in board order.
    pub remaining: Vec<QualityItem>,
}

/// Audit `board_ids` (top-level frames on the active page) as they stand
/// now. Ids that are not a top-level frame are skipped; when none resolve
/// the audit covers nothing and says so (`audited_topics` empty).
pub fn audit_final_quality(state: &EditorState, board_ids: &[String]) -> QualityAudit {
    let boards: Vec<_> = state
        .active_children()
        .iter()
        .filter(|node| matches!(node, jian_ops_schema::node::PenNode::Frame(_)))
        .filter(|node| board_ids.iter().any(|id| id == node.id_str()))
        .collect();
    if boards.is_empty() {
        return QualityAudit::default();
    }
    let mut remaining = Vec::new();
    for board in &boards {
        for issue in op_design_lint::detect_all(board, &state.doc) {
            let Some(category) = lint_category_key(issue.category) else {
                continue;
            };
            let Some(topic) = topic_for_lint(&category) else {
                continue;
            };
            let node_name = op_editor_core::walkers::find_node(
                std::slice::from_ref(*board),
                &NodeId::new(issue.node_id.clone()),
            )
            .and_then(|node| node.base().name.clone());
            remaining.push(QualityItem {
                topic,
                source: category,
                node_id: Some(issue.node_id.clone()).filter(|id| !id.is_empty()),
                node_name,
                board_id: Some(board.id_str().to_string()),
                detail: issue.reason.clone(),
            });
        }
    }
    for screen in crate::unfilled_screens::detect_unfilled_screens(state) {
        if !board_ids.contains(&screen.node_id) {
            continue;
        }
        remaining.push(QualityItem {
            topic: QualityTopic::Completeness,
            source: "unfilled-screen".to_string(),
            node_id: Some(screen.node_id.clone()),
            node_name: Some(screen.name),
            board_id: Some(screen.node_id),
            detail: String::new(),
        });
    }
    let mut audited_topics = lint_topics();
    audited_topics.push(QualityTopic::Completeness);
    audited_topics.sort();
    audited_topics.dedup();
    QualityAudit {
        audited_topics,
        remaining,
    }
}

/// The lint category's wire key (`text-bg-contrast`) — the vocabulary the
/// topic table is written in.
fn lint_category_key(category: op_design_lint::IssueCategory) -> Option<String> {
    match serde_json::to_value(category) {
        Ok(serde_json::Value::String(key)) => Some(key),
        _ => None,
    }
}

#[cfg(test)]
#[path = "quality_audit_tests.rs"]
mod tests;
