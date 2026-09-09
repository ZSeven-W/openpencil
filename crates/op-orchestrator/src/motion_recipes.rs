//! Structural motion recipes for generated mobile and landing-page roots.

use crate::design_type::DesignForm;
use crate::types::DocSink;
use jian_ops_schema::motion::MotionPreference;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::PenFill;
use op_editor_core::fills::node_fills;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap};

const MAX_ANIMATED_NODES: usize = 24;
const HERO_DURATION_MS: u64 = 400;
const DISPLAY_DURATION_MS: u64 = 300;
const CARD_DURATION_MS: u64 = 320;
const INTERACTIVE_DURATION_MS: u64 = 150;
const DISPLAY_DELAY_MS: u64 = 120;
const CARD_STAGGER_MS: u64 = 60;
const CARD_MAX_DELAY_MS: u64 = 360;
const HERO_MEDIA_SCREEN_FRACTION: f64 = 0.25;

// Material 3 motion tokens: the recipe timings are the short/medium/long
// durations used by the M3 motion scheme, with the M3 emphasized and
// standard easing families named in the document declaration.

#[derive(Debug, Default)]
struct PatchPlan {
    fields: BTreeMap<String, Map<String, Value>>,
    animated_nodes: usize,
}

/// Apply structural motion recipes to one eligible generated root.
///
/// The pass only issues `PatchNodeData` commands. That keeps all edits on the
/// existing repair bookkeeping path and makes the pass geometry-neutral.
pub(super) fn apply(sink: &mut dyn DocSink, root_id: &str) -> usize {
    let Some(root) = super::find_root(sink.state(), root_id) else {
        return 0;
    };
    if !eligible_root(root, sink.state().doc.motion) || has_authored_motion(root) {
        return 0;
    }

    let root_width = root.width_px().unwrap_or(0.0);
    let first_screen_height = root.height_px().unwrap_or(800.0).max(1.0);
    let hero_id = find_hero_id(root, root_width, first_screen_height);
    let mut plan = PatchPlan::default();

    if let Some(hero_id) = hero_id.as_deref() {
        add_animation(
            &mut plan,
            hero_id,
            mount_fade_up(0, 16, HERO_DURATION_MS, "emphasizedDecelerate"),
        );
    }
    if let Some(display_id) = find_display_id(root, first_screen_height) {
        add_animation(
            &mut plan,
            &display_id,
            mount_fade(DISPLAY_DURATION_MS, DISPLAY_DELAY_MS, "standard"),
        );
    }

    let mut order = HashMap::new();
    let mut next_order = 0;
    collect_document_order(root, &mut order, &mut next_order);
    let hero_order = hero_id.as_deref().and_then(|id| order.get(id).copied());
    collect_card_family_patches(
        root,
        hero_id.as_deref(),
        hero_order,
        &order,
        &mut plan,
        true,
    );
    collect_interactive_patches(root, &mut plan, true);

    let planned_animated_nodes = plan.animated_nodes;
    let changed = plan
        .fields
        .into_iter()
        .filter(|(node_id, fields)| {
            sink.apply(EditorCommand::PatchNodeData {
                node_id: NodeId::new(node_id.clone()),
                patch_json: Value::Object(fields.clone()).to_string(),
                page_id: None,
            })
        })
        .count();
    tracing::debug!(
        pass = "motion-recipes",
        root_id,
        changed,
        animated_nodes = planned_animated_nodes,
        "structural motion recipe pass completed"
    );
    changed
}

fn eligible_root(root: &PenNode, motion: Option<MotionPreference>) -> bool {
    if motion == Some(MotionPreference::Reduced) {
        return false;
    }
    let Some(width) = root
        .width_px()
        .filter(|width| width.is_finite() && *width > 0.0)
    else {
        return false;
    };
    if !(width <= 480.0 || width >= 960.0) {
        return false;
    }
    let form = crate::design_type::classify_root_form_node(root);
    if matches!(form, DesignForm::Deck | DesignForm::Card) {
        return false;
    }
    !surface_marker(root, &["deck", "slide", "card", "component"])
}

fn surface_marker(node: &PenNode, markers: &[&str]) -> bool {
    let role = node.base().role.as_deref().unwrap_or_default();
    let name = node.base().name.as_deref().unwrap_or_default();
    markers.iter().any(|marker| {
        role.eq_ignore_ascii_case(marker) || name.to_ascii_lowercase().contains(marker)
    })
}

fn has_authored_motion(node: &PenNode) -> bool {
    let (transition, animations) = node.motion_declarations();
    transition.is_some()
        || animations.is_some()
        || node
            .children()
            .is_some_and(|children| children.iter().any(has_authored_motion))
}

fn find_hero_id(root: &PenNode, root_width: f64, first_screen_height: f64) -> Option<String> {
    let children = root.children()?;
    let screen_area = root_width * first_screen_height;
    children
        .iter()
        .filter(|child| !is_chrome(child))
        .find(|child| {
            child
                .base()
                .name
                .as_deref()
                .is_some_and(|name| name.ends_with(" (bleed)"))
        })
        .or_else(|| {
            children
                .iter()
                .filter(|child| !is_chrome(child))
                .find(|child| contains_large_media(child, screen_area))
        })
        .map(|node| node.id_str().to_string())
}

fn contains_large_media(section: &PenNode, screen_area: f64) -> bool {
    let threshold = screen_area * HERO_MEDIA_SCREEN_FRACTION;
    section.children().is_some_and(|children| {
        children.iter().any(|child| {
            let media = matches!(child, PenNode::Image(_)) || has_solid_fill(child);
            let (width, height) = (child.width_px(), child.height_px());
            let large_enough = width
                .zip(height)
                .is_some_and(|(width, height)| width * height >= threshold);
            (media && large_enough) || contains_large_media(child, screen_area)
        })
    })
}

fn has_solid_fill(node: &PenNode) -> bool {
    node_fills(node).is_some_and(|fills| fills.iter().any(|fill| matches!(fill, PenFill::Solid(_))))
}

fn find_display_id(root: &PenNode, first_screen_height: f64) -> Option<String> {
    if let PenNode::Text(text) = root {
        let y = root.base().y.unwrap_or(0.0);
        if text.font_size.is_some_and(|size| size >= 32.0)
            && y.is_finite()
            && y <= first_screen_height
        {
            return Some(root.id_str().to_string());
        }
    }
    root.children()
        .into_iter()
        .flatten()
        .filter(|child| !is_chrome(child))
        .find_map(|child| find_display_id(child, first_screen_height))
}

fn collect_document_order(node: &PenNode, order: &mut HashMap<String, usize>, next: &mut usize) {
    order.insert(node.id_str().to_string(), *next);
    *next += 1;
    for child in node.children().into_iter().flatten() {
        collect_document_order(child, order, next);
    }
}

fn collect_card_family_patches(
    node: &PenNode,
    hero_id: Option<&str>,
    hero_order: Option<usize>,
    order: &HashMap<String, usize>,
    plan: &mut PatchPlan,
    is_root: bool,
) {
    if (!is_root && is_excluded_family_subtree(node)) || hero_id == Some(node.id_str()) {
        return;
    }
    let Some(children) = node.children() else {
        return;
    };
    let mut families: BTreeMap<String, Vec<&PenNode>> = BTreeMap::new();
    for child in children {
        if child.base().visible == Some(false) || !matches!(child, PenNode::Frame(_)) {
            continue;
        }
        let signature = crate::cleanup::structural_signature(child);
        if signature.contains('[') {
            families.entry(signature).or_default().push(child);
        }
    }
    for family in families.into_values().filter(|family| family.len() >= 3) {
        if hero_order.is_some_and(|hero| {
            family
                .iter()
                .any(|member| order.get(member.id_str()).copied().unwrap_or(0) <= hero)
        }) {
            continue;
        }
        for (index, member) in family.into_iter().enumerate() {
            let delay = (CARD_STAGGER_MS * index as u64).min(CARD_MAX_DELAY_MS);
            add_animation(
                plan,
                member.id_str(),
                in_view_fade_up(delay, CARD_DURATION_MS),
            );
        }
    }
    for child in children {
        collect_card_family_patches(child, hero_id, hero_order, order, plan, false);
    }
}

fn collect_interactive_patches(node: &PenNode, plan: &mut PatchPlan, is_root: bool) {
    if !is_root && is_excluded_family_subtree(node) {
        return;
    }
    let role = node.base().role.as_deref().unwrap_or_default();
    if node.on_tap().is_some() || matches!(role, "button" | "primary-button" | "cta") {
        add_field(
            plan,
            node.id_str(),
            "transition",
            json!({
                "durationMs": INTERACTIVE_DURATION_MS,
                "easing": "standard",
                "properties": ["fill", "opacity", "scaleX", "scaleY"]
            }),
        );
    }
    for child in node.children().into_iter().flatten() {
        collect_interactive_patches(child, plan, false);
    }
}

fn is_excluded_family_subtree(node: &PenNode) -> bool {
    is_chrome(node)
        || surface_marker(node, &["keypad"])
        || serde_json::to_value(node)
            .ok()
            .as_ref()
            .and_then(super::category_grid_density::category_grid_rows)
            .is_some()
}

fn is_chrome(node: &PenNode) -> bool {
    super::is_status_bar(node) || super::is_bottom_nav_section(node)
}

fn add_animation(plan: &mut PatchPlan, node_id: &str, animation: Value) {
    if plan.animated_nodes >= MAX_ANIMATED_NODES {
        return;
    }
    add_field(plan, node_id, "animations", json!([animation]));
    plan.animated_nodes += 1;
}

fn add_field(plan: &mut PatchPlan, node_id: &str, field: &str, value: Value) {
    plan.fields
        .entry(node_id.to_string())
        .or_default()
        .insert(field.to_string(), value);
}

fn mount_fade_up(delay: u64, translate_y: i64, duration: u64, easing: &str) -> Value {
    json!({
        "trigger": "mount",
        "keyframes": [
            {"offset": 0, "values": {"opacity": 0, "translateY": translate_y}},
            {"offset": 1, "values": {"opacity": 1, "translateY": 0}}
        ],
        "durationMs": duration,
        "delayMs": delay,
        "easing": easing,
        "fillMode": "forwards"
    })
}

fn mount_fade(duration: u64, delay: u64, easing: &str) -> Value {
    json!({
        "trigger": "mount",
        "keyframes": [
            {"offset": 0, "values": {"opacity": 0}},
            {"offset": 1, "values": {"opacity": 1}}
        ],
        "durationMs": duration,
        "delayMs": delay,
        "easing": easing,
        "fillMode": "forwards"
    })
}

fn in_view_fade_up(delay: u64, duration: u64) -> Value {
    json!({
        "trigger": "inView",
        "keyframes": [
            {"offset": 0, "values": {"opacity": 0, "translateY": 12}},
            {"offset": 1, "values": {"opacity": 1, "translateY": 0}}
        ],
        "durationMs": duration,
        "delayMs": delay,
        "easing": "standard",
        "fillMode": "forwards",
        "once": true
    })
}
