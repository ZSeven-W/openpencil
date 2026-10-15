//! Optional transaction facts in a single delivery-tracking brief.
//! Requested fields, demo collections and reference sources remain creative.

use super::{collect_local_texts, semantic_name, text_content, text_only_source, visible};
use crate::{plan::Subtask, types::DesignRequest};
use jian_ops_schema::node::PenNode;
use op_editor_core::PenNodeExt;
use regex::Regex;
use std::sync::LazyLock;

static MONEY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
    r"(?i)(?:[¥￥$€£]\s*\d+(?:\.\d{1,2})?|\b(?:CNY|RMB|USD|EUR)\s*\d+(?:\.\d{1,2})?|\d+(?:\.\d{1,2})?\s*(?:元|人民币|美元))"
).unwrap()
});
static CLOCK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|[^0-9])(?:[01]?[0-9]|2[0-3])[:：][0-5][0-9](?:$|[^0-9])").unwrap()
});
static QUANTITY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:[×✕]|(?:^|\s)[xX])\s*\d+\b").unwrap());
static NUMBER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d+(?:\.\d{1,2})?$").unwrap());

#[derive(Clone, Copy)]
struct Checks {
    price: bool,
    clock: bool,
    quantity: bool,
}

fn contains(text: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| {
        if term.is_ascii() {
            crate::design_type::contains_word(text, term)
        } else {
            text.contains(term)
        }
    })
}

fn price_terms(text: &str) -> bool {
    contains(
        text,
        &[
            "价格", "售价", "金额", "实付", "合计", "费用", "支付", "付款", "price", "prices",
            "cost", "total", "payment", "subtotal", "amount",
        ],
    )
}

fn arrival_terms(text: &str) -> bool {
    contains(
        text,
        &[
            "送达",
            "预计",
            "eta",
            "arrival",
            "arrives",
            "estimated delivery",
        ],
    )
}

fn quantity_terms(text: &str) -> bool {
    contains(text, &["数量", "份数", "quantity", "quantities", "qty"])
}

fn checks(request: &DesignRequest) -> Option<Checks> {
    if !text_only_source(request) {
        return None;
    }
    let brief = semantic_name(&request.prompt);
    let delivery = contains(&brief, &["配送", "外卖", "delivery", "courier"]);
    let detail = contains(
        &brief,
        &[
            "追踪", "跟踪", "详情", "tracking", "track", "detail", "details",
        ],
    );
    let demo = contains(
        &brief,
        &[
            "示例数据",
            "演示数据",
            "虚构数据",
            "sample data",
            "demo data",
            "fictional data",
        ],
    );
    let no_demo = contains(
        &brief,
        &[
            "不要示例",
            "不要演示",
            "不要虚构",
            "不得虚构",
            "不要编造",
            "no sample",
            "no demo",
            "no fictional",
            "do not invent",
        ],
    );
    let collection = contains(
        &brief,
        &[
            "订单列表",
            "订单表格",
            "orders table",
            "order list",
            "order table",
        ],
    );
    if !delivery || !detail || (!no_demo && (demo || collection)) {
        return None;
    }
    Some(Checks {
        price: !price_terms(&brief) && !MONEY.is_match(&brief),
        clock: !CLOCK.is_match(&brief)
            && !contains(
                &brief,
                &[
                    "送达时间",
                    "送达时刻",
                    "几点送达",
                    "arrival time",
                    "delivery time",
                    "arrival clock",
                ],
            ),
        quantity: !quantity_terms(&brief) && !QUANTITY.is_match(&brief),
    })
}

pub(super) fn append_section_instruction(
    request: &DesignRequest,
    subtask: &Subtask,
    prompt: &mut String,
) {
    let Some(check) = checks(request) else { return };
    let elements = semantic_name(subtask.elements.as_deref().unwrap_or_default());
    let fields: Vec<_> = [
        (
            check.price && (price_terms(&elements) || MONEY.is_match(&elements)),
            "price/total",
        ),
        (
            check.clock && CLOCK.is_match(&elements) && arrival_terms(&elements),
            "absolute arrival clock (HH:MM)",
        ),
        (
            check.quantity && (quantity_terms(&elements) || QUANTITY.is_match(&elements)),
            "item quantity (×N)",
        ),
    ]
    .into_iter()
    .filter_map(|(enabled, field)| enabled.then_some(field))
    .collect();
    if !fields.is_empty() {
        prompt.push_str(&format!("\n\nDELIVERY SOURCE: The original brief does not request or supply {}. Omit these optional facts and their labels even if planner elements propose them. Retain the supplied relative ETA, meal, address, identity, contact actions and complete visual composition. A relative ETA does not establish a clock time.", fields.join(", ")));
    }
}

pub(super) fn rejection(request: &DesignRequest, nodes: &[PenNode]) -> Option<String> {
    let check = checks(request)?;
    let mut found = Vec::new();
    for node in nodes {
        collect(node, check, &mut found);
    }
    if found.is_empty() {
        return None;
    }
    Some(format!("self-check failed: unsupported-delivery-facts: {}. These optional facts are neither requested nor supplied by the ORIGINAL USER BRIEF. Return the complete section without these extra fields and labels; preserve the requested relative ETA, meal, address, identity and contact actions. Do not replace them with other invented numbers. Planner proposals are not sources.", serde_json::to_string(&found).unwrap()))
}

fn collect(node: &PenNode, check: Checks, found: &mut Vec<serde_json::Value>) {
    if !visible(node) || found.len() >= 8 {
        return;
    }
    let name = node.base().name.as_deref().unwrap_or("");
    let semantic = semantic_name(name);
    if let Some(text) = text_content(node) {
        inspect(&text, &semantic, name, check, found);
    } else if let Some(children) = node.children() {
        // Pair labels with values only inside a small component. Never use a
        // page-wide ETA label to interpret the phone's status-bar clock.
        let mut texts = Vec::new();
        if collect_local_texts(node, 0, &mut texts) {
            let mut context = semantic.clone();
            for text in &texts {
                let label = text.trim().trim_end_matches([':', '：']).to_lowercase();
                if [
                    "预计送达",
                    "送达时间",
                    "预计送达时间",
                    "eta",
                    "arrival",
                    "arrival time",
                    "estimated arrival",
                    "价格",
                    "金额",
                    "实付",
                    "数量",
                    "price",
                    "total",
                    "quantity",
                ]
                .contains(&label.as_str())
                {
                    context.push(' ');
                    context.push_str(&label);
                }
            }
            for text in texts {
                inspect(&text, &context, name, check, found);
            }
        }
        for child in children {
            collect(child, check, found);
        }
    }
}

fn inspect(
    text: &str,
    semantic: &str,
    name: &str,
    check: Checks,
    found: &mut Vec<serde_json::Value>,
) {
    let lower = text.to_lowercase();
    let price = check.price
        && (MONEY.is_match(text) || (price_terms(semantic) && NUMBER.is_match(text.trim())));
    let clock =
        check.clock && CLOCK.is_match(text) && (arrival_terms(semantic) || arrival_terms(&lower));
    let quantity = check.quantity
        && (QUANTITY.is_match(text) || (quantity_terms(semantic) && NUMBER.is_match(text.trim())));
    if price || clock || quantity {
        let evidence = serde_json::json!({"node":name.chars().take(80).collect::<String>(),"text":text.chars().take(160).collect::<String>(),"price":price,"arrival_clock":clock,"quantity":quantity});
        if !found.contains(&evidence) {
            found.push(evidence);
        }
    }
}

#[cfg(test)]
#[path = "source_facts_delivery_tests.rs"]
mod tests;
