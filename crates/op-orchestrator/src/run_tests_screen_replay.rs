//! Replay-order / progress-visibility tests plus the per-group agent
//! identity tagging.

use super::*;

// ── Three-piece visibility fix (2026-07-17) ─────────────────────────────

/// Order in which each of `markers` FIRST appears across `applied`'s
/// `InsertSubtree` commands — proves REPLAY order (which command actually
/// landed in the real sink first), independent of plan order.
fn insert_marker_order(applied: &[EditorCommand], markers: &[&str]) -> Vec<String> {
    fn visit(cmd: &EditorCommand, markers: &[&str], order: &mut Vec<String>) {
        match cmd {
            EditorCommand::InsertSubtree { nodes, .. } => {
                for marker in markers {
                    if nodes.iter().any(|n| contains_text(n, marker))
                        && !order.contains(&marker.to_string())
                    {
                        order.push(marker.to_string());
                    }
                }
            }
            EditorCommand::Batch { commands } => {
                for command in commands {
                    visit(command, markers, order);
                }
            }
            _ => {}
        }
    }

    let mut order = Vec::new();
    for cmd in applied {
        visit(cmd, markers, &mut order);
    }
    order
}

/// Item 2 ("按组完成即 replay"): the group whose LLM call resolves FIRST
/// replays into the real document FIRST, regardless of its plan index.
/// `library-body` is plan index 1 (after `search-body`), but `DelayedLlm`
/// makes its call resolve with zero extra polls while Search's takes many —
/// the replayed `InsertSubtree` order must show Library before Search.
#[test]
fn faster_group_replays_before_a_slower_earlier_plan_index_group() {
    let llm = DelayedLlm::new(vec![
        (0, THREE_SCREEN_PLAN_JSON.into()), // planning call — no delay
        (8, node_json("s")),                // search-body: many extra polls
        (0, node_json("l")),                // library-body: resolves immediately
        (4, node_json("p")),                // premium-body: some delay
    ]);
    let mut sink = VecDocSink::new();

    futures::executor::block_on(Orchestrator::new().run(
        req_with_concurrency(3),
        &mut sink,
        &llm,
        &mut |_| {},
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("3-group concurrent run ok");

    // Final document is correct regardless of replay interleaving.
    let roots = sink.state.active_children();
    for name in ["Search", "Library", "Premium"] {
        find_root_by_name(roots, name);
    }

    let order = insert_marker_order(&sink.applied, &["s", "l", "p"]);
    let l_pos = order
        .iter()
        .position(|m| m == "l")
        .expect("library replayed");
    let s_pos = order
        .iter()
        .position(|m| m == "s")
        .expect("search replayed");
    assert!(
        l_pos < s_pos,
        "the faster (Library) group must replay before the slower, EARLIER-plan-index \
         (Search) group: replay order was {order:?}"
    );
}

/// Item 1 ("进度实时 drain"): `on_progress` must observe a group's terminal
/// event WHILE its siblings are still in flight — not only after every
/// worker has resolved. `DelayedLlm::finished()` (a counter incremented the
/// instant each call's stream produces its chunk, read from OUTSIDE the
/// callback) is what actually distinguishes "delivered live" from
/// "delivered late but in the right order" — an mpsc channel preserves send
/// order either way, so final-Vec ordering alone can't prove this.
#[test]
fn progress_is_delivered_while_slower_siblings_are_still_running() {
    let llm = std::sync::Arc::new(DelayedLlm::new(vec![
        (0, THREE_SCREEN_PLAN_JSON.into()),
        (10, node_json("s")),
        (0, node_json("l")),
        (6, node_json("p")),
    ]));
    let mut sink = VecDocSink::new();
    let llm_for_progress = std::sync::Arc::clone(&llm);
    // Snapshot `llm.finished()` at the moment Library's terminal event
    // arrives — if delivery is live, Search (delay=10) and Premium
    // (delay=6) cannot have resolved yet, so the count must be < 3.
    let mut finished_at_library_done: Option<usize> = None;

    futures::executor::block_on(Orchestrator::new().run(
        req_with_concurrency(3),
        &mut sink,
        &*llm,
        &mut |p| {
            if matches!(progress_event(&p), Progress::SubtaskDone { id, .. } if id == "library-body") {
                finished_at_library_done = Some(llm_for_progress.finished());
            }
        },
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("3-group concurrent run ok");

    let observed = finished_at_library_done.expect("library-body must report SubtaskDone");
    assert!(
        observed < 3,
        "Library's SubtaskDone must reach on_progress before every group's LLM call has \
         resolved (observed {observed}/3 finished) — a post-hoc batched drain would always \
         observe 3/3"
    );
}

/// Item 4: a fact-line precedes the concurrent phase so ⚡Nx's effect is
/// legible in the progress panel instead of silent.
#[test]
fn concurrent_run_announces_group_and_worker_count_upfront() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(THREE_SCREEN_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("s")),
        ScriptResponse::Text(node_json("l")),
        ScriptResponse::Text(node_json("p")),
    ]);
    let mut sink = VecDocSink::new();
    let mut progress = Vec::new();

    futures::executor::block_on(Orchestrator::new().run(
        req_with_concurrency(3),
        &mut sink,
        &llm,
        &mut |p| progress.push(p),
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("3-group concurrent run ok");

    assert!(
        progress.iter().any(|p| matches!(
            p,
            Progress::ConcurrentGroupsStarted {
                group_count: 3,
                workers: 3,
            }
        )),
        "expected a ConcurrentGroupsStarted{{group_count:3, workers:3}} event: {progress:?}"
    );
    // Never fires on the sequential path.
    let mut seq_progress = Vec::new();
    let seq_llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(THREE_SCREEN_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("s")),
        ScriptResponse::Text(node_json("l")),
        ScriptResponse::Text(node_json("p")),
    ]);
    let mut seq_sink = VecDocSink::new();
    futures::executor::block_on(Orchestrator::new().run(
        req_with_concurrency(1),
        &mut seq_sink,
        &seq_llm,
        &mut |p| seq_progress.push(p),
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("sequential 3-group run ok");
    assert!(
        !seq_progress
            .iter()
            .any(|p| matches!(p, Progress::ConcurrentGroupsStarted { .. })),
        "concurrency=1 must never announce a concurrent phase"
    );
}

/// Item 3: when the groups genuinely run concurrently, each screen's root
/// gets its OWN distinct agent-indicator identity (colour + name) — not one
/// shared identity for all of them.
#[test]
fn concurrent_groups_get_distinct_agent_identities() {
    let _guard = crate::agent_indicator_test_support::lock();
    let epoch = op_editor_core::agent_indicators::begin();
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(THREE_SCREEN_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("s")),
        ScriptResponse::Text(node_json("l")),
        ScriptResponse::Text(node_json("p")),
    ]);
    let mut sink = VecDocSink::new();

    futures::executor::block_on(Orchestrator::new().with_indicator_epoch(epoch).run(
        req_with_concurrency(3),
        &mut sink,
        &llm,
        &mut |_| {},
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("3-group concurrent run ok");

    let roots = sink.state.active_children();
    let search_id = find_root_by_name(roots, "Search").id_str().to_string();
    let library_id = find_root_by_name(roots, "Library").id_str().to_string();
    let premium_id = find_root_by_name(roots, "Premium").id_str().to_string();

    let snap = op_editor_core::agent_indicators::snapshot();
    let search_tag = snap.frames.get(&search_id).expect("Search root tagged");
    let library_tag = snap.frames.get(&library_id).expect("Library root tagged");
    let premium_tag = snap.frames.get(&premium_id).expect("Premium root tagged");
    assert_ne!(
        search_tag, library_tag,
        "each group must get its OWN identity"
    );
    assert_ne!(search_tag, premium_tag);
    assert_ne!(library_tag, premium_tag);

    op_editor_core::agent_indicators::end_if_epoch(epoch);
}

/// Regression lock (dual-cursor-identity fix, 2026-07-17): the SEQUENTIAL
/// path (concurrency=1, even with multiple screen groups) must tag NO root
/// with its own identity — every reveal falls through to the host-confirmed
/// `cursor_agent`, the single source of truth for a single-agent run. Before
/// this fix the sequential path minted its OWN "Kiki" (seed-0) identity and
/// tagged every root with it — a second, orchestrator-only identity source
/// independent of the transcript's confirmed identity, which surfaced as two
/// simultaneous cursors once `canvas_agent_cursor.rs`'s ownership-tag
/// precedence was flipped (three-piece visibility fix) to stop hiding it.
#[test]
fn sequential_multi_screen_run_tags_no_frame_identity() {
    let _guard = crate::agent_indicator_test_support::lock();
    let epoch = op_editor_core::agent_indicators::begin();
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(THREE_SCREEN_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("s")),
        ScriptResponse::Text(node_json("l")),
        ScriptResponse::Text(node_json("p")),
    ]);
    let mut sink = VecDocSink::new();

    futures::executor::block_on(Orchestrator::new().with_indicator_epoch(epoch).run(
        req_with_concurrency(1),
        &mut sink,
        &llm,
        &mut |_| {},
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("sequential 3-group run ok");

    let roots = sink.state.active_children();
    let search_id = find_root_by_name(roots, "Search").id_str().to_string();
    let library_id = find_root_by_name(roots, "Library").id_str().to_string();
    let premium_id = find_root_by_name(roots, "Premium").id_str().to_string();

    let snap = op_editor_core::agent_indicators::snapshot();
    assert!(
        snap.frames.is_empty(),
        "sequential path must tag NO root — orchestrator identity minting is \
         concurrent-only now: {:?}",
        snap.frames
    );
    // Simulate the host's transcript-identity confirmation (the ONE real
    // source for a single-agent run) and prove every root's waypoints
    // resolve to it once tagged this way — `canvas_agent_cursor.rs`'s
    // `confirmed` fallback for untagged nodes.
    op_editor_core::agent_indicators::confirm_cursor_agent(epoch, "#4ECDC4", "Nova");
    let confirmed = op_editor_core::agent_indicators::snapshot()
        .cursor_agent
        .expect("confirmed just now");
    assert_eq!(confirmed.name, "Nova");
    for id in [&search_id, &library_id, &premium_id] {
        assert!(
            !snap.frames.contains_key(id),
            "root {id} must stay untagged so it inherits the confirmed identity"
        );
    }

    op_editor_core::agent_indicators::end_if_epoch(epoch);
}

/// Dual-cursor-identity fix companion: when groups genuinely run
/// CONCURRENTLY, the orchestrator confirms the FIRST group's identity as the
/// canonical `cursor_agent` — so the transcript speaker (which the host
/// resolves from `cursor_agent` when nothing is stamped on the message yet)
/// matches one of the N visible group cursors (the primary/first group)
/// instead of an unrelated third persona.
#[test]
fn concurrent_run_confirms_primary_group_identity_as_cursor_agent() {
    let _guard = crate::agent_indicator_test_support::lock();
    let epoch = op_editor_core::agent_indicators::begin();
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(THREE_SCREEN_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("s")),
        ScriptResponse::Text(node_json("l")),
        ScriptResponse::Text(node_json("p")),
    ]);
    let mut sink = VecDocSink::new();

    futures::executor::block_on(Orchestrator::new().with_indicator_epoch(epoch).run(
        req_with_concurrency(3),
        &mut sink,
        &llm,
        &mut |_| {},
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("3-group concurrent run ok");

    let roots = sink.state.active_children();
    let search_id = find_root_by_name(roots, "Search").id_str().to_string();

    let snap = op_editor_core::agent_indicators::snapshot();
    let search_tag = snap.frames.get(&search_id).expect("Search root tagged");
    let confirmed = snap
        .cursor_agent
        .expect("concurrent phase must confirm a primary identity");
    assert_eq!(
        &confirmed, search_tag,
        "the confirmed cursor_agent must match the FIRST (primary) group's identity"
    );

    op_editor_core::agent_indicators::end_if_epoch(epoch);
}

/// Web-streaming half of the dual-cursor-identity fix (2026-07-17):
/// `web_chat_standard.rs` confirms + announces an identity to its SSE client
/// BEFORE calling `Orchestrator::run()` at all (the transcript needs a
/// persona immediately, well before groups/concurrency is known). This test
/// simulates exactly that — pre-confirm a `cursor_agent`, THEN run a
/// genuinely concurrent 3-group plan — and proves the primary group's
/// identity ADOPTS the pre-confirmed one instead of the orchestrator
/// silently overwriting it with a freshly minted "Kiki". Content-level
/// mirror of `agent_identity::tests::with_primary_keeps_the_primary_as_the_first_identity`,
/// exercised through the real `Orchestrator::run()` path.
#[test]
fn concurrent_run_adopts_a_pre_confirmed_identity_as_the_primary() {
    let _guard = crate::agent_indicator_test_support::lock();
    let epoch = op_editor_core::agent_indicators::begin();
    // Simulate the web route: confirm + (conceptually) stream this identity
    // to the client BEFORE `run()` is ever called.
    op_editor_core::agent_indicators::confirm_cursor_agent(epoch, "#4ECDC4", "Nova");

    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(THREE_SCREEN_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("s")),
        ScriptResponse::Text(node_json("l")),
        ScriptResponse::Text(node_json("p")),
    ]);
    let mut sink = VecDocSink::new();

    futures::executor::block_on(Orchestrator::new().with_indicator_epoch(epoch).run(
        req_with_concurrency(3),
        &mut sink,
        &llm,
        &mut |_| {},
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("3-group concurrent run ok");

    let roots = sink.state.active_children();
    let search_id = find_root_by_name(roots, "Search").id_str().to_string();
    let library_id = find_root_by_name(roots, "Library").id_str().to_string();
    let premium_id = find_root_by_name(roots, "Premium").id_str().to_string();

    let snap = op_editor_core::agent_indicators::snapshot();
    assert_eq!(
        snap.cursor_agent,
        Some(op_editor_core::agent_indicators::AgentTag {
            color: "#4ECDC4".into(),
            name: "Nova".into(),
        }),
        "the pre-confirmed identity must survive the concurrent phase unchanged"
    );
    assert_eq!(
        snap.frames.get(&search_id),
        snap.cursor_agent.as_ref(),
        "the primary (first) group's tag must equal the adopted pre-confirmed identity"
    );
    // The other two groups still get their own DISTINCT identities.
    let library_tag = snap.frames.get(&library_id).expect("Library tagged");
    let premium_tag = snap.frames.get(&premium_id).expect("Premium tagged");
    assert_ne!(library_tag.color, "#4ECDC4");
    assert_ne!(premium_tag.color, "#4ECDC4");
    assert_ne!(library_tag.color, premium_tag.color);

    op_editor_core::agent_indicators::end_if_epoch(epoch);
}

// ── Sequential-execution self-diagnostic (2026-07-17 root-cause hunt) ──────

/// `ScreenGroupsSequential` fires exactly once, with the raw (unclamped)
/// `request.concurrency` the turn actually carried — the dual of
/// `concurrent_run_announces_group_and_worker_count_upfront`. This is the
/// line that lets a future real-world repro self-diagnose: if it shows
/// `requested_workers: 1`, the ⚡Nx picker's value never reached
/// `DesignRequest.concurrency` for that turn; `effective_concurrency`
/// itself is proven correct by `multi_group_is_capped_by_both_clamp_and_group_count`
/// (concurrent_tests.rs) — it CANNOT return 1 here unless `request.concurrency`
/// really was ≤1.
#[test]
fn sequential_run_announces_group_count_and_requested_workers() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(THREE_SCREEN_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("s")),
        ScriptResponse::Text(node_json("l")),
        ScriptResponse::Text(node_json("p")),
    ]);
    let mut sink = VecDocSink::new();
    let mut progress = Vec::new();

    futures::executor::block_on(Orchestrator::new().run(
        req_with_concurrency(1),
        &mut sink,
        &llm,
        &mut |p| progress.push(p),
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("sequential 3-group run ok");

    let hits: Vec<_> = progress
        .iter()
        .filter(|p| matches!(p, Progress::ScreenGroupsSequential { .. }))
        .collect();
    assert_eq!(hits.len(), 1, "must fire exactly once: {progress:?}");
    assert!(
        matches!(
            hits[0],
            Progress::ScreenGroupsSequential {
                group_count: 3,
                requested_workers: 1,
            }
        ),
        "expected group_count:3, requested_workers:1, got {:?}",
        hits[0]
    );
    // Never fires alongside the concurrent phase's own announcement.
    assert!(!progress
        .iter()
        .any(|p| matches!(p, Progress::ConcurrentGroupsStarted { .. })));
}

/// Regression: an ordinary single-screen plan (the overwhelming majority of
/// runs — no `screen` tags at all, `groups.len() <= 1`) must NEVER emit this
/// diagnostic line. It exists to flag an ANOMALY (multi-group plan that
/// still went sequential), not to narrate the always-sequential common case.
#[test]
fn single_screen_run_never_announces_the_sequential_diagnostic() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(MULTI_SCREEN_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("home")),
        ScriptResponse::Text(node_json("profile")),
    ]);
    let mut sink = VecDocSink::new();
    let mut progress = Vec::new();

    // MULTI_SCREEN_PLAN_JSON actually has 2 screens — use the plain `req()`
    // fixture with concurrency=1 (its default) so this exercises the
    // 2-group-but-sequential case too, alongside a genuinely single-group
    // plan below.
    futures::executor::block_on(Orchestrator::new().run(
        req(),
        &mut sink,
        &llm,
        &mut |p| progress.push(p),
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("2-group sequential run ok");
    assert!(
        progress
            .iter()
            .any(|p| matches!(p, Progress::ScreenGroupsSequential { group_count: 2, .. })),
        "a 2-group plan run sequentially SHOULD still announce (only single-group is exempt)"
    );

    // Genuinely single-group plan (no `screen` tags): must stay silent.
    let single_llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(
            r##"{
  "sharedContext": {}, "rootFrame": { "id": "root", "name": "App", "width": 1200, "height": 800,
                 "layout": "vertical", "gap": 0,
                 "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
  "subtasks": [
    { "id": "hero", "label": "Hero", "region": { "width": 1200, "height": 400 } }
  ]
}"##
            .into(),
        ),
        ScriptResponse::Text(node_json("hero")),
    ]);
    let mut single_sink = VecDocSink::new();
    let mut single_progress = Vec::new();
    futures::executor::block_on(Orchestrator::new().run(
        req(),
        &mut single_sink,
        &single_llm,
        &mut |p| single_progress.push(p),
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("single-group run ok");
    assert!(
        !single_progress
            .iter()
            .any(|p| matches!(p, Progress::ScreenGroupsSequential { .. })),
        "single-group plans must never announce: {single_progress:?}"
    );
}

#[path = "run_tests_screen_groups_atomic_replay.rs"]
mod atomic_replay;
