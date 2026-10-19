//! Quota evidence for navigation, schedules and mixed video-feed compositions.
//! Style signatures stay strict; optional badges and row wrappers are content.

use crate::types::DocSink;
use jian_ops_schema::node::{
    base::NumberOrExpression, container::LayoutMode, PenNode, TextContent,
};
use op_editor_core::{NodeId, PenNodeExt};
use regex::Regex;
use std::sync::LazyLock;

#[derive(Clone, Copy)]
enum Kind {
    Navigation,
    Schedule,
    Video,
}

pub(super) fn delivered_count(sink: &dyn DocSink, root_ids: &[String], text: &str) -> usize {
    let text = text.to_lowercase();
    let kind = if text.contains("底部导航")
        || text.contains("bottom navigation")
        || text.contains("bottom nav")
        || text.contains("bottom-nav")
    {
        Kind::Navigation
    } else if (text.contains("日程") || text.contains("schedule") || text.contains("agenda"))
        && (text.contains("列表") || text.contains("list"))
    {
        Kind::Schedule
    } else if (text.contains("视频") || text.contains("video"))
        && ["流", "卡", "feed", "card", "tile"]
            .iter()
            .any(|word| text.contains(word))
    {
        Kind::Video
    } else {
        return 0;
    };
    root_ids
        .iter()
        .filter_map(|id| {
            op_editor_core::walkers::find_node(sink.state().active_children(), &NodeId::new(id))
        })
        .map(|node| match kind {
            Kind::Video => video_frontier(node),
            _ => sibling_count(node, kind),
        })
        .max()
        .unwrap_or(0)
}

fn visible(node: &PenNode) -> bool {
    op_design_lint::node_util::is_node_visible(node)
        && !matches!(&node.base().opacity, Some(NumberOrExpression::Number(n)) if *n <= 0.0)
}

fn text(node: &PenNode) -> Option<String> {
    let PenNode::Text(node) = node else {
        return None;
    };
    let value = match &node.content {
        TextContent::Plain(value) => value.clone(),
        TextContent::Styled(runs) => runs.iter().map(|run| run.text.as_str()).collect(),
    };
    (!value.trim().is_empty()).then_some(value)
}

/// Count actual adjacent items, never their internal icons, chips or FABs.
fn sibling_count(node: &PenNode, kind: Kind) -> usize {
    if !visible(node) || !matches!(node, PenNode::Frame(_)) {
        return 0;
    }
    let children = node.children().map(Vec::as_slice).unwrap_or(&[]);
    let mut count = 0;
    let mut nested = 0;
    for child in children.iter().filter(|node| visible(node)) {
        let item = match kind {
            Kind::Navigation => navigation_item(child),
            Kind::Schedule => schedule_item(child),
            Kind::Video => false,
        };
        if item {
            count += 1;
        } else {
            nested = nested.max(sibling_count(child, kind));
        }
    }
    count.max(nested)
}

fn navigation_item(node: &PenNode) -> bool {
    if !matches!(node, PenNode::Frame(_)) {
        return false;
    }
    fn slots(node: &PenNode, icons: &mut usize, labels: &mut usize) -> bool {
        if !visible(node) {
            return true;
        }
        match node {
            PenNode::IconFont(_) => *icons += 1,
            PenNode::Text(_) => *labels += usize::from(text(node).is_some()),
            PenNode::Ellipse(_) | PenNode::Rectangle(_) => {
                // Unread dots and active underlines cannot supply an item.
                return node.width_px().is_some_and(|n| n <= 40.0)
                    && node.height_px().is_some_and(|n| n <= 8.0);
            }
            PenNode::Frame(_) => {
                for child in node.children().into_iter().flatten() {
                    if !slots(child, icons, labels) {
                        return false;
                    }
                }
            }
            _ => return false,
        }
        true
    }
    let (mut icons, mut labels) = (0, 0);
    slots(node, &mut icons, &mut labels) && icons == 1 && labels == 1
}

fn schedule_item(node: &PenNode) -> bool {
    if !matches!(node, PenNode::Frame(_)) {
        return false;
    }
    static TIME: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\d{1,2}:\d{2}(?:\s*[–—-]\s*\d{1,2}:\d{2})?$").expect("valid schedule time")
    });
    fn evidence(node: &PenNode, times: &mut usize, labels: &mut usize) -> bool {
        if !visible(node) {
            return true;
        }
        if let Some(value) = text(node) {
            if TIME.is_match(value.trim()) {
                *times += 1;
            } else {
                *labels += 1;
            }
        } else if matches!(node, PenNode::Ref(_) | PenNode::Image(_)) {
            return false;
        }
        node.children()
            .into_iter()
            .flatten()
            .all(|child| evidence(child, times, labels))
    }
    let (mut times, mut labels) = (0, 0);
    evidence(node, &mut times, &mut labels) && (1..=2).contains(&times) && labels >= 1
}

/// A feed may own a lead card plus row-wrapped tiles. Stop at each proven
/// card so author avatars, labels and embedded recommendations cannot add items.
fn video_frontier(node: &PenNode) -> usize {
    if !visible(node) || !matches!(node, PenNode::Frame(_)) {
        return 0;
    }
    if video_card(node) {
        return 1;
    }
    node.children()
        .into_iter()
        .flatten()
        .map(video_frontier)
        .sum()
}

fn video_card(node: &PenNode) -> bool {
    let children = node.children().map(Vec::as_slice).unwrap_or(&[]);
    let covers = children.iter().filter(|child| cover_slot(child)).count();
    let title = children.iter().any(|child| {
        if text(child).is_some() && visible(child) {
            return true;
        }
        // A horizontal media row may wrap its title and author information.
        matches!(child, PenNode::Frame(_)) && text_only_information(child)
    });
    covers == 1 && title
}

fn cover_slot(node: &PenNode) -> bool {
    if !visible(node) {
        return false;
    }
    if matches!(node, PenNode::Image(_)) {
        return node.height_px().is_some_and(|n| n >= 60.0)
            && node.width_px().is_none_or(|n| n >= 80.0);
    }
    // A cover wrapper may contain a play icon, duration and a scrim.
    if let PenNode::Frame(frame) = node {
        let children = node.children().map(Vec::as_slice).unwrap_or(&[]);
        if children.len() == 1 {
            return cover_slot(&children[0]);
        }
        return frame.container.layout == Some(LayoutMode::None)
            && children.iter().filter(|child| cover_slot(child)).count() == 1
            && children
                .iter()
                .all(|child| cover_slot(child) || media_overlay(child));
    }
    false
}

fn media_overlay(node: &PenNode) -> bool {
    if !visible(node) {
        return true;
    }
    match node {
        PenNode::Rectangle(_) | PenNode::IconFont(_) => true,
        PenNode::Text(_) => text(node).is_some_and(|text| {
            text.chars().all(|c| c.is_ascii_digit() || c == ':') && text.contains(':')
        }),
        PenNode::Frame(_) => {
            node.width_px().is_some_and(|n| n <= 48.0)
                && node.height_px().is_some_and(|n| n <= 48.0)
                && node.children().into_iter().flatten().all(media_overlay)
        }
        _ => false,
    }
}

fn text_only_information(node: &PenNode) -> bool {
    if !visible(node) {
        return false;
    }
    if text(node).is_some() {
        return true;
    }
    let children = node.children().map(Vec::as_slice).unwrap_or(&[]);
    !children.iter().any(cover_slot) && children.iter().any(text_only_information)
}
