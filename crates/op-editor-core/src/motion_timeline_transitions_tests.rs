use super::*;
use crate::EditorCommand;
use serde_json::json;

fn fade(delay: u64) -> serde_json::Value {
    json!({
        "trigger": "mount", "durationMs": 400, "delayMs": delay, "easing": "linear",
        "keyframes": [
            {"offset": 0, "values": {"opacity": 0}},
            {"offset": 1, "values": {"opacity": 1}}
        ]
    })
}

fn state() -> EditorState {
    let doc = jian_ops_schema::load_str(
        &json!({
            "version": "1.0",
            "children": [
                {"type": "frame", "id": "still", "width": 390, "height": 844, "children": [
                    {"type": "rectangle", "id": "plain", "width": 10, "height": 10}
                ]},
                {"type": "frame", "id": "home", "width": 390, "height": 844, "children": [
                    {"type": "rectangle", "id": "a", "width": 10, "height": 10,
                     "animations": [fade(0)]},
                    {"type": "rectangle", "id": "b", "width": 10, "height": 10,
                     "animations": [fade(100)]}
                ]}
            ]
        })
        .to_string(),
    )
    .expect("document")
    .value;
    EditorState::from_document(doc)
}

#[test]
fn the_scope_follows_the_selection_then_the_first_animating_screen() {
    let mut state = state();
    // Nothing selected: the first top-level node that animates.
    assert_eq!(
        state.motion_timeline_scope().map(PenNodeExt::id_str),
        Some("home")
    );
    state.apply(EditorCommand::SetSelection {
        node_id: NodeId::new("plain"),
    });
    assert_eq!(
        state.motion_timeline_scope().map(PenNodeExt::id_str),
        Some("still")
    );
    // An explicit scope wins; a stale one falls back.
    state.editor_ui.motion_timeline.scope_root = Some("home".into());
    assert_eq!(
        state.motion_timeline_scope().map(PenNodeExt::id_str),
        Some("home")
    );
    state.editor_ui.motion_timeline.scope_root = Some("gone".into());
    assert_eq!(
        state.motion_timeline_scope().map(PenNodeExt::id_str),
        Some("still")
    );
}

#[test]
fn playback_follows_the_clock_and_stops_or_wraps_at_the_end() {
    let mut state = state();
    state.toggle_motion_timeline();
    assert_eq!(state.motion_timeline_projection().clip_ms, 500.0);
    state.toggle_motion_playback(1_000);
    assert_eq!(state.motion_timeline_deadline(1_000), Some(1_016));
    assert!(state.tick_motion_timeline(1_200));
    assert_eq!(state.editor_ui.motion_timeline.playhead_ms, 200.0);
    // Past the end without looping: stop on the last frame.
    assert!(state.tick_motion_timeline(2_000));
    assert_eq!(state.editor_ui.motion_timeline.playhead_ms, 500.0);
    assert!(!state.editor_ui.motion_timeline.playing);
    assert_eq!(state.motion_timeline_deadline(2_000), None);
    // Playing again from the end restarts; looping wraps.
    state.toggle_motion_loop();
    state.toggle_motion_playback(3_000);
    assert_eq!(state.editor_ui.motion_timeline.playhead_ms, 0.0);
    state.tick_motion_timeline(3_700);
    assert_eq!(state.editor_ui.motion_timeline.playhead_ms, 200.0);
    assert!(state.editor_ui.motion_timeline.playing);
}

#[test]
fn scrubbing_clamps_and_closing_stops() {
    let mut state = state();
    state.toggle_motion_timeline();
    state.scrub_motion_playhead(9_999.0, 0);
    assert_eq!(state.editor_ui.motion_timeline.playhead_ms, 500.0);
    state.scrub_motion_playhead(-5.0, 0);
    assert_eq!(state.editor_ui.motion_timeline.playhead_ms, 0.0);
    state.toggle_motion_playback(100);
    state.scrub_motion_playhead(300.0, 150);
    state.tick_motion_timeline(250);
    assert_eq!(state.editor_ui.motion_timeline.playhead_ms, 400.0);
    state.toggle_motion_timeline();
    assert!(!state.editor_ui.motion_timeline.playing);
    // The manual tab without an id projects nothing and cannot play.
    state.toggle_motion_timeline();
    state.set_motion_timeline_tab(MotionTimelineTab::Manual, None);
    state.toggle_motion_playback(0);
    assert!(!state.editor_ui.motion_timeline.playing);
}
