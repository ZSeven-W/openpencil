use super::*;
use crate::editor_ui_state::MotionTimelineTab;
use serde_json::json;

fn state() -> EditorState {
    let fade = |trigger: &str, id: Option<&str>| {
        let mut value = json!({
            "trigger": trigger, "durationMs": 400, "easing": "linear",
            "keyframes": [
                {"offset": 0, "values": {"opacity": 0}},
                {"offset": 1, "values": {"opacity": 1}}
            ]
        });
        if let Some(id) = id {
            value["id"] = json!(id);
        }
        value
    };
    let doc = jian_ops_schema::load_str(
        &json!({
            "version": "1.0",
            "children": [
                {"type": "frame", "id": "home", "name": "Home", "width": 390, "height": 844,
                 "children": [{"type": "rectangle", "id": "a", "width": 10, "height": 10,
                               "animations": [fade("mount", None)]}]},
                {"type": "frame", "id": "menu", "width": 390, "height": 844,
                 "children": [{"type": "rectangle", "id": "b", "width": 10, "height": 10,
                               "animations": [fade("manual", Some("open"))]}]},
                {"type": "rectangle", "id": "loose", "width": 10, "height": 10},
                {"type": "ellipse", "id": "spinner", "width": 10, "height": 10,
                 "animations": [fade("mount", None)]}
            ]
        })
        .to_string(),
    )
    .expect("document")
    .value;
    let mut state = EditorState::from_document(doc);
    state.toggle_motion_timeline();
    state
}

#[test]
fn the_dropdown_lists_frames_and_animating_top_level_nodes() {
    let screens = state().motion_timeline_screens();
    let listed: Vec<_> = screens
        .iter()
        .map(|screen| (screen.id.as_str(), screen.label.as_str(), screen.animates))
        .collect();
    assert_eq!(
        listed,
        [
            ("home", "Home", true),
            ("menu", "menu", true),
            ("spinner", "spinner", true)
        ]
    );
}

#[test]
fn picking_a_screen_closes_the_menu_and_lands_on_a_tab_with_motion() {
    let mut state = state();
    state.toggle_motion_timeline_scope_menu();
    state.editor_ui.motion_timeline.playhead_ms = 120.0;
    state.pick_motion_timeline_scope("menu");
    let timeline = &state.editor_ui.motion_timeline;
    assert!(!timeline.scope_menu_open);
    assert_eq!(timeline.scope_root.as_deref(), Some("menu"));
    // Its only motion is manual, so the manual tab opens on that id.
    assert_eq!(timeline.tab, MotionTimelineTab::Manual);
    assert_eq!(timeline.manual_id.as_deref(), Some("open"));
    assert_eq!(timeline.playhead_ms, 0.0);
    assert_eq!(state.motion_timeline_projection().strips.len(), 1);
}
