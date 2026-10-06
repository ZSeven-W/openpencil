//! Give generated prose a reading gutter beside an in-flow vertical hairline.
//! Change only column padding; fixed/painted/positioned compositions stand down.

use super::*;

fn in_flow(v: &Value) -> bool {
    v.get("visible") != Some(&Value::Bool(false))
        && v.get("opacity").is_none_or(|_| num(v, "opacity") > 0.0)
        && num(v, "rotation") == 0.0
        && ![
            "x",
            "y",
            "constraints",
            "pin",
            "maskType",
            "effects",
            "flipX",
            "flipY",
        ]
        .iter()
        .any(|key| v.get(*key).is_some())
}

fn rect<'a>(v: &Value, rects: &'a HashMap<String, Rect>) -> Option<&'a Rect> {
    rects.get(v.get("id")?.as_str()?)
}

fn column(v: &Value) -> bool {
    in_flow(v)
        && v.get("type").and_then(Value::as_str) == Some("frame")
        && layout_str(v) == Some("vertical")
        && v.get("width").and_then(Value::as_str) == Some("fill_container")
        && v.get("height").and_then(Value::as_str) == Some("fit_content")
        && v.get("clipContent") != Some(&Value::Bool(true))
        && v.get("stroke").is_none_or(Value::is_null)
        && v.get("fill")
            .is_none_or(|fill| fill.is_null() || fill.as_array().is_some_and(Vec::is_empty))
}

fn divider(v: &Value) -> bool {
    in_flow(v)
        && v.get("type").and_then(Value::as_str) == Some("rectangle")
        && (0.5..=3.0).contains(&num(v, "width"))
        && v.get("height").and_then(Value::as_str) == Some("fill_container")
        && v.get("fill")
            .and_then(Value::as_array)
            .is_some_and(|fill| !fill.is_empty())
}

fn paragraphs<'a>(v: &'a Value, out: &mut Vec<&'a Value>) {
    if !in_flow(v) {
        return;
    }
    if v.get("type").and_then(Value::as_str) == Some("text") {
        let name = v
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_lowercase();
        let role = v.get("role").and_then(Value::as_str).unwrap_or("");
        let body = matches!(role, "body" | "body-text")
            || ["body", "paragraph", "正文", "段落"]
                .iter()
                .any(|token| name.contains(token));
        let content = v.get("content").and_then(Value::as_str).unwrap_or("");
        if body
            && !matches!(role, "heading" | "caption" | "label")
            && content.chars().count() >= 18
            && !content.contains(['\n', '\r', '\u{2028}', '\u{2029}'])
            && (18.0..=60.0).contains(&num(v, "fontSize"))
            && v.get("width").and_then(Value::as_str) == Some("fill_container")
            && v.get("height").and_then(Value::as_str) == Some("fit_content")
            && v.get("textGrowth").and_then(Value::as_str) == Some("fixed-width")
        {
            out.push(v);
        }
    } else if layout_str(v) == Some("vertical") {
        for child in children(v) {
            paragraphs(child, out);
        }
    }
}

fn collect(
    v: &Value,
    rects: &HashMap<String, Rect>,
    fixes: &mut HashMap<String, [f64; 4]>,
    root: bool,
) {
    let open_stack = layout_str(v) == Some("vertical")
        && matches!(
            v.get("height").and_then(Value::as_str),
            Some("fit_content" | "fill_container")
        );
    if !root && (!in_flow(v) || (v.get("clipContent") == Some(&Value::Bool(true)) && !open_stack)) {
        return;
    }
    let kids = children(v);
    if layout_str(v) == Some("horizontal")
        && matches!(kids.len(), 3 | 5)
        && kids.iter().enumerate().all(|(index, v)| {
            if index % 2 == 0 {
                column(v)
            } else {
                divider(v)
            }
        })
    {
        for index in (1..kids.len()).step_by(2) {
            let Some(rule) = rect(&kids[index], rects) else {
                continue;
            };
            for (col, side) in [(&kids[index - 1], 1), (&kids[index + 1], 3)] {
                let Some(bounds) = rect(col, rects) else {
                    continue;
                };
                let mut prose = Vec::new();
                paragraphs(col, &mut prose);
                let Some(id) = col.get("id").and_then(Value::as_str) else {
                    continue;
                };
                for body in prose {
                    let Some(text) = rect(body, rects) else {
                        continue;
                    };
                    let size = num(body, "fontSize");
                    let clear = if side == 1 {
                        rule.x - (text.x + text.w)
                    } else {
                        text.x - (rule.x + rule.w)
                    };
                    let required = size.clamp(16.0, 48.0);
                    // A healthy small gutter is a design choice, not a defect.
                    // Require near-flush geometry and enough remaining reading measure.
                    let delta = required - clear;
                    if clear >= -1.0
                        && clear < size * 0.5
                        && text.w - delta >= size * 8.0
                        && text.y < rule.y + rule.h
                        && text.y + text.h > rule.y
                        && bounds.w.is_finite()
                        && delta.is_finite()
                    {
                        let padding = if col.get("padding").is_none() {
                            Some([0.0; 4])
                        } else {
                            numeric_padding_sides(col)
                        };
                        if let Some(pad) = padding {
                            let target = pad[side] + delta;
                            let pad = fixes.entry(id.into()).or_insert(pad);
                            pad[side] = pad[side].max(target);
                        }
                    }
                }
            }
        }
    }
    for child in kids {
        collect(child, rects, fixes, false);
    }
}

pub(crate) fn repair_divided_prose_gutters(sink: &mut dyn DocSink, root_id: &str) -> usize {
    if sink.state().editor_ui.preserve_authored_geometry
        || sink.state().editor_ui.home.imported_from.is_some()
        || !matches!(
            root_design_form(sink.state(), root_id),
            DesignForm::Card | DesignForm::Deck
        )
    {
        return 0;
    }
    let Some(root) =
        op_editor_core::walkers::find_node(sink.state().active_children(), &NodeId::new(root_id))
    else {
        return 0;
    };
    let Ok(value) = serde_json::to_value(root) else {
        return 0;
    };
    if num(&value, "rotation") != 0.0 || value.get("visible") == Some(&Value::Bool(false)) {
        return 0;
    }
    let rects = resolved_rects(sink.state());
    let mut fixes = HashMap::new();
    collect(&value, &rects, &mut fixes, true);
    // Stable ordering keeps the repair ledger and replay output deterministic.
    let mut fixes: Vec<_> = fixes.into_iter().collect();
    fixes.sort_by(|a, b| a.0.cmp(&b.0));
    fixes
        .into_iter()
        .filter(|(id, padding)| {
            sink.apply(EditorCommand::PatchNodeData {
                node_id: NodeId::new(id),
                patch_json: serde_json::json!({"padding":padding}).to_string(),
                page_id: None,
            })
        })
        .count()
}

#[cfg(test)]
#[path = "editorial_column_gutters_tests.rs"]
mod tests;
