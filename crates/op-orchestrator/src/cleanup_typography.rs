//! Typography cleanup for recurring LLM over-bolding patterns.

use crate::types::DocSink;
use jian_ops_schema::node::{FontWeight, PenNode, TextContent};
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextWeightFix {
    pub node_id: NodeId,
    pub font_weight: u16,
}

#[derive(Debug)]
struct TextCandidate {
    node_id: NodeId,
    font_size: f64,
    font_weight: u16,
    haystack: String,
    content_len: usize,
}

pub(crate) fn collect_overbold_text_weight_fixes(root: &PenNode) -> Vec<TextWeightFix> {
    let mut candidates = Vec::new();
    collect_text_candidates(root, String::new(), &mut candidates);

    let weighted = candidates.len();
    if weighted < 4 {
        return Vec::new();
    }

    let boldish = candidates
        .iter()
        .filter(|candidate| candidate.font_weight >= 700)
        .count();
    let has_overbold_non_heading = candidates
        .iter()
        .any(|candidate| candidate.font_weight >= 700 && !is_heading_like(candidate));
    if (boldish as f64 / weighted as f64) < 0.65 || !has_overbold_non_heading {
        return Vec::new();
    }

    candidates
        .into_iter()
        .filter_map(|candidate| {
            let target = target_weight(&candidate);
            (candidate.font_weight > target).then_some(TextWeightFix {
                node_id: candidate.node_id,
                font_weight: target,
            })
        })
        .collect()
}

pub(crate) fn repair_overbold_text_hierarchy(sink: &mut dyn DocSink, root_id: &str) {
    let fixes = {
        let Some(root) = sink
            .state()
            .active_children()
            .iter()
            .find(|n| n.id_str() == root_id)
        else {
            return;
        };
        collect_overbold_text_weight_fixes(root)
    };

    for fix in fixes {
        sink.apply(EditorCommand::SetNodeFontWeight {
            node_id: fix.node_id,
            font_weight: fix.font_weight,
        });
    }
}

fn collect_text_candidates(
    node: &PenNode,
    ancestor_haystack: String,
    out: &mut Vec<TextCandidate>,
) {
    let own_haystack = node_haystack(node);
    let combined = format!("{ancestor_haystack} {own_haystack}");

    if let PenNode::Text(text) = node {
        if let Some(font_weight) = font_weight_number(text.font_weight.as_ref()) {
            let content = text_content(&text.content);
            out.push(TextCandidate {
                node_id: NodeId::new(node.id_str().to_string()),
                font_size: text.font_size.unwrap_or(14.0),
                font_weight,
                haystack: format!("{} {}", combined, content.to_lowercase()),
                content_len: content.chars().count(),
            });
        }
    }

    if let Some(children) = node.children() {
        for child in children {
            collect_text_candidates(child, combined.clone(), out);
        }
    }
}

fn node_haystack(node: &PenNode) -> String {
    [
        node.id_str(),
        node.base().name.as_deref().unwrap_or(""),
        node.base().role.as_deref().unwrap_or(""),
    ]
    .join(" ")
    .to_lowercase()
}

fn font_weight_number(weight: Option<&FontWeight>) -> Option<u16> {
    match weight? {
        FontWeight::Number(n) => u16::try_from(*n).ok(),
        FontWeight::Keyword(keyword) => match keyword.to_lowercase().as_str() {
            "thin" => Some(100),
            "light" => Some(300),
            "normal" | "regular" => Some(400),
            "medium" => Some(500),
            "semibold" | "semi-bold" | "demibold" => Some(600),
            "bold" => Some(700),
            "heavy" | "black" => Some(900),
            _ => None,
        },
    }
}

fn text_content(content: &TextContent) -> String {
    match content {
        TextContent::Plain(text) => text.clone(),
        TextContent::Styled(segments) => segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>()
            .join(""),
    }
}

fn target_weight(candidate: &TextCandidate) -> u16 {
    if is_heading_like(candidate) {
        return 700;
    }
    if is_button_label(candidate) {
        return 600;
    }
    if is_interactive_label(candidate) {
        return 500;
    }
    if is_body_like(candidate) || candidate.content_len > 18 {
        return 400;
    }
    if candidate.font_size <= 14.0 {
        return 400;
    }
    if candidate.font_size <= 17.0 {
        return 500;
    }
    if candidate.font_size <= 22.0 {
        return 600;
    }
    700
}

fn is_heading_like(candidate: &TextCandidate) -> bool {
    if is_body_like(candidate) {
        return false;
    }
    candidate.font_size >= 24.0
        || contains_any(
            &candidate.haystack,
            &[
                " heading",
                "headline",
                " title",
                "title ",
                "section-title",
                "restaurant name",
                "location",
            ],
        )
}

fn is_button_label(candidate: &TextCandidate) -> bool {
    contains_any(
        &candidate.haystack,
        &[" button", "cta", "call to action", "order"],
    )
}

fn is_interactive_label(candidate: &TextCandidate) -> bool {
    contains_any(
        &candidate.haystack,
        &[
            "chip", "badge", "pill", "tag", "tab", "nav", "filter", "category",
        ],
    )
}

fn is_body_like(candidate: &TextCandidate) -> bool {
    contains_any(
        &candidate.haystack,
        &[
            "body-text",
            "body",
            "caption",
            "subtitle",
            "description",
            "placeholder",
            "metadata",
            "meta",
            "address",
            "helper",
            "secondary",
            "muted",
            "minute",
            " min",
            "price",
            "rating",
        ],
    )
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detects_overbold_mobile_food_screen_and_downgrades_non_headings() {
        let root: PenNode = serde_json::from_value(json!({
            "type": "frame",
            "id": "root",
            "name": "Food App Home",
            "width": 390,
            "height": 844,
            "children": [
                {
                    "type": "text",
                    "id": "headline",
                    "name": "Popular Restaurants",
                    "role": "heading",
                    "content": "Popular Restaurants",
                    "fontSize": 30,
                    "fontWeight": 800
                },
                {
                    "type": "text",
                    "id": "subtitle",
                    "name": "Promo Subtitle",
                    "role": "body-text",
                    "content": "Fresh Brooklyn favorites, delivered fast.",
                    "fontSize": 16,
                    "fontWeight": 800
                },
                {
                    "type": "frame",
                    "id": "search",
                    "name": "Search Bar",
                    "children": [{
                        "type": "text",
                        "id": "placeholder",
                        "name": "Placeholder",
                        "content": "Search restaurants or dishes",
                        "fontSize": 17,
                        "fontWeight": 800
                    }]
                },
                {
                    "type": "frame",
                    "id": "cta",
                    "name": "Order Button",
                    "role": "button",
                    "children": [{
                        "type": "text",
                        "id": "cta-label",
                        "name": "CTA Label",
                        "role": "label",
                        "content": "Order now",
                        "fontSize": 16,
                        "fontWeight": 800
                    }]
                },
                {
                    "type": "text",
                    "id": "metadata",
                    "name": "Delivery Meta",
                    "role": "caption",
                    "content": "20-30 min",
                    "fontSize": 14,
                    "fontWeight": 800
                }
            ]
        }))
        .expect("root json");

        let fixes = collect_overbold_text_weight_fixes(&root);
        let pairs = fixes
            .iter()
            .map(|fix| (fix.node_id.as_str(), fix.font_weight))
            .collect::<Vec<_>>();

        assert!(
            pairs.contains(&("headline", 700)),
            "display headings may stay bold but should not remain black-weight"
        );
        assert!(pairs.contains(&("subtitle", 400)));
        assert!(pairs.contains(&("placeholder", 400)));
        assert!(pairs.contains(&("cta-label", 600)));
        assert!(pairs.contains(&("metadata", 400)));
    }

    #[test]
    fn does_not_rewrite_when_only_headings_are_bold() {
        let root: PenNode = serde_json::from_value(json!({
            "type": "frame",
            "id": "root",
            "name": "Editorial",
            "width": 1200,
            "height": 800,
            "children": [
                {
                    "type": "text",
                    "id": "hero",
                    "role": "heading",
                    "content": "Brooklyn After Dark",
                    "fontSize": 56,
                    "fontWeight": 800
                },
                {
                    "type": "text",
                    "id": "body",
                    "role": "body-text",
                    "content": "A quiet guide to the neighborhood.",
                    "fontSize": 18,
                    "fontWeight": 400
                },
                {
                    "type": "text",
                    "id": "caption",
                    "role": "caption",
                    "content": "Updated tonight",
                    "fontSize": 13,
                    "fontWeight": 400
                }
            ]
        }))
        .expect("root json");

        assert!(collect_overbold_text_weight_fixes(&root).is_empty());
    }
}
