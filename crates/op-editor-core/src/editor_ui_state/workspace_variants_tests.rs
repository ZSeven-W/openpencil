//! Tests for side-by-side design directions on the workspace.

use super::*;
use crate::{HomeFamily, LaunchRoute, TaskDraft, WorkspacePhase, WorkspaceView};
use jian_ops_schema::node::PenNode;
use serde_json::json;

fn board(id: &str, name: &str, x: f64) -> PenNode {
    serde_json::from_value(json!({
        "type": "frame", "id": id, "name": name, "x": x, "y": 0,
        "width": 375, "height": 812,
        "children": [{"type": "text", "id": format!("{id}-t"), "content": "Hi"}]
    }))
    .expect("board fixture")
}

fn palette(hex: &str) -> BTreeMap<String, VariableDefinition> {
    let mut vars = BTreeMap::new();
    vars.insert(
        "accent".to_string(),
        serde_json::from_value(json!({"type": "color", "value": hex})).unwrap(),
    );
    vars
}

fn variant(index: usize, root_ids: Vec<String>, hex: &str) -> WorkspaceVariant {
    let letter = variant_letter(index);
    WorkspaceVariant {
        index,
        name: format!("方案 {letter}"),
        style_guide: format!("guide-{letter}"),
        style_label: format!("Guide {letter}"),
        name_prefix: format!("方案 {letter} · Guide {letter} · "),
        root_ids,
        variables: Some(palette(hex)),
        themes: None,
    }
}

/// A settled workspace holding three directions (B has two screens).
fn three_directions() -> EditorState {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    let mut nodes = Vec::new();
    for (letter, count, x) in [('A', 1, 0.0), ('B', 2, 615.0), ('C', 1, 1705.0)] {
        for screen in 0..count {
            nodes.push(board(
                &format!("{letter}{screen}"),
                &format!("方案 {letter} · Guide {letter} · Screen {screen}"),
                x + screen as f64 * 415.0,
            ));
        }
    }
    let ids = state
        .insert_subtree_returning_root_ids(nodes, &NodeId::NONE)
        .expect("boards insert");
    let workspace = &mut state.editor_ui.workspace;
    workspace.open_for_generation(
        HomeFamily::AppUi,
        "记账 app",
        TaskDraft::default(),
        0,
        1,
        None,
    );
    workspace.begin_variants(3);
    workspace.record_variant(variant(2, vec![ids[3].clone()], "#000002"));
    workspace.record_variant(variant(0, vec![ids[0].clone()], "#000000"));
    workspace.record_variant(variant(1, vec![ids[1].clone(), ids[2].clone()], "#000001"));
    workspace.phase = WorkspacePhase::Done;
    state
}

#[test]
fn the_count_is_clamped_and_letters_follow_slots() {
    assert_eq!(clamp_variant_count(1), 2);
    assert_eq!(clamp_variant_count(9), MAX_VARIANT_COUNT);
    assert_eq!(variant_letter(0), 'A');
    assert_eq!(variant_letter(3), 'D');
    assert_eq!(LaunchRoute::Variants(9).variant_count(), Some(4));
    assert_eq!(LaunchRoute::Orchestrator.variant_count(), None);
    assert!(LaunchRoute::Variants(3).bypasses_design_agent_loop());
    assert!(LaunchRoute::Variants(3).implies_design_intent());
    assert!(!LaunchRoute::Variants(3).forces_in_place_refine());
}

#[test]
fn directions_are_recorded_in_slot_order_and_replaced_in_place() {
    let state = three_directions();
    let workspace = &state.editor_ui.workspace;
    assert_eq!(
        workspace
            .variants
            .iter()
            .map(|v| v.index)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(workspace.variant_count, 3);
    assert_eq!(workspace.view, WorkspaceView::AllBoards);

    let mut workspace = workspace.clone();
    workspace.record_variant(variant(1, vec!["x".into()], "#ffffff"));
    assert_eq!(workspace.variants.len(), 3);
    assert_eq!(workspace.variants[1].root_ids, vec!["x".to_string()]);
    assert_eq!(workspace.variants[1].label(), "方案 B · Guide B");
}

#[test]
fn picking_waits_for_the_run_to_settle() {
    let mut state = three_directions();
    state.editor_ui.workspace.phase = WorkspacePhase::Generating;
    assert!(!state.editor_ui.workspace.variant_pick_enabled());
    state.editor_ui.workspace.phase = WorkspacePhase::Stopped;
    assert!(state.editor_ui.workspace.variant_pick_enabled());
}

#[test]
fn picking_keeps_one_direction_and_moves_the_rest_to_their_own_page() {
    let mut state = three_directions();
    let pages_before = state.doc.pages.as_ref().map_or(1, Vec::len);
    let active_before = state.ui.active_page_index;

    assert!(pick_workspace_variant(&mut state, 1, "其他方案"));

    // B stays on the working page, its direction prefix dropped.
    assert_eq!(state.ui.active_page_index, active_before);
    let names: Vec<String> = state
        .active_children()
        .iter()
        .map(|node| node.base().name.clone().unwrap_or_default())
        .collect();
    assert_eq!(names, vec!["Screen 0".to_string(), "Screen 1".to_string()]);

    // A and C moved — not deleted — to a new page.
    let pages = state.doc.pages.as_ref().expect("paged document");
    assert_eq!(pages.len(), pages_before + 1);
    let other = pages.last().unwrap();
    assert_eq!(other.name, "其他方案");
    let moved: Vec<String> = other
        .children
        .iter()
        .map(|node| node.base().name.clone().unwrap_or_default())
        .collect();
    assert_eq!(moved.len(), 2);
    assert!(moved[0].starts_with("方案 A"));
    assert!(moved[1].starts_with("方案 C"));

    // B's palette is the document's again, and the offer is gone.
    assert_eq!(state.doc.variables.as_ref(), Some(&palette("#000001")));
    assert!(state.editor_ui.workspace.variants.is_empty());
    assert!(!state.editor_ui.workspace.is_variants_run());

    // One undo reverts the whole pick.
    assert!(state.apply(EditorCommand::Undo));
    assert_eq!(state.active_children().len(), 4);
    assert_eq!(state.doc.pages.as_ref().map_or(1, Vec::len), pages_before);
}

#[test]
fn a_pick_after_the_other_boards_were_deleted_still_keeps_the_choice() {
    let mut state = three_directions();
    // The user deleted A and C by hand; the pick only renames B.
    let doomed: Vec<String> = [0usize, 2]
        .iter()
        .flat_map(|slot| state.editor_ui.workspace.variants[*slot].root_ids.clone())
        .collect();
    for id in doomed {
        assert!(state.apply(EditorCommand::DeleteNode {
            node_id: NodeId::new(id),
            page_id: None,
        }));
    }
    let pages_before = state.doc.pages.as_ref().map_or(1, Vec::len);
    assert!(pick_workspace_variant(&mut state, 1, "其他方案"));
    assert_eq!(state.doc.pages.as_ref().map_or(1, Vec::len), pages_before);
    assert_eq!(state.active_children().len(), 2);
}

#[test]
fn an_unknown_direction_is_not_picked() {
    let mut state = three_directions();
    assert!(!pick_workspace_variant(&mut state, 7, "x"));
    assert_eq!(state.editor_ui.workspace.variants.len(), 3);
}

#[test]
fn a_new_run_forgets_the_directions() {
    let mut state = three_directions();
    state.editor_ui.workspace.open_for_generation(
        HomeFamily::Web,
        "b",
        TaskDraft::default(),
        0,
        2,
        None,
    );
    assert!(state.editor_ui.workspace.variants.is_empty());
    assert_eq!(state.editor_ui.workspace.variant_count, 0);
    let mut state = three_directions();
    state.editor_ui.workspace.reset_for_new_document();
    assert!(state.editor_ui.workspace.variants.is_empty());
}
