//! Pen-tool authoring + Select-tool path-editing dispatch tests
//! (`pen_press.rs`). TS reference: `skia-pen-tool.ts` /
//! `skia-interaction.ts:277-306,1356-1378` / `skia-canvas.tsx:38-71`.

use super::WidgetHostNative;
use jian_ops_schema::node::{PenNode, PenPathPointType};
use op_editor_core::{NodeId, Tool};

const VW: f32 = 1440.0;
const VH: f32 = 900.0;

/// Seed a host's `editor_state` from a canonical `.op` JSON snippet.
fn seed(host: &mut WidgetHostNative, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.mark_paint_dirty_for_test();
}

fn empty_host() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    seed(&mut host, r#"{"version":"0.8.0","children":[]}"#);
    host
}

/// A pre-authored 3-anchor open path fixture.
fn path_host() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[
          {"type":"path","id":"n60","name":"p","x":300,"y":300,
           "anchors":[{"x":300,"y":300},{"x":380,"y":300},{"x":380,"y":360}]}
        ]}"#,
    );
    host
}

/// Map a doc point to viewport coords (zoom 1, pan 0 after seeding).
fn screen(host: &WidgetHostNative, doc_x: f32, doc_y: f32) -> (f32, f32) {
    let (cx0, cy0, _, _) = host.canvas_region(VW, VH);
    (cx0 + doc_x, cy0 + doc_y)
}

fn pen_node_anchors(host: &WidgetHostNative) -> Vec<jian_ops_schema::node::PenPathAnchor> {
    let id = host
        .editor_state()
        .ui
        .pen_in_progress
        .clone()
        .expect("pen in progress");
    match op_editor_core::walkers::find_node(host.editor_state().active_children(), &id) {
        Some(PenNode::Path(p)) => p.anchors.clone().unwrap_or_default(),
        _ => panic!("pen path node missing"),
    }
}

fn find_path<'a>(host: &'a WidgetHostNative, id: &str) -> &'a jian_ops_schema::node::PathNode {
    match op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new(id),
    ) {
        Some(PenNode::Path(p)) => p,
        _ => panic!("path node {id} missing"),
    }
}

// --- Drag-to-curve (TS skia-pen-tool.ts:94-121) -------------------

#[test]
fn pen_press_drag_mints_mirrored_handles_on_last_anchor() {
    let mut host = empty_host();
    host.editor_state_mut().tool = Tool::Pen;
    let (px, py) = screen(&host, 300.0, 300.0);
    assert!(host.apply_press(px, py, VW, VH));
    assert!(host.editor_state().ui.pen_dragging_handle);
    // Drag 30 doc px right while the press is held.
    host.apply_cursor_move(px + 30.0, py);
    let anchors = pen_node_anchors(&host);
    let last = anchors.last().expect("anchor placed");
    let ho = last.handle_out.clone().expect("handle_out minted");
    let hi = last.handle_in.clone().expect("handle_in minted");
    assert!(
        (ho.x - 30.0).abs() < 0.5 && ho.y.abs() < 0.5,
        "out = drag delta"
    );
    assert!((hi.x + 30.0).abs() < 0.5, "in = negated delta");
    assert_eq!(last.point_type, Some(PenPathPointType::Mirrored));
    // Release stops minting; later moves only track the rubber band.
    assert!(host.apply_release_with_viewport(VW, VH));
    assert!(!host.editor_state().ui.pen_dragging_handle);
    host.apply_cursor_move(px + 80.0, py + 40.0);
    let after = pen_node_anchors(&host);
    let ho2 = after.last().unwrap().handle_out.clone().unwrap();
    assert!((ho2.x - 30.0).abs() < 0.5, "handle frozen after release");
}

#[test]
fn pen_drag_within_two_px_clears_back_to_corner() {
    // TS: hypot <= 2 resets the handles + pointType 'corner'.
    let mut host = empty_host();
    host.editor_state_mut().tool = Tool::Pen;
    let (px, py) = screen(&host, 300.0, 300.0);
    host.apply_press(px, py, VW, VH);
    host.apply_cursor_move(px + 30.0, py);
    host.apply_cursor_move(px + 1.0, py);
    let anchors = pen_node_anchors(&host);
    let last = anchors.last().unwrap();
    assert!(last.handle_out.is_none() && last.handle_in.is_none());
    assert_eq!(last.point_type, Some(PenPathPointType::Corner));
}

// --- Close path (TS skia-pen-tool.ts:71-79) -----------------------

#[test]
fn click_on_first_anchor_closes_the_path() {
    let mut host = empty_host();
    host.editor_state_mut().tool = Tool::Pen;
    host.set_now_ms(0);
    for (i, (x, y)) in [(300.0, 300.0), (380.0, 300.0), (380.0, 360.0)]
        .into_iter()
        .enumerate()
    {
        // Space presses 1 s apart so the double-click detector never fires.
        host.set_now_ms(i as u64 * 1000);
        let (px, py) = screen(&host, x, y);
        host.apply_press(px, py, VW, VH);
        host.apply_release_with_viewport(VW, VH);
    }
    let id = host
        .editor_state()
        .ui
        .pen_in_progress
        .clone()
        .expect("3-anchor session open");
    // Press within 8 px / zoom of the FIRST anchor.
    host.set_now_ms(5000);
    let (px, py) = screen(&host, 303.0, 302.0);
    let history_before = host.editor_state().history.past.len();
    host.apply_press(px, py, VW, VH);
    assert!(
        host.editor_state().ui.pen_in_progress.is_none(),
        "committed"
    );
    let p = find_path(&host, id.as_str());
    assert_eq!(p.closed, Some(true), "close-hit produces closed:true");
    assert_eq!(p.anchors.as_ref().map(|a| a.len()), Some(3));
    let d = p.d.clone().expect("d persisted on commit");
    assert!(d.ends_with('Z'), "closed path d ends with Z: {d}");
    assert_eq!(
        host.editor_state().history.past.len(),
        history_before + 1,
        "one history entry per committed pen path"
    );
    // TS finalizePen lands back on the Select tool.
    assert_eq!(host.editor_state().tool, Tool::Select);
}

#[test]
fn close_hit_needs_at_least_three_anchors() {
    // TS gates the close-hit on penPoints.length >= 3 — with two
    // anchors a press near the first just adds a third anchor.
    let mut host = empty_host();
    host.editor_state_mut().tool = Tool::Pen;
    host.set_now_ms(0);
    let (px, py) = screen(&host, 300.0, 300.0);
    host.apply_press(px, py, VW, VH);
    host.apply_release_with_viewport(VW, VH);
    host.set_now_ms(1000);
    let (qx, qy) = screen(&host, 380.0, 300.0);
    host.apply_press(qx, qy, VW, VH);
    host.apply_release_with_viewport(VW, VH);
    host.set_now_ms(2000);
    let (rx, ry) = screen(&host, 303.0, 302.0);
    host.apply_press(rx, ry, VW, VH);
    assert!(
        host.editor_state().ui.pen_in_progress.is_some(),
        "2-anchor session must not close"
    );
    assert_eq!(pen_node_anchors(&host).len(), 3);
}

// --- Double-click finish (TS skia-pen-tool.ts:123-129) ------------

#[test]
fn double_click_finishes_the_path_open() {
    let mut host = empty_host();
    host.editor_state_mut().tool = Tool::Pen;
    host.set_now_ms(0);
    let (px, py) = screen(&host, 300.0, 300.0);
    host.apply_press(px, py, VW, VH);
    host.apply_release_with_viewport(VW, VH);
    host.set_now_ms(1000);
    let (qx, qy) = screen(&host, 380.0, 340.0);
    host.apply_press(qx, qy, VW, VH);
    host.apply_release_with_viewport(VW, VH);
    let id = host.editor_state().ui.pen_in_progress.clone().unwrap();
    // Second press at the same spot within 400 ms = double-click.
    host.set_now_ms(1200);
    let history_before = host.editor_state().history.past.len();
    host.apply_press(qx + 1.0, qy, VW, VH);
    assert!(
        host.editor_state().ui.pen_in_progress.is_none(),
        "committed"
    );
    let p = find_path(&host, id.as_str());
    // TS pops the second click's anchor before finalizing — the net
    // anchor set keeps only the two real placements.
    assert_eq!(p.anchors.as_ref().map(|a| a.len()), Some(2));
    assert_eq!(p.closed, Some(false));
    assert_eq!(host.editor_state().history.past.len(), history_before + 1);
    assert_eq!(host.editor_state().tool, Tool::Select);
}

// --- Backspace (TS skia-pen-tool.ts:142-150) ----------------------

#[test]
fn backspace_pops_last_anchor_without_killing_the_session() {
    let mut host = empty_host();
    host.editor_state_mut().tool = Tool::Pen;
    host.set_now_ms(0);
    for (i, (x, y)) in [(300.0, 300.0), (380.0, 300.0), (380.0, 360.0)]
        .into_iter()
        .enumerate()
    {
        host.set_now_ms(i as u64 * 1000);
        let (px, py) = screen(&host, x, y);
        host.apply_press(px, py, VW, VH);
        host.apply_release_with_viewport(VW, VH);
    }
    assert!(host.apply_backspace());
    assert!(
        host.editor_state().ui.pen_in_progress.is_some(),
        "session survives"
    );
    assert_eq!(pen_node_anchors(&host).len(), 2);
    assert_eq!(host.editor_state().tool, Tool::Pen, "tool unchanged");
}

#[test]
fn backspace_on_lone_anchor_cancels_and_returns_to_select() {
    let mut host = empty_host();
    host.editor_state_mut().tool = Tool::Pen;
    let (px, py) = screen(&host, 300.0, 300.0);
    host.apply_press(px, py, VW, VH);
    host.apply_release_with_viewport(VW, VH);
    let history_before = host.editor_state().history.past.len();
    assert!(host.apply_backspace());
    assert!(host.editor_state().ui.pen_in_progress.is_none());
    assert!(
        host.editor_state().active_children().is_empty(),
        "lone-anchor cancel strips the node"
    );
    assert_eq!(
        host.editor_state().history.past.len(),
        history_before,
        "cancel never pushes history"
    );
    assert_eq!(
        host.editor_state().tool,
        Tool::Select,
        "TS cancel() → select"
    );
}

// --- Escape cancels (TS cancel(), not commit) ----------------------

#[test]
fn escape_discards_the_in_progress_path() {
    let mut host = empty_host();
    host.editor_state_mut().tool = Tool::Pen;
    host.set_now_ms(0);
    let (px, py) = screen(&host, 300.0, 300.0);
    host.apply_press(px, py, VW, VH);
    host.apply_release_with_viewport(VW, VH);
    host.set_now_ms(1000);
    let (qx, qy) = screen(&host, 380.0, 340.0);
    host.apply_press(qx, qy, VW, VH);
    host.apply_release_with_viewport(VW, VH);
    let history_before = host.editor_state().history.past.len();
    assert!(host.apply_escape());
    assert!(host.editor_state().ui.pen_in_progress.is_none());
    assert!(
        host.editor_state().active_children().is_empty(),
        "Escape CANCELS — the 2-anchor path is discarded, not committed"
    );
    assert_eq!(host.editor_state().history.past.len(), history_before);
    assert_eq!(host.editor_state().tool, Tool::Select);
}

#[test]
fn tool_switch_discards_the_in_progress_path() {
    // TS onToolChange resets the pen preview without committing.
    let mut host = empty_host();
    host.editor_state_mut().tool = Tool::Pen;
    host.set_now_ms(0);
    let (px, py) = screen(&host, 300.0, 300.0);
    host.apply_press(px, py, VW, VH);
    host.apply_release_with_viewport(VW, VH);
    host.set_now_ms(1000);
    let (qx, qy) = screen(&host, 380.0, 340.0);
    host.apply_press(qx, qy, VW, VH);
    host.apply_release_with_viewport(VW, VH);
    host.apply_set_tool(Tool::Rect);
    assert!(host.editor_state().ui.pen_in_progress.is_none());
    assert!(host.editor_state().active_children().is_empty());
    assert_eq!(host.editor_state().tool, Tool::Rect);
}

// --- Commit shape (canonical schema parity) ------------------------

#[test]
fn committed_path_persists_d_closed_fill_and_stroke() {
    let mut host = empty_host();
    host.editor_state_mut().tool = Tool::Pen;
    host.set_now_ms(0);
    let (px, py) = screen(&host, 300.0, 300.0);
    host.apply_press(px, py, VW, VH);
    host.apply_release_with_viewport(VW, VH);
    host.set_now_ms(1000);
    let (qx, qy) = screen(&host, 380.0, 340.0);
    host.apply_press(qx, qy, VW, VH);
    host.apply_release_with_viewport(VW, VH);
    let id = host.editor_state().ui.pen_in_progress.clone().unwrap();
    assert!(host.apply_send(), "Enter finishes");
    let p = find_path(&host, id.as_str());
    assert_eq!(p.closed, Some(false));
    assert!(p.d.as_deref().unwrap_or("").starts_with("M "));
    // TS commits fill: transparent + stroke #374151 / 1 px.
    let fills = p.fill.as_ref().expect("fill seeded");
    assert!(matches!(
        &fills[0],
        jian_ops_schema::style::PenFill::Solid(b) if b.color == "transparent"
    ));
    let stroke = p.stroke.as_ref().expect("stroke seeded");
    let stroke_fill = stroke.fill.as_ref().expect("stroke fill");
    assert!(matches!(
        &stroke_fill[0],
        jian_ops_schema::style::PenFill::Solid(b) if b.color == "#374151"
    ));
}

// --- Select-tool path editing (#5) ---------------------------------

#[test]
fn select_tool_press_on_anchor_starts_the_drag() {
    let mut host = path_host();
    host.editor_state_mut().tool = Tool::Select;
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n60"));
    host.mark_paint_dirty_for_test();
    let (px, py) = screen(&host, 380.0, 300.0);
    assert!(host.apply_press(px, py, VW, VH));
    let drag = host.path_anchor_drag.as_ref().expect("anchor drag armed");
    assert_eq!(drag.anchor_index, 1);
    assert!(matches!(
        drag.target,
        crate::widget_host::AnchorDragTarget::Anchor
    ));
}

#[test]
fn handle_drag_preserves_grab_offset_and_anchor_type() {
    // TS `movePathControl` (`path-editing.ts:66-114`) applies the
    // cumulative cursor DELTA to the grabbed handle — pressing 2 px
    // off the dot and moving 10 px must land at original + 10, not
    // snap the handle onto the cursor. An untyped one-sided anchor
    // stays untyped and never mints the opposite handle.
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[
          {"type":"path","id":"n60","name":"p","x":300,"y":300,
           "anchors":[{"x":300,"y":300},
                      {"x":380,"y":300,"handleOut":{"x":20,"y":0}},
                      {"x":380,"y":360}]}
        ]}"#,
    );
    host.editor_state_mut().tool = Tool::Select;
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n60"));
    host.mark_paint_dirty_for_test();
    // Handle dot sits at doc (400, 300); press 2 px off-center.
    let (px, py) = screen(&host, 398.0, 298.0);
    assert!(host.apply_press(px, py, VW, VH));
    let drag = host.path_anchor_drag.as_ref().expect("handle drag armed");
    assert!(matches!(
        drag.target,
        crate::widget_host::AnchorDragTarget::Handle(op_editor_core::pen::PathHandleSide::Out)
    ));
    let grab = drag.grab_offset.expect("existing handle records grab");
    assert!((grab.x - 20.0).abs() < 0.5 && grab.y.abs() < 0.5);
    // Move the cursor +10 doc px right.
    host.apply_cursor_move(px + 10.0, py);
    let a = &find_path(&host, "n60").anchors.as_ref().unwrap()[1];
    let ho = a.handle_out.clone().expect("handle kept");
    assert!(
        (ho.x - 30.0).abs() < 0.5 && ho.y.abs() < 0.5,
        "offset = grab + delta (no snap-to-cursor); got ({}, {})",
        ho.x,
        ho.y
    );
    assert_eq!(a.point_type, None, "untyped anchor stays untyped");
    assert!(a.handle_in.is_none(), "no opposite handle minted");
    // Release commits exactly one history entry; undo restores.
    let history_before = host.editor_state().history.past.len();
    assert!(host.apply_release_with_viewport(VW, VH));
    assert_eq!(host.editor_state().history.past.len(), history_before + 1);
    assert!(host.editor_state_mut().undo());
    let a = &find_path(&host, "n60").anchors.as_ref().unwrap()[1];
    let ho = a.handle_out.clone().expect("handle restored");
    assert!((ho.x - 20.0).abs() < 0.5);
}

#[test]
fn select_tool_ignores_ghost_handles() {
    // Unset handles are grabbable only with the Pen tool — under
    // Select the same point falls through to node selection.
    let mut host = path_host();
    host.editor_state_mut().tool = Tool::Select;
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n60"));
    host.mark_paint_dirty_for_test();
    // Ghost handle sits 26 px right of the anchor at zoom 1.
    let (px, py) = screen(&host, 380.0 + 26.0, 300.0);
    host.apply_press(px, py, VW, VH);
    assert!(
        host.path_anchor_drag.is_none(),
        "ghost handle must not arm a drag under Select"
    );
}

#[test]
fn right_click_anchor_opens_menu_and_action_commits_history() {
    let mut host = path_host();
    host.editor_state_mut().tool = Tool::Select;
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n60"));
    host.mark_paint_dirty_for_test();
    let (px, py) = screen(&host, 380.0, 300.0);
    assert!(host.apply_right_press(px, py, VW, VH));
    let menu = host
        .editor_state()
        .ui
        .path_anchor_menu
        .clone()
        .expect("menu open");
    assert_eq!(menu.anchor_index, 1);
    // Row 2 = "Symmetric Curve" (Mirrored). Rows are 26 px tall
    // under a 4 px pad, anchored at the right-press point.
    let history_before = host.editor_state().history.past.len();
    let row_y = py + 4.0 + 26.0 * 1.5;
    assert!(host.apply_press(px + 10.0, row_y, VW, VH));
    assert!(
        host.editor_state().ui.path_anchor_menu.is_none(),
        "menu closed"
    );
    let p = find_path(&host, "n60");
    let a = &p.anchors.as_ref().unwrap()[1];
    assert_eq!(a.point_type, Some(PenPathPointType::Mirrored));
    assert!(a.handle_in.is_some() && a.handle_out.is_some());
    assert_eq!(
        host.editor_state().history.past.len(),
        history_before + 1,
        "menu action commits exactly one history entry"
    );
    // Undo restores the corner anchor.
    assert!(host.editor_state_mut().undo());
    let p = find_path(&host, "n60");
    assert!(p.anchors.as_ref().unwrap()[1].handle_in.is_none());
}

#[test]
fn right_click_miss_closes_menu_without_consuming() {
    let mut host = path_host();
    host.editor_state_mut().tool = Tool::Select;
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n60"));
    host.mark_paint_dirty_for_test();
    let (px, py) = screen(&host, 380.0, 300.0);
    host.apply_right_press(px, py, VW, VH);
    assert!(host.editor_state().ui.path_anchor_menu.is_some());
    // Press far away — the menu closes and the press still routes
    // (returns false from the dispatcher; apply_press continues).
    let (qx, qy) = screen(&host, 600.0, 500.0);
    host.apply_press(qx, qy, VW, VH);
    assert!(host.editor_state().ui.path_anchor_menu.is_none());
    assert!(
        host.marquee_drag.is_some(),
        "the dismissing press still starts the canvas marquee (TS parity)"
    );
}
