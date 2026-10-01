//! Keep a bottom-anchored composer (reply box, comment input, chat input)
//! inside the clipped pane that holds it.
//!
//! Measured, arena-w02 (Opus 5.5, 2026-10-01, both runs): the 1440×900 mail
//! client's reading pane sits in a `clipContent` column that fills the root's
//! height. The model wrote four body paragraphs and then the reply box the
//! brief asked for; the body pushed the reply box below the column's bottom
//! edge, the clip hid it, and the defects judge scored the design for a
//! missing reply box that was in the document all along. A real client
//! scrolls the message and pins the composer, so the repair does the same:
//! every container between the pane and the composer fills the pane's height,
//! and the tallest block before the composer takes the remaining space and
//! clips — the composer stays on screen at the bottom.

use super::*;
use crate::cleanup::is_status_bar_from_json;

/// Slack before a composer's bottom counts as cut by the clip edge.
const CUT_EPS: f64 = 1.0;
/// The body that yields its height must be a real block, not a divider.
const MIN_BODY_HEIGHT: f64 = 40.0;

const COMPOSER_CUES_ASCII: &[&str] = &[
    "reply",
    "composer",
    "compose box",
    "message input",
    "chat input",
    "comment input",
    "comment box",
    "input bar",
];
const COMPOSER_CUES_CJK: &[&str] = &["回复", "输入框", "输入栏", "评论框"];
/// Parts of a composer, or controls that merely mention one.
const NOT_A_COMPOSER_CUES: &[&str] = &["button", "btn", "icon", "label", "link", "action"];

pub(super) fn collect_pinned_composer_fixes(
    root: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    for child in children(root) {
        visit(child, rects, cmds);
    }
}

fn visit(v: &Value, rects: &HashMap<String, Rect>, cmds: &mut Vec<EditorCommand>) {
    if is_status_bar_from_json(v) {
        return;
    }
    if let Some(fixes) = pin_composer(v, rects) {
        cmds.extend(fixes);
        return;
    }
    for child in children(v) {
        visit(child, rects, cmds);
    }
}

fn identity(v: &Value) -> String {
    ["name", "id"]
        .iter()
        .filter_map(|key| v.get(*key).and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
        .replace(['-', '_'], " ")
}

fn is_composer(v: &Value) -> bool {
    if !matches!(
        v.get("type").and_then(Value::as_str),
        Some("frame" | "text_input")
    ) {
        return false;
    }
    let identity = identity(v);
    if NOT_A_COMPOSER_CUES
        .iter()
        .any(|cue| identity.split_whitespace().any(|word| word == *cue))
    {
        return false;
    }
    COMPOSER_CUES_ASCII.iter().any(|cue| identity.contains(cue))
        || COMPOSER_CUES_CJK.iter().any(|cue| identity.contains(cue))
}

/// Path from a child of `v` down to the first composer in document order.
fn composer_path(v: &Value) -> Option<Vec<&Value>> {
    for child in children(v) {
        if is_status_bar_from_json(child) {
            continue;
        }
        if is_composer(child) {
            return Some(vec![child]);
        }
        if let Some(mut rest) = composer_path(child) {
            rest.insert(0, child);
            return Some(rest);
        }
    }
    None
}

fn node_id(v: &Value) -> Option<NodeId> {
    v.get("id")
        .and_then(Value::as_str)
        .map(|id| NodeId::new(id.to_string()))
}

fn keyword(node: NodeId, property: &str, value: &str) -> EditorCommand {
    EditorCommand::SetNodeLayoutProp {
        node_id: node,
        property: property.to_string(),
        value: LayoutPropValue::Keyword(value.to_string()),
    }
}

/// The edits that pin `v`'s composer, when `v` is a clipped vertical pane
/// whose bottom edge hides the composer it holds.
fn pin_composer(v: &Value, rects: &HashMap<String, Rect>) -> Option<Vec<EditorCommand>> {
    if v.get("clipContent").and_then(Value::as_bool) != Some(true)
        || layout_str(v) != Some("vertical")
    {
        return None;
    }
    let pane = rects.get(v.get("id").and_then(Value::as_str)?)?;
    let pane_bottom = pane.y + pane.h;
    let path = composer_path(v)?;
    let composer = *path.last()?;
    let cut = rects
        .get(composer.get("id").and_then(Value::as_str)?)
        .is_some_and(|r| r.y + r.h > pane_bottom + CUT_EPS);
    if !cut {
        return None;
    }
    let parent = if path.len() >= 2 {
        path[path.len() - 2]
    } else {
        v
    };
    if layout_str(parent) != Some("vertical") {
        return None;
    }
    let siblings = children(parent);
    let at = siblings
        .iter()
        .position(|s| s.get("id") == composer.get("id"))?;
    // Bottom-anchored only: a composer with content after it is not a footer.
    if at + 1 != siblings.len() {
        return None;
    }
    let body = siblings[..at]
        .iter()
        .filter_map(|s| {
            let r = rects.get(s.get("id").and_then(Value::as_str)?)?;
            Some((s, r.h))
        })
        .filter(|(_, h)| *h >= MIN_BODY_HEIGHT)
        .max_by(|a, b| a.1.total_cmp(&b.1))?
        .0;

    let mut fixes = Vec::new();
    for container in &path[..path.len() - 1] {
        if container.get("height").and_then(Value::as_str) != Some("fill_container") {
            fixes.push(keyword(node_id(container)?, "height", "fill_container"));
        }
    }
    let body_id = node_id(body)?;
    fixes.push(keyword(body_id.clone(), "height", "fill_container"));
    if body.get("clipContent").and_then(Value::as_bool) != Some(true) {
        fixes.push(EditorCommand::SetNodeLayoutProp {
            node_id: body_id,
            property: "clipContent".to_string(),
            value: LayoutPropValue::Bool(true),
        });
    }
    Some(fixes)
}

#[cfg(test)]
#[path = "geometry_pinned_composer_tests.rs"]
mod tests;
