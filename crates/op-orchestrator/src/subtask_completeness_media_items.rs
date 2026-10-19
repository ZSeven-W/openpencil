//! Local media-card quotas, independent of style-repair structural signatures.

use super::{ignored_number_context, parse_count};
use crate::types::DocSink;
use jian_ops_schema::node::{
    base::NumberOrExpression, container::LayoutMode, PenNode, TextContent,
};
use op_design_lint::node_util::{is_node_visible, node_kind_str};
use op_editor_core::PenNodeExt;
use regex::Regex;
use std::collections::{BTreeMap, HashSet};
use std::sync::LazyLock;

/// The count and per-card media description must belong to one local promise.
/// Call only with `local_item_text`, after copy and ownership masking.
pub(super) fn has_local_media_card_contract(text: &str, expected: usize) -> bool {
    static CONTRACT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?ix)
          (?P<count>[0-9][0-9,]*|[零〇一二两三四五六七八九十百千万亿]+)\s*
          (?:
            (?:[个条张款]?\s*(?:(?:商家|商品|产品)(?:行卡片|卡片|卡|行)?|行卡片|卡片|条目)|[个条张款])
            \s*[:：]\s*(?:左(?:侧)?|右(?:侧)?|顶部|每(?:个|条|张|项))
            [^,，;；。\n]{0,60}(?:实拍图|缩略图|图片|照片|图像|图)
            |
            \s+(?:cards?|items?|rows?|products?|merchants?)\b\s*[:,]?\s*
            (?:each|every)\s+(?:with|has|contains|shows|includes)\b[^,;.!\n]{0,60}\b(?:image|photo|thumbnail)\b
          )",
        )
        .expect("valid local media-card contract")
    });
    CONTRACT.captures_iter(text).any(|capture| {
        let count = capture.name("count").unwrap();
        parse_count(count.as_str()) == Some(expected)
            && !ignored_number_context(text, count.start())
    })
}

/// Return a proven media-card family, scoped to actual sibling ownership.
/// This adds evidence to the generic count without replacing it. Nested
/// badges and unrelated images never pad this independently proven family.
pub(super) fn delivered_media_card_count(
    sink: &dyn DocSink,
    inserted_root_ids: &[String],
) -> Option<usize> {
    let roots: HashSet<&str> = inserted_root_ids.iter().map(String::as_str).collect();
    let mut largest = 0;
    count_siblings(sink.state().active_children(), &roots, false, &mut largest);
    (largest > 0).then_some(largest)
}

fn is_evidence_visible(node: &PenNode) -> bool {
    is_node_visible(node)
        && !matches!(&node.base().opacity, Some(NumberOrExpression::Number(value)) if *value <= 0.0)
}

fn count_siblings(
    children: &[PenNode],
    roots: &HashSet<&str>,
    inside_inserted_root: bool,
    largest: &mut usize,
) {
    let mut families = BTreeMap::<String, usize>::new();
    for child in children {
        if !is_evidence_visible(child) || crate::cleanup::is_status_bar(child) {
            continue;
        }
        let owned = inside_inserted_root || roots.contains(child.base().id.as_str());
        if owned {
            if let Some(signature) = media_card_signature(child) {
                *families.entry(signature).or_default() += 1;
            }
        }
        // Raw instance slot children may be shadowed by the master's content.
        // Instance materialization is outside this proof; generic counting stays intact.
        if !matches!(child, PenNode::Ref(_)) {
            if let Some(nested) = child.children() {
                count_siblings(nested, roots, owned, largest);
            }
        }
    }
    *largest = (*largest).max(families.values().copied().max().unwrap_or(0));
}

fn media_card_signature(node: &PenNode) -> Option<String> {
    if matches!(node, PenNode::Ref(_)) {
        return None;
    }
    let PenNode::Frame(frame) = node else {
        return None;
    };
    let children: Vec<_> = node
        .children()
        .into_iter()
        .flatten()
        .filter(|child| is_evidence_visible(child))
        .collect();
    let has_image = children.iter().any(|child| single_image_slot(child, 0));
    let has_text_branch = children.iter().any(|child| {
        let (text, image) = content_evidence(child, 0);
        text && !image
    });
    if !has_image || !has_text_branch {
        return None;
    }
    // Keep every immediate slot, in order. Text information may rearrange its
    // metadata or badges without changing the item; image slots never collapse.
    let slots = children
        .into_iter()
        .map(|child| normalized_signature(child, 0))
        .collect::<Option<Vec<_>>>()?;
    Some(format!(
        "frame:{:?}({})",
        frame.container.layout,
        slots.join(",")
    ))
}

fn single_image_slot(node: &PenNode, depth: usize) -> bool {
    if depth >= 8 || !is_evidence_visible(node) {
        return false;
    }
    if matches!(node, PenNode::Image(_)) {
        return true;
    }
    if let PenNode::Frame(frame) = node {
        if let Some(children) = &frame.children {
            if children.len() == 1 {
                return single_image_slot(&children[0], depth + 1);
            }
            // A fixed image slot may carry a compact corner badge. It is still
            // one image, not a second item or an information panel/gallery.
            if frame.container.layout != Some(LayoutMode::None) {
                return false;
            }
            let images: Vec<_> = children
                .iter()
                .filter(|c| matches!(c, PenNode::Image(_)))
                .collect();
            if images.len() != 1 || !is_evidence_visible(images[0]) {
                return false;
            }
            let (Some(width), Some(height)) = (images[0].width_px(), images[0].height_px()) else {
                return false;
            };
            return children.iter().all(|child| {
                matches!(child, PenNode::Image(_)) || matches!(child, PenNode::Frame(badge)
                    if is_evidence_visible(child)
                        && child.width_px().is_some_and(|w| w > 0.0 && w <= width)
                        && child.height_px().is_some_and(|h| h > 0.0 && h <= 48.0 && h <= height / 2.0)
                        && badge.children.as_ref().is_none_or(|labels|
                            labels.len() <= 1 && labels.iter().all(|label| matches!(label, PenNode::Text(_)))))
            });
        }
    }
    false
}

fn content_evidence(node: &PenNode, depth: usize) -> (bool, bool) {
    if depth >= 64 || !is_evidence_visible(node) {
        return (false, false);
    }
    if matches!(node, PenNode::Ref(_)) {
        return (false, false);
    }
    if matches!(node, PenNode::Image(_)) {
        return (false, true);
    }
    if let PenNode::Text(text) = node {
        let nonempty = match &text.content {
            TextContent::Plain(value) => !value.trim().is_empty(),
            TextContent::Styled(runs) => runs.iter().any(|run| !run.text.trim().is_empty()),
        };
        return (nonempty, false);
    }
    node.children()
        .into_iter()
        .flatten()
        .fold((false, false), |(text, image), child| {
            let (child_text, child_image) = content_evidence(child, depth + 1);
            (text || child_text, image || child_image)
        })
}

fn normalized_signature(node: &PenNode, depth: usize) -> Option<String> {
    if depth >= 64 {
        return None;
    }
    if matches!(node, PenNode::Ref(_)) {
        return None;
    }
    let children: Vec<_> = node
        .children()
        .into_iter()
        .flatten()
        .filter(|child| is_evidence_visible(child))
        .collect();
    let layout = match node {
        PenNode::Frame(frame) => format!("{:?}", frame.container.layout),
        _ => String::new(),
    };
    if matches!(node, PenNode::Frame(_)) && single_image_slot(node, 0) {
        return Some(format!("frame:{layout}(image)"));
    }
    let nested = children
        .into_iter()
        .map(|child| normalized_signature(child, depth + 1))
        .collect::<Option<Vec<_>>>()?;
    // Descend first so unsupported instances and excessive depth cannot be
    // hidden by this normalization. The parent still proves an image and a
    // nonempty information slot in the same actual sibling card.
    if matches!(node, PenNode::Frame(_)) && content_evidence(node, 0) == (true, false) {
        return Some(format!("frame:{layout}(text-information)"));
    }
    Some(format!(
        "{}:{layout}({})",
        node_kind_str(node),
        nested.join(",")
    ))
}
