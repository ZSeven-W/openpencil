//! Read-only diagnostics for ambiguous design-agent output.
//!
//! These checks deliberately report instead of mutating. Geometry can prove
//! that a row is narrow or a painted box is picture-sized, but it cannot prove
//! whether the author wanted a scroller, a grid, media, or decoration.
//! The design model must see the evidence and make that choice in-loop.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use jian_ops_schema::node::{ContainerProps, LayoutMode, NumberOrExpression, Padding, PenNode};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use jian_ops_schema::style::PenFill;
use jian_ops_schema::variable::VariableDefinition;
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::EditorState;

const LIMIT: usize = 8;

#[derive(Debug, Default)]
pub(crate) struct BatchDesignDiagnostics {
    pub(crate) layout_issues: Vec<String>,
    pub(crate) intent_questions: Vec<String>,
    pub(crate) variable_issues: Vec<String>,
    pub(crate) image_slot_candidates: Vec<String>,
}

pub(crate) fn collect_batch_design_diagnostics(state: &EditorState) -> BatchDesignDiagnostics {
    let rects = resolved_sizes(state);
    let effective_theme = op_editor_core::variables_resolve::effective_theme(
        &state.doc,
        &state.ui.variables.active_theme,
    );
    let mut out = BatchDesignDiagnostics::default();
    for root in state.active_children() {
        walk(
            root,
            &rects,
            state.doc.variables.as_ref(),
            &effective_theme,
            &mut out,
        );
    }
    out.layout_issues.truncate(LIMIT);
    out.intent_questions.truncate(LIMIT);
    out.variable_issues.truncate(LIMIT);
    out.image_slot_candidates.truncate(LIMIT);
    out
}

fn walk(
    node: &PenNode,
    rects: &HashMap<String, (f64, f64)>,
    variables: Option<&BTreeMap<String, VariableDefinition>>,
    theme: &BTreeMap<String, String>,
    out: &mut BatchDesignDiagnostics,
) {
    scan_variable_refs(node, variables, theme, &mut out.variable_issues);
    scan_container_intent(node, rects, &mut out.intent_questions);
    scan_absolute_stack_order(node, rects, &mut out.layout_issues);

    let Some(children) = node.children() else {
        return;
    };
    scan_image_slot_candidates(children, rects, &mut out.image_slot_candidates);
    scan_stray_image(children, &mut out.intent_questions);
    scan_duplicate_image_carriers(children, &mut out.intent_questions);
    for child in children {
        walk(child, rects, variables, theme, out);
    }
}

/// Report a provably reversed overlay stack without changing child order.
///
/// Canonical children are front-to-back: index 0 is topmost and painters walk
/// the slice in reverse. A full-bleed surface before a semantic badge/overlay
/// therefore hides that overlay. The check stays narrow: explicit
/// `layout:none`, origin-anchored full-bleed media/surface, and an explicitly
/// named/roled badge or overlay are all required.
fn scan_absolute_stack_order(
    node: &PenNode,
    rects: &HashMap<String, (f64, f64)>,
    out: &mut Vec<String>,
) {
    if out.len() >= LIMIT
        || !container(node)
            .is_some_and(|container| matches!(container.layout, Some(LayoutMode::None)))
    {
        return;
    }
    let Some(children) = node.children() else {
        return;
    };
    for (overlay_index, overlay) in children.iter().enumerate().skip(1) {
        if !has_badge_or_overlay_semantics(overlay) {
            continue;
        }
        let Some(surface) = children[..overlay_index]
            .iter()
            .find(|candidate| is_full_bleed_media_or_surface(node, candidate, rects))
        else {
            continue;
        };
        out.push(format!(
            "{}: full-bleed media/surface {} precedes {} in a layout:none stack. children[0] is topmost because canvas paint is reverse-order, so the surface covers the overlay. Move the overlay to index 0 with M(\"{}\", \"{}\", 0). Keep a separate EMPTY frame/rectangle image slot for strict G(...) and target that slot, never the stack container that owns the overlay.",
            label(node),
            label(surface),
            label(overlay),
            overlay.id_str(),
            node.id_str(),
        ));
        return;
    }
}

fn has_badge_or_overlay_semantics(node: &PenNode) -> bool {
    [node.base().name.as_deref(), node.base().role.as_deref()]
        .into_iter()
        .flatten()
        .any(|value| {
            let compact: String = value
                .chars()
                .filter(|character| character.is_ascii_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect();
            compact.contains("badge") || compact.contains("overlay")
        })
}

fn is_full_bleed_media_or_surface(
    parent: &PenNode,
    child: &PenNode,
    rects: &HashMap<String, (f64, f64)>,
) -> bool {
    let is_media_or_surface = matches!(child, PenNode::Image(_))
        || has_explicit_image_semantics(child)
        || container(child).is_some_and(|container| {
            container
                .fill
                .as_deref()
                .is_some_and(|fills| !fills.is_empty())
        });
    if !is_media_or_surface
        || child.base().x.unwrap_or(0.0).abs() > 1.0
        || child.base().y.unwrap_or(0.0).abs() > 1.0
    {
        return false;
    }

    let parent_resolved = rects.get(parent.id_str()).copied();
    let child_resolved = rects.get(child.id_str()).copied();
    axis_covers(
        width_sizing(child),
        width_sizing(parent),
        child_resolved.map(|size| size.0),
        parent_resolved.map(|size| size.0),
    ) && axis_covers(
        height_sizing(child),
        height_sizing(parent),
        child_resolved.map(|size| size.1),
        parent_resolved.map(|size| size.1),
    )
}

fn width_sizing(node: &PenNode) -> Option<&SizingBehavior> {
    match node {
        PenNode::Frame(node) => node.container.width.as_ref(),
        PenNode::Group(node) => node.container.width.as_ref(),
        PenNode::Rectangle(node) => node.container.width.as_ref(),
        PenNode::Image(node) => node.width.as_ref(),
        _ => None,
    }
}

fn height_sizing(node: &PenNode) -> Option<&SizingBehavior> {
    match node {
        PenNode::Frame(node) => node.container.height.as_ref(),
        PenNode::Group(node) => node.container.height.as_ref(),
        PenNode::Rectangle(node) => node.container.height.as_ref(),
        PenNode::Image(node) => node.height.as_ref(),
        _ => None,
    }
}

fn axis_covers(
    child: Option<&SizingBehavior>,
    parent: Option<&SizingBehavior>,
    child_resolved: Option<f64>,
    parent_resolved: Option<f64>,
) -> bool {
    if matches!(
        child,
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
    ) {
        return true;
    }
    if let (Some(SizingBehavior::Number(child)), Some(SizingBehavior::Number(parent))) =
        (child, parent)
    {
        return *parent > 0.0 && *child >= *parent * 0.9;
    }
    matches!((child_resolved, parent_resolved), (Some(child), Some(parent)) if parent > 0.0 && child >= parent * 0.9)
}

fn scan_container_intent(
    node: &PenNode,
    rects: &HashMap<String, (f64, f64)>,
    out: &mut Vec<String>,
) {
    if out.len() >= LIMIT {
        return;
    }
    let Some(container) = container(node) else {
        return;
    };
    let children = node.children().map(Vec::as_slice).unwrap_or(&[]);
    let flow_children: Vec<&PenNode> = children
        .iter()
        .filter(|child| child.base().x.is_none() && child.base().y.is_none())
        .collect();

    if container.layout.is_none() && flow_children.len() >= 2 {
        out.push(format!(
            "{}: has {} flow children but no layout. Choose layout vertical, horizontal, or none explicitly; the finalizer will not guess.",
            label(node),
            flow_children.len()
        ));
    }
    if out.len() >= LIMIT {
        return;
    }

    if matches!(container.layout, Some(LayoutMode::Horizontal))
        && container.clip_content != Some(true)
        && flow_children.len() >= 3
        && flow_children.iter().all(|child| is_fill_width(child))
    {
        let starved: Vec<&str> = flow_children
            .iter()
            .filter(|child| {
                let resolved = rects
                    .get(child.id_str())
                    .map(|(width, _)| *width)
                    .unwrap_or(f64::MAX);
                resolved < 120.0 && fixed_descendant_width(child, 3) > resolved + 12.0
            })
            .map(|child| child.id_str())
            .collect();
        if starved.len() >= 3 {
            out.push(format!(
                "{}: cards [{}] resolve below 120px while containing wider fixed content. Decide whether this is a clipped scroller, fewer/wider cards, or content that should shrink; do not infer card width from an image's numbers.",
                label(node),
                starved.join(", ")
            ));
        }
    }
    if out.len() >= LIMIT {
        return;
    }

    if matches!(container.layout, Some(LayoutMode::Horizontal))
        && is_fill_width(node)
        && has_solid_fill(container)
        && flow_children.len() >= 2
        && flow_children.iter().all(|child| child.width_px().is_some())
    {
        let content_width = flow_children
            .iter()
            .filter_map(|child| child.width_px())
            .sum::<f64>()
            + numeric_gap(container) * (flow_children.len() - 1) as f64
            + horizontal_padding(container);
        if rects
            .get(node.id_str())
            .is_some_and(|(width, _)| *width > content_width * 1.4 && *width > content_width + 24.0)
        {
            out.push(format!(
                "{}: a painted fill-width row is much wider than its fixed children. If it is a control cluster, set fit_content; if it is a full-width surface, keep it. The finalizer will not choose.",
                label(node)
            ));
        }
    }
}

fn scan_image_slot_candidates(
    children: &[PenNode],
    rects: &HashMap<String, (f64, f64)>,
    out: &mut Vec<String>,
) {
    for (index, node) in children.iter().enumerate() {
        if out.len() >= LIMIT || !is_empty_solid_box(node) {
            continue;
        }
        let explicit = has_explicit_image_semantics(node);
        let context = children
            .iter()
            .enumerate()
            .filter(|(other, _)| *other != index)
            .find_map(|(_, sibling)| first_text(sibling, 3));
        let picture_sized = rects.get(node.id_str()).is_some_and(|(width, height)| {
            *width >= 80.0 && *height >= 60.0 && *width / *height <= 4.0 && *height / *width <= 4.0
        });
        let geometry_candidate =
            picture_sized && context.is_some() && has_only_generic_box_name(node);
        if !(explicit || geometry_candidate) {
            continue;
        }
        let subject = context
            .as_deref()
            .or_else(|| node.base().name.as_deref())
            .unwrap_or("nearby content");
        out.push(format!(
            "{} may be an unfilled image slot near {:?}. If it is media, bind it with operations-mode G(id, \"search\", \"<specific subject>\"); G is rejected in script mode. If it is decorative, remove any media role/query and give it a descriptive non-media name.",
            label(node), subject
        ));
    }
}

fn scan_stray_image(children: &[PenNode], out: &mut Vec<String>) {
    if out.len() >= LIMIT {
        return;
    }
    let images: Vec<&PenNode> = children
        .iter()
        .filter(|node| is_bound_image_node(node))
        .collect();
    if images.len() != 1 {
        return;
    }
    let mut slots = Vec::new();
    for child in children {
        collect_explicit_empty_slots(child, 0, &mut slots);
    }
    if let [slot] = slots.as_slice() {
        out.push(format!(
            "{} is a direct image sibling of empty media slot {}. Inspect explicit parent/slot hierarchy only; if that structure assigns the image to this slot, move it with operations-mode M(imageId, slotId) and then size it. Do not decide from image subject, aesthetics, or perceived quality. Nothing is moved automatically.",
            label(images[0]), label(slot)
        ));
    }
}

fn scan_duplicate_image_carriers(children: &[PenNode], out: &mut Vec<String>) {
    if out.len() >= LIMIT || !children.iter().any(is_bound_image_node) {
        return;
    }
    let plates: Vec<&PenNode> = children
        .iter()
        .filter(|node| {
            node.children().is_none_or(Vec::is_empty) && container(node).is_some_and(has_image_fill)
        })
        .collect();
    if !plates.is_empty() {
        out.push(format!(
            "This slot contains both image node(s) and image-filled plate(s) [{}]. They may be intentional layers or duplicate carriers; compare source equality and bounds/overlap only, then delete only the confirmed duplicate. Never choose from image subject, aesthetics, or perceived quality.",
            plates
                .iter()
                .map(|node| label(node))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
}

fn is_bound_image_node(node: &PenNode) -> bool {
    matches!(node, PenNode::Image(image)
        if !image.src.is_empty()
            || image.image_prompt.as_deref().is_some_and(|prompt| !prompt.trim().is_empty())
            || image.image_search_query.as_deref().is_some_and(|query| !query.trim().is_empty()))
}

fn scan_variable_refs(
    node: &PenNode,
    variables: Option<&BTreeMap<String, VariableDefinition>>,
    theme: &BTreeMap<String, String>,
    out: &mut Vec<String>,
) {
    if out.len() >= LIMIT {
        return;
    }
    let Ok(value) = serde_json::to_value(node) else {
        return;
    };
    let mut refs = BTreeSet::new();
    for key in ["fill", "stroke"] {
        if let Some(value) = value.get(key) {
            collect_ref_strings(value, &mut refs);
        }
    }
    for reference in refs {
        if !reference.starts_with('$') {
            continue;
        }
        let valid_color =
            op_editor_core::variables_resolve::resolve_color_ref(&reference, variables, theme)
                .is_some_and(|color| op_editor_core::parse_hex_rgb(&color).is_some());
        if valid_color {
            continue;
        }
        out.push(format!(
            "{}: variable reference {} does not resolve to a valid color for the current theme. Keep the node unchanged, then replace the reference with an existing semantic color token or an explicit value after checking the intended contrast.",
            label(node), reference
        ));
        if out.len() >= LIMIT {
            break;
        }
    }
}

fn collect_ref_strings(value: &serde_json::Value, out: &mut BTreeSet<String>) {
    match value {
        serde_json::Value::String(value) if value.starts_with('$') => {
            out.insert(value.clone());
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_ref_strings(value, out);
            }
        }
        serde_json::Value::Object(values) => {
            for value in values.values() {
                collect_ref_strings(value, out);
            }
        }
        _ => {}
    }
}

fn resolved_sizes(state: &EditorState) -> HashMap<String, (f64, f64)> {
    let scene = op_pen_loader::editor_state_to_layout_scene(state);
    let mut out = HashMap::new();
    fn walk(
        nodes: &[op_editor_ui::layout_scene::SceneNode],
        out: &mut HashMap<String, (f64, f64)>,
    ) {
        for node in nodes {
            let bounds = node.aggregate_bounds();
            out.insert(
                node.id.clone(),
                (bounds.size.x as f64, bounds.size.y as f64),
            );
            walk(&node.children, out);
        }
    }
    for page in &scene.pages {
        walk(&page.children, &mut out);
    }
    out
}

fn container(node: &PenNode) -> Option<&ContainerProps> {
    match node {
        PenNode::Frame(node) => Some(&node.container),
        PenNode::Group(node) => Some(&node.container),
        PenNode::Rectangle(node) => Some(&node.container),
        _ => None,
    }
}

fn is_fill_width(node: &PenNode) -> bool {
    container(node).is_some_and(|container| {
        matches!(
            container.width,
            Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
        )
    })
}

fn has_solid_fill(container: &ContainerProps) -> bool {
    container
        .fill
        .as_deref()
        .is_some_and(|fills| fills.iter().any(|fill| matches!(fill, PenFill::Solid(_))))
}

fn has_image_fill(container: &ContainerProps) -> bool {
    container
        .fill
        .as_deref()
        .is_some_and(|fills| fills.iter().any(|fill| matches!(fill, PenFill::Image(_))))
}

fn is_empty_solid_box(node: &PenNode) -> bool {
    matches!(node, PenNode::Frame(_) | PenNode::Rectangle(_))
        && node.children().is_none_or(Vec::is_empty)
        && container(node).is_some_and(has_solid_fill)
}

fn has_explicit_image_semantics(node: &PenNode) -> bool {
    if node.base().role.as_deref() == Some("image-placeholder") {
        return true;
    }
    if matches!(node, PenNode::Frame(frame) if frame.image_search_query.as_deref().is_some_and(|query| !query.trim().is_empty()))
    {
        return true;
    }
    node.base()
        .name
        .as_deref()
        .is_some_and(has_image_area_keyword)
}

/// Geometry may raise a question only when the box has no authored semantic
/// meaning of its own. A name such as `Chart` or `Promo Block` is positive
/// evidence that the surface is not a photo slot; repeatedly asking the model
/// to rename it would make the diagnostic impossible to satisfy.
fn has_only_generic_box_name(node: &PenNode) -> bool {
    let Some(name) = node.base().name.as_deref().map(str::trim) else {
        return true;
    };
    if name.is_empty() {
        return true;
    }
    let normalized = name
        .trim_end_matches(|character: char| character.is_ascii_digit() || character.is_whitespace())
        .to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "frame" | "rectangle" | "box" | "slot" | "shape" | "placeholder"
    )
}

fn has_image_area_keyword(name: &str) -> bool {
    let compact: String = name
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    if compact.ends_with("img")
        || compact.ends_with("image")
        || compact.ends_with("photo")
        || compact.ends_with("cover")
        || compact.ends_with("thumbnail")
        || compact.ends_with("artwork")
        || compact.ends_with("media")
    {
        return true;
    }
    name.split(|character: char| !character.is_ascii_alphanumeric())
        .map(str::to_ascii_lowercase)
        .any(|word| {
            matches!(
                word.as_str(),
                "image"
                    | "img"
                    | "photo"
                    | "picture"
                    | "pic"
                    | "cover"
                    | "thumbnail"
                    | "thumb"
                    | "hero"
                    | "banner"
                    | "poster"
                    | "art"
                    | "artwork"
                    | "album"
                    | "avatar"
                    | "media"
                    | "placeholder"
                    | "ph"
            )
        })
}

fn first_text(node: &PenNode, depth: usize) -> Option<String> {
    if let PenNode::Text(text) = node {
        return match &text.content {
            jian_ops_schema::node::TextContent::Plain(content) => Some(content.trim().to_string()),
            jian_ops_schema::node::TextContent::Styled(segments) => Some(
                segments
                    .iter()
                    .map(|segment| segment.text.as_str())
                    .collect::<String>()
                    .trim()
                    .to_string(),
            ),
        }
        .filter(|content| !content.is_empty());
    }
    if depth == 0 {
        return None;
    }
    node.children()
        .into_iter()
        .flatten()
        .find_map(|child| first_text(child, depth - 1))
}

fn collect_explicit_empty_slots<'a>(node: &'a PenNode, depth: usize, out: &mut Vec<&'a PenNode>) {
    if depth > 2 {
        return;
    }
    if is_empty_solid_box(node) && has_explicit_image_semantics(node) {
        out.push(node);
        return;
    }
    for child in node.children().into_iter().flatten() {
        collect_explicit_empty_slots(child, depth + 1, out);
    }
}

fn fixed_descendant_width(node: &PenNode, depth: usize) -> f64 {
    if depth == 0 {
        return 0.0;
    }
    node.children()
        .into_iter()
        .flatten()
        .map(|child| {
            child
                .width_px()
                .unwrap_or(0.0)
                .max(fixed_descendant_width(child, depth - 1))
        })
        .fold(0.0, f64::max)
}

fn numeric_gap(container: &ContainerProps) -> f64 {
    match container.gap.as_ref() {
        Some(NumberOrExpression::Number(gap)) => *gap,
        _ => 0.0,
    }
}

fn horizontal_padding(container: &ContainerProps) -> f64 {
    match container.padding.as_ref() {
        Some(Padding::Uniform(value)) => value * 2.0,
        Some(Padding::XY([_, horizontal])) => horizontal * 2.0,
        Some(Padding::LtrB([left, _, right, _])) => left + right,
        _ => 0.0,
    }
}

fn label(node: &PenNode) -> String {
    format!(
        "{} ({})",
        node.base().name.as_deref().unwrap_or("node"),
        node.id_str()
    )
}

#[cfg(test)]
#[path = "design_agent_overlay_order_tests.rs"]
mod overlay_order_tests;

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use jian_ops_schema::variable::VariableDefinition;

    fn state_with(root: serde_json::Value) -> EditorState {
        let mut state = EditorState::new();
        state.active_children_mut().clear();
        state
            .active_children_mut()
            .push(serde_json::from_value(root).expect("valid fixture"));
        state
    }

    #[test]
    fn ambiguous_structure_is_reported_without_mutation() {
        let state = state_with(serde_json::json!({
            "type": "frame", "id": "root", "name": "Page", "width": 390, "height": 844,
            "layout": "vertical", "children": [{
                "type": "frame", "id": "section", "name": "Deals", "width": "fill_container",
                "children": [
                    { "type": "text", "id": "title", "content": "Deals" },
                    { "type": "frame", "id": "card", "name": "Deal Card", "layout": "vertical",
                      "children": [
                        { "type": "frame", "id": "band", "name": "Media Band", "height": 80,
                          "layout": "none", "children": [
                            { "type": "frame", "id": "slot", "name": "DealImg", "width": 160,
                              "height": 80, "fill": [{"type":"solid","color":"#EEEEEE"}] }
                          ] },
                        { "type": "image", "id": "stray", "name": "Maldives", "width": 160,
                          "height": 120, "src": "", "imagePrompt": "maldives resort" }
                      ] }
                ]
            }]
        }));
        let before = serde_json::to_value(&state.doc).unwrap();

        let diagnostics = collect_batch_design_diagnostics(&state);

        assert!(diagnostics
            .intent_questions
            .iter()
            .any(|line| line.contains("no layout") && line.contains("section")));
        assert!(diagnostics
            .intent_questions
            .iter()
            .any(|line| line.contains("direct image sibling") && line.contains("stray")));
        assert!(diagnostics.intent_questions.iter().any(|line| {
            line.contains("explicit parent/slot hierarchy")
                && line.contains("Do not decide from image subject")
        }));
        assert_eq!(serde_json::to_value(&state.doc).unwrap(), before);
    }

    #[test]
    fn valid_white_and_black_tokens_survive_while_unknown_refs_are_reported() {
        let mut state = state_with(serde_json::json!({
            "type": "frame", "id": "root", "layout": "vertical", "children": [
                { "type": "text", "id": "white", "content": "Valid", "fill": [{"type":"solid","color":"$--white"}] },
                { "type": "rectangle", "id": "unknown", "width": 40, "height": 40,
                  "fill": [{"type":"solid","color":"$--gray-100"}] },
                { "type": "rectangle", "id": "numeric-token", "width": 40, "height": 40,
                  "fill": [{"type":"solid","color":"$radius-lg"}] },
                { "type": "rectangle", "id": "black", "width": 40, "height": 40,
                  "fill": [{"type":"solid","color":"$--black"}] }
            ]
        }));
        let mut variables = BTreeMap::<String, VariableDefinition>::new();
        for (name, value) in [("--white", "#FFFFFF"), ("--black", "#000000")] {
            variables.insert(
                name.to_string(),
                serde_json::from_value(serde_json::json!({"type":"color","value":value})).unwrap(),
            );
        }
        state.doc.variables = Some(variables);
        let before = serde_json::to_value(&state.doc).unwrap();

        let diagnostics = collect_batch_design_diagnostics(&state);

        assert_eq!(diagnostics.variable_issues.len(), 2);
        assert!(diagnostics
            .variable_issues
            .iter()
            .any(|issue| issue.contains("$--gray-100")));
        assert!(diagnostics
            .variable_issues
            .iter()
            .any(|issue| issue.contains("$radius-lg")));
        assert_eq!(serde_json::to_value(&state.doc).unwrap(), before);
    }

    #[test]
    fn resolved_anonymous_media_box_is_a_candidate_not_an_auto_fill() {
        let state = state_with(serde_json::json!({
            "type": "frame", "id": "root", "width": 390, "height": 844, "layout": "vertical",
            "children": [{
                "type": "frame", "id": "card", "name": "Santorini Card", "width": 340,
                "height": 220, "layout": "vertical", "children": [
                    { "type": "rectangle", "id": "slot", "width": "fill_container", "height": 140,
                      "fill": [{"type":"solid","color":"#E5E7EB"}] },
                    { "type": "text", "id": "title", "content": "Santorini escape" }
                ]
            }]
        }));
        let before = serde_json::to_value(&state.doc).unwrap();

        let diagnostics = collect_batch_design_diagnostics(&state);

        assert!(diagnostics
            .image_slot_candidates
            .iter()
            .any(|line| line.contains("slot") && line.contains("Santorini")));
        assert_eq!(serde_json::to_value(&state.doc).unwrap(), before);
    }

    #[test]
    fn descriptive_non_media_name_vetoes_geometry_only_image_diagnostic() {
        let state = state_with(serde_json::json!({
            "type": "frame", "id": "root", "width": 390, "height": 844, "layout": "vertical",
            "children": [{
                "type": "frame", "id": "card", "name": "Metrics Card", "width": 340,
                "height": 220, "layout": "vertical", "children": [
                    { "type": "rectangle", "id": "chart", "name": "Chart", "width": 320,
                      "height": 140, "fill": [{"type":"solid","color":"#E5E7EB"}] },
                    { "type": "text", "id": "title", "content": "Weekly revenue" }
                ]
            }]
        }));

        let diagnostics = collect_batch_design_diagnostics(&state);

        assert!(
            diagnostics.image_slot_candidates.is_empty(),
            "a chart has explicit non-media semantics: {:?}",
            diagnostics.image_slot_candidates
        );
    }

    #[test]
    fn xy_padding_uses_the_horizontal_component() {
        let container: ContainerProps = serde_json::from_value(serde_json::json!({
            "padding": [8, 24]
        }))
        .expect("valid container props");

        assert_eq!(horizontal_padding(&container), 48.0);
    }
}
