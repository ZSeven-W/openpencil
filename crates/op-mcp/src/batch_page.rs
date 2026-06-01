//! Outer `pageId` handling shared by batch design paths.

use std::collections::BTreeMap;

use super::EditorCommand;

pub(crate) fn optional_page_id(args: &BTreeMap<String, String>) -> Option<String> {
    args.get("pageId")
        .or_else(|| args.get("page_id"))
        .or_else(|| args.get("page"))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

pub(crate) fn command_with_outer_page_id(
    cmd: EditorCommand,
    page_id: Option<String>,
) -> EditorCommand {
    let Some(page_id) = page_id else {
        return cmd;
    };
    match cmd {
        EditorCommand::UpdateNode {
            node_id,
            x,
            y,
            width,
            height,
            name,
            fill_hex,
            page_id: existing,
        } => EditorCommand::UpdateNode {
            node_id,
            x,
            y,
            width,
            height,
            name,
            fill_hex,
            page_id: existing.or(Some(page_id)),
        },
        EditorCommand::PatchNodeData {
            node_id,
            patch_json,
            page_id: existing,
        } => EditorCommand::PatchNodeData {
            node_id,
            patch_json,
            page_id: existing.or(Some(page_id)),
        },
        EditorCommand::DeleteNode {
            node_id,
            page_id: existing,
        } => EditorCommand::DeleteNode {
            node_id,
            page_id: existing.or(Some(page_id)),
        },
        EditorCommand::MoveNode {
            node_id,
            target_parent,
            page_id: existing,
            index,
        } => EditorCommand::MoveNode {
            node_id,
            target_parent,
            page_id: existing.or(Some(page_id)),
            index,
        },
        EditorCommand::CopyNode {
            node_id,
            target_parent,
            overrides_json,
            page_id: existing,
        } => EditorCommand::CopyNode {
            node_id,
            target_parent,
            overrides_json,
            page_id: existing.or(Some(page_id)),
        },
        EditorCommand::ReplaceNode {
            node_id,
            kind,
            name,
            x,
            y,
            width,
            height,
            fill_hex,
            drop_children,
            page_id: existing,
        } => EditorCommand::ReplaceNode {
            node_id,
            kind,
            name,
            x,
            y,
            width,
            height,
            fill_hex,
            drop_children,
            page_id: existing.or(Some(page_id)),
        },
        EditorCommand::ReplaceSubtree {
            node_id,
            node,
            drop_children,
            page_id: existing,
        } => EditorCommand::ReplaceSubtree {
            node_id,
            node,
            drop_children,
            page_id: existing.or(Some(page_id)),
        },
        EditorCommand::BatchInsert {
            items,
            page_id: existing,
        } => EditorCommand::BatchInsert {
            items,
            page_id: existing.or(Some(page_id)),
        },
        EditorCommand::InsertSubtree {
            nodes,
            parent_id,
            page_id: existing,
        } => EditorCommand::InsertSubtree {
            nodes,
            parent_id,
            page_id: existing.or(Some(page_id)),
        },
        other => other,
    }
}
