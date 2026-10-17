//! Narrow pre-insert guards for unrequested, explicitly identifiable facts.
//!
//! This is not a general fact checker. Ambiguous/reference-derived content is
//! left alone; a rejected forest is regenerated, never edited or pruned here.

use crate::{plan::Subtask, types::DesignRequest};
use jian_ops_schema::node::{PenNode, TextContent};
use op_design_lint::node_util::{is_node_visible, opacity};
use op_editor_core::PenNodeExt;

#[path = "source_facts_order.rs"]
mod order;

#[path = "source_facts_delivery.rs"]
mod delivery;

#[path = "source_facts_exercise.rs"]
mod exercise;

fn rating_terms(text: &str) -> bool {
    let lower = text.to_lowercase();
    ["rating", "ratings"]
        .iter()
        .any(|word| crate::design_type::contains_word(&lower, word))
        || ["评分", "星级"].iter().any(|word| lower.contains(word))
}

fn may_check(request: &DesignRequest) -> bool {
    let lower = request.prompt.to_lowercase();
    // Mentioning a rating/review field is already sufficient to avoid the
    // absence-based inference. Validating supplied values is a separate task.
    if rating_terms(&lower)
        || lower.contains("评价")
        || lower.contains(['★', '☆'])
        || ["星", "满意度", "口碑", "好评"]
            .iter()
            .any(|word| lower.contains(word))
        || ["review", "reviews", "score", "scores", "star", "stars"]
            .iter()
            .any(|word| crate::design_type::contains_word(&lower, word))
    {
        return false;
    }
    text_only_source(request)
}

fn text_only_source(request: &DesignRequest) -> bool {
    let lower = request.prompt.to_lowercase();
    // The request does not carry the visible text of attachments/live source
    // documents. Do not call their content invented based on brief text alone.
    // A reference_skeleton alone is deliberately NOT a source: it is content-free.
    request.append_context.is_none()
        && request.continuation_context.is_none()
        && ![
            "http://",
            "https://",
            "参考",
            "原稿",
            "复刻",
            "照着",
            "参考图",
            "截图",
            "上传的图",
            "上传图片",
            "附图",
            "这张图",
            "图中",
            "原图",
            "screenshot",
            "reference image",
            "attached image",
            "uploaded image",
            "reference design",
            "replicate",
            "reproduce",
            "clone",
        ]
        .iter()
        .any(|source| lower.contains(source))
}

pub(crate) fn append_section_instruction(
    request: &DesignRequest,
    subtask: &Subtask,
    prompt: &mut String,
) {
    if may_check(request)
        && subtask
            .elements
            .as_deref()
            .is_some_and(|elements| rating_terms(elements) || elements.contains("评价"))
    {
        prompt.push_str("\n\nTHIS SECTION — UNSUPPORTED RATING: The planner proposed a rating/review badge, but the ORIGINAL USER BRIEF does not request or supply it. Omit that entire optional rating badge, label, stars and value. Do not replace it with a made-up score or an empty rating placeholder. Preserve the requested identity, contact actions, other content and visual detail; the planner proposal is not a source.");
    }
    order::append_section_instruction(request, subtask, prompt);
    delivery::append_section_instruction(request, subtask, prompt);
    exercise::append_instruction(request, prompt);
}

/// Return an actionable same-section retry reason before any nodes are inserted.
pub(crate) fn rejection(request: &DesignRequest, nodes: &[PenNode]) -> Option<String> {
    rating_rejection(request, nodes)
        .or_else(|| order::rejection(request, nodes))
        .or_else(|| delivery::rejection(request, nodes))
        .or_else(|| exercise::rejection(request, nodes))
}

fn rating_rejection(request: &DesignRequest, nodes: &[PenNode]) -> Option<String> {
    if !may_check(request) {
        return None;
    }
    let (name, value) = nodes.iter().find_map(find_rating)?;
    let evidence = serde_json::json!({"node": name, "rating": value});
    Some(format!(
        "self-check failed: unsupported-optional-rating: {evidence}. The ORIGINAL USER BRIEF neither requests nor supplies a rating. Return one complete editable section without the extra rating badge/label/value; preserve all requested identity, contact actions and visual detail. Planner elements are not factual sources. Do not return only a fragment."
    ))
}

fn text_content(node: &PenNode) -> Option<String> {
    let PenNode::Text(text) = node else {
        return None;
    };
    Some(match &text.content {
        TextContent::Plain(text) => text.clone(),
        TextContent::Styled(runs) => runs.iter().map(|run| run.text.as_str()).collect(),
    })
}

fn visible(node: &PenNode) -> bool {
    is_node_visible(node) && opacity(node) > 0.0
}

fn rating_name(name: &str) -> bool {
    let separated = semantic_name(name);
    rating_terms(&separated) || separated.contains("评价标签")
}

fn semantic_name(name: &str) -> String {
    // Names commonly use underscores or camelCase; words still need boundaries
    // so unrelated names such as saturation and operating-cost cannot match.
    let mut separated = String::new();
    let chars: Vec<_> = name.chars().collect();
    for (index, &c) in chars.iter().enumerate() {
        let camel_boundary = c.is_ascii_uppercase()
            && index > 0
            && (chars[index - 1].is_ascii_lowercase()
                || chars[index - 1].is_ascii_digit()
                || chars.get(index + 1).is_some_and(char::is_ascii_lowercase));
        if c == '_' || camel_boundary {
            separated.push(' ');
        }
        if c != '_' {
            separated.extend(c.to_lowercase());
        }
    }
    separated
}

fn find_rating(node: &PenNode) -> Option<(String, String)> {
    if !visible(node) {
        return None;
    }
    let name = node.base().name.as_deref().unwrap_or("");
    if let Some(text) = text_content(node) {
        if (!count_name(name) && rating_name(name) && numeric_rating(&text))
            || labelled_rating(&text)
        {
            return Some((name.chars().take(80).collect(), text));
        }
    } else if let Some(children) = node.children() {
        // Only a small local component may pair a label with a number. Do not
        // search an entire page for any number near the word "rating".
        let mut texts = Vec::new();
        if collect_local_texts(node, 0, &mut texts)
            && !count_name(name)
            && (rating_name(name) || texts.iter().any(|text| rating_label(text)))
        {
            if let Some(value) = texts.iter().find(|text| numeric_rating(text)) {
                return Some((name.chars().take(80).collect(), value.clone()));
            }
        }
        return children.iter().find_map(find_rating);
    }
    None
}

fn collect_local_texts(node: &PenNode, depth: usize, out: &mut Vec<String>) -> bool {
    if !visible(node) || count_name(node.base().name.as_deref().unwrap_or("")) {
        return true;
    }
    if depth > 2 || out.len() > 3 {
        return false;
    }
    if let Some(text) = text_content(node) {
        out.push(text);
    } else if let Some(children) = node.children() {
        if children.len() > 4 {
            return false;
        }
        for child in children {
            if !collect_local_texts(child, depth + 1, out) {
                return false;
            }
        }
    }
    out.len() <= 3
}

fn count_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("count") || lower.contains("数量") || lower.contains("人数")
}

fn rating_label(text: &str) -> bool {
    matches!(
        text.trim()
            .trim_end_matches([':', '：'])
            .to_lowercase()
            .as_str(),
        "rating" | "ratings" | "评分" | "星级"
    )
}

fn labelled_rating(text: &str) -> bool {
    let lower = text.trim().to_lowercase();
    ["rating", "ratings", "评分", "星级"].iter().any(|label| {
        lower.strip_prefix(label).is_some_and(|tail| {
            let value = tail.trim_start_matches([' ', ':', '：']);
            numeric_rating(value)
        })
    })
}

fn numeric_rating(text: &str) -> bool {
    let value = text.trim().trim_start_matches(['★', '☆']).trim();
    let value = value.trim_end_matches('分').trim();
    let value = if let Some((score, scale)) = value.split_once('/') {
        if !matches!(scale.trim(), "5" | "10") {
            return false;
        }
        score.trim()
    } else {
        value
    };
    // Only a literal score in an already identified rating component counts.
    // Prices, percentages, text references and unrelated numbers do not.
    !value.is_empty()
        && value.len() <= 5
        && value.chars().all(|c| c.is_ascii_digit() || c == '.')
        && value
            .parse::<f64>()
            .is_ok_and(|n| (0.0..=10.0).contains(&n))
}

#[cfg(test)]
#[path = "source_facts_tests.rs"]
mod tests;
