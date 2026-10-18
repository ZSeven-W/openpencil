//! Carry the planner's explicit pinned-action intent into the generated tree.
use crate::{
    plan::{OrchestratorPlan, Subtask},
    types::DesignRequest,
};
use jian_ops_schema::node::{container::LayoutMode, PenNode};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use op_editor_core::{EditorState, PenNodeExt};

const MARKER: &str = "OP PINNED BOTTOM ACTION v1";

#[path = "mobile_action_bar_full_page.rs"]
mod full_page;
pub(crate) use full_page::preserve as preserve_full_page;
pub(crate) use full_page::{full_page_wrapper_gutter, is_safe_full_page_wrapper};

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
    context: &EditorState,
) {
    if req.append_context.is_some()
        || req.continuation_context.is_some()
        || req.reference_skeleton.is_some()
        || !(280.0..=480.0).contains(&plan.root_frame.width)
        || !(500.0..=1000.0).contains(&plan.root_frame.height)
        || plan.root_frame.layout.as_deref() != Some("vertical")
        || !is_last_on_screen(task, plan)
    {
        return;
    }
    if preserve_full_page(nodes, task, plan, context) || !explicit_bottom_action(task) {
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
        || !has_bounded_height(node, task, plan, context)
    {
        return;
    }
    let explain = node.base_mut().explain.get_or_insert_with(String::new);
    if !explain.is_empty() {
        explain.push('\n');
    }
    explain.push_str(MARKER);
}

fn has_bounded_height(
    node: &PenNode,
    task: &Subtask,
    plan: &OrchestratorPlan,
    context: &EditorState,
) -> bool {
    if task.region.height != 0.0 {
        return (44.0..=160.0).contains(&task.region.height);
    }
    let Ok(section) = serde_json::to_value(node) else {
        return false;
    };
    if has_unresolved_sizing(&section) {
        return false;
    }
    // Zero is an intrinsic section budget, not evidence that explicit fixed
    // intent was withdrawn. Measure at the real phone width with the document's
    // selected variable theme. The zero-budget fallback uses cleanup's stricter
    // 128px resolved-height ceiling; authored positive budgets stay unchanged.
    let mut scratch = EditorState::new();
    scratch.doc.variables = context.doc.variables.clone();
    scratch.doc.themes = context.doc.themes.clone();
    scratch.ui.variables.active_theme = context.ui.variables.active_theme.clone();
    scratch.editor_ui.preserve_authored_geometry = context.editor_ui.preserve_authored_geometry;
    let Ok(parent) = serde_json::from_value(serde_json::json!({
        "type":"frame", "id":"__pinned_action_measure",
        "width":plan.root_frame.width, "height":"fit_content",
        "layout":"vertical", "children":[section]
    })) else {
        return false;
    };
    scratch.doc.children = vec![parent];
    let rects = crate::image_fallback_policy::resolved_rects(&scratch);
    let Some(bar) = rects.get(node.id_str()) else {
        return false;
    };
    (44.0..=128.0).contains(&bar.height)
        // A short authored/clipped parent must not conceal a tall content
        // section. Every measured descendant must fit the same action rail.
        && rects.values().all(|rect| {
            rect.y >= bar.y - 1.0
                && rect.y + rect.height <= bar.y + bar.height + 1.0
        })
}

fn has_unresolved_sizing(node: &serde_json::Value) -> bool {
    // Canvas variable resolution supports padding/gap, but sizing expressions
    // still fall back to auto. Such a measurement cannot prove a bounded bar.
    ["width", "height"].iter().any(|key| {
        node.get(key)
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !matches!(value, "fit_content" | "fill_container"))
    }) || node
        .get("children")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|children| children.iter().any(has_unresolved_sizing))
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
        "not floating",
        "not a floating",
        "non-floating",
        "no floating",
        "scroll with content",
        "scrolls with content",
        "normal document flow",
        "不要固定",
        "不固定",
        "非固定",
        "不要悬浮",
        "不悬浮",
        "非悬浮",
        "不要浮动",
        "不浮动",
        "非浮动",
        "随内容滚动",
        "随正文滚动",
        "正常文档流",
        "不含操作",
    ]) && has(&[
        "pinned", "fixed", "sticky", "floating", "固定", "吸底", "悬浮", "浮动",
    ]) && has(&["bottom", "底部", "吸底"])
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

#[cfg(test)]
#[path = "mobile_action_bar_floating_tests.rs"]
mod floating_tests;

#[cfg(test)]
#[path = "mobile_action_bar_zero_height_tests.rs"]
mod zero_height_tests;
