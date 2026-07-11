//! Pathological benchmarks for the structurally-shared undo history.
//!
//! No `criterion` dependency — each test measures wall time with
//! `Instant` and asserts a *relation* that holds with a comfortable
//! margin, plus a deterministic structural check (Arc-sharing counts)
//! that proves the memory behaviour independent of timing noise. The
//! four cases mirror the design's cost analysis:
//!
//!   1. many top-level nodes, one changed — sharing must reuse all but
//!      the changed subtree, and a re-capture of an UNCHANGED doc must
//!      beat a fresh (all-clone) capture;
//!   2. one huge top-level frame with a deep leaf changed early vs late
//!      — the acknowledged worst case: no sharing plus one compare;
//!   3. large reordered sibling lists — the id-map fallback must still
//!      recover full sharing despite the index fast path missing;
//!   4. a large component prototype changed — one fresh prototype, the
//!      rest shared.

#![cfg(test)]

use crate::components::{Component, ComponentLibrary};
use crate::history_snapshot::{SharedComponents, SharedDoc};
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::test_support::{frame, rect, text};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::PenDocument;
use std::sync::Arc;
use std::time::Instant;

fn empty_doc() -> PenDocument {
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

fn small_frame(i: usize) -> PenNode {
    frame(
        &format!("f{i}"),
        "F",
        0.0,
        0.0,
        100.0,
        80.0,
        vec![
            rect(&format!("f{i}_r"), "R", 0.0, 0.0, 10.0, 10.0),
            text(&format!("f{i}_t"), "T", 0.0, 20.0, 40.0, 12.0, "hi"),
        ],
    )
}

/// A deeply / widely nested single frame — `width` leaves at `depth`.
fn huge_frame(width: usize, depth: usize) -> PenNode {
    let leaves: Vec<PenNode> = (0..width)
        .map(|i| rect(&format!("leaf{i}"), "L", i as f64, 0.0, 5.0, 5.0))
        .collect();
    let mut node = frame("huge", "Huge", 0.0, 0.0, 1000.0, 1000.0, leaves);
    for d in 0..depth {
        node = frame(
            &format!("wrap{d}"),
            "W",
            0.0,
            0.0,
            1000.0,
            1000.0,
            vec![node],
        );
    }
    node
}

#[test]
fn bench_many_top_level_one_changed() {
    const N: usize = 1500;
    let mut doc = empty_doc();
    doc.children = (0..N).map(small_frame).collect();

    // Fresh capture (all Arcs allocated + all subtrees cloned).
    let t0 = Instant::now();
    let snap1 = SharedDoc::capture(&doc, None);
    let fresh = t0.elapsed();

    // Re-capture with ZERO changes: only equality walks + Arc bumps, no
    // deep clones. Must reuse every Arc and beat the fresh capture.
    let t1 = Instant::now();
    let snap_same = SharedDoc::capture(&doc, Some(&snap1));
    let reshare = t1.elapsed();
    let shared_all = (0..N)
        .filter(|&i| Arc::ptr_eq(&snap1.root_children()[i], &snap_same.root_children()[i]))
        .count();
    assert_eq!(shared_all, N, "an unchanged re-capture shares every Arc");

    // Change one deep leaf, re-capture: exactly one fresh Arc.
    doc.children[N / 2].children_mut().unwrap()[0]
        .base_mut()
        .name = Some("changed".into());
    let snap2 = SharedDoc::capture(&doc, Some(&snap1));
    let shared = (0..N)
        .filter(|&i| Arc::ptr_eq(&snap1.root_children()[i], &snap2.root_children()[i]))
        .count();
    assert_eq!(shared, N - 1, "only the changed top-level subtree is fresh");

    eprintln!("bench_many_top_level: fresh={fresh:?} reshare(unchanged)={reshare:?}");
    assert!(
        reshare < fresh,
        "re-capturing an unchanged {N}-node doc (equality walk + Arc bumps) must beat a fresh all-clone capture: reshare={reshare:?} fresh={fresh:?}"
    );
}

#[test]
fn bench_one_huge_frame_deep_leaf_changed() {
    // One giant top-level entry — the acknowledged pathological case:
    // any change re-clones the whole entry AND pays a compare.
    let base = huge_frame(4000, 6);
    let mut doc = empty_doc();
    doc.children = vec![base];

    let snap1 = SharedDoc::capture(&doc, None);

    // Change an EARLY leaf vs a LATE leaf; both force a full re-clone of
    // the single top-level entry (no sharing possible).
    let mut early = doc.clone();
    rename_deep_leaf(&mut early.children[0], false, "early");
    let mut late = doc.clone();
    rename_deep_leaf(&mut late.children[0], true, "late");

    let t0 = Instant::now();
    let snap_early = SharedDoc::capture(&early, Some(&snap1));
    let te = t0.elapsed();
    let t1 = Instant::now();
    let snap_late = SharedDoc::capture(&late, Some(&snap1));
    let tl = t1.elapsed();

    // No sharing on the single huge entry, either way.
    assert!(!Arc::ptr_eq(
        &snap1.root_children()[0],
        &snap_early.root_children()[0]
    ));
    assert!(!Arc::ptr_eq(
        &snap1.root_children()[0],
        &snap_late.root_children()[0]
    ));
    // Materialization is still lossless.
    assert_eq!(snap_early.materialize(), early);
    assert_eq!(snap_late.materialize(), late);

    eprintln!("bench_huge_frame: early-change={te:?} late-change={tl:?}");
    // Both are the same order of magnitude (clone + one compare) — the
    // compare short-circuits, so neither should be pathologically worse.
    let (hi, lo) = if te > tl { (te, tl) } else { (tl, te) };
    assert!(
        hi.as_nanos() <= lo.as_nanos().saturating_mul(50).max(1),
        "early vs late change should stay the same order of magnitude: {te:?} vs {tl:?}"
    );
}

#[test]
fn bench_large_reordered_sibling_list() {
    const N: usize = 1200;
    let mut doc = empty_doc();
    doc.children = (0..N).map(small_frame).collect();
    let snap1 = SharedDoc::capture(&doc, None);

    // Reverse the sibling order: the index fast path misses on (almost)
    // every entry, so the id-map fallback must recover full sharing.
    doc.children.reverse();
    let t0 = Instant::now();
    let snap2 = SharedDoc::capture(&doc, Some(&snap1));
    let reorder = t0.elapsed();

    // Every entry is content-equal to some anchor entry (by id), so all
    // are shared despite the reorder.
    let mut shared = 0usize;
    for (i, node) in doc.children.iter().enumerate() {
        // snap2[i] corresponds to reversed node == snap1[N-1-i].
        let orig = &snap1.root_children()[N - 1 - i];
        if Arc::ptr_eq(orig, &snap2.root_children()[i]) {
            shared += 1;
        }
        assert_eq!(snap2.root_children()[i].id_str(), node.id_str());
    }
    eprintln!("bench_reorder: {N} nodes reordered, shared={shared}, t={reorder:?}");
    assert_eq!(
        shared, N,
        "the id-map fallback recovers full sharing under reorder"
    );
}

#[test]
fn bench_large_component_prototype_changed() {
    const N: usize = 400;
    let mut lib = ComponentLibrary::default();
    for i in 0..N {
        // Each prototype carries a chunky subtree.
        let root = frame(
            &format!("c{i}"),
            "Proto",
            0.0,
            0.0,
            200.0,
            200.0,
            (0..20)
                .map(|j| rect(&format!("c{i}_{j}"), "R", j as f64, 0.0, 5.0, 5.0))
                .collect(),
        );
        lib.insert(Component {
            id: NodeId::new(format!("c{i}")),
            name: format!("Proto{i}"),
            root,
        });
    }
    let snap1 = SharedComponents::capture(&lib, None);

    // Change one large prototype.
    lib.rename(&NodeId::new("c200"), "Renamed");
    let t0 = Instant::now();
    let snap2 = SharedComponents::capture(&lib, Some(&snap1));
    let dt = t0.elapsed();

    let shared = (0..N)
        .filter(|&i| Arc::ptr_eq(&snap1.components()[i], &snap2.components()[i]))
        .count();
    eprintln!("bench_component_proto: {N} protos, shared={shared}, t={dt:?}");
    assert_eq!(shared, N - 1, "only the changed prototype is re-allocated");
}

/// Descend `wrap` frames (following child 0) to the leaf list and
/// rename its first (or last) leaf. Recursive to sidestep the
/// reborrow-in-loop borrow-checker limitation.
fn rename_deep_leaf(node: &mut PenNode, last: bool, name: &str) {
    let Some(children) = node.children_mut() else {
        return;
    };
    if children.is_empty() {
        return;
    }
    if children[0].children().is_none() {
        // These are the leaves.
        let idx = if last { children.len() - 1 } else { 0 };
        children[idx].base_mut().name = Some(name.into());
    } else {
        rename_deep_leaf(&mut children[0], last, name);
    }
}
