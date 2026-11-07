//! Interactions authored through the property panel's structured edits
//! (`op_editor_core::interaction_edit_apply`) must run in Preview
//! exactly as written — the panel and the runtime share one wire shape.

#![cfg(test)]

use super::input_event::{PreviewInput, PreviewInputEnvelope};
use super::{test_measure, PreviewHostCapabilities, PreviewSession};
use op_editor_core::interaction_edit::{InteractionActionKind, InteractionTrigger};
use op_editor_core::interaction_edit_apply::InteractionEdit;

fn authored_document() -> jian_ops_schema::PenDocument {
    let doc = jian_ops_schema::load_str(
        r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "x",
        "app": { "name": "x", "version": "1", "id": "x", "capabilities": [] },
        "children": [
            { "type": "frame", "id": "screen", "x": 0, "y": 0, "width": 300, "height": 400,
              "children": [
                { "type": "frame", "id": "button", "x": 0, "y": 0,
                  "width": 100, "height": 100 },
                { "type": "frame", "id": "sheet", "x": 0, "y": 120, "visible": false,
                  "width": 100, "height": 100 }
              ] }
        ]
    }"##,
    )
    .expect("parse document")
    .value;
    let mut state = op_editor_core::EditorState::from_document(doc);
    state.selection.anchor = op_editor_core::NodeId::new("button");
    let tap = InteractionTrigger::Tap;
    let edits = [
        InteractionEdit::AddTrigger(tap),
        InteractionEdit::SetActionKind {
            trigger: tap,
            index: 0,
            kind: InteractionActionKind::ToggleVisibility,
        },
        InteractionEdit::AddAction(tap),
        InteractionEdit::SetActionKind {
            trigger: tap,
            index: 1,
            kind: InteractionActionKind::ToggleState,
        },
    ];
    for edit in &edits {
        assert!(state.apply_selected_interaction_edit(edit), "{edit:?}");
    }
    state.doc.clone()
}

fn tap(session: &mut PreviewSession) {
    for (phase, t) in [
        (jian_core::gesture::pointer::PointerPhase::Down, 0),
        (jian_core::gesture::pointer::PointerPhase::Up, 10),
    ] {
        let mut event = jian_core::gesture::PointerEvent::simple_at(
            1,
            phase,
            jian_core::geometry::point(50.0, 50.0),
            t,
        );
        event.kind = jian_core::gesture::pointer::PointerKind::Touch;
        let _ = session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Pointer(event)));
    }
}

#[test]
fn panel_authored_toggle_layer_and_state_run_in_preview() {
    let document = authored_document();
    let mut session = PreviewSession::enter_with_capabilities(
        &document,
        (800.0, 600.0),
        &std::collections::BTreeMap::new(),
        0,
        false,
        false,
        test_measure(),
        PreviewHostCapabilities::none(),
        0,
    )
    .expect("panel-authored actions must load");

    tap(&mut session);
    assert!(
        session.action_visibility_for("sheet", false),
        "the first tap reveals the hidden sheet"
    );
    tap(&mut session);
    assert!(
        !session.action_visibility_for("sheet", false),
        "the second tap hides it again"
    );
}
