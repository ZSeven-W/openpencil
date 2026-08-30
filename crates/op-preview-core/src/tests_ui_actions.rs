//! R5 Preview UI actions through the live action/effect delivery path.

#![cfg(test)]

use super::input_event::{PreviewInput, PreviewInputEnvelope};
use super::{test_measure, PreviewEffect, PreviewHostCapabilities, PreviewSession};
use jian_core::action::services::{ScrollAlignment, UiMutationWork};

fn ui_actions_doc() -> &'static str {
    r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "x",
        "app": { "name": "x", "version": "1", "id": "x", "capabilities": [] },
        "children": [
            { "type": "frame", "id": "button", "x": 0, "y": 0,
              "width": 100, "height": 100,
              "events": { "onTap": [
                  { "show": "panel" },
                  { "hide": "panel" },
                  { "toggle_visibility": "panel" },
                  { "scroll_to": { "target": "panel", "alignment": "center" } },
                  { "dismiss_keyboard": {} }
              ] } },
            { "type": "frame", "id": "panel", "x": 0, "y": 120,
              "width": 100, "height": 100 }
        ]
    }"##
}

fn tap(session: &mut PreviewSession) {
    let mut down = jian_core::gesture::PointerEvent::simple_at(
        1,
        jian_core::gesture::pointer::PointerPhase::Down,
        jian_core::geometry::point(50.0, 50.0),
        0,
    );
    down.kind = jian_core::gesture::pointer::PointerKind::Touch;
    let _ = session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Pointer(down)));

    let mut up = jian_core::gesture::PointerEvent::simple_at(
        1,
        jian_core::gesture::pointer::PointerPhase::Up,
        jian_core::geometry::point(50.0, 50.0),
        10,
    );
    up.kind = jian_core::gesture::pointer::PointerKind::Touch;
    let _ = session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Pointer(up)));
}

#[test]
fn safe_ui_chain_parses_and_dismiss_keyboard_reaches_the_effect_queue() {
    let document = jian_ops_schema::load_str(ui_actions_doc())
        .expect("parse document")
        .value;
    let mut session = PreviewSession::enter_with_capabilities(
        &document,
        (800.0, 600.0),
        &std::collections::BTreeMap::new(),
        0,
        false,
        false,
        test_measure(),
        PreviewHostCapabilities {
            dismiss_keyboard: true,
            ..PreviewHostCapabilities::none()
        },
    )
    .expect("safe UI actions must load");

    tap(&mut session);
    assert!(
        session.action_visibility_for("panel", true),
        "show then hide then toggle leaves the panel visible"
    );
    assert_eq!(
        session.drain_action_scroll_requests(),
        vec![("panel".to_owned(), ScrollAlignment::Center)]
    );
    assert_eq!(
        session.take_ui_action_work(),
        UiMutationWork::REDRAW_AND_HIT_TEST
    );
    assert_eq!(session.take_ui_action_work(), UiMutationWork::NONE);
    assert!(matches!(
        session.drain_effects().as_slice(),
        [PreviewEffect::DismissKeyboard { .. }]
    ));
}
