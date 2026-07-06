use super::WidgetHost;
use jian_ops_schema::node::PenNode;
use op_editor_core::{own_bounds, walkers::find_node, NodeId, PenNodeExt, Tool};
use op_editor_ui::Point2D;

const VW: f32 = 1200.0;
const VH: f32 = 800.0;

fn seed(host: &mut WidgetHost) {
    seed_json(
        host,
        r##"{"version":"0.8.0","children":[
          {"type":"rectangle","id":"box","name":"Box","x":100,"y":100,"width":120,"height":80,
           "fill":[{"type":"solid","color":"#2563EB"}]}
        ]}"##,
    );
}

fn seed_json(host: &mut WidgetHost, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    host.editor_state = op_editor_core::EditorState::from_document(doc);
    host.editor_state.tool = Tool::Select;
    host.editor_state_dirty = true;
}

fn screen(host: &WidgetHost, doc_x: f32, doc_y: f32) -> Point2D {
    let (cx0, cy0, _, _) = host.canvas_region(VW, VH);
    Point2D::new(cx0 + doc_x, cy0 + doc_y)
}

fn child_order(host: &WidgetHost, parent: &str) -> Vec<String> {
    find_node(host.editor_state.active_children(), &NodeId::new(parent))
        .and_then(|n| n.children())
        .map(|cs| cs.iter().map(|c| c.id_str().to_string()).collect())
        .unwrap_or_default()
}

fn box_bounds(host: &WidgetHost) -> op_editor_core::DocRect {
    let node = find_node(host.editor_state.active_children(), &NodeId::new("box"))
        .expect("box remains in document");
    match node {
        PenNode::Rectangle(_) => own_bounds(node),
        _ => panic!("fixture box is not a rectangle"),
    }
}

fn scene_origin(host: &WidgetHost, id: &str) -> Point2D {
    host.layout_scene
        .active_page()
        .and_then(|page| page.find(id))
        .expect("node present in scene")
        .bounds
        .origin
}

#[test]
fn select_tool_dragging_selected_node_moves_it_without_resizing() {
    let mut host = WidgetHost::new();
    seed(&mut host);

    let press = screen(&host, 160.0, 140.0);
    let move_to = screen(&host, 200.0, 165.0);

    assert!(host.apply_press(press.x, press.y, VW, VH));
    assert!(host.apply_cursor_move(move_to.x, move_to.y));
    assert!(host.apply_release_with_viewport(VW, VH));

    let bounds = box_bounds(&host);
    assert_eq!(bounds.x, 140.0);
    assert_eq!(bounds.y, 125.0);
    assert_eq!(bounds.w, 120.0);
    assert_eq!(bounds.h, 80.0);
}

#[test]
fn option_dragging_selected_node_duplicates_and_moves_the_clone() {
    let mut host = WidgetHost::new();
    seed(&mut host);

    host.set_modifier_alt(true);
    let press = screen(&host, 160.0, 140.0);
    let move_to = screen(&host, 200.0, 165.0);

    assert!(host.apply_press(press.x, press.y, VW, VH));
    assert_eq!(host.editor_state.selection.anchor, NodeId::new("box"));
    assert!(host.apply_cursor_move(move_to.x, move_to.y));
    assert_eq!(host.editor_state.active_children().len(), 2);
    assert!(host.apply_release_with_viewport(VW, VH));

    let children = host.editor_state.active_children();
    assert_eq!(children.len(), 2);
    let original = find_node(children, &NodeId::new("box")).expect("original remains");
    assert_eq!(own_bounds(original).x, 100.0);
    assert_eq!(own_bounds(original).y, 100.0);

    let clone_id = host.editor_state.selection.anchor.clone();
    assert_ne!(clone_id, NodeId::new("box"));
    let clone = find_node(children, &clone_id).expect("clone selected");
    let clone_bounds = own_bounds(clone);
    assert_eq!(clone_bounds.x, 140.0);
    assert_eq!(clone_bounds.y, 125.0);
    assert_eq!(clone_bounds.w, 120.0);
    assert_eq!(clone_bounds.h, 80.0);
}

#[test]
fn arrow_nudge_reorders_selected_child_on_layout_axis() {
    let mut host = WidgetHost::new();
    seed_json(
        &mut host,
        r#"{"version":"0.8.0","children":[{
          "type":"frame","id":"stack","name":"Stack","x":400,"y":60,"width":200,"height":300,
          "layout":"vertical","gap":8,
          "children":[
            {"type":"rectangle","id":"a","name":"A","width":80,"height":40},
            {"type":"rectangle","id":"b","name":"B","width":80,"height":40},
            {"type":"rectangle","id":"c","name":"C","width":80,"height":40}
          ]}
        ]}"#,
    );
    host.editor_state.set_single_selection(NodeId::new("b"));

    assert!(host.apply_nudge(0.0, 1.0));

    assert_eq!(child_order(&host, "stack"), vec!["a", "c", "b"]);
    assert_eq!(host.editor_state.selection.anchor, NodeId::new("b"));
}

#[test]
fn select_tool_dragging_flex_child_reorders_on_release_like_native() {
    let mut host = WidgetHost::new();
    let doc = jian_ops_schema::load_str(
        r#"{"version":"0.8.0","children":[{
          "type":"frame","id":"stack","name":"Stack","x":400,"y":60,"width":200,"height":300,
          "layout":"vertical","gap":8,
          "children":[
            {"type":"rectangle","id":"a","name":"A","width":80,"height":40},
            {"type":"rectangle","id":"b","name":"B","width":80,"height":40},
            {"type":"rectangle","id":"c","name":"C","width":80,"height":40}
          ]}
        ]}"#,
    )
    .expect("fixture JSON parses")
    .value;
    host.editor_state = op_editor_core::EditorState::from_document(doc);
    host.editor_state.tool = Tool::Select;
    host.editor_state.editor_ui.entered_container = Some(NodeId::new("stack"));
    host.editor_state_dirty = true;

    let press = screen(&host, 440.0, 80.0);
    assert!(host.apply_press(press.x, press.y, VW, VH));
    assert_eq!(host.editor_state.selection.anchor, NodeId::new("a"));

    let move_to = screen(&host, 440.0, 160.0);
    assert!(host.apply_cursor_move(move_to.x, move_to.y));
    let preview = host
        .editor_state
        .editor_ui
        .canvas_drop_indicator
        .as_ref()
        .expect("dragging a flex child paints a drop preview");
    assert!(preview.target.is_some());
    assert!(preview.insertion.is_some());

    let a = find_node(host.editor_state.active_children(), &NodeId::new("a")).expect("a present");
    assert_eq!(
        a.base().x,
        None,
        "flex child must not materialize x during drag"
    );

    assert!(host.apply_release_with_viewport(VW, VH));
    assert!(host.editor_state.editor_ui.canvas_drop_indicator.is_none());

    assert_eq!(child_order(&host, "stack"), vec!["b", "a", "c"]);
}

#[test]
fn select_tool_dragging_flex_child_to_blank_canvas_makes_root_at_dropped_position() {
    let mut host = WidgetHost::new();
    seed_json(
        &mut host,
        r#"{"version":"0.8.0","children":[{
          "type":"frame","id":"stack","name":"Stack","x":400,"y":60,"width":200,"height":300,
          "layout":"vertical","gap":8,
          "children":[
            {"type":"rectangle","id":"a","name":"A","width":80,"height":40},
            {"type":"rectangle","id":"b","name":"B","width":80,"height":40},
            {"type":"rectangle","id":"c","name":"C","width":80,"height":40}
          ]}
        ]}"#,
    );
    host.editor_state.editor_ui.entered_container = Some(NodeId::new("stack"));

    let press = screen(&host, 440.0, 80.0);
    assert!(host.apply_press(press.x, press.y, VW, VH));
    assert_eq!(host.editor_state.selection.anchor, NodeId::new("a"));
    let move_to = screen(&host, 760.0, 80.0);
    assert!(host.apply_cursor_move(move_to.x, move_to.y));
    let preview = host
        .editor_state
        .editor_ui
        .canvas_drop_indicator
        .as_ref()
        .expect("drag out paints a root-drop ghost");
    assert!(preview.target.is_none());
    assert!(preview.insertion.is_none());
    assert!((preview.ghost.x - 720.0).abs() < 1.0);
    assert!((preview.ghost.y - 60.0).abs() < 1.0);
    assert!(host.apply_release_with_viewport(VW, VH));
    assert!(host.editor_state.editor_ui.canvas_drop_indicator.is_none());

    let children = host.editor_state.active_children();
    assert_eq!(children[0].id_str(), "a");
    let moved = find_node(children, &NodeId::new("a")).unwrap();
    assert!((moved.base().x.unwrap_or(0.0) - 720.0).abs() < 1.0);
    assert!((moved.base().y.unwrap_or(0.0) - 60.0).abs() < 1.0);
    assert_eq!(child_order(&host, "stack"), vec!["b", "c"]);
}

#[test]
fn select_tool_dragging_fill_sized_child_to_blank_canvas_freezes_resolved_size() {
    let mut host = WidgetHost::new();
    seed_json(
        &mut host,
        r#"{"version":"0.8.0","children":[{
          "type":"frame","id":"screen","name":"Screen","x":400,"y":60,"width":360,"height":220,
          "layout":"vertical","gap":8,
          "children":[
            {"type":"rectangle","id":"box","name":"Box","width":"fill_container","height":72}
          ]}
        ]}"#,
    );
    host.editor_state.editor_ui.entered_container = Some(NodeId::new("screen"));

    let press = screen(&host, 420.0, 80.0);
    assert!(host.apply_press(press.x, press.y, VW, VH));
    assert_eq!(host.editor_state.selection.anchor, NodeId::new("box"));
    let move_to = screen(&host, 840.0, 80.0);
    assert!(host.apply_cursor_move(move_to.x, move_to.y));
    let preview = host
        .editor_state
        .editor_ui
        .canvas_drop_indicator
        .as_ref()
        .expect("drag out paints a root-drop ghost");
    let expected_w = preview.ghost.w;
    let expected_h = preview.ghost.h;
    assert!(preview.target.is_none());

    assert!(host.apply_release_with_viewport(VW, VH));

    let moved = find_node(host.editor_state.active_children(), &NodeId::new("box")).unwrap();
    assert!(
        (moved.width_px().unwrap_or(0.0) - expected_w).abs() < 1.0,
        "root width should freeze to dragged width {expected_w}, got {:?}",
        moved.width_px()
    );
    assert!(
        (moved.height_px().unwrap_or(0.0) - expected_h).abs() < 1.0,
        "root height should freeze to dragged height {expected_h}, got {:?}",
        moved.height_px()
    );
}

#[test]
fn select_tool_dragging_child_into_sibling_frame_reparents_like_native() {
    let mut host = WidgetHost::new();
    seed_json(
        &mut host,
        r#"{"version":"0.8.0","children":[
          {"type":"frame","id":"src","name":"Source","x":400,"y":60,"width":200,"height":120,
           "children":[
             {"type":"rectangle","id":"box","name":"Box","x":20,"y":20,"width":50,"height":50}
           ]},
          {"type":"frame","id":"target","name":"Target","x":700,"y":60,"width":220,"height":160,
           "children":[]}
        ]}"#,
    );
    host.editor_state.editor_ui.entered_container = Some(NodeId::new("src"));

    let press = screen(&host, 445.0, 105.0);
    assert!(host.apply_press(press.x, press.y, VW, VH));
    assert_eq!(host.editor_state.selection.anchor, NodeId::new("box"));
    let move_to = screen(&host, 760.0, 100.0);
    assert!(host.apply_cursor_move(move_to.x, move_to.y));
    assert!(host.apply_release_with_viewport(VW, VH));

    let src = find_node(host.editor_state.active_children(), &NodeId::new("src")).unwrap();
    assert!(src.children().unwrap().is_empty());
    let target = find_node(host.editor_state.active_children(), &NodeId::new("target")).unwrap();
    let moved = &target.children().unwrap()[0];
    assert_eq!(moved.id_str(), "box");
    assert!((moved.base().x.unwrap_or(0.0) - 35.0).abs() < 1.0);
    assert!((moved.base().y.unwrap_or(0.0) - 15.0).abs() < 1.0);
}

#[test]
fn select_tool_dragging_child_to_blank_canvas_makes_it_page_root_like_native() {
    let mut host = WidgetHost::new();
    seed_json(
        &mut host,
        r#"{"version":"0.8.0","children":[
          {"type":"frame","id":"card","name":"Card","x":400,"y":60,"width":200,"height":100,
           "children":[
             {"type":"rectangle","id":"box","name":"Box","x":20,"y":20,"width":50,"height":50}
           ]}
        ]}"#,
    );
    host.editor_state.editor_ui.entered_container = Some(NodeId::new("card"));

    let press = screen(&host, 440.0, 100.0);
    assert!(host.apply_press(press.x, press.y, VW, VH));
    let move_to = screen(&host, 840.0, 100.0);
    assert!(host.apply_cursor_move(move_to.x, move_to.y));
    assert!(host.apply_release_with_viewport(VW, VH));

    let children = host.editor_state.active_children();
    assert_eq!(children[0].id_str(), "box");
    let card = find_node(children, &NodeId::new("card")).unwrap();
    assert!(card.children().unwrap().is_empty());
    let moved = find_node(children, &NodeId::new("box")).unwrap();
    assert!((moved.base().x.unwrap_or(0.0) - 820.0).abs() < 1.0);
    assert!((moved.base().y.unwrap_or(0.0) - 80.0).abs() < 1.0);
}

#[test]
fn dragging_a_selection_with_a_locked_node_does_not_drift_it_in_the_scene() {
    let mut host = WidgetHost::new();
    let doc = jian_ops_schema::load_str(
        r##"{"version":"0.8.0","children":[
          {"type":"rectangle","id":"free","name":"Free","x":100,"y":100,"width":80,"height":60},
          {"type":"rectangle","id":"locked","name":"Locked","x":300,"y":100,"width":80,"height":60,"locked":true}
        ]}"##,
    )
    .expect("fixture JSON parses")
    .value;
    host.editor_state = op_editor_core::EditorState::from_document(doc);
    host.editor_state.tool = Tool::Select;

    // Press the free node to start a drag, then widen the selection to include
    // the locked node — the incremental fast path reads the live selection at
    // move time.
    let press = screen(&host, 140.0, 130.0);
    assert!(host.apply_press(press.x, press.y, VW, VH));
    host.editor_state.selection.set = vec![NodeId::new("free"), NodeId::new("locked")];

    // Force-resolve the scene, then clear dirty so the move takes the
    // incremental path (a real frame would have painted + cleared the flag).
    host.editor_state_dirty = true;
    host.refresh_layout_scene();
    host.editor_state_dirty = false;

    let free_before = scene_origin(&host, "free");
    let locked_before = scene_origin(&host, "locked");

    let move_to = screen(&host, 200.0, 130.0); // +60 doc px in x
    assert!(host.apply_cursor_move(move_to.x, move_to.y));

    let free_after = scene_origin(&host, "free");
    let locked_after = scene_origin(&host, "locked");

    // The editable node tracks the drag in the scene...
    assert!(
        (free_after.x - free_before.x).abs() > 1.0,
        "editable node should move in the scene during the drag"
    );
    // ...the locked node, which `translate_selected` leaves untouched, must not
    // drift — otherwise it would jump and then snap back on the release-time
    // reconversion.
    assert_eq!(
        locked_after.x, locked_before.x,
        "locked node must not drift in x"
    );
    assert_eq!(
        locked_after.y, locked_before.y,
        "locked node must not drift in y"
    );
}
