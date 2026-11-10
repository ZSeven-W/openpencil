use super::*;
use serde_json::json;

const VW: f32 = 1440.0;
const VH: f32 = 1000.0;

fn animation(trigger: &str, id: Option<&str>, delay: u64) -> serde_json::Value {
    let mut value = json!({
        "trigger": trigger, "durationMs": 400, "delayMs": delay, "easing": "linear",
        "keyframes": [
            {"offset": 0, "values": {"opacity": 0}},
            {"offset": 0.5, "values": {"opacity": 0.5}},
            {"offset": 1, "values": {"opacity": 1}}
        ]
    });
    if let Some(id) = id {
        value["id"] = json!(id);
    }
    value
}

fn state() -> EditorState {
    let doc = jian_ops_schema::load_str(
        &json!({
            "version": "1.0",
            "children": [{"type": "frame", "id": "home", "width": 390, "height": 844, "children": [
                {"type": "rectangle", "id": "a", "width": 10, "height": 10, "animations": [
                    animation("mount", None, 0),
                    animation("manual", Some("pulse"), 0),
                    animation("manual", Some("shake"), 0)
                ]},
                {"type": "rectangle", "id": "b", "width": 10, "height": 10,
                 "animations": [animation("mount", None, 100)]}
            ]}]
        })
        .to_string(),
    )
    .expect("document")
    .value;
    let mut state = EditorState::from_document(doc);
    state.toggle_motion_timeline();
    state
}

fn layout_of(state: &EditorState) -> (MotionTimelineView, TimelineLayout) {
    layout(state, VW, VH).expect("the panel is shown")
}

fn center(rect: crate::Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn delay(state: &EditorState, node: &str) -> u64 {
    let node = op_editor_core::walkers::find_node(
        state.active_children(),
        &op_editor_core::NodeId::new(node),
    )
    .expect("node");
    node.motion_declarations().1.expect("animations")[0].delay_ms
}

#[test]
fn header_presses_drive_playback_zoom_and_tabs() {
    let mut state = state();
    assert!(!press(
        &mut state,
        Point2D::new(10.0, 10.0),
        VW,
        VH,
        0,
        false
    ));
    let (_, layout) = layout_of(&state);
    assert!(press(&mut state, center(layout.play), VW, VH, 0, false));
    assert!(state.editor_ui.motion_timeline.playing);
    let zoom = state.editor_ui.motion_timeline.zoom;
    assert!(press(&mut state, center(layout.zoom_in), VW, VH, 0, false));
    assert_eq!(state.editor_ui.motion_timeline.zoom, zoom * ZOOM_STEP);

    // The manual tab steps through the manual ids, wrapping.
    let manual = center(layout.tabs[2].1);
    let mut seen = Vec::new();
    for _ in 0..3 {
        press(&mut state, manual, VW, VH, 0, false);
        seen.push(state.editor_ui.motion_timeline.manual_id.clone().unwrap());
    }
    assert_eq!(seen, ["pulse", "shake", "pulse"]);
}

#[test]
fn a_strip_drag_follows_the_pointer_and_writes_once_on_release() {
    let mut state = state();
    let (_, layout) = layout_of(&state);
    let b = &layout.rows[1];
    let grab = Point2D::new(
        (b.keyframes[0].x + b.keyframes[1].x) / 2.0,
        b.keyframes[0].y,
    );
    assert!(press(&mut state, grab, VW, VH, 0, false));
    // 25 px at the default 0.25 px/ms is 100 ms later.
    let zoom = state.editor_ui.motion_timeline.zoom;
    let moved = Point2D::new(grab.x + 100.0 * zoom, grab.y + 40.0);
    assert!(drag(&mut state, moved, VW, VH, 0));
    assert_eq!(delay(&state, "b"), 100, "a drag only drafts");
    assert!(release(&mut state));
    assert_eq!(delay(&state, "b"), 200);
    assert!(!release(&mut state), "nothing left to release");
    assert!(state.undo());
    assert_eq!(delay(&state, "b"), 100);
}

#[test]
fn keyframes_select_toggle_and_delete_together() {
    let mut state = state();
    let (_, layout) = layout_of(&state);
    let a_mid = layout.rows[0].keyframes[1];
    let b_mid = layout.rows[1].keyframes[1];
    press(&mut state, a_mid, VW, VH, 0, false);
    release(&mut state);
    press(&mut state, b_mid, VW, VH, 0, true);
    release(&mut state);
    assert_eq!(state.editor_ui.motion_timeline.selection.len(), 2);

    // Pressing a selected keyframe keeps both and drags them together.
    press(&mut state, a_mid, VW, VH, 0, false);
    assert!(matches!(
        &state.editor_ui.motion_timeline.drag,
        Some(MotionTimelineDrag::Keyframes { keys, .. }) if keys.len() == 2
    ));
    release(&mut state);

    assert!(delete_keyframes(&mut state));
    assert!(!delete_keyframes(&mut state), "the selection is gone");
    let keyframes = |node: &str| {
        op_editor_core::walkers::find_node(
            state.active_children(),
            &op_editor_core::NodeId::new(node),
        )
        .and_then(|node| node.motion_declarations().1)
        .map(|animations| animations[0].keyframes.len())
    };
    assert_eq!((keyframes("a"), keyframes("b")), (Some(2), Some(2)));

    // A press on the empty lanes clears a selection.
    press(&mut state, layout.rows[0].keyframes[0], VW, VH, 0, false);
    release(&mut state);
    press(&mut state, center(layout.lanes), VW, VH, 0, false);
    assert!(state.editor_ui.motion_timeline.selection.is_empty());
}

#[test]
fn the_ruler_scrubs_and_the_end_grip_stretches() {
    let mut state = state();
    let (_, layout) = layout_of(&state);
    let at_200 = Point2D::new(layout.x_for_ms(200.0), center(layout.ruler).y);
    press(&mut state, at_200, VW, VH, 0, false);
    assert_eq!(state.editor_ui.motion_timeline.playhead_ms, 200.0);
    release(&mut state);

    let grip = center(layout.rows[1].end_handle);
    assert!(over_strip_end(&state, grip, VW, VH));
    press(&mut state, grip, VW, VH, 0, false);
    let zoom = state.editor_ui.motion_timeline.zoom;
    drag(
        &mut state,
        Point2D::new(grip.x + 400.0 * zoom, grip.y),
        VW,
        VH,
        0,
    );
    release(&mut state);
    let b = op_editor_core::walkers::find_node(
        state.active_children(),
        &op_editor_core::NodeId::new("b"),
    )
    .and_then(|node| node.motion_declarations().1)
    .map(|animations| animations[0].duration_ms);
    assert_eq!(b, Some(800));
}

#[test]
fn a_press_outside_the_panel_hands_delete_back_to_the_canvas() {
    let mut state = state();
    let (_, layout) = layout_of(&state);
    press(&mut state, layout.rows[0].keyframes[1], VW, VH, 0, false);
    release(&mut state);
    assert!(has_keyframe_selection(&state));
    assert!(!press(
        &mut state,
        Point2D::new(700.0, 300.0),
        VW,
        VH,
        0,
        false
    ));
    assert!(!has_keyframe_selection(&state));
}
