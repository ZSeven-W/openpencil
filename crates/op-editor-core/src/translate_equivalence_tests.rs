//! Equivalence oracle for Task 10's `translate_selected` rewrite.
//!
//! `translate_selected` used to run three full-tree walks PER selected
//! id (`is_flow_child_of_flex` / `is_ancestor_in_set` / `find_node_mut`)
//! — O(|selection| × nodes), near-quadratic on select-all. It was
//! replaced with a single recursive walk
//! ([`walkers::translate_editable_subtree`]) that threads both skip
//! conditions down the recursion instead of recomputing them from
//! scratch per id.
//!
//! [`reference_translate_selected`] below is a byte-for-byte port of the
//! OLD three-walk body, kept here ONLY as a test oracle (production code
//! never calls it). Every case constructs a document, clones the
//! `EditorState`, runs the reference implementation on one clone and
//! the live `EditorState::translate_selected` on the other with an
//! IDENTICAL selection + delta, and asserts the resulting `doc`s are
//! byte-for-byte equal.

#![cfg(test)]

use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::test_support::{flex_frame, flow_rect, frame, group, rect, state_with};
use crate::walkers::{self, find_node_mut};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::page::PenPage;

/// Byte-for-byte port of `translate_selected`'s pre-Task-10 body — three
/// full-tree walks per selected id. See the module doc: this exists
/// ONLY to pin down old behavior as an equivalence oracle.
fn reference_translate_selected(state: &mut EditorState, dx: f64, dy: f64) -> bool {
    if state.selection.set.is_empty() || (dx == 0.0 && dy == 0.0) {
        return false;
    }
    let editable: Vec<NodeId> = state
        .selection
        .set
        .iter()
        .filter(|id| state.is_editable(id))
        .cloned()
        .collect();
    if editable.is_empty() {
        return false;
    }
    let children = state.active_children_mut();
    let mut moved = false;
    for target in &editable {
        if walkers::is_flow_child_of_flex(children, target) {
            continue;
        }
        if !walkers::is_ancestor_in_set(children, target, &editable) {
            if let Some(node) = find_node_mut(children, target) {
                walkers::translate_subtree(node, dx, dy);
                moved = true;
            }
        }
    }
    moved
}

/// Force every container-capable node's `children` field to `Some(_)`
/// up front (recursively). This exists ONLY to neutralize an
/// orthogonal, pre-existing quirk of the SHARED `find_node_mut` walker
/// (used by the old reference body): its search calls
/// `PenNodeExt::children_mut()` on every non-matching node it visits
/// while looking for a target, and `children_mut()` eagerly upgrades
/// a container-capable node's `children: None` to `Some(vec![])`
/// (`Option::get_or_insert_with`) as a side effect — even for nodes
/// that turn out to be unrelated to the search. That upgrade depends
/// on find_node_mut's data-dependent visitation order (which siblings
/// it happens to scan before finding a match), not on the actual
/// translate/skip/dedup semantics this test exists to pin down. The
/// single-pass rewrite deliberately visits every node once during its
/// own descent (see `translate_editable_subtree`), so without this
/// normalization the two implementations would disagree on which
/// untouched leaves flip from `None` to `Some(vec![])` — a distinction
/// that carries no geometric meaning (an absent vs. empty children
/// list behaves identically everywhere else) but would still trip
/// `PenDocument`'s derived `PartialEq`. Normalizing BOTH clones to the
/// same starting shape keeps the comparison scoped to what actually
/// matters: node positions and the `moved` verdict.
fn normalize_children(nodes: &mut [PenNode]) {
    for node in nodes.iter_mut() {
        if let Some(children) = node.children_mut() {
            normalize_children(children);
        }
    }
}

/// Runs both implementations from identical clones of `state` with an
/// identical selection + delta, then asserts they land on the same
/// document AND the same `moved` verdict.
fn assert_equivalent(mut state: EditorState, selected: &[&str], dx: f64, dy: f64) {
    state.selection.set = selected.iter().map(|id| NodeId::new(*id)).collect();
    state.selection.anchor = state.selection.set.last().cloned().unwrap_or(NodeId::NONE);
    normalize_children(&mut state.doc.children);
    if let Some(pages) = state.doc.pages.as_mut() {
        for page in pages.iter_mut() {
            normalize_children(&mut page.children);
        }
    }

    let mut reference = state.clone();
    let mut actual = state;

    let ref_moved = reference_translate_selected(&mut reference, dx, dy);
    let actual_moved = actual.translate_selected(dx, dy);

    assert_eq!(
        ref_moved, actual_moved,
        "moved verdict diverged for selection {selected:?}"
    );
    assert_eq!(
        reference.doc, actual.doc,
        "resulting document diverged for selection {selected:?}"
    );
}

#[test]
fn equivalence_single_leaf() {
    assert_equivalent(
        state_with(vec![rect("n1", "A", 10.0, 10.0, 50.0, 50.0)]),
        &["n1"],
        7.0,
        3.0,
    );
}

#[test]
fn equivalence_nested_containers_ancestor_and_descendant_dedup() {
    // n1 (frame) > n2 (frame) > n3 (rect) — select all three. The
    // ancestor dedup must fire twice: n2 is skipped because n1 (its
    // ancestor) is selected, n3 is skipped because n2 (its ancestor,
    // even though itself skipped) is selected.
    let doc = frame(
        "n1",
        "Outer",
        0.0,
        0.0,
        200.0,
        200.0,
        vec![frame(
            "n2",
            "Inner",
            10.0,
            10.0,
            100.0,
            100.0,
            vec![rect("n3", "Leaf", 5.0, 5.0, 20.0, 20.0)],
        )],
    );
    assert_equivalent(state_with(vec![doc]), &["n1", "n2", "n3"], 5.0, -4.0);
}

#[test]
fn equivalence_descendant_only_no_ancestor_selected() {
    // Same tree, but only the deepest leaf is selected — no dedup
    // applies, the leaf itself must translate.
    let doc = frame(
        "n1",
        "Outer",
        0.0,
        0.0,
        200.0,
        200.0,
        vec![frame(
            "n2",
            "Inner",
            10.0,
            10.0,
            100.0,
            100.0,
            vec![rect("n3", "Leaf", 5.0, 5.0, 20.0, 20.0)],
        )],
    );
    assert_equivalent(state_with(vec![doc]), &["n3"], 2.0, 2.0);
}

#[test]
fn equivalence_flex_parent_and_flow_children() {
    // A flex frame plus its two flow children, with EVERY id selected
    // at once: the frame moves (top-level, no ancestor), the flow
    // children are skipped because their immediate parent is flex.
    let doc = flex_frame(
        "f1",
        "Flex",
        100.0,
        100.0,
        200.0,
        300.0,
        vec![
            flow_rect("c1", "A", 80.0, 24.0),
            flow_rect("c2", "B", 80.0, 24.0),
        ],
    );
    assert_equivalent(state_with(vec![doc]), &["f1", "c1", "c2"], 5.0, 7.0);
}

#[test]
fn equivalence_flex_child_selected_alone() {
    let doc = flex_frame(
        "f1",
        "Flex",
        0.0,
        0.0,
        200.0,
        300.0,
        vec![flow_rect("c1", "A", 80.0, 24.0)],
    );
    assert_equivalent(state_with(vec![doc]), &["c1"], 9.0, 11.0);
}

#[test]
fn equivalence_locked_node_in_selection_is_excluded() {
    let mut locked = rect("n2", "Locked", 60.0, 60.0, 30.0, 30.0);
    locked.base_mut().locked = Some(true);
    let doc = frame(
        "n1",
        "Frame",
        0.0,
        0.0,
        200.0,
        200.0,
        vec![rect("n3", "Free", 10.0, 10.0, 20.0, 20.0), locked],
    );
    // n1 stays free-standing (not selected) so n3's translate isn't
    // deduped; n2 is locked and must be excluded from the editable set
    // entirely (no translate, no dedup contribution).
    assert_equivalent(state_with(vec![doc]), &["n2", "n3"], 4.0, -6.0);
}

#[test]
fn equivalence_hidden_ancestor_does_not_dedupe_a_selected_child() {
    // The ancestor is selected but HIDDEN — `is_editable` excludes it
    // from the editable set, so it must not count as an "ancestor in
    // set" for its selected, visible child either.
    let mut hidden_parent = frame(
        "n1",
        "Hidden",
        0.0,
        0.0,
        200.0,
        200.0,
        vec![rect("n2", "Child", 10.0, 10.0, 20.0, 20.0)],
    );
    hidden_parent.base_mut().visible = Some(false);
    assert_equivalent(state_with(vec![hidden_parent]), &["n1", "n2"], 3.0, 3.0);
}

#[test]
fn equivalence_overlapping_selection_across_disjoint_branches() {
    // Two independent subtrees, each with an ancestor+descendant pair
    // selected, plus one lone top-level leaf. Exercises dedup running
    // independently per branch within a single pass.
    let branch_a = frame(
        "a1",
        "A",
        0.0,
        0.0,
        100.0,
        100.0,
        vec![rect("a2", "AChild", 5.0, 5.0, 10.0, 10.0)],
    );
    let branch_b = group(
        "b1",
        "B",
        vec![
            rect("b2", "BChild1", 5.0, 5.0, 10.0, 10.0),
            rect("b3", "BChild2", 20.0, 20.0, 10.0, 10.0),
        ],
    );
    let leaf = rect("c1", "Lone", 300.0, 300.0, 40.0, 40.0);
    assert_equivalent(
        state_with(vec![branch_a, branch_b, leaf]),
        &["a1", "a2", "b1", "b3", "c1"],
        -3.0,
        8.0,
    );
}

#[test]
fn equivalence_large_multi_selection_select_all() {
    // A wide, moderately deep forest with every node selected at once —
    // the near-quadratic case the single-pass rewrite targets. Mixes
    // top-level leaves, nested containers with multiple children each,
    // and one locked leaf to keep the editable-set filter exercised.
    let mut roots = Vec::new();
    let mut all_ids: Vec<String> = Vec::new();
    for i in 0..12u32 {
        let root_id = format!("root{i}");
        all_ids.push(root_id.clone());
        let mut mid_children = Vec::new();
        for j in 0..6u32 {
            let mid_id = format!("mid{i}_{j}");
            all_ids.push(mid_id.clone());
            let mut leaves = Vec::new();
            for k in 0..4u32 {
                let leaf_id = format!("leaf{i}_{j}_{k}");
                all_ids.push(leaf_id.clone());
                let mut leaf = rect(&leaf_id, "Leaf", k as f64, k as f64, 8.0, 8.0);
                // Lock exactly one leaf per mid-container so the editable
                // filter has real work to do at scale.
                if k == 3 {
                    leaf.base_mut().locked = Some(true);
                }
                leaves.push(leaf);
            }
            mid_children.push(frame(
                &mid_id,
                "Mid",
                j as f64 * 10.0,
                j as f64 * 10.0,
                60.0,
                60.0,
                leaves,
            ));
        }
        roots.push(frame(
            &root_id,
            "Root",
            i as f64 * 100.0,
            0.0,
            300.0,
            300.0,
            mid_children,
        ));
    }
    let selected: Vec<&str> = all_ids.iter().map(String::as_str).collect();
    assert_equivalent(state_with(roots), &selected, 11.0, -13.0);
}

#[test]
fn equivalence_multi_page_document_only_touches_the_active_page() {
    // A second, inactive page carries an id that COLLIDES with nothing
    // on the active page but would corrupt the result if either
    // implementation accidentally walked `doc.pages` instead of the
    // active page's children.
    let mut state = state_with(vec![]);
    state.doc.children.clear();
    state.doc.pages = Some(vec![
        PenPage {
            id: "p0".to_string(),
            name: "Page 0".to_string(),
            children: vec![rect("x1", "Other page leaf", 0.0, 0.0, 10.0, 10.0)],
            state: None,
            lifecycle: None,
        },
        PenPage {
            id: "p1".to_string(),
            name: "Page 1".to_string(),
            children: vec![frame(
                "n1",
                "Active page frame",
                20.0,
                20.0,
                100.0,
                100.0,
                vec![rect("n2", "Active page child", 5.0, 5.0, 10.0, 10.0)],
            )],
            state: None,
            lifecycle: None,
        },
    ]);
    state.ui.active_page_index = 1;
    assert_equivalent(state, &["n1", "n2"], 6.0, 2.0);
}
