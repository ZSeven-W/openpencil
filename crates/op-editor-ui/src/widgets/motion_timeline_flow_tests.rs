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
        (b.keyframes[0].1.x + b.keyframes[1].1.x) / 2.0,
        b.keyframes[0].1.y,
    );
    assert!(press(&mut state, grab, VW, VH, 0, false));
    // 25 px at the default 0.25 px/ms is 100 ms later.
    let zoom = state.editor_ui.motion_timeline.zoom;
    let moved = Point2D::new(grab.x + 100.0 * zoom, grab.y + 40.0);
    assert!(drag(&mut state, moved, VW, VH, 0, true));
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
    let a_mid = layout.rows[0].keyframes[1].1;
    let b_mid = layout.rows[1].keyframes[1].1;
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
    press(&mut state, layout.rows[0].keyframes[0].1, VW, VH, 0, false);
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
    assert_eq!(
        hover_cursor(&state, grip, VW, VH),
        Some(CursorHint::ResizeEw)
    );
    press(&mut state, grip, VW, VH, 0, false);
    let zoom = state.editor_ui.motion_timeline.zoom;
    drag(
        &mut state,
        Point2D::new(grip.x + 400.0 * zoom, grip.y),
        VW,
        VH,
        0,
        true,
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
    press(&mut state, layout.rows[0].keyframes[1].1, VW, VH, 0, false);
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

#[test]
fn the_top_edge_resizes_the_panel_and_holds_its_cursor() {
    let mut state = state();
    let (_, layout) = layout_of(&state);
    let edge = Point2D::new(center(layout.rect).x, layout.rect.origin.y);
    assert_eq!(
        hover_cursor(&state, edge, VW, VH),
        Some(CursorHint::ResizeNs)
    );
    assert!(press(&mut state, edge, VW, VH, 0, false));
    assert!(!drag_writes(&state), "a resize writes nothing");
    assert!(drag(
        &mut state,
        Point2D::new(edge.x, edge.y - 60.0),
        VW,
        VH,
        0,
        true
    ));
    assert_eq!(
        state.editor_ui.motion_timeline.height,
        layout.rect.size.y + 60.0
    );
    // The cursor stays while the pointer wanders off the edge.
    assert_eq!(drag_cursor(&state), Some(CursorHint::ResizeNs));
    assert!(release(&mut state));
    let (_, resized) = layout_of(&state);
    assert_eq!(resized.rect.size.y, layout.rect.size.y + 60.0);
}

#[test]
fn the_scope_button_opens_a_screen_menu_that_picks_and_closes() {
    let mut state = state();
    let (_, layout) = layout_of(&state);
    assert!(press(&mut state, center(layout.scope), VW, VH, 0, false));
    assert!(state.editor_ui.motion_timeline.scope_menu_open);
    let (view, layout) = layout_of(&state);
    assert_eq!(view.scope_menu.as_ref().map(Vec::len), Some(1));
    let (frame, rows) = layout.scope_menu.clone().expect("menu laid out");
    // It rises above the panel, over the canvas.
    assert!(frame.origin.y + frame.size.y <= layout.rect.origin.y);
    assert!(press(&mut state, center(rows[0].1), VW, VH, 0, false));
    let timeline = &state.editor_ui.motion_timeline;
    assert!(!timeline.scope_menu_open);
    assert_eq!(timeline.scope_root.as_deref(), Some("home"));
    // A press anywhere else just closes it again.
    state.toggle_motion_timeline_scope_menu();
    assert!(!press(
        &mut state,
        Point2D::new(700.0, 300.0),
        VW,
        VH,
        0,
        false
    ));
    assert!(!state.editor_ui.motion_timeline.scope_menu_open);
}

#[test]
fn a_chevron_shows_property_sub_tracks_whose_diamonds_pick_keyframes() {
    let mut state = state();
    let (_, layout) = layout_of(&state);
    let chevron = layout.rows[0].chevron.expect("a animates opacity");
    assert!(press(&mut state, center(chevron), VW, VH, 0, false));
    let (view, layout) = layout_of(&state);
    let labels: Vec<&str> = view.rows.iter().map(|row| row.label.as_str()).collect();
    assert_eq!(labels, ["a", "opacity", "b"]);
    assert_eq!(view.rows[1].depth, view.rows[0].depth + 1);
    assert!(layout.rows[1].label_x > layout.rows[0].label_x);
    assert!(press(
        &mut state,
        layout.rows[1].keyframes[2].1,
        VW,
        VH,
        0,
        false
    ));
    assert_eq!(
        state.editor_ui.motion_timeline.selection,
        [TimelineKeyframeRef {
            node_id: "a".into(),
            animation: 0,
            keyframe: 2,
        }]
    );
    // The keyframe's node is selected, so the Inspector shows it.
    assert_eq!(state.selection.anchor.as_str(), "a");
}

#[test]
fn the_selected_node_gets_a_row_before_it_animates() {
    let doc = jian_ops_schema::load_str(
        &json!({
            "version": "1.0",
            "children": [{"type": "frame", "id": "home", "width": 390, "height": 844, "children": [
                {"type": "rectangle", "id": "a", "width": 10, "height": 10,
                 "animations": [animation("mount", None, 0)]},
                {"type": "frame", "id": "group", "width": 50, "height": 50, "children": [
                    {"type": "rectangle", "id": "still", "width": 10, "height": 10}
                ]}
            ]}]
        })
        .to_string(),
    )
    .expect("document")
    .value;
    let mut state = EditorState::from_document(doc);
    state.toggle_motion_timeline();
    state.select_motion_lane("still");
    let (view, _) = layout_of(&state);
    let rows: Vec<(&str, usize)> = view
        .rows
        .iter()
        .map(|row| (row.label.as_str(), row.depth))
        .collect();
    assert_eq!(rows, [("a", 1), ("still", 2)]);
    assert!(view.rows[1].strip().is_none());
}

#[test]
fn double_clicking_a_strip_focuses_the_inspector_on_it() {
    let mut state = state();
    state.editor_ui.property_tab = op_editor_core::PropertyTab::Code;
    let (_, layout) = layout_of(&state);
    let b = &layout.rows[1];
    let grab = Point2D::new(
        (b.keyframes[0].1.x + b.keyframes[1].1.x) / 2.0,
        b.keyframes[0].1.y,
    );
    assert!(press(&mut state, grab, VW, VH, 1000, false));
    release(&mut state);
    assert_eq!(state.editor_ui.motion_row_expanded("b"), None);
    assert!(press(&mut state, grab, VW, VH, 1200, false));
    assert_eq!(state.selection.anchor.as_str(), "b");
    assert_eq!(state.editor_ui.motion_row_expanded("b"), Some(0));
    assert_eq!(
        state.editor_ui.property_tab,
        op_editor_core::PropertyTab::Design
    );
    assert!(state.editor_ui.motion_timeline.drag.is_none());
}

#[test]
fn without_snapping_a_drag_lands_off_grid() {
    let mut state = state();
    let (_, layout) = layout_of(&state);
    let b = &layout.rows[1];
    let grab = Point2D::new(
        (b.keyframes[0].1.x + b.keyframes[1].1.x) / 2.0,
        b.keyframes[0].1.y,
    );
    press(&mut state, grab, VW, VH, 0, false);
    // 3.25 px at 0.25 px/ms is 13 ms: the grid would make it 10.
    drag(
        &mut state,
        Point2D::new(grab.x + 3.25, grab.y),
        VW,
        VH,
        0,
        false,
    );
    release(&mut state);
    assert_eq!(delay(&state, "b"), 113);
}

#[test]
fn presses_in_the_inspector_keep_the_keyframe_being_edited() {
    let mut state = state();
    let (_, layout) = layout_of(&state);
    press(&mut state, layout.rows[0].keyframes[1].1, VW, VH, 0, false);
    release(&mut state);
    let inspector = crate::widgets::host_canvas_geometry::property_panel_rect(&state, VW, VH);
    assert!(state.property_panel_visible());
    assert!(!press(&mut state, center(inspector), VW, VH, 0, false));
    assert!(has_keyframe_selection(&state));
}

#[test]
fn the_shortcut_toggles_the_panel_only_where_it_can_show() {
    let mut state = state();
    assert!(toggle(&mut state));
    assert!(!state.editor_ui.motion_timeline.open);
    state.editor_ui.preview.mode = true;
    assert!(!toggle(&mut state));
    assert!(!state.editor_ui.motion_timeline.open);
}
