//! Preserve actionable self-check feedback across the end-of-run salvage pass.

use crate::plan::{OrchestratorPlan, RetryFeedback, Subtask};
use crate::retry::{is_non_retryable, is_self_check_rejection};
use crate::types::{DocSink, SubtaskOutcome};
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use std::collections::HashSet;

pub(super) fn should_salvage(outcome: Option<&SubtaskOutcome>) -> bool {
    !outcome
        .and_then(|outcome| outcome.error.as_deref())
        .is_some_and(is_non_retryable)
}

pub(super) fn subtask_for_salvage(outcome: Option<&SubtaskOutcome>, fallback: &Subtask) -> Subtask {
    let mut subtask = outcome
        .and_then(|outcome| outcome.subtask.clone())
        .unwrap_or_else(|| fallback.clone());
    let latest_error = outcome.and_then(|outcome| outcome.error.as_deref());
    if latest_error.is_some_and(is_self_check_rejection) {
        subtask.retry_feedback = latest_error
            .map(str::to_string)
            .map(RetryFeedback::SelfCheck);
    }
    subtask
}

pub(super) fn finalize_failed_salvage(outcome: &mut SubtaskOutcome) -> String {
    let error = outcome
        .error
        .clone()
        .unwrap_or_else(|| "salvage attempt still empty".into());
    if is_self_check_rejection(&error) {
        if let Some(subtask) = outcome.subtask.as_mut() {
            subtask.retry_feedback = Some(RetryFeedback::SelfCheck(error.clone()));
        }
    }
    error
}

/// Where a salvaged section belongs among its current siblings: right
/// after the last sibling an EARLIER plan subtask produced (0 when none).
///
/// `InsertSubtree` only appends, so a section recovered by the salvage
/// pass lands after every section generated in the meantime — a hero
/// that failed twice on a flaky endpoint came back below the footer
/// (glm-5.3-flash, 2026-09-03). The plan order is the design order; the
/// salvage pass restores it.
pub(super) fn planned_child_index(
    plan_subtasks: &[Subtask],
    outcomes: &[SubtaskOutcome],
    subtask_index: usize,
    siblings: &[String],
) -> usize {
    let earlier: HashSet<&str> = plan_subtasks
        .iter()
        .take(subtask_index)
        .filter_map(|earlier| outcomes.iter().find(|outcome| outcome.id == earlier.id))
        .flat_map(|outcome| outcome.inserted_root_ids.iter().map(String::as_str))
        .collect();
    siblings
        .iter()
        .rposition(|id| earlier.contains(id.as_str()))
        .map_or(0, |position| position + 1)
}

/// The parent that currently holds `id` (None = page level) and that
/// parent's child ids in document order.
fn parent_and_siblings(children: &[PenNode], id: &str) -> Option<(Option<String>, Vec<String>)> {
    if children.iter().any(|node| node.id_str() == id) {
        return Some((
            None,
            children.iter().map(|n| n.id_str().to_owned()).collect(),
        ));
    }
    for node in children {
        let Some(kids) = node.children() else {
            continue;
        };
        if kids.iter().any(|kid| kid.id_str() == id) {
            return Some((
                Some(node.id_str().to_owned()),
                kids.iter().map(|n| n.id_str().to_owned()).collect(),
            ));
        }
        if let Some(found) = parent_and_siblings(kids, id) {
            return Some(found);
        }
    }
    None
}

/// Move the roots a successful salvage inserted back to their planned
/// position among their siblings. No-op when they already sit there.
pub(super) fn restore_planned_order(
    sink: &mut dyn DocSink,
    plan: &OrchestratorPlan,
    outcomes: &[SubtaskOutcome],
    subtask_index: usize,
    salvaged: &SubtaskOutcome,
) {
    let Some(first) = salvaged.inserted_root_ids.first() else {
        return;
    };
    let Some((parent, siblings)) = parent_and_siblings(sink.state().active_children(), first)
    else {
        return;
    };
    let mut index = planned_child_index(&plan.subtasks, outcomes, subtask_index, &siblings);
    let target_parent = parent.as_deref().map_or(NodeId::NONE, NodeId::new);
    for root in &salvaged.inserted_root_ids {
        if siblings.iter().position(|sibling| sibling == root) != Some(index) {
            sink.apply(EditorCommand::MoveNode {
                node_id: NodeId::new(root.clone()),
                target_parent: target_parent.clone(),
                page_id: None,
                index: Some(index),
            });
        }
        index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subtask(id: &str) -> Subtask {
        Subtask {
            id: id.into(),
            label: id.into(),
            region: crate::plan::Region {
                width: 1440.0,
                height: 400.0,
            },
            id_prefix: id.into(),
            parent_frame_id: None,
            elements: None,
            screen: None,
            generated_root_id: None,
            existing_section_labels: None,
            retry_feedback: None,
        }
    }

    fn outcome(id: &str, roots: &[&str]) -> SubtaskOutcome {
        SubtaskOutcome {
            id: id.into(),
            node_count: roots.len(),
            error: None,
            inserted_root_ids: roots.iter().map(|r| (*r).to_owned()).collect(),
            headline: None,
            subtask: None,
        }
    }

    /// A hero salvaged after nav/features/pricing/footer landed at the end
    /// belongs right after the nav — the last sibling an earlier plan
    /// subtask produced.
    #[test]
    fn a_salvaged_section_goes_back_after_its_planned_predecessor() {
        let plan = [
            subtask("nav"),
            subtask("hero"),
            subtask("features"),
            subtask("pricing"),
            subtask("footer"),
        ];
        let outcomes = [
            outcome("nav", &["n1"]),
            outcome("hero", &["n9"]),
            outcome("features", &["n2"]),
            outcome("pricing", &["n3"]),
            outcome("footer", &["n4"]),
        ];
        let siblings: Vec<String> = ["n1", "n2", "n3", "n4", "n9"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        assert_eq!(planned_child_index(&plan, &outcomes, 1, &siblings), 1);
        assert_eq!(
            planned_child_index(&plan, &outcomes, 0, &siblings),
            0,
            "the first plan subtask goes first"
        );
        // Salvages run in plan order, so by the time a later section is
        // restored the earlier ones already sit in place.
        let restored: Vec<String> = ["n1", "n9", "n2", "n3", "n4"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        assert_eq!(
            planned_child_index(&plan, &outcomes, 4, &restored),
            4,
            "a salvaged footer stays after pricing"
        );
    }
}
