//! Sibling test file for `agent_indicators.rs` (800-line cap
//! convention) — epoch scoping, run-long retention, graceful drain,
//! reveal queue scheduling and burst recovery.

#![cfg(test)]

use crate::agent_indicators::*;
use std::sync::{LazyLock, Mutex};

static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[test]
fn frame_generating_requires_active_run_and_registered_frame() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear();
    assert!(!is_frame_generating("frame"));

    let epoch = begin();
    assert!(!is_frame_generating("frame"));
    add_frame(epoch, "frame", "#4ECDC4", "Mochi");
    assert!(is_frame_generating("frame"));
    assert!(!is_frame_generating("other"));

    end_if_epoch(epoch);
    assert!(!is_frame_generating("frame"));
}

#[test]
fn generation_scan_deadline_ticks_only_for_active_registered_frames() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear();
    assert_eq!(next_generation_scan_deadline_ms(1_000), None);

    let epoch = begin();
    assert_eq!(next_generation_scan_deadline_ms(1_000), None);
    add_frame(epoch, "frame", "#4ECDC4", "Mochi");
    assert_eq!(
        next_generation_scan_deadline_ms(1_000),
        Some(1_000 + REVEAL_FRAME_MS)
    );

    end_if_epoch(epoch);
    assert_eq!(next_generation_scan_deadline_ms(1_000), None);
}

// One test owns the whole flow so it doesn't race the process-global
// registry against a sibling test under the default parallel runner.
#[test]
fn epoch_scopes_registration_and_teardown() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // Round-trip under a live epoch.
    let e1 = begin();
    add_node(e1, "n5", "#FF6B6B", "Nova");
    add_frame(e1, "n1", "#4ECDC4", "Mochi");
    mark_preview(e1, "n5");
    assert!(is_active());
    let snap = snapshot();
    assert_eq!(snap.nodes.get("n5").unwrap().color, "#FF6B6B");
    assert_eq!(snap.frames.get("n1").unwrap().name, "Mochi");
    assert!(snap.previews.contains("n5"));
    clear_preview(e1, "n5");
    assert!(!snapshot().previews.contains("n5"));

    // A newer run takes over: its begin() clears e1 and claims a fresh
    // epoch.
    let e2 = begin();
    assert!(e2 > e1, "begin bumps the epoch");
    assert!(snapshot().nodes.is_empty(), "begin clears the prior run");

    // The stale run (e1) keeps registering as it tears down — every
    // such call must be dropped, not folded into the live run.
    add_frame(e1, "stale", "#FF6B6B", "Nova");
    add_node(e1, "stale", "#FF6B6B", "Nova");
    mark_preview(e1, "stale");
    let snap = snapshot();
    assert!(snap.frames.is_empty(), "stale frame registration rejected");
    assert!(snap.nodes.is_empty(), "stale node registration rejected");
    assert!(
        snap.previews.is_empty(),
        "stale preview registration rejected"
    );

    // The live run registers fine under its own epoch.
    add_frame(e2, "live", "#4ECDC4", "Mochi");
    assert!(snapshot().frames.contains_key("live"));

    // The stale run's late teardown must not wipe the live run.
    clear_if_epoch(e1);
    assert!(
        snapshot().frames.contains_key("live"),
        "stale teardown is a no-op"
    );

    // The live run's own teardown clears.
    clear_if_epoch(e2);
    assert!(snapshot().frames.is_empty());
    assert!(!is_active());

    // end_if_epoch retires the epoch: after the host ends a run, a
    // worker still mid-registration under it can't re-populate.
    let e3 = begin();
    add_frame(e3, "f3", "#FFD93D", "Pixel");
    end_if_epoch(e3);
    assert!(snapshot().frames.is_empty(), "end_if_epoch clears");
    add_frame(e3, "late", "#FFD93D", "Pixel"); // in-flight registration
    assert!(
        snapshot().frames.is_empty(),
        "registration under a retired epoch no-ops"
    );

    // end_if_epoch on a stale epoch must not touch the live run.
    let e4 = begin();
    add_frame(e4, "f4", "#6C5CE7", "Echo");
    end_if_epoch(e3);
    assert!(
        snapshot().frames.contains_key("f4"),
        "end_if_epoch ignores a stale epoch"
    );
    clear();
}

#[test]
fn finish_drains_queued_reveals_before_clearing_the_overlay() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let epoch = begin();
    add_frame(epoch, "f1", "#4ECDC4", "Mochi");
    add_reveal(epoch, "queued", 5_000);
    finish_if_epoch(epoch);

    let draining = snapshot_at(1_000);
    assert!(
        draining.reveals.contains_key("queued"),
        "queued reveals keep playing after a graceful finish"
    );
    assert!(
        draining.frames.contains_key("f1"),
        "glow + badges stay up while the queue drains"
    );

    let after = snapshot_at(5_000 + REVEAL_DURATION_MS + 1);
    assert!(
        after.reveals.is_empty() && after.frames.is_empty(),
        "the whole overlay clears once the last reveal finishes"
    );
    assert!(!is_active(), "drained run stops requesting redraws");
    add_frame(epoch, "late", "#4ECDC4", "Mochi");
    assert!(
        snapshot().frames.is_empty(),
        "drain retires the epoch so stragglers no-op"
    );
    clear();
}

#[test]
fn finish_with_empty_queue_clears_immediately() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let epoch = begin();
    add_frame(epoch, "f1", "#4ECDC4", "Mochi");
    finish_if_epoch(epoch);
    assert!(!is_active(), "nothing queued — the overlay clears at once");

    // A stale finish must not touch the run that replaced it.
    let e2 = begin();
    add_frame(e2, "f2", "#FF6B6B", "Nova");
    finish_if_epoch(epoch);
    assert!(
        snapshot().frames.contains_key("f2"),
        "stale finish is a no-op"
    );
    clear();
}

#[test]
fn empty_finish_does_not_arm_a_spurious_erase_frame() {
    // A run that finishes without ever queuing a reveal never painted a
    // cursor, so it must NOT leave the process-global erase-frame flag set
    // — otherwise an unrelated animation-deadline query would observe a
    // stray redraw request. Regression guard for the shared-registry race.
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let epoch = begin();
    add_frame(epoch, "f1", "#4ECDC4", "Mochi");
    finish_if_epoch(epoch);
    assert_eq!(
        next_reveal_deadline_ms(5_000),
        None,
        "an empty finish leaves no pending erase frame for other queries to trip on"
    );
    clear();
}

#[test]
fn drain_end_requests_one_final_erase_frame() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let epoch = begin();
    add_reveal(epoch, "n1", 1_000);
    finish_if_epoch(epoch);

    // The host's "anything animating?" probe lands after the last
    // reveal's window: the drain clears the overlay inside that very
    // probe, which must still demand one more frame so the paint
    // path can erase the last-drawn cursor.
    let after = 1_000 + REVEAL_DURATION_MS + 20;
    assert!(
        next_reveal_deadline_ms(after).is_some(),
        "the drain must schedule the erase frame it just made necessary"
    );
    // The erase frame's paint consumes the request…
    assert!(snapshot_at_if_active(after).is_none());
    // …after which the loop settles for good.
    assert_eq!(next_reveal_deadline_ms(after + 16), None);
}

#[test]
fn stop_during_drain_clears_immediately() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let epoch = begin();
    add_reveal(epoch, "queued", 5_000);
    finish_if_epoch(epoch);
    assert!(is_active(), "drain keeps the overlay alive");
    end_if_epoch(epoch);
    assert!(
        !is_active(),
        "a user stop during the drain kills the overlay at once"
    );
}

#[test]
fn reveal_retention_spans_the_run_and_clears_with_it() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let epoch = begin();
    add_reveal(epoch, "n7", 1_000);
    assert!(is_active(), "reveal should keep the paint loop active");
    assert_eq!(snapshot_at(1_250).reveals.get("n7"), Some(&1_000));
    assert!(
        snapshot_at(30_000).reveals.contains_key("n7"),
        "a live run retains settled reveals as cursor / border anchors"
    );
    end_if_epoch(epoch);
    assert!(
        snapshot_at(30_001).reveals.is_empty(),
        "run end drops every anchor"
    );
    assert!(!is_active(), "a finished run should not keep animating");
    clear();
}

#[test]
fn snapshot_at_if_active_avoids_idle_snapshot_clone() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear();
    assert!(
        snapshot_at_if_active(1_000).is_none(),
        "idle paint should skip cloning an empty indicator snapshot"
    );

    let epoch = begin();
    add_frame(epoch, "frame", "#4ECDC4", "Mochi");
    assert!(
        snapshot_at_if_active(1_000).is_some_and(|snap| snap.frames.contains_key("frame")),
        "active frame indicators still produce a paint snapshot"
    );

    clear();
    let epoch = begin();
    add_reveal(epoch, "settled", 1_000);
    assert!(
        snapshot_at_if_active(3_000).is_some_and(|snap| snap.reveals.contains_key("settled")),
        "a live run keeps settled reveals as parked cursor anchors"
    );
    end_if_epoch(epoch);
    assert!(
        snapshot_at_if_active(3_000).is_none(),
        "a finished run leaves no idle snapshot"
    );
}

#[test]
fn snapshot_rebases_external_clock_reveals_to_paint_clock() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let epoch = begin();
    add_reveal(epoch, "a", 1_700_000_000_000);
    add_reveal(epoch, "b", 1_700_000_000_400);

    let snap = snapshot_at(1_000);

    // The batch's first slot floors at now + CURSOR_FLIGHT_LEAD_MS so the
    // pencil cursor gets a full eased flight instead of teleporting onto a
    // reveal that starts "now". Followers keep at least the queue beat.
    assert_eq!(
        snap.reveals.get("a"),
        Some(&(1_000 + CURSOR_FLIGHT_LEAD_MS))
    );
    assert_eq!(
        snap.reveals.get("b"),
        Some(&(1_000 + CURSOR_FLIGHT_LEAD_MS + REVEAL_STAGGER_MS))
    );
    clear();
}

#[test]
fn snapshot_queues_external_reveals_after_active_local_tail() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let epoch = begin();
    add_reveal(epoch, "local-a", 1_000);
    add_reveal(epoch, "local-b", 1_080);
    add_reveal(epoch, "external-a", 1_700_000_000_000);
    add_reveal(epoch, "external-b", 1_700_000_000_005);

    let snap = snapshot_at(1_040);

    // The flight-lead floor (1_040 + 350) wins over the local tail + beat
    // (1_080 + 160) here — the first external reveal still gets a full
    // cursor flight; followers keep the queue beat.
    assert_eq!(
        snap.reveals.get("external-a"),
        Some(&(1_040 + CURSOR_FLIGHT_LEAD_MS))
    );
    assert_eq!(
        snap.reveals.get("external-b"),
        Some(&(1_040 + CURSOR_FLIGHT_LEAD_MS + REVEAL_STAGGER_MS))
    );
    clear();
}

#[test]
fn active_reveal_deadline_ticks_at_animation_frame_rate() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let epoch = begin();
    add_reveal(epoch, "active", 1_000);

    assert_eq!(next_reveal_deadline_ms(1_100), Some(1_116));
    end_if_epoch(epoch);
}

#[test]
fn snapshot_reschedules_overdue_reveals_after_frame_gap() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let epoch = begin();
    for i in 0..6 {
        add_reveal(
            epoch,
            &format!("n{i}"),
            1_000 + i as u64 * REVEAL_STAGGER_MS,
        );
    }

    assert_eq!(snapshot_at(1_000).reveals.get("n0"), Some(&1_000));
    let snap = snapshot_at(1_220);
    let visible_count = snap
        .reveals
        .values()
        .filter(|started_at| **started_at <= 1_220)
        .count();

    assert!(
        visible_count <= 2,
        "a delayed frame should release at most one new reveal beyond the already-active tail"
    );
    assert!(
        snap.reveals
            .get("n2")
            .is_some_and(|started_at| *started_at > 1_220),
        "overdue nodes beyond the per-frame budget should be queued forward"
    );
    assert!(
        snap.reveals
            .get("n2")
            .is_some_and(|started_at| *started_at <= 1_220 + REVEAL_BURST_RECOVERY_STAGGER_MS),
        "overdue recovery should stay close enough to feel continuous"
    );
    end_if_epoch(epoch);
}

#[test]
fn snapshot_replays_first_overdue_reveal_instead_of_jumping_mid_animation() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let epoch = begin();
    for i in 0..6 {
        add_reveal(
            epoch,
            &format!("n{i}"),
            1_000 + i as u64 * REVEAL_STAGGER_MS,
        );
    }

    assert_eq!(snapshot_at(1_000).reveals.get("n0"), Some(&1_000));
    // A frame gap long enough that three queued reveals became due at
    // once (n1..n3 at the 160ms cadence).
    let probe = 1_000 + 3 * REVEAL_STAGGER_MS + 20;
    let snap = snapshot_at(probe);

    assert_eq!(
        snap.reveals.get("n1"),
        Some(&probe),
        "the first overdue node should restart at the current frame instead of appearing partway through"
    );
    assert_eq!(
        snap.reveals.get("n2"),
        Some(&(probe + REVEAL_BURST_RECOVERY_STAGGER_MS)),
        "overdue siblings should recover one-by-one on the queue cadence"
    );
    end_if_epoch(epoch);
}

#[test]
fn first_snapshot_requeues_overdue_reveals_instead_of_showing_burst() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let epoch = begin();
    for i in 0..8 {
        add_reveal(
            epoch,
            &format!("n{i}"),
            1_000 + i as u64 * REVEAL_STAGGER_MS,
        );
    }

    let snap = snapshot_at(1_360);
    let visible_count = snap
        .reveals
        .values()
        .filter(|started_at| **started_at <= 1_360)
        .count();

    assert_eq!(
        visible_count, 1,
        "the first paint after a busy apply should not materialize every overdue node at once"
    );
    assert_eq!(
        snap.reveals.get("n0"),
        Some(&1_360),
        "the first overdue node should replay from the current frame"
    );
    assert!(
        snap.reveals
            .get("n1")
            .is_some_and(|started_at| *started_at >= 1_360 + REVEAL_STAGGER_MS),
        "remaining overdue nodes should requeue at normal stream cadence"
    );
    end_if_epoch(epoch);
}

#[test]
fn snapshot_recovery_preserves_stream_order_after_frame_gap() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let epoch = begin();
    for i in 0..16 {
        add_reveal(epoch, &format!("n{i:02}"), 1_000 + reveal_offset_ms(0, i));
    }

    assert_eq!(snapshot_at(1_000).reveals.get("n00"), Some(&1_000));
    let snap = snapshot_at(1_220);
    let starts: Vec<u64> = (0..16)
        .map(|i| {
            *snap
                .reveals
                .get(&format!("n{i:02}"))
                .expect("reveal is retained")
        })
        .collect();

    assert!(
        starts.windows(2).all(|pair| pair[0] < pair[1]),
        "overdue recovery must preserve stream order instead of letting future nodes jump ahead"
    );
    assert!(
        starts
            .windows(2)
            .skip(1)
            .all(|pair| pair[1] - pair[0] >= REVEAL_STAGGER_MS),
        "recovered starts should stay at the normal stream cadence instead of clustering"
    );
    end_if_epoch(epoch);
}

#[test]
fn reveal_offsets_form_a_uniform_queue() {
    let offsets: Vec<u64> = (0..40).map(|i| reveal_offset_ms(1, i)).collect();

    assert_eq!(offsets[0], REVEAL_DEPTH_STAGGER_MS);
    assert!(
        offsets
            .windows(2)
            .all(|pair| pair[1] - pair[0] == REVEAL_STAGGER_MS),
        "every placement gets the same readable beat — no burst compression"
    );
    assert!(
        offsets
            .windows(2)
            .all(|pair| pair[1] - pair[0] >= REVEAL_FRAME_MS),
        "stream items should not share an entrance start frame at 60 fps"
    );
}

#[test]
fn reveal_offsets_give_each_element_a_readable_beat() {
    let offsets: Vec<u64> = (0..24).map(|i| reveal_offset_ms(0, i)).collect();

    assert!(
        offsets.windows(2).all(|pair| pair[1] - pair[0] >= 100),
        "the beat must stay slow enough that each placement reads distinctly"
    );
    assert_eq!(offsets[1] - offsets[0], REVEAL_STAGGER_MS);
}

// ── Relay (daemon → browser) ────────────────────────────────────────────

#[test]
fn relay_json_round_trips_through_parse() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let e = begin();
    add_node(e, "n5", "#FF6B6B", "Nova");
    add_frame(e, "n1", "#4ECDC4", "Mochi");
    mark_preview(e, "n9");
    add_reveal(e, "n5", 1_234);

    let remote = parse_relay_json(&relay_json()).expect("relay body parses");
    assert!(remote.run_active);
    assert_eq!(
        remote.nodes,
        vec![(
            "n5".to_string(),
            AgentTag {
                color: "#FF6B6B".to_string(),
                name: "Nova".to_string()
            }
        )]
    );
    assert_eq!(remote.frames.len(), 1);
    assert_eq!(remote.previews, vec!["n9".to_string()]);
    assert_eq!(remote.reveals, vec![("n5".to_string(), 1_234)]);
    clear();
}

#[test]
fn apply_remote_mirrors_a_daemon_run_locally() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear();
    let remote = RemoteIndicators {
        run_active: true,
        nodes: vec![(
            "n5".to_string(),
            AgentTag {
                color: "#FF6B6B".to_string(),
                name: "Nova".to_string(),
            },
        )],
        frames: vec![],
        previews: vec!["n9".to_string()],
        reveals: vec![("n5".to_string(), 777)],
    };
    assert!(
        apply_remote(&remote),
        "an active remote run keeps the pump alive"
    );
    let snap = snapshot();
    assert!(snap.run_active);
    assert_eq!(snap.nodes.get("n5").unwrap().name, "Nova");
    assert!(snap.previews.contains("n9"));
    assert_eq!(snap.reveals.get("n5"), Some(&777));

    // Re-applying must not clobber a reveal timestamp the paint path
    // may already have rebased onto the local clock.
    let mut second = remote.clone();
    second.reveals = vec![("n5".to_string(), 999_999), ("n6".to_string(), 888)];
    apply_remote(&second);
    let snap = snapshot();
    assert_eq!(snap.reveals.get("n5"), Some(&777), "existing reveal kept");
    assert_eq!(snap.reveals.get("n6"), Some(&888), "new reveal inserted");

    // Remote run ends with reveals still queued → graceful drain, not an
    // instant wipe; the pump keeps running until the queue plays out.
    let mut ended = second.clone();
    ended.run_active = false;
    assert!(apply_remote(&ended), "drain keeps animating");
    assert!(!snapshot().run_active);

    // An idle remote on an idle local registry is a cheap no-op.
    clear();
    let idle = RemoteIndicators::default();
    assert!(!apply_remote(&idle));
}
