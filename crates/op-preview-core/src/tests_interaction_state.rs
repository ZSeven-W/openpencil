//! R4 interaction-state tracking — per-pointer pressed nodes and the
//! hovered node, updated by the session's own dispatch paths and read
//! through [`PreviewSession::interaction`].
//!
//! Rules under test: any kind's `Down` records the pressed node, `Up`/
//! `Cancel` clear it, unpressed Mouse (and Pen) movement tracks hover,
//! Touch NEVER hovers, and a lifecycle-exit ownership cancel clears all
//! presses without moving the mouse's hover.

#![cfg(test)]

use super::input_event::{PreviewInput, PreviewInputEnvelope};
use super::{test_measure, PreviewSession};
use jian_core::geometry::point;
use jian_core::gesture::pointer::{PointerKind, PointerPhase};
use jian_core::gesture::PointerEvent;

fn default_theme() -> std::collections::BTreeMap<String, String> {
    std::collections::BTreeMap::new()
}

fn enter() -> PreviewSession {
    let src = r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "children": [
            { "type": "frame", "id": "screen", "width": 400, "height": 400,
              "children": [
                  { "type": "rectangle", "id": "stage", "x": 40, "y": 40,
                    "width": 320, "height": 320 }
              ] }
        ]
    }"##;
    let doc = jian_ops_schema::load_str(src).expect("parse doc").value;
    PreviewSession::enter(
        &doc,
        (800.0, 600.0),
        &default_theme(),
        0,
        false,
        false,
        test_measure(),
    )
    .expect("enter preview")
}

fn pointer(
    id: u32,
    kind: PointerKind,
    phase: PointerPhase,
    x: f32,
    y: f32,
) -> PreviewInputEnvelope {
    let mut ev = PointerEvent::simple_at(id, phase, point(x, y), 0);
    ev.kind = kind;
    PreviewInputEnvelope::new(PreviewInput::Pointer(ev))
}

/// A Touch Down presses the hit node; the Up clears back to idle.
/// Touch movement (pressed or not) NEVER sets hover.
#[test]
fn touch_down_presses_and_up_clears_without_hover() {
    let mut s = enter();
    let _ = s.dispatch_input(pointer(
        1,
        PointerKind::Touch,
        PointerPhase::Down,
        100.0,
        100.0,
    ));
    assert_eq!(
        s.interaction().pressed_node(1),
        Some("stage"),
        "the hit node records as pressed"
    );
    // Pressed touch movement stays pressed and never hovers.
    let _ = s.dispatch_input(pointer(
        1,
        PointerKind::Touch,
        PointerPhase::Move,
        120.0,
        120.0,
    ));
    assert_eq!(s.interaction().pressed_node(1), Some("stage"));
    let _ = s.dispatch_input(pointer(
        1,
        PointerKind::Touch,
        PointerPhase::Hover,
        130.0,
        130.0,
    ));
    assert!(
        s.interaction().hovered_node().is_none(),
        "touch hover is a contradiction — never tracked"
    );
    let _ = s.dispatch_input(pointer(
        1,
        PointerKind::Touch,
        PointerPhase::Up,
        130.0,
        130.0,
    ));
    assert!(s.interaction().pressed_node(1).is_none(), "Up clears");
}

/// A Mouse Down presses the hit node; unpressed mouse movement hovers,
/// including clearing hover when the pointer leaves every node.
#[test]
fn mouse_presses_and_hover_tracks_unpressed_movement() {
    let mut s = enter();
    let _ = s.dispatch_input(pointer(
        1,
        PointerKind::Mouse,
        PointerPhase::Hover,
        100.0,
        100.0,
    ));
    assert_eq!(s.interaction().hovered_node(), Some("stage"));
    // Off every mapped node (the 400x400 screen included): hover clears.
    let _ = s.dispatch_input(pointer(
        1,
        PointerKind::Mouse,
        PointerPhase::Hover,
        405.0,
        405.0,
    ));
    assert!(
        s.interaction().hovered_node().is_none(),
        "leaving clears hover"
    );
    // A mouse press records the pressed node like any kind.
    let _ = s.dispatch_input(pointer(
        1,
        PointerKind::Mouse,
        PointerPhase::Down,
        100.0,
        100.0,
    ));
    assert_eq!(s.interaction().pressed_node(1), Some("stage"));
    let _ = s.dispatch_input(pointer(
        1,
        PointerKind::Mouse,
        PointerPhase::Cancel,
        100.0,
        100.0,
    ));
    assert!(s.interaction().pressed_node(1).is_none(), "Cancel clears");
}

/// Two pointers press independently; a lifecycle-exit ownership cancel
/// (`cancel_input_ownership`) clears every press while leaving the
/// (unpressed) mouse's hover alone.
#[test]
fn ownership_cancel_clears_all_presses_but_keeps_hover() {
    let mut s = enter();
    let _ = s.dispatch_input(pointer(
        1,
        PointerKind::Touch,
        PointerPhase::Down,
        100.0,
        100.0,
    ));
    let _ = s.dispatch_input(pointer(
        2,
        PointerKind::Touch,
        PointerPhase::Down,
        300.0,
        300.0,
    ));
    let _ = s.dispatch_input(pointer(
        3,
        PointerKind::Mouse,
        PointerPhase::Hover,
        120.0,
        120.0,
    ));
    // Both pointers press the SAME node — the deduplicated view has one
    // entry.
    assert_eq!(s.interaction().pressed_nodes(), vec!["stage"]);
    s.cancel_input_ownership("background-barrier");
    assert!(
        s.interaction().pressed_nodes().is_empty(),
        "every press cleared"
    );
    assert_eq!(
        s.interaction().hovered_node(),
        Some("stage"),
        "hover survives — the mouse did not move"
    );
}
