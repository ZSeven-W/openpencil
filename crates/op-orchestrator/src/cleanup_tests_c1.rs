//! Task C1 (S3b-2) tests — `aggregate_concurrent_verdict` +
//! `cleanup_concurrent_roots`.
//!
//! Wired as `#[path = "cleanup_tests_c1.rs"] mod tests_c1;` inside
//! `cleanup.rs`; stays a child module of `cleanup`, so `use super::*`
//! resolves to `cleanup`.

use super::*;
use crate::test_support::VecDocSink;
use crate::types::{OrchestratorError, SubtaskOutcome};
use crate::variables::VarSnapshot;
use serde_json::json;

// ── Helpers ────────────────────────────────────────────────────────────────

fn outcome(node_count: usize, error: Option<&str>) -> SubtaskOutcome {
    SubtaskOutcome {
        id: "s0".into(),
        node_count,
        error: error.map(|s| s.to_string()),
    }
}

fn frame_json(id: &str, children: serde_json::Value) -> PenNode {
    serde_json::from_value(json!({
        "type": "frame", "id": id, "name": id,
        "x": 0, "y": 0, "width": 100, "height": 100,
        "children": children,
    }))
    .expect("frame json")
}

fn child_frame_json(id: &str) -> serde_json::Value {
    json!({
        "type": "frame", "id": id, "name": id,
        "x": 0, "y": 0, "width": 50, "height": 50,
        "children": [],
    })
}

// ── aggregate_concurrent_verdict ────────────────────────────────────────────

/// ALL outcomes zero nodes → Err carrying first error string.
#[test]
fn aggregate_all_zero_returns_err_with_first_error() {
    let outcomes = vec![
        outcome(0, Some("model timed out")),
        outcome(0, Some("invalid JSON")),
    ];
    let result = aggregate_concurrent_verdict(&outcomes);
    match result {
        Err(OrchestratorError::AllFailed(msg)) => {
            assert_eq!(msg, "model timed out", "should carry first non-empty error");
        }
        other => panic!("expected Err(AllFailed), got {other:?}"),
    }
}

/// All zero nodes, no error strings → fallback message.
#[test]
fn aggregate_all_zero_no_errors_returns_fallback_message() {
    let outcomes = vec![outcome(0, None), outcome(0, None)];
    let result = aggregate_concurrent_verdict(&outcomes);
    match result {
        Err(OrchestratorError::AllFailed(msg)) => {
            assert!(
                msg.contains("failed to generate"),
                "fallback message should describe failure, got: {msg}"
            );
        }
        other => panic!("expected Err(AllFailed), got {other:?}"),
    }
}

/// One group succeeds + one fails → partial success → Ok(()).
#[test]
fn aggregate_partial_success_returns_ok() {
    let outcomes = vec![
        outcome(3, None),        // group 0: success
        outcome(0, Some("err")), // group 1: failure
    ];
    let result = aggregate_concurrent_verdict(&outcomes);
    assert!(
        result.is_ok(),
        "partial success must be accepted, got: {result:?}"
    );
}

/// All outcomes have nodes → Ok(()).
#[test]
fn aggregate_all_succeed_returns_ok() {
    let outcomes = vec![outcome(2, None), outcome(1, None)];
    let result = aggregate_concurrent_verdict(&outcomes);
    assert!(result.is_ok());
}

/// Empty collected (all aborted before any subtask ran) → Ok(()).
#[test]
fn aggregate_empty_collected_returns_ok() {
    let result = aggregate_concurrent_verdict(&[]);
    assert!(
        result.is_ok(),
        "empty collected must be Ok, got: {result:?}"
    );
}

/// First error is picked even when the first outcome has None error but
/// a later outcome has Some — picks the FIRST non-empty.
#[test]
fn aggregate_picks_first_non_empty_error() {
    let outcomes = vec![
        outcome(0, None),           // no error
        outcome(0, Some("second")), // first non-empty
        outcome(0, Some("third")),
    ];
    let result = aggregate_concurrent_verdict(&outcomes);
    match result {
        Err(OrchestratorError::AllFailed(msg)) => {
            assert_eq!(msg, "second");
        }
        other => panic!("expected Err(AllFailed), got {other:?}"),
    }
}

// ── cleanup_concurrent_roots ──────────────────────────────────────────────

/// Scaffold-only root (descendant_count <= baseline) is deleted.
#[test]
fn cleanup_concurrent_roots_removes_scaffold_only_root() {
    let mut sink = VecDocSink::new();
    // Insert a root with 0 children (scaffold is empty desktop root).
    let tree = frame_json("root-a", json!([]));
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    // Get actual remapped ID (InsertSubtree remaps IDs).
    let root_id = sink.state.active_children()[0].id_str().to_string();
    sink.applied.clear();

    let snap = VarSnapshot::default();
    // baseline = 0 (desktop), descendant_count = 0 → should be deleted.
    cleanup_concurrent_roots(&mut sink, &[&root_id], &[0], &snap);

    let deletes: Vec<_> = sink
        .applied
        .iter()
        .filter(|c| matches!(c, EditorCommand::DeleteNode { .. }))
        .collect();
    assert_eq!(deletes.len(), 1, "scaffold-only root should be deleted");
}

/// Root with content (descendant_count > baseline) is NOT deleted.
#[test]
fn cleanup_concurrent_roots_keeps_root_with_content() {
    let mut sink = VecDocSink::new();
    // Insert a root with 1 content child (above baseline 0).
    let tree = frame_json("root-b", json!([child_frame_json("child-1")]));
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    // Get actual remapped ID.
    let root_id = sink.state.active_children()[0].id_str().to_string();
    sink.applied.clear();

    let snap = VarSnapshot::default();
    // baseline = 0, descendant_count = 1 → content survived → keep
    cleanup_concurrent_roots(&mut sink, &[&root_id], &[0], &snap);

    let deletes: Vec<_> = sink
        .applied
        .iter()
        .filter(|c| matches!(c, EditorCommand::DeleteNode { .. }))
        .collect();
    assert!(
        deletes.is_empty(),
        "root with content should NOT be deleted"
    );
}

/// N-root: one root has content, another is scaffold-only.
/// Only the scaffold-only root is deleted; variables are NOT rolled back
/// because content survived.
#[test]
fn cleanup_concurrent_n_roots_partial_content_keeps_variables() {
    let mut sink = VecDocSink::new();
    // Root A: scaffold-only (0 children).
    let tree_a = frame_json("root-a", json!([]));
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree_a],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    // capture actual remapped ID for root-a (InsertSubtree remaps IDs).
    let root_a = sink.state.active_children()[0].id_str().to_string();

    // Root B: has content child.
    let tree_b = frame_json("root-b", json!([child_frame_json("content-1")]));
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree_b],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    // root-b is the second child at index 1.
    let root_b = sink.state.active_children()[1].id_str().to_string();

    sink.applied.clear();

    let snap = VarSnapshot::default();
    cleanup_concurrent_roots(&mut sink, &[&root_a, &root_b], &[0, 0], &snap);

    let delete_ids: Vec<String> = sink
        .applied
        .iter()
        .filter_map(|c| {
            if let EditorCommand::DeleteNode { node_id, .. } = c {
                Some(node_id.as_str().to_string())
            } else {
                None
            }
        })
        .collect();
    // Only root-a should be deleted (it has 0 descendants, baseline 0).
    assert_eq!(
        delete_ids,
        vec![root_a.clone()],
        "only scaffold root deleted"
    );
    // root-b was not deleted.
    assert!(
        !delete_ids.contains(&root_b),
        "root with content should not be deleted"
    );

    // No variable rollback commands (empty snap → nothing to roll back).
    let var_deletes: Vec<_> = sink
        .applied
        .iter()
        .filter(|c| matches!(c, EditorCommand::DeleteVariable { .. }))
        .collect();
    assert!(
        var_deletes.is_empty(),
        "variables should NOT be rolled back when content survived"
    );
}

/// ALL roots are scaffold-only → all deleted + variables rolled back.
#[test]
fn cleanup_concurrent_all_scaffold_only_deletes_all_and_rolls_back_vars() {
    let mut sink = VecDocSink::new();
    // Two empty roots.
    let tree_a = frame_json("root-a", json!([]));
    let tree_b = frame_json("root-b", json!([]));
    for tree in [tree_a, tree_b] {
        sink.state.apply(EditorCommand::InsertSubtree {
            nodes: vec![tree],
            parent_id: NodeId::NONE,
            page_id: None,
        });
    }
    // Capture actual remapped IDs.
    let root_a = sink.state.active_children()[0].id_str().to_string();
    let root_b = sink.state.active_children()[1].id_str().to_string();
    sink.applied.clear();

    // A snapshot with one "created" variable — should be rolled back when
    // no content survived.
    let snap = VarSnapshot {
        created: vec!["color-primary".into()],
    };
    cleanup_concurrent_roots(&mut sink, &[&root_a, &root_b], &[0, 0], &snap);

    let cmds: Vec<String> = sink
        .applied
        .iter()
        .map(|c| match c {
            EditorCommand::DeleteNode { node_id, .. } => {
                format!("del:{}", node_id.as_str())
            }
            EditorCommand::DeleteVariable { name } => format!("delvar:{name}"),
            other => format!("other:{other:?}"),
        })
        .collect();

    // Both roots deleted (use actual IDs, not "root-a"/"root-b").
    assert!(
        cmds.iter().filter(|s| s.starts_with("del:")).count() == 2,
        "both roots should be deleted, cmds: {cmds:?}"
    );
    // Variable rolled back.
    assert!(
        cmds.contains(&"delvar:color-primary".to_string()),
        "variable should be rolled back when nothing survived, cmds: {cmds:?}"
    );
}

/// Mobile baseline = 1 (status bar): a root that only has the status bar child
/// (descendant_count == 1) is treated as scaffold-only and deleted.
#[test]
fn cleanup_concurrent_mobile_baseline_one_removes_status_bar_only_root() {
    let mut sink = VecDocSink::new();
    // Root with exactly 1 child (the status bar injected by scaffold).
    let tree = frame_json("root-mobile", json!([child_frame_json("status-bar")]));
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    // Get actual remapped ID.
    let root_id = sink.state.active_children()[0].id_str().to_string();
    sink.applied.clear();

    let snap = VarSnapshot::default();
    // baseline = 1 (mobile), descendant_count = 1 → scaffold-only → delete
    cleanup_concurrent_roots(&mut sink, &[&root_id], &[1], &snap);

    let deletes: Vec<_> = sink
        .applied
        .iter()
        .filter(|c| matches!(c, EditorCommand::DeleteNode { .. }))
        .collect();
    assert_eq!(
        deletes.len(),
        1,
        "mobile scaffold-only root (only status bar) should be deleted"
    );
}
