//! Carry the planner's explicit pinned-action intent into the generated tree.
use crate::{
    plan::{OrchestratorPlan, Subtask},
    types::DesignRequest,
};
use jian_ops_schema::node::{container::LayoutMode, PenNode};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use op_editor_core::PenNodeExt;

const MARKER: &str = "OP PINNED BOTTOM ACTION v1";

pub(crate) fn is_marked(node: &PenNode) -> bool {
    node.base()
        .explain
        .as_deref()
        .is_some_and(|explain| explain.lines().any(|line| line.trim() == MARKER))
}

pub(crate) fn preserve(
    nodes: &mut [PenNode],
    task: &Subtask,
    plan: &OrchestratorPlan,
    req: &DesignRequest,
) {
    if req.append_context.is_some()
        || req.continuation_context.is_some()
        || req.reference_skeleton.is_some()
        || !(280.0..=480.0).contains(&plan.root_frame.width)
        || !(500.0..=1000.0).contains(&plan.root_frame.height)
        || plan.root_frame.layout.as_deref() != Some("vertical")
        || !(44.0..=160.0).contains(&task.region.height)
        || !explicit_bottom_action(task)
        || !is_last_on_screen(task, plan)
    {
        return;
    }
    let [node] = nodes else { return };
    let PenNode::Frame(frame) = node else { return };
    if frame.base.x.is_some()
        || frame.base.y.is_some()
        || frame.base.constraints.is_some()
        || !matches!(
            frame.container.layout,
            Some(LayoutMode::Vertical | LayoutMode::Horizontal)
        )
        || !matches!(
            frame.container.width,
            Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
        )
        || !contains_action(node)
        || is_marked(node)
    {
        return;
    }
    let explain = node.base_mut().explain.get_or_insert_with(String::new);
    if !explain.is_empty() {
        explain.push('\n');
    }
    explain.push_str(MARKER);
}

fn contains_action(node: &PenNode) -> bool {
    matches!(node.base().role.as_deref(), Some("button" | "icon-button"))
        || crate::wire_screen_navigation::node_has_events(node)
        || node
            .children()
            .is_some_and(|children| children.iter().any(contains_action))
}

fn explicit_bottom_action(task: &Subtask) -> bool {
    let text = format!("{} {}", task.label, task.elements.as_deref().unwrap_or("")).to_lowercase();
    let has = |terms: &[&str]| terms.iter().any(|term| text.contains(term));
    !has(&[
        "not pinned",
        "not fixed",
        "not sticky",
        "不要固定",
        "不固定",
        "非固定",
    ]) && has(&["pinned", "fixed", "sticky", "固定", "吸底"])
        && has(&["bottom", "底部", "吸底"])
        && has(&["button", "cta", "action", "按钮", "操作"])
}

fn is_last_on_screen(task: &Subtask, plan: &OrchestratorPlan) -> bool {
    let Some(index) = plan
        .subtasks
        .iter()
        .position(|candidate| candidate.id == task.id)
    else {
        return false;
    };
    let groups = crate::screen_groups::group_subtasks_by_screen(&plan.subtasks);
    if groups.is_empty() {
        index + 1 == plan.subtasks.len()
    } else {
        groups
            .iter()
            .find(|group| group.indices.contains(&index))
            .is_some_and(|group| group.indices.last() == Some(&index))
    }
}

#[cfg(test)]
#[path = "mobile_action_bar_plan_tests.rs"]
mod tests;
