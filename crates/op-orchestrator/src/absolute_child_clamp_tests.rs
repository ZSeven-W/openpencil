//! `collect_absolute_child_clamp_fixes` — the "locate me" button repair.
//!
//! Fixtures mirror the measured failure: a fixed-size control pinned inside
//! a `layout: "none"` map block so it hangs past the block's right edge.
//! Resolved rects are hand-built at the parent's origin, the way the real
//! jian pass reports them.

use super::*;
use serde_json::json;

fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
    Rect { x, y, w, h }
}

/// A 375x320 `layout: "none"` map block (painted + rounded, so the clip
/// fallback would otherwise claim it) holding one 44x44 control.
fn map_with_control(control: Value) -> Value {
    json!({
        "type":"frame","id":"map","name":"Map","layout":"none",
        "width":375,"height":320,"cornerRadius":16,
        "fill":[{"type":"solid","color":"#1A2B3C"}],
        "children":[control]
    })
}

fn update_node_xy(cmds: &[EditorCommand], id: &str) -> Option<(Option<i32>, Option<i32>)> {
    cmds.iter().find_map(|cmd| match cmd {
        EditorCommand::UpdateNode { node_id, x, y, .. } if node_id.as_str() == id => Some((*x, *y)),
        _ => None,
    })
}

#[test]
fn control_past_the_right_edge_is_shifted_back_inside() {
    // The measured shape: a 44x44 control at x=340 inside a 375-wide map —
    // its right edge hangs 9px past the map's right edge.
    let map = map_with_control(json!({"type":"frame","id":"locate","name":"Locate me",
               "x":340,"y":260,"width":44,"height":44,"children":[]}));
    let rects = HashMap::from([
        ("map".to_string(), rect(0.0, 0.0, 375.0, 320.0)),
        ("locate".to_string(), rect(340.0, 260.0, 44.0, 44.0)),
    ]);
    let mut cmds = Vec::new();
    collect_absolute_child_clamp_fixes(&map, &rects, &mut cmds);
    // x = min(340, 375 - 44) = 331; y (260) already fits 320 - 44 = 276.
    assert_eq!(update_node_xy(&cmds, "locate"), Some((Some(331), None)));
    assert_eq!(cmds.len(), 1, "only the shift, no clip: {cmds:?}");
}

#[test]
fn control_at_a_negative_offset_is_clamped_to_zero() {
    let map = map_with_control(
        json!({"type":"frame","id":"locate","x":-8,"y":24,"width":44,"height":44,"children":[]}),
    );
    let rects = HashMap::from([
        ("map".to_string(), rect(0.0, 0.0, 375.0, 320.0)),
        ("locate".to_string(), rect(-8.0, 24.0, 44.0, 44.0)),
    ]);
    let mut cmds = Vec::new();
    collect_absolute_child_clamp_fixes(&map, &rects, &mut cmds);
    assert_eq!(update_node_xy(&cmds, "locate"), Some((Some(0), None)));
}

#[test]
fn control_wider_than_its_parent_is_left_to_the_clip_fallback() {
    // A child BIGGER than the parent can never be shifted inside: the clamp
    // stays out of it and the existing clip path still runs.
    let map = map_with_control(
        json!({"type":"frame","id":"wide","x":10,"y":10,"width":400,"height":44,"children":[]}),
    );
    let rects = HashMap::from([
        ("map".to_string(), rect(0.0, 0.0, 375.0, 320.0)),
        ("wide".to_string(), rect(10.0, 10.0, 400.0, 44.0)),
    ]);
    let mut cmds = Vec::new();
    collect_absolute_child_clamp_fixes(&map, &rects, &mut cmds);
    assert!(cmds.is_empty(), "oversized child untouched: {cmds:?}");

    collect_card_overflow_clips(&map, &rects, &mut cmds);
    assert!(
        cmds.iter().any(|cmd| matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp { node_id, property, .. }
                if node_id.as_str() == "map" && property == "clipContent"
        )),
        "clip fallback still fires for the oversized child: {cmds:?}"
    );
}

#[test]
fn control_in_a_flex_parent_is_untouched() {
    // A vertical parent owns its children's placement; an authored x/y there
    // is not this pass's contract.
    let column = json!({
        "type":"frame","id":"col","layout":"vertical","width":375,"height":320,
        "children":[
            {"type":"frame","id":"row","x":340,"y":260,"width":44,"height":44,"children":[]}
        ]
    });
    let rects = HashMap::from([
        ("col".to_string(), rect(0.0, 0.0, 375.0, 320.0)),
        ("row".to_string(), rect(340.0, 260.0, 44.0, 44.0)),
    ]);
    let mut cmds = Vec::new();
    collect_absolute_child_clamp_fixes(&column, &rects, &mut cmds);
    assert!(cmds.is_empty(), "flex child untouched: {cmds:?}");
}

#[test]
fn keyword_sized_control_is_untouched() {
    // No authored fixed box to preserve — left to the clip logic.
    let map = map_with_control(json!({"type":"frame","id":"fill","x":340,"y":260,
               "width":"fill_container","height":44,"children":[]}));
    let rects = HashMap::from([
        ("map".to_string(), rect(0.0, 0.0, 375.0, 320.0)),
        ("fill".to_string(), rect(340.0, 260.0, 44.0, 44.0)),
    ]);
    let mut cmds = Vec::new();
    collect_absolute_child_clamp_fixes(&map, &rects, &mut cmds);
    assert!(cmds.is_empty(), "keyword-sized child untouched: {cmds:?}");
}

#[test]
fn control_past_the_bottom_edge_is_shifted_up() {
    let map = map_with_control(
        json!({"type":"frame","id":"locate","x":24,"y":300,"width":44,"height":44,"children":[]}),
    );
    let rects = HashMap::from([
        ("map".to_string(), rect(0.0, 0.0, 375.0, 320.0)),
        ("locate".to_string(), rect(24.0, 300.0, 44.0, 44.0)),
    ]);
    let mut cmds = Vec::new();
    collect_absolute_child_clamp_fixes(&map, &rects, &mut cmds);
    // y = min(300, 320 - 44) = 276; x (24) already fits.
    assert_eq!(update_node_xy(&cmds, "locate"), Some((None, Some(276))));
}

// ── Symmetric-inset shrink ──

fn update_node_width(cmds: &[EditorCommand], id: &str) -> Option<Option<i32>> {
    cmds.iter().find_map(|cmd| match cmd {
        EditorCommand::UpdateNode { node_id, width, .. } if node_id.as_str() == id => Some(*width),
        _ => None,
    })
}

/// A 327x300 `layout: "none"` hero stack (painted + rounded, so the clip
/// fallback would otherwise claim it) — the width the app-18 fitness hero
/// resolved to inside its section's 24px side padding.
fn hero_stack_with(child: Value) -> Value {
    json!({
        "type":"frame","id":"hero","name":"Hero Stack","layout":"none",
        "width":327,"height":300,"cornerRadius":16,
        "fill":[{"type":"solid","color":"#101418"}],
        "children":[child]
    })
}

fn hero_rects(child_id: &str, x: f64, w: f64) -> HashMap<String, Rect> {
    HashMap::from([
        ("hero".to_string(), rect(0.0, 0.0, 327.0, 300.0)),
        (child_id.to_string(), rect(x, 16.0, w, 44.0)),
    ])
}

#[test]
fn wider_than_parent_row_is_shrunk_to_mirrored_insets() {
    // The measured app-18 shape: "Hero Top Controls", a 343px space_between
    // row (back button … bookmark button) authored for a 375-wide full-bleed
    // hero, pinned at x=16 inside a hero stack that resolved to 327px.
    // 16 + 343 = 359 > 327 and no shift can fit a 343px child in 327px.
    let hero = hero_stack_with(
        json!({"type":"frame","id":"controls","name":"Hero Top Controls",
               "x":16,"y":16,"width":343,"height":44,"children":[]}),
    );
    let rects = hero_rects("controls", 16.0, 343.0);
    let mut cmds = Vec::new();
    collect_absolute_child_shrink_fixes(&hero, &rects, &mut cmds);
    // 327 - 2·16 = 295: the left inset mirrored on the right; x/y/height untouched.
    assert_eq!(update_node_width(&cmds, "controls"), Some(Some(295)));
    assert_eq!(cmds.len(), 1, "only the width shrink: {cmds:?}");
}

#[test]
fn zero_inset_child_shrinks_to_the_full_parent_width() {
    // A flush-left child mirrors an inset of 0: full width, not a crop.
    let hero = hero_stack_with(
        json!({"type":"frame","id":"band","x":0,"y":16,"width":400,"height":44,"children":[]}),
    );
    let rects = hero_rects("band", 0.0, 400.0);
    let mut cmds = Vec::new();
    collect_absolute_child_shrink_fixes(&hero, &rects, &mut cmds);
    assert_eq!(update_node_width(&cmds, "band"), Some(Some(327)));
}

#[test]
fn far_inset_child_below_half_the_parent_is_left_to_the_clip_fallback() {
    // x=100 leaves 327 - 200 = 127 < 50% of 327: mirroring would crush the
    // child into a sliver, so the shrink stands down. (At w=300 this shape
    // is the SHIFT rule's domain — 300 fits 327 — so it also fails the
    // wider-than-parent gate; w=350 crosses that gate and isolates the
    // 50% floor as the deciding check.)
    for w in [300.0, 350.0] {
        let hero = hero_stack_with(
            json!({"type":"frame","id":"row","x":100,"y":16,"width":w,"height":44,"children":[]}),
        );
        let rects = hero_rects("row", 100.0, w);
        let mut cmds = Vec::new();
        collect_absolute_child_shrink_fixes(&hero, &rects, &mut cmds);
        assert!(cmds.is_empty(), "no shrink at w={w}: {cmds:?}");

        collect_card_overflow_clips(&hero, &rects, &mut cmds);
        assert!(
            cmds.iter().any(|cmd| matches!(
                cmd,
                EditorCommand::SetNodeLayoutProp { node_id, property, .. }
                    if node_id.as_str() == "hero" && property == "clipContent"
            )),
            "clip fallback still runs at w={w}: {cmds:?}"
        );
    }
}

#[test]
fn a_child_that_fits_is_untouched_by_the_shrink() {
    let hero = hero_stack_with(
        json!({"type":"frame","id":"chip","x":16,"y":16,"width":200,"height":44,"children":[]}),
    );
    let rects = hero_rects("chip", 16.0, 200.0);
    let mut cmds = Vec::new();
    collect_absolute_child_shrink_fixes(&hero, &rects, &mut cmds);
    assert!(cmds.is_empty(), "fitting child untouched: {cmds:?}");
}

#[test]
fn a_wider_child_in_a_flex_parent_is_untouched() {
    // A flex parent owns its children's sizing; authored x there is not
    // this pass's contract.
    let column = json!({
        "type":"frame","id":"col","layout":"vertical","width":327,"height":300,
        "children":[
            {"type":"frame","id":"row","x":16,"y":16,"width":343,"height":44,"children":[]}
        ]
    });
    let rects = HashMap::from([
        ("col".to_string(), rect(0.0, 0.0, 327.0, 300.0)),
        ("row".to_string(), rect(16.0, 16.0, 343.0, 44.0)),
    ]);
    let mut cmds = Vec::new();
    collect_absolute_child_shrink_fixes(&column, &rects, &mut cmds);
    assert!(cmds.is_empty(), "flex child untouched: {cmds:?}");
}

#[test]
fn full_bleed_chrome_is_untouched() {
    // A status bar pinned flush-left at the device width inside a narrower
    // resolved parent is a chrome contract, not an overflow to mirror.
    let hero = hero_stack_with(json!({"type":"frame","id":"sb","name":"Status Bar",
               "role":"status-bar","x":0,"y":0,"width":375,"height":54,"children":[]}));
    let rects = hero_rects("sb", 0.0, 375.0);
    let mut cmds = Vec::new();
    collect_absolute_child_shrink_fixes(&hero, &rects, &mut cmds);
    assert!(cmds.is_empty(), "status bar untouched: {cmds:?}");
}

#[test]
fn keyword_width_child_is_untouched_by_the_shrink() {
    // No authored fixed width to mirror — left to the clip logic.
    let hero = hero_stack_with(json!({"type":"frame","id":"fill","x":16,"y":16,
               "width":"fill_container","height":44,"children":[]}));
    let rects = hero_rects("fill", 16.0, 343.0);
    let mut cmds = Vec::new();
    collect_absolute_child_shrink_fixes(&hero, &rects, &mut cmds);
    assert!(cmds.is_empty(), "keyword-sized child untouched: {cmds:?}");
}

#[test]
fn the_shrink_is_recorded_under_its_own_label() {
    use crate::repair_summary::{CheckCategory, RepairCounter, RepairSummary};
    use crate::test_support::VecDocSink;
    use crate::types::DocSink;
    use jian_ops_schema::node::PenNode;
    use op_editor_core::PenNodeExt;

    // The evidence case end-to-end through a real sink, so the record the
    // QualityChecked credential shows carries the shrink's own label. The
    // hero stack is `fill_container` inside the section's 24px side padding
    // (its flex parent pins it to 327px); a `layout: "none"` frame with an
    // authored numeric width would instead resolve to its children's union
    // and hide the overflow.
    let root: PenNode = serde_json::from_value(json!({
        "type":"frame","id":"root","name":"Screen","width":375,"height":812,"layout":"vertical",
        "children":[
            {"type":"frame","id":"section","name":"Hero Section","layout":"vertical",
             "width":"fill_container","height":400,"padding":[0,24,0,24],"children":[
                {"type":"frame","id":"hero","name":"Hero Stack","layout":"none",
                 "width":"fill_container","height":300,"children":[
                    {"type":"frame","id":"controls","name":"Hero Top Controls",
                     "x":16,"y":16,"width":343,"height":44,"children":[]}
                ]}
            ]}
        ]
    }))
    .expect("valid root");
    let mut sink = VecDocSink::new();
    sink.apply(EditorCommand::InsertSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state().active_children()[0].id_str().to_string();

    let mut counter = RepairCounter::new();
    let mut summary = RepairSummary::default();
    {
        let mut counting = counter.wrap(&mut sink);
        let applied = crate::geometry_validation::shrink_oversized_absolute_children_into_parent(
            &mut counting,
            &root_id,
        );
        assert_eq!(applied, 1, "the oversized row is shrunk");
        counter.checkpoint(
            &mut summary,
            CheckCategory::Overflow,
            "absolute-child-shrink",
        );
    }
    assert!(
        summary
            .records()
            .iter()
            .any(|record| record.pass == "absolute-child-shrink"
                && record.node_name.as_deref() == Some("Hero Top Controls")
                && record.detail.contains("width 343 → 295")),
        "the record names the rule, the node, and the resize: {:?}",
        summary.records()
    );
}
