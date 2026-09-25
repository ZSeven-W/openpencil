//! Generator section: layout agreement, status resolution, and the
//! panel's commit / action paths on a host without a runtime (the only
//! runtime lives in op-mcp; its end-to-end tests are there).

use jian_ops_schema::node::PenNode;
use op_editor_core::generator::{GeneratorError, GeneratorRequest, GENERATOR_STARTERS};
use op_editor_core::{EditorState, NodeId, PropertyFocus};

use super::*;
use crate::widgets::PropertyPanel;

fn one_rect(_: &GeneratorRequest<'_>) -> Result<Vec<PenNode>, GeneratorError> {
    Ok(vec![serde_json::from_value(serde_json::json!({
        "type": "rectangle", "id": "x", "width": 10, "height": 10
    }))
    .unwrap()])
}

fn state_with_starter() -> (EditorState, NodeId) {
    let mut state = EditorState::new();
    let id = state
        .insert_generator_starter(&GENERATOR_STARTERS[0], Some(one_rect))
        .expect("inserted");
    (state, id)
}

#[test]
fn selected_generator_gets_a_summary_with_every_param() {
    let (state, id) = state_with_starter();
    let panel = PropertyPanel::for_selection(&state).expect("panel");
    let summary = panel
        .snapshot
        .generator
        .as_ref()
        .expect("generator summary");
    assert_eq!(summary.node_id, id);
    assert_eq!(summary.params.len(), 5);
    assert_eq!(summary.params[0].label, "Title");
    let rows = summary.rows();
    assert_eq!(rows.count, 5);
    assert_eq!(rows.bool_mask, 1 << 4, "showValues is the only boolean");
}

#[test]
fn input_and_action_rects_follow_the_row_shapes() {
    let (state, _) = state_with_starter();
    let panel = PropertyPanel::for_selection(&state).expect("panel");
    let rows = panel.snapshot.generator.as_ref().unwrap().rows();
    let mut inputs = Vec::new();
    push_generator_input_rects(&mut inputs, rows, 0.0, 100.0, 260.0);
    let mut actions = Vec::new();
    push_generator_action_rects(&mut actions, rows, 0.0, 100.0, 260.0);
    let focuses: Vec<_> = inputs.iter().map(|(focus, _)| *focus).collect();
    assert_eq!(
        focuses,
        (0..4)
            .map(PropertyFocus::GeneratorParam)
            .collect::<Vec<_>>()
    );
    assert!(actions
        .iter()
        .any(|(action, _)| *action == PropertyPanelAction::ToggleGeneratorParam(4)));
    let bottom = actions
        .iter()
        .map(|(_, rect)| rect.origin.y + rect.size.y)
        .fold(0.0f32, f32::max);
    assert!(bottom <= 100.0 + generator_section_height(rows));
    // Every rect stays below the section's own header + notes.
    assert!(inputs.iter().all(|(_, rect)| rect.origin.y > 100.0));
}

#[test]
fn panel_walkers_include_the_generator_rows() {
    let (state, _) = state_with_starter();
    let panel = PropertyPanel::for_selection(&state).expect("panel");
    let rect = crate::Rect::xywh(0.0, 0.0, 260.0, 4000.0);
    let editable = crate::widgets::property_panel_input_layout::editable_input_rects(
        rect,
        panel.visible_sections(),
        &panel.snapshot.fills,
        &panel.snapshot.effects,
    );
    assert!(editable
        .iter()
        .any(|(focus, _)| *focus == PropertyFocus::GeneratorParam(2)));
}

#[test]
fn without_a_runtime_edits_are_refused_inline_and_the_doc_is_unchanged() {
    let (mut state, id) = state_with_starter();
    let doc = serde_json::to_string(&state.doc).unwrap();
    commit_param(&mut state, 2, "4");
    assert_eq!(serde_json::to_string(&state.doc).unwrap(), doc);
    let panel = PropertyPanel::for_selection(&state).expect("panel");
    let summary = panel.snapshot.generator.as_ref().unwrap();
    assert!(matches!(summary.status, Some(GeneratorStatus::Failed(_))));
    assert_eq!(state.ui.generator_error.as_ref().unwrap().node_id, id);
}

#[test]
fn detach_needs_no_runtime_and_removes_the_section() {
    let (mut state, _) = state_with_starter();
    apply_generator_action(&mut state, &PropertyPanelAction::DetachGenerator);
    let panel = PropertyPanel::for_selection(&state).expect("panel");
    assert!(panel.snapshot.generator.is_none());
    assert!(state.undo());
    let panel = PropertyPanel::for_selection(&state).expect("panel");
    assert!(panel.snapshot.generator.is_some());
}
