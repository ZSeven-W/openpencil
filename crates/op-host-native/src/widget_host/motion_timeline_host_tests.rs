//! The motion timeline through the native ladders: a press lands on the
//! panel before the canvas tiers, the drag holds the pointer, the release
//! writes one undo step, and Delete takes selected keyframes before the
//! canvas selection.

use super::WidgetHostNative;
use op_editor_core::EditorState;
use op_editor_ui::widgets::host_canvas_geometry::motion_timeline_rect;
use op_editor_ui::widgets::motion_timeline::{MotionTimelineView, TimelineLayout};

const VW: f32 = 1440.0;
const VH: f32 = 1000.0;

fn host() -> WidgetHostNative {
    let doc = jian_ops_schema::load_str(
        r##"{
            "version":"1.0",
            "children":[{"type":"frame","id":"home","width":390,"height":844,"children":[
                {"type":"rectangle","id":"card","width":100,"height":100,"animations":[{
                    "trigger":"mount","durationMs":400,"delayMs":100,"easing":"linear",
                    "keyframes":[
                        {"offset":0,"values":{"opacity":0}},
                        {"offset":0.5,"values":{"opacity":0.5}},
                        {"offset":1,"values":{"opacity":1}}
                    ]
                }]}
            ]}]
        }"##,
    )
    .expect("fixture parses")
    .value;
    let mut host = WidgetHostNative::new();
    host.install_imported_state(EditorState::from_document(doc));
    host.last_viewport_w = VW;
    host.last_viewport_h = VH;
    host.editor_state_mut().toggle_motion_timeline();
    host
}

fn layout(host: &WidgetHostNative) -> TimelineLayout {
    let state = host.editor_state();
    let rect = motion_timeline_rect(state, VW, VH).expect("the panel is shown");
    TimelineLayout::new(rect, &MotionTimelineView::for_editor(state))
}

fn card_animation(host: &WidgetHostNative) -> jian_ops_schema::motion::NodeAnimation {
    op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &op_editor_core::NodeId::new("card"),
    )
    .and_then(|node| {
        node.motion_declarations()
            .1
            .map(|animations| animations[0].clone())
    })
    .expect("card animates")
}

#[test]
fn a_strip_drag_moves_the_delay_in_one_undo_step() {
    let mut host = host();
    let row = layout(&host).rows[0].clone();
    let grab = op_editor_ui::Point2D::new(
        (row.keyframes[0].x + row.keyframes[1].x) / 2.0,
        row.keyframes[0].y,
    );
    assert!(host.apply_press(grab.x, grab.y, VW, VH));
    let zoom = host.editor_state().editor_ui.motion_timeline.zoom;
    // Over the canvas now: the drag keeps the pointer.
    host.apply_cursor_move(grab.x + 200.0 * zoom, grab.y - 300.0);
    assert_eq!(card_animation(&host).delay_ms, 100);
    host.apply_release_with_viewport(VW, VH);
    assert_eq!(card_animation(&host).delay_ms, 300);
    assert!(host.editor_state().editor_ui.motion_timeline.drag.is_none());
    assert!(host.editor_state_mut().undo());
    assert_eq!(card_animation(&host).delay_ms, 100);
}

#[test]
fn delete_takes_selected_keyframes_before_the_canvas_selection() {
    let mut host = host();
    let _ = host
        .editor_state_mut()
        .apply(op_editor_core::EditorCommand::SetSelection {
            node_id: op_editor_core::NodeId::new("card"),
        });
    let middle = layout(&host).rows[0].keyframes[1];
    host.apply_press(middle.x, middle.y, VW, VH);
    host.apply_release_with_viewport(VW, VH);
    assert!(host.apply_delete());
    assert_eq!(card_animation(&host).keyframes.len(), 2);
    // The node is still there and still selected; the next Delete is the
    // canvas's again.
    assert_eq!(host.editor_state().selection.anchor.as_str(), "card");
    assert!(host
        .editor_state()
        .editor_ui
        .motion_timeline
        .selection
        .is_empty());
}
