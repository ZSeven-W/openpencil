//! Coarse perf regression guards for large documents. Not a real
//! benchmark (no criterion in the workspace yet) — just lower-bound
//! tests that assert common operations stay reasonable on a 1000+
//! node document. Tightening the bounds later is an alternative to
//! adding criterion.
//!
//! The gap report flagged that "1000+ node documents render but
//! pan / zoom is not benchmarked vs Electron". These tests at
//! least pin the apex hot paths so future patches can't silently
//! 10× them.

use openpencil_shell_core::document::{Document, Node, NodeKind};
use openpencil_shell_core::{Point2D, Rect};
use std::time::Instant;

/// Build a Document with `n` leaf rect nodes spread across a grid
/// at the active page root. Used as the perf workload below.
fn build_doc_with_n_leaves(n: u64) -> Document {
    let mut doc = Document::empty();
    let page = doc.pages.first_mut().expect("default page");
    let cols = 32u64;
    for i in 1..=n {
        let row = (i - 1) / cols;
        let col = (i - 1) % cols;
        let mut node = Node::leaf(format!("n{i}"), NodeKind::Rect, format!("rect{i}"));
        node.bounds = Rect::xywh(col as f32 * 24.0, row as f32 * 24.0, 20.0, 20.0);
        page.children.push(node);
    }
    doc
}

#[test]
fn max_node_id_on_1000_leaves_is_under_50ms() {
    let doc = build_doc_with_n_leaves(1000);
    let t = Instant::now();
    let mut acc: u64 = 0;
    for _ in 0..100 {
        acc = acc.wrapping_add(doc.max_node_id());
    }
    let elapsed_ms = t.elapsed().as_secs_f64() * 1000.0;
    // 100 walks should easily finish in well under 50 ms even on
    // debug builds (the walk is O(node_count) and each is a few
    // i64 comparisons). The bound is generous on purpose — its
    // job is to catch a future 10× regression, not to measure
    // wall-clock precision.
    assert!(
        elapsed_ms < 50.0,
        "max_node_id × 100 took {elapsed_ms:.1} ms on 1000-leaf doc; \
         expected < 50 ms (debug build). Investigate the walker."
    );
    // Touch `acc` so the optimizer doesn't elide the walks.
    assert!(acc > 0);
}

#[test]
fn node_at_doc_point_on_1000_leaves_under_25ms_per_hit_batch() {
    let doc = build_doc_with_n_leaves(1000);
    let t = Instant::now();
    let mut hits: u64 = 0;
    // Spray 100 points across the populated grid; each call walks
    // top-most-first until a hit. Worst case is the empty space
    // (full walk).
    for i in 0..100 {
        let x = (i * 7 % 32) as f32 * 24.0 + 5.0;
        let y = (i * 11 % 32) as f32 * 24.0 + 5.0;
        if doc.node_at_doc_point(Point2D::new(x, y)).is_some() {
            hits += 1;
        }
    }
    let elapsed_ms = t.elapsed().as_secs_f64() * 1000.0;
    assert!(
        elapsed_ms < 25.0,
        "node_at_doc_point × 100 took {elapsed_ms:.1} ms on 1000-leaf doc; \
         expected < 25 ms (debug build). The hit-test must stay top-most-first \
         + bail on first match."
    );
    assert!(hits > 0, "at least one of the 100 sprayed points must hit");
}

#[test]
fn apply_batch_insert_of_1000_descriptors_under_100ms() {
    // Phase 5: the MCP write path now applies `EditorCommand` through
    // `op_editor_core::EditorState::apply`, not the old shell-core
    // `Document::apply_mcp_command`.
    use op_editor_core::{BatchInsertItem, EditorCommand, EditorState};
    let mut state = EditorState::new();
    let mut items = Vec::with_capacity(1000);
    for i in 0..1000 {
        let row = i / 32;
        let col = i % 32;
        items.push(BatchInsertItem {
            kind: "rect".into(),
            name: format!("item{i}"),
            x: col as i32 * 24,
            y: row as i32 * 24,
            width: 20,
            height: 20,
            fill_hex: None,
        });
    }
    let cmd = EditorCommand::BatchInsert { items };
    let t = Instant::now();
    assert!(state.apply(cmd));
    let elapsed_ms = t.elapsed().as_secs_f64() * 1000.0;
    assert!(
        elapsed_ms < 100.0,
        "BatchInsert of 1000 leaves took {elapsed_ms:.1} ms; expected < 100 ms \
         (debug build). Investigate id allocation + Vec growth in command_node."
    );
    assert_eq!(state.active_children().len(), 1000);
}
