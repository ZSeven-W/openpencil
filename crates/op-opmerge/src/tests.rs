//! Tests for the structured `.op` three-way merge.

use std::collections::HashMap;

use super::*;

/// A two-node document — `pages[0].children` = `n1`, `n2`, each a
/// rect carrying an `x`. `(x1, x2)` set the two rects' `x` values.
fn doc(x1: i64, x2: i64) -> String {
    format!(
        r#"{{"version":"1.0","pages":[{{"id":"p1","name":"Page 1","children":[
           {{"id":"n1","type":"rect","name":"Rect A","x":{x1}}},
           {{"id":"n2","type":"rect","name":"Rect B","x":{x2}}}
        ]}}]}}"#
    )
}

/// Find the node object with `id` anywhere in `doc`.
fn find<'a>(doc: &'a Value, id: &str) -> Option<&'a Value> {
    match doc {
        Value::Object(map) => {
            if map.get("id") == Some(&Value::String(id.to_string())) {
                return Some(doc);
            }
            for key in ["children", "pages"] {
                if let Some(Value::Array(children)) = map.get(key) {
                    for child in children {
                        if let Some(found) = find(child, id) {
                            return Some(found);
                        }
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// The `x` of node `id` in `doc`.
fn node_x(doc: &Value, id: &str) -> i64 {
    find(doc, id).unwrap().get("x").unwrap().as_i64().unwrap()
}

#[test]
fn no_changes_merges_cleanly() {
    let r = merge_op_documents(&doc(0, 0), &doc(0, 0), &doc(0, 0)).unwrap();
    assert!(r.is_clean());
}

#[test]
fn remote_only_change_is_auto_merged() {
    // `theirs` moved n1 (0 → 10); `ours` left it. The merge takes
    // the remote value with no conflict.
    let r = merge_op_documents(&doc(0, 5), &doc(0, 5), &doc(10, 5)).unwrap();
    assert!(r.is_clean(), "a one-sided change is not a conflict");
    assert_eq!(node_x(&r.merged, "n1"), 10, "remote change applied");
    assert_eq!(node_x(&r.merged, "n2"), 5, "untouched node kept");
}

#[test]
fn local_only_change_keeps_ours() {
    // Only `ours` moved n1 — kept, no conflict, no remote override.
    let r = merge_op_documents(&doc(0, 0), &doc(7, 0), &doc(0, 0)).unwrap();
    assert!(r.is_clean());
    assert_eq!(node_x(&r.merged, "n1"), 7);
}

#[test]
fn identical_change_on_both_sides_is_clean() {
    // Both branches moved n1 to the same place — no conflict.
    let r = merge_op_documents(&doc(0, 0), &doc(9, 0), &doc(9, 0)).unwrap();
    assert!(r.is_clean());
    assert_eq!(node_x(&r.merged, "n1"), 9);
}

#[test]
fn divergent_change_to_one_node_conflicts() {
    // `ours` moved n1 → 5, `theirs` → 10: a real per-node conflict.
    // n2 is untouched and stays clean.
    let r = merge_op_documents(&doc(0, 0), &doc(5, 0), &doc(10, 0)).unwrap();
    assert_eq!(r.conflicts.len(), 1);
    assert_eq!(r.conflicts[0].id, "n1");
    assert_eq!(r.conflicts[0].kind, NodeConflictKind::BothModified);
    assert_eq!(r.conflicts[0].label, "Rect A");
    // The merged tree keeps the local value until the user resolves.
    assert_eq!(node_x(&r.merged, "n1"), 5);
}

#[test]
fn independent_changes_to_different_nodes_both_merge() {
    // `ours` moved n1, `theirs` moved n2 — disjoint, so both land
    // with no conflict.
    let r = merge_op_documents(&doc(0, 0), &doc(5, 0), &doc(0, 8)).unwrap();
    assert!(r.is_clean(), "disjoint node edits merge cleanly");
    assert_eq!(node_x(&r.merged, "n1"), 5);
    assert_eq!(node_x(&r.merged, "n2"), 8);
}

#[test]
fn delete_versus_modify_conflicts() {
    // `theirs` dropped n2; `ours` kept it → a delete/modify conflict.
    let base = doc(0, 0);
    let ours = doc(0, 3);
    let theirs = r#"{"version":"1.0","pages":[{"id":"p1","name":"Page 1","children":[
        {"id":"n1","type":"rect","name":"Rect A","x":0}
    ]}]}"#;
    let r = merge_op_documents(&base, &ours, theirs).unwrap();
    assert_eq!(r.conflicts.len(), 1);
    assert_eq!(r.conflicts[0].id, "n2");
    assert_eq!(r.conflicts[0].kind, NodeConflictKind::DeleteModify);
}

#[test]
fn node_added_only_on_the_remote_branch_conflicts() {
    let base = doc(0, 0);
    let ours = doc(0, 0);
    let theirs = r#"{"version":"1.0","pages":[{"id":"p1","name":"Page 1","children":[
        {"id":"n1","type":"rect","name":"Rect A","x":0},
        {"id":"n2","type":"rect","name":"Rect B","x":0},
        {"id":"n3","type":"rect","name":"Rect C","x":0}
    ]}]}"#;
    let r = merge_op_documents(&base, &ours, theirs).unwrap();
    assert_eq!(r.conflicts.len(), 1);
    assert_eq!(r.conflicts[0].id, "n3");
    assert_eq!(r.conflicts[0].kind, NodeConflictKind::AddedOnRemote);
}

#[test]
fn both_branches_add_the_same_id_differently() {
    let base = doc(0, 0);
    let with_n3 = |x: i64| {
        format!(
            r#"{{"version":"1.0","pages":[{{"id":"p1","name":"Page 1","children":[
               {{"id":"n1","type":"rect","name":"Rect A","x":0}},
               {{"id":"n2","type":"rect","name":"Rect B","x":0}},
               {{"id":"n3","type":"rect","name":"Rect C","x":{x}}}
            ]}}]}}"#
        )
    };
    let r = merge_op_documents(&base, &with_n3(1), &with_n3(2)).unwrap();
    assert_eq!(r.conflicts.len(), 1);
    assert_eq!(r.conflicts[0].id, "n3");
    assert_eq!(r.conflicts[0].kind, NodeConflictKind::BothAdded);
}

#[test]
fn one_sided_child_reorder_is_surfaced_not_dropped() {
    // `theirs` swaps the order of n1 / n2 under the page. A reorder
    // is a structural change — it must surface as a conflict, never
    // be silently reported clean.
    let reordered = r#"{"version":"1.0","pages":[{"id":"p1","name":"Page 1","children":[
        {"id":"n2","type":"rect","name":"Rect B","x":0},
        {"id":"n1","type":"rect","name":"Rect A","x":0}
    ]}]}"#;
    let r = merge_op_documents(&doc(0, 0), &doc(0, 0), reordered).unwrap();
    assert_eq!(r.conflicts.len(), 1);
    assert_eq!(r.conflicts[0].id, "p1");
    assert_eq!(r.conflicts[0].kind, NodeConflictKind::StructuralChange);
}

#[test]
fn remote_reparent_is_surfaced_not_dropped() {
    // `theirs` moves nX from frame fA to frame fB. A reparent is a
    // structural move — it must surface, never merge silently clean.
    let tree = |under_a: bool| {
        let (a_kids, b_kids) = if under_a {
            (r#"{"id":"nX","type":"rect","x":0}"#, "")
        } else {
            ("", r#"{"id":"nX","type":"rect","x":0}"#)
        };
        format!(
            r#"{{"version":"1.0","pages":[{{"id":"p1","children":[
               {{"id":"fA","type":"frame","children":[{a_kids}]}},
               {{"id":"fB","type":"frame","children":[{b_kids}]}}
            ]}}]}}"#
        )
    };
    let r = merge_op_documents(&tree(true), &tree(true), &tree(false)).unwrap();
    assert!(!r.is_clean(), "a reparent must not merge silently clean");
    assert!(
        r.conflicts
            .iter()
            .any(|c| c.id == "nX" && c.kind == NodeConflictKind::StructuralChange),
        "the moved node is surfaced as a structural conflict"
    );
}

#[test]
fn duplicate_node_id_is_rejected() {
    let dup = r#"{"version":"1.0","pages":[{"id":"p1","children":[
        {"id":"dup","type":"rect","x":0},
        {"id":"dup","type":"rect","x":1}
    ]}]}"#;
    let err = merge_op_documents(&doc(0, 0), dup, &doc(0, 0)).unwrap_err();
    assert!(matches!(err, OpMergeError::DuplicateId("ours", _)));
}

#[test]
fn resolve_op_merge_applies_per_node_choices() {
    let (base, ours, theirs) = (doc(0, 0), doc(5, 0), doc(10, 0));
    // No choices → the n1 conflict surfaces unresolved.
    let unresolved = resolve_op_merge(&base, &ours, &theirs, &HashMap::new()).unwrap();
    assert_eq!(unresolved.conflicts.len(), 1);

    // Choose theirs for n1 → resolved, merged takes theirs' value.
    let take_theirs = HashMap::from([("n1".to_string(), true)]);
    let r = resolve_op_merge(&base, &ours, &theirs, &take_theirs).unwrap();
    assert!(r.is_clean(), "a decided conflict is no longer a conflict");
    assert_eq!(node_x(&r.merged, "n1"), 10);

    // Choose ours for n1 → resolved, merged keeps ours' value.
    let keep_ours = HashMap::from([("n1".to_string(), false)]);
    let r = resolve_op_merge(&base, &ours, &theirs, &keep_ours).unwrap();
    assert!(r.is_clean());
    assert_eq!(node_x(&r.merged, "n1"), 5);
}

#[test]
fn invalid_json_is_an_error_not_a_panic() {
    let err = merge_op_documents("{not json", &doc(0, 0), &doc(0, 0)).unwrap_err();
    assert!(matches!(err, OpMergeError::Parse("base", _)));
}

#[test]
fn nested_children_are_walked() {
    // A node nested two levels deep still merges per-id.
    let nested = |x: i64| {
        format!(
            r#"{{"version":"1.0","pages":[{{"id":"p1","children":[
               {{"id":"frame","type":"frame","children":[
                 {{"id":"deep","type":"rect","x":{x}}}
               ]}}
            ]}}]}}"#
        )
    };
    let r = merge_op_documents(&nested(0), &nested(0), &nested(4)).unwrap();
    assert!(r.is_clean());
    assert_eq!(node_x(&r.merged, "deep"), 4);
}
