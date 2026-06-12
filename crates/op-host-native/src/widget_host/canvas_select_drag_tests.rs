//! Press-path tests for selection promotion / enter-group and the
//! node-drag release commit (auto-layout reorder + reparent-to-root).
//!
//! Geometry discipline: viewport 1440×900, fixtures placed at doc
//! x ≥ 400 so every screen press lands right of the AI chat float
//! (x ≤ 612) and the floating toolbar (x ≈ 252-300, y ≈ 60-400).

use super::WidgetHostNative;
use op_editor_core::{NodeId, PenNodeExt};

const VIEWPORT_W: f32 = 1440.0;
const VIEWPORT_H: f32 = 900.0;

fn seed(host: &mut WidgetHostNative, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.mark_paint_dirty_for_test();
}

/// `card` frame (400, 60, 200×200) with a nested `leaf` rect at
/// relative (40, 40) — leaf renders at doc (440..490, 100..150) —
/// plus a far-away top-level `other` rect at (650, 60, 40×40).
const NESTED: &str = r#"{"version":"0.8.0","children":[
  {"type":"frame","id":"card","name":"Card","x":400,"y":60,"width":200,"height":200,
   "children":[
     {"type":"rectangle","id":"leaf","name":"Leaf","x":40,"y":40,"width":50,"height":50}
   ]},
  {"type":"rectangle","id":"other","name":"Other","x":650,"y":60,"width":40,"height":40}
]}"#;

/// Screen point for a doc point (zoom 1, pan 0 in a fresh host).
fn screen_at(host: &WidgetHostNative, doc_x: f32, doc_y: f32) -> (f32, f32) {
    let (cx0, cy0, _cw, _ch) = host.canvas_region(VIEWPORT_W, VIEWPORT_H);
    (cx0 + doc_x, cy0 + doc_y)
}

fn press_doc(host: &mut WidgetHostNative, doc_x: f32, doc_y: f32) {
    let (x, y) = screen_at(host, doc_x, doc_y);
    host.apply_press(x, y, VIEWPORT_W, VIEWPORT_H);
}

fn release(host: &mut WidgetHostNative) {
    let _ = host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H);
}

// --- GAP A: promotion / enter-group / Escape ---------------------------

#[test]
fn click_on_nested_child_promotes_to_top_level_frame() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, NESTED);
    press_doc(&mut host, 450.0, 110.0); // over `leaf`
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("card"));
    assert_eq!(host.editor_state().editor_ui.entered_container, None);
}

#[test]
fn click_on_child_of_selected_multi_set_keeps_the_set() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, NESTED);
    host.editor_state_mut().selection.set = vec![NodeId::new("card"), NodeId::new("other")];
    host.editor_state_mut().selection.anchor = NodeId::new("other");
    host.mark_paint_dirty_for_test();
    press_doc(&mut host, 450.0, 110.0); // child of selected `card`
    assert_eq!(host.editor_state().selection_count(), 2, "set preserved");
    assert!(host.node_drag.is_some(), "press still drags the set");
}

#[test]
fn double_click_selected_container_enters_and_selects_child() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, NESTED);
    press_doc(&mut host, 450.0, 110.0); // first click → selects `card`
    release(&mut host);
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("card"));
    press_doc(&mut host, 450.0, 110.0); // double-click (same node, <400 ms)
    release(&mut host);
    assert_eq!(
        host.editor_state().editor_ui.entered_container,
        Some(NodeId::new("card")),
        "double-click on the selected container enters it"
    );
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("leaf"));
}

#[test]
fn escape_clears_selection_first_then_exits_entered_container() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, NESTED);
    host.editor_state_mut().editor_ui.entered_container = Some(NodeId::new("card"));
    host.editor_state_mut()
        .set_single_selection(NodeId::new("leaf"));
    // TS Escape order (use-tool-shortcuts.ts:38-49): selection first…
    assert!(host.apply_escape());
    assert!(host.editor_state().selection.is_empty());
    assert_eq!(
        host.editor_state().editor_ui.entered_container,
        Some(NodeId::new("card"))
    );
    // …then the entered container.
    assert!(host.apply_escape());
    assert_eq!(host.editor_state().editor_ui.entered_container, None);
}

#[test]
fn selecting_outside_the_entered_container_exits_it() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, NESTED);
    host.editor_state_mut().editor_ui.entered_container = Some(NodeId::new("card"));
    host.editor_state_mut()
        .set_single_selection(NodeId::new("leaf"));
    host.mark_paint_dirty_for_test();
    press_doc(&mut host, 670.0, 80.0); // top-level `other`
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("other"));
    assert_eq!(host.editor_state().editor_ui.entered_container, None);
}

#[test]
fn blank_canvas_press_exits_the_entered_container() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, NESTED);
    host.editor_state_mut().editor_ui.entered_container = Some(NodeId::new("card"));
    host.editor_state_mut()
        .set_single_selection(NodeId::new("leaf"));
    host.mark_paint_dirty_for_test();
    press_doc(&mut host, 700.0, 350.0); // dead canvas
    assert!(host.editor_state().selection.is_empty());
    assert_eq!(host.editor_state().editor_ui.entered_container, None);
}

#[test]
fn promotion_inside_entered_container_stops_at_its_child() {
    // card > inner (frame) > deep (rect): with card entered, a press
    // on `deep` selects `inner`, not the page-root `card`.
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[
          {"type":"frame","id":"card","name":"Card","x":400,"y":60,"width":200,"height":200,
           "children":[
             {"type":"frame","id":"inner","name":"Inner","x":20,"y":20,"width":150,"height":150,
              "children":[
                {"type":"rectangle","id":"deep","name":"Deep","x":20,"y":20,"width":60,"height":60}
              ]}
           ]}
        ]}"#,
    );
    host.editor_state_mut().editor_ui.entered_container = Some(NodeId::new("card"));
    host.mark_paint_dirty_for_test();
    press_doc(&mut host, 450.0, 110.0); // over `deep` (abs 440..500, 100..160)
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("inner"));
    assert_eq!(
        host.editor_state().editor_ui.entered_container,
        Some(NodeId::new("card")),
        "selection inside the container keeps it entered"
    );
}

// --- GAP B: drag-release reorder / reparent -----------------------------

/// Vertical auto-layout stack at (400, 60) with three 80×40 flow
/// children (gap 8): a 60..100, b 108..148, c 156..196 in doc y.
const VSTACK: &str = r#"{"version":"0.8.0","children":[
  {"type":"frame","id":"stack","name":"Stack","x":400,"y":60,"width":200,"height":300,
   "layout":"vertical","gap":8,
   "children":[
     {"type":"rectangle","id":"a","name":"A","width":80,"height":40},
     {"type":"rectangle","id":"b","name":"B","width":80,"height":40},
     {"type":"rectangle","id":"c","name":"C","width":80,"height":40}
   ]}
]}"#;

fn child_order(host: &WidgetHostNative, parent: &str) -> Vec<String> {
    op_editor_core::walkers::find_node(host.editor_state().active_children(), &NodeId::new(parent))
        .and_then(|n| n.children())
        .map(|cs| cs.iter().map(|c| c.id_str().to_string()).collect())
        .unwrap_or_default()
}

#[test]
fn dragging_flex_child_reorders_at_midpoint_index_on_release() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, VSTACK);
    // Entered context so the press selects the flex child itself.
    host.editor_state_mut().editor_ui.entered_container = Some(NodeId::new("stack"));
    host.mark_paint_dirty_for_test();
    press_doc(&mut host, 440.0, 80.0); // over `a`
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("a"));
    // Drag down 80 doc px: dropped midpoint 160 lands between b (128)
    // and c (176) → index 1.
    let (mx, my) = screen_at(&host, 440.0, 160.0);
    host.apply_cursor_move(mx, my);
    // Flex child must not doc-translate during the drag.
    let a = op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new("a"),
    )
    .expect("a present");
    assert_eq!(a.base().x, None, "no live x materialization");
    release(&mut host);
    assert_eq!(child_order(&host, "stack"), vec!["b", "a", "c"]);
}

#[test]
fn dragging_flex_child_within_its_own_slot_keeps_order() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, VSTACK);
    host.editor_state_mut().editor_ui.entered_container = Some(NodeId::new("stack"));
    host.mark_paint_dirty_for_test();
    press_doc(&mut host, 440.0, 80.0);
    let (mx, my) = screen_at(&host, 442.0, 86.0); // tiny wiggle past threshold
    host.apply_cursor_move(mx, my);
    release(&mut host);
    assert_eq!(child_order(&host, "stack"), vec!["a", "b", "c"]);
}

#[test]
fn text_dragged_fully_outside_parent_reparents_to_page_root() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[
          {"type":"frame","id":"card","name":"Card","x":400,"y":60,"width":200,"height":100,
           "children":[
             {"type":"text","id":"label","name":"Label","x":20,"y":20,
              "width":100,"height":20,"content":"Hi"}
           ]}
        ]}"#,
    );
    host.editor_state_mut().editor_ui.entered_container = Some(NodeId::new("card"));
    host.mark_paint_dirty_for_test();
    press_doc(&mut host, 450.0, 90.0); // over `label` (abs 420..520, 80..100)
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("label"));
    // Drag 400 doc px right — fully clear of card's 400..600 span.
    let (mx, my) = screen_at(&host, 850.0, 90.0);
    host.apply_cursor_move(mx, my);
    release(&mut host);
    let children = host.editor_state().active_children();
    assert_eq!(
        children[0].id_str(),
        "label",
        "content primitives detach to the page root (drag-reparent-policy)"
    );
    let label = &children[0];
    assert!(
        (label.base().x.unwrap_or(0.0) - 820.0).abs() < 1.0,
        "visual position preserved; got {:?}",
        label.base().x
    );
    let card = op_editor_core::walkers::find_node(children, &NodeId::new("card")).unwrap();
    assert!(card.children().unwrap().is_empty());
}

#[test]
fn shape_dragged_outside_parent_keeps_its_parent() {
    // TS drag-reparent-policy: frame/shape-style nodes never detach.
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[
          {"type":"frame","id":"card","name":"Card","x":400,"y":60,"width":200,"height":100,
           "children":[
             {"type":"rectangle","id":"box","name":"Box","x":20,"y":20,"width":50,"height":50}
           ]}
        ]}"#,
    );
    host.editor_state_mut().editor_ui.entered_container = Some(NodeId::new("card"));
    host.mark_paint_dirty_for_test();
    press_doc(&mut host, 440.0, 100.0); // over `box` (abs 420..470, 80..130)
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("box"));
    let (mx, my) = screen_at(&host, 840.0, 100.0); // +400 doc px right
    host.apply_cursor_move(mx, my);
    release(&mut host);
    let children = host.editor_state().active_children();
    let card = op_editor_core::walkers::find_node(children, &NodeId::new("card")).unwrap();
    let kept: Vec<&str> = card
        .children()
        .unwrap()
        .iter()
        .map(|c| c.id_str())
        .collect();
    assert_eq!(kept, vec!["box"], "shape stays inside its parent");
    // The free-layout translate itself still committed.
    let boxn = op_editor_core::walkers::find_node(children, &NodeId::new("box")).unwrap();
    assert!(
        (boxn.base().x.unwrap_or(0.0) - 420.0).abs() < 1.0,
        "live translate kept; got {:?}",
        boxn.base().x
    );
}
