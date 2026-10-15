//! Geometry helpers for compact status badges in narrow card headers.
//!
//! Kept separate from the generic geometry pass because these helpers form one
//! transaction: recognize the badge, keep it Hug, and reflow the whole header
//! when its title and badge cannot coexist on one line.

use super::*;

/// A small painted status badge by authored anatomy. Models frequently omit an
/// explicit height and corner radius from these controls, so the stricter
/// `is_pill_chip` shape cannot recognize them. Requiring a direct <=12px status
/// dot plus one short label keeps this much narrower than a generic button.
pub(super) fn is_compact_status_badge_structure(v: &Value) -> bool {
    if !matches!(
        v.get("type").and_then(Value::as_str),
        Some("frame" | "group")
    ) || layout_str(v) != Some("horizontal")
        || !fill_is_non_empty(v)
        || horizontal_padding(v) <= 0.0
    {
        return false;
    }
    let [a, b] = children(v) else {
        return false;
    };
    let is_short_label = |child: &Value| {
        child.get("type").and_then(Value::as_str) == Some("text")
            && child
                .get("content")
                .and_then(Value::as_str)
                .map(str::trim)
                .is_some_and(|label| !label.is_empty() && label.chars().count() <= 16)
    };
    let is_status_dot = |child: &Value| {
        child.get("type").and_then(Value::as_str) == Some("ellipse")
            && fixed_width(child).is_some_and(|width| width > 0.0 && width <= 12.0)
            && num(child, "height") > 0.0
            && num(child, "height") <= 12.0
    };
    (is_short_label(a) && is_status_dot(b)) || (is_status_dot(a) && is_short_label(b))
}

pub(super) fn is_compact_status_badge(v: &Value, rects: &HashMap<String, Rect>) -> bool {
    if !is_compact_status_badge_structure(v) {
        return false;
    }
    let Some(rect) = v
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| rects.get(id))
    else {
        return false;
    };
    if rect.w <= 0.0 || rect.h <= 0.0 {
        return false;
    }
    let healthy_compact_geometry = rect.w <= 120.0 && rect.h <= 48.0;
    // Do not cap a proven damaged state by its CURRENT height. A 6px fixed
    // label made "Good" 62px tall; a longer status can balloon further. The
    // exact dot + short-label structure and explicit bad sizing marker are the
    // recovery proof, while healthy badges retain the strict compact bounds.
    healthy_compact_geometry || status_badge_has_damaged_marker(v)
}

/// Exact two-part mobile-card header anatomy: a text-bearing title frame
/// followed by a compact painted status badge, or a direct title text followed
/// by that same badge. A direct title does not require space_between: normal
/// gapped name/status rows have the same intrinsic width constraint.
///
/// Keeping this structural gate shared across the text, frame, gap, and
/// overfull repairers prevents those independent passes from issuing
/// contradictory commands for the same trailing badge.
pub(super) fn compact_trailing_status_pair<'a>(
    v: &'a Value,
    rects: &HashMap<String, Rect>,
) -> Option<(&'a Value, &'a Value)> {
    if layout_str(v) != Some("horizontal") {
        return None;
    }
    let [leading, tail] = children(v) else {
        return None;
    };
    let direct_text = leading.get("type").and_then(Value::as_str) == Some("text");
    if !direct_text && v.get("justifyContent").and_then(Value::as_str) != Some("space_between") {
        return None;
    }
    if has_authored_position(leading)
        || has_authored_position(tail)
        || !matches!(
            leading.get("type").and_then(Value::as_str),
            Some("frame" | "group" | "text")
        )
        || !bears_text(leading)
        || !is_compact_status_badge(tail, rects)
    {
        return None;
    }
    let leading_h = leading
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| rects.get(id))
        .map(|rect| rect.h)?;
    (leading_h > 0.0 && leading_h <= 48.0).then_some((leading, tail))
}

pub(super) fn compact_status_header_is_damaged(leading: &Value, tail: &Value) -> bool {
    let fill_width =
        |node: &Value| node.get("width").and_then(Value::as_str) == Some("fill_container");
    let wrapped_text = |node: &Value| {
        children(node).iter().any(|child| {
            child.get("type").and_then(Value::as_str) == Some("text")
                && (fill_width(child)
                    || child
                        .get("textGrowth")
                        .and_then(Value::as_str)
                        .is_some_and(|growth| growth.starts_with("fixed-width")))
        })
    };
    wrapped_text(leading) || wrapped_text(tail) || status_badge_has_damaged_marker(tail)
}

pub(super) fn stack_compact_status_header(
    row: &Value,
    leading: &Value,
    tail: &Value,
    cmds: &mut Vec<EditorCommand>,
) {
    let Some(row_id) = row.get("id").and_then(Value::as_str) else {
        return;
    };
    for (property, value) in [
        ("layout", LayoutPropValue::Keyword("vertical".to_string())),
        (
            "height",
            LayoutPropValue::Keyword("fit_content".to_string()),
        ),
        (
            "justifyContent",
            LayoutPropValue::Keyword("start".to_string()),
        ),
        ("alignItems", LayoutPropValue::Keyword("start".to_string())),
        ("gap", LayoutPropValue::Number(8.0)),
    ] {
        cmds.push(EditorCommand::SetNodeLayoutProp {
            node_id: NodeId::new(row_id.to_string()),
            property: property.to_string(),
            value,
        });
    }
    for container in [leading, tail] {
        if let Some(id) = container.get("id").and_then(Value::as_str) {
            cmds.push(EditorCommand::SetNodeLayoutProp {
                node_id: NodeId::new(id.to_string()),
                property: "width".to_string(),
                value: LayoutPropValue::Keyword("fit_content".to_string()),
            });
        }
        if container.get("type").and_then(Value::as_str) == Some("text") {
            restore_hug_text(container, cmds);
        }
        for child in children(container)
            .iter()
            .filter(|child| child.get("type").and_then(Value::as_str) == Some("text"))
        {
            restore_hug_text(child, cmds);
        }
    }
}

fn restore_hug_text(text: &Value, cmds: &mut Vec<EditorCommand>) {
    let Some(id) = text.get("id").and_then(Value::as_str) else {
        return;
    };
    if text.get("width").is_some()
        && text.get("width").and_then(Value::as_str) != Some("fit_content")
    {
        cmds.push(EditorCommand::SetNodeLayoutProp {
            node_id: NodeId::new(id.to_string()),
            property: "width".to_string(),
            value: LayoutPropValue::Keyword("fit_content".to_string()),
        });
    }
    if text
        .get("textGrowth")
        .and_then(Value::as_str)
        .is_some_and(|growth| growth.starts_with("fixed-width"))
    {
        cmds.push(EditorCommand::SetNodeLayoutProp {
            node_id: NodeId::new(id.to_string()),
            property: "textGrowth".to_string(),
            value: LayoutPropValue::Keyword("auto".to_string()),
        });
    }
}

fn status_badge_has_damaged_marker(v: &Value) -> bool {
    let tail_fill = v.get("width").and_then(Value::as_str) == Some("fill_container");
    let label_damaged = children(v).iter().any(|child| {
        if child.get("type").and_then(Value::as_str) != Some("text") {
            return false;
        }
        let explicit_non_hug_width = child.get("width").is_some()
            && child.get("width").and_then(Value::as_str) != Some("fit_content");
        let fixed_growth = child
            .get("textGrowth")
            .and_then(Value::as_str)
            .is_some_and(|growth| growth.starts_with("fixed-width"));
        explicit_non_hug_width || fixed_growth
    });
    tail_fill || label_damaged
}
