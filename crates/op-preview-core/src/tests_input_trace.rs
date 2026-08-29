//! R4 Canonical PreviewInput — the unified `dispatch_input` trace.
//!
//! Drives pointers, keys, text, IME, focus, lifecycle, and the pump
//! through ONE API ([`PreviewSession::dispatch_input`]) and asserts the
//! [`PreviewDispatchOutcome`] plus the runtime effects directly — no
//! dependence on the R9 trace API. The multi-pointer identity acceptance
//! lives in `tests_multi_pointer`; this file covers the REST of the
//! canonical input surface.

#![cfg(test)]

use super::input_event::{AppLifecyclePhase, PreviewInput, PreviewInputEnvelope, PreviewLifecycle};
use super::{test_measure, PreviewSession};
use jian_core::action::services::Router as _;
use jian_core::geometry::point;
use jian_core::gesture::pointer::{Modifiers, PointerKind, PointerPhase};
use jian_core::gesture::PointerEvent;

fn default_theme() -> std::collections::BTreeMap<String, String> {
    std::collections::BTreeMap::new()
}

fn enter_doc(src: &str) -> PreviewSession {
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

/// A Touch pointer event at scene coordinates, through the canonical
/// envelope.
fn touch(id: u32, phase: PointerPhase, x: f32, y: f32, t: u64) -> PreviewInputEnvelope {
    let mut ev = PointerEvent::simple_at(id, phase, point(x, y), t);
    ev.kind = PointerKind::Touch;
    PreviewInputEnvelope::new(PreviewInput::Pointer(ev))
}

fn counter(session: &PreviewSession, key: &str) -> i64 {
    session
        .app_state_value_for_test(key)
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

/// A transform surface (two pointers claim Scale) plus an `onKey` target
/// editable — the mixed-input trace fixture.
fn trace_doc() -> &'static str {
    r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "state": {
            "ss": { "type": "int", "default": 0 },
            "keys": { "type": "int", "default": 0 }
        },
        "children": [
            { "type": "frame", "id": "screen", "width": 400, "height": 400,
              "events": {
                "onScaleStart": [ { "set": { "$app.ss": "$app.ss + 1" } } ]
              },
              "children": [
                  { "type": "rectangle", "id": "stage", "x": 40, "y": 40,
                    "width": 320, "height": 320 },
                  { "type": "text_input", "id": "field", "x": 60, "y": 350,
                    "width": 200, "height": 32,
                    "events": { "onKey": [ { "set": { "$app.keys": "$app.keys + 1" } } ] } }
              ] }
        ]
    }"##
}

/// Two real pointer ids through ONE `dispatch_input` API claim Scale —
/// the canonical-entry mirror of the multi-pointer acceptance.
#[test]
fn canonical_pointer_input_claims_scale_with_two_ids() {
    let mut s = enter_doc(trace_doc());
    let _ = s.dispatch_input(touch(1, PointerPhase::Down, 100.0, 100.0, 0));
    let _ = s.dispatch_input(touch(2, PointerPhase::Down, 300.0, 300.0, 10));
    // The spread past the 5% threshold claims on the claiming Move: the
    // first spread Move can surface ScaleStart, the second ScaleUpdate
    // (or the co-winning Rotate family) — the claim itself is proven by
    // the action counter.
    let spread1 = s.dispatch_input(touch(1, PointerPhase::Move, 90.0, 90.0, 20));
    let spread2 = s.dispatch_input(touch(2, PointerPhase::Move, 310.0, 310.0, 30));
    let claimed = spread1.semantic_handlers.contains(&"onScaleStart")
        || spread2.semantic_handlers.contains(&"onScaleStart")
        || counter(&s, "ss") > 0;
    assert!(
        claimed,
        "the scale claim surfaced through the canonical entry"
    );
    assert_eq!(counter(&s, "ss"), 1, "exactly one ScaleStart action ran");
}

/// Text + IME commit land on the focused editable through the canonical
/// entry, exactly like the platform IME path: preedit composes over the
/// field, the commit replaces the composition, and the snapshot reads
/// the committed text.
#[test]
fn canonical_ime_commits_into_the_focused_editable() {
    let mut s = enter_doc(trace_doc());
    // Tap the field to focus it (widget activation on Tap).
    let _ = s.dispatch_input(touch(1, PointerPhase::Down, 160.0, 366.0, 0));
    let _ = s.dispatch_input(touch(1, PointerPhase::Up, 160.0, 366.0, 20));
    let preedit = s.dispatch_input(PreviewInputEnvelope::new(PreviewInput::ImePreedit {
        text: "ni".into(),
        selection: 2..2,
    }));
    assert!(
        preedit.needs_redraw,
        "the focused editable composed the preedit"
    );
    let commit = s.dispatch_input(PreviewInputEnvelope::new(PreviewInput::ImeCommit {
        text: "你".into(),
    }));
    assert!(commit.needs_redraw, "the commit landed");
    let snapshot = s
        .runtime_mut()
        .focused_editable_snapshot()
        .expect("the tapped field stays focused");
    assert_eq!(snapshot.text, "你", "committed text replaced the preedit");
}

/// Keys route to the focused editable's `onKey` handler; `Text` input is
/// consumed by the focused editable.
#[test]
fn canonical_key_and_text_reach_the_focused_editable() {
    let mut s = enter_doc(trace_doc());
    let _ = s.dispatch_input(touch(1, PointerPhase::Down, 160.0, 366.0, 0));
    let _ = s.dispatch_input(touch(1, PointerPhase::Up, 160.0, 366.0, 20));
    let typed = s.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Text("hi".into())));
    assert!(typed.needs_redraw, "focused editable consumed the text");
    let key = s.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Key {
        key: "Enter".into(),
        code: "Enter".into(),
        repeat: false,
        modifiers: Modifiers::empty(),
    }));
    assert!(
        key.semantic_handlers.contains(&"onKey"),
        "unconsumed key reaches the authored onKey handler"
    );
    assert_eq!(counter(&s, "keys"), 1);
}

/// App lifecycle phases dispatch the authored hooks through
/// `dispatch_input` — launch/resume/background/terminate each exactly
/// once per dispatch.
#[test]
fn canonical_app_lifecycle_dispatches_authored_hooks() {
    let mut s = enter_doc(
        r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "lifecycle": {
            "onLaunch":     [ { "set": { "$app.l": "$app.l + 1" } } ],
            "onResume":     [ { "set": { "$app.r": "$app.r + 1" } } ],
            "onBackground": [ { "set": { "$app.b": "$app.b + 1" } } ],
            "onTerminate":  [ { "set": { "$app.t": "$app.t + 1" } } ]
        },
        "state": {
            "l": { "type": "int", "default": 0 },
            "r": { "type": "int", "default": 0 },
            "b": { "type": "int", "default": 0 },
            "t": { "type": "int", "default": 0 }
        },
        "children": [
            { "type": "frame", "id": "screen", "width": 100, "height": 100 }
        ]
    }"##,
    );
    for (phase, key) in [
        (AppLifecyclePhase::Launch, "l"),
        (AppLifecyclePhase::Resume, "r"),
        (AppLifecyclePhase::Background, "b"),
        (AppLifecyclePhase::Terminate, "t"),
    ] {
        let outcome = s.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Lifecycle(
            PreviewLifecycle::App(phase),
        )));
        assert!(outcome.needs_redraw, "{key} hook spawned");
        assert_eq!(counter(&s, key), 1, "{key} ran exactly once");
    }
}

/// A lone Tap into a Tap+DoubleTap node delays until the double-tap
/// window; `next_wake_deadline_ms` reports the deadline and ONE pump
/// flushes the delayed `onTap` exactly once, with no further input.
#[test]
fn pump_flushes_the_delayed_tap_at_the_reported_deadline() {
    let mut s = enter_doc(
        r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "state": {
            "taps": { "type": "int", "default": 0 },
            "doubles": { "type": "int", "default": 0 }
        },
        "children": [
            { "type": "frame", "id": "btn", "x": 0, "y": 0, "width": 200, "height": 200,
              "gestures": { "doubleTapTimeout": 120 },
              "events": {
                "onTap":        [ { "set": { "$app.taps": "$app.taps + 1" } } ],
                "onDoubleTap":  [ { "set": { "$app.doubles": "$app.doubles + 1" } } ]
              } }
        ]
    }"##,
    );
    // A lone tap: Down + Up, nothing else.
    let _ = s.dispatch_input(touch(1, PointerPhase::Down, 100.0, 100.0, 0));
    let _ = s.dispatch_input(touch(1, PointerPhase::Up, 100.0, 100.0, 10));
    assert_eq!(counter(&s, "taps"), 0, "first tap is buffered");
    let deadline = s
        .next_wake_deadline_ms()
        .expect("the buffered tap arms a wake deadline");
    assert!(deadline >= 120, "deadline is the double-tap window end");
    // Pumping PAST the deadline with NO further input flushes it once.
    let _ = s.pump(deadline + 1);
    assert_eq!(counter(&s, "taps"), 1, "delayed tap fired at the deadline");
    let _ = s.pump(deadline + 50);
    assert_eq!(counter(&s, "taps"), 1, "exactly once");
    // A matching second tap inside the window yields ONLY DoubleTap.
    let _ = s.dispatch_input(touch(1, PointerPhase::Down, 100.0, 100.0, 200));
    let _ = s.dispatch_input(touch(1, PointerPhase::Up, 100.0, 100.0, 210));
    let _ = s.dispatch_input(touch(1, PointerPhase::Down, 100.0, 100.0, 220));
    let up = s.dispatch_input(touch(1, PointerPhase::Up, 100.0, 100.0, 230));
    assert!(up.semantic_handlers.contains(&"onDoubleTap"));
    assert_eq!(counter(&s, "doubles"), 1);
    assert_eq!(counter(&s, "taps"), 1, "second tap never runs onTap");
}

/// `cancel_input_ownership` clears every capture anchor and settles the
/// cancelled pointers' arena state — the teardown-barrier primitive.
#[test]
fn cancel_input_ownership_releases_every_pointer() {
    let mut s = enter_doc(trace_doc());
    let _ = s.dispatch_input(touch(1, PointerPhase::Down, 100.0, 100.0, 0));
    let _ = s.dispatch_input(touch(2, PointerPhase::Down, 300.0, 300.0, 10));
    assert_eq!(s.anchored_pointer_ids_for_test(), vec![1, 2]);
    s.cancel_input_ownership("test-background");
    assert!(
        s.anchored_pointer_ids_for_test().is_empty(),
        "every anchor released by the ownership cancel"
    );
    assert!(
        s.interaction().pressed_nodes().is_empty(),
        "no pressed node survives the ownership cancel"
    );
}
/// Screen-switch reconciliation dispatches the lifecycle hooks in the
/// deterministic leave → unmount → mount → enter order: the outgoing
/// screen's child unmounts, the incoming screen's child mounts (page
/// lifecycle has no producer through screen projection — its synthetic
/// pages carry no hooks — so the node scope carries the assertion).
#[test]
fn reconcile_dispatches_mount_unmount_around_a_screen_switch() {
    let src = r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "state": {
            "m": { "type": "int", "default": 0 },
            "u": { "type": "int", "default": 0 }
        },
        "pages": [
            { "id": "canvas", "name": "Canvas", "children": [
                { "type": "frame", "id": "home", "screen": "/",
                  "x": 0, "y": 0, "width": 200, "height": 200,
                  "children": [
                      { "type": "frame", "id": "home-body", "x": 10, "y": 10, "width": 100, "height": 100,
                        "lifecycle": { "onUnmount": [ { "set": { "$app.u": "$app.u + 1" } } ] } }
                  ] },
                { "type": "frame", "id": "detail", "screen": "/detail",
                  "x": 500, "y": 0, "width": 200, "height": 200,
                  "children": [
                      { "type": "frame", "id": "detail-body", "x": 10, "y": 10, "width": 100, "height": 100,
                        "lifecycle": { "onMount": [ { "set": { "$app.m": "$app.m + 1" } } ] } }
                  ] }
            ] }
        ]
    }"##;
    let doc = jian_ops_schema::load_str(src)
        .expect("parse routed doc")
        .value;
    let mut s = PreviewSession::enter(
        &doc,
        (1200.0, 800.0),
        &default_theme(),
        0,
        false,
        false,
        test_measure(),
    )
    .expect("enter app mode");
    assert_eq!(
        counter(&s, "m"),
        0,
        "entry screen mounts before any reconcile"
    );
    // Push a route, then reconcile: the switch fires unmount + mount.
    s.router_for_test().push("/detail");
    let outcome = s.reconcile(1000);
    assert!(outcome.switched, "the route switch landed");
    let _ = s.pump(1001);
    assert_eq!(counter(&s, "u"), 1, "outgoing screen child unmounted");
    assert_eq!(counter(&s, "m"), 1, "incoming screen child mounted");
}
