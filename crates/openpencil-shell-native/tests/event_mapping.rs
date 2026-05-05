//! Spec v19 §5.1.1 unit tests — Jian `PointerEvent` → OP `ShellEvent`
//! mapper (`JianPointerMapper`).
//!
//! Coverage target (plan v7 Task 3 Step 11 + spec §5.1.1 fixture list):
//! - 4 Touch phases (Started / Moved / Ended / Cancelled).
//! - Mouse Hover → `[PointerMove]`; LEFT Down/Up pair → Pressed/Released.
//! - Multi-button Move (press / release mid-gesture) — diff each bit
//!   then trailing PointerMove (CONCERN-R3-1).
//! - Pen / Stylus / Trackpad route through the Mouse branch.
//! - Degraded inputs (no button change on Down/Up, Touch Hover) →
//!   `Vec::new()` (spec round 4 CONCERN-R4-1; no `ShellEvent::Other`).
//! - Modifiers propagate (shift/ctrl/alt/meta/CMD).

use std::time::Instant;

use jian_core::geometry::point;
use jian_core::gesture::{
    Modifiers as JianModifiers, MouseButtons as JianMouseButtons, PointerEvent as JianPointerEvent,
    PointerId as JianPointerId, PointerKind, PointerPhase,
};
use openpencil_shell_core::event::{
    ElementState, Modifiers, MouseButton, PointerId, ShellEvent, TouchForce, TouchId, TouchPhase,
};
use openpencil_shell_native::JianPointerMapper;

/// Build a `JianPointerEvent` with default tilt / pressure / timestamp,
/// overriding the fields each test cares about.
fn jian_event(
    id: u32,
    kind: PointerKind,
    phase: PointerPhase,
    buttons: JianMouseButtons,
    modifiers: JianModifiers,
    pos_x: f32,
    pos_y: f32,
) -> JianPointerEvent {
    JianPointerEvent {
        id: JianPointerId(id),
        kind,
        phase,
        position: point(pos_x, pos_y),
        pressure: 1.0,
        buttons,
        modifiers,
        tilt: None,
        timestamp: Instant::now(),
    }
}

// ---------------------------------------------------------------- Touch ----

#[test]
fn touch_down_emits_started_phase() {
    let mut mapper = JianPointerMapper::new();
    let ev = jian_event(
        7,
        PointerKind::Touch,
        PointerPhase::Down,
        JianMouseButtons::empty(),
        JianModifiers::empty(),
        10.0,
        20.0,
    );
    let out = mapper.from_jian_pointer(&ev);
    assert_eq!(out.len(), 1);
    match &out[0] {
        ShellEvent::Touch {
            id,
            phase,
            pos,
            force,
        } => {
            assert_eq!(*id, TouchId(7));
            assert_eq!(*phase, TouchPhase::Started);
            assert_eq!(pos.x, 10.0);
            assert_eq!(pos.y, 20.0);
            assert_eq!(*force, Some(TouchForce::Normalized(1.0)));
        }
        other => panic!("expected ShellEvent::Touch, got {other:?}"),
    }
}

#[test]
fn touch_move_emits_moved_phase() {
    let mut mapper = JianPointerMapper::new();
    let ev = jian_event(
        1,
        PointerKind::Touch,
        PointerPhase::Move,
        JianMouseButtons::empty(),
        JianModifiers::empty(),
        0.0,
        0.0,
    );
    let out = mapper.from_jian_pointer(&ev);
    assert!(matches!(
        out.as_slice(),
        [ShellEvent::Touch {
            phase: TouchPhase::Moved,
            ..
        }]
    ));
}

#[test]
fn touch_up_emits_ended_phase() {
    let mut mapper = JianPointerMapper::new();
    let ev = jian_event(
        1,
        PointerKind::Touch,
        PointerPhase::Up,
        JianMouseButtons::empty(),
        JianModifiers::empty(),
        0.0,
        0.0,
    );
    let out = mapper.from_jian_pointer(&ev);
    assert!(matches!(
        out.as_slice(),
        [ShellEvent::Touch {
            phase: TouchPhase::Ended,
            ..
        }]
    ));
}

#[test]
fn touch_cancel_emits_cancelled_phase() {
    let mut mapper = JianPointerMapper::new();
    let ev = jian_event(
        1,
        PointerKind::Touch,
        PointerPhase::Cancel,
        JianMouseButtons::empty(),
        JianModifiers::empty(),
        0.0,
        0.0,
    );
    let out = mapper.from_jian_pointer(&ev);
    assert!(matches!(
        out.as_slice(),
        [ShellEvent::Touch {
            phase: TouchPhase::Cancelled,
            ..
        }]
    ));
}

#[test]
fn touch_hover_returns_empty_vec() {
    // Touches never `Hover`; mapper drops the event so callers don't
    // synthesize a fake `Moved` phase.
    let mut mapper = JianPointerMapper::new();
    let ev = jian_event(
        1,
        PointerKind::Touch,
        PointerPhase::Hover,
        JianMouseButtons::empty(),
        JianModifiers::empty(),
        0.0,
        0.0,
    );
    let out = mapper.from_jian_pointer(&ev);
    assert!(out.is_empty(), "expected empty Vec for touch Hover");
}

// ---------------------------------------------------------------- Mouse ----

#[test]
fn mouse_hover_emits_pointer_move() {
    let mut mapper = JianPointerMapper::new();
    let ev = jian_event(
        3,
        PointerKind::Mouse,
        PointerPhase::Hover,
        JianMouseButtons::empty(),
        JianModifiers::empty(),
        100.0,
        50.0,
    );
    let out = mapper.from_jian_pointer(&ev);
    assert_eq!(out.len(), 1);
    match &out[0] {
        ShellEvent::PointerMove { id, pos, modifiers } => {
            assert_eq!(*id, PointerId(3));
            assert_eq!(pos.x, 100.0);
            assert_eq!(pos.y, 50.0);
            assert_eq!(*modifiers, Modifiers::default());
        }
        other => panic!("expected PointerMove, got {other:?}"),
    }
}

#[test]
fn mouse_left_down_then_up() {
    let mut mapper = JianPointerMapper::new();

    // Down: previous = empty, current = LEFT → emit Pressed.
    let down = jian_event(
        9,
        PointerKind::Mouse,
        PointerPhase::Down,
        JianMouseButtons::LEFT,
        JianModifiers::empty(),
        0.0,
        0.0,
    );
    let out = mapper.from_jian_pointer(&down);
    assert_eq!(out.len(), 1);
    match &out[0] {
        ShellEvent::PointerButton { button, state, .. } => {
            assert_eq!(*button, MouseButton::Left);
            assert_eq!(*state, ElementState::Pressed);
        }
        other => panic!("expected PointerButton{{Pressed}}, got {other:?}"),
    }

    // Up: previous = LEFT, current = empty → emit Released.
    let up = jian_event(
        9,
        PointerKind::Mouse,
        PointerPhase::Up,
        JianMouseButtons::empty(),
        JianModifiers::empty(),
        0.0,
        0.0,
    );
    let out = mapper.from_jian_pointer(&up);
    assert_eq!(out.len(), 1);
    match &out[0] {
        ShellEvent::PointerButton { button, state, .. } => {
            assert_eq!(*button, MouseButton::Left);
            assert_eq!(*state, ElementState::Released);
        }
        other => panic!("expected PointerButton{{Released}}, got {other:?}"),
    }
}

#[test]
fn multi_button_press_during_move() {
    // CONCERN-R3-1 fixture (a): LEFT held + Move with LEFT|RIGHT →
    // [PointerButton{RIGHT, Pressed}, PointerMove].
    let mut mapper = JianPointerMapper::new();

    // Prime previous = LEFT via a Down.
    let prime = jian_event(
        4,
        PointerKind::Mouse,
        PointerPhase::Down,
        JianMouseButtons::LEFT,
        JianModifiers::empty(),
        0.0,
        0.0,
    );
    let _ = mapper.from_jian_pointer(&prime);

    // Now Move with LEFT|RIGHT.
    let mid = jian_event(
        4,
        PointerKind::Mouse,
        PointerPhase::Move,
        JianMouseButtons::LEFT | JianMouseButtons::RIGHT,
        JianModifiers::empty(),
        5.0,
        7.0,
    );
    let out = mapper.from_jian_pointer(&mid);
    assert_eq!(out.len(), 2, "expected button Pressed + PointerMove");
    match &out[0] {
        ShellEvent::PointerButton { button, state, .. } => {
            assert_eq!(*button, MouseButton::Right);
            assert_eq!(*state, ElementState::Pressed);
        }
        other => panic!("expected PointerButton{{Right,Pressed}}, got {other:?}"),
    }
    assert!(matches!(out[1], ShellEvent::PointerMove { .. }));
}

#[test]
fn multi_button_release_during_move() {
    // CONCERN-R3-1 fixture (b): LEFT|RIGHT held + Move with LEFT only →
    // [PointerButton{RIGHT, Released}, PointerMove].
    let mut mapper = JianPointerMapper::new();

    // Prime previous = LEFT|RIGHT via two Downs.
    let _ = mapper.from_jian_pointer(&jian_event(
        2,
        PointerKind::Mouse,
        PointerPhase::Down,
        JianMouseButtons::LEFT,
        JianModifiers::empty(),
        0.0,
        0.0,
    ));
    let _ = mapper.from_jian_pointer(&jian_event(
        2,
        PointerKind::Mouse,
        PointerPhase::Move,
        JianMouseButtons::LEFT | JianMouseButtons::RIGHT,
        JianModifiers::empty(),
        0.0,
        0.0,
    ));

    let release = jian_event(
        2,
        PointerKind::Mouse,
        PointerPhase::Move,
        JianMouseButtons::LEFT,
        JianModifiers::empty(),
        9.0,
        9.0,
    );
    let out = mapper.from_jian_pointer(&release);
    assert_eq!(out.len(), 2, "expected button Released + PointerMove");
    match &out[0] {
        ShellEvent::PointerButton { button, state, .. } => {
            assert_eq!(*button, MouseButton::Right);
            assert_eq!(*state, ElementState::Released);
        }
        other => panic!("expected PointerButton{{Right,Released}}, got {other:?}"),
    }
    assert!(matches!(out[1], ShellEvent::PointerMove { .. }));
}

// ---------------------------------------------------- Pen / Stylus / Trackpad

#[test]
fn pen_phase_routes_through_mouse_branch() {
    let mut mapper = JianPointerMapper::new();
    let ev = jian_event(
        11,
        PointerKind::Pen,
        PointerPhase::Down,
        JianMouseButtons::LEFT,
        JianModifiers::empty(),
        0.0,
        0.0,
    );
    let out = mapper.from_jian_pointer(&ev);
    assert_eq!(out.len(), 1);
    assert!(matches!(
        out[0],
        ShellEvent::PointerButton {
            button: MouseButton::Left,
            state: ElementState::Pressed,
            ..
        }
    ));
}

#[test]
fn stylus_phase_routes_through_mouse_branch() {
    let mut mapper = JianPointerMapper::new();
    let ev = jian_event(
        12,
        PointerKind::Stylus,
        PointerPhase::Down,
        JianMouseButtons::LEFT,
        JianModifiers::empty(),
        0.0,
        0.0,
    );
    let out = mapper.from_jian_pointer(&ev);
    assert!(matches!(
        out.as_slice(),
        [ShellEvent::PointerButton {
            button: MouseButton::Left,
            state: ElementState::Pressed,
            ..
        }]
    ));
}

#[test]
fn trackpad_phase_routes_through_mouse_branch() {
    let mut mapper = JianPointerMapper::new();
    let ev = jian_event(
        13,
        PointerKind::Trackpad,
        PointerPhase::Move,
        JianMouseButtons::empty(),
        JianModifiers::empty(),
        1.0,
        2.0,
    );
    let out = mapper.from_jian_pointer(&ev);
    // No buttons changed → only a PointerMove.
    assert!(matches!(out.as_slice(), [ShellEvent::PointerMove { .. }]));
}

// ---------------------------------------------------------- Degraded inputs

#[test]
fn degraded_down_no_button_change_returns_empty() {
    // Round 4 CONCERN-R4-1 fix: empty buttons on Down with empty
    // previous → mapper returns `Vec::new()` (no `ShellEvent::Other`).
    let mut mapper = JianPointerMapper::new();
    let ev = jian_event(
        5,
        PointerKind::Mouse,
        PointerPhase::Down,
        JianMouseButtons::empty(),
        JianModifiers::empty(),
        0.0,
        0.0,
    );
    let out = mapper.from_jian_pointer(&ev);
    assert!(out.is_empty(), "expected empty Vec, got {out:?}");
}

#[test]
fn degraded_up_no_button_change_returns_empty() {
    // Up where current buttons match previous → no diff, no Move
    // (Up phase doesn't emit Move) → empty Vec.
    let mut mapper = JianPointerMapper::new();
    // Prime previous = LEFT.
    let _ = mapper.from_jian_pointer(&jian_event(
        6,
        PointerKind::Mouse,
        PointerPhase::Down,
        JianMouseButtons::LEFT,
        JianModifiers::empty(),
        0.0,
        0.0,
    ));
    // Now Up but buttons still LEFT (e.g. host-side bookkeeping bug).
    let ev = jian_event(
        6,
        PointerKind::Mouse,
        PointerPhase::Up,
        JianMouseButtons::LEFT,
        JianModifiers::empty(),
        0.0,
        0.0,
    );
    let out = mapper.from_jian_pointer(&ev);
    assert!(out.is_empty(), "expected empty Vec, got {out:?}");
}

// ---------------------------------------------------------------- Modifiers

#[test]
fn modifiers_propagate() {
    let mut mapper = JianPointerMapper::new();
    let mods = JianModifiers::SHIFT | JianModifiers::CTRL | JianModifiers::ALT | JianModifiers::CMD;
    let ev = jian_event(
        8,
        PointerKind::Mouse,
        PointerPhase::Hover,
        JianMouseButtons::empty(),
        mods,
        0.0,
        0.0,
    );
    let out = mapper.from_jian_pointer(&ev);
    assert_eq!(out.len(), 1);
    match &out[0] {
        ShellEvent::PointerMove { modifiers, .. } => {
            assert!(modifiers.shift);
            assert!(modifiers.ctrl);
            assert!(modifiers.alt);
            assert!(modifiers.meta, "Jian CMD must map to OP meta");
        }
        other => panic!("expected PointerMove, got {other:?}"),
    }
}
