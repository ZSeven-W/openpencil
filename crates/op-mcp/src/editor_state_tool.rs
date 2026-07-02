//! Consolidated `get_editor_state` read tool — the recommended FIRST call
//! when starting a design task. Composes from the existing per-domain
//! snapshot helpers so no field computation is duplicated:
//!
//!   - active page + page count   ← `document_info_snapshot` / `list_pages_snapshot`
//!   - selection ids + count      ← `get_selection_set_snapshot`
//!   - top-level node records     ← direct walk of `active_children` using
//!     `kind_label` (mirrors `list_node_kinds`)
//!   - registered components      ← `list_components_snapshot`

use std::collections::BTreeMap;

use op_editor_core::EditorState;

use super::{McpTool, ToolOutcome};
use crate::tools::{
    active_children, document_info_snapshot, escape_record_field, get_selection_set_snapshot,
    kind_label, list_components_snapshot, list_pages_snapshot,
};

// --- GetEditorState --------------------------------------------------

/// Consolidated editor-state snapshot returned by `get_editor_state`.
pub struct GetEditorState {
    /// 0-based active page index.
    pub active_page_index: usize,
    /// Id string of the active page ("0" for single-page docs).
    pub active_page_id: String,
    /// Total page count (≥ 1).
    pub page_count: usize,
    /// Comma-separated ids of the current selection set (empty = no selection).
    pub selection_ids: String,
    /// Number of selected nodes.
    pub selection_count: usize,
    /// `;`-separated `id|name|kind` records for the active page's direct children.
    pub top_level_nodes: String,
    /// `;`-separated `name|id` records for registered components.
    pub components: String,
}

impl McpTool for GetEditorState {
    fn name(&self) -> &str {
        "get_editor_state"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        out.insert(
            "active_page_index".into(),
            self.active_page_index.to_string(),
        );
        out.insert("active_page_id".into(), self.active_page_id.clone());
        out.insert("page_count".into(), self.page_count.to_string());
        out.insert("selection_ids".into(), self.selection_ids.clone());
        out.insert("selection_count".into(), self.selection_count.to_string());
        out.insert("top_level_nodes".into(), self.top_level_nodes.clone());
        out.insert("components".into(), self.components.clone());
        ToolOutcome::Ok(out)
    }
}

/// Build a consolidated `GetEditorState` from the live `EditorState`.
/// Composes from the sibling snapshots — does NOT recompute what they
/// already compute.
pub fn get_editor_state_snapshot(state: &EditorState) -> GetEditorState {
    // --- page info (via list_pages_snapshot + document_info_snapshot) ---
    let pages_snap = list_pages_snapshot(state);
    let doc_snap = document_info_snapshot(state);

    let active_page_id = pages_snap
        .pages
        .get(pages_snap.active_page_index)
        .map(|(id, _)| id.clone())
        .unwrap_or_else(|| "0".to_string());

    // --- selection (via get_selection_set_snapshot) ---
    let sel_snap = get_selection_set_snapshot(state);
    // Filter to real ids only (mirrors selection_snapshot's filter).
    let real_ids: Vec<String> = sel_snap
        .ids
        .iter()
        .filter(|id| {
            // NodeId::NONE is represented as "0" in the snapshot string.
            id.as_str() != "0" && !id.is_empty()
        })
        .cloned()
        .collect();
    let selection_ids = real_ids.join(",");
    let selection_count = real_ids.len();

    // --- top-level nodes: id|name|kind per active page direct child ---
    // Mirror how `list_node_kinds` / `batch_get` walk active_children.
    let top_level_nodes = {
        let children = active_children(state);
        let records: Vec<String> = children
            .iter()
            .map(|n| {
                use op_editor_core::pen_node_ext::PenNodeExt;
                let id = n.id_str().to_string();
                let name = escape_record_field(n.base().name.as_deref().unwrap_or(""));
                let kind = kind_label(n);
                format!("{id}|{name}|{kind}")
            })
            .collect();
        records.join(";")
    };

    // --- components (via list_components_snapshot) ---
    let comp_snap = list_components_snapshot(state);
    let components = comp_snap
        .items
        .iter()
        .map(|(name, id)| format!("{}|{}", escape_record_field(name), id))
        .collect::<Vec<_>>()
        .join(";");

    GetEditorState {
        active_page_index: doc_snap.active_page_index,
        active_page_id,
        page_count: doc_snap.page_count,
        selection_ids,
        selection_count,
        top_level_nodes,
        components,
    }
}

// --- tests -----------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{frame, rect, state_with};
    use op_editor_core::{EditorCommand, EditorState, NodeId};

    fn make_two_frames_one_selected_one_component() -> EditorState {
        // Two top-level frames on the active (only) page.
        let mut s = state_with(vec![
            frame("n1", "Card A", 0.0, 0.0, 300.0, 200.0, Vec::new()),
            frame("n2", "Card B", 320.0, 0.0, 300.0, 200.0, Vec::new()),
        ]);
        // Select n1.
        s.set_single_selection(NodeId::new("n1"));
        // Register n1 as a component named "MyCard".
        assert!(s.apply(EditorCommand::CreateComponent {
            node_id: NodeId::new("n1"),
            name: "MyCard".into(),
        }));
        s
    }

    #[test]
    fn get_editor_state_page_count_at_least_one() {
        let s = make_two_frames_one_selected_one_component();
        let snap = get_editor_state_snapshot(&s);
        match snap.call(&BTreeMap::new()) {
            ToolOutcome::Ok(out) => {
                let page_count: usize = out["page_count"].parse().expect("page_count numeric");
                assert!(
                    page_count >= 1,
                    "page_count should be >= 1, got {page_count}"
                );
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn get_editor_state_selection_reports_selected_id_and_count_one() {
        let s = make_two_frames_one_selected_one_component();
        let snap = get_editor_state_snapshot(&s);
        match snap.call(&BTreeMap::new()) {
            ToolOutcome::Ok(out) => {
                let ids = &out["selection_ids"];
                let count: usize = out["selection_count"]
                    .parse()
                    .expect("selection_count numeric");
                assert!(
                    ids.contains("n1"),
                    "selection_ids should contain selected id 'n1', got {ids:?}"
                );
                assert_eq!(count, 1, "selection_count should be 1, got {count}");
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn get_editor_state_top_level_nodes_contains_both_frame_ids_with_kinds() {
        let s = make_two_frames_one_selected_one_component();
        let snap = get_editor_state_snapshot(&s);
        match snap.call(&BTreeMap::new()) {
            ToolOutcome::Ok(out) => {
                let nodes = &out["top_level_nodes"];
                assert!(
                    nodes.contains("n1") && nodes.contains("frame"),
                    "top_level_nodes should contain n1 with kind frame, got {nodes:?}"
                );
                assert!(
                    nodes.contains("n2"),
                    "top_level_nodes should contain n2, got {nodes:?}"
                );
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn get_editor_state_components_contains_component_name_and_id() {
        let s = make_two_frames_one_selected_one_component();
        let snap = get_editor_state_snapshot(&s);
        match snap.call(&BTreeMap::new()) {
            ToolOutcome::Ok(out) => {
                let components = &out["components"];
                assert!(
                    components.contains("MyCard") && components.contains("n1"),
                    "components should contain 'MyCard|n1', got {components:?}"
                );
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn get_editor_state_empty_doc_returns_valid_snapshot() {
        let s = EditorState::new();
        let snap = get_editor_state_snapshot(&s);
        match snap.call(&BTreeMap::new()) {
            ToolOutcome::Ok(out) => {
                let page_count: usize = out["page_count"].parse().expect("page_count");
                assert!(page_count >= 1);
                assert_eq!(out["selection_count"], "0");
                assert_eq!(out["top_level_nodes"], "");
                assert_eq!(out["components"], "");
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn get_editor_state_top_level_nodes_format_is_id_pipe_name_pipe_kind() {
        let s = state_with(vec![rect("r1", "Box", 0.0, 0.0, 50.0, 50.0)]);
        let snap = get_editor_state_snapshot(&s);
        match snap.call(&BTreeMap::new()) {
            ToolOutcome::Ok(out) => {
                // Each record: id|name|kind
                let nodes = &out["top_level_nodes"];
                assert_eq!(nodes, "r1|Box|rectangle", "got {nodes:?}");
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }
}
