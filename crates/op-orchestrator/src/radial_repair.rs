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
    let kids = children(v);
    let arc_count = kids.iter().filter(|child| is_arc_ellipse(child)).count();
    if arc_count < 2 {
        return None;
    }
    let id = v.get("id").and_then(Value::as_str)?;
    let max_arc = kids
        .iter()
        .filter(|child| is_arc_ellipse(child))
        .filter_map(|child| child_diameter(child, rects))
        .fold(0.0, f64::max);
    if max_arc <= 0.0 {
        return None;
    }

    let parent_rect = rects.get(id).copied();
    let parent_w = parent_axis_size(v, "width", parent_rect.map(|r| r.w), max_arc);
    let parent_h = parent_axis_size(v, "height", parent_rect.map(|r| r.h), max_arc);
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
    if !has_numeric(v, "width") {
        cmds.push(update_size(id, Some(max_arc), None));
    }
    if !has_numeric(v, "height") {
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

fn child_diameter(v: &Value, rects: &HashMap<String, Rect>) -> Option<f64> {
    let w = numeric(v, "width");
    let h = numeric(v, "height");
    match (w, h) {
        (Some(w), Some(h)) => Some(w.max(h)),
        _ => v
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| rects.get(id))
            .map(|r| r.w.max(r.h)),
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
        let w = numeric(child, "width").or_else(|| estimated.map(|size| size.0));
        let h = numeric(child, "height").or_else(|| estimated.map(|size| size.1));
        if let (Some(w), Some(h)) = (w, h) {
            sizes.push((w, h));
        }
    }
    if sizes.is_empty() {
        return None;
    }
    match v.get("layout").and_then(Value::as_str) {
        Some("horizontal") => Some((
            sizes.iter().map(|size| size.0).sum::<f64>()
                + gap * sizes.len().saturating_sub(1) as f64,
            sizes.iter().map(|size| size.1).fold(0.0, f64::max),
        )),
        _ => Some((
            sizes.iter().map(|size| size.0).fold(0.0, f64::max),
            sizes.iter().map(|size| size.1).sum::<f64>()
                + gap * sizes.len().saturating_sub(1) as f64,
        )),
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
        && (v.get("sweepAngle").is_some() || numeric(v, "innerRadius").is_some_and(|r| r > 0.0))
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
