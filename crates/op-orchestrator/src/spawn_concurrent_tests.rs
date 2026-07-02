//! Tests for `run_spawned_agents_concurrent` (Track-1 Step 5).
//!
//! Proves the loop's `spawn_agents` runs REAL sub-agent generation — N
//! distinct LLM calls, N subtrees inserted — and that concurrency is
//! genuinely overlapping AND bounded by the reused `clamp_concurrency` cap.
//!
//! Wired as a child `#[path] mod tests` of `spawn_concurrent`, so
//! `use super::*` resolves to that module.

use super::*;
use crate::test_support::{CountingLlm, ScriptResponse, ScriptedLlm, VecDocSink};
use futures::executor::block_on;
use op_editor_core::{EditorCommand, PenNodeExt};

/// A default `DesignRequest`; `concurrency` is irrelevant here (the spawn
/// path takes concurrency as an explicit arg) but the struct is required.
fn make_req() -> crate::types::DesignRequest {
    crate::types::DesignRequest {
        prompt: "test".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: true,
        visual_ref_enabled: false,
    }
}

/// One node-JSON response per agent — distinct ids so each subtree is
/// independently identifiable after insertion.
fn node_json(id: &str) -> String {
    format!(
        r#"[{{"type":"frame","id":"{id}-1","name":"Sec","x":0,"y":0,"width":400,"height":120,"children":[{{"type":"text","id":"{id}-t","content":"Hi","fontSize":18}}]}}]"#
    )
}

fn spec(id: &str) -> SpawnAgentSpec {
    SpawnAgentSpec {
        id: id.into(),
        label: id.into(),
        prompt: format!("design the {id} section"),
        parent_frame_id: None,
    }
}

/// The core Step-5 proof: spawn_agents drives the REAL subagent runner —
/// N LLM calls happen and N subtrees are inserted into the live document
/// (not a canned `{spawned, agentIds}` placeholder).
#[test]
fn run_spawned_agents_invokes_real_runner_and_inserts_n_subtrees() {
    // Three agents → three scripted, distinct responses.
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(node_json("a")),
        ScriptResponse::Text(node_json("b")),
        ScriptResponse::Text(node_json("c")),
    ]);
    let mut sink = VecDocSink::new();
    let specs = vec![spec("a"), spec("b"), spec("c")];

    let results = block_on(run_spawned_agents_concurrent(
        &specs,
        &make_req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        3,
        None,
    ));

    // One structured result per agent, all real (non-zero node_count).
    assert_eq!(results.len(), 3);
    for r in &results {
        assert!(r.error.is_none(), "agent {} failed: {:?}", r.id, r.error);
        assert_eq!(r.node_count, 1, "each agent inserts one section root");
        assert_eq!(r.inserted_root_ids.len(), 1);
    }

    // N real InsertSubtree commands hit the document — not a no-op ack.
    let inserts = sink
        .applied
        .iter()
        .filter(|c| matches!(c, EditorCommand::InsertSubtree { .. }))
        .count();
    assert_eq!(inserts, 3, "three subtrees inserted into the live document");

    // The live document carries three top-level section frames.
    assert_eq!(
        sink.state().active_children().len(),
        3,
        "document has three inserted roots"
    );
}

/// Concurrency is GENUINE: with a cap ≥ N, `CountingLlm` observes more than
/// one in-flight call — the workers' LLM calls overlap (vs. strictly
/// sequential, which would cap at 1).
#[test]
fn run_spawned_agents_overlaps_calls_when_cap_allows() {
    let llm = CountingLlm::new(vec![node_json("a"), node_json("b"), node_json("c")]);
    let mut sink = VecDocSink::new();
    let specs = vec![spec("a"), spec("b"), spec("c")];

    let results = block_on(run_spawned_agents_concurrent(
        &specs,
        &make_req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        3, // cap ≥ 3 → all three may overlap
        None,
    ));

    assert_eq!(results.len(), 3);
    assert!(
        llm.max_concurrent() > 1,
        "calls must genuinely overlap with a cap ≥ N (got max_concurrent={})",
        llm.max_concurrent()
    );
}

/// Concurrency is BOUNDED by the reused `clamp_concurrency` cap: with cap=1
/// the calls run strictly one-at-a-time (`max_concurrent == 1`), proving the
/// semaphore — not an unbounded fan-out — gates the workers.
#[test]
fn run_spawned_agents_respects_concurrency_cap_of_one() {
    let llm = CountingLlm::new(vec![node_json("a"), node_json("b"), node_json("c")]);
    let mut sink = VecDocSink::new();
    let specs = vec![spec("a"), spec("b"), spec("c")];

    let results = block_on(run_spawned_agents_concurrent(
        &specs,
        &make_req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        1, // cap = 1 → strictly sequential
        None,
    ));

    assert_eq!(results.len(), 3);
    assert_eq!(
        llm.max_concurrent(),
        1,
        "cap=1 must serialize the workers (semaphore-bounded, not unbounded)"
    );
    // Even serialized, all three subtrees still land.
    assert_eq!(sink.state().active_children().len(), 3);
}

/// The `concurrency` arg is clamped through the SAME `clamp_concurrency`
/// `[1,6]` cap the orchestrator uses: an over-large request never produces an
/// unbounded fan-out. With 7 specs and a requested concurrency of 99, at most
/// 6 can be in-flight at once.
#[test]
fn run_spawned_agents_clamps_concurrency_to_six() {
    let responses: Vec<String> = (0..7).map(|i| node_json(&format!("s{i}"))).collect();
    let llm = CountingLlm::new(responses);
    let mut sink = VecDocSink::new();
    let specs: Vec<SpawnAgentSpec> = (0..7).map(|i| spec(&format!("s{i}"))).collect();

    let results = block_on(run_spawned_agents_concurrent(
        &specs,
        &make_req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        99, // way over the [1,6] cap
        None,
    ));

    assert_eq!(results.len(), 7);
    assert!(
        llm.max_concurrent() <= 6,
        "concurrency must be clamped to ≤ 6 (got {})",
        llm.max_concurrent()
    );
    assert!(
        llm.max_concurrent() > 1,
        "but still genuinely concurrent (got {})",
        llm.max_concurrent()
    );
}

/// Buffered commands are replayed in SPEC order (deterministic), so the
/// inserted roots appear in the live document in the order the model named
/// the agents — never torn or reordered by which LLM call finished first.
#[test]
fn run_spawned_agents_replays_in_spec_order() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(node_json("first")),
        ScriptResponse::Text(node_json("second")),
    ]);
    let mut sink = VecDocSink::new();
    let specs = vec![spec("first"), spec("second")];

    let results = block_on(run_spawned_agents_concurrent(
        &specs,
        &make_req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        2,
        None,
    ));

    assert_eq!(results[0].id, "first");
    assert_eq!(results[1].id, "second");
    // The InsertSubtree commands appear in spec order.
    let insert_ids: Vec<String> = sink
        .applied
        .iter()
        .filter_map(|c| match c {
            EditorCommand::InsertSubtree { nodes, .. } => {
                nodes.first().map(|n| n.id_str().to_string())
            }
            _ => None,
        })
        .collect();
    assert_eq!(insert_ids, vec!["first-1", "second-1"]);
}

/// A subtree is scoped to its spec's `parent_frame_id`: when a spec names a
/// container, the produced nodes insert UNDER that container, not at the page
/// root.
#[test]
fn run_spawned_agents_scopes_subtree_to_parent_container() {
    // Seed a container frame to insert under.
    let mut sink = VecDocSink::new();
    let container_json: Vec<jian_ops_schema::node::PenNode> = serde_json::from_str(
        r#"[{"type":"frame","id":"container","name":"Shell","width":1200,"height":800,"children":[]}]"#,
    )
    .unwrap();
    sink.apply(EditorCommand::InsertSubtree {
        nodes: container_json,
        parent_id: op_editor_core::NodeId::NONE,
        page_id: None,
    });

    // `InsertSubtree` REMAPS ids on apply, so the container's real id in the
    // live document is NOT "container" — read it back (this mirrors the loop,
    // where the model gets real ids from `get_editor_state`).
    let container_id = sink.state().active_children()[0].id_str().to_string();

    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(node_json("child"))]);
    let mut s = SpawnAgentSpec {
        id: "child".into(),
        label: "child".into(),
        prompt: "fill the shell".into(),
        parent_frame_id: Some(container_id),
    };
    // borrow-check: pass as a slice
    let specs = std::slice::from_mut(&mut s);

    let results = block_on(run_spawned_agents_concurrent(
        specs,
        &make_req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        1,
        None,
    ));

    assert_eq!(results.len(), 1);
    assert!(results[0].error.is_none(), "{:?}", results[0].error);
    assert_eq!(
        results[0].inserted_root_ids.len(),
        1,
        "the nested subtree's real root id is captured at replay"
    );
    // The container now has a child; the page root still has exactly one
    // top-level node (the container) — the subtree did NOT land at the root.
    assert_eq!(
        sink.state().active_children().len(),
        1,
        "no new top-level root — child nested under the container"
    );
    let container = &sink.state().active_children()[0];
    assert!(
        container.children().map(|c| !c.is_empty()).unwrap_or(false),
        "the container must have received the inserted subtree"
    );
}

/// Empty spec list → empty result (no LLM calls, no document mutation).
#[test]
fn run_spawned_agents_empty_specs_is_noop() {
    let llm = ScriptedLlm::new(vec![]);
    let mut sink = VecDocSink::new();
    let results = block_on(run_spawned_agents_concurrent(
        &[],
        &make_req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        4,
        None,
    ));
    assert!(results.is_empty());
    assert!(
        sink.applied.is_empty(),
        "no commands applied for empty specs"
    );
}
