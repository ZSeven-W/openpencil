//! Count complete sibling reviews while permitting one optional photo attachment.
//! The global style signature and generic item count remain unchanged.

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

/// Call only with ownership/copy/statistics-masked local task text.
pub(super) fn has_local_review_contract(text: &str, expected: usize) -> bool {
    static CONTRACT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?ix)
            (?P<before>[0-9][0-9,]*|[零〇一二两三四五六七八九十百千万亿]+)\s*[条个项]\s*
                (?:用户|客户|买家)?(?:评价|评论)
            |(?:评价|评论)(?:列表|卡片|区)?\s*[:：]?\s*
                (?P<after>[0-9][0-9,]*|[零〇一二两三四五六七八九十百千万亿]+)\s*[条个项]
            |\b(?P<english>\d[\d,]*|two|three|four|five|six|seven|eight|nine|ten)\s+
                (?:customer\s+|user\s+)?reviews?\b",
        )
        .expect("valid local review-count contract")
    });
    CONTRACT.captures_iter(text).any(|capture| {
        let count = capture
            .name("before")
            .or_else(|| capture.name("after"))
            .or_else(|| capture.name("english"))
            .unwrap();
        parse_count(count.as_str()) == Some(expected)
            && !ignored_number_context(text, count.start())
    })
}

#[derive(Debug, Default)]
pub(super) struct ReviewEvidence {
    pub card_ids: Vec<String>,
    pub attachment_ids: Vec<String>,
}

pub(super) fn delivered_reviews(sink: &dyn DocSink, root_ids: &[String]) -> Option<ReviewEvidence> {
    let roots = root_ids.iter().map(String::as_str).collect();
    let mut largest = ReviewEvidence::default();
    count_siblings(sink.state().active_children(), &roots, false, &mut largest);
    (!largest.card_ids.is_empty()).then_some(largest)
}

fn count_siblings(
    nodes: &[PenNode],
    roots: &HashSet<&str>,
    inside: bool,
    largest: &mut ReviewEvidence,
) {
    let mut families = BTreeMap::<String, ReviewEvidence>::new();
    for node in nodes {
        if !visible(node) || crate::cleanup::is_status_bar(node) {
            continue;
        }
        let owned = inside || roots.contains(node.id_str());
        if let Some((signature, attachment)) = review_signature(node) {
            if owned {
                let family = families.entry(signature).or_default();
                family.card_ids.push(node.id_str().into());
                family.attachment_ids.extend(attachment);
            }
            // Descendants of one complete record cannot supply more records.
            continue;
        }
        if let PenNode::Frame(frame) = node {
            if let Some(children) = &frame.children {
                count_siblings(children, roots, owned, largest);
            }
        }
    }
    for family in families.into_values() {
        if family.card_ids.len() > largest.card_ids.len() {
            *largest = family;
        }
    }
}

fn visible(node: &PenNode) -> bool {
    if !is_node_visible(node)
        || matches!(node.base().enabled, Some(BoolOrExpression::Expression(_)))
        || matches!(node.base().opacity, Some(NumberOrExpression::Expression(_)))
        || matches!(node.base().opacity, Some(NumberOrExpression::Number(x)) if x <= 0.0)
    {
        return false;
    }
    let bindings = match node {
        PenNode::Frame(n) => n.bindings.as_ref(),
        PenNode::Text(n) => n.bindings.as_ref(),
        PenNode::Image(n) => n.bindings.as_ref(),
        PenNode::IconFont(n) => n.bindings.as_ref(),
        PenNode::Ellipse(n) => n.bindings.as_ref(),
        PenNode::Rectangle(n) => n.bindings.as_ref(),
        _ => return false,
    };
    !bindings.is_some_and(|b| {
        ["visible", "enabled", "opacity"]
            .iter()
            .any(|k| b.contains_key(*k))
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
        .is_some_and(|b| b.contains_key("content"))
    {
        return None;
    }
    let value: String = match &text.content {
        TextContent::Plain(value) => value.clone(),
        TextContent::Styled(runs) => runs.iter().map(|r| r.text.as_str()).collect(),
    };
    (!value.trim().is_empty() && !value.trim().starts_with('$')).then_some(value)
}

fn review_signature(node: &PenNode) -> Option<(String, Option<String>)> {
    let PenNode::Frame(card) = node else {
        return None;
    };
    if !visible(node) || card.container.layout != Some(LayoutMode::Vertical) {
        return None;
    }
    let children = card.children.as_deref()?;
    if !(2..=3).contains(&children.len()) || children.iter().any(|n| !visible(n)) {
        return None;
    }
    let header = header_signature(&children[0])?;
    let (body, nested_photo) = body_signature(&children[1])?;
    let direct_photo = children.get(2).map(photo_attachment).transpose().ok()?;
    if nested_photo.is_some() && direct_photo.is_some() {
        return None;
    }
    Some((
        format!("review({header},{body})"),
        nested_photo.or(direct_photo),
    ))
}

fn header_signature(node: &PenNode) -> Option<String> {
    let PenNode::Frame(header) = node else {
        return None;
    };
    if header.container.layout != Some(LayoutMode::Horizontal) || !visible(node) {
        return None;
    }
    let children = header.children.as_deref()?;
    if children.len() < 2 || !avatar(&children[0]) {
        return None;
    }
    if !identity_and_rating(&children[1]) {
        return None;
    }
    // Preserve every metadata slot and its structure; only photo attachments
    // outside this header may differ between family members.
    if children[2..].iter().any(|n| !metadata(n)) {
        return None;
    }
    Some(crate::cleanup::structural_signature(node))
}

fn avatar(node: &PenNode) -> bool {
    if !visible(node) || node.events().is_some() {
        return false;
    }
    match node {
        PenNode::Ellipse(_) | PenNode::Image(_) => true,
        PenNode::Frame(f) => f.children.as_deref().is_some_and(|c| {
            c.len() == 1 && static_text(&c[0]).is_some_and(|s| s.chars().count() <= 2)
        }),
        _ => false,
    }
}

fn identity_and_rating(node: &PenNode) -> bool {
    let PenNode::Frame(identity) = node else {
        return false;
    };
    if !visible(node) || identity.container.layout != Some(LayoutMode::Vertical) {
        return false;
    }
    let Some(children) = identity.children.as_deref() else {
        return false;
    };
    if children.len() != 2 || static_text(&children[0]).is_none() {
        return false;
    }
    let PenNode::Frame(rating) = &children[1] else {
        return false;
    };
    if !visible(&children[1]) || rating.container.layout != Some(LayoutMode::Horizontal) {
        return false;
    }
    rating.children.as_deref().is_some_and(|stars| (1..=5).contains(&stars.len())
        && stars.iter().all(|s| visible(s) && s.events().is_none()
            && matches!(s, PenNode::IconFont(i) if matches!(i.icon_font_name.as_str(), "star" | "star-half" | "star-off"))))
}

fn metadata(node: &PenNode) -> bool {
    if !visible(node) || node.events().is_some() {
        return false;
    }
    match node {
        PenNode::Text(_) => static_text(node).is_some(),
        PenNode::Rectangle(_) => true,
        PenNode::Frame(f) => f.children.as_deref().is_some_and(|c| {
            c.len() == 1 && static_text(&c[0]).is_some() && c[0].events().is_none()
        }),
        _ => false,
    }
}

fn body_signature(node: &PenNode) -> Option<(String, Option<String>)> {
    if static_text(node).is_some() {
        return Some(("text".into(), None));
    }
    let PenNode::Frame(body) = node else {
        return None;
    };
    if !visible(node) || body.container.layout != Some(LayoutMode::Horizontal) {
        return None;
    }
    let children = body.children.as_deref()?;
    if !(1..=2).contains(&children.len()) || static_text(&children[0]).is_none() {
        return None;
    }
    let photo = children.get(1).map(photo_attachment).transpose().ok()?;
    Some(("horizontal(text)".into(), photo))
}

fn photo_attachment(node: &PenNode) -> Result<String, ()> {
    if !visible(node) || node.events().is_some() {
        return Err(());
    }
    if matches!(node, PenNode::Image(_)) {
        return Ok(node.id_str().into());
    }
    if let PenNode::Frame(frame) = node {
        if let Some([image @ PenNode::Image(_)]) = frame.children.as_deref() {
            if visible(image) && image.events().is_none() {
                return Ok(image.id_str().into());
            }
        }
    }
    Err(())
}
