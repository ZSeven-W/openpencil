//! The motion timeline through the web ladders: its resize grips set the
//! canvas cursor, M toggles it, and Escape drops a drag in flight.

use super::WidgetHost;
use op_editor_core::EditorState;
use op_editor_ui::widgets::host_canvas_geometry::motion_timeline_rect;
use op_editor_ui::widgets::motion_timeline::{MotionTimelineView, TimelineLayout};

const VW: f32 = 1440.0;
const VH: f32 = 1000.0;

fn host() -> WidgetHost {
    let doc = jian_ops_schema::load_str(
        r##"{
            "version":"1.0",
            "children":[{"type":"frame","id":"home","width":390,"height":844,"children":[
                {"type":"rectangle","id":"card","width":100,"height":100,"animations":[{
                    "trigger":"mount","durationMs":400,"delayMs":100,"easing":"linear",
                    "keyframes":[
                        {"offset":0,"values":{"opacity":0}},
                        {"offset":1,"values":{"opacity":1}}
                    ]
                }]}
            ]}]
        }"##,
    )
    .expect("fixture parses")
    .value;
    let mut host = WidgetHost::new();
    host.editor_state = EditorState::from_document(doc);
    host.last_viewport_w = VW;
    host.last_viewport_h = VH;
    host.editor_state.toggle_motion_timeline();
    host
}

fn timeline_layout(host: &WidgetHost) -> TimelineLayout {
    let state = &host.editor_state;
    let rect = motion_timeline_rect(state, VW, VH).expect("the panel is shown");
    TimelineLayout::new(rect, &MotionTimelineView::for_editor(state))
}

#[test]
fn the_resize_grips_set_the_canvas_cursor() {
    let host = host();
    let layout = timeline_layout(&host);
    assert_eq!(
        host.motion_timeline_cursor_css(700.0, layout.rect.origin.y),
        "ns-resize"
    );
    let grip = layout.rows[0].end_handle;
    assert_eq!(
        host.motion_timeline_cursor_css(
            grip.origin.x + grip.size.x / 2.0,
            grip.origin.y + grip.size.y / 2.0
        ),
        "ew-resize"
    );
    assert_eq!(host.motion_timeline_cursor_css(700.0, 300.0), "");
}

#[test]
fn m_toggles_the_timeline() {
    let mut host = host();
    assert!(host.apply_tool_shortcut("m"));
    assert!(!host.editor_state.editor_ui.motion_timeline.open);
    assert!(host.apply_tool_shortcut("M"));
    assert!(host.editor_state.editor_ui.motion_timeline.open);
}

#[test]
fn escape_cancels_a_timeline_drag_without_writing() {
    let mut host = host();
    let row = timeline_layout(&host).rows[0].clone();
    let grab = op_editor_ui::Point2D::new(
        (row.keyframes[0].1.x + row.keyframes[1].1.x) / 2.0,
        row.keyframes[0].1.y,
    );
    assert!(host.apply_press(grab.x, grab.y, VW, VH));
    host.apply_cursor_move(grab.x + 60.0, grab.y);
    assert!(host.apply_escape());
    assert!(host.editor_state.editor_ui.motion_timeline.drag.is_none());
    host.apply_release_with_viewport(VW, VH);
    let delay = op_editor_core::walkers::find_node(
        host.editor_state.active_children(),
        &op_editor_core::NodeId::new("card"),
    )
    .and_then(|node| node.motion_declarations().1.map(|a| a[0].delay_ms));
    assert_eq!(delay, Some(100), "nothing was written");
}

#[test]
fn browser_wheel_and_two_axis_pan_are_owned_by_the_timeline() {
    let rows:Vec<_>=(0..40).map(|index|serde_json::json!({
        "type":"rectangle","id":format!("track-{index}"),"width":20,"height":20,
        "animations":[{"trigger":"mount","durationMs":182_310,
            "keyframes":[{"offset":0,"values":{"opacity":0}},{"offset":1,"values":{"opacity":1}}]}]
    })).collect();
    let document=jian_ops_schema::load_str(&serde_json::json!({
        "version":"1.1","children":[{"type":"frame","id":"home","width":390,"height":844,"children":rows}]
    }).to_string()).unwrap().value;
    let mut host = host();
    host.editor_state = EditorState::from_document(document);
    host.editor_state.toggle_motion_timeline();
    let first = timeline_layout(&host);
    let (x, y) = (first.lanes.origin.x + 160.0, first.lanes.origin.y + 30.0);
    let viewport = host.editor_state.viewport;
    assert!(host.apply_wheel(x, y, -80.0, VW, VH));
    assert_eq!(
        host.editor_state
            .editor_ui
            .motion_timeline
            .vertical_scroll
            .offset,
        80.0
    );
    assert!(host.apply_pan_gesture(x, y, -4100.0, -70.0, VW, VH));
    assert_eq!(
        host.editor_state
            .editor_ui
            .motion_timeline
            .horizontal_scroll
            .offset,
        4100.0
    );
    assert_eq!(
        host.editor_state
            .editor_ui
            .motion_timeline
            .vertical_scroll
            .offset,
        150.0
    );
    let ruler = timeline_layout(&host).ruler;
    assert!(host.apply_wheel(x, ruler.origin.y + 8.0, -90.0, VW, VH));
    assert_eq!(
        host.editor_state
            .editor_ui
            .motion_timeline
            .horizontal_scroll
            .offset,
        4190.0
    );
    assert_eq!(
        host.editor_state
            .editor_ui
            .motion_timeline
            .vertical_scroll
            .offset,
        150.0
    );
    assert_eq!(host.editor_state.viewport, viewport);
    assert!(host.apply_pan_gesture(x, y, -1e6, -1e6, VW, VH));
    let last = timeline_layout(&host);
    assert_eq!(last.scroll_x, last.max_scroll_x);
    assert_eq!(last.scroll_y, last.max_scroll_y);
    assert!(host.apply_wheel(x, y, -90.0, VW, VH));
    assert_eq!(host.editor_state.viewport, viewport);
}
