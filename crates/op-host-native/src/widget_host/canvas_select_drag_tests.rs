//! Press-path tests for selection promotion / enter-group and the
//! node-drag release commit (auto-layout reorder + reparent-to-root).
//!
//! Geometry discipline: viewport 1440×900, fixtures placed at doc
//! x ≥ 400 so every screen press lands right of the AI chat float
//! (x ≤ 612) and the floating toolbar (x ≈ 252-300, y ≈ 60-400).

use super::{CursorHint, NodeDragState, WidgetHostNative};
use op_editor_core::{NodeId, PenNodeExt, Tool};

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
const NESTED: &str = r#"{"version":"1.0.0","children":[
  {"type":"frame","id":"card","name":"Card","x":400,"y":60,"width":200,"height":200,
   "children":[
     {"type":"rectangle","id":"leaf","name":"Leaf","x":40,"y":40,"width":50,"height":50}
   ]},
  {"type":"rectangle","id":"other","name":"Other","x":650,"y":60,"width":40,"height":40}
]}"#;

/// Four selection depths at the shared probe point: root > l1 > l2 > l3.
/// Every nested node contains doc (470, 130), while `other` remains available
/// for multi-selection and outside-scope tests.
const FOUR_LEVEL: &str = r#"{"version":"1.0.0","children":[
  {"type":"frame","id":"root","name":"Root","x":400,"y":60,"width":240,"height":240,
   "children":[
     {"type":"frame","id":"l1","name":"Level 1","x":20,"y":20,"width":200,"height":200,
      "children":[
        {"type":"frame","id":"l2","name":"Level 2","x":20,"y":20,"width":160,"height":160,
         "children":[
           {"type":"rectangle","id":"l3","name":"Level 3","x":20,"y":20,"width":60,"height":60}
         ]}
      ]}
   ]},
  {"type":"rectangle","id":"other","name":"Other","x":700,"y":60,"width":40,"height":40}
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

// --- Relative-depth selection / enter-group / Escape -------------------

#[test]
fn first_press_on_four_level_hit_selects_level_one() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, FOUR_LEVEL);

    press_doc(&mut host, 470.0, 130.0);

    assert_eq!(host.editor_state().selection.anchor, NodeId::new("l1"));
    assert_eq!(host.editor_state().editor_ui.entered_container, None);
}

#[test]
fn click_on_selected_primary_in_multi_set_keeps_the_set() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, FOUR_LEVEL);
    host.editor_state_mut().selection.set = vec![NodeId::new("l1"), NodeId::new("other")];
    host.editor_state_mut().selection.anchor = NodeId::new("other");
    host.mark_paint_dirty_for_test();
    press_doc(&mut host, 470.0, 130.0);
    assert_eq!(host.editor_state().selection_count(), 2, "set preserved");
    assert!(host.node_drag.is_some(), "press still drags the set");
}

#[test]
fn double_click_drills_one_level_and_third_click_does_not_chain() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, FOUR_LEVEL);
    host.set_now_ms(1_000);

    press_doc(&mut host, 470.0, 130.0);
    release(&mut host);
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("l1"));

    host.set_now_ms(1_200);
    press_doc(&mut host, 470.0, 130.0);
    release(&mut host);
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("l2"));
    assert_eq!(
        host.editor_state().editor_ui.entered_container,
        Some(NodeId::new("l1")),
        "double-click enters exactly the primary level"
    );
    assert_eq!(
        host.editor_state().editor_ui.canvas_hover_node,
        Some(NodeId::new("l2")),
        "stationary hover rebases to the newly selected second level"
    );

    host.set_now_ms(1_300);
    press_doc(&mut host, 470.0, 130.0);
    release(&mut host);
    assert_eq!(
        host.editor_state().selection.anchor,
        NodeId::new("l2"),
        "the click after a consumed double-click must not chain into l3"
    );
    assert_eq!(
        host.editor_state().editor_ui.entered_container,
        Some(NodeId::new("l1"))
    );
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
fn clicking_root_frame_label_selects_that_root() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, FOUR_LEVEL);

    press_doc(&mut host, 424.0, 42.0);

    assert_eq!(host.editor_state().selection.anchor, NodeId::new("root"));
    assert!(
        host.node_drag.is_some(),
        "label press should behave like a root press"
    );
}

#[test]
fn cursor_hover_inside_four_level_tree_resolves_to_level_one() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, FOUR_LEVEL);
    let _ = host.layout_scene();
    host.last_viewport_w = VIEWPORT_W;
    host.last_viewport_h = VIEWPORT_H;
    let (x, y) = screen_at(&host, 470.0, 130.0);

    assert!(host.apply_cursor_move(x, y));
    assert_eq!(
        host.editor_state().editor_ui.canvas_hover_node,
        Some(NodeId::new("l1"))
    );
}

#[test]
fn cursor_hover_on_frame_label_resolves_to_root() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, FOUR_LEVEL);
    let _ = host.layout_scene();
    host.last_viewport_w = VIEWPORT_W;
    host.last_viewport_h = VIEWPORT_H;
    let (x, y) = screen_at(&host, 424.0, 42.0);

    assert!(host.apply_cursor_move(x, y));
    assert_eq!(
        host.editor_state().editor_ui.canvas_hover_node,
        Some(NodeId::new("root"))
    );
}

fn overlapping_rect_stack(count: usize) -> String {
    let children = (0..count)
        .map(|i| {
            let x = if i + 1 == count {
                400.0
            } else {
                10_000.0 + i as f32 * 100.0
            };
            format!(
                r#"{{"type":"rectangle","id":"n{i}","name":"Layer {i}","x":{x},"y":60,"width":80,"height":80}}"#
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(r#"{{"version":"1.0.0","children":[{children}]}}"#)
}

#[test]
fn canvas_selection_scrolls_layer_panel_to_hidden_selected_row() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, &overlapping_rect_stack(40));
    host.editor_state_mut().editor_ui.layer_layers_scroll.offset = 0.0;
    host.mark_paint_dirty_for_test();

    press_doc(&mut host, 440.0, 100.0);

    assert_eq!(host.editor_state().selection.anchor, NodeId::new("n39"));
    assert!(
        host.editor_state().editor_ui.layer_layers_scroll.offset > 0.0,
        "selecting a canvas node below the visible layer rows should reveal it"
    );
}

#[test]
fn promotion_inside_entered_container_stops_at_its_child() {
    // card > inner (frame) > deep (rect): with card entered, a press
    // on `deep` selects `inner`, not the page-root `card`.
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[
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
const VSTACK: &str = r#"{"version":"1.0.0","children":[
  {"type":"frame","id":"stack","name":"Stack","x":400,"y":60,"width":200,"height":300,
   "layout":"vertical","gap":8,
   "children":[
     {"type":"rectangle","id":"a","name":"A","width":80,"height":40},
     {"type":"rectangle","id":"b","name":"B","width":80,"height":40},
     {"type":"rectangle","id":"c","name":"C","width":80,"height":40}
   ]}
]}"#;

const HSTACK: &str = r#"{"version":"1.0.0","children":[
  {"type":"frame","id":"row","name":"Row","x":400,"y":60,"width":360,"height":120,
   "layout":"horizontal","gap":8,
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
fn dragging_flex_child_reorders_at_midpoint_index_during_preview() {
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
    assert!(
        host.editor_state()
            .editor_ui
            .canvas_drop_indicator
            .is_none(),
        "same-container preview uses live sibling reflow, not an insertion line"
    );
    assert_eq!(
        child_order(&host, "stack"),
        vec!["b", "a", "c"],
        "siblings should avoid the dragged child during cursor move"
    );
    let overlay = host
        .node_drag
        .and_then(|drag| drag.overlay_bounds)
        .expect("same-container flex drag should paint a floating selected-node overlay");
    assert!(
        (overlay.origin.y - 140.0).abs() < 1.0,
        "overlay follows the cursor instead of the reflow slot; got {:?}",
        overlay
    );
    assert!(
        host.layout_transition
            .as_ref()
            .is_some_and(|transition| transition.is_active(host.now_ms)),
        "same-container reorder should animate sibling avoidance"
    );
    // Flex child must not doc-translate during the drag.
    let a = op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new("a"),
    )
    .expect("a present");
    assert_eq!(a.base().x, None, "no live x materialization");
    release(&mut host);
    assert!(host
        .editor_state()
        .editor_ui
        .canvas_drop_indicator
        .is_none());
    assert!(
        host.layout_transition
            .as_ref()
            .is_some_and(|transition| transition.is_active(host.now_ms)),
        "release should animate the floating dragged node into its final slot"
    );
    assert_eq!(child_order(&host, "stack"), vec!["b", "a", "c"]);
}

#[test]
fn select_tool_hovering_canvas_node_keeps_default_cursor() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, NESTED);
    let (x, y) = screen_at(&host, 660.0, 80.0);

    assert_eq!(
        host.cursor_hint(x, y, VIEWPORT_W, VIEWPORT_H),
        CursorHint::Default
    );
}

#[test]
fn placement_tool_hovering_canvas_node_keeps_default_cursor() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, NESTED);
    host.editor_state_mut().tool = Tool::Rect;
    host.mark_paint_dirty_for_test();
    let (x, y) = screen_at(&host, 660.0, 80.0);

    assert_eq!(
        host.cursor_hint(x, y, VIEWPORT_W, VIEWPORT_H),
        CursorHint::Default
    );
}

#[test]
fn active_node_drag_keeps_default_cursor() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, NESTED);
    host.node_drag = Some(NodeDragState {
        last_screen_x: 500.0,
        last_screen_y: 500.0,
        press_screen_x: 500.0,
        press_screen_y: 500.0,
        moved: true,
        total_dx: 0.0,
        total_dy: 0.0,
        overlay_bounds: None,
    });

    assert_eq!(
        host.cursor_hint(900.0, 220.0, VIEWPORT_W, VIEWPORT_H),
        CursorHint::Default
    );
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
fn option_dragging_vertical_layout_child_up_copies_before_source() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, VSTACK);
    host.editor_state_mut().editor_ui.entered_container = Some(NodeId::new("stack"));
    host.set_modifier_alt(true);
    host.mark_paint_dirty_for_test();

    press_doc(&mut host, 440.0, 128.0);
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("b"));
    let (mx, my) = screen_at(&host, 440.0, 116.0);
    host.apply_cursor_move(mx, my);
    release(&mut host);

    let order = child_order(&host, "stack");
    let clone_id = host.editor_state().selection.anchor.as_str().to_string();
    assert_ne!(clone_id, "b");
    assert_eq!(
        order,
        vec!["a".to_string(), clone_id, "b".to_string(), "c".to_string()]
    );
}

#[test]
fn option_dragging_horizontal_layout_child_left_copies_before_source() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, HSTACK);
    host.editor_state_mut().editor_ui.entered_container = Some(NodeId::new("row"));
    host.set_modifier_alt(true);
    host.mark_paint_dirty_for_test();

    press_doc(&mut host, 528.0, 80.0);
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("b"));
    let (mx, my) = screen_at(&host, 516.0, 80.0);
    host.apply_cursor_move(mx, my);
    release(&mut host);

    let order = child_order(&host, "row");
    let clone_id = host.editor_state().selection.anchor.as_str().to_string();
    assert_ne!(clone_id, "b");
    assert_eq!(
        order,
        vec!["a".to_string(), clone_id, "b".to_string(), "c".to_string()]
    );
}

#[test]
fn flex_child_dragged_to_blank_canvas_becomes_root_at_dropped_position() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, VSTACK);
    host.editor_state_mut().editor_ui.entered_container = Some(NodeId::new("stack"));
    host.mark_paint_dirty_for_test();

    press_doc(&mut host, 440.0, 80.0); // over `a`, abs origin 400,60
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("a"));
    let (mx, my) = screen_at(&host, 760.0, 80.0); // +320 doc px, outside stack
    host.apply_cursor_move(mx, my);
    assert!(
        host.editor_state()
            .editor_ui
            .canvas_drop_indicator
            .is_none(),
        "blank-canvas drag preview is the real page-root position"
    );
    let children = host.editor_state().active_children();
    assert_eq!(
        children[0].id_str(),
        "a",
        "dragged flow child leaves its source container during cursor move"
    );
    assert_eq!(child_order(&host, "stack"), vec!["b", "c"]);
    let moved = op_editor_core::walkers::find_node(children, &NodeId::new("a")).unwrap();
    assert!(
        (moved.base().x.unwrap_or(0.0) - 720.0).abs() < 1.0,
        "preview root x should use dropped bounds, got {:?}",
        moved.base().x
    );
    assert!(
        (moved.base().y.unwrap_or(0.0) - 60.0).abs() < 1.0,
        "preview root y should use dropped bounds, got {:?}",
        moved.base().y
    );
    release(&mut host);
    assert!(host
        .editor_state()
        .editor_ui
        .canvas_drop_indicator
        .is_none());

    let children = host.editor_state().active_children();
    assert_eq!(children[0].id_str(), "a", "flow child becomes a page root");
    let moved = op_editor_core::walkers::find_node(children, &NodeId::new("a")).unwrap();
    assert!(
        (moved.base().x.unwrap_or(0.0) - 720.0).abs() < 1.0,
        "root x should use dropped bounds, got {:?}",
        moved.base().x
    );
    assert!(
        (moved.base().y.unwrap_or(0.0) - 60.0).abs() < 1.0,
        "root y should use dropped bounds, got {:?}",
        moved.base().y
    );
    assert_eq!(child_order(&host, "stack"), vec!["b", "c"]);
}

#[test]
fn child_dragged_into_other_layout_uses_cross_container_placeholder() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[
          {"type":"frame","id":"src","name":"Source","x":400,"y":60,"width":200,"height":120,
           "children":[
             {"type":"rectangle","id":"box","name":"Box","x":20,"y":20,"width":50,"height":50}
           ]},
          {"type":"frame","id":"target","name":"Target","x":700,"y":60,"width":220,"height":180,
           "layout":"vertical","gap":8,
           "children":[
             {"type":"rectangle","id":"top","name":"Top","width":120,"height":40},
             {"type":"rectangle","id":"bottom","name":"Bottom","width":120,"height":40}
           ]}
        ]}"#,
    );
    host.editor_state_mut().editor_ui.entered_container = Some(NodeId::new("src"));
    host.mark_paint_dirty_for_test();

    press_doc(&mut host, 445.0, 105.0);
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("box"));
    let (mx, my) = screen_at(&host, 760.0, 120.0);
    host.apply_cursor_move(mx, my);

    let src = op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new("src"),
    )
    .unwrap();
    assert!(
        src.children().unwrap().is_empty(),
        "drag preview should remove the item from its source container"
    );
    let preview = host
        .editor_state()
        .editor_ui
        .canvas_drop_indicator
        .as_ref()
        .expect("cross-container drag keeps a target placeholder");
    assert!(preview.target.is_some());
    assert!(
        preview.insertion.is_some(),
        "auto-layout cross-container target should show an insertion line"
    );

    release(&mut host);
    assert!(host
        .editor_state()
        .editor_ui
        .canvas_drop_indicator
        .is_none());
    assert_eq!(child_order(&host, "target"), vec!["top", "box", "bottom"]);
}

#[cfg_attr(
    target_os = "windows",
    ignore = "WINDOWS_WIDGET_HOST_TEXT_DRAG_DIRECTWRITE_ABORT: text-node drag layout aborts in Windows CI; macOS and Linux keep coverage"
)]
#[test]
fn text_dragged_fully_outside_parent_reparents_to_page_root() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[
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
        "nested nodes detach to the page root at the drop target"
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
fn shape_dragged_outside_parent_becomes_page_root() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[
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
    assert_eq!(children[0].id_str(), "box", "dragged shape becomes a root");
    let card = op_editor_core::walkers::find_node(children, &NodeId::new("card")).unwrap();
    assert!(card.children().unwrap().is_empty());
    let boxn = op_editor_core::walkers::find_node(children, &NodeId::new("box")).unwrap();
    assert!(
        (boxn.base().x.unwrap_or(0.0) - 820.0).abs() < 1.0,
        "visual x preserved as root; got {:?}",
        boxn.base().x
    );
}

#[test]
fn shape_dragged_into_sibling_frame_reparents_to_that_frame() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[
          {"type":"frame","id":"src","name":"Source","x":400,"y":60,"width":200,"height":120,
           "children":[
             {"type":"rectangle","id":"box","name":"Box","x":20,"y":20,"width":50,"height":50}
           ]},
          {"type":"frame","id":"target","name":"Target","x":700,"y":60,"width":220,"height":160,
           "children":[]}
        ]}"#,
    );
    host.editor_state_mut().editor_ui.entered_container = Some(NodeId::new("src"));
    host.mark_paint_dirty_for_test();

    press_doc(&mut host, 445.0, 105.0); // box center: abs origin 420,80
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("box"));
    let (mx, my) = screen_at(&host, 760.0, 100.0); // inside `target`
    host.apply_cursor_move(mx, my);
    release(&mut host);

    let children = host.editor_state().active_children();
    let src = op_editor_core::walkers::find_node(children, &NodeId::new("src")).unwrap();
    assert!(src.children().unwrap().is_empty());
    let target = op_editor_core::walkers::find_node(children, &NodeId::new("target")).unwrap();
    let moved = &target.children().unwrap()[0];
    assert_eq!(moved.id_str(), "box");
    assert!(
        (moved.base().x.unwrap_or(0.0) - 35.0).abs() < 1.0,
        "visual x preserved relative to target; got {:?}",
        moved.base().x
    );
    assert!(
        (moved.base().y.unwrap_or(0.0) - 15.0).abs() < 1.0,
        "visual y preserved relative to target; got {:?}",
        moved.base().y
    );
}

#[test]
fn root_shape_dragged_into_frame_becomes_that_frame_child() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[
          {"type":"rectangle","id":"box","name":"Box","x":400,"y":80,"width":50,"height":50},
          {"type":"frame","id":"target","name":"Target","x":700,"y":60,"width":220,"height":160,
           "children":[]}
        ]}"#,
    );
    host.mark_paint_dirty_for_test();

    press_doc(&mut host, 425.0, 105.0); // box center
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("box"));
    let (mx, my) = screen_at(&host, 760.0, 100.0);
    host.apply_cursor_move(mx, my);
    release(&mut host);

    let children = host.editor_state().active_children();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].id_str(), "target");
    let target = op_editor_core::walkers::find_node(children, &NodeId::new("target")).unwrap();
    let moved = &target.children().unwrap()[0];
    assert_eq!(moved.id_str(), "box");
    assert!(
        (moved.base().x.unwrap_or(0.0) - 35.0).abs() < 1.0,
        "root visual x preserved relative to target; got {:?}",
        moved.base().x
    );
    assert!(
        (moved.base().y.unwrap_or(0.0) - 15.0).abs() < 1.0,
        "root visual y preserved relative to target; got {:?}",
        moved.base().y
    );
}
