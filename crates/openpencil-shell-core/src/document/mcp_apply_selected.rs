//! Apply helpers for selection / multi-selection / tool / flag /
//! viewport / undo-redo MCP write commands. Carved off
//! `mcp_apply.rs` to stay under the 800-line cap.

use super::mcp_apply::find_node_mut_in_doc;
use super::{Document, NodeId, ReorderDirection, Tool};

/// Selection lookup must be scoped to the *active page* — the
/// canvas viewport, layer panel, and every selection-aware
/// mutator (`set_selected_bounds`, `translate_selected`, etc.)
/// only operate against the active page's tree. Codex stop-gate:
/// `find_node_in_doc` walks every page, so plumbing it through
/// the selection apply path would let an MCP caller stash an
/// off-page id in `selected_set`. Subsequent reads (`get_node`,
/// `is_editable`, etc.) would then return None for that id and
/// the doc would behave as if nothing was selected — silent
/// breakage instead of an upfront reject.
fn active_page_has(doc: &Document, target: NodeId) -> bool {
    doc.active_page()
        .map(|p| p.find(target).is_some())
        .unwrap_or(false)
}

pub(super) fn apply_set_selection(doc: &mut Document, node_id: u64) -> bool {
    let Some(target) = NodeId::new_opt(node_id) else {
        return false;
    };
    if !active_page_has(doc, target) {
        return false;
    }
    doc.set_single_selection(target);
    true
}

pub(super) fn apply_toggle_node_selection(doc: &mut Document, node_id: u64) -> bool {
    let Some(target) = NodeId::new_opt(node_id) else {
        return false;
    };
    if !active_page_has(doc, target) {
        return false;
    }
    doc.toggle_selection(target);
    true
}

pub(super) fn apply_set_selection_set(doc: &mut Document, node_ids: &[u64]) -> bool {
    // Resolve every id against the active page, dropping unknown
    // / off-page ones (panel races with delete or stale ids from
    // a previous page must not abort the call). Empty post-filter
    // list clears the selection — matching the documented behavior.
    let mut resolved: Vec<NodeId> = Vec::with_capacity(node_ids.len());
    let mut seen: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    for raw in node_ids {
        if !seen.insert(*raw) {
            continue;
        }
        if let Some(id) = NodeId::new_opt(*raw) {
            if active_page_has(doc, id) {
                resolved.push(id);
            }
        }
    }
    if resolved.is_empty() {
        doc.clear_selection();
    } else {
        doc.selected_set = resolved.clone();
        doc.selected = *resolved.last().expect("resolved non-empty");
        doc.ui.align_toolbar_hover = None;
    }
    true
}

pub(super) fn apply_duplicate_selected(doc: &mut Document, offset_px: i32) -> bool {
    let Some(mut next_id) = doc.next_node_id_seed() else {
        return false;
    };
    doc.duplicate_selected(&mut next_id, offset_px as f32)
        .is_some()
}

pub(super) fn apply_delete_selected(doc: &mut Document) -> bool {
    if !doc.selected.is_real() {
        return false;
    }
    let snap = doc.snapshot_for_history();
    if doc.delete_selected() {
        doc.history_push_past(snap);
        return true;
    }
    false
}

pub(super) fn apply_nudge_selected(doc: &mut Document, dx: i32, dy: i32) -> bool {
    if !doc.selected.is_real() {
        return false;
    }
    if dx == 0 && dy == 0 {
        return false;
    }
    let snap = doc.snapshot_for_history();
    doc.translate_selected(dx as f32, dy as f32);
    doc.history_push_past(snap);
    true
}

pub(super) fn apply_group_selected(doc: &mut Document) -> bool {
    let Some(mut next_id) = doc.next_node_id_seed() else {
        return false;
    };
    let snap = doc.snapshot_for_history();
    if doc.group_selected(&mut next_id).is_some() {
        doc.history_push_past(snap);
        return true;
    }
    false
}

pub(super) fn apply_ungroup_selected(doc: &mut Document) -> bool {
    let snap = doc.snapshot_for_history();
    if doc.ungroup_selected() {
        doc.history_push_past(snap);
        return true;
    }
    false
}

pub(super) fn apply_reorder_selected(doc: &mut Document, direction: &str) -> bool {
    let dir = match direction {
        "up" => ReorderDirection::Up,
        "down" => ReorderDirection::Down,
        _ => return false,
    };
    if !doc.selected.is_real() {
        return false;
    }
    let snap = doc.snapshot_for_history();
    if doc.reorder_selected(dir) {
        doc.history_push_past(snap);
        return true;
    }
    false
}

pub(super) fn apply_set_active_tool(doc: &mut Document, tool: &str) -> bool {
    let new_tool = match tool {
        "select" => Tool::Select,
        "rect" => Tool::Rect,
        "ellipse" => Tool::Ellipse,
        "polygon" => Tool::Polygon,
        "line" => Tool::Line,
        "pen" => Tool::Pen,
        "text" => Tool::Text,
        "frame" => Tool::Frame,
        "hand" => Tool::Hand,
        _ => return false,
    };
    if doc.tool == new_tool {
        return true;
    }
    doc.tool = new_tool;
    true
}

pub(super) fn apply_set_node_flag(
    doc: &mut Document,
    node_id: u64,
    flag: crate::mcp::NodeFlag,
    value: bool,
) -> bool {
    let Some(target) = NodeId::new_opt(node_id) else {
        return false;
    };
    let Some(node) = find_node_mut_in_doc(doc, target) else {
        return false;
    };
    match flag {
        crate::mcp::NodeFlag::Hidden => node.hidden = value,
        crate::mcp::NodeFlag::Locked => node.locked = value,
        crate::mcp::NodeFlag::Collapsed => node.collapsed = value,
    }
    true
}

pub(super) fn apply_copy_selected(doc: &mut Document) -> bool {
    doc.copy_selected()
}

pub(super) fn apply_cut_selected(doc: &mut Document) -> bool {
    // Atomicity (clipboard rollback on failed delete) lives in
    // `Document::cut_selected` so any caller — MCP, future
    // keyboard handler, ad-hoc test — gets the same guarantee.
    // Codex stop-gate: previously the rollback lived only in this
    // MCP wrapper, so a non-MCP `cut_selected` call still leaked
    // the would-be-cut nodes into the clipboard on delete reject.
    let snap = doc.snapshot_for_history();
    if doc.cut_selected() {
        doc.history_push_past(snap);
        return true;
    }
    false
}

pub(super) fn apply_paste_clipboard(doc: &mut Document, offset_px: i32) -> bool {
    let Some(mut next_id) = doc.next_node_id_seed() else {
        return false;
    };
    let snap = doc.snapshot_for_history();
    let new_ids = doc.paste_clipboard(&mut next_id, offset_px as f32);
    if new_ids.is_empty() {
        return false;
    }
    doc.history_push_past(snap);
    true
}

pub(super) fn apply_set_viewport(
    doc: &mut Document,
    pan_x: Option<i32>,
    pan_y: Option<i32>,
    zoom_percent: Option<i32>,
) -> bool {
    let mut changed = false;
    if let Some(x) = pan_x {
        doc.viewport.pan_x = x as f32;
        changed = true;
    }
    if let Some(y) = pan_y {
        doc.viewport.pan_y = y as f32;
        changed = true;
    }
    if let Some(z) = zoom_percent {
        // Clamp into a sane visual range so the canvas can't be
        // zoomed to a degenerate size.
        let z = z.max(10).min(2000);
        doc.viewport.zoom = z as f32 / 100.0;
        changed = true;
    }
    changed
}
