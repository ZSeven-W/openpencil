//! Additional evidence for explicitly counted, priced lists with add controls.
//!
//! A row is not identified by its name or by matching the expected family size.
//! The strict style-repair signature and the existing generic counter stay intact.

use super::{ignored_number_context, parse_count};
use crate::types::DocSink;
use jian_ops_schema::node::{
    base::{BoolOrExpression, NumberOrExpression},
    container::LayoutMode,
    PenNode, TextContent,
};
use op_design_lint::node_util::is_node_visible;
use op_editor_core::PenNodeExt;
use regex::Regex;
use std::collections::{BTreeMap, HashSet};
use std::sync::LazyLock;

/// The caller supplies ownership/copy-masked local text, never the whole brief.
pub(super) fn has_local_list_contract(text: &str, expected: usize) -> bool {
    static CONTRACT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?ix)
            (?:列表|清单)\s*(?:[:：]|共)?\s*
            (?P<cjk>[0-9][0-9,]*|[零〇一二两三四五六七八九十百千万亿]+)\s*[个条项行款]
            |\blist\s+of\s+(?P<english>\d[\d,]*|two|three|four|five|six|seven|eight|nine|ten)\b",
        )
        .expect("valid explicit list count")
    });
    CONTRACT.captures_iter(text).any(|capture| {
        let count = capture
            .name("cjk")
            .or_else(|| capture.name("english"))
            .unwrap();
        parse_count(count.as_str()) == Some(expected)
            && !ignored_number_context(text, count.start())
    })
}

pub(super) fn delivered_list_row_count(sink: &dyn DocSink, root_ids: &[String]) -> usize {
    let roots = root_ids.iter().map(String::as_str).collect();
    let mut largest = 0;
    count_siblings(sink.state().active_children(), &roots, false, &mut largest);
    largest
}

fn count_siblings(nodes: &[PenNode], roots: &HashSet<&str>, inside: bool, largest: &mut usize) {
    let mut families = BTreeMap::<String, usize>::new();
    for node in nodes {
        if !visible(node) || crate::cleanup::is_status_bar(node) {
            continue;
        }
        let owned = inside || roots.contains(node.id_str());
        if let Some(signature) = row_signature(node) {
            if owned {
                *families.entry(signature).or_default() += 1;
            }
            // An item's descendants cannot supply a second list's quota.
            continue;
        }
        if matches!(node, PenNode::Frame(frame)
            if frame.container.layout == Some(LayoutMode::Horizontal)
                && frame.children.as_ref().is_some_and(|children|
                    children.len() >= 2 && children.iter().any(media_slot_shape)))
        {
            // A media row's embedded recommendations are not a section list.
            continue;
        }
        // Ref overrides/slots and native panel visibility need materialization.
        if let PenNode::Frame(frame) = node {
            if let Some(children) = &frame.children {
                count_siblings(children, roots, owned, largest);
            }
        }
    }
    *largest = (*largest).max(families.values().copied().max().unwrap_or(0));
}

fn visible(node: &PenNode) -> bool {
    if !is_node_visible(node)
        || matches!(node.base().enabled, Some(BoolOrExpression::Expression(_)))
        || matches!(node.base().opacity, Some(NumberOrExpression::Expression(_)))
        || matches!(node.base().opacity, Some(NumberOrExpression::Number(value)) if value <= 0.0)
    {
        return false;
    }
    let bindings = match node {
        PenNode::Frame(node) => node.bindings.as_ref(),
        PenNode::Text(node) => node.bindings.as_ref(),
        PenNode::Image(node) => node.bindings.as_ref(),
        PenNode::IconFont(node) => node.bindings.as_ref(),
        _ => return false,
    };
    !bindings.is_some_and(|bindings| {
        ["visible", "enabled", "opacity"]
            .iter()
            .any(|key| bindings.contains_key(*key))
    })
}

fn row_signature(node: &PenNode) -> Option<String> {
    let PenNode::Frame(frame) = node else {
        return None;
    };
    if !visible(node) || frame.container.layout != Some(LayoutMode::Horizontal) {
        return None;
    }
    let children = frame.children.as_deref()?;
    if !(2..=3).contains(&children.len()) || children.iter().any(|child| !visible(child)) {
        return None;
    }
    let mut media = 0;
    let mut information = 0;
    let mut controls = 0;
    let mut slots = Vec::new();
    for child in children {
        if let Some(signature) = media_signature(child) {
            media += 1;
            slots.push(signature);
        } else if let Some(signature) = add_control_signature(child, 0) {
            controls += 1;
            slots.push(signature);
        } else {
            let PenNode::Frame(info) = child else {
                return None;
            };
            if info.container.layout != Some(LayoutMode::Vertical) {
                return None;
            }
            let proof = info_signature(child, 0)?;
            if proof.prices == 0 || proof.labels == 0 {
                return None;
            }
            information += 1;
            controls += proof.controls;
            slots.push(proof.signature);
        }
    }
    (media == 1 && information == 1 && controls == 1).then(|| format!("row({})", slots.join(",")))
}

fn media_signature(node: &PenNode) -> Option<String> {
    if !visible(node) {
        return None;
    }
    if matches!(node, PenNode::Image(_)) {
        return Some("image".into());
    }
    let PenNode::Frame(frame) = node else {
        return None;
    };
    let children = frame.children.as_deref()?;
    // Only one literal image. Hidden extras and recursive wrappers are not slots.
    if children.len() != 1 || !visible(&children[0]) || !matches!(&children[0], PenNode::Image(_)) {
        return None;
    }
    Some(format!("frame:{:?}(image)", frame.container.layout))
}

fn media_slot_shape(node: &PenNode) -> bool {
    matches!(node, PenNode::Image(_))
        || matches!(node, PenNode::Frame(frame) if frame.children.as_ref().is_some_and(|children|
            children.len() == 1 && matches!(children[0], PenNode::Image(_))))
}

/// A standalone plus control, optionally inside one alignment slot.
/// No event is required: completeness must also accept static editable UI.
fn add_control_signature(node: &PenNode, depth: usize) -> Option<String> {
    if depth > 2 || !visible(node) {
        return None;
    }
    match node {
        PenNode::IconFont(icon) if depth > 0 && icon.icon_font_name == "plus" => {
            Some("icon:plus".into())
        }
        PenNode::Frame(frame) => {
            let children = frame.children.as_deref()?;
            if children.len() != 1 {
                return None;
            }
            Some(format!(
                "control:{:?}({})",
                frame.container.layout,
                add_control_signature(&children[0], depth + 1)?
            ))
        }
        _ => None,
    }
}

struct InfoProof {
    signature: String,
    prices: usize,
    labels: usize,
    controls: usize,
}

fn info_signature(node: &PenNode, depth: usize) -> Option<InfoProof> {
    if depth >= 16 || !visible(node) {
        return None;
    }
    if let Some(signature) = add_control_signature(node, 0) {
        return Some(InfoProof {
            signature,
            prices: 0,
            labels: 0,
            controls: 1,
        });
    }
    if let Some(content) = static_text(node) {
        let price = is_price(&content);
        return Some(InfoProof {
            signature: if price { "price" } else { "text" }.into(),
            prices: usize::from(price),
            labels: usize::from(!price),
            controls: 0,
        });
    }
    let PenNode::Frame(frame) = node else {
        return None;
    };
    let children = frame.children.as_deref()?;
    if children.is_empty() {
        return None;
    }
    let proofs = children
        .iter()
        .map(|child| info_signature(child, depth + 1))
        .collect::<Option<Vec<_>>>()?;
    // Normalize only one/two literal prices in a horizontal price branch.
    // Other metadata, badges, slots, order and layout remain structural.
    let price_row = frame.container.layout == Some(LayoutMode::Horizontal)
        && (1..=2).contains(&children.len())
        && children
            .iter()
            .all(|child| static_text(child).is_some_and(|text| is_price(&text)));
    let nested = if price_row {
        "price+".into()
    } else {
        proofs
            .iter()
            .map(|proof| proof.signature.as_str())
            .collect::<Vec<_>>()
            .join(",")
    };
    Some(InfoProof {
        signature: format!("frame:{:?}({nested})", frame.container.layout),
        prices: proofs.iter().map(|proof| proof.prices).sum(),
        labels: proofs.iter().map(|proof| proof.labels).sum(),
        controls: proofs.iter().map(|proof| proof.controls).sum(),
    })
}

fn static_text(node: &PenNode) -> Option<String> {
    if !visible(node) {
        return None;
    }
    let PenNode::Text(text) = node else {
        return None;
    };
    if text
        .bindings
        .as_ref()
        .is_some_and(|bindings| bindings.contains_key("content"))
    {
        return None;
    }
    let value = match &text.content {
        TextContent::Plain(value) => value.clone(),
        TextContent::Styled(runs) => runs.iter().map(|run| run.text.as_str()).collect(),
    };
    (!value.trim().is_empty() && (!value.trim().starts_with('$') || is_price(value.trim())))
        .then_some(value)
}

fn is_price(text: &str) -> bool {
    static PRICE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:[¥￥$€£]\s*\d+(?:[.,]\d+)*|\d+(?:\.\d+)?\s*元)$")
            .expect("valid standalone price")
    });
    PRICE.is_match(text.trim())
}
