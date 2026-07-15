use std::collections::HashMap;

use jian_scene::layout_scene::SceneNode;
use op_editor_core::{EditorCommand, EditorState, LayoutPropValue, NodeId};
use serde_json::Value;

use crate::types::DocSink;

#[derive(Clone, Copy)]
struct Rect {
    w: f64,
    h: f64,
}

const MAX_SAFE_ASPECT_RATIO: f64 = 1.2;
const MIN_SAFE_ARC_DIAMETER_RATIO: f64 = 0.8;
const MIN_SAFE_ARC_TO_PARENT_RATIO: f64 = 0.6;
const MAX_SAFE_ARC_TO_PARENT_RATIO: f64 = 1.05;

#[derive(Clone, Copy)]
struct AuthoredChildPatch {
    index: usize,
    x: f64,
    y: f64,
    width: Option<f64>,
    height: Option<f64>,
}

struct AuthoredRadialPatch {
    children: Vec<AuthoredChildPatch>,
    order: Vec<usize>,
}

#[derive(Clone, Copy)]
struct AuthoredRadialGeometry {
    parent_w: f64,
    parent_h: f64,
}

struct RadialLayers {
    centres: Vec<usize>,
    progress: Vec<usize>,
    tracks: Vec<usize>,
}

/// True when direct arc children would still participate in flex flow.
///
/// A missing layout is a row in jian's resolver, so only `layout:none` is
/// already safe. Authored child x/y does not make a flex radial stack safe in
/// this pipeline. A full-track + partial-progress pair (or a segmented donut
/// with explicit angles) distinguishes an intended stack from independent
/// gauges without trusting the wrapper dimensions.
pub(crate) fn is_radial_stack_in_flow(v: &Value) -> bool {
    v.get("layout").and_then(Value::as_str) != Some("none") && radial_layers(v).is_some()
}

/// True when an authored track/progress pair cannot be shown concentrically
/// on the first reveal. Besides flex-flow stacks this catches `layout:none`
/// wrappers whose child coordinates, fixed geometry, or painter order are
/// incomplete. Those cases must be repaired before insertion or retried; the
/// late cleanup should not be the first moment the ring looks correct.
pub(crate) fn is_authored_radial_stack_unsafe(v: &Value) -> bool {
    if radial_layers(v).is_none() {
        return false;
    }
    let Some(patch) = authored_radial_patch(v) else {
        return true;
    };
    authored_patch_changes(v, &patch)
}

/// Normalize high-confidence radial stacks directly in an authored JSON
/// forest. This is intentionally independent of `EditorState`: it runs before
/// `InsertSubtree`, while the existing sink-based repair remains the late
/// resolved-layout fallback.
///
/// Only fixed, near-square wrappers with similarly sized authored arcs and
/// fully measurable direct children are changed. A high-confidence candidate
/// with unmeasurable centre content stays untouched so self-check retries it;
/// malformed but explicit track/progress pairs are left untouched so
/// self-check can request a retry rather than guessing wrapper geometry.
pub(crate) fn repair_authored_radial_stacks(value: &mut Value) -> bool {
    match value {
        Value::Array(nodes) => {
            let mut changed = false;
            for node in nodes {
                changed |= repair_authored_radial_node(node);
            }
            changed
        }
        Value::Object(_) => repair_authored_radial_node(value),
        _ => false,
    }
}

fn repair_authored_radial_node(node: &mut Value) -> bool {
    let patch = authored_radial_patch(node);
    let mut changed = patch.is_some_and(|patch| apply_authored_radial_patch(node, patch));
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children {
            changed |= repair_authored_radial_node(child);
        }
    }
    changed
}

fn authored_radial_patch(v: &Value) -> Option<AuthoredRadialPatch> {
    if radial_layer_order(v).is_none() || has_nonzero_padding(v) {
        return None;
    }
    let geometry = authored_radial_geometry(v)?;
    let kids = children(v);
    let order = radial_layer_order(v)?;
    let mut patches = Vec::with_capacity(kids.len());
    for (index, child) in kids.iter().enumerate() {
        let estimate = estimated_subtree_size(child);
        let width = numeric(child, "width").or_else(|| estimate.map(|size| size.0))?;
        let height = numeric(child, "height").or_else(|| estimate.map(|size| size.1))?;
        if !valid_size(width, height) {
            return None;
        }
        let width_patch = (!has_numeric(child, "width")).then_some(width.round());
        let height_patch = (!has_numeric(child, "height")).then_some(height.round());
        let positioned_width = width_patch.unwrap_or(width);
        let positioned_height = height_patch.unwrap_or(height);
        if !is_arc_ellipse(child)
            && (positioned_width > geometry.parent_w || positioned_height > geometry.parent_h)
        {
            return None;
        }
        patches.push(AuthoredChildPatch {
            index,
            x: ((geometry.parent_w - positioned_width) / 2.0).round(),
            y: ((geometry.parent_h - positioned_height) / 2.0).round(),
            width: width_patch,
            height: height_patch,
        });
    }
    Some(AuthoredRadialPatch {
        children: patches,
        order,
    })
}

fn authored_radial_geometry(v: &Value) -> Option<AuthoredRadialGeometry> {
    if !matches!(
        v.get("type").and_then(Value::as_str),
        Some("frame" | "group" | "rectangle")
    ) {
        return None;
    }
    let parent_w = numeric(v, "width")?;
    let parent_h = numeric(v, "height")?;
    if !near_square(parent_w, parent_h) {
        return None;
    }

    let kids = children(v);
    let arcs: Vec<&Value> = radial_layer_order(v)?
        .into_iter()
        .filter_map(|index| kids.get(index))
        .filter(|child| is_arc_ellipse(child))
        .collect();
    let mut diameters = Vec::with_capacity(arcs.len());
    for arc in arcs {
        let width = numeric(arc, "width")?;
        let height = numeric(arc, "height")?;
        if !near_square(width, height) {
            return None;
        }
        diameters.push(width.max(height));
    }
    let min_arc = diameters.iter().copied().fold(f64::INFINITY, f64::min);
    let max_arc = diameters.iter().copied().fold(0.0, f64::max);
    if !min_arc.is_finite() || max_arc <= 0.0 || min_arc / max_arc < MIN_SAFE_ARC_DIAMETER_RATIO {
        return None;
    }
    let parent_min = parent_w.min(parent_h);
    let arc_to_parent = max_arc / parent_min;
    if !(MIN_SAFE_ARC_TO_PARENT_RATIO..=MAX_SAFE_ARC_TO_PARENT_RATIO).contains(&arc_to_parent) {
        return None;
    }
    Some(AuthoredRadialGeometry { parent_w, parent_h })
}

fn authored_patch_changes(v: &Value, patch: &AuthoredRadialPatch) -> bool {
    if v.get("layout").and_then(Value::as_str) != Some("none")
        || numeric(v, "gap") != Some(0.0)
        || v.get("justifyContent").and_then(Value::as_str) != Some("start")
        || v.get("alignItems").and_then(Value::as_str) != Some("start")
    {
        return true;
    }
    let kids = children(v);
    if patch.order.iter().copied().ne(0..kids.len()) {
        return true;
    }
    patch.children.iter().any(|child_patch| {
        let Some(child) = kids.get(child_patch.index) else {
            return true;
        };
        numeric(child, "x") != Some(child_patch.x)
            || numeric(child, "y") != Some(child_patch.y)
            || child_patch
                .width
                .is_some_and(|width| numeric(child, "width") != Some(width))
            || child_patch
                .height
                .is_some_and(|height| numeric(child, "height") != Some(height))
    })
}

fn apply_authored_radial_patch(v: &mut Value, patch: AuthoredRadialPatch) -> bool {
    let changed = authored_patch_changes(v, &patch);
    if !changed {
        return false;
    }
    v["layout"] = Value::String("none".to_string());
    v["gap"] = Value::from(0.0);
    v["justifyContent"] = Value::String("start".to_string());
    v["alignItems"] = Value::String("start".to_string());
    let Some(kids) = v.get_mut("children").and_then(Value::as_array_mut) else {
        return false;
    };
    for child_patch in patch.children {
        let Some(child) = kids.get_mut(child_patch.index) else {
            continue;
        };
        child["x"] = Value::from(child_patch.x);
        child["y"] = Value::from(child_patch.y);
        if let Some(width) = child_patch.width {
            child["width"] = Value::from(width);
        }
        if let Some(height) = child_patch.height {
            child["height"] = Value::from(height);
        }
    }
    if patch.order.iter().copied().ne(0..kids.len()) {
        let mut old: Vec<Option<Value>> = std::mem::take(kids).into_iter().map(Some).collect();
        *kids = patch
            .order
            .into_iter()
            .filter_map(|index| old.get_mut(index).and_then(Option::take))
            .collect();
    }
    true
}

/// Canonical painter order for a radial visual. In OpenPencil lower
/// child indexes paint on top, so centre content comes first, partial progress
/// or pie-segment arcs next, and the full track last. A track/progress pair or
/// explicit-angle segmented donut excludes rows of independent gauges.
fn radial_layer_order(v: &Value) -> Option<Vec<usize>> {
    let RadialLayers {
        mut centres,
        progress,
        tracks,
    } = radial_layers(v)?;
    if centres.len() > 1 {
        return None;
    }
    centres.extend(progress);
    centres.extend(tracks);
    Some(centres)
}

fn radial_layers(v: &Value) -> Option<RadialLayers> {
    if !matches!(
        v.get("type").and_then(Value::as_str),
        Some("frame" | "group" | "rectangle")
    ) {
        return None;
    }
    let kids = children(v);
    let mut centres = Vec::new();
    let mut progress = Vec::new();
    let mut tracks = Vec::new();
    for (index, child) in kids.iter().enumerate() {
        if !is_arc_ellipse(child) {
            centres.push(index);
            continue;
        }
        match arc_layer(child) {
            Some(ArcLayer::Progress) => progress.push(index),
            Some(ArcLayer::Track) => tracks.push(index),
            None => return None,
        }
    }
    let track_progress_pair = !progress.is_empty() && !tracks.is_empty();
    let segmented_donut = tracks.is_empty() && segmented_arcs_cover_ring(kids, &progress);
    if !track_progress_pair && !segmented_donut {
        return None;
    }
    Some(RadialLayers {
        centres,
        progress,
        tracks,
    })
}

fn segmented_arcs_cover_ring(kids: &[Value], progress: &[usize]) -> bool {
    if progress.len() < 2 {
        return false;
    }
    let mut intervals = Vec::with_capacity(progress.len() + 1);
    let mut total = 0.0;
    for index in progress {
        let Some(child) = kids.get(*index) else {
            return false;
        };
        let (Some(start), Some(sweep)) =
            (numeric(child, "startAngle"), numeric(child, "sweepAngle"))
        else {
            return false;
        };
        if !start.is_finite() || !sweep.is_finite() || sweep <= 0.01 || sweep >= 359.5 {
            return false;
        }
        let start = start.rem_euclid(360.0);
        let end = start + sweep;
        total += sweep;
        if end <= 360.0 {
            intervals.push((start, end));
        } else {
            intervals.push((start, 360.0));
            intervals.push((0.0, end - 360.0));
        }
    }
    if !(270.0..=360.5).contains(&total) {
        return false;
    }
    intervals.sort_by(|left, right| left.0.total_cmp(&right.0));
    intervals
        .windows(2)
        .all(|pair| pair[1].0 >= pair[0].1 - 0.01)
}

#[derive(Clone, Copy)]
enum ArcLayer {
    Progress,
    Track,
}

fn arc_layer(v: &Value) -> Option<ArcLayer> {
    if !is_arc_ellipse(v) {
        return None;
    }
    if let Some(layer) = semantic_arc_layer(v) {
        return Some(layer);
    }
    match numeric(v, "sweepAngle") {
        Some(sweep) if sweep.is_finite() && sweep.abs() > 0.01 && sweep.abs() < 359.5 => {
            Some(ArcLayer::Progress)
        }
        Some(sweep) if sweep.is_finite() && sweep.abs() >= 359.5 => Some(ArcLayer::Track),
        None if numeric(v, "innerRadius").is_some_and(|radius| radius > 0.0) => {
            Some(ArcLayer::Track)
        }
        _ => None,
    }
}

fn semantic_arc_layer(v: &Value) -> Option<ArcLayer> {
    if v.get("type").and_then(Value::as_str) != Some("ellipse") {
        return None;
    }
    let semantic = ["name", "id", "role"]
        .into_iter()
        .filter_map(|key| v.get(key).and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    if semantic.contains("progress") || semantic.contains("foreground") {
        Some(ArcLayer::Progress)
    } else if semantic.contains("track") || semantic.contains("background") {
        Some(ArcLayer::Track)
    } else {
        None
    }
}

fn near_square(width: f64, height: f64) -> bool {
    valid_size(width, height) && width.max(height) / width.min(height) <= MAX_SAFE_ASPECT_RATIO
}

fn valid_size(width: f64, height: f64) -> bool {
    width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0
}

fn has_nonzero_padding(v: &Value) -> bool {
    match v.get("padding") {
        Some(Value::Number(number)) => number.as_f64().is_some_and(|value| value != 0.0),
        Some(Value::String(value)) => value.parse::<f64>().is_ok_and(|value| value != 0.0),
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_f64)
            .any(|value| value != 0.0),
        _ => false,
    }
}

pub fn repair_radial_stacks(sink: &mut dyn DocSink, root_id: &str) -> bool {
    let rects = resolved_rects(sink.state());
    let Some(root) = op_editor_core::walkers::find_node(
        sink.state().active_children(),
        &NodeId::new(root_id.to_string()),
    ) else {
        return false;
    };
    let Ok(v) = serde_json::to_value(root) else {
        return false;
    };
    let mut cmds = Vec::new();
    collect_radial_stack_repairs(&v, &rects, &mut cmds);
    if cmds.is_empty() {
        return false;
    }
    for cmd in cmds {
        sink.apply(cmd);
    }
    true
}

fn resolved_rects(state: &EditorState) -> HashMap<String, Rect> {
    let scene = op_pen_loader::editor_state_to_layout_scene(state);
    let mut map = HashMap::new();
    for page in &scene.pages {
        collect_rects(&page.children, &mut map);
    }
    map
}

fn collect_rects(nodes: &[SceneNode], map: &mut HashMap<String, Rect>) {
    for node in nodes {
        let b = node.aggregate_bounds();
        map.insert(
            node.id.clone(),
            Rect {
                w: f64::from(b.size.x),
                h: f64::from(b.size.y),
            },
        );
        collect_rects(&node.children, map);
    }
}

fn collect_radial_stack_repairs(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    if let Some(repair) = radial_stack_repair(v, rects) {
        cmds.extend(repair);
    }
    for child in children(v) {
        collect_radial_stack_repairs(child, rects, cmds);
    }
}

fn radial_stack_repair(v: &Value, rects: &HashMap<String, Rect>) -> Option<Vec<EditorCommand>> {
    if !matches!(
        v.get("type").and_then(Value::as_str),
        Some("frame" | "group" | "rectangle")
    ) {
        return None;
    }
    if has_nonzero_padding(v) {
        return None;
    }
    let kids = children(v);
    let layer_order = radial_layer_order(v)?;
    let id = v.get("id").and_then(Value::as_str)?;
    let arc_sizes: Vec<(f64, f64)> = layer_order
        .iter()
        .filter_map(|index| kids.get(*index))
        .filter(|child| is_arc_ellipse(child))
        .map(|child| child_size(child, rects))
        .collect::<Option<Vec<_>>>()?;
    if arc_sizes
        .iter()
        .any(|(width, height)| !near_square(*width, *height))
    {
        return None;
    }
    let min_arc = arc_sizes
        .iter()
        .map(|(width, height)| width.max(*height))
        .fold(f64::INFINITY, f64::min);
    let max_arc = arc_sizes
        .iter()
        .map(|(width, height)| width.max(*height))
        .fold(0.0, f64::max);
    if !min_arc.is_finite() || max_arc <= 0.0 || min_arc / max_arc < MIN_SAFE_ARC_DIAMETER_RATIO {
        return None;
    }

    let parent_rect = rects.get(id).copied();
    let fix_parent_width = should_fix_radial_parent_axis(v, "width");
    let fix_parent_height = should_fix_radial_parent_axis(v, "height");
    let parent_w = if fix_parent_width {
        max_arc
    } else {
        parent_axis_size(v, "width", parent_rect.map(|r| r.w), max_arc)
    };
    let parent_h = if fix_parent_height {
        max_arc
    } else {
        parent_axis_size(v, "height", parent_rect.map(|r| r.h), max_arc)
    };
    let arc_to_parent = max_arc / parent_w.min(parent_h);
    if !(MIN_SAFE_ARC_TO_PARENT_RATIO..=MAX_SAFE_ARC_TO_PARENT_RATIO).contains(&arc_to_parent) {
        return None;
    }
    let mut cmds = vec![
        EditorCommand::SetNodeLayoutProp {
            node_id: NodeId::new(id.to_string()),
            property: "layout".to_string(),
            value: LayoutPropValue::Keyword("none".to_string()),
        },
        EditorCommand::SetNodeLayoutProp {
            node_id: NodeId::new(id.to_string()),
            property: "gap".to_string(),
            value: LayoutPropValue::Number(0.0),
        },
        EditorCommand::SetNodeLayoutProp {
            node_id: NodeId::new(id.to_string()),
            property: "justifyContent".to_string(),
            value: LayoutPropValue::Keyword("start".to_string()),
        },
        EditorCommand::SetNodeLayoutProp {
            node_id: NodeId::new(id.to_string()),
            property: "alignItems".to_string(),
            value: LayoutPropValue::Keyword("start".to_string()),
        },
    ];
    if fix_parent_width {
        cmds.push(update_size(id, Some(max_arc), None));
    }
    if fix_parent_height {
        cmds.push(update_size(id, None, Some(max_arc)));
    }

    for child in kids {
        let Some(child_id) = child.get("id").and_then(Value::as_str) else {
            continue;
        };
        let estimate = estimated_subtree_size(child);
        let force_arc_size = is_arc_ellipse(child)
            && (!has_numeric(child, "width") || !has_numeric(child, "height"));
        let force_estimated_width =
            !is_arc_ellipse(child) && !has_numeric(child, "width") && estimate.is_some();
        let force_estimated_height =
            !is_arc_ellipse(child) && !has_numeric(child, "height") && estimate.is_some();
        let (cw, ch) = if force_arc_size {
            (max_arc, max_arc)
        } else if let Some(size) = estimate {
            (
                if has_numeric(child, "width") {
                    numeric(child, "width").unwrap()
                } else {
                    size.0
                },
                if has_numeric(child, "height") {
                    numeric(child, "height").unwrap()
                } else {
                    size.1
                },
            )
        } else {
            child_size(child, rects).unwrap_or((max_arc, max_arc))
        };
        let x = ((parent_w - cw) / 2.0).round();
        let y = ((parent_h - ch) / 2.0).round();
        cmds.push(EditorCommand::UpdateNode {
            node_id: NodeId::new(child_id.to_string()),
            x: Some(x as i32),
            y: Some(y as i32),
            width: (force_arc_size || force_estimated_width).then_some(cw.round() as i32),
            height: (force_arc_size || force_estimated_height).then_some(ch.round() as i32),
            name: None,
            fill_hex: None,
            page_id: None,
        });
    }
    Some(cmds)
}

fn parent_axis_size(v: &Value, key: &str, resolved: Option<f64>, max_arc: f64) -> f64 {
    if let Some(n) = numeric(v, key) {
        n
    } else if v.get(key).and_then(Value::as_str) == Some("fill_container") {
        resolved.unwrap_or(max_arc).max(1.0)
    } else {
        max_arc
    }
}

fn should_fix_radial_parent_axis(v: &Value, key: &str) -> bool {
    !has_numeric(v, key) && v.get(key).and_then(Value::as_str) != Some("fill_container")
}

fn child_size(v: &Value, rects: &HashMap<String, Rect>) -> Option<(f64, f64)> {
    let w = numeric(v, "width").or_else(|| estimated_text_size(v).map(|size| size.0));
    let h = numeric(v, "height").or_else(|| estimated_text_size(v).map(|size| size.1));
    match (w, h) {
        (Some(w), Some(h)) => Some((w, h)),
        _ => v
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| rects.get(id))
            .map(|r| (r.w, r.h)),
    }
}

fn estimated_text_size(v: &Value) -> Option<(f64, f64)> {
    if v.get("type").and_then(Value::as_str) != Some("text") {
        return None;
    }
    let content = v.get("content").and_then(Value::as_str).unwrap_or("");
    let font_size = numeric(v, "fontSize").unwrap_or(16.0).max(1.0);
    // 0.56em/char under-measures wider font stacks: on Linux CI the same
    // label resolved wide enough to WRAP inside the authored estimate,
    // adding a line and sinking the ring label 6px below center (measured).
    // 35% headroom keeps the forced width single-line on every platform;
    // the label is centered, so surplus width cannot misplace it.
    let width = (content.chars().count() as f64 * font_size * 0.56 * 1.35).max(font_size);
    let line_height = numeric(v, "lineHeight")
        .map(|lh| if lh <= 4.0 { lh * font_size } else { lh })
        .unwrap_or(font_size * 1.2);
    Some((width, line_height))
}

fn estimated_subtree_size(v: &Value) -> Option<(f64, f64)> {
    if let Some(size) = estimated_text_size(v) {
        return Some(size);
    }
    if !matches!(
        v.get("type").and_then(Value::as_str),
        Some("frame" | "group" | "rectangle")
    ) {
        return None;
    }
    let kids = children(v);
    if kids.is_empty() {
        return None;
    }
    let gap = numeric(v, "gap").unwrap_or(0.0);
    let mut sizes = Vec::new();
    for child in kids {
        let estimated = estimated_subtree_size(child);
        let w = numeric(child, "width").or_else(|| estimated.map(|size| size.0))?;
        let h = numeric(child, "height").or_else(|| estimated.map(|size| size.1))?;
        if !valid_size(w, h) {
            return None;
        }
        sizes.push((w, h));
    }
    let (content_w, content_h) = match v.get("layout").and_then(Value::as_str) {
        Some("none") => (
            sizes.iter().map(|size| size.0).fold(0.0, f64::max),
            sizes.iter().map(|size| size.1).fold(0.0, f64::max),
        ),
        Some("vertical") => (
            sizes.iter().map(|size| size.0).fold(0.0, f64::max),
            sizes.iter().map(|size| size.1).sum::<f64>()
                + gap * sizes.len().saturating_sub(1) as f64,
        ),
        _ => (
            sizes.iter().map(|size| size.0).sum::<f64>()
                + gap * sizes.len().saturating_sub(1) as f64,
            sizes.iter().map(|size| size.1).fold(0.0, f64::max),
        ),
    };
    let (padding_w, padding_h) = padding_extents(v)?;
    Some((content_w + padding_w, content_h + padding_h))
}

fn padding_extents(v: &Value) -> Option<(f64, f64)> {
    let Some(padding) = v.get("padding") else {
        return Some((0.0, 0.0));
    };
    let number = |value: &Value| match value {
        Value::Number(number) => number.as_f64(),
        Value::String(value) => value.parse::<f64>().ok(),
        _ => None,
    };
    let valid = |value: f64| value.is_finite() && value >= 0.0;
    match padding {
        Value::Number(_) | Value::String(_) => {
            let value = number(padding)?;
            valid(value).then_some((value * 2.0, value * 2.0))
        }
        Value::Array(values) => {
            let values: Vec<f64> = values.iter().map(number).collect::<Option<Vec<_>>>()?;
            if values.iter().any(|value| !valid(*value)) {
                return None;
            }
            match values.as_slice() {
                [] => Some((0.0, 0.0)),
                [all] => Some((all * 2.0, all * 2.0)),
                [vertical, horizontal] => Some((horizontal * 2.0, vertical * 2.0)),
                [top, right, bottom, left] => Some((right + left, top + bottom)),
                _ => None,
            }
        }
        _ => None,
    }
}

fn update_size(id: &str, width: Option<f64>, height: Option<f64>) -> EditorCommand {
    EditorCommand::UpdateNode {
        node_id: NodeId::new(id.to_string()),
        x: None,
        y: None,
        width: width.map(|w| w.round() as i32),
        height: height.map(|h| h.round() as i32),
        name: None,
        fill_hex: None,
        page_id: None,
    }
}

fn is_arc_ellipse(v: &Value) -> bool {
    v.get("type").and_then(Value::as_str) == Some("ellipse")
        && (v.get("sweepAngle").is_some()
            || numeric(v, "innerRadius").is_some_and(|r| r > 0.0)
            || (v.get("stroke").is_some() && semantic_arc_layer(v).is_some()))
}

fn has_numeric(v: &Value, key: &str) -> bool {
    numeric(v, key).is_some()
}

fn numeric(v: &Value, key: &str) -> Option<f64> {
    match v.get(key) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.parse().ok(),
        _ => None,
    }
}

fn children(v: &Value) -> &[Value] {
    v.get("children")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}
