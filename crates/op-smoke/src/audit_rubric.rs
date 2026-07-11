//! Rubric metrics for the `OPENPENCIL_SMOKE_AUDIT` gate — the dimensions the
//! 07-04 G3 A/B missed. Geometry issues alone declared the loop the winner
//! while its real output shipped without mobile chrome and with a degraded
//! node vocabulary; these deterministic metrics make "chrome completeness",
//! "vocabulary richness", and "content density" first-class columns so the
//! next loop-vs-orchestrator comparison weighs what a viewer actually sees.

use std::collections::BTreeMap;

use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorState, PenNodeExt};
use serde_json::{json, Value};

/// Mobile-screen width band (390 reference ± the 320-480 devices we seed).
const MOBILE_WIDTH_RANGE: std::ops::RangeInclusive<f64> = 320.0..=480.0;

/// Build the rubric JSON for an audited document.
pub fn rubric_report(state: &EditorState) -> Value {
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    let mut text_nodes = 0usize;
    let mut icon_nodes = 0usize;
    let mut image_fills = 0usize;
    let mut screens: Vec<Value> = Vec::new();

    for root in state.active_children() {
        if !matches!(root, PenNode::Frame(_)) || root.children().is_none_or(|c| c.is_empty()) {
            continue;
        }
        collect_counts(
            root,
            &mut kinds,
            &mut text_nodes,
            &mut icon_nodes,
            &mut image_fills,
        );
        let mobile = root
            .width_px()
            .is_some_and(|w| MOBILE_WIDTH_RANGE.contains(&w));
        let has_status_bar = subtree_any(root, &is_status_bar);
        let has_bottom_nav = subtree_any(root, &is_bottom_nav);
        screens.push(json!({
            "name": root.base().name.as_deref().unwrap_or("?"),
            "width": root.width_px(),
            "mobile": mobile,
            "hasStatusBar": has_status_bar,
            "hasBottomNav": has_bottom_nav,
            // Chrome completeness is only judged where chrome is expected.
            "chromeComplete": if mobile {
                Value::Bool(has_status_bar && has_bottom_nav)
            } else {
                Value::Null
            },
        }));
    }

    // Vocabulary richness = distinct node kinds beyond the frame+text floor a
    // degraded generation collapses to (07-05 regression shipped 0/0/0
    // path/rectangle/text_input).
    let richness = kinds
        .keys()
        .filter(|k| !matches!(k.as_str(), "frame" | "text"))
        .count();

    json!({
        "screens": screens,
        "nodeKinds": kinds,
        "vocabularyRichness": richness,
        "textNodes": text_nodes,
        "iconNodes": icon_nodes,
        "imageFills": image_fills,
    })
}

fn collect_counts(
    node: &PenNode,
    kinds: &mut BTreeMap<String, usize>,
    text_nodes: &mut usize,
    icon_nodes: &mut usize,
    image_fills: &mut usize,
) {
    let kind = node_kind(node);
    *kinds.entry(kind.to_string()).or_insert(0) += 1;
    match kind {
        "text" => *text_nodes += 1,
        "icon_font" => *icon_nodes += 1,
        _ => {}
    }
    if has_image_fill(node) {
        *image_fills += 1;
    }
    for child in node.children().into_iter().flatten() {
        collect_counts(child, kinds, text_nodes, icon_nodes, image_fills);
    }
}

fn node_kind(node: &PenNode) -> &'static str {
    match node {
        PenNode::Frame(_) => "frame",
        PenNode::Group(_) => "group",
        PenNode::Rectangle(_) => "rectangle",
        PenNode::Ellipse(_) => "ellipse",
        PenNode::Line(_) => "line",
        PenNode::Polygon(_) => "polygon",
        PenNode::Path(_) => "path",
        PenNode::Text(_) => "text",
        PenNode::TextInput(_) => "text_input",
        PenNode::Image(_) => "image",
        PenNode::IconFont(_) => "icon_font",
        PenNode::Ref(_) => "ref",
        // Widget-kind additions (text_area / select / switch / …) — count
        // them under one bucket; the rubric's vocabulary signal only needs
        // "beyond frame+text", not a per-widget census.
        _ => "widget",
    }
}

/// An `image` node, an `imagePrompt`-tagged frame, or a fill carrying an
/// image source all count as one populated image slot.
fn has_image_fill(node: &PenNode) -> bool {
    if matches!(node, PenNode::Image(_)) {
        return true;
    }
    let Ok(value) = serde_json::to_value(node) else {
        return false;
    };
    if value
        .get("imagePrompt")
        .and_then(Value::as_str)
        .is_some_and(|p| !p.is_empty())
    {
        return true;
    }
    value
        .get("fill")
        .and_then(Value::as_array)
        .is_some_and(|fills| {
            fills.iter().any(|f| {
                f.get("type").and_then(Value::as_str) == Some("image")
                    || f.get("src")
                        .and_then(Value::as_str)
                        .is_some_and(|s| !s.is_empty())
            })
        })
}

fn subtree_any(node: &PenNode, pred: &dyn Fn(&PenNode) -> bool) -> bool {
    if pred(node) {
        return true;
    }
    node.children()
        .into_iter()
        .flatten()
        .any(|child| subtree_any(child, pred))
}

fn is_status_bar(node: &PenNode) -> bool {
    node.base().role.as_deref() == Some("status-bar")
        || node
            .base()
            .name
            .as_deref()
            .is_some_and(|n| n.to_ascii_lowercase().contains("status bar"))
}

fn is_bottom_nav(node: &PenNode) -> bool {
    if node.base().role.as_deref() == Some("bottom-tab-bar") {
        return true;
    }
    node.base().name.as_deref().is_some_and(|n| {
        let n = n.to_ascii_lowercase();
        n.contains("tab bar") || n.contains("tab-bar") || n.contains("bottom nav")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_from(nodes: serde_json::Value) -> EditorState {
        let doc: jian_ops_schema::PenDocument = serde_json::from_value(serde_json::json!({
            "version": "1.0",
            "children": nodes,
        }))
        .expect("doc");
        EditorState::from_document(doc)
    }

    #[test]
    fn mobile_screen_with_full_chrome_scores_complete() {
        let state = state_from(serde_json::json!([{
            "type": "frame", "id": "r", "name": "Home", "width": 390, "height": 844,
            "children": [
                { "type": "frame", "id": "sb", "name": "Status Bar", "role": "status-bar",
                  "width": "fill_container", "height": 62 },
                { "type": "icon_font", "id": "i", "name": "home", "iconFontName": "home",
                  "width": 20, "height": 20 },
                { "type": "frame", "id": "nav", "name": "bottom-tab-bar", "role": "bottom-tab-bar",
                  "width": "fill_container", "height": 72 }
            ]
        }]));
        let rubric = rubric_report(&state);
        let screen = &rubric["screens"][0];
        assert_eq!(screen["mobile"], serde_json::json!(true));
        assert_eq!(
            screen["chromeComplete"],
            serde_json::json!(true),
            "{rubric}"
        );
        assert_eq!(rubric["iconNodes"], serde_json::json!(1));
        assert!(
            rubric["vocabularyRichness"].as_u64().unwrap() >= 1,
            "{rubric}"
        );
    }

    #[test]
    fn mobile_screen_missing_chrome_scores_incomplete_and_desktop_is_exempt() {
        let state = state_from(serde_json::json!([
            { "type": "frame", "id": "m", "name": "Bare Mobile", "width": 390, "height": 844,
              "children": [ { "type": "text", "id": "t", "name": "T", "content": "hi",
                              "width": 100, "height": 20 } ] },
            { "type": "frame", "id": "d", "name": "Dashboard", "width": 1440, "height": 900,
              "children": [ { "type": "text", "id": "t2", "name": "T2", "content": "hi",
                              "width": 100, "height": 20 } ] }
        ]));
        let rubric = rubric_report(&state);
        assert_eq!(
            rubric["screens"][0]["chromeComplete"],
            serde_json::json!(false)
        );
        assert_eq!(
            rubric["screens"][1]["chromeComplete"],
            serde_json::Value::Null
        );
    }
}
