//! Structural motion recipes for generated mobile and landing-page roots.

use crate::design_type::DesignForm;
use crate::types::DocSink;
use jian_ops_schema::motion::{Easing, MotionPreference, MotionTrigger, NodeAnimation};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use jian_ops_schema::style::{PenEffect, PenFill};
use op_editor_core::fills::{node_effects, node_fills, node_stroke_width};
use op_editor_core::{EditorCommand, EditorState, NodeId, PenNodeExt};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap, HashSet};

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
    let resolved_sizes = resolved_sizes(sink.state());
    let hero_id = find_hero_id(root, root_width, first_screen_height);
    let mut plan = PatchPlan::default();
    clear_recipe_animations(root, &mut plan);

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
        &resolved_sizes,
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

// A previous run of this pass is safe to replace; unrelated authored motion
// still vetoes the recipe so the repair pass never overwrites user intent.
fn has_authored_motion(node: &PenNode) -> bool {
    let (transition, animations) = node.motion_declarations();
    let authored_transition =
        transition.is_some_and(|transition| !is_recipe_transition(transition));
    let authored_animation = animations.is_some_and(|animations| {
        animations
            .iter()
            .any(|animation| !is_recipe_animation(animation))
    });
    authored_transition
        || authored_animation
        || node
            .children()
            .is_some_and(|children| children.iter().any(has_authored_motion))
}

fn is_recipe_transition(transition: &jian_ops_schema::motion::Transition) -> bool {
    transition.duration_ms == INTERACTIVE_DURATION_MS
        && matches!(&transition.easing, Easing::Standard)
        && transition.properties.as_ref().is_some_and(|properties| {
            properties.len() == 4
                && properties
                    .iter()
                    .zip(["fill", "opacity", "scaleX", "scaleY"])
                    .all(|(actual, expected)| actual == expected)
        })
}

fn is_recipe_animation(animation: &NodeAnimation) -> bool {
    match animation.trigger {
        MotionTrigger::InView => {
            animation.duration_ms == CARD_DURATION_MS
                && matches!(&animation.easing, Easing::Standard)
        }
        MotionTrigger::Mount => {
            matches!(
                (animation.duration_ms, &animation.easing),
                (HERO_DURATION_MS, Easing::EmphasizedDecelerate)
                    | (DISPLAY_DURATION_MS, Easing::Standard)
            )
        }
    }
}

fn clear_recipe_animations(node: &PenNode, plan: &mut PatchPlan) {
    let (_, animations) = node.motion_declarations();
    if animations.is_some_and(|animations| {
        !animations.is_empty() && animations.iter().all(is_recipe_animation)
    }) {
        add_field(plan, node.id_str(), "animations", Value::Null);
    }
    for child in node.children().into_iter().flatten() {
        clear_recipe_animations(child, plan);
    }
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

fn resolved_sizes(state: &EditorState) -> HashMap<String, (f64, f64)> {
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
    let mut sizes = HashMap::new();
    if let Some(page) = scene.active_page() {
        collect_resolved_sizes(&page.children, &mut sizes);
    }
    sizes
}

fn collect_resolved_sizes(
    nodes: &[jian_scene::layout_scene::SceneNode],
    sizes: &mut HashMap<String, (f64, f64)>,
) {
    for node in nodes {
        let bounds = node.aggregate_bounds();
        sizes.insert(
            node.id.clone(),
            (f64::from(bounds.size.x), f64::from(bounds.size.y)),
        );
        collect_resolved_sizes(&node.children, sizes);
    }
}

fn resolved_size(
    node: &PenNode,
    resolved_sizes: &HashMap<String, (f64, f64)>,
) -> Option<(f64, f64)> {
    resolved_sizes
        .get(node.id_str())
        .copied()
        .filter(|(width, height)| {
            width.is_finite() && height.is_finite() && *width > 0.0 && *height > 0.0
        })
}

fn collect_card_family_patches(
    node: &PenNode,
    hero_id: Option<&str>,
    hero_order: Option<usize>,
    order: &HashMap<String, usize>,
    resolved_sizes: &HashMap<String, (f64, f64)>,
    plan: &mut PatchPlan,
    is_root: bool,
) {
    if (!is_root && is_excluded_family_subtree(node)) || hero_id == Some(node.id_str()) {
        return;
    }
    let Some(children) = node.children() else {
        return;
    };
    let parent_width = resolved_size(node, resolved_sizes)
        .map(|(width, _)| width)
        .or_else(|| node.width_px());
    let families = shell_families(children, parent_width, resolved_sizes);
    let families = if families.is_empty() {
        structural_families(children, resolved_sizes)
    } else {
        families
    };
    let mut animated_members = HashSet::new();
    for family in families {
        if hero_order.is_some_and(|hero| {
            family
                .iter()
                .any(|member| order.get(member.id_str()).copied().unwrap_or(0) <= hero)
        }) {
            continue;
        }
        for (index, member) in family.into_iter().enumerate() {
            animated_members.insert(member.id_str().to_string());
            let delay = (CARD_STAGGER_MS * index as u64).min(CARD_MAX_DELAY_MS);
            add_animation(
                plan,
                member.id_str(),
                in_view_fade_up(delay, CARD_DURATION_MS),
            );
            if is_tappable_card(member) {
                add_transition(plan, member.id_str());
            }
        }
    }
    for child in children {
        if animated_members.contains(child.id_str()) {
            continue;
        }
        collect_card_family_patches(
            child,
            hero_id,
            hero_order,
            order,
            resolved_sizes,
            plan,
            false,
        );
    }
}

/// Find shell families without looking through the preview subtree. A shell
/// is deliberately judged by its own paint surface and resolved size, so a
/// card can still join when its preview has a different tree shape.
fn shell_families<'a>(
    children: &'a [PenNode],
    parent_width: Option<f64>,
    resolved_sizes: &HashMap<String, (f64, f64)>,
) -> Vec<Vec<&'a PenNode>> {
    let mut families: Vec<Vec<&PenNode>> = Vec::new();
    for child in children
        .iter()
        .filter(|child| is_shell_candidate(child, parent_width, resolved_sizes))
    {
        if let Some(family) = families
            .iter_mut()
            .find(|family| shell_sizes_match(family[0], child, parent_width, resolved_sizes))
        {
            family.push(child);
        } else {
            families.push(vec![child]);
        }
    }
    families.retain(|family| family.len() >= 3);
    families
}

fn structural_families<'a>(
    children: &'a [PenNode],
    resolved_sizes: &HashMap<String, (f64, f64)>,
) -> Vec<Vec<&'a PenNode>> {
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
    families
        .into_values()
        .filter(|family| {
            family.len() >= 3
                && family
                    .iter()
                    .all(|node| has_resolved_card_size(node, resolved_sizes))
        })
        .collect()
}

fn is_shell_candidate(
    node: &PenNode,
    parent_width: Option<f64>,
    resolved_sizes: &HashMap<String, (f64, f64)>,
) -> bool {
    node.base().visible != Some(false)
        && matches!(node, PenNode::Frame(_))
        && has_visible_shell_surface(node)
        && shell_size(node, parent_width, resolved_sizes).is_some()
}

fn has_visible_shell_surface(node: &PenNode) -> bool {
    node_fills(node).is_some_and(|fills| !fills.is_empty())
        || node_stroke_width(node).is_some_and(|width| width.is_finite() && width > 0.0)
        || node_effects(node).iter().any(|effect| match effect {
            PenEffect::Shadow(shadow) => shadow.visible != Some(false),
            _ => false,
        })
}

fn has_resolved_card_size(node: &PenNode, resolved_sizes: &HashMap<String, (f64, f64)>) -> bool {
    resolved_size(node, resolved_sizes)
        .or_else(|| node.width_px().zip(node.height_px()))
        .is_some_and(|(width, height)| {
            width.is_finite() && height.is_finite() && width >= 64.0 && height >= 64.0
        })
}

#[derive(Clone)]
enum ShellSize {
    Numeric { width: f64, height: f64 },
    FillWidth { height: HeightPolicy },
}

#[derive(Clone)]
enum HeightPolicy {
    Number(f64),
    Keyword(SizingKeyword),
}

fn shell_size(
    node: &PenNode,
    parent_width: Option<f64>,
    resolved_sizes: &HashMap<String, (f64, f64)>,
) -> Option<ShellSize> {
    let PenNode::Frame(frame) = node else {
        return None;
    };
    if let Some((width, height)) = resolved_size(node, resolved_sizes) {
        if width >= 64.0 && height >= 64.0 {
            return Some(ShellSize::Numeric { width, height });
        }
        return None;
    }
    let width = node.width_px();
    let height = node.height_px();
    if let (Some(width), Some(height)) = (width, height) {
        if width.is_finite() && height.is_finite() && width >= 64.0 && height >= 64.0 {
            return Some(ShellSize::Numeric { width, height });
        }
        return None;
    }
    if !matches!(
        frame.container.width.as_ref(),
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
    ) || !parent_width.is_some_and(|width| width.is_finite() && width >= 64.0)
    {
        return None;
    }
    let height = match frame.container.height.as_ref()? {
        SizingBehavior::Number(height) if height.is_finite() && *height >= 64.0 => {
            HeightPolicy::Number(*height)
        }
        SizingBehavior::Keyword(policy)
            if frame
                .container
                .limits
                .min_height
                .is_some_and(|height| height >= 64.0) =>
        {
            HeightPolicy::Keyword(policy.clone())
        }
        _ => return None,
    };
    Some(ShellSize::FillWidth { height })
}

fn shell_sizes_match(
    first: &PenNode,
    candidate: &PenNode,
    parent_width: Option<f64>,
    resolved_sizes: &HashMap<String, (f64, f64)>,
) -> bool {
    match (
        shell_size(first, parent_width, resolved_sizes),
        shell_size(candidate, parent_width, resolved_sizes),
    ) {
        (
            Some(ShellSize::Numeric {
                width: first_width,
                height: first_height,
            }),
            Some(ShellSize::Numeric {
                width: candidate_width,
                height: candidate_height,
            }),
        ) => {
            within_fifteen_percent(first_width, candidate_width)
                && within_fifteen_percent(first_height, candidate_height)
        }
        (
            Some(ShellSize::FillWidth {
                height: first_height,
            }),
            Some(ShellSize::FillWidth {
                height: candidate_height,
            }),
        ) => height_policies_match(first_height, candidate_height),
        _ => false,
    }
}

fn within_fifteen_percent(first: f64, second: f64) -> bool {
    let smaller = first.min(second);
    smaller > 0.0 && (first.max(second) - smaller) / smaller <= 0.15
}

fn height_policies_match(first: HeightPolicy, second: HeightPolicy) -> bool {
    match (first, second) {
        (HeightPolicy::Number(first), HeightPolicy::Number(second)) => {
            within_fifteen_percent(first, second)
        }
        (HeightPolicy::Keyword(first), HeightPolicy::Keyword(second)) => first == second,
        _ => false,
    }
}

fn is_tappable_card(node: &PenNode) -> bool {
    if node.on_tap().is_some() {
        return true;
    }
    let role = node.base().role.as_deref().unwrap_or_default();
    let name = node.base().name.as_deref().unwrap_or_default();
    [role, name].iter().any(|value| {
        let value = value.to_ascii_lowercase();
        ["card", "tile", "option", "choice"]
            .iter()
            .any(|marker| value.contains(marker))
    })
}

fn collect_interactive_patches(node: &PenNode, plan: &mut PatchPlan, is_root: bool) {
    if !is_root && is_excluded_family_subtree(node) {
        return;
    }
    let role = node.base().role.as_deref().unwrap_or_default();
    if node.on_tap().is_some() || matches!(role, "button" | "primary-button" | "cta") {
        add_transition(plan, node.id_str());
    }
    for child in node.children().into_iter().flatten() {
        collect_interactive_patches(child, plan, false);
    }
}

fn add_transition(plan: &mut PatchPlan, node_id: &str) {
    add_field(
        plan,
        node_id,
        "transition",
        json!({
            "durationMs": INTERACTIVE_DURATION_MS,
            "easing": "standard",
            "properties": ["fill", "opacity", "scaleX", "scaleY"]
        }),
    );
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
