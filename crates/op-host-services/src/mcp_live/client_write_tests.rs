//! `client_write` session tests — the `now_ms`-driven coverage the
//! conn/pump tests can't reach without burning real seconds on
//! `RELAY_GRACE_MS`: relay-grace retire semantics (served-once + hard
//! cap), the `WhileRelayLive` reuse boundary, the foreign-run guard,
//! and the register-path filters (dead ids, new∧touched precedence,
//! the single cursor anchor).

#![cfg(test)]

use super::*;

/// Land `ids` as authored-id rectangles on the active page — the same
/// JSON-built shape `mcp_live_tests` uses for `batch_design` fixtures.
fn seeded_state(ids: &[&str]) -> EditorState {
    let mut state = EditorState::new();
    for id in ids {
        let node: PenNode = serde_json::from_str(&format!(
            r#"{{"type":"rectangle","id":"{id}","name":"n{id}","x":0,"y":0,"width":10,"height":10}}"#
        ))
        .expect("valid node json");
        assert!(state.apply(EditorCommand::InsertAuthoredSubtree {
            nodes: vec![node],
            parent_id: NodeId::NONE,
            page_id: None,
        }));
    }
    state
}

fn touched(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| id.to_string()).collect()
}

/// The deadline alone must not retire a run the browser never saw:
/// `apply_remote` mirrors `run_active` runs only, so an epoch that
/// expired before its first `active:true` body was served would drop
/// the write's whole presence on a throttled first poll.
#[test]
fn relay_epoch_retires_only_after_an_active_body_was_served() {
    let _g = agent_indicators::test_guard();
    agent_indicators::clear();
    let state = seeded_state(&["n1"]);
    let mut session = ClientWriteIndicators::default();
    let t0 = 10_000;
    let epoch = session
        .register_applied_writes(
            &state,
            None,
            &touched(&["n1"]),
            EpochReuse::WhileRelayLive,
            t0,
        )
        .expect("a landed write registers");
    let deadline = t0 + RELAY_GRACE_MS;

    session.finish_expired_relay(deadline - 1);
    assert_eq!(
        agent_indicators::active_epoch(),
        Some(epoch),
        "inside the grace window: still live"
    );
    session.finish_expired_relay(deadline);
    assert_eq!(
        agent_indicators::active_epoch(),
        Some(epoch),
        "deadline passed but no active body was ever served — the run must outlive the late first poll"
    );

    session.note_relay_served();
    session.finish_expired_relay(deadline);
    assert_eq!(
        agent_indicators::active_epoch(),
        None,
        "once an active body was served, the deadline retires the run"
    );
    agent_indicators::clear();
}

/// A run no browser ever polls cannot park `run_active` forever — the
/// hard cap retires it one extra grace after the deadline.
#[test]
fn a_never_polled_relay_epoch_retires_at_the_hard_cap() {
    let _g = agent_indicators::test_guard();
    agent_indicators::clear();
    let state = seeded_state(&["n1"]);
    let mut session = ClientWriteIndicators::default();
    let t0 = 10_000;
    session
        .register_applied_writes(
            &state,
            None,
            &touched(&["n1"]),
            EpochReuse::WhileRelayLive,
            t0,
        )
        .expect("a landed write registers");

    session.finish_expired_relay(t0 + 2 * RELAY_GRACE_MS + 1);
    assert_eq!(
        agent_indicators::active_epoch(),
        None,
        "past deadline + one extra grace with no serve: the cap retires it"
    );
    agent_indicators::clear();
}

/// `WhileRelayLive` coalesces a burst inside the grace window into one
/// epoch and mints fresh once it retires — the wire-observable version
/// of the desktop pump's drain-window coalescing.
#[test]
fn while_relay_live_reuses_the_epoch_inside_the_grace_window() {
    let _g = agent_indicators::test_guard();
    agent_indicators::clear();
    let state = seeded_state(&["n1"]);
    let mut session = ClientWriteIndicators::default();
    let t0 = 10_000;

    let first = session
        .register_applied_writes(
            &state,
            None,
            &touched(&["n1"]),
            EpochReuse::WhileRelayLive,
            t0,
        )
        .expect("first write");
    session.note_relay_served();
    let second = session
        .register_applied_writes(
            &state,
            None,
            &touched(&["n1"]),
            EpochReuse::WhileRelayLive,
            t0 + 100,
        )
        .expect("second write");
    assert_eq!(second, first, "a write inside the grace reuses the run");

    session.finish_expired_relay(t0 + 100 + RELAY_GRACE_MS);
    assert_eq!(agent_indicators::active_epoch(), None, "served + due");

    let third = session
        .register_applied_writes(
            &state,
            None,
            &touched(&["n1"]),
            EpochReuse::WhileRelayLive,
            t0 + 100 + RELAY_GRACE_MS + 10,
        )
        .expect("third write");
    assert_ne!(third, first, "a retired run is never extended");
    agent_indicators::clear();
}

/// `begin()` clears every map unconditionally — minting over a foreign
/// live run (an in-app or in-daemon design turn) would wipe its badges,
/// reveals and pending root-seed contract mid-flight. The write skips
/// its own presence instead.
#[test]
fn a_foreign_live_run_is_never_clobbered() {
    let _g = agent_indicators::test_guard();
    agent_indicators::clear();
    let foreign = agent_indicators::begin();
    let state = seeded_state(&["n1"]);
    let mut session = ClientWriteIndicators::default();

    let out = session.register_applied_writes(
        &state,
        None,
        &touched(&["n1"]),
        EpochReuse::WhileDraining,
        5_000,
    );
    assert!(out.is_none(), "the MCP write yields to the foreign run");
    assert_eq!(
        agent_indicators::active_epoch(),
        Some(foreign),
        "the foreign run keeps its epoch and its maps"
    );
    assert_eq!(session.last_epoch(), 0, "no epoch was minted or adopted");
    agent_indicators::clear();
}

/// "Always overwrite" includes the empty case: an `initialize` that
/// carries no `clientInfo.name` drops the previous client's badge back
/// to the honest fallback rather than misattributing the next write.
#[test]
fn an_anonymous_reinitialize_drops_the_previous_clients_badge() {
    let _g = agent_indicators::test_guard();
    agent_indicators::clear();
    let mut session = ClientWriteIndicators::default();
    session.note_client_name(Some("cursor-agent".to_string()));
    session.note_client_name(None);
    assert_eq!(
        session.tag(),
        (
            MCP_CLIENT_FALLBACK_NAME.to_string(),
            MCP_CLIENT_COLOR.to_string()
        )
    );
}

/// Touched ids that resolve to nothing on the post-apply active tree —
/// a `DeleteNode`/`ReplaceNode` target, or a node the write pushed to a
/// different page — get no badge and no reveal: the canvas cannot paint
/// them, and phantom entries would still count toward the burst
/// smoother and the drain window.
#[test]
fn a_touched_id_the_tree_no_longer_has_registers_nothing() {
    let _g = agent_indicators::test_guard();
    agent_indicators::clear();
    let state = seeded_state(&["n1"]);
    let mut session = ClientWriteIndicators::default();
    let out = session.register_applied_writes(
        &state,
        None,
        &touched(&["n1", "ghost"]),
        EpochReuse::WhileDraining,
        5_000,
    );
    assert!(out.is_some(), "n1 is a live focus target");
    let snap = agent_indicators::snapshot();
    assert!(snap.nodes.contains_key("n1"));
    assert!(
        !snap.nodes.contains_key("ghost"),
        "the dead id carries no badge"
    );
    assert!(
        !snap.reveals.contains_key("ghost"),
        "the dead id anchors no waypoint"
    );
    agent_indicators::clear();
}

/// Every touched node gets the breathing border — but only ONE
/// back-dated reveal anchors the cursor. A second due-in-window reveal
/// counts as a burst at the next paint (`smooth_overdue_reveal_burst`
/// reschedules them) and the edited nodes would replay the entrance
/// pop the back-date exists to prevent.
#[test]
fn multiple_touched_nodes_share_one_cursor_anchor() {
    let _g = agent_indicators::test_guard();
    agent_indicators::clear();
    let state = seeded_state(&["a", "b", "c"]);
    let mut session = ClientWriteIndicators::default();
    let t0 = 10_000;
    session
        .register_applied_writes(
            &state,
            None,
            &touched(&["a", "b", "c"]),
            EpochReuse::WhileDraining,
            t0,
        )
        .expect("touched nodes register");

    let snap = agent_indicators::snapshot();
    for id in ["a", "b", "c"] {
        assert!(
            snap.nodes.contains_key(id),
            "every touched node carries the border: {id}"
        );
    }
    assert_eq!(
        snap.reveals.len(),
        1,
        "one anchor — more would trigger the burst reschedule"
    );
    assert!(
        snap.reveals.contains_key("c"),
        "the anchor lands on the write's last target"
    );

    // Through a real paint pass the single back-dated reveal is NOT a
    // burst: `newly_due <= 1` early-returns and the waypoint keeps its
    // parked start instead of being rescheduled into a pop.
    let painted = agent_indicators::snapshot_at(t0 + 1);
    assert_eq!(
        painted.reveals.get("c"),
        Some(&(t0 - FOCUS_REVEAL_LOOKBACK_MS)),
        "the parked anchor survives the burst smoother un-rescheduled"
    );
    agent_indicators::clear();
}

/// An id that is BOTH freshly created and directly targeted in the same
/// write (a `Batch{InsertAuthoredSubtree{X}, SetNodeFillHex{X}}`) keeps
/// the staggered entrance reveal `register_new_node_reveals` wrote —
/// the focus path's back-dated start must never overwrite it — and
/// takes no redundant focus badge.
#[test]
fn a_created_and_touched_node_keeps_its_staggered_reveal() {
    let _g = agent_indicators::test_guard();
    agent_indicators::clear();
    let mut state = EditorState::new();
    let before = crate::design_agent_tools::collect_active_node_ids(&state);
    let node: PenNode = serde_json::from_str(
        r#"{"type":"rectangle","id":"fresh-1","name":"F","x":0,"y":0,"width":10,"height":10}"#,
    )
    .expect("valid node json");
    assert!(state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![node],
        parent_id: NodeId::NONE,
        page_id: None,
    }));
    let mut session = ClientWriteIndicators::default();
    let t0 = 5_000;
    session
        .register_applied_writes(
            &state,
            Some(&before),
            &touched(&["fresh-1"]),
            EpochReuse::WhileDraining,
            t0,
        )
        .expect("the write registers");

    let snap = agent_indicators::snapshot();
    let started = snap
        .reveals
        .get("fresh-1")
        .expect("a created node keeps its entrance reveal");
    assert!(
        t0.saturating_sub(*started) < FOCUS_REVEAL_LOOKBACK_MS,
        "the staggered start was kept — a back-dated start would sit exactly {FOCUS_REVEAL_LOOKBACK_MS}ms in the past: started={started}"
    );
    assert!(
        !snap.nodes.contains_key("fresh-1"),
        "the new-node treatment wins — no focus badge on a just-created node"
    );
    agent_indicators::clear();
}
