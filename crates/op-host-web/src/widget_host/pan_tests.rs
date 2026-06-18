use super::WidgetHost;
use op_editor_core::Tool;
use op_editor_ui::Point2D;

const VIEWPORT_W: f32 = 1200.0;
const VIEWPORT_H: f32 = 800.0;

fn canvas_point(host: &WidgetHost, x: f32, y: f32) -> Point2D {
    let (cx0, cy0, _, _) = host.canvas_region(VIEWPORT_W, VIEWPORT_H);
    Point2D::new(cx0 + x, cy0 + y)
}

#[test]
fn hand_tool_drag_pans_the_viewport() {
    let mut host = WidgetHost::new();
    host.editor_state.tool = Tool::Hand;

    let start = canvas_point(&host, 420.0, 260.0);
    let end = canvas_point(&host, 470.0, 295.0);

    let _ = host.apply_press(start.x, start.y, VIEWPORT_W, VIEWPORT_H);
    assert!(host.apply_cursor_move(end.x, end.y));
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));

    assert_eq!(host.editor_state.viewport.pan_x, 50.0);
    assert_eq!(host.editor_state.viewport.pan_y, 35.0);
}

#[test]
fn space_pan_drag_pans_even_when_select_tool_is_active() {
    let mut host = WidgetHost::new();
    host.editor_state.tool = Tool::Select;
    host.set_space_pan(true);

    let start = canvas_point(&host, 420.0, 260.0);
    let end = canvas_point(&host, 470.0, 295.0);

    let _ = host.apply_press(start.x, start.y, VIEWPORT_W, VIEWPORT_H);
    assert!(host.apply_cursor_move(end.x, end.y));
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));

    assert_eq!(host.editor_state.viewport.pan_x, 50.0);
    assert_eq!(host.editor_state.viewport.pan_y, 35.0);
    assert!(host.marquee_drag.is_none());
}

#[test]
fn horizontal_trackpad_pan_moves_canvas_viewport() {
    let mut host = WidgetHost::new();
    let point = canvas_point(&host, 420.0, 260.0);

    assert!(host.apply_pan_gesture(point.x, point.y, -120.0, 0.0, VIEWPORT_W, VIEWPORT_H));

    assert_eq!(host.editor_state.viewport.pan_x, -120.0);
    assert_eq!(host.editor_state.viewport.pan_y, 0.0);
}

fn nested_frame_doc(depth: usize) -> String {
    let mut src = String::from(r#"{"version":"0.8.0","children":["#);
    for i in 0..depth {
        src.push_str(&format!(
            r##"{{"type":"frame","id":"nest-{i:05}","name":"Nested Layer {i:05}","x":8,"y":6,"width":400,"height":220,"fill":[{{"type":"solid","color":"#ffffff20"}}],"stroke":{{"thickness":1,"fill":[{{"type":"solid","color":"#0088ff"}}]}},"children":["##
        ));
    }
    for _ in 0..depth {
        src.push_str("]}");
    }
    src.push_str("]}");
    src
}

fn seed(host: &mut WidgetHost, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.mark_dirty();
}

fn run_deep_layer_fixture(test: impl FnOnce() + Send + 'static) {
    let handle = std::thread::Builder::new()
        .name("op-host-web-deep-layer-fixture".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(test)
        .expect("spawn deep layer fixture test");
    if let Err(payload) = handle.join() {
        std::panic::resume_unwind(payload);
    }
}

#[test]
fn layer_panel_trackpad_pan_scrolls_horizontally() {
    run_deep_layer_fixture(|| {
        let mut host = WidgetHost::new();
        seed(&mut host, &nested_frame_doc(50));
        let panel = op_editor_ui::widgets::LayerPanel::from_editor(host.editor_state());
        let rect = op_editor_ui::Rect {
            origin: op_editor_ui::Point2D::new(0.0, op_editor_ui::widgets::TOP_BAR_HEIGHT),
            size: op_editor_ui::Point2D::new(
                host.editor_state().editor_ui.layer_panel_width,
                VIEWPORT_H - op_editor_ui::widgets::TOP_BAR_HEIGHT,
            ),
        };
        let regions = panel.regions(rect);
        assert!(regions.layers.max_horizontal_offset > 0.0);

        assert!(host.apply_pan_gesture(
            80.0,
            regions.layers_rows_top + 12.0,
            -180.0,
            0.0,
            VIEWPORT_W,
            VIEWPORT_H
        ));

        assert!(host.editor_state().editor_ui.layer_layers_h_scroll.offset > 0.0);
    });
}

#[test]
fn middle_pan_press_starts_canvas_pan_without_primary_press_dispatch() {
    let mut host = WidgetHost::new();
    host.editor_state.tool = Tool::Select;

    let start = canvas_point(&host, 420.0, 260.0);
    let end = canvas_point(&host, 450.0, 285.0);

    assert!(host.apply_pan_press(start.x, start.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.apply_cursor_move(end.x, end.y));
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));

    assert_eq!(host.editor_state.viewport.pan_x, 30.0);
    assert_eq!(host.editor_state.viewport.pan_y, 25.0);
    assert!(host.marquee_drag.is_none());
}
