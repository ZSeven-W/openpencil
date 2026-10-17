//! Explicit icon buttons and label badges need content before reaching canvas.
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
    fn badge_needs_content(node: &Value) -> bool {
        let name = node["name"].as_str().unwrap_or("").to_lowercase();
        let words: Vec<_> = name.split(|c: char| !c.is_alphanumeric()).collect();
        let role = node["role"].as_str().unwrap_or("");
        if !matches!(role, "badge" | "pill" | "chip")
            && !words
                .iter()
                .any(|word| matches!(*word, "badge" | "pill" | "chip"))
            && !name.contains("徽章")
        {
            return false;
        }
        // Solid dots, palette swatches and decorative marks are complete
        // without a label. Do not turn graphic-only badges into invented copy.
        let tiny_mark = node["width"]
            .as_f64()
            .zip(node["height"].as_f64())
            .is_some_and(|(w, h)| w > 0.0 && h > 0.0 && w <= 16.0 && h <= 16.0);
        !tiny_mark
            && !matches!(role, "status-dot" | "color-swatch" | "indicator")
            && !words.iter().any(|word| {
                matches!(
                    *word,
                    "dot"
                        | "swatch"
                        | "indicator"
                        | "pattern"
                        | "decorative"
                        | "decoration"
                        | "color"
                        | "colour"
                )
            })
            && !["状态点", "色板", "色块", "图案", "装饰"]
                .iter()
                .any(|s| name.contains(s))
    }
    fn badge_drawing_layer(node: &Value) -> bool {
        let name = node["name"].as_str().unwrap_or("").to_lowercase();
        let named_layer = name
            .split(|c: char| !c.is_alphanumeric())
            .any(|word| matches!(word, "background" | "bg" | "outline" | "border"))
            || ["背景", "轮廓", "边框"]
                .iter()
                .any(|word| name.contains(word));
        let painted = ["fill", "stroke"].iter().any(|key| match &node[*key] {
            Value::Array(paint) => !paint.is_empty(),
            Value::Object(paint) => !paint.is_empty(),
            _ => false,
        });
        named_layer
            && painted
            && node["events"]
                .as_object()
                .is_none_or(|events| events.is_empty())
    }
    fn collect(
        node: &Value,
        inside_complete_badge: bool,
        icons: &mut Vec<String>,
        badges: &mut Vec<String>,
    ) {
        if !visible(node) {
            return;
        }
        let has_content = content(node);
        let is_badge = node["type"] == "frame" && badge_needs_content(node);
        if node["type"] == "frame" && !has_content {
            if node["role"] == "icon-button" {
                icons.push(node["name"].as_str().unwrap_or("icon-button").to_string());
            } else if is_badge && !(inside_complete_badge && badge_drawing_layer(node)) {
                badges.push(node["name"].as_str().unwrap_or("badge").to_string());
            }
        }
        if let Some(children) = node["children"].as_array() {
            for child in children {
                // A complete badge can contain separately painted background
                // layers. This context exempts only explicitly named drawing
                // layers, never an empty sibling badge or a standalone shell.
                collect(
                    child,
                    inside_complete_badge || (is_badge && has_content),
                    icons,
                    badges,
                );
            }
        }
    }
    let mut icons = Vec::new();
    let mut badges = Vec::new();
    for node in nodes {
        collect(
            &serde_json::to_value(node).ok()?,
            false,
            &mut icons,
            &mut badges,
        );
    }
    if !icons.is_empty() {
        return Some(format!(
        "self-check failed: empty-icon-button: {}. Regenerate this section with a visible icon or label INSIDE each named control. Empty rounded frames are not buttons; preserve the other content and layout.",
        icons.join(", ")
    ));
    }
    (!badges.is_empty()).then(|| format!(
        "self-check failed: empty-label-badge: {}. Regenerate this section with the planned label or icon INSIDE each named badge/pill/chip. Restore the original plan's content, not invented replacement text; preserve the other content and layout.",
        badges.join(", ")
    ))
}
