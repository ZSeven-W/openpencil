//! Keep small, informational badge groups readable before generic row clipping.
//!
//! Metadata is not a filter rail. Pack complete, intrinsically sized badges
//! into explicit lines; neither their labels nor their type size can yield.

use super::*;
use serde_json::json;

struct Reflow {
    row: String,
    lines: Vec<Vec<String>>,
    gap: f64,
}

pub(super) fn repair(sink: &mut dyn DocSink, root_id: &str, rects: &HashMap<String, Rect>) -> bool {
    let Some(root) =
        op_editor_core::walkers::find_node(sink.state().active_children(), &NodeId::new(root_id))
    else {
        return false;
    };
    let Ok(value) = serde_json::to_value(root) else {
        return false;
    };
    let mut reflows = Vec::new();
    collect(&value, rects, false, &mut reflows);
    let mut changed = false;
    for reflow in reflows {
        // Insert empty wrappers first, then move the authored badges intact.
        // The normal allocator supplies document-unique IDs; no subtree replace
        // can invalidate existing text, animation or selection references.
        let wrappers = reflow
            .lines
            .iter()
            .enumerate()
            .map(|(index, _)| {
                serde_json::from_value(json!({
                    "type":"frame", "id":format!("metadata-line-{index}"),
                    "name":"Metadata badge line", "width":"fit_content",
                    "height":"fit_content", "layout":"horizontal", "gap":reflow.gap,
                    "alignItems":"center", "children":[]
                }))
                .expect("valid metadata line")
            })
            .collect();
        let Some(ids) = sink.insert_subtree_returning_root_ids(wrappers, &NodeId::new(&reflow.row))
        else {
            continue;
        };
        if ids.len() != reflow.lines.len() {
            continue;
        }
        for (property, value) in [
            ("layout", LayoutPropValue::Keyword("vertical".into())),
            ("height", LayoutPropValue::Keyword("fit_content".into())),
            ("alignItems", LayoutPropValue::Keyword("start".into())),
            ("justifyContent", LayoutPropValue::Keyword("start".into())),
            ("gap", LayoutPropValue::Number(reflow.gap.max(8.0))),
            ("clipContent", LayoutPropValue::Bool(false)),
        ] {
            sink.apply(EditorCommand::SetNodeLayoutProp {
                node_id: NodeId::new(&reflow.row),
                property: property.into(),
                value,
            });
        }
        for (line, parent) in reflow.lines.iter().zip(ids) {
            for badge in line {
                for (property, value) in [
                    ("width", LayoutPropValue::Keyword("fit_content".into())),
                    ("clipContent", LayoutPropValue::Bool(false)),
                ] {
                    sink.apply(EditorCommand::SetNodeLayoutProp {
                        node_id: NodeId::new(badge),
                        property: property.into(),
                        value,
                    });
                }
                sink.apply(EditorCommand::MoveNode {
                    node_id: NodeId::new(badge),
                    target_parent: NodeId::new(&parent),
                    page_id: None,
                    index: None,
                });
            }
        }
        changed = true;
    }
    changed
}

fn collect(v: &Value, rects: &HashMap<String, Rect>, metadata: bool, out: &mut Vec<Reflow>) {
    // A vertical page scroll viewport may contain ordinary metadata. Only
    // horizontal lanes and explicit navigation/filter intent are protected.
    if excluded_semantics(v)
        || (has_interaction_contract(v) && !vertical_scroll_content(v))
        || layout_str(v) == Some("none")
        || is_table_shape(v)
        || is_intentional_horizontal_scroller(v, rects)
        || crate::cleanup::is_status_bar_from_json(v)
    {
        return;
    }
    let metadata = metadata || has_metadata_semantics(v);
    if metadata {
        if let Some(reflow) = candidate(v, rects) {
            out.push(reflow);
            return;
        }
    }
    for child in children(v) {
        collect(child, rects, metadata, out);
    }
}

fn semantics(v: &Value) -> String {
    format!(
        "{} {}",
        v.get("name").and_then(Value::as_str).unwrap_or(""),
        v.get("role").and_then(Value::as_str).unwrap_or("")
    )
    .to_lowercase()
}

fn has_metadata_semantics(v: &Value) -> bool {
    let name = semantics(v);
    [
        "metadata",
        "meta row",
        "meta badges",
        "difficulty",
        "attributes",
        "specifications",
        "元信息",
        "难度",
        "属性",
        "规格",
        "训练参数",
    ]
    .iter()
    .any(|word| name.contains(word))
}

fn excluded(v: &Value) -> bool {
    excluded_semantics(v) || has_interaction_contract(v)
}

fn excluded_semantics(v: &Value) -> bool {
    let name = semantics(v);
    [
        "filter",
        "category",
        "categories",
        "tabs",
        "tab bar",
        "tab-bar",
        "navigation",
        "selector",
        "segmented",
        "carousel",
        "chip rail",
        "筛选",
        "分类",
        "导航",
        "选项卡",
        "选择器",
    ]
    .iter()
    .any(|word| name.contains(word))
        || (layout_str(v) == Some("horizontal") && has_scroller_semantics(v))
}

fn has_interaction_contract(v: &Value) -> bool {
    ["events", "state", "constraints"]
        .iter()
        .any(|key| v.get(*key).is_some_and(|v| !v.is_null()))
}

fn vertical_scroll_content(v: &Value) -> bool {
    layout_str(v) == Some("vertical")
        && v.get("clipContent").and_then(Value::as_bool) == Some(true)
        && v.get("role").and_then(Value::as_str) == Some("scroll-area")
        && v.get("constraints").is_none_or(Value::is_null)
        && v.get("events").is_none_or(|events| {
            events
                .as_object()
                .is_some_and(|events| events.keys().all(|hook| hook == "onScroll"))
        })
}

fn candidate(v: &Value, rects: &HashMap<String, Rect>) -> Option<Reflow> {
    if layout_str(v) != Some("horizontal")
        || has_authored_position(v)
        || !matches!(
            v.get("height").and_then(Value::as_str),
            None | Some("fit_content")
        )
        || v.get("height").is_some_and(Value::is_number)
        || !(2..=6).contains(&children(v).len())
    {
        return None;
    }
    let id = v.get("id")?.as_str()?;
    let [_, right, _, left] = numeric_padding_sides(v)?;
    let inner = rects.get(id)?.w - left - right;
    let gap = num(v, "gap");
    if !inner.is_finite()
        || !(120.0..=480.0).contains(&inner)
        || !gap.is_finite()
        || !(0.0..=24.0).contains(&gap)
    {
        return None;
    }
    let widths = children(v)
        .iter()
        .map(|badge| intrinsic_badge_width(badge, rects))
        .collect::<Option<Vec<_>>>()?;
    if widths.iter().any(|width| *width > inner)
        || widths.iter().sum::<f64>() + gap * (widths.len() - 1) as f64 <= inner + TEXT_OVERFLOW_EPS
    {
        return None;
    }
    let mut lines = vec![Vec::new()];
    let mut used = 0.0;
    for (badge, width) in children(v).iter().zip(widths) {
        let current = lines.last().unwrap();
        if !current.is_empty() && used + gap + width > inner {
            lines.push(Vec::new());
            used = 0.0;
        }
        let current = lines.last_mut().unwrap();
        used += if current.is_empty() {
            width
        } else {
            gap + width
        };
        current.push(badge.get("id")?.as_str()?.to_owned());
    }
    Some(Reflow {
        row: id.to_owned(),
        lines,
        gap,
    })
}

fn intrinsic_badge_width(v: &Value, rects: &HashMap<String, Rect>) -> Option<f64> {
    if excluded(v)
        || has_authored_position(v)
        || layout_str(v) != Some("horizontal")
        || !crate::chip_repair::is_pill_chip(v)
        || fixed_width(v).is_some()
        || !matches!(
            v.get("width").and_then(Value::as_str),
            None | Some("fit_content" | "fill_container")
        )
        || !(20.0..=36.0).contains(&num(v, "height"))
        || !fill_is_non_empty(v)
        || !(1..=2).contains(&children(v).len())
    {
        return None;
    }
    let mut text_count = 0;
    let mut width = horizontal_padding(v);
    if !(8.0..=48.0).contains(&width) {
        return None;
    }
    let gap = num(v, "gap");
    if !(0.0..=12.0).contains(&gap) {
        return None;
    }
    for child in children(v) {
        if excluded(child) || has_authored_position(child) {
            return None;
        }
        match child.get("type")?.as_str()? {
            "text" => {
                let content = child.get("content")?.as_str()?;
                if content.contains('\n')
                    || !(1..=32).contains(&content.chars().count())
                    || !matches!(
                        child.get("width").and_then(Value::as_str),
                        None | Some("fit_content")
                    )
                    || child.get("width").is_some_and(Value::is_number)
                    || child
                        .get("textGrowth")
                        .and_then(Value::as_str)
                        .is_some_and(|v| v != "auto")
                {
                    return None;
                }
                text_count += 1;
            }
            "icon_font" => (),
            _ => return None,
        }
        let r = rects.get(child.get("id")?.as_str()?)?;
        if !r.w.is_finite()
            || !r.h.is_finite()
            || r.w <= 0.0
            || r.h <= 0.0
            || r.h > num(v, "height")
        {
            return None;
        }
        width += r.w;
    }
    (text_count == 1).then_some(width + gap * (children(v).len() - 1) as f64)
}

#[cfg(test)]
#[path = "geometry_metadata_badges_tests.rs"]
mod tests;
