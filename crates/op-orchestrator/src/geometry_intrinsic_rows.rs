//! Reflow compact, indivisible information groups before generic flex repairs.
//!
//! A delivery ETA and a three-step tracker can each fit a phone rail, but not
//! side by side. Making the tracker and then its labels fill the leftover
//! sliver produces vertical CJK text. Stack the two groups while they still
//! retain their intrinsic widths. This is not a generic mobile-column rewrite.

use super::*;

pub(super) fn collect_reflows(
    root: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    visit(root, rects, cmds, false);
}

fn visit(v: &Value, rects: &HashMap<String, Rect>, cmds: &mut Vec<EditorCommand>, in_table: bool) {
    // Tables keep aligned columns; intentional lanes and positioned/chrome
    // subtrees retain their authored arrangement, including descendants.
    if in_table
        || has_scroller_semantics(v)
        || is_intentional_horizontal_scroller(v, rects)
        || crate::cleanup::is_status_bar_from_json(v)
        || layout_str(v) == Some("none")
    {
        return;
    }
    let table = is_table_shape(v);
    if !table && layout_str(v) == Some("horizontal") && !has_authored_position(v) {
        if let Some(inner) = rects
            .get(v.get("id").and_then(Value::as_str).unwrap_or_default())
            .map(|r| r.w - horizontal_padding(v))
            .filter(|width| *width > 0.0 && *width <= 480.0)
        {
            if let Some((leading, badge)) = compact_trailing_status_pair(v, rects) {
                let required = [leading, badge]
                    .iter()
                    .try_fold(num(v, "gap"), |sum, node| {
                        Some(sum + rects.get(node.get("id")?.as_str()?)?.w)
                    });
                if required.is_some_and(|width| width > inner + OVERFLOW_EPS) {
                    stack_compact_status_header(v, leading, badge, cmds);
                    return;
                }
            }
            if fits_as_separate_groups(v, rects, inner) {
                let id = v.get("id").and_then(Value::as_str).unwrap();
                for (property, value) in [
                    ("layout", LayoutPropValue::Keyword("vertical".into())),
                    ("height", LayoutPropValue::Keyword("fit_content".into())),
                    ("alignItems", LayoutPropValue::Keyword("start".into())),
                    ("justifyContent", LayoutPropValue::Keyword("start".into())),
                    ("gap", LayoutPropValue::Number(num(v, "gap").max(12.0))),
                ] {
                    cmds.push(EditorCommand::SetNodeLayoutProp {
                        node_id: NodeId::new(id.to_owned()),
                        property: property.into(),
                        value,
                    });
                }
                return;
            }
        }
    }
    for child in children(v) {
        visit(child, rects, cmds, table);
    }
}

fn fits_as_separate_groups(v: &Value, rects: &HashMap<String, Rect>, inner: f64) -> bool {
    if v.get("clipContent").and_then(Value::as_bool) == Some(true)
        || v.get("height")
            .is_some_and(|h| h.as_str() != Some("fit_content"))
    {
        return false;
    }
    let [a, b] = children(v) else { return false };
    let groups = [a, b];
    if !groups.iter().all(|group| {
        layout_str(group) == Some("horizontal")
            && group.get("width").and_then(Value::as_str) == Some("fit_content")
            && !fill_is_non_empty(group)
            && bears_text(group)
            && rigid_inline_content(group, 0)
    }) {
        return false;
    }
    // At least one group is compound (e.g. step chips plus connectors).
    // Two plain text columns can wrap normally and are not this contract.
    if !groups.iter().any(|group| {
        children(group)
            .iter()
            .any(|c| layout_str(c) == Some("horizontal"))
    }) {
        return false;
    }
    let widths: Option<Vec<f64>> = groups
        .iter()
        .map(|group| {
            let rect = rects.get(group.get("id")?.as_str()?)?;
            (rect.h > 0.0 && rect.h <= 80.0 && rect.w > 0.0).then_some(rect.w)
        })
        .collect();
    widths.is_some_and(|widths| {
        widths
            .iter()
            .all(|width| *width <= inner + TEXT_OVERFLOW_EPS)
            && widths.iter().sum::<f64>() + num(v, "gap") > inner + OVERFLOW_EPS
    })
}

fn rigid_inline_content(v: &Value, depth: usize) -> bool {
    if depth > 4
        || has_authored_position(v)
        || v.get("width").and_then(Value::as_str) == Some("fill_container")
        || ["events", "scroll", "overflow"]
            .iter()
            .any(|key| v.get(*key).is_some())
    {
        return false;
    }
    match v.get("type").and_then(Value::as_str) {
        Some("frame" | "group") => {
            layout_str(v) == Some("horizontal")
                && !crate::chip_repair::is_pill_chip(v)
                && !children(v).is_empty()
                && children(v)
                    .iter()
                    .all(|c| rigid_inline_content(c, depth + 1))
        }
        Some("text") => v
            .get("content")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.contains('\n') && s.chars().count() <= 40),
        Some("icon_font" | "ellipse" | "rectangle" | "line") => true,
        _ => false,
    }
}

#[cfg(test)]
#[path = "geometry_intrinsic_rows_tests.rs"]
mod tests;
