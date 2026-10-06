//! `collect_none_stack_inset_fixes` — the `fill_container`-plus-offset inset
//! repair for `layout: "none"` stacks.
//!
//! Fixtures mirror the measured phone screens: a 375-wide stack holding a
//! floating card authored `x: 24` + `width: "fill_container"` (the taxi /
//! hotel search-card shape). Resolved rects are hand-built at the parent's
//! origin, the way the real jian pass reports them.

use super::*;
use serde_json::json;

fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
    Rect { x, y, w, h }
}

fn stack(child: Value) -> Value {
    json!({
        "type":"frame","id":"stack","name":"搜索区堆叠","layout":"none",
        "width":"fill_container","height":500,
        "children":[child]
    })
}

fn update_node_size(cmds: &[EditorCommand], id: &str) -> Option<(Option<i32>, Option<i32>)> {
    cmds.iter().find_map(|cmd| match cmd {
        EditorCommand::UpdateNode {
            node_id,
            width,
            height,
            ..
        } if node_id.as_str() == id => Some((*width, *height)),
        _ => None,
    })
}

#[test]
fn fill_width_card_with_x_offset_is_inset_on_both_sides() {
    // The taxi shape: a 375-wide stack, a card at x=24 authored
    // `fill_container`. jian gave it the full 375 and the offset pushed it
    // 24px past the right edge; the inset rewrite makes it 375 − 2·24 = 327.
    let stack = stack(json!({"type":"frame","id":"card","name":"搜索表单卡",
        "x":24,"y":176,"width":"fill_container","height":"fit_content","children":[]}));
    let rects = HashMap::from([("stack".to_string(), rect(0.0, 0.0, 375.0, 500.0))]);
    let mut cmds = Vec::new();
    collect_none_stack_inset_fixes(&stack, &rects, &mut cmds);
    assert_eq!(update_node_size(&cmds, "card"), Some((Some(327), None)));
    assert_eq!(cmds.len(), 1, "only the width rewrite: {cmds:?}");
}

#[test]
fn fill_height_card_with_y_offset_is_inset_top_and_bottom() {
    // Vertical variant: `height: "fill_container"` + y=176 inside the 500px
    // stack → 500 − 2·176 = 148.
    let stack = stack(json!({"type":"frame","id":"card","x":0,"y":176,
        "width":327,"height":"fill_container","children":[]}));
    let rects = HashMap::from([("stack".to_string(), rect(0.0, 0.0, 375.0, 500.0))]);
    let mut cmds = Vec::new();
    collect_none_stack_inset_fixes(&stack, &rects, &mut cmds);
    assert_eq!(update_node_size(&cmds, "card"), Some((None, Some(148))));
}

#[test]
fn numeric_sized_child_is_untouched() {
    // An authored fixed box is not this pass's contract — the absolute-child
    // clamp owns fixed-size overflow.
    let stack = stack(json!({"type":"frame","id":"card","x":24,"y":176,
        "width":327,"height":200,"children":[]}));
    let rects = HashMap::from([("stack".to_string(), rect(0.0, 0.0, 375.0, 500.0))]);
    let mut cmds = Vec::new();
    collect_none_stack_inset_fixes(&stack, &rects, &mut cmds);
    assert!(cmds.is_empty(), "numeric-sized child untouched: {cmds:?}");
}

#[test]
fn fill_width_card_at_zero_offset_is_untouched() {
    // x=0 + fill_container already means edge-to-edge — no inset intent.
    let stack = stack(json!({"type":"frame","id":"card","x":0,"y":176,
        "width":"fill_container","height":"fit_content","children":[]}));
    let rects = HashMap::from([("stack".to_string(), rect(0.0, 0.0, 375.0, 500.0))]);
    let mut cmds = Vec::new();
    collect_none_stack_inset_fixes(&stack, &rects, &mut cmds);
    assert!(cmds.is_empty(), "zero-offset child untouched: {cmds:?}");
}

#[test]
fn offset_leaving_no_room_is_untouched() {
    // 2·x ≥ parent width: nothing left to inset into, so a rewrite would
    // produce a zero / negative width.
    let stack = stack(json!({"type":"frame","id":"card","x":200,"y":176,
        "width":"fill_container","height":"fit_content","children":[]}));
    let rects = HashMap::from([("stack".to_string(), rect(0.0, 0.0, 375.0, 500.0))]);
    let mut cmds = Vec::new();
    collect_none_stack_inset_fixes(&stack, &rects, &mut cmds);
    assert!(
        cmds.is_empty(),
        "offset ≥ half the parent untouched: {cmds:?}"
    );
}

#[test]
fn stack_without_a_resolved_width_is_untouched() {
    // No resolved parent box → no width to inset against.
    let stack = stack(json!({"type":"frame","id":"card","x":24,"y":176,
        "width":"fill_container","height":"fit_content","children":[]}));
    let rects = HashMap::new();
    let mut cmds = Vec::new();
    collect_none_stack_inset_fixes(&stack, &rects, &mut cmds);
    assert!(cmds.is_empty(), "unresolved parent untouched: {cmds:?}");
}

#[test]
fn status_bar_stack_is_untouched() {
    // The status bar stacks its chrome the same way (layout: "none" + pinned
    // children); chrome is protected everywhere else, so here too.
    let bar = json!({
        "type":"frame","id":"sb","name":"Status Bar","layout":"none",
        "width":"fill_container","height":62,
        "children":[{"type":"frame","id":"pill","x":24,"y":20,
            "width":"fill_container","height":24,"children":[]}]
    });
    let rects = HashMap::from([("sb".to_string(), rect(0.0, 0.0, 375.0, 62.0))]);
    let mut cmds = Vec::new();
    collect_none_stack_inset_fixes(&bar, &rects, &mut cmds);
    assert!(cmds.is_empty(), "status-bar child untouched: {cmds:?}");
}
