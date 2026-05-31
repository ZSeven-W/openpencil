use super::WidgetHost;
use op_editor_core::{walkers::find_node, BooleanOp, EditorState, NodeId, PenNodeExt, Tool};
use op_editor_ui::widgets::{AlignToolbar, AlignToolbarHit, TOP_BAR_HEIGHT};
use op_editor_ui::{Point2D, Rect};

fn seed_two_selected_rects(host: &mut WidgetHost) -> (NodeId, NodeId) {
    let mut state = EditorState::starter();
    state.active_children_mut().clear();
    let mut next_id = 100;
    let a = state
        .create_node_for_tool(Tool::Rect, &mut next_id, 0.0, 0.0, 120.0, 80.0)
        .expect("first rect");
    let b = state
        .create_node_for_tool(Tool::Rect, &mut next_id, 40.0, 0.0, 120.0, 80.0)
        .expect("second rect");
    state.selection.set = vec![a.clone(), b.clone()];
    state.selection.anchor = b.clone();
    host.editor_state = state;
    host.editor_state_dirty = true;
    (a, b)
}

fn toolbar_point_for(
    host: &WidgetHost,
    viewport_w: f32,
    viewport_h: f32,
    hit: AlignToolbarHit,
) -> Point2D {
    let (cx, _, cw, ch) = host.canvas_region(viewport_w, viewport_h);
    let canvas_region = Rect {
        origin: Point2D::new(cx, TOP_BAR_HEIGHT),
        size: Point2D::new(cw, ch),
    };
    let toolbar = AlignToolbar::for_canvas_region(canvas_region, &host.editor_state)
        .expect("selection toolbar");
    let rect = toolbar.rect();
    let min_x = rect.origin.x.ceil() as i32;
    let max_x = (rect.origin.x + rect.size.x).floor() as i32;
    let min_y = rect.origin.y.ceil() as i32;
    let max_y = (rect.origin.y + rect.size.y).floor() as i32;
    for y in min_y..max_y {
        for x in min_x..max_x {
            let p = Point2D::new(x as f32 + 0.5, y as f32 + 0.5);
            if toolbar.hit_test_action(p) == Some(hit) {
                return p;
            }
        }
    }
    panic!("toolbar did not expose {hit:?}");
}

#[test]
fn pressing_boolean_toolbar_union_commits_path_result() {
    let mut host = WidgetHost::new();
    let (a, b) = seed_two_selected_rects(&mut host);
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    let p = toolbar_point_for(
        &host,
        viewport_w,
        viewport_h,
        AlignToolbarHit::Boolean(BooleanOp::Union),
    );

    assert!(host.apply_press(p.x, p.y, viewport_w, viewport_h));

    let children = host.editor_state.active_children();
    assert!(find_node(children, &a).is_none());
    assert!(find_node(children, &b).is_none());
    assert_eq!(children.len(), 1);
    assert_eq!(host.editor_state.selection_count(), 1);
    let selected = host.editor_state.selection.anchor.clone();
    let result = find_node(children, &selected).expect("boolean result is selected");
    assert_eq!(result.base().name.as_deref(), Some("Boolean Result"));
}
