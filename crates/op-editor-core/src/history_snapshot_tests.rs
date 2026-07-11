//! Correctness suite for the structurally-shared undo history
//! ([`crate::history_snapshot`]).
//!
//! Covers:
//!   - a reference-model property/equivalence suite: random
//!     edit/undo/redo/batch-collapse sequences compared step-by-step
//!     against a full-clone reference history (documents equal at every
//!     step) for both the single-page and multi-page models;
//!   - `materialize(capture(doc)) == doc` round-trips;
//!   - `Arc` sharing (`ptr_eq`) across consecutive snapshots of an
//!     unchanged subtree AND across the undo/redo anchor transitions;
//!   - component-prototype sharing;
//!   - the `repair_swap` copy-on-write path proven not to contaminate a
//!     sibling snapshot that shares the same `Arc`;
//!   - the save→edit→undo (clean) / save→undo→divergent-edit (dirty)
//!     revision + `sync_dirty_flag` interplay, incl. the batch-collapse
//!     pre-batch revision reset.

#![cfg(test)]

use crate::history::HISTORY_CAP;
use crate::history_snapshot::SharedDoc;
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::test_support::{frame, rect, state_with, text};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::page::PenPage;
use jian_ops_schema::PenDocument;
use std::sync::Arc;

// --- Fixtures + deterministic PRNG -----------------------------------

/// xorshift64 — deterministic, dependency-free.
fn next_rand(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// A handful of top-level frames, each holding a couple of leaves — so
/// edits can touch a top-level entry, a deep leaf, or the sibling list.
fn many_roots(n: usize) -> Vec<PenNode> {
    (0..n)
        .map(|i| {
            frame(
                &format!("f{i}"),
                "F",
                (i as f64) * 20.0,
                0.0,
                100.0,
                80.0,
                vec![
                    rect(&format!("f{i}_r"), "R", 0.0, 0.0, 10.0, 10.0),
                    text(&format!("f{i}_t"), "T", 0.0, 20.0, 40.0, 12.0, "hi"),
                ],
            )
        })
        .collect()
}

/// The editable top-level list — page 0's children when paged, else the
/// root `children`.
fn editable(doc: &mut PenDocument) -> &mut Vec<PenNode> {
    if let Some(pages) = doc.pages.as_mut() {
        if let Some(p) = pages.get_mut(0) {
            return &mut p.children;
        }
    }
    &mut doc.children
}

/// Apply one deterministic edit to `doc`, driven by `seed`. Identical
/// input on two equal documents keeps them equal — the property the
/// suite relies on.
fn edit_doc(doc: &mut PenDocument, seed: u64) {
    let kind = seed % 5;
    let children = editable(doc);
    match kind {
        0 => {
            // Nudge a top-level node's x.
            if !children.is_empty() {
                let i = (seed >> 3) as usize % children.len();
                let b = children[i].base_mut();
                b.x = Some(b.x.unwrap_or(0.0) + 1.0);
            }
        }
        1 => {
            // Rename a top-level node.
            if !children.is_empty() {
                let i = (seed >> 3) as usize % children.len();
                children[i].base_mut().name = Some(format!("r{}", seed % 1000));
            }
        }
        2 => {
            // Mutate a deep leaf.
            if !children.is_empty() {
                let i = (seed >> 3) as usize % children.len();
                if let Some(sub) = children[i].children_mut() {
                    if !sub.is_empty() {
                        let j = (seed >> 8) as usize % sub.len();
                        sub[j].base_mut().name = Some(format!("d{}", seed % 1000));
                    }
                }
            }
        }
        3 => {
            // Add a fresh top-level node (unique id).
            let id = format!("gen{seed}");
            children.push(rect(&id, "Gen", (seed % 100) as f64, 0.0, 10.0, 10.0));
        }
        _ => {
            // Remove a top-level node (keep at least one).
            if children.len() > 1 {
                let i = (seed >> 3) as usize % children.len();
                children.remove(i);
            }
        }
    }
}

/// Full-clone reference history — the oracle the shared implementation
/// must match. Mirrors the editor's semantics: `commit` parks the
/// pre-edit document, undo/redo swap through `past`/`future`.
struct Reference {
    doc: PenDocument,
    past: Vec<PenDocument>,
    future: Vec<PenDocument>,
}

impl Reference {
    fn new(doc: PenDocument) -> Self {
        Reference {
            doc,
            past: Vec::new(),
            future: Vec::new(),
        }
    }
    fn commit(&mut self) {
        self.past.push(self.doc.clone());
        if self.past.len() > HISTORY_CAP {
            self.past.remove(0);
        }
        self.future.clear();
    }
    fn undo(&mut self) -> bool {
        if let Some(prev) = self.past.pop() {
            self.future.push(self.doc.clone());
            self.doc = prev;
            true
        } else {
            false
        }
    }
    fn redo(&mut self) -> bool {
        if let Some(next) = self.future.pop() {
            self.past.push(self.doc.clone());
            self.doc = next;
            true
        } else {
            false
        }
    }
}

// --- Property / equivalence suite ------------------------------------

fn run_equivalence(mut state: EditorState, mut seed: u64, steps: usize) {
    let mut reference = Reference::new(state.doc.clone());
    for step in 0..steps {
        match next_rand(&mut seed) % 6 {
            0 | 1 => {
                // Ordinary edit: commit the pre-edit state, then mutate.
                let s = next_rand(&mut seed);
                state.commit_history();
                edit_doc(&mut state.doc, s);
                reference.commit();
                edit_doc(&mut reference.doc, s);
            }
            2 => {
                assert_eq!(state.undo(), reference.undo(), "undo return @ step {step}");
            }
            3 => {
                assert_eq!(state.redo(), reference.redo(), "redo return @ step {step}");
            }
            _ => {
                // Batch-collapse: one pre-batch snapshot, several edits,
                // one parked history entry — the shape `cmd_batch`
                // collapses a successful program into.
                let k = 1 + (next_rand(&mut seed) % 3);
                let pre = state.snapshot_for_history();
                reference.commit();
                for _ in 0..k {
                    let s = next_rand(&mut seed);
                    edit_doc(&mut state.doc, s);
                    edit_doc(&mut reference.doc, s);
                }
                state.history_push_past(pre);
            }
        }
        assert_eq!(
            state.doc, reference.doc,
            "document diverged from full-clone reference @ step {step}"
        );
        assert_eq!(
            state.history.past.len(),
            reference.past.len(),
            "undo depth diverged @ step {step}"
        );
        assert_eq!(
            state.history.future.len(),
            reference.future.len(),
            "redo depth diverged @ step {step}"
        );
    }
}

#[test]
fn equivalence_single_page_matches_full_clone_reference() {
    for seed in [0x1234_5678u64, 0xdead_beef, 0x0f0f_0f0f, 42] {
        let state = state_with(many_roots(6));
        run_equivalence(state, seed, 400);
    }
}

#[test]
fn equivalence_multi_page_matches_full_clone_reference() {
    for seed in [0xa5a5_a5a5u64, 0x1111_2222, 7] {
        let mut state = state_with(Vec::new());
        state.doc.pages = Some(vec![
            PenPage {
                id: "p0".into(),
                name: "One".into(),
                children: many_roots(4),
                state: None,
                lifecycle: None,
            },
            PenPage {
                id: "p1".into(),
                name: "Two".into(),
                children: many_roots(3),
                state: None,
                lifecycle: None,
            },
        ]);
        run_equivalence(state, seed, 300);
    }
}

#[test]
fn materialize_round_trips_the_document() {
    let mut state = state_with(many_roots(5));
    state.doc.name = Some("Doc".into());
    let shared = SharedDoc::capture(&state.doc, None);
    assert_eq!(shared.materialize(), state.doc);

    // Round-trip after an anchored re-capture too.
    let mut edited = state.doc.clone();
    edit_doc(&mut edited, 3);
    let shared2 = SharedDoc::capture(&edited, Some(&shared));
    assert_eq!(shared2.materialize(), edited);
}

// --- Arc sharing (ptr_eq) --------------------------------------------

#[test]
fn unchanged_top_level_subtrees_share_arcs_across_snapshots() {
    let mut state = state_with(many_roots(5));
    let snap1 = SharedDoc::capture(&state.doc, None);

    // Change exactly one top-level entry (f2's deep leaf).
    {
        let sub = state.doc.children[2].children_mut().unwrap();
        sub[0].base_mut().name = Some("changed".into());
    }
    let snap2 = SharedDoc::capture(&state.doc, Some(&snap1));

    let a = snap1.root_children();
    let b = snap2.root_children();
    assert_eq!(a.len(), b.len());
    for i in 0..a.len() {
        if i == 2 {
            assert!(
                !Arc::ptr_eq(&a[i], &b[i]),
                "the changed top-level entry must be a fresh Arc"
            );
        } else {
            assert!(
                Arc::ptr_eq(&a[i], &b[i]),
                "unchanged entry {i} must reuse the anchor's Arc"
            );
        }
    }
}

#[test]
fn undo_redo_transitions_share_arcs_with_the_destination() {
    let mut state = state_with(many_roots(5));
    // Two edits so there is something to undo.
    state.commit_history();
    edit_doc(&mut state.doc, 0); // nudge f0.x
    state.commit_history();
    edit_doc(&mut state.doc, 2); // change a deep leaf of some frame

    // Undo: the parked redo entry is captured anchored on the popped
    // destination, so its unchanged subtrees share that destination's
    // Arcs.
    let dest = state.history.past.back().unwrap().clone();
    assert!(state.undo());
    let redo_entry = state.history.future.back().unwrap();
    let d = dest.doc.root_children();
    let r = redo_entry.doc.root_children();
    let shared = (0..d.len()).filter(|&i| Arc::ptr_eq(&d[i], &r[i])).count();
    assert!(
        shared >= d.len() - 1,
        "undo's parked entry should share all-but-the-changed subtree with the destination (shared={shared}/{})",
        d.len()
    );

    // Redo is symmetric.
    let dest2 = state.history.future.back().unwrap().clone();
    assert!(state.redo());
    let undo_entry = state.history.past.back().unwrap();
    let d2 = dest2.doc.root_children();
    let u = undo_entry.doc.root_children();
    let shared2 = (0..d2.len())
        .filter(|&i| Arc::ptr_eq(&d2[i], &u[i]))
        .count();
    assert!(
        shared2 >= d2.len() - 1,
        "redo's parked entry should share all-but-the-changed subtree with the destination"
    );
}

#[test]
fn unchanged_component_prototypes_share_arcs() {
    use crate::components::{Component, ComponentLibrary};
    use crate::history_snapshot::SharedComponents;

    let mut lib = ComponentLibrary::default();
    lib.insert(Component {
        id: NodeId::new("c0"),
        name: "A".into(),
        root: rect("c0", "A", 0.0, 0.0, 10.0, 10.0),
    });
    lib.insert(Component {
        id: NodeId::new("c1"),
        name: "B".into(),
        root: rect("c1", "B", 0.0, 0.0, 10.0, 10.0),
    });
    let snap1 = SharedComponents::capture(&lib, None);

    // Change only c1.
    lib.rename(&NodeId::new("c1"), "B2");
    let snap2 = SharedComponents::capture(&lib, Some(&snap1));

    let a = snap1.components();
    let b = snap2.components();
    assert!(
        Arc::ptr_eq(&a[0], &b[0]),
        "unchanged prototype c0 must share"
    );
    assert!(
        !Arc::ptr_eq(&a[1], &b[1]),
        "changed prototype c1 must be fresh"
    );
}

// --- COW isolation (repair_swap) -------------------------------------

fn ref_node(id: &str, target: &str) -> PenNode {
    serde_json::from_value(serde_json::json!({
        "type": "ref", "id": id, "ref": target
    }))
    .expect("ref node parses")
}

#[test]
fn repair_swap_is_copy_on_write_and_does_not_contaminate_a_shared_sibling_snapshot() {
    // A doc whose top-level entry "inst1" is a NON-Ref node — the
    // contamination signature a scope-time snapshot would carry.
    let mut doc = PenDocument {
        version: "0.8.0".into(),
        ..empty()
    };
    doc.children = vec![
        rect("keep", "Keep", 0.0, 0.0, 10.0, 10.0),
        rect("inst1", "DisplayNode", 0.0, 0.0, 10.0, 10.0),
    ];

    let snap_a = SharedDoc::capture(&doc, None);
    // snap_b shares both entries with snap_a (anchored, no change).
    let mut snap_b = SharedDoc::capture(&doc, Some(&snap_a));
    assert!(Arc::ptr_eq(
        &snap_a.root_children()[0],
        &snap_b.root_children()[0]
    ));
    assert!(Arc::ptr_eq(
        &snap_a.root_children()[1],
        &snap_b.root_children()[1]
    ));

    // Repair snap_b: swap the contaminated "inst1" for a real Ref.
    let replacement = ref_node("inst1", "card");
    snap_b.repair_swap(&NodeId::new("inst1"), &replacement);

    // snap_b now holds the Ref; snap_a is untouched (still the rect).
    let b_inst = &snap_b.root_children()[1];
    assert!(
        matches!(b_inst.as_ref(), PenNode::Ref(_)),
        "snap_b repaired"
    );
    let a_inst = &snap_a.root_children()[1];
    assert!(
        matches!(a_inst.as_ref(), PenNode::Rectangle(_)),
        "sibling snapshot NOT contaminated by the COW repair"
    );
    assert!(
        !Arc::ptr_eq(a_inst, b_inst),
        "make_mut must have cloned the contaminated entry away from the sibling"
    );
    // The untouched "keep" entry keeps its shared Arc — repair only
    // clones the entry it must.
    assert!(
        Arc::ptr_eq(&snap_a.root_children()[0], &snap_b.root_children()[0]),
        "an entry without the target id keeps its shared Arc"
    );
}

fn empty() -> PenDocument {
    PenDocument {
        version: "0.8.0".into(),
        name: None,
        themes: None,
        variables: None,
        pages: None,
        children: Vec::new(),
        format_version: None,
        id: None,
        app: None,
        routes: None,
        state: None,
        lifecycle: None,
        logic_modules: None,
        design_md: None,
        conversion: None,
    }
}

// --- Dirty-flag / revision interplay ---------------------------------

#[test]
fn save_then_edit_then_undo_reports_clean() {
    let mut state = state_with(many_roots(3));
    state.mark_saved_revision(); // baseline clean
    assert!(!state.is_dirty());

    state.commit_history(); // parks pre-edit snapshot @ saved revision, bumps revision
    edit_doc(&mut state.doc, 0);
    assert!(state.is_dirty(), "an edit makes the document dirty");

    assert!(state.undo());
    assert!(
        !state.is_dirty(),
        "undo back to the saved snapshot restores its revision → clean"
    );
    assert!(state.editor_ui.document_dirty == state.is_dirty());
}

#[test]
fn save_then_undo_then_divergent_edit_reports_dirty() {
    let mut state = state_with(many_roots(3));
    // Edit A, then save at that revision.
    state.commit_history();
    edit_doc(&mut state.doc, 0);
    state.mark_saved_revision();
    assert!(!state.is_dirty());

    // Edit B, then undo back to the saved (A) revision → clean.
    state.commit_history();
    edit_doc(&mut state.doc, 1);
    assert!(state.undo());
    assert!(!state.is_dirty(), "undo to the saved revision reads clean");

    // A divergent edit must NOT reuse the saved revision value — the
    // monotonic counter guards against a false-clean report.
    state.commit_history();
    edit_doc(&mut state.doc, 3);
    assert!(
        state.is_dirty(),
        "a divergent edit after undo-to-saved must read dirty"
    );
}

#[test]
fn batch_collapse_resets_pre_batch_revision_before_the_bump() {
    use crate::command::EditorCommand;

    let mut state = crate::test_support::sample();
    state.mark_saved_revision();
    let saved = state.saved_revision();

    // A successful batch lands as ONE history entry whose parked
    // snapshot carries the PRE-batch revision, and the live revision is
    // bumped exactly once past it.
    assert!(state.apply(EditorCommand::Batch {
        commands: vec![
            EditorCommand::NudgeSelected { dx: 1, dy: 0 },
            EditorCommand::NudgeSelected { dx: 0, dy: 1 },
        ],
    }));
    assert!(state.is_dirty());
    assert_eq!(state.history.past.len(), 1, "batch collapsed to one entry");
    // Undo restores the parked pre-batch revision → clean again.
    assert!(state.undo());
    assert_eq!(state.document_revision(), saved);
    assert!(!state.is_dirty());
}
