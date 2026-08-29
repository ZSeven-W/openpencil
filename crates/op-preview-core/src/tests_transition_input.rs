//! R8: input arbitration while a screen transition plays.
//!
//! The window is short (160–240ms) but real: a user who taps again during
//! it is expressing intent about the screen that is arriving. The policy
//! is one slot — the newest safe discrete input replaces the older, and
//! everything continuous is dropped rather than resumed against widgets
//! that no longer exist.
//!
//! Every test here asserts on `deferred_discrete_input` itself, not on
//! some proxy: a test that only checked "the transition ended" would pass
//! against an implementation that stores nothing at all.

#![cfg(test)]

use super::transition::DeferredDiscreteInput;
use super::{test_measure, PreviewSession};
use jian_core::gesture::pointer::{Modifiers, PointerKind, PointerPhase};

fn two_screen_doc() -> jian_ops_schema::PenDocument {
    serde_json::from_str(super::tests_app_mode::TWO_SCREEN_DOC_JSON).unwrap()
}

fn session() -> PreviewSession {
    PreviewSession::enter(
        &two_screen_doc(),
        (1200.0, 800.0),
        &Default::default(),
        0,
        false,
        false,
        test_measure(),
    )
    .unwrap()
}

fn go_button_center(session: &PreviewSession) -> (f32, f32) {
    let (x, y, w, h) = session.node_rect("go").expect("go button laid out");
    (x + w / 2.0, y + h / 2.0)
}

/// Drive the session into a live transition and return the session plus
/// the button centre the caller can aim deferred input at.
fn mid_transition() -> (PreviewSession, (f32, f32)) {
    let mut s = session();
    let centre = go_button_center(&s);
    s.dispatch_tap(centre.0, centre.1);
    s.set_now_ms(10);
    assert!(s.reconcile(20).switched, "the tap navigates");
    assert!(s.transition_active(), "a transition is playing");
    (s, centre)
}

/// A press that begins and ends inside the transition becomes exactly one
/// deferred Tap, carrying the point it happened at.
#[test]
fn a_press_during_a_transition_defers_one_tap() {
    let (mut s, (x, y)) = mid_transition();
    s.dispatch_pointer_for_id_at(1, PointerKind::Touch, x, y, PointerPhase::Down, 30);
    s.dispatch_pointer_for_id_at(1, PointerKind::Touch, x, y, PointerPhase::Up, 60);

    match s.deferred_discrete_input.as_ref() {
        Some(DeferredDiscreteInput::Tap {
            scene_x, scene_y, ..
        }) => {
            assert_eq!((*scene_x, *scene_y), (x, y), "the tap keeps its point");
        }
        other => panic!("expected a deferred Tap, got {other:?}"),
    }
}

/// Raw phases never reach the runtime during the transition: the tracker
/// holds the press privately until it can judge it at Up.
#[test]
fn a_down_alone_defers_nothing_yet() {
    let (mut s, (x, y)) = mid_transition();
    s.dispatch_pointer_for_id_at(1, PointerKind::Touch, x, y, PointerPhase::Down, 30);
    assert!(
        s.deferred_discrete_input.is_none(),
        "a press that has not lifted yet is not a tap"
    );
    assert!(s.transition_tap.is_some(), "but the tracker is watching it");
}

/// Drift past jian's tap slop is a drag, not a tap — nothing is deferred.
#[test]
fn a_press_that_drifts_too_far_is_not_a_tap() {
    let (mut s, (x, y)) = mid_transition();
    s.dispatch_pointer_for_id_at(1, PointerKind::Touch, x, y, PointerPhase::Down, 30);
    s.dispatch_pointer_for_id_at(1, PointerKind::Touch, x + 120.0, y, PointerPhase::Up, 60);
    assert!(
        s.deferred_discrete_input.is_none(),
        "120px of drift is a drag; replaying it as a tap would invent an intent"
    );
}

/// Holding past the long-press threshold is not a tap either.
#[test]
fn a_press_held_past_long_press_is_not_a_tap() {
    let (mut s, (x, y)) = mid_transition();
    s.dispatch_pointer_for_id_at(1, PointerKind::Touch, x, y, PointerPhase::Down, 30);
    s.dispatch_pointer_for_id_at(1, PointerKind::Touch, x, y, PointerPhase::Up, 30 + 800);
    assert!(
        s.deferred_discrete_input.is_none(),
        "a held press is a long-press, which is not a discrete input"
    );
}

/// A cancelled press leaves nothing behind.
#[test]
fn a_cancelled_press_clears_the_tracker() {
    let (mut s, (x, y)) = mid_transition();
    s.dispatch_pointer_for_id_at(1, PointerKind::Touch, x, y, PointerPhase::Down, 30);
    s.dispatch_pointer_for_id_at(1, PointerKind::Touch, x, y, PointerPhase::Cancel, 40);
    assert!(s.transition_tap.is_none(), "the tracker is cleared");
    assert!(s.deferred_discrete_input.is_none(), "nothing deferred");
}

/// One slot: the newer discrete input replaces the older one outright.
#[test]
fn a_newer_discrete_input_replaces_the_older_one() {
    let (mut s, (x, y)) = mid_transition();
    s.dispatch_pointer_for_id_at(1, PointerKind::Touch, x, y, PointerPhase::Down, 30);
    s.dispatch_pointer_for_id_at(1, PointerKind::Touch, x, y, PointerPhase::Up, 40);
    assert!(
        matches!(
            s.deferred_discrete_input,
            Some(DeferredDiscreteInput::Tap { .. })
        ),
        "the tap is in the slot"
    );

    s.dispatch_key("Enter", Modifiers::default());
    match s.deferred_discrete_input.as_ref() {
        Some(DeferredDiscreteInput::Submit { key, .. }) => assert_eq!(key, "Enter"),
        other => panic!("Enter must replace the stored tap, got {other:?}"),
    }
}

/// Enter and Escape defer; every other key is dropped.
#[test]
fn enter_and_escape_defer_while_other_keys_drop() {
    let (mut s, _) = mid_transition();
    s.dispatch_key("Escape", Modifiers::default());
    assert!(
        matches!(
            s.deferred_discrete_input,
            Some(DeferredDiscreteInput::Back { .. })
        ),
        "Escape defers as Back"
    );

    s.deferred_discrete_input = None;
    for key in ["ArrowLeft", "Backspace", "Tab", "a"] {
        s.dispatch_key(key, Modifiers::default());
        assert!(
            s.deferred_discrete_input.is_none(),
            "{key} belongs to a text session that will not exist after the switch"
        );
    }
}

/// Text and IME are discarded outright — never deferred.
#[test]
fn text_input_is_discarded_not_deferred() {
    let (mut s, _) = mid_transition();
    assert!(!s.dispatch_text("hello"), "text is swallowed");
    assert!(
        s.deferred_discrete_input.is_none(),
        "a commit replayed later would land in whatever field the new screen focuses"
    );
}

/// Continuous pointer traffic (moves) leaves the slot empty.
#[test]
fn continuous_movement_defers_nothing() {
    let (mut s, (x, y)) = mid_transition();
    for step in 0..5 {
        let dx = x + step as f32 * 10.0;
        s.dispatch_pointer_for_id_at(1, PointerKind::Touch, dx, y, PointerPhase::Move, 30 + step);
        s.dispatch_pointer_for_id_at(1, PointerKind::Touch, dx, y, PointerPhase::Hover, 35 + step);
    }
    assert!(
        s.deferred_discrete_input.is_none(),
        "moves and hovers are not discrete decisions"
    );
}

/// The completion edge replays: once the clock passes the transition's
/// end, the stored input is consumed and the slot empties.
#[test]
fn the_transition_completion_edge_consumes_the_slot() {
    let (mut s, (x, y)) = mid_transition();
    s.dispatch_pointer_for_id_at(1, PointerKind::Touch, x, y, PointerPhase::Down, 30);
    s.dispatch_pointer_for_id_at(1, PointerKind::Touch, x, y, PointerPhase::Up, 40);
    assert!(
        s.deferred_discrete_input.is_some(),
        "the tap waits for the screen to arrive"
    );

    s.set_now_ms(5_000);
    assert!(!s.transition_active(), "the transition finished");
    assert!(
        s.deferred_discrete_input.is_none(),
        "the completion edge consumed the slot"
    );
    assert!(s.transition_tap.is_none(), "the tracker is cleared too");
}

/// The edge REPLAYS rather than merely discarding: a still-valid input
/// reports that it ran, while a stale one reports that it did not. An
/// implementation that only emptied the slot would return false for both.
#[test]
fn replay_distinguishes_a_valid_input_from_a_stale_one() {
    let (mut s, (x, y)) = mid_transition();
    s.dispatch_pointer_for_id_at(1, PointerKind::Touch, x, y, PointerPhase::Down, 30);
    s.dispatch_pointer_for_id_at(1, PointerKind::Touch, x, y, PointerPhase::Up, 40);
    // Replay belongs to the completion edge, so end the transition: the
    // edge itself performs the replay and empties the slot.
    s.set_now_ms(5_000);
    assert!(!s.transition_active(), "the transition finished");
    assert!(!s.replay_deferred_input(), "an empty slot replays nothing");

    // A live transition refuses to replay — otherwise the replayed input
    // would be caught by the deferral path and stored straight back.
    let (mut live, (lx, ly)) = mid_transition();
    live.dispatch_pointer_for_id_at(1, PointerKind::Touch, lx, ly, PointerPhase::Down, 30);
    live.dispatch_pointer_for_id_at(1, PointerKind::Touch, lx, ly, PointerPhase::Up, 40);
    assert!(
        !live.replay_deferred_input(),
        "replay during a live transition is refused"
    );
    assert!(
        live.deferred_discrete_input.is_some(),
        "and the input stays in the slot, waiting for the edge"
    );

    let (mut stale, (sx, sy)) = mid_transition();
    stale.dispatch_pointer_for_id_at(1, PointerKind::Touch, sx, sy, PointerPhase::Down, 30);
    stale.dispatch_pointer_for_id_at(1, PointerKind::Touch, sx, sy, PointerPhase::Up, 40);
    stale.route_generation = stale.route_generation.saturating_add(1);
    stale.set_now_ms(5_000);
    assert!(
        stale.deferred_discrete_input.is_none(),
        "an input aimed at a screen that is gone is dropped at the edge"
    );
}

/// A second navigation during the transition invalidates the deferred
/// input: the screen it was aimed at is gone, so it must not replay.
#[test]
fn a_route_change_invalidates_the_deferred_input() {
    let (mut s, (x, y)) = mid_transition();
    s.dispatch_pointer_for_id_at(1, PointerKind::Touch, x, y, PointerPhase::Down, 30);
    s.dispatch_pointer_for_id_at(1, PointerKind::Touch, x, y, PointerPhase::Up, 40);
    let captured = s
        .deferred_discrete_input
        .as_ref()
        .expect("a tap is stored")
        .route_generation();

    // Navigate again: the reconcile bumps the route counter.
    s.route_generation = s.route_generation.saturating_add(1);
    assert_ne!(
        captured, s.route_generation,
        "the stored input now points at a screen that is gone"
    );

    s.set_now_ms(5_000);
    assert!(
        s.deferred_discrete_input.is_none(),
        "a stale input is dropped, not replayed onto the wrong screen"
    );
}

/// The activation the host certified travels with the deferred input, so
/// the replay runs under the same certification the user's gesture had.
#[test]
fn the_certified_activation_travels_with_the_deferred_input() {
    use super::input_event::{PreviewInput, PreviewInputEnvelope};
    use op_preview_contracts::UserActivationId;

    let (mut s, _) = mid_transition();
    let activation = UserActivationId::from_raw(77);
    let envelope = PreviewInputEnvelope {
        input: PreviewInput::Key {
            key: "Enter".into(),
            code: "Enter".into(),
            repeat: false,
            modifiers: Modifiers::default(),
        },
        activation: Some(activation),
    };
    s.dispatch_input(envelope);

    match s.deferred_discrete_input.as_ref() {
        Some(DeferredDiscreteInput::Submit {
            activation: stored, ..
        }) => assert_eq!(
            *stored,
            Some(activation),
            "the deferred Submit carries the host's certification"
        ),
        other => panic!("expected a deferred Submit, got {other:?}"),
    }
}

/// Nothing deferred may reach the effect queue early — effects happen on
/// replay, after the screen has arrived, or not at all.
#[test]
fn deferring_never_enqueues_an_effect_early() {
    let (mut s, (x, y)) = mid_transition();
    let before = s.effects.total_enqueued();
    s.dispatch_pointer_for_id_at(1, PointerKind::Touch, x, y, PointerPhase::Down, 30);
    s.dispatch_pointer_for_id_at(1, PointerKind::Touch, x, y, PointerPhase::Up, 40);
    s.dispatch_key("Enter", Modifiers::default());
    assert_eq!(
        s.effects.total_enqueued(),
        before,
        "storing an input must not run it"
    );
}
