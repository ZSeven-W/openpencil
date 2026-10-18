//! Locate a planned bottom action inside a generated whole mobile page.
use super::{is_marked, MARKER};
use crate::{
    image_fallback_policy::ResolvedRect,
    plan::{OrchestratorPlan, Subtask},
};
use jian_ops_schema::{
    node::{
        base::NumberOrExpression,
        container::{CornerRadius, JustifyContent, LayoutMode, Padding},
        PenNode,
    },
    sizing::{SizingBehavior, SizingKeyword},
};
use op_editor_core::{EditorState, PenNodeExt};
use std::collections::HashMap;

/// The caller owns fresh-generation/reference/append and mobile-plan gates.
/// Only an unambiguous descendant is marked; the whole-page root never is.
pub(crate) fn preserve(
    nodes: &mut [PenNode],
    task: &Subtask,
    plan: &OrchestratorPlan,
    context: &EditorState,
) -> bool {
    if (task.region.height - plan.root_frame.height).abs() > 1.0
        || (task.region.width - plan.root_frame.width).abs() > 1.0
        || task.region.height < 500.0
        || !explicit_rail(task)
    {
        return false;
    }
    let [root] = nodes else { return false };
    let Some(rects) = measure(root, plan, context) else {
        return false;
    };
    let Some(path) = candidate_path(root, plan.root_frame.width, &rects) else {
        return false;
    };
    let mut target = root;
    for index in path {
        let Some(children) = target.children_mut() else {
            return false;
        };
        let Some(child) = children.get_mut(index) else {
            return false;
        };
        target = child;
    }
    if !is_marked(target) {
        let explain = target.base_mut().explain.get_or_insert_with(String::new);
        if !explain.is_empty() {
            explain.push('\n');
        }
        explain.push_str(MARKER);
    }
    true
}

fn explicit_rail(task: &Subtask) -> bool {
    let text = format!(
        "{} {} {}",
        task.label,
        task.elements.as_deref().unwrap_or(""),
        task.covers.as_deref().unwrap_or_default().join(" ")
    )
    .to_lowercase();
    let has = |terms: &[&str]| terms.iter().any(|term| text.contains(term));
    if has(&[
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
        "no footer",
        "without footer",
        "without a footer",
        "no bottom",
        "without bottom",
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
        "无底部",
        "不要底部",
        "不含底部",
        "无页脚",
        "不要页脚",
        "不含页脚",
    ]) {
        return false;
    }
    has(&[
        "bottom action bar",
        "bottom-action-bar",
        "bottom purchase bar",
        "bottom checkout bar",
        "底部加购条",
        "底部结算条",
        "底部操作栏",
        "底部操作条",
        "底部购物车栏",
    ]) || super::explicit_bottom_action(task)
}

fn candidate_path(
    root: &PenNode,
    width: f64,
    rects: &HashMap<String, ResolvedRect>,
) -> Option<Vec<usize>> {
    let mut node = root;
    let mut expected_width = width;
    let mut path = Vec::new();
    let mut candidates = Vec::new();
    // A bounded walk prevents treating arbitrary nested cards as page shells.
    for depth in 0..4 {
        if !page_wrapper(node, depth == 0, expected_width, rects) {
            break;
        }
        let inner_width = rects.get(node.id_str())?.width - full_page_wrapper_gutter(node)? * 2.0;
        let children = node.children()?;
        let last = children.len().checked_sub(1)?;
        if action_group(&children[last], inner_width, rects) {
            // Adjacent trailing groups are ambiguous. An ordinary top
            // navigation action, separated by content, is not a footer.
            if (last > 0 && action_group(&children[last - 1], inner_width, rects))
                || !candidates.is_empty()
            {
                return None;
            }
            let mut candidate = path.clone();
            candidate.push(last);
            candidates.push(candidate);
        }
        path.push(last);
        node = &children[last];
        expected_width = inner_width;
    }
    (candidates.len() == 1).then(|| candidates.remove(0))
}

fn page_wrapper(
    node: &PenNode,
    is_root: bool,
    expected_width: f64,
    rects: &HashMap<String, ResolvedRect>,
) -> bool {
    is_safe_full_page_wrapper(node, is_root) && full_width(node, expected_width, rects)
}

/// Ordinary page shells only. Decoration belongs to cards or the footer
/// itself; lifting through those containers would discard authored surfaces.
pub(crate) fn is_safe_full_page_wrapper(node: &PenNode, allow_fixed_root: bool) -> bool {
    let PenNode::Frame(frame) = node else {
        return false;
    };
    let ordinary_height = matches!(
        frame.container.height,
        None | Some(SizingBehavior::Keyword(
            SizingKeyword::FitContent | SizingKeyword::FillContainer
        ))
    ) || (allow_fixed_root
        && matches!(frame.container.height,
            Some(SizingBehavior::Number(height)) if (500.0..=1000.0).contains(&height)));
    safe_node(node)
        && matches!(
            node.base().role.as_deref(),
            None | Some("content" | "main" | "app-content")
        )
        && frame.container.layout == Some(LayoutMode::Vertical)
        && matches!(
            frame.container.justify_content,
            None | Some(JustifyContent::Start)
        )
        && ordinary_height
        && (matches!(
            frame.container.width,
            Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
        ) || matches!(frame.container.width, Some(SizingBehavior::Number(width)) if width.is_finite() && width > 0.0))
        && full_page_wrapper_gutter(node).is_some()
        && numeric_gap(frame.container.gap.as_ref())
        && frame.container.stroke.is_none()
        && frame.container.effects.as_ref().is_none_or(Vec::is_empty)
        && match frame.container.corner_radius {
            None => true,
            Some(CornerRadius::Uniform(radius)) => radius == 0.0,
            Some(CornerRadius::PerCorner(radii)) => radii.iter().all(|radius| *radius == 0.0),
        }
        && !crate::wire_screen_navigation::node_has_events(node)
        && !has_geometry_bindings(node)
}

pub(crate) fn full_page_wrapper_gutter(node: &PenNode) -> Option<f64> {
    let PenNode::Frame(frame) = node else {
        return None;
    };
    let [top, right, bottom, left] = numeric_padding(frame.container.padding.as_ref())?;
    (top == 0.0 && bottom == 0.0 && left == right && left <= 32.0).then_some(left)
}

fn action_group(
    node: &PenNode,
    expected_width: f64,
    rects: &HashMap<String, ResolvedRect>,
) -> bool {
    let PenNode::Frame(frame) = node else {
        return false;
    };
    if !matches!(
        node.base().role.as_deref(),
        None | Some("bottom-action-bar" | "cart-bar" | "checkout-bar" | "submit-bar")
    ) || !matches!(
        frame.container.layout,
        Some(LayoutMode::Vertical | LayoutMode::Horizontal)
    ) || !full_width(node, expected_width, rects)
        || !safe_subtree(node)
        || !node
            .children()
            .is_some_and(|children| children.iter().any(real_action))
    {
        return false;
    }
    let Some(bounds) = rects.get(node.id_str()) else {
        return false;
    };
    (44.0..=128.0).contains(&bounds.height) && contained(node, bounds, rects)
}

fn real_action(node: &PenNode) -> bool {
    matches!(node.base().role.as_deref(), Some("button" | "icon-button"))
        || serde_json::to_value(node).ok().is_some_and(|value| {
            value
                .get("events")
                .and_then(|events| events.get("onTap"))
                .is_some_and(|tap| tap.as_array().is_some_and(|actions| !actions.is_empty()))
        })
}

fn safe_node(node: &PenNode) -> bool {
    let base = node.base();
    base.x.is_none()
        && base.y.is_none()
        && base.constraints.is_none()
        && base.pin.is_none()
        && base.visible != Some(false)
        && base.rotation.is_none_or(|rotation| rotation == 0.0)
        && match base.opacity {
            None => true,
            Some(NumberOrExpression::Number(value)) => value.is_finite() && value > 0.0,
            Some(NumberOrExpression::Expression(_)) => false,
        }
}

fn safe_subtree(node: &PenNode) -> bool {
    if !safe_node(node) {
        return false;
    }
    let Ok(value) = serde_json::to_value(node) else {
        return false;
    };
    if ["width", "height"].iter().any(|key| {
        value.get(key).is_some_and(|v| {
            !v.as_f64().is_some_and(|n| n.is_finite() && n >= 0.0)
                && !matches!(v.as_str(), Some("fit_content" | "fill_container"))
        })
    }) || value.get("padding").is_some_and(|v| {
        serde_json::from_value::<Padding>(v.clone())
            .ok()
            .is_none_or(|padding| numeric_padding(Some(&padding)).is_none())
    }) || value
        .get("gap")
        .is_some_and(|v| !v.as_f64().is_some_and(|n| n.is_finite() && n >= 0.0))
        || has_geometry_bindings(node)
    {
        return false;
    }
    node.children()
        .is_none_or(|children| children.iter().all(safe_subtree))
}

fn has_geometry_bindings(node: &PenNode) -> bool {
    serde_json::to_value(node).ok().is_some_and(|value| {
        value.get("bindings").is_some_and(|bindings| {
            [
                "width", "height", "padding", "gap", "visible", "x", "y", "rotation",
            ]
            .iter()
            .any(|key| bindings.get(key).is_some())
        })
    })
}

fn full_width(node: &PenNode, expected: f64, rects: &HashMap<String, ResolvedRect>) -> bool {
    let PenNode::Frame(frame) = node else {
        return false;
    };
    (matches!(
        frame.container.width,
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
    ) || matches!(frame.container.width, Some(SizingBehavior::Number(width)) if (width - expected).abs() <= 1.0))
        && rects
            .get(node.id_str())
            .is_some_and(|rect| (rect.width - expected).abs() <= 1.0)
}

fn numeric_gap(gap: Option<&NumberOrExpression>) -> bool {
    gap.is_none_or(|gap| matches!(gap, NumberOrExpression::Number(n) if n.is_finite() && *n >= 0.0))
}

fn numeric_padding(padding: Option<&Padding>) -> Option<[f64; 4]> {
    let values = match padding {
        None => [0.0; 4],
        Some(Padding::Uniform(n)) => [*n; 4],
        Some(Padding::XY([y, x])) => [*y, *x, *y, *x],
        Some(Padding::LtrB(values)) => *values,
        Some(Padding::Expression(_)) => return None,
    };
    values
        .iter()
        .all(|n| n.is_finite() && *n >= 0.0)
        .then_some(values)
}

fn contained(node: &PenNode, bar: &ResolvedRect, rects: &HashMap<String, ResolvedRect>) -> bool {
    let Some(rect) = rects.get(node.id_str()) else {
        return false;
    };
    [rect.x, rect.y, rect.width, rect.height]
        .iter()
        .all(|n| n.is_finite())
        && rect.x >= bar.x - 1.0
        && rect.y >= bar.y - 1.0
        && rect.x + rect.width <= bar.x + bar.width + 1.0
        && rect.y + rect.height <= bar.y + bar.height + 1.0
        && node
            .children()
            .is_none_or(|children| children.iter().all(|child| contained(child, bar, rects)))
}

fn measure(
    root: &PenNode,
    plan: &OrchestratorPlan,
    context: &EditorState,
) -> Option<HashMap<String, ResolvedRect>> {
    let parent = serde_json::from_value(serde_json::json!({
        "type":"frame", "id":"__full_page_action_measure", "layout":"vertical",
        "width":plan.root_frame.width, "height":plan.root_frame.height, "children":[root]
    }))
    .ok()?;
    let mut scratch = EditorState::new();
    scratch.doc.variables = context.doc.variables.clone();
    scratch.doc.themes = context.doc.themes.clone();
    scratch.ui.variables.active_theme = context.ui.variables.active_theme.clone();
    scratch.editor_ui.preserve_authored_geometry = context.editor_ui.preserve_authored_geometry;
    scratch.doc.children = vec![parent];
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&scratch);
    let page = scene.active_page()?;
    let mut rects = HashMap::new();
    fn collect(
        nodes: &[jian_scene::layout_scene::SceneNode],
        rects: &mut HashMap<String, ResolvedRect>,
    ) -> Option<()> {
        for node in nodes {
            // Own layout bounds, not aggregate bounds: clipped overflowing
            // children must not make their short parent appear safely bounded.
            let bounds = node.bounds;
            if rects
                .insert(
                    node.id.clone(),
                    ResolvedRect {
                        x: f64::from(bounds.origin.x),
                        y: f64::from(bounds.origin.y),
                        width: f64::from(bounds.size.x),
                        height: f64::from(bounds.size.y),
                    },
                )
                .is_some()
            {
                return None;
            }
            collect(&node.children, rects)?;
        }
        Some(())
    }
    collect(&page.children, &mut rects)?;
    Some(rects)
}

#[cfg(test)]
#[path = "mobile_action_bar_full_page_tests.rs"]
mod tests;
