//! The pre-insertion self-check gate of one subtask attempt, plus the
//! rejection-log subtree digest — split out of `subagent.rs` at the
//! 800-line cap.

use crate::orchestration_self_check::{
    auto_fix_fixable_issues, check_generated_nodes_for_prompt, IntentCheckMode, SelfCheckReport,
};
use jian_ops_schema::node::PenNode;
use serde_json::Value;
use std::collections::BTreeSet;

/// Run the self-check over the parsed forest, auto-fix what is fixable,
/// and decide whether the attempt may insert.
///
/// `Err(message)` carries the `self-check failed: …` reason the retry
/// ladder echoes back (`retry::is_self_check_rejection` matches the
/// prefix). Intent-class findings reject only in [`IntentCheckMode::Reject`];
/// in [`IntentCheckMode::Advisory`] (the ladder's last rung / salvage /
/// manual Retry) they are logged and the content is accepted — a subtask
/// has no summary channel of its own, so the log is the advisory channel
/// here (the same one `deck_echoes` use), and `finalize_design` re-reports
/// drift over the final document for MCP callers.
pub(crate) fn gate_generated_nodes(
    nodes: &mut [PenNode],
    canvas_width: f64,
    prompt: &str,
    subtask_id: &str,
    mode: IntentCheckMode,
) -> Result<(), String> {
    let mut report = check_generated_nodes_for_prompt(nodes, canvas_width, prompt);
    if report.rejects(mode) {
        let fixed = auto_fix_fixable_issues(nodes, canvas_width);
        report = check_generated_nodes_for_prompt(nodes, canvas_width, prompt);
        if report.rejects(mode) {
            let message = report.failure_message_for(mode);
            let subtree = rejected_subtree_digest(nodes, &report);
            tracing::warn!(
                subtask = %subtask_id,
                issues = %message,
                subtree = %subtree,
                "subagent self-check rejected generated nodes (unfixable after auto-fix)"
            );
            return Err(format!("self-check failed: {message}"));
        }
        if fixed {
            tracing::info!(
                subtask = %subtask_id,
                "subagent self-check auto-fixed fixable layout issues before insertion"
            );
        }
    }
    for advisory in report.advisory_lines(mode) {
        tracing::warn!(
            subtask = %subtask_id,
            advisory = %advisory,
            "subagent self-check intent finding accepted on the final attempt (advisory)"
        );
    }
    Ok(())
}

const SUBTREE_DIGEST_MAX_CHILDREN: usize = 12;

/// Node fields carried into a rejection-log digest (depth 2): the rejected
/// node itself plus its direct children.
fn subtree_digest(node: &Value) -> Value {
    let mut digest = serde_json::Map::new();
    for key in ["id", "type", "name", "x", "y", "width", "height", "layout"] {
        if let Some(value) = node.get(key).filter(|value| !value.is_null()) {
            digest.insert(key.to_string(), value.clone());
        }
    }
    Value::Object(digest)
}

/// One-line JSON summary of `node` plus its direct children (depth 2, at
/// most 12 children, `+N more` beyond) — the structural evidence that makes
/// a self-check rejection diagnosable from the log alone.
pub(crate) fn rejected_subtree_summary(node: &Value) -> String {
    let kids = node
        .get("children")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let mut items = vec![subtree_digest(node)];
    for child in kids.iter().take(SUBTREE_DIGEST_MAX_CHILDREN) {
        items.push(subtree_digest(child));
    }
    let extra = kids.len().saturating_sub(SUBTREE_DIGEST_MAX_CHILDREN);
    if extra > 0 {
        items.push(Value::String(format!("+{extra} more")));
    }
    Value::Array(items).to_string()
}

/// `rejected_subtree_summary` for every node the report names, joined into
/// one log field; falls back to the forest roots when it names none.
fn rejected_subtree_digest(nodes: &[PenNode], report: &SelfCheckReport) -> String {
    let ids: BTreeSet<&str> = report
        .issues
        .iter()
        .filter_map(|issue| issue.node_id.as_deref())
        .collect();
    let Ok(forest) = serde_json::to_value(nodes) else {
        return "unavailable".to_string();
    };
    let (mut roots, mut summaries): (Vec<&Value>, Vec<String>) = Default::default();
    collect_rejected_summaries(&forest, &ids, &mut roots, &mut summaries);
    if summaries.is_empty() {
        summaries.extend(roots.iter().map(|root| rejected_subtree_summary(root)));
    }
    if summaries.is_empty() {
        return "unavailable".into();
    }
    summaries.join("; ")
}

fn collect_rejected_summaries<'a>(
    node: &'a Value,
    ids: &BTreeSet<&str>,
    roots: &mut Vec<&'a Value>,
    summaries: &mut Vec<String>,
) {
    match node {
        Value::Array(forest) => {
            for root in forest {
                collect_rejected_summaries(root, ids, roots, summaries);
            }
        }
        Value::Object(_) => {
            roots.push(node);
            if node
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| ids.contains(id))
            {
                summaries.push(rejected_subtree_summary(node));
            }
            for child in node
                .get("children")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                collect_rejected_summaries(child, ids, roots, summaries);
            }
        }
        _ => {}
    }
}
