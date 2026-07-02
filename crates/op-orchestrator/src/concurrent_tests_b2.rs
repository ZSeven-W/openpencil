//! `concurrent` Task B2 tests — `run_concurrent` (concurrent executor).
//!
//! Split from `concurrent_tests.rs` to keep both test files under the
//! 800-line cap.  Wired as a second `#[path = ...] mod tests_b2;` inside
//! `concurrent.rs`; like `concurrent_tests.rs` this stays a child module of
//! `concurrent`, so `use super::*` resolves to `concurrent`.

use super::*;
use crate::test_support::{CountingLlm, ScriptResponse, ScriptedLlm, VecDocSink};
use futures::executor::block_on;
use op_editor_core::{EditorState, PenNodeExt};

const NODE_JSON: &str = r#"[{"type":"frame","id":"h1","name":"H","x":0,"y":0,"width":100,"height":100,"children":[{"type":"text","id":"h-title","content":"H","fontSize":18}]}]"#;

/// A default `DesignRequest` (concurrency overridden per-test as needed).
fn make_req() -> crate::types::DesignRequest {
    crate::types::DesignRequest {
        prompt: "test".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 2,
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
    }
}

/// Build a multi-screen plan: s0=login (plan index 0), s1=home (plan index 1).
fn make_two_screen_plan() -> crate::plan::OrchestratorPlan {
    use crate::plan::{OrchestratorPlan, Region, RootFrameSpec};
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Page".into(),
            width: 1200.0,
            height: 800.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![
            crate::plan::Subtask {
                id: "s0".into(),
                label: "Login".into(),
                region: Region {
                    width: 1200.0,
                    height: 400.0,
                },
                id_prefix: "s0".into(),
                parent_frame_id: Some("root".into()),
                elements: None,
                screen: Some("login".into()),
                generated_root_id: None,
                existing_section_labels: None,
            },
            crate::plan::Subtask {
                id: "s1".into(),
                label: "Home".into(),
                region: Region {
                    width: 1200.0,
                    height: 400.0,
                },
                id_prefix: "s1".into(),
                parent_frame_id: Some("root".into()),
                elements: None,
                screen: Some("home".into()),
                generated_root_id: None,
                existing_section_labels: None,
            },
        ],
        style_guide_name: None,
    }
}

/// Build a 3-screen plan (s0=a, s1=b, s2=c) — used for the genuine-interleaving
/// test where all 3 groups should overlap.
fn make_three_screen_plan() -> crate::plan::OrchestratorPlan {
    use crate::plan::{OrchestratorPlan, Region, RootFrameSpec};
    let mk = |id: &str, screen: &str| crate::plan::Subtask {
        id: id.into(),
        label: id.into(),
        region: Region {
            width: 1200.0,
            height: 400.0,
        },
        id_prefix: id.into(),
        parent_frame_id: Some("root".into()),
        elements: None,
        screen: Some(screen.into()),
        generated_root_id: None,
        existing_section_labels: None,
    };
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Page".into(),
            width: 1200.0,
            height: 800.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![mk("s0", "a"), mk("s1", "b"), mk("s2", "c")],
        style_guide_name: None,
    }
}

/// Happy path: 2 screen groups, both succeed.
/// After `run_concurrent`, the real sink receives 2 commands (one per subtask)
/// in plan order, and all 4 progress events arrive.
#[test]
fn concurrent_two_groups_both_succeed_replays_in_plan_order() {
    let plan = make_two_screen_plan();
    let req = make_req();
    let groups = group_subtasks_by_screen(&plan.subtasks);
    assert_eq!(groups.len(), 2);

    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(NODE_JSON.into()), // group[0] s0
        ScriptResponse::Text(NODE_JSON.into()), // group[1] s1
    ]);

    let abort = AbortFlag::new();
    let snapshot = EditorState::new();
    let mut real_sink = BufferDocSink::new(snapshot.clone());
    let mut progress_events: Vec<String> = Vec::new();

    let outcomes = block_on(run_concurrent(
        &groups,
        &plan,
        &req,
        &llm,
        &abort,
        snapshot,
        &mut real_sink,
        &mut |p| match &p {
            Progress::SubtaskStarted { id, .. } => progress_events.push(format!("start:{id}")),
            Progress::SubtaskDone { id, .. } => progress_events.push(format!("done:{id}")),
            Progress::SubtaskFailed { id, .. } => progress_events.push(format!("fail:{id}")),
            _ => {}
        },
        None,
    ));

    // Both subtasks produced 1 node each.
    assert_eq!(outcomes.len(), 2);
    assert_eq!(outcomes[0].as_ref().map(|o| o.node_count), Some(1));
    assert_eq!(outcomes[1].as_ref().map(|o| o.node_count), Some(1));

    // Real sink got 2 commands (one per subtask) — replayed in plan order.
    assert_eq!(real_sink.commands.len(), 2);

    // All 4 progress events arrived.
    assert!(
        progress_events.contains(&"start:s0".to_string()),
        "missing start:s0, events: {progress_events:?}"
    );
    assert!(
        progress_events.contains(&"done:s0".to_string()),
        "missing done:s0"
    );
    assert!(
        progress_events.contains(&"start:s1".to_string()),
        "missing start:s1"
    );
    assert!(
        progress_events.contains(&"done:s1".to_string()),
        "missing done:s1"
    );
}

#[test]
fn concurrent_replay_registers_reveals_for_live_inserted_nodes() {
    let _guard = crate::agent_indicator_test_support::lock();
    let mut plan = make_two_screen_plan();
    for subtask in &mut plan.subtasks {
        subtask.parent_frame_id = None;
    }
    let req = make_req();
    let groups = group_subtasks_by_screen(&plan.subtasks);
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(NODE_JSON.into()),
        ScriptResponse::Text(NODE_JSON.into()),
    ]);
    let abort = AbortFlag::new();
    let snapshot = EditorState::new();
    let mut real_sink = VecDocSink::new();
    let mut progress_events: Vec<Progress> = Vec::new();
    let epoch = op_editor_core::agent_indicators::begin();

    let outcomes = block_on(run_concurrent(
        &groups,
        &plan,
        &req,
        &llm,
        &abort,
        snapshot,
        &mut real_sink,
        &mut |p| progress_events.push(p),
        Some(epoch),
    ));

    assert_eq!(outcomes.len(), 2);
    assert_eq!(outcomes.iter().filter(|o| o.is_some()).count(), 2);
    let reveal_snapshot = op_editor_core::agent_indicators::snapshot();
    assert_eq!(
        reveal_snapshot.reveals.len(),
        4,
        "each replayed subtree root and child should reveal"
    );
    let mut live_ids = std::collections::HashSet::new();
    for node in real_sink.state.active_children() {
        collect_live_ids(node, &mut live_ids);
    }
    assert!(
        reveal_snapshot
            .reveals
            .keys()
            .all(|id| live_ids.contains(id.as_str())),
        "reveals must use live remapped ids, got {:?} vs {:?}",
        reveal_snapshot.reveals.keys().collect::<Vec<_>>(),
        live_ids
    );
    op_editor_core::agent_indicators::end_if_epoch(epoch);
}

fn collect_live_ids(
    node: &jian_ops_schema::node::PenNode,
    out: &mut std::collections::HashSet<String>,
) {
    out.insert(node.id_str().to_string());
    if let Some(children) = node.children() {
        for child in children {
            collect_live_ids(child, out);
        }
    }
}

/// **Genuine interleaving** — with `concurrency=3` and 3 screen groups, the
/// 3 workers' LLM calls overlap in wall-clock time.  `CountingLlm`'s stream
/// yields once (Poll::Pending) before producing its chunk, so under `join_all`
/// worker A goes pending → worker B's `call` runs → worker C's `call` runs →
/// all 3 are in-flight simultaneously.  `max_concurrent` must reach 3.
///
/// This test would FAIL with a sequential `for worker.await` loop: a sequential
/// loop awaits worker A to FULL completion before starting B, so `current`
/// never exceeds 1 — `max_concurrent` would be 1, not 3.  (Verified by
/// temporarily reverting `run_concurrent` to a sequential loop — this test then
/// fails with `left: 1, right: 3`.)
#[test]
fn concurrent_workers_genuinely_interleave() {
    let plan = make_three_screen_plan();
    let mut req = make_req();
    req.concurrency = 3; // 3 groups → effective_concurrency = 3 (no cap pressure)

    let groups = group_subtasks_by_screen(&plan.subtasks);
    assert_eq!(groups.len(), 3);

    let llm = CountingLlm::new(vec![NODE_JSON.into(), NODE_JSON.into(), NODE_JSON.into()]);

    let abort = AbortFlag::new();
    let snapshot = EditorState::new();
    let mut real_sink = BufferDocSink::new(snapshot.clone());

    let _outcomes = block_on(run_concurrent(
        &groups,
        &plan,
        &req,
        &llm,
        &abort,
        snapshot,
        &mut real_sink,
        &mut |_| {},
        None,
    ));

    // All 3 workers' LLM calls genuinely overlapped.
    assert_eq!(
        llm.max_concurrent(),
        3,
        "with concurrency=3 + 3 groups, all 3 workers must be in-flight at once \
         (a sequential loop would yield 1)"
    );
}

/// Semaphore cap: with `concurrency=1` and 2 groups, the semaphore caps
/// in-flight LLM calls at 1 — even though `join_all` drives both workers
/// concurrently, the single permit serializes their LLM calls.
/// `max_concurrent` must stay ≤ 1.
#[test]
fn concurrent_semaphore_caps_in_flight_calls() {
    let plan = make_two_screen_plan();
    let mut req = make_req();
    req.concurrency = 1; // 2 groups → effective_concurrency = 1 (hard cap)

    let groups = group_subtasks_by_screen(&plan.subtasks);
    let llm = CountingLlm::new(vec![NODE_JSON.into(), NODE_JSON.into()]);

    let abort = AbortFlag::new();
    let snapshot = EditorState::new();
    let mut real_sink = BufferDocSink::new(snapshot.clone());

    let _outcomes = block_on(run_concurrent(
        &groups,
        &plan,
        &req,
        &llm,
        &abort,
        snapshot,
        &mut real_sink,
        &mut |_| {},
        None,
    ));

    // The cap-1 semaphore must serialize the LLM calls despite join_all driving
    // both workers — at most 1 call in-flight at a time.
    assert_eq!(
        llm.max_concurrent(),
        1,
        "cap-1 semaphore must keep in-flight ≤ 1, got {}",
        llm.max_concurrent()
    );
}

/// Semaphore cap honored under pressure: with `concurrency=2` and 3 groups,
/// the LLM calls overlap (peak > 1) but never exceed the cap of 2.
#[test]
fn concurrent_semaphore_cap_two_with_three_groups() {
    let plan = make_three_screen_plan();
    let mut req = make_req();
    req.concurrency = 2; // 3 groups → effective_concurrency = 2 (cap < group count)

    let groups = group_subtasks_by_screen(&plan.subtasks);
    let llm = CountingLlm::new(vec![NODE_JSON.into(), NODE_JSON.into(), NODE_JSON.into()]);

    let abort = AbortFlag::new();
    let snapshot = EditorState::new();
    let mut real_sink = BufferDocSink::new(snapshot.clone());

    let _outcomes = block_on(run_concurrent(
        &groups,
        &plan,
        &req,
        &llm,
        &abort,
        snapshot,
        &mut real_sink,
        &mut |_| {},
        None,
    ));

    let peak = llm.max_concurrent();
    assert!(
        peak >= 2,
        "concurrency=2 should let 2 workers overlap, got peak {peak}"
    );
    assert!(
        peak <= 2,
        "concurrency=2 cap must not be exceeded, got peak {peak}"
    );
}

/// Replay order: real sink receives commands in plan-index order.
/// We use distinct JSON payloads so we can identify which subtask each command
/// came from by looking at the first node's id in the InsertSubtree.
#[test]
fn concurrent_replay_is_in_plan_index_order() {
    let plan = make_two_screen_plan();
    let req = make_req();
    let groups = group_subtasks_by_screen(&plan.subtasks);

    let s0_json = r#"[{"type":"frame","id":"s0-node","name":"S0","x":0,"y":0,"width":10,"height":10,"children":[{"type":"text","id":"s0-title","content":"S0","fontSize":18}]}]"#;
    let s1_json = r#"[{"type":"frame","id":"s1-node","name":"S1","x":0,"y":0,"width":20,"height":20,"children":[{"type":"text","id":"s1-title","content":"S1","fontSize":18}]}]"#;

    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(s0_json.into()),
        ScriptResponse::Text(s1_json.into()),
    ]);

    let abort = AbortFlag::new();
    let snapshot = EditorState::new();
    let mut real_sink = BufferDocSink::new(snapshot.clone());

    let _outcomes = block_on(run_concurrent(
        &groups,
        &plan,
        &req,
        &llm,
        &abort,
        snapshot,
        &mut real_sink,
        &mut |_| {},
        None,
    ));

    assert_eq!(real_sink.commands.len(), 2);

    // First command's nodes come from s0 (plan index 0).
    match &real_sink.commands[0] {
        EditorCommand::InsertSubtree { nodes, .. } => {
            if !nodes.is_empty() {
                let first_id = nodes[0].id_str();
                assert!(
                    first_id.starts_with("s0"),
                    "first command should be from s0, got id={first_id}"
                );
            }
        }
        other => panic!("expected InsertSubtree, got {other:?}"),
    }
    // Second command's nodes come from s1 (plan index 1).
    match &real_sink.commands[1] {
        EditorCommand::InsertSubtree { nodes, .. } => {
            if !nodes.is_empty() {
                let first_id = nodes[0].id_str();
                assert!(
                    first_id.starts_with("s1"),
                    "second command should be from s1, got id={first_id}"
                );
            }
        }
        other => panic!("expected InsertSubtree, got {other:?}"),
    }
}

/// Abort flag set before run_concurrent → all outcomes are None, no LLM calls.
#[test]
fn concurrent_aborts_before_any_work() {
    let plan = make_two_screen_plan();
    let req = make_req();
    let groups = group_subtasks_by_screen(&plan.subtasks);
    let llm = ScriptedLlm::new(vec![]);

    let abort = AbortFlag::new();
    abort.set();

    let snapshot = EditorState::new();
    let mut real_sink = BufferDocSink::new(snapshot.clone());
    let mut started_ids: Vec<String> = Vec::new();

    let outcomes = block_on(run_concurrent(
        &groups,
        &plan,
        &req,
        &llm,
        &abort,
        snapshot,
        &mut real_sink,
        &mut |p| {
            if let Progress::SubtaskStarted { id, .. } = &p {
                started_ids.push(id.clone());
            }
        },
        None,
    ));

    // All outcomes are None (aborted before any subtask ran).
    assert!(
        outcomes.iter().all(|o| o.is_none()),
        "aborted run should produce only None outcomes"
    );
    // No commands replayed.
    assert_eq!(real_sink.commands.len(), 0);
}

/// E5: `MergeAppState` rides the `BufferDocSink`→replay path and resolves
/// deterministically.
///
/// Two screen groups each own a `BufferDocSink` (exactly as `run_concurrent`
/// constructs them).  Group A buffers `MergeAppState { plan_idx: 0, count: 10 }`
/// (lower index, must win) and group B buffers `MergeAppState { plan_idx: 1,
/// count: 20 }` (higher index, must lose).
///
/// The test then replicates the sort-then-drain loop from `concurrent.rs:445-458`:
/// groups are sorted by their minimum `plan_idx` so the group with the lowest
/// index drains first, regardless of the physical order the workers finished.
///
/// Both physical finish orders (A-then-B and B-then-A) are tested; in both cases
/// the final `doc.state["count"]` must equal 10.
///
/// The test would fail if:
/// - `BufferDocSink::apply` dropped `MergeAppState` instead of buffering it, OR
/// - `apply_command_with_reveal` failed to forward `MergeAppState` to the sink, OR
/// - `EditorState::merge_app_state` ignored `plan_idx` (last-write-wins → 20 for
///   B-then-A drain order).
#[test]
fn merge_app_state_buffers_and_replays_deterministically() {
    use jian_ops_schema::state::{PrimitiveType, StateEntry, StateType};
    use std::collections::BTreeMap;

    let snapshot = EditorState::new();

    let entry = |d: i64| StateEntry {
        kind: StateType::Primitive(PrimitiveType::Int),
        default: Some(serde_json::json!(d)),
        description: None,
        persist: None,
    };

    // Group A: plan_idx=0, count=10  (lower index, must win the conflict)
    let make_buffer_a = || {
        let mut buf = BufferDocSink::new(snapshot.clone());
        let mut m = BTreeMap::new();
        m.insert("count".into(), entry(10));
        buf.apply(EditorCommand::MergeAppState {
            plan_idx: 0,
            state: m,
        });
        (0usize, buf) // (min_plan_idx, buffer)
    };
    // Group B: plan_idx=1, count=20  (higher index, must lose)
    let make_buffer_b = || {
        let mut buf = BufferDocSink::new(snapshot.clone());
        let mut m = BTreeMap::new();
        m.insert("count".into(), entry(20));
        buf.apply(EditorCommand::MergeAppState {
            plan_idx: 1,
            state: m,
        });
        (1usize, buf)
    };

    // Replicate the sort-then-drain loop from concurrent.rs:445-458.
    // `groups_with_min_idx` is [(min_plan_idx, BufferDocSink)] in the physical
    // finish order that came out of join_all; we sort by min_plan_idx then drain.
    let replay = |mut groups_with_min_idx: Vec<(usize, BufferDocSink)>| -> serde_json::Value {
        groups_with_min_idx.sort_by_key(|(min_idx, _)| *min_idx);
        let mut real_sink = VecDocSink::new();
        for (_min_idx, mut buf) in groups_with_min_idx {
            for cmd in buf.commands.drain(..) {
                apply_command_with_reveal(&mut real_sink, cmd, None, 0);
            }
        }
        real_sink
            .state
            .doc
            .state
            .as_ref()
            .expect("doc.state must be set after MergeAppState")
            .get("count")
            .expect("key 'count' must be present")
            .default
            .clone()
            .expect("default value must be set")
    };

    // Physical finish order A-then-B: group A (plan_idx=0) appears first in
    // worker_results — sort keeps this order, A drains first.
    let result_ab = replay(vec![make_buffer_a(), make_buffer_b()]);
    // Physical finish order B-then-A: group B (plan_idx=1) appears first —
    // sort reverses this, A still drains first because its min_plan_idx is lower.
    let result_ba = replay(vec![make_buffer_b(), make_buffer_a()]);

    let expected = serde_json::json!(10);
    assert_eq!(
        result_ab, expected,
        "A-then-B: lower plan_idx (0) must win, got {result_ab}"
    );
    assert_eq!(
        result_ba, expected,
        "B-then-A: lower plan_idx (0) must still win, got {result_ba}"
    );
    // Substantive check: the losing value (20) must NOT be the final state.
    assert_ne!(
        result_ba,
        serde_json::json!(20),
        "B-then-A must not produce the losing value 20"
    );
}

/// Progress fan-in: all 4 events (2 started + 2 done) arrive after run_concurrent.
#[test]
fn concurrent_progress_fanin_all_events_arrive() {
    let plan = make_two_screen_plan();
    let req = make_req();
    let groups = group_subtasks_by_screen(&plan.subtasks);
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(NODE_JSON.into()),
        ScriptResponse::Text(NODE_JSON.into()),
    ]);

    let abort = AbortFlag::new();
    let snapshot = EditorState::new();
    let mut real_sink = BufferDocSink::new(snapshot.clone());
    let mut events: Vec<String> = Vec::new();

    block_on(run_concurrent(
        &groups,
        &plan,
        &req,
        &llm,
        &abort,
        snapshot,
        &mut real_sink,
        &mut |p| match &p {
            Progress::SubtaskStarted { id, .. } => events.push(format!("start:{id}")),
            Progress::SubtaskDone { id, .. } => events.push(format!("done:{id}")),
            _ => {}
        },
        None,
    ));

    assert_eq!(
        events.len(),
        4,
        "expected 4 progress events, got {events:?}"
    );
    assert!(events.contains(&"start:s0".to_string()));
    assert!(events.contains(&"done:s0".to_string()));
    assert!(events.contains(&"start:s1".to_string()));
    assert!(events.contains(&"done:s1".to_string()));
}
