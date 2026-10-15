//! An explicit icon button needs visible content before it reaches the canvas.
//! Run after orphan adoption, so a recoverable sibling icon is not rejected.

use jian_ops_schema::node::PenNode;
use serde_json::Value;

pub(super) fn rejection(nodes: &[PenNode]) -> Option<String> {
    fn visible(node: &Value) -> bool {
        node["visible"].as_bool() != Some(false)
            && node["enabled"].as_bool() != Some(false)
            && node["opacity"].as_f64() != Some(0.0)
    }
    fn image_paint(node: &Value) -> bool {
        node["fill"]
            .as_array()
            .is_some_and(|fills| fills.iter().any(|fill| fill["type"] == "image"))
    }
    fn content(node: &Value) -> bool {
        if !visible(node) {
            return false;
        }
        if image_paint(node) {
            return true;
        }
        match node["type"].as_str() {
            Some("frame" | "group") => node["children"]
                .as_array()
                .is_some_and(|children| children.iter().any(content)),
            Some("text") => node["content"]
                .as_str()
                .is_none_or(|s| !s.trim().is_empty()),
            Some("icon_font") => node["iconFontName"]
                .as_str()
                .is_some_and(|s| !s.trim().is_empty()),
            // Custom paths, shapes, image slots, instances and widgets can
            // legitimately draw the glyph. Do not prescribe its appearance.
            _ => true,
        }
    }
    fn collect(node: &Value, out: &mut Vec<String>) {
        if !visible(node) {
            return;
        }
        if node["type"] == "frame" && node["role"] == "icon-button" && !content(node) {
            out.push(node["name"].as_str().unwrap_or("icon-button").to_string());
        }
        if let Some(children) = node["children"].as_array() {
            for child in children {
                collect(child, out);
            }
        }
    }
    let mut empty = Vec::new();
    for node in nodes {
        collect(&serde_json::to_value(node).ok()?, &mut empty);
    }
    (!empty.is_empty()).then(|| format!(
        "self-check failed: empty-icon-button: {}. Regenerate this section with a visible icon or label INSIDE each named control. Empty rounded frames are not buttons; preserve the other content and layout.",
        empty.join(", ")
    ))
}
