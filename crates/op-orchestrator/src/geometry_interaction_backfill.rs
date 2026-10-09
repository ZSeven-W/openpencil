//! Shared facts for deterministic interaction backfill and its geometry echo.
//!
//! Back controls are recognized only from resolved geometry and dedicated icon
//! data (`iconFontName` / `iconId`). Card routing requires one unambiguous
//! detail-shaped screen and repeated image+text subtree shapes. Node names,
//! ids, and inferred intent never participate.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use jian_ops_schema::node::PenNode;

use super::geometry_bottom_gap::is_bottom_nav_shape;
use super::*;

const HEADER_STRIP_MAX_Y: f64 = 120.0;
const LEFT_STRIP_MAX_X: f64 = 120.0;
const MAX_BACK_CONTROL_SIZE: f64 = 56.0;
const SQUARE_EPS: f64 = 1.0;
const BOUNDS_EPS: f64 = 1.0;

#[derive(Clone, Debug, PartialEq, Eq)]
struct InteractionTarget {
    node_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct InteractionBackfillFacts {
    back_targets: Vec<InteractionTarget>,
    card_targets: Vec<InteractionTarget>,
    detail_route: Option<String>,
}

struct ScreenFact {
    id: String,
    route: String,
    value: Value,
    rect: Rect,
}

#[derive(Clone, Debug)]
struct BackShapeFact {
    node_id: String,
    can_wire: bool,
    has_pop: bool,
}

/// Persist every interaction that the shared fact scan proves unambiguous.
/// This is cleanup-only; preview's cloned-state fallback never calls it.
pub(crate) fn wire_interaction_backfill(sink: &mut dyn DocSink) {
    crate::table_filter_contract::wire(sink);
    let facts = interaction_backfill_facts(sink.state());
    for target in facts.back_targets {
        sink.apply(EditorCommand::PatchNodeData {
            node_id: NodeId::new(target.node_id),
            patch_json: r#"{"events":{"onTap":[{"pop":null}]}}"#.to_string(),
            page_id: None,
        });
    }
    if let Some(route) = facts.detail_route {
        for target in facts.card_targets {
            sink.apply(EditorCommand::PatchNodeData {
                node_id: NodeId::new(target.node_id),
                patch_json: crate::wire_screen_navigation::navigate_patch("push", &route),
                page_id: None,
            });
        }
    }
}

/// Shape-only strict back-control fact for compatibility consumers that must
/// recognize an already-wired detail screen without repeating M1's detector.
pub(crate) fn screen_has_back_control_shape(state: &EditorState, screen_id: &str) -> bool {
    let rects = resolved_rects(state);
    let Some(root) = op_editor_core::walkers::find_node(
        state.active_children(),
        &NodeId::new(screen_id.to_string()),
    ) else {
        return false;
    };
    let Some(root_rect) = rects.get(screen_id).copied() else {
        return false;
    };
    let Ok(value) = serde_json::to_value(root) else {
        return false;
    };
    let mut shapes = Vec::new();
    collect_back_shapes(&value, &root_rect, &rects, false, &mut shapes);
    !shapes.is_empty()
}

/// Whole-document interaction-gap echo. The repair and echo consume the exact
/// same [`interaction_backfill_facts`] result, so their eligibility cannot drift.
pub(super) fn push_interaction_backfill_diagnostics(
    state: &EditorState,
    root_ids: Option<&[String]>,
    out: &mut Vec<String>,
) {
    if out.len() >= MAX_DIAGNOSTICS {
        return;
    }
    let facts = interaction_backfill_facts(state);
    let allowed = root_ids.map(|ids| descendant_ids_for_roots(state, ids));
    let included = |target: &InteractionTarget| {
        allowed
            .as_ref()
            .is_none_or(|ids| ids.contains(&target.node_id))
    };
    let back_count = facts
        .back_targets
        .iter()
        .filter(|target| included(target))
        .count();
    if back_count > 0 {
        out.push(format!(
            "interaction-unwired-back: {back_count} non-entry top-left back control(s) have no \
             onTap; bind events.onTap to pop."
        ));
    }
    if out.len() >= MAX_DIAGNOSTICS {
        return;
    }
    let card_count = facts
        .card_targets
        .iter()
        .filter(|target| included(target))
        .count();
    if card_count > 0 {
        if let Some(route) = facts.detail_route {
            out.push(format!(
                "interaction-unwired-cards: {card_count} repeated image+text card(s) have no \
                 onTap, and the document has one detail route {route:?}; bind their onTap to \
                 push that exact route."
            ));
        }
    }
}

fn interaction_backfill_facts(state: &EditorState) -> InteractionBackfillFacts {
    let rects = resolved_rects(state);
    let screens = collect_screen_facts(state, &rects);
    if screens.is_empty() {
        return InteractionBackfillFacts::default();
    }

    let mut existing_push_routes = BTreeSet::new();
    for root in state.active_children() {
        if let Ok(value) = serde_json::to_value(root) {
            collect_push_routes(&value, &mut existing_push_routes);
        }
    }

    let mut back_targets = Vec::new();
    let mut detail_indices = Vec::new();
    for (index, screen) in screens.iter().enumerate() {
        let mut shapes = Vec::new();
        collect_back_shapes(&screen.value, &screen.rect, &rects, false, &mut shapes);
        if screen.route != "/" {
            let unreachable_single =
                screens.len() == 1 && !existing_push_routes.contains(screen.route.as_str());
            if !unreachable_single {
                back_targets.extend(shapes.iter().filter(|fact| fact.can_wire).map(|fact| {
                    InteractionTarget {
                        node_id: fact.node_id.clone(),
                    }
                }));
            }
            let has_trailing_bottom_nav = node_children(&screen.value)
                .last()
                .is_some_and(|last| is_bottom_nav_shape(last, &screen.rect, &rects));
            let has_return_control = shapes.iter().any(|shape| shape.can_wire || shape.has_pop);
            if !has_trailing_bottom_nav && has_return_control {
                detail_indices.push(index);
            }
        }
    }

    if detail_indices.len() != 1 {
        return InteractionBackfillFacts {
            back_targets,
            card_targets: Vec::new(),
            detail_route: None,
        };
    }

    let detail = &screens[detail_indices[0]];
    let mut card_targets = Vec::new();
    for screen in &screens {
        if screen.id == detail.id {
            continue;
        }
        collect_card_targets(&screen.value, false, &mut card_targets);
    }
    InteractionBackfillFacts {
        back_targets,
        card_targets,
        detail_route: Some(detail.route.clone()),
    }
}

fn collect_screen_facts(state: &EditorState, rects: &HashMap<String, Rect>) -> Vec<ScreenFact> {
    state
        .active_children()
        .iter()
        .filter_map(|node| {
            let PenNode::Frame(frame) = node else {
                return None;
            };
            let route = frame
                .screen
                .clone()
                .filter(|route| route.starts_with('/'))?;
            let rect = rects.get(&frame.base.id).copied()?;
            let value = serde_json::to_value(node).ok()?;
            Some(ScreenFact {
                id: frame.base.id.clone(),
                route,
                value,
                rect,
            })
        })
        .collect()
}

fn collect_back_shapes(
    node: &Value,
    root_rect: &Rect,
    rects: &HashMap<String, Rect>,
    ancestor_has_on_tap: bool,
    out: &mut Vec<BackShapeFact>,
) {
    let node_has_on_tap = has_on_tap(node);
    if is_back_control_shape(node, root_rect, rects) {
        if let Some(node_id) = node.get("id").and_then(Value::as_str) {
            out.push(BackShapeFact {
                node_id: node_id.to_string(),
                can_wire: !has_events(node) && !ancestor_has_on_tap && !subtree_has_on_tap(node),
                has_pop: !ancestor_has_on_tap
                    && !descendants_have_on_tap(node)
                    && has_exact_pop_action(node),
            });
        }
        return;
    }
    let descendant_ancestor_has_on_tap = ancestor_has_on_tap || node_has_on_tap;
    for child in node_children(node) {
        collect_back_shapes(child, root_rect, rects, descendant_ancestor_has_on_tap, out);
    }
}

fn is_back_control_shape(node: &Value, root_rect: &Rect, rects: &HashMap<String, Rect>) -> bool {
    if node.get("type").and_then(Value::as_str) != Some("frame") {
        return false;
    }
    let children = node_children(node);
    let [icon] = children.as_slice() else {
        return false;
    };
    if !is_back_icon_data(icon) {
        return false;
    }
    let Some(rect) = node
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| rects.get(id))
        .copied()
    else {
        return false;
    };
    if rect.w <= 0.0
        || rect.h <= 0.0
        || rect.w > MAX_BACK_CONTROL_SIZE
        || rect.h > MAX_BACK_CONTROL_SIZE
        || (rect.w - rect.h).abs() > SQUARE_EPS
    {
        return false;
    }
    let local_x = rect.x - root_rect.x;
    let local_y = rect.y - root_rect.y;
    local_x >= -BOUNDS_EPS
        && local_x + rect.w <= LEFT_STRIP_MAX_X + BOUNDS_EPS
        && (-BOUNDS_EPS..HEADER_STRIP_MAX_Y).contains(&local_y)
}

fn is_back_icon_data(icon: &Value) -> bool {
    let raw = match icon.get("type").and_then(Value::as_str) {
        Some("icon_font") => icon.get("iconFontName").and_then(Value::as_str),
        Some("path") => icon.get("iconId").and_then(Value::as_str),
        _ => None,
    };
    raw.and_then(|name| name.trim().rsplit(':').next())
        .map(str::to_ascii_lowercase)
        .is_some_and(|name| matches!(name.as_str(), "chevron-left" | "arrow-left"))
}

fn collect_card_targets(
    parent: &Value,
    ancestor_has_on_tap: bool,
    out: &mut Vec<InteractionTarget>,
) {
    let parent_chain_has_on_tap = ancestor_has_on_tap || has_on_tap(parent);
    let mut groups: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    if !parent_chain_has_on_tap {
        for child in node_children(parent) {
            if is_card_container(child)
                && !has_events(child)
                && !subtree_has_on_tap(child)
                && subtree_has_text(child)
                && subtree_has_image(child)
            {
                groups.entry(subtree_shape(child)).or_default().push(child);
            }
        }
    }

    let mut selected = HashSet::new();
    for group in groups.values().filter(|group| group.len() >= 2) {
        for card in group {
            if let Some(id) = card.get("id").and_then(Value::as_str) {
                selected.insert(id.to_string());
                out.push(InteractionTarget {
                    node_id: id.to_string(),
                });
            }
        }
    }

    for child in node_children(parent) {
        let selected_here = child
            .get("id")
            .and_then(Value::as_str)
            .is_some_and(|id| selected.contains(id));
        if !selected_here {
            collect_card_targets(child, parent_chain_has_on_tap, out);
        }
    }
}

fn is_card_container(value: &Value) -> bool {
    matches!(
        value.get("type").and_then(Value::as_str),
        Some("frame" | "group" | "rectangle")
    ) && !node_children(value).is_empty()
}

fn subtree_shape(value: &Value) -> String {
    let kind = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let child_shapes = node_children(value)
        .iter()
        .map(|child| subtree_shape(child))
        .collect::<Vec<_>>()
        .join(",");
    format!("{kind}[{child_shapes}]")
}

fn subtree_has_text(value: &Value) -> bool {
    value.get("type").and_then(Value::as_str) == Some("text")
        || node_children(value)
            .iter()
            .any(|child| subtree_has_text(child))
}

fn subtree_has_image(value: &Value) -> bool {
    let has_image_fill = value
        .get("fill")
        .and_then(Value::as_array)
        .is_some_and(|fills| {
            fills
                .iter()
                .any(|fill| fill.get("type").and_then(Value::as_str) == Some("image"))
        });
    value.get("type").and_then(Value::as_str) == Some("image")
        || value
            .get("imagePrompt")
            .and_then(Value::as_str)
            .is_some_and(|prompt| !prompt.trim().is_empty())
        || has_image_fill
        || node_children(value)
            .iter()
            .any(|child| subtree_has_image(child))
}

fn has_events(value: &Value) -> bool {
    value.get("events").is_some()
}

fn has_on_tap(value: &Value) -> bool {
    value
        .get("events")
        .and_then(|events| events.get("onTap"))
        .and_then(Value::as_array)
        .is_some_and(|actions| !actions.is_empty())
}

fn has_exact_pop_action(value: &Value) -> bool {
    let Some(actions) = value
        .get("events")
        .and_then(|events| events.get("onTap"))
        .and_then(Value::as_array)
    else {
        return false;
    };
    let [action] = actions.as_slice() else {
        return false;
    };
    action
        .as_object()
        .is_some_and(|action| action.len() == 1 && action.get("pop").is_some_and(Value::is_null))
}

fn descendants_have_on_tap(value: &Value) -> bool {
    node_children(value)
        .iter()
        .any(|child| subtree_has_on_tap(child))
}

fn subtree_has_on_tap(value: &Value) -> bool {
    has_on_tap(value)
        || node_children(value)
            .iter()
            .any(|child| subtree_has_on_tap(child))
}

fn collect_push_routes(value: &Value, out: &mut BTreeSet<String>) {
    if let Some(actions) = value
        .get("events")
        .and_then(|events| events.get("onTap"))
        .and_then(Value::as_array)
    {
        for raw in actions
            .iter()
            .filter_map(|action| action.get("push"))
            .filter_map(Value::as_str)
        {
            let decoded = serde_json::from_str::<String>(raw).unwrap_or_else(|_| raw.to_string());
            out.insert(decoded);
        }
    }
    for child in node_children(value) {
        collect_push_routes(child, out);
    }
}

fn descendant_ids_for_roots(state: &EditorState, root_ids: &[String]) -> HashSet<String> {
    let mut ids = HashSet::new();
    for root_id in root_ids {
        let Some(root) = op_editor_core::walkers::find_node(
            state.active_children(),
            &NodeId::new(root_id.clone()),
        ) else {
            continue;
        };
        if let Ok(value) = serde_json::to_value(root) {
            collect_ids(&value, &mut ids);
        }
    }
    ids
}

fn collect_ids(value: &Value, out: &mut HashSet<String>) {
    if let Some(id) = value.get("id").and_then(Value::as_str) {
        out.insert(id.to_string());
    }
    for child in node_children(value) {
        collect_ids(child, out);
    }
}

fn node_children(value: &Value) -> Vec<&Value> {
    value
        .get("children")
        .and_then(Value::as_array)
        .map(|children| children.iter().collect())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "geometry_interaction_backfill_tests.rs"]
mod tests;
