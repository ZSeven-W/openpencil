use super::*;
use serde_json::json;

fn three_stops(delay: u64) -> serde_json::Value {
    json!({
        "trigger": "mount", "durationMs": 400, "delayMs": delay, "easing": "linear",
        "keyframes": [
            {"offset": 0, "values": {"opacity": 0}},
            {"offset": 0.5, "values": {"opacity": 0.6}},
            {"offset": 1, "values": {"opacity": 1}}
        ]
    })
}

fn state() -> EditorState {
    let doc = jian_ops_schema::load_str(
        &json!({
            "version": "1.0",
            "children": [{"type": "frame", "id": "home", "width": 390, "height": 844, "children": [
                {"type": "rectangle", "id": "a", "width": 10, "height": 10,
                 "animations": [three_stops(0), {
                    "trigger": "mount", "durationMs": 200, "easing": "linear",
                    "keyframes": [
                        {"offset": 0, "values": {"opacity": 0}},
                        {"offset": 1, "values": {"opacity": 1}}
                    ]
                 }]},
                {"type": "rectangle", "id": "b", "width": 10, "height": 10,
                 "animations": [three_stops(100)]}
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

fn key(node: &str, animation: usize, keyframe: usize) -> TimelineKeyframeRef {
    TimelineKeyframeRef {
        node_id: node.into(),
        animation,
        keyframe,
    }
}

fn animation(state: &EditorState, node: &str, index: usize) -> Option<NodeAnimation> {
    state.motion_animation(node, index)
}

#[test]
fn snapping_prefers_targets_in_reach_then_the_grid() {
    assert_eq!(snap_time(203.0, &[200.0, 260.0], 8.0), 200.0);
    assert_eq!(snap_time(254.0, &[200.0, 260.0], 8.0), 260.0);
    assert_eq!(snap_time(233.0, &[200.0, 260.0], 8.0), 230.0);
    let state = state();
    // a's middle keyframe and a's second animation's end both sit at 200 ms;
    // dragging the first leaves only the second as a target there.
    let at_200 = |targets: Vec<f64>| targets.iter().filter(|t| **t == 200.0).count();
    assert_eq!(at_200(state.motion_snap_targets(&[])), 2);
    assert_eq!(at_200(state.motion_snap_targets(&[key("a", 0, 1)])), 1);
    assert!(
        state.motion_snap_targets(&[]).contains(&0.0),
        "the playhead is a target"
    );
}

#[test]
fn a_released_strip_drag_writes_once_and_an_abandoned_one_writes_nothing() {
    let mut state = state();
    state.begin_motion_timeline_drag(MotionTimelineDrag::Strip {
        node_id: "b".into(),
        animation: 0,
        delta_ms: 0.0,
    });
    state.update_motion_timeline_drag(-40.0, 0);
    assert_eq!(
        animation(&state, "b", 0).unwrap().delay_ms,
        100,
        "drafts only"
    );
    assert!(state.finish_motion_timeline_drag());
    assert_eq!(animation(&state, "b", 0).unwrap().delay_ms, 60);
    assert!(state.editor_ui.motion_timeline.drag.is_none());

    state.begin_motion_timeline_drag(MotionTimelineDrag::StripEnd {
        node_id: "b".into(),
        animation: 0,
        end_ms: 60.0,
    });
    state.update_motion_timeline_drag(860.0, 0);
    state.cancel_motion_timeline_drag();
    assert!(!state.finish_motion_timeline_drag());
    assert_eq!(animation(&state, "b", 0).unwrap().duration_ms, 400);

    state.begin_motion_timeline_drag(MotionTimelineDrag::StripEnd {
        node_id: "b".into(),
        animation: 0,
        end_ms: 60.0,
    });
    state.update_motion_timeline_drag(860.0, 0);
    assert!(state.finish_motion_timeline_drag());
    assert_eq!(animation(&state, "b", 0).unwrap().duration_ms, 800);
    assert!(state.undo());
    assert_eq!(animation(&state, "b", 0).unwrap().duration_ms, 400);
}

#[test]
fn keyframes_of_several_animations_move_together_in_one_step() {
    let mut state = state();
    state.begin_motion_timeline_drag(MotionTimelineDrag::Keyframes {
        keys: vec![key("a", 0, 1), key("b", 0, 1)],
        delta_ms: 0.0,
    });
    state.update_motion_timeline_drag(40.0, 0);
    assert!(state.finish_motion_timeline_drag());
    assert!((animation(&state, "a", 0).unwrap().keyframes[1].offset - 0.6).abs() < 1e-6);
    assert!((animation(&state, "b", 0).unwrap().keyframes[1].offset - 0.6).abs() < 1e-6);
    assert!(state.undo());
    assert_eq!(animation(&state, "a", 0).unwrap().keyframes[1].offset, 0.5);
    assert_eq!(animation(&state, "b", 0).unwrap().keyframes[1].offset, 0.5);
}

#[test]
fn deleting_keyframes_removes_animations_that_drop_below_two() {
    let mut state = state();
    state.select_motion_keyframe(key("a", 0, 1), false);
    state.select_motion_keyframe(key("a", 1, 0), true);
    state.select_motion_keyframe(key("b", 0, 2), true);
    state.select_motion_keyframe(key("b", 0, 2), true);
    assert_eq!(state.editor_ui.motion_timeline.selection.len(), 2);
    assert!(state.delete_selected_motion_keyframes());
    // a: the middle keyframe went, and its two-keyframe second animation
    // went with its first keyframe.
    assert_eq!(animation(&state, "a", 0).unwrap().keyframes.len(), 2);
    assert!(animation(&state, "a", 1).is_none());
    assert_eq!(animation(&state, "b", 0).unwrap().keyframes.len(), 3);
    assert!(state.editor_ui.motion_timeline.selection.is_empty());
    assert!(state.undo());
    assert!(animation(&state, "a", 1).is_some());
}

#[test]
fn a_playhead_drag_scrubs_without_writing() {
    let mut state = state();
    state.begin_motion_timeline_drag(MotionTimelineDrag::Playhead);
    state.update_motion_timeline_drag(250.0, 0);
    assert_eq!(state.editor_ui.motion_timeline.playhead_ms, 250.0);
    assert!(!state.finish_motion_timeline_drag());
}

#[test]
fn a_followed_drag_moves_by_the_pointer_travel_and_snaps() {
    let mut state = state();
    // b runs 100..500 ms; grab it 150 ms in, at 250 ms.
    let anchor = |held_ms| TimelineDragAnchor {
        pointer_ms: 250.0,
        held_ms,
    };
    state.begin_motion_timeline_drag_at(
        MotionTimelineDrag::Strip {
            node_id: "b".into(),
            animation: 0,
            delta_ms: 0.0,
        },
        anchor(100.0),
    );
    // 33 ms of travel lands the start on the grid at 130 ms...
    state.drag_motion_timeline_to(283.0, Some(8.0), 0);
    assert_eq!(drag_delta(&state), Some(30.0));
    // ...and 96 ms puts it 4 ms from a's 200 ms keyframe, which wins.
    state.drag_motion_timeline_to(346.0, Some(8.0), 0);
    assert_eq!(drag_delta(&state), Some(100.0));
    assert!(state.finish_motion_timeline_drag());
    assert_eq!(animation(&state, "b", 0).unwrap().delay_ms, 200);
    assert!(state.editor_ui.motion_timeline.drag_anchor.is_none());

    // An end drag reports the snapped end itself.
    state.begin_motion_timeline_drag_at(
        MotionTimelineDrag::StripEnd {
            node_id: "b".into(),
            animation: 0,
            end_ms: 600.0,
        },
        anchor(600.0),
    );
    state.drag_motion_timeline_to(452.0, Some(8.0), 0);
    assert_eq!(
        state.editor_ui.motion_timeline.drag,
        Some(MotionTimelineDrag::StripEnd {
            node_id: "b".into(),
            animation: 0,
            end_ms: 800.0,
        })
    );
}

fn drag_delta(state: &EditorState) -> Option<f64> {
    match state.editor_ui.motion_timeline.drag {
        Some(MotionTimelineDrag::Strip { delta_ms, .. }) => Some(delta_ms),
        _ => None,
    }
}

#[test]
fn without_snapping_the_held_time_follows_the_pointer_exactly() {
    let mut state = state();
    state.begin_motion_timeline_drag_at(
        MotionTimelineDrag::Strip {
            node_id: "b".into(),
            animation: 0,
            delta_ms: 0.0,
        },
        TimelineDragAnchor {
            pointer_ms: 250.0,
            held_ms: 100.0,
        },
    );
    // 96 ms of travel would snap to a's keyframe; free, it stays at 96.
    state.drag_motion_timeline_to(346.0, None, 0);
    assert_eq!(drag_delta(&state), Some(96.0));
}

#[test]
fn a_height_drag_follows_the_pointer_and_cancels_back() {
    let mut state = state();
    state.editor_ui.motion_timeline.height = 220.0;
    state.begin_motion_timeline_drag(MotionTimelineDrag::Height {
        pointer_y: 700.0,
        height: 220.0,
    });
    state.resize_motion_timeline_to(650.0, 1000.0);
    assert_eq!(state.editor_ui.motion_timeline.height, 270.0);
    // Capped by the share of the canvas area, then by the design range.
    state.resize_motion_timeline_to(600.0, 300.0);
    assert_eq!(state.editor_ui.motion_timeline.height, 300.0);
    state.resize_motion_timeline_to(100.0, 1000.0);
    assert_eq!(state.editor_ui.motion_timeline.height, 360.0);
    // Releasing writes nothing; cancelling restores the start height.
    assert!(state.cancel_motion_timeline_drag());
    assert_eq!(state.editor_ui.motion_timeline.height, 220.0);
    assert!(!state.cancel_motion_timeline_drag());
}
