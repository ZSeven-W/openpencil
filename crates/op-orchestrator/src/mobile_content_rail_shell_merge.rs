//! Duplicate root-direct page-shell merge — split from
//! `mobile_content_rail.rs` (800-line file cap). A two-script fresh
//! generation can append a second, byte-equivalent page-content shell under
//! the same mobile page; this cluster merges that narrow shape before the
//! rail/header repairs run.

use super::*;
use jian_ops_schema::node::Padding;
use std::collections::BTreeMap;

/// Merge repeated root-direct page-content shells created by independent
/// fresh-generation scripts. This is intentionally a whole-document repair:
/// selected-frame append and multi-screen documents stay out of scope.
pub(super) fn merge_duplicate_page_shells(sink: &mut dyn DocSink, root_id: &str) {
    let commands = {
        let roots = sink.state().active_children();
        let [root] = roots else {
            return;
        };
        if root.id_str() != root_id || !looks_like_mobile_screen(root) {
            return;
        }
        collect_page_shell_merge_commands(root)
    };

    if let Some(commands) = commands {
        // `EditorCommand::Batch` snapshots and rolls back the whole document
        // when any move/delete is rejected. No partially merged shell can
        // escape on a concurrent-edit or stale-state failure.
        sink.apply(EditorCommand::Batch { commands });
    }
}

fn collect_page_shell_merge_commands(root: &PenNode) -> Option<Vec<EditorCommand>> {
    let mut groups: BTreeMap<String, Vec<&PenNode>> = BTreeMap::new();
    for child in root.children()? {
        if let Some(name) = page_shell_name(child) {
            groups.entry(name).or_default().push(child);
        }
    }

    let mut duplicate_groups = groups.into_values().filter(|shells| shells.len() >= 2);
    let shells = duplicate_groups.next()?;
    // Two independently duplicated shell families under one page are not a
    // high-confidence two-script shape. Decline instead of merging either.
    if duplicate_groups.next().is_some() {
        return None;
    }

    let first_fingerprint = page_shell_fingerprint(shells[0])?;
    if shells.iter().skip(1).any(|shell| {
        page_shell_fingerprint(shell).as_ref() != Some(&first_fingerprint)
            || !page_shell_heights_are_mergeable(shells[0], shell)
    }) {
        return None;
    }

    let target_id = NodeId::new(shells[0].id_str().to_string());
    let mut commands = Vec::new();
    for shell in shells.iter().skip(1) {
        for child in shell.children()? {
            commands.push(EditorCommand::MoveNode {
                node_id: NodeId::new(child.id_str().to_string()),
                target_parent: target_id.clone(),
                page_id: None,
                index: None,
            });
        }
    }
    for shell in shells.iter().skip(1) {
        commands.push(EditorCommand::DeleteNode {
            node_id: NodeId::new(shell.id_str().to_string()),
            page_id: None,
        });
    }
    Some(commands)
}

fn page_shell_name(node: &PenNode) -> Option<String> {
    let PenNode::Frame(frame) = node else {
        return None;
    };
    if frame.container.layout != Some(LayoutMode::Vertical)
        || !matches!(
            frame.container.width.as_ref(),
            Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
        )
        || frame.screen.is_some()
        || frame.route.is_some()
        || frame.breakpoint.is_some()
        || !frame.children.as_ref().is_some_and(|children| {
            children
                .iter()
                .any(op_design_lint::node_util::is_node_visible)
        })
    {
        return None;
    }

    let normalized = frame
        .base
        .name
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    matches!(
        normalized.as_str(),
        "app content" | "page content" | "main content" | "content root"
    )
    .then_some(normalized)
}

/// Canonical shell attributes. Height is deliberately excluded: the measured
/// two-script lesion is `fit_content` followed by a stale numeric flow height.
/// Padding encodings are normalized so `[v,h]` and `[v,h,v,h]` compare by
/// meaning; every other authored property remains an exact equality gate.
fn page_shell_fingerprint(node: &PenNode) -> Option<serde_json::Value> {
    let mut value = serde_json::to_value(node).ok()?;
    let object = value.as_object_mut()?;
    object.remove("id");
    object.remove("name");
    object.remove("height");
    object.remove("children");
    if let Some(padding) = container_props(node)?.padding.as_ref() {
        let canonical = match padding {
            Padding::Uniform(value) => serde_json::json!([value, value, value, value]),
            Padding::XY([vertical, horizontal]) => {
                serde_json::json!([vertical, horizontal, vertical, horizontal])
            }
            Padding::LtrB(values) => serde_json::json!(values),
            Padding::Expression(expression) => serde_json::json!(expression),
        };
        object.insert("padding".to_string(), canonical);
    }
    Some(value)
}

fn page_shell_heights_are_mergeable(first: &PenNode, later: &PenNode) -> bool {
    let first_height = container_props(first).and_then(|props| props.height.as_ref());
    let later_height = container_props(later).and_then(|props| props.height.as_ref());
    first_height == later_height
        || page_shell_height_is_hug(first_height)
        || page_shell_height_is_hug(later_height)
}

fn page_shell_height_is_hug(height: Option<&SizingBehavior>) -> bool {
    height.is_none()
        || matches!(
            height,
            Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
        )
}
