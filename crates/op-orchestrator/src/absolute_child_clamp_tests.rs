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
