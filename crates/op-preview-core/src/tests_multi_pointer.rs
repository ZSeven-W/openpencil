//! R4 Canonical PreviewInput — multi-pointer identity through the product
//! preview path.
//!
//! Before R4 the preview session synthesized a single `id=1` Mouse stream,
//! so Scale/Rotate could never claim through the product preview panel and
//! two concurrent pointers shared ONE capture anchor. These tests drive
//! real two-finger streams through
//! [`PreviewSession::dispatch_pointer_for_id_at`] and assert the
//! engine-side transform families actually fire, plus the bookkeeping
//! seams: per-id anchor lifetime, teardown [`PreviewSession::cancel_pointer`],
//! and the legacy synthetic-id wrappers staying compatible.

#![cfg(test)]

use super::{test_measure, PreviewSession};
use jian_core::gesture::pointer::{PointerKind, PointerPhase};

fn transform_doc() -> jian_ops_schema::PenDocument {
    let src = r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "state": {
            "ss": { "type": "int", "default": 0 },
            "se": { "type": "int", "default": 0 },
            "rs": { "type": "int", "default": 0 },
            "re": { "type": "int", "default": 0 },
            "taps": { "type": "int", "default": 0 }
        },
        "children": [
            { "type": "frame", "id": "screen", "width": 400, "height": 400,
              "events": {
                "onScaleStart":  [ { "set": { "$app.ss": "$app.ss + 1" } } ],
                "onScaleEnd":    [ { "set": { "$app.se": "$app.se + 1" } } ],
                "onRotateStart": [ { "set": { "$app.rs": "$app.rs + 1" } } ],
                "onRotateEnd":   [ { "set": { "$app.re": "$app.re + 1" } } ],
                "onTap":         [ { "set": { "$app.taps": "$app.taps + 1" } } ]
              },
              "children": [
                  { "type": "rectangle", "id": "stage", "x": 40, "y": 40,
                    "width": 320, "height": 320 }
              ] }
        ]
    }"##;
    jian_ops_schema::load_str(src)
        .expect("parse transform doc")
        .value
}

fn default_theme() -> std::collections::BTreeMap<String, String> {
    std::collections::BTreeMap::new()
}

fn enter() -> PreviewSession {
    PreviewSession::enter(
        &transform_doc(),
        (800.0, 600.0),
        &default_theme(),
        0,
        false,
        false,
        test_measure(),
    )
    .expect("enter preview")
}

fn counter(session: &PreviewSession, key: &str) -> i64 {
    session
        .app_state_value_for_test(key)
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

/// The R4 acceptance microcosm: two pointers dispatched under their own
/// ids cross BOTH transform thresholds against each other — something the
/// former synthetic-single-id path can never produce.
#[test]
fn two_pointer_ids_claim_scale_and_rotate_through_one_session() {
    let mut s = enter();
    const TOUCH: PointerKind = PointerKind::Touch;
    // Two fingers land wide apart on the stage (scene == runtime space).
    let _ = s.dispatch_pointer_for_id_at(1, TOUCH, 100.0, 100.0, PointerPhase::Down, 0);
    let _ = s.dispatch_pointer_for_id_at(2, TOUCH, 300.0, 300.0, PointerPhase::Down, 10);
    assert_eq!(
        s.anchored_pointer_ids_for_test(),
        vec![1, 2],
        "each finger owns its capture anchor"
    );

    // Spread apart past 5%: distance 282.8 -> 311.1. Scale claims.
    let spread1 = s.dispatch_pointer_for_id_at(1, TOUCH, 90.0, 90.0, PointerPhase::Move, 20);
    let spread2 = s.dispatch_pointer_for_id_at(2, TOUCH, 310.0, 310.0, PointerPhase::Move, 30);
    assert!(
        spread1 || spread2 || counter(&s, "ss") > 0,
        "scale claim surfaced"
    );
    assert_eq!(counter(&s, "ss"), 1, "ScaleStart fired exactly once");

    // Twist around the midpoint (~45 deg -> ~30 deg): Rotate claims too —
    // the co-win that requires TWO distinct live pointer streams.
    let _ = s.dispatch_pointer_for_id_at(1, TOUCH, 130.0, 230.0, PointerPhase::Move, 40);
    let twist2 = s.dispatch_pointer_for_id_at(2, TOUCH, 270.0, 170.0, PointerPhase::Move, 50);
    assert!(twist2 || counter(&s, "rs") > 0, "rotate claim surfaced");
    assert_eq!(counter(&s, "rs"), 1, "RotateStart fired exactly once");

    // Symmetric teardown: settle exactly one End per family overall.
    let _ = s.cancel_pointer(2, 60);
    let _ = s.dispatch_pointer_for_id_at(1, TOUCH, 130.0, 230.0, PointerPhase::Up, 70);
    assert_eq!(counter(&s, "se"), 1, "one ScaleEnd");
    assert_eq!(counter(&s, "re"), 1, "one RotateEnd");
    assert!(
        s.anchored_pointer_ids_for_test().is_empty(),
        "every anchor released"
    );
}

/// Anchor hygiene across interleaved lifecycles: a Cancel frees only its
/// own pointer's anchor, a lone Move without a Down stores nothing, and
/// the next pairing anchors fresh ids.
#[test]
fn cancel_and_stray_moves_manage_anchors_per_pointer() {
    let mut s = enter();
    const TOUCH: PointerKind = PointerKind::Touch;
    let _ = s.dispatch_pointer_for_id_at(1, TOUCH, 100.0, 100.0, PointerPhase::Down, 0);
    let _ = s.dispatch_pointer_for_id_at(2, TOUCH, 260.0, 260.0, PointerPhase::Down, 10);
    assert_eq!(s.anchored_pointer_ids_for_test(), vec![1, 2]);

    // Cancelling pointer 2 must not disturb pointer 1's anchor. The
    // return is false here because this fixture declares no press
    // handlers (nothing to emit) — consumption still settles pointer 2's
    // arena timers and releases its anchor.
    let _ = s.cancel_pointer(2, 20);
    assert_eq!(s.anchored_pointer_ids_for_test(), vec![1]);
    // A stray Move for an unknown pointer resolves but stores nothing.
    let _ = s.dispatch_pointer_for_id_at(9, TOUCH, 50.0, 50.0, PointerPhase::Move, 30);
    assert_eq!(s.anchored_pointer_ids_for_test(), vec![1]);

    // Pointer 1 lifts cleanly; the session is ready for a fresh pairing.
    let _ = s.dispatch_pointer_for_id_at(1, TOUCH, 110.0, 110.0, PointerPhase::Up, 40);
    assert!(s.anchored_pointer_ids_for_test().is_empty());
    let _ = s.dispatch_pointer_for_id_at(3, TOUCH, 80.0, 80.0, PointerPhase::Down, 50);
    let _ = s.dispatch_pointer_for_id_at(4, TOUCH, 240.0, 240.0, PointerPhase::Down, 60);
    assert_eq!(s.anchored_pointer_ids_for_test(), vec![3, 4]);
}

/// Legacy compatibility pin: the synthetic id-1 Mouse wrappers keep
/// working unchanged (the `dispatch_tap` route), intermixed with
/// explicit-id traffic in the SAME session.
#[test]
fn legacy_wrappers_still_dispatch_the_synthetic_mouse_stream() {
    let mut s = enter();
    const TOUCH: PointerKind = PointerKind::Touch;
    let down = s.dispatch_pointer_phase_at(160.0, 160.0, PointerPhase::Down, 0);
    let up = s.dispatch_pointer_phase_at(160.0, 160.0, PointerPhase::Up, 20);
    assert!(down || up, "legacy tap consumed");
    assert_eq!(counter(&s, "taps"), 1);

    // Explicit ids coexist with the legacy stream afterwards — their own
    // completed tap also bubbles to the frame handler.
    let _ = s.dispatch_pointer_for_id_at(2, TOUCH, 260.0, 260.0, PointerPhase::Down, 40);
    let _ = s.dispatch_pointer_for_id_at(2, TOUCH, 260.0, 260.0, PointerPhase::Up, 50);
    assert_eq!(counter(&s, "taps"), 2, "explicit-id tap delivered");
    // And another legacy tap keeps counting for the same stream.
    let _ = s.dispatch_pointer_phase_at(160.0, 160.0, PointerPhase::Down, 60);
    let _ = s.dispatch_pointer_phase_at(160.0, 160.0, PointerPhase::Up, 70);
    assert_eq!(counter(&s, "taps"), 3, "second legacy tap delivered");
}

/// R2B2 parity: a doc whose transform handlers append digits to a
/// sequence code, so ORDER is assertable through app state alone —
/// ScaleStart pushes 1, RotateStart pushes 2, and a fixed evaluation
/// order must read 12, never 21. Mirrors the engine-side
/// `scale_and_rotate_cowin_in_fixed_scale_rotate_order`.
fn sequence_doc() -> jian_ops_schema::PenDocument {
    let src = r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "state": { "seq": { "type": "int", "default": 0 } },
        "children": [
            { "type": "frame", "id": "screen", "width": 400, "height": 400,
              "events": {
                "onScaleStart":  [ { "set": { "$app.seq": "$app.seq * 10 + 1" } } ],
                "onRotateStart": [ { "set": { "$app.seq": "$app.seq * 10 + 2" } } ]
              },
              "children": [
                  { "type": "rectangle", "id": "stage", "x": 40, "y": 40,
                    "width": 320, "height": 320 }
              ] }
        ]
    }"##;
    jian_ops_schema::load_str(src)
        .expect("parse sequence doc")
        .value
}

/// One movement crossing BOTH thresholds at once must still deliver
/// ScaleStart before RotateStart — the deterministic order authors can
/// rely on regardless of which threshold the geometry crossed "first".
#[test]
fn cowin_delivers_scale_before_rotate_in_one_burst() {
    let mut s = PreviewSession::enter(
        &sequence_doc(),
        (800.0, 600.0),
        &default_theme(),
        0,
        false,
        false,
        test_measure(),
    )
    .expect("enter preview");
    const TOUCH: PointerKind = PointerKind::Touch;
    let _ = s.dispatch_pointer_for_id_at(1, TOUCH, 100.0, 100.0, PointerPhase::Down, 0);
    let _ = s.dispatch_pointer_for_id_at(2, TOUCH, 300.0, 300.0, PointerPhase::Down, 10);
    // One burst: finger 1 both spreads AND twists past the thresholds,
    // finger 2 mirrors it. Both families qualify inside the same slice.
    let _ = s.dispatch_pointer_for_id_at(1, TOUCH, 60.0, 220.0, PointerPhase::Move, 20);
    let _ = s.dispatch_pointer_for_id_at(2, TOUCH, 340.0, 180.0, PointerPhase::Move, 30);
    assert_eq!(
        counter(&s, "seq"),
        12,
        "ScaleStart (1) must precede RotateStart (2); 21 means the order inverted"
    );
    let _ = s.dispatch_pointer_for_id_at(1, TOUCH, 60.0, 220.0, PointerPhase::Up, 40);
    let _ = s.dispatch_pointer_for_id_at(2, TOUCH, 340.0, 180.0, PointerPhase::Up, 50);
}

/// R2B2 parity with the engine's 2→1→2 regrab: lifting one finger closes
/// the transform session (one End per family), and a fresh second finger
/// opens a NEW session with a new baseline — a second full pair of
/// Starts, never a resume of the old one.
#[test]
fn two_one_two_regrab_opens_a_fresh_session_through_the_client() {
    let mut s = enter();
    const TOUCH: PointerKind = PointerKind::Touch;
    let _ = s.dispatch_pointer_for_id_at(1, TOUCH, 100.0, 100.0, PointerPhase::Down, 0);
    let _ = s.dispatch_pointer_for_id_at(2, TOUCH, 300.0, 300.0, PointerPhase::Down, 10);
    let _ = s.dispatch_pointer_for_id_at(1, TOUCH, 90.0, 90.0, PointerPhase::Move, 20);
    let _ = s.dispatch_pointer_for_id_at(2, TOUCH, 310.0, 310.0, PointerPhase::Move, 30);
    let _ = s.dispatch_pointer_for_id_at(1, TOUCH, 130.0, 230.0, PointerPhase::Move, 40);
    let _ = s.dispatch_pointer_for_id_at(2, TOUCH, 270.0, 170.0, PointerPhase::Move, 50);
    assert_eq!(
        (counter(&s, "ss"), counter(&s, "rs")),
        (1, 1),
        "first session co-won"
    );

    // 2 → 1: the second finger lifts; the session ends symmetrically.
    let _ = s.dispatch_pointer_for_id_at(2, TOUCH, 270.0, 170.0, PointerPhase::Up, 60);
    assert_eq!(
        (counter(&s, "se"), counter(&s, "re")),
        (1, 1),
        "one End per family"
    );

    // 1 → 2: a fresh finger lands; the NEW baseline is the current
    // geometry, and crossing the thresholds again opens a second session.
    let _ = s.dispatch_pointer_for_id_at(3, TOUCH, 300.0, 300.0, PointerPhase::Down, 70);
    let _ = s.dispatch_pointer_for_id_at(1, TOUCH, 100.0, 200.0, PointerPhase::Move, 80);
    let _ = s.dispatch_pointer_for_id_at(3, TOUCH, 330.0, 330.0, PointerPhase::Move, 90);
    let _ = s.dispatch_pointer_for_id_at(1, TOUCH, 160.0, 280.0, PointerPhase::Move, 100);
    let _ = s.dispatch_pointer_for_id_at(3, TOUCH, 280.0, 250.0, PointerPhase::Move, 110);
    assert_eq!(
        counter(&s, "ss"),
        2,
        "the regrab opened a fresh Scale session"
    );
    assert_eq!(counter(&s, "rs"), 2, "and a fresh Rotate session");

    let _ = s.dispatch_pointer_for_id_at(1, TOUCH, 160.0, 280.0, PointerPhase::Up, 120);
    let _ = s.dispatch_pointer_for_id_at(3, TOUCH, 280.0, 250.0, PointerPhase::Up, 130);
    assert_eq!(
        (counter(&s, "se"), counter(&s, "re")),
        (2, 2),
        "both sessions closed"
    );
}

/// R2B2 parity with the engine's third-finger rule: a third pointer
/// landing mid-transform neither restarts the session nor joins the
/// team — the counts stay exactly where the two-finger team put them.
#[test]
fn a_third_finger_stays_out_of_the_transform_team() {
    let mut s = enter();
    const TOUCH: PointerKind = PointerKind::Touch;
    let _ = s.dispatch_pointer_for_id_at(1, TOUCH, 100.0, 100.0, PointerPhase::Down, 0);
    let _ = s.dispatch_pointer_for_id_at(2, TOUCH, 300.0, 300.0, PointerPhase::Down, 10);
    let _ = s.dispatch_pointer_for_id_at(1, TOUCH, 90.0, 90.0, PointerPhase::Move, 20);
    let _ = s.dispatch_pointer_for_id_at(2, TOUCH, 310.0, 310.0, PointerPhase::Move, 30);
    let _ = s.dispatch_pointer_for_id_at(1, TOUCH, 130.0, 230.0, PointerPhase::Move, 40);
    let _ = s.dispatch_pointer_for_id_at(2, TOUCH, 270.0, 170.0, PointerPhase::Move, 50);
    assert_eq!(
        (counter(&s, "ss"), counter(&s, "rs")),
        (1, 1),
        "team of two co-won"
    );

    // The third finger lands and moves hard; the team must not care.
    let _ = s.dispatch_pointer_for_id_at(7, TOUCH, 200.0, 60.0, PointerPhase::Down, 60);
    let _ = s.dispatch_pointer_for_id_at(7, TOUCH, 60.0, 340.0, PointerPhase::Move, 70);
    let _ = s.dispatch_pointer_for_id_at(7, TOUCH, 340.0, 60.0, PointerPhase::Move, 80);
    assert_eq!(
        (
            counter(&s, "ss"),
            counter(&s, "rs"),
            counter(&s, "se"),
            counter(&s, "re")
        ),
        (1, 1, 0, 0),
        "the third finger neither restarted nor ended the transform session"
    );
    assert!(
        s.anchored_pointer_ids_for_test().contains(&7),
        "but it does own its own capture anchor"
    );

    // Its lift is equally irrelevant to the team.
    let _ = s.dispatch_pointer_for_id_at(7, TOUCH, 340.0, 60.0, PointerPhase::Up, 90);
    assert_eq!(
        (counter(&s, "se"), counter(&s, "re")),
        (0, 0),
        "no End from a bystander"
    );

    let _ = s.dispatch_pointer_for_id_at(1, TOUCH, 130.0, 230.0, PointerPhase::Up, 100);
    let _ = s.dispatch_pointer_for_id_at(2, TOUCH, 270.0, 170.0, PointerPhase::Up, 110);
    assert_eq!(
        (counter(&s, "se"), counter(&s, "re")),
        (1, 1),
        "the team settles once"
    );
}
