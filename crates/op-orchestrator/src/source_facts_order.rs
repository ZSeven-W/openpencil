//! Detect only explicit order identifiers added without source authorization.
//! Requested fields, populated order tables and actual reference content are
//! outside this absence-based check; no generated nodes are rewritten here.

use super::{
    collect_local_texts, count_name, semantic_name, text_content, text_only_source, visible,
};
use crate::{plan::Subtask, types::DesignRequest};
use jian_ops_schema::node::PenNode;
use op_editor_core::PenNodeExt;

const LABELS: &[&str] = &[
    "order number",
    "order id",
    "order no.",
    "order no",
    "order #",
    "订单编号",
    "订单号",
];

fn order_terms(text: &str) -> bool {
    let normalized = semantic_name(text).replace('-', " ");
    LABELS.iter().any(|label| {
        if label.is_ascii() {
            crate::design_type::contains_word(&normalized, label)
        } else {
            normalized.contains(label)
        }
    })
}

fn may_check(request: &DesignRequest) -> bool {
    if !text_only_source(request) || order_terms(&request.prompt) {
        return false;
    }
    // A populated conceptual order collection authorizes representative row
    // data, including identifiers. Do not turn an orders table into blanks.
    let lower = request.prompt.to_lowercase();
    let orders = lower.contains("订单")
        || ["order", "orders"]
            .iter()
            .any(|word| crate::design_type::contains_word(&lower, word));
    let collection = ["表格", "列表", "清单", "table", "list", "rows"]
        .iter()
        .any(|word| lower.contains(word));
    let explicit_demo = [
        "示例数据",
        "演示数据",
        "虚构数据",
        "sample data",
        "demo data",
        "fictional data",
    ]
    .iter()
    .any(|word| lower.contains(word));
    let no_demo = [
        "不要示例",
        "不要演示",
        "不要虚构",
        "不得虚构",
        "不要编造",
        "不可编造",
        "no sample",
        "no demo",
        "no fictional",
        "do not invent",
    ]
    .iter()
    .any(|word| lower.contains(word));
    !(orders && !no_demo && (collection || explicit_demo))
}

pub(super) fn append_section_instruction(
    request: &DesignRequest,
    subtask: &Subtask,
    prompt: &mut String,
) {
    if may_check(request) && subtask.elements.as_deref().is_some_and(order_terms) {
        prompt.push_str("\n\nTHIS SECTION — UNSUPPORTED ORDER IDENTIFIER: The planner proposed an order number, but the ORIGINAL USER BRIEF neither requests nor supplies one. Omit this optional identifier and its label; preserve requested order contents, address, contact actions and the complete section. Do not invent a number to fill the space. Planner proposals are not a source.");
    }
}

pub(super) fn rejection(request: &DesignRequest, nodes: &[PenNode]) -> Option<String> {
    if !may_check(request) {
        return None;
    }
    let (name, value) = nodes
        .iter()
        .find_map(|node| find_identifier(node, &request.prompt))?;
    let evidence = serde_json::json!({"node": name, "order_identifier": value});
    Some(format!(
        "self-check failed: unsupported-order-identifier: {evidence}. The ORIGINAL USER BRIEF neither requests nor supplies an order number. Return one complete editable section without this extra identifier and label; preserve all requested order contents, address, contact actions and visual detail. Do not substitute a different number or drop the whole order card. Planner proposals are not factual sources."
    ))
}

fn find_identifier(node: &PenNode, brief: &str) -> Option<(String, String)> {
    if !visible(node) {
        return None;
    }
    let name = node.base().name.as_deref().unwrap_or("");
    let unsupported = |value: &str| {
        let id = identifier(value)?;
        (!supplied_identifier(brief, id)).then(|| (name.chars().take(80).collect(), id.to_owned()))
    };
    if let Some(text) = text_content(node) {
        if let Some(value) = labelled_identifier(&text) {
            return unsupported(value);
        }
        if !count_name(name) && order_terms(name) {
            return unsupported(&text);
        }
    } else if let Some(children) = node.children() {
        // Pair a short label with its value only within a compact component,
        // never with unrelated numbers found elsewhere on the page.
        let mut texts = Vec::new();
        if collect_local_texts(node, 0, &mut texts)
            && !count_name(name)
            && (order_terms(name) || texts.iter().any(|text| order_label(text)))
        {
            if let Some(found) = texts.iter().find_map(|value| unsupported(value)) {
                return Some(found);
            }
        }
        return children
            .iter()
            .find_map(|child| find_identifier(child, brief));
    }
    None
}

fn order_label(text: &str) -> bool {
    let lower = text.trim().trim_end_matches([':', '：']).to_lowercase();
    LABELS.contains(&lower.as_str())
}

fn labelled_identifier(text: &str) -> Option<&str> {
    let text = text.trim();
    let lower = text.to_lowercase();
    LABELS.iter().find_map(|label| {
        lower.strip_prefix(label)?;
        let tail = text.get(label.len()..)?;
        // "order identity" must not match the "order id" prefix.
        if label.is_ascii() && tail.starts_with(char::is_alphabetic) {
            return None;
        }
        Some(tail.trim_start_matches([' ', ':', '：', '#']))
    })
}

fn identifier(value: &str) -> Option<&str> {
    let value = value.trim().trim_start_matches('#');
    (value.len() >= 3
        && value.len() <= 64
        && value.chars().any(|c| c.is_ascii_digit())
        && value.chars().all(identifier_char))
    .then_some(value)
}

fn identifier_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_')
}

fn supplied_identifier(brief: &str, value: &str) -> bool {
    brief
        .split(|c| !identifier_char(c))
        .any(|token| token == value)
}

#[cfg(test)]
#[path = "source_facts_order_tests.rs"]
mod tests;
