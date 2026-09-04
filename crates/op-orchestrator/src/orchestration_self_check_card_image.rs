//! Request-aware raster-intent gate for social-card generation.

use super::*;

/// A social-card prompt that supplies only text must never gain a
/// model-invented raster slot. Once a slot exists the host's image enrichment
/// correctly treats it as intent and may fall back to stock search, which is
/// too late to recover the user's original no-image request.
pub(crate) fn check_generated_nodes_for_prompt(
    nodes: &[PenNode],
    canvas_width: f64,
    prompt: &str,
) -> SelfCheckReport {
    let mut report = check_generated_nodes(nodes, canvas_width);
    if is_text_only_social_card_prompt(prompt) {
        let value = serde_json::to_value(nodes).unwrap_or(Value::Null);
        if let Some(node_id) = first_raster_asset_node_id(&value) {
            report.issues.push(SelfCheckIssue {
                code: "unsolicited-card-image",
                node_id,
                message: "a text-only social card must not create image nodes, imageSearchQuery/imagePrompt slots, image fills, or stock-search backgrounds; use the fixed board itself with typography, colour fields, vector paths/icons, rules, and shapes unless the user explicitly requests raster media".into(),
                severity: SelfCheckSeverity::Fatal,
            });
        }
    }
    report
}

fn is_text_only_social_card_prompt(prompt: &str) -> bool {
    if crate::design_type::detect_design_type(prompt).type_ != crate::design_type::DesignType::Card
    {
        return false;
    }
    let lower = prompt.to_lowercase();
    let explicit_no_image = [
        "纯文字",
        "不要图片",
        "不要照片",
        "无需图片",
        "不需要图片",
        "不使用图片",
        "禁止图片",
        "text-only",
        "text only",
        "no image",
        "no photo",
        "without image",
        "without photo",
    ]
    .iter()
    .any(|marker| lower.contains(marker));
    if explicit_no_image {
        return true;
    }
    let raster_intent = [
        "图片",
        "图像",
        "照片",
        "摄影",
        "插画",
        "配图",
        "背景图",
        "纹理",
        "image",
        "photo",
        "photograph",
        "picture",
        "illustration",
        "texture",
    ]
    .iter()
    .any(|marker| crate::design_type::contains_word(&lower, marker));
    !raster_intent
}

fn first_raster_asset_node_id(value: &Value) -> Option<Option<String>> {
    match value {
        Value::Array(nodes) => nodes.iter().find_map(first_raster_asset_node_id),
        Value::Object(node) => {
            let is_image_node = string_prop(value, "type") == Some("image");
            let has_image_intent = ["imageSearchQuery", "imagePrompt"]
                .iter()
                .any(|key| node.get(*key).is_some_and(nonempty_value));
            let has_image_fill = node
                .get("fill")
                .and_then(Value::as_array)
                .is_some_and(|fills| {
                    fills
                        .iter()
                        .any(|fill| string_prop(fill, "type") == Some("image"))
                });
            if is_image_node || has_image_intent || has_image_fill {
                return Some(string_prop(value, "id").map(str::to_string));
            }
            node.get("children")
                .and_then(Value::as_array)
                .and_then(|children| children.iter().find_map(first_raster_asset_node_id))
        }
        _ => None,
    }
}

fn nonempty_value(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(value) => !value.trim().is_empty(),
        _ => true,
    }
}
