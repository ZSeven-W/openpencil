//! Repair proven tutorial label collisions without rewriting screenshot pixels,
//! step content, fonts, or source-space annotations.

use super::*;

/// Run only tutorial label repairs on the captured modify scope. This does
/// not invoke general generation cleanup or resize an existing board.
pub fn repair_scoped_tutorial_labels(state: &mut EditorState, root_ids: &[String]) -> usize {
    let has_tutorial = root_ids.iter().any(|id| {
        op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(id.to_string()))
            .and_then(|n| serde_json::to_value(n).ok())
            .is_some_and(|v| contains_step(&v))
    });
    if !has_tutorial {
        return 0;
    }
    let mut applied = 0;
    for _ in 0..4 {
        let rects = resolved_rects(state);
        let mut cmds = Vec::new();
        for id in root_ids {
            let Some(root) = op_editor_core::walkers::find_node(
                state.active_children(),
                &NodeId::new(id.to_string()),
            ) else {
                continue;
            };
            if let Ok(v) = serde_json::to_value(root) {
                collect_fixes(&v, &rects, &mut cmds);
            }
        }
        if cmds.is_empty() {
            break;
        }
        for cmd in cmds {
            applied += usize::from(state.apply(cmd));
        }
    }
    applied
}

fn name(v: &Value) -> String {
    v.get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn step_number(v: &Value) -> bool {
    let label = name(v);
    v.get("type").and_then(Value::as_str) == Some("text")
        && (label.contains("步骤编号") || label.contains("step-number"))
        && v.get("content")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.trim().is_empty())
}

fn rect<'a>(v: &Value, rects: &'a HashMap<String, Rect>) -> Option<&'a Rect> {
    rects.get(v.get("id")?.as_str()?)
}

fn intersects(a: &Rect, b: &Rect) -> bool {
    a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y
}

pub(super) fn collect_fixes(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    collect_step_ink(v, rects, cmds);
    // A measured GLM tutorial header omitted layout on a wrapper containing
    // a step/title row followed by prose. The default row put prose outside
    // the board. Explicit tutorial headers stack these two blocks vertically.
    if layout_str(v).is_none()
        && name(v).contains("标题栏")
        && children(v)
            .iter()
            .any(|n| n.get("type").and_then(Value::as_str) == Some("text") && name(n) == "说明")
        && children(v).iter().any(contains_step)
    {
        if let Some(id) = v.get("id").and_then(Value::as_str) {
            cmds.push(EditorCommand::SetNodeLayoutProp {
                node_id: NodeId::new(id.to_string()),
                property: "layout".into(),
                value: LayoutPropValue::Keyword("vertical".into()),
            });
        }
    }
    // Both explicit none stacks and screenshot templates with omitted layout
    // carry authored positions. Never apply absolute edits to a flex flow.
    if matches!(layout_str(v), None | Some("none")) {
        collect_badges(v, rects, cmds);
        collect_captions(v, rects, cmds);
    }
    for child in children(v) {
        collect_fixes(child, rects, cmds);
    }
}

fn contains_step(v: &Value) -> bool {
    step_number(v) || children(v).iter().any(contains_step)
}

fn solid_hex(v: &Value) -> Option<&str> {
    let [fill] = v.get("fill")?.as_array()?.as_slice() else {
        return None;
    };
    let color = fill.get("color")?.as_str()?;
    (fill.get("type").and_then(Value::as_str) == Some("solid")
        && geometry_buried_overlay::paints_opaque(v)
        && op_design_lint::color::parse_hex_color(color).is_some())
    .then_some(color)
}

fn collect_step_ink(v: &Value, rects: &HashMap<String, Rect>, cmds: &mut Vec<EditorCommand>) {
    for label in children(v).iter().filter(|n| step_number(n)) {
        let sibling_backing = if matches!(layout_str(v), None | Some("none")) {
            let Some(label_rect) = rect(label, rects) else {
                continue;
            };
            children(v).iter().find_map(|n| {
                let n_name = name(n);
                ((n_name.contains("编号底") || n_name.contains("step-number-background"))
                    && rect(n, rects).is_some_and(|r| intersects(label_rect, r)))
                .then(|| solid_hex(n))
                .flatten()
            })
        } else {
            None
        };
        let backing = sibling_backing.or_else(|| solid_hex(v));
        let Some(background) = backing else {
            continue;
        };
        let foreground = if label.get("fill").is_none() {
            Some("#000000")
        } else {
            solid_hex(label)
        };
        let Some(foreground) = foreground else {
            continue;
        };
        if op_design_lint::color::color_contrast(foreground, background) >= 4.5 {
            continue;
        }
        let ink = ["#FFFFFF", "#000000"]
            .into_iter()
            .max_by(|a, b| {
                op_design_lint::color::color_contrast(a, background)
                    .total_cmp(&op_design_lint::color::color_contrast(b, background))
            })
            .unwrap();
        if let Some(id) = label.get("id").and_then(Value::as_str) {
            cmds.push(EditorCommand::PatchNodeData {
                node_id: NodeId::new(id.to_string()),
                patch_json: serde_json::json!({"fill":[{"type":"solid","color":ink}]}).to_string(),
                page_id: None,
            });
        }
    }
}

fn collect_badges(v: &Value, rects: &HashMap<String, Rect>, cmds: &mut Vec<EditorCommand>) {
    let Some(parent) = v.get("id").and_then(Value::as_str) else {
        return;
    };
    let kids = children(v);
    for (index, label) in kids.iter().enumerate().rev() {
        if !step_number(label) {
            continue;
        }
        let Some(label_rect) = rect(label, rects) else {
            continue;
        };
        let backing = kids[..index].iter().position(|cover| {
            let cover_name = name(cover);
            (cover_name.contains("编号底") || cover_name.contains("step-number-background"))
                && matches!(
                    cover.get("type").and_then(Value::as_str),
                    Some("ellipse" | "rectangle")
                )
                && children(cover).is_empty()
                && geometry_buried_overlay::paints_opaque(cover)
                && rect(cover, rects).is_some_and(|r| intersects(label_rect, r))
        });
        if let (Some(target), Some(id)) = (backing, label.get("id").and_then(Value::as_str)) {
            cmds.push(EditorCommand::MoveNode {
                node_id: NodeId::new(id.to_string()),
                target_parent: NodeId::new(parent.to_string()),
                page_id: None,
                index: Some(target),
            });
        }
    }
}

fn collect_captions(v: &Value, rects: &HashMap<String, Rect>, cmds: &mut Vec<EditorCommand>) {
    let kids = children(v);
    if !kids.iter().any(step_number) {
        return;
    }
    let Some(parent) = rect(v, rects) else {
        return;
    };
    for image in kids.iter().filter(|n| {
        n.get("type").and_then(Value::as_str) == Some("image")
            && (name(n).contains("screenshot") || name(n).contains("截图"))
    }) {
        let Some(image_rect) = rect(image, rects) else {
            continue;
        };
        for caption in kids.iter().filter(|n| {
            n.get("type").and_then(Value::as_str) == Some("text")
                && (name(n).starts_with("标注") || name(n).contains("screenshot-caption"))
        }) {
            let (Some(caption_rect), Some(id)) = (
                rect(caption, rects),
                caption.get("id").and_then(Value::as_str),
            ) else {
                continue;
            };
            // Only a lower-edge explanatory caption is moved. Hero copy and
            // source annotations intentionally placed inside images stay put.
            if !intersects(caption_rect, image_rect)
                || caption_rect.y < image_rect.y + image_rect.h * 0.8
            {
                continue;
            }
            let new_y = image_rect.y + image_rect.h + 12.0;
            let bottom = kids
                .iter()
                .filter(|n| n.get("id") != caption.get("id"))
                .filter_map(|n| rect(n, rects))
                .filter(|r| r.y >= image_rect.y + image_rect.h)
                .map(|r| r.y - 12.0)
                .fold(parent.y + parent.h - 12.0, f64::min);
            let wider = image_rect.w > caption_rect.w + 1.0;
            // A wrapped caption first gets the screenshot's width. The next
            // geometry round measures real text height before moving it.
            if new_y + caption_rect.h > bottom && !wider {
                continue;
            }
            cmds.push(EditorCommand::UpdateNode {
                node_id: NodeId::new(id.to_string()),
                x: Some((image_rect.x - parent.x).round() as i32),
                y: (new_y + caption_rect.h <= bottom).then_some((new_y - parent.y).round() as i32),
                width: wider.then_some(image_rect.w.round() as i32),
                height: None,
                name: None,
                fill_hex: None,
                page_id: None,
            });
        }
    }
}

#[cfg(test)]
#[path = "geometry_tutorial_labels_tests.rs"]
mod tests;
