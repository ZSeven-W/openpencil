#![cfg(test)]

use crate::command::EditorCommand;
use crate::node_id::NodeId;
use crate::state::EditorState;
use crate::test_support::frame;
use jian_ops_schema::state::{PrimitiveType, StateEntry, StateType};
use std::collections::BTreeMap;

fn entry(d: i64) -> StateEntry {
    StateEntry {
        kind: StateType::Primitive(PrimitiveType::Int),
        default: Some(serde_json::json!(d)),
        description: None,
        persist: None,
    }
}

// (a) Additive merge: new keys from MergeAppState appear in doc.state.
#[test]
fn merge_app_state_adds_new_keys() {
    let mut s = EditorState::new();
    let mut m = BTreeMap::new();
    m.insert("score".into(), entry(42));
    assert!(s.apply(EditorCommand::MergeAppState {
        plan_idx: 0,
        state: m,
    }));
    let state = s.doc.state.as_ref().unwrap();
    assert_eq!(
        state.get("score").unwrap().default,
        Some(serde_json::json!(42))
    );
}

// (b) Pre-existing doc-root key is NEVER overwritten (old .op file compat).
//
// Contract change (deliberate): every incoming key being skipped is a
// legitimate additive no-op, not a failure — `apply` now reports `true`
// ("command processed") even though the doc-owned value didn't change.
// The old `!s.apply(...)` assertion encoded the wrong contract: `Batch`
// rolls back on the first `false` sub-command, so a `false` here used
// to reject an otherwise-valid insert/replace riding in the same batch
// whenever a regenerated section declared a key the document already
// owned (see `merge_app_state`'s doc comment in `command_apply.rs`).
#[test]
fn merge_app_state_does_not_overwrite_existing_key() {
    let mut s = EditorState::new();
    let mut existing: BTreeMap<String, StateEntry> = BTreeMap::new();
    existing.insert("owned".into(), entry(99));
    s.doc.state = Some(existing);

    let mut m = BTreeMap::new();
    m.insert("owned".into(), entry(0));
    // Apply returns true — the merge was processed (nothing to add is
    // the designed outcome), but the doc-owned value must be untouched.
    assert!(s.apply(EditorCommand::MergeAppState {
        plan_idx: 0,
        state: m,
    }));
    let state = s.doc.state.as_ref().unwrap();
    assert_eq!(
        state.get("owned").unwrap().default,
        Some(serde_json::json!(99)),
        "pre-existing key must survive"
    );
}

// (c) Order-independence: applying [0, 2, 1] yields the same doc-state as [0, 1, 2].
// (d) Tracing-warn path (conflict) does not panic.
#[test]
fn merge_app_state_is_additive_lower_plan_idx_wins_order_independent() {
    // Pre-existing doc-root key that must never be overwritten.
    let mut existing: BTreeMap<String, StateEntry> = BTreeMap::new();
    existing.insert("owned".into(), entry(99));

    let run = |order: &[(usize, i64)]| {
        let mut st = EditorState::default();
        st.doc.state = Some(existing.clone());
        for &(plan_idx, def) in order {
            let mut m = BTreeMap::new();
            m.insert("owned".into(), entry(def)); // collides w/ pre-existing
            m.insert("count".into(), entry(def)); // generation-added, conflicts across subtasks
                                                  // Return value means "processed", not "changed" — it stays
                                                  // true even when a higher plan_idx loses to a previously
                                                  // registered owner (a legitimate no-op, not a failure).
            let _ = st.apply(EditorCommand::MergeAppState { plan_idx, state: m });
        }
        let s = st.doc.state.clone().unwrap();
        (
            s.get("owned").unwrap().default.clone(),
            s.get("count").unwrap().default.clone(),
        )
    };
    // Pre-existing "owned" survives; generation "count" = lower plan_idx (1) wins.
    let in_order = run(&[(1, 10), (2, 20)]);
    let reversed = run(&[(2, 20), (1, 10)]);
    assert_eq!(
        in_order.0,
        Some(serde_json::json!(99)),
        "pre-existing key kept"
    );
    assert_eq!(
        in_order.1,
        Some(serde_json::json!(10)),
        "lower plan_idx wins"
    );
    assert_eq!(in_order, reversed, "merge must be order-independent");
}

// Empty incoming state is a no-op — processed successfully (returns
// true), doc left untouched. Same contract-change rationale as
// `merge_app_state_does_not_overwrite_existing_key` above: an empty
// merge is not an invalid command, so it must not be able to sink a
// `Batch` it happens to ride in.
#[test]
fn merge_app_state_empty_incoming_is_noop() {
    let mut s = EditorState::new();
    assert!(s.apply(EditorCommand::MergeAppState {
        plan_idx: 0,
        state: BTreeMap::new(),
    }));
    assert!(s.doc.state.is_none());
}

// Regression for the codex BLOCKER: a no-op merge (every incoming key
// already doc-owned) must report `true` so a `Batch` carrying it
// alongside a real insert does NOT roll the whole batch back. Before
// the fix, `merge_app_state` returned `false` here, `cmd_batch` saw
// that as sub-command #1 failing, and rolled back — discarding the
// insert too, even though the insert itself was perfectly valid. This
// is exactly the shape `op-mcp`'s `with_hoisted_state` builds on every
// generation path (`insert_node` / `replace_node` / `design_content` /
// `design_skeleton` / `batch_design` operations-with-state).
#[test]
fn noop_merge_is_success_and_batch_survives() {
    let mut s = EditorState::new();
    let mut existing: BTreeMap<String, StateEntry> = BTreeMap::new();
    existing.insert("owned".into(), entry(99));
    s.doc.state = Some(existing);

    let mut m = BTreeMap::new();
    m.insert("owned".into(), entry(0)); // same key the doc already owns — pure no-op
    let before = s.active_children().len();
    assert!(
        s.apply(EditorCommand::Batch {
            commands: vec![
                EditorCommand::MergeAppState {
                    plan_idx: 0,
                    state: m,
                },
                EditorCommand::InsertSubtree {
                    nodes: vec![frame("f1", "Card", 0.0, 0.0, 100.0, 100.0, vec![])],
                    parent_id: NodeId::NONE,
                    page_id: None,
                },
            ],
        }),
        "a no-op merge must not sink the batch"
    );
    assert_eq!(
        s.active_children().len(),
        before + 1,
        "the insert must have landed"
    );
    assert_eq!(
        s.doc.state.as_ref().unwrap().get("owned").unwrap().default,
        Some(serde_json::json!(99)),
        "doc-owned value must be unchanged by the no-op merge"
    );
}

// (e) A rolled-back batch must not leave stale ownership: the failed
// batch's MergeAppState never landed in doc.state, so a later merge of
// the same key (any plan_idx) must land instead of being skipped
// against a phantom owner.
#[test]
fn rolled_back_batch_leaves_no_stale_app_state_ownership() {
    let mut s = EditorState::new();
    let mut m = BTreeMap::new();
    m.insert("cart".into(), entry(1));
    let failed = s.apply(EditorCommand::Batch {
        commands: vec![
            EditorCommand::MergeAppState {
                plan_idx: 0,
                state: m,
            },
            // Batchable but fails: no node with this id exists.
            EditorCommand::SetNodeText {
                node_id: crate::node_id::NodeId::new("no-such-node".to_string()),
                text: "x".into(),
            },
        ],
    });
    assert!(
        !failed,
        "batch with a failing sub-command must report false"
    );
    assert!(
        s.doc
            .state
            .as_ref()
            .is_none_or(|st| !st.contains_key("cart")),
        "rolled-back merge must not survive in doc.state"
    );

    let mut retry = BTreeMap::new();
    retry.insert("cart".into(), entry(7));
    assert!(
        s.apply(EditorCommand::MergeAppState {
            plan_idx: 9,
            state: retry,
        }),
        "post-rollback merge must land — stale ownership would skip it"
    );
    assert_eq!(
        s.doc.state.as_ref().unwrap().get("cart").unwrap().default,
        Some(serde_json::json!(7))
    );
}

// (f) Undoing a batch restores the ownership map alongside doc.state,
// so a re-generation after undo starts from a clean slate.
#[test]
fn undo_restores_app_state_ownership_with_the_document() {
    let mut s = EditorState::new();
    let mut m = BTreeMap::new();
    m.insert("tab".into(), entry(3));
    assert!(s.apply(EditorCommand::Batch {
        commands: vec![EditorCommand::MergeAppState {
            plan_idx: 2,
            state: m,
        }],
    }));
    assert!(s.doc.state.as_ref().unwrap().contains_key("tab"));

    assert!(s.undo(), "batch lands as one undo step");
    assert!(
        s.doc
            .state
            .as_ref()
            .is_none_or(|st| !st.contains_key("tab")),
        "undo must remove the merged key from doc.state"
    );

    let mut again = BTreeMap::new();
    again.insert("tab".into(), entry(8));
    assert!(
        s.apply(EditorCommand::MergeAppState {
            plan_idx: 5,
            state: again,
        }),
        "post-undo merge must land — ownership must have been restored"
    );
    assert_eq!(
        s.doc.state.as_ref().unwrap().get("tab").unwrap().default,
        Some(serde_json::json!(8))
    );
}
