use super::*;
use crate::editor_ui_state::TimelineKeyframeRef;
use serde_json::json;

fn state() -> EditorState {
    let doc = jian_ops_schema::load_str(
        &json!({
            "version": "1.0",
            "children": [{"type": "frame", "id": "home", "width": 390, "height": 844, "children": [
                {"type": "rectangle", "id": "a", "width": 10, "height": 10, "animations": [
                    {"trigger": "manual", "id": "open", "durationMs": 400, "easing": "linear",
                     "keyframes": [
                        {"offset": 0, "values": {"opacity": 0}},
                        {"offset": 0.5, "values": {"translateY": 4}},
                        {"offset": 1, "values": {"opacity": 1}}
                     ]}
                ]},
                {"type": "rectangle", "id": "b", "width": 10, "height": 10, "animations": [
                    {"trigger": "manual", "id": "close", "durationMs": 200,
                     "keyframes": [{"offset": 0, "values": {}}, {"offset": 1, "values": {}}]}
                ]}
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

fn key(node: &str, keyframe: usize) -> TimelineKeyframeRef {
    TimelineKeyframeRef {
        node_id: node.into(),
        animation: 0,
        keyframe,
    }
}

#[test]
fn a_second_press_on_the_same_strip_soon_after_is_a_double_click() {
    let mut state = state();
    assert!(!state.press_motion_strip("a", 0, 1000));
    assert!(state.press_motion_strip("a", 0, 1300));
    // The pair is used up; another strip or a slow press starts over.
    assert!(!state.press_motion_strip("a", 0, 1400));
    assert!(!state.press_motion_strip("b", 0, 1500));
    assert!(!state.press_motion_strip("b", 0, 2000));
}

#[test]
fn focusing_an_animation_selects_its_node_and_opens_its_row() {
    let mut state = state();
    state.editor_ui.property_tab = PropertyTab::Code;
    state.editor_ui.motion_timeline.selection.push(key("a", 1));
    state.focus_motion_animation("a", 0);
    assert_eq!(state.selection.anchor.as_str(), "a");
    assert_eq!(state.editor_ui.property_tab, PropertyTab::Design);
    assert_eq!(state.editor_ui.motion_row_expanded("a"), Some(0));
    assert!(state.editor_ui.motion_timeline.selection.is_empty());
}

#[test]
fn the_section_edits_the_selected_nodes_keyframe_only() {
    let mut state = state();
    state.select_motion_lane("a");
    state.editor_ui.motion_timeline.selection.push(key("b", 0));
    assert!(state.motion_keyframe_focus().is_none());
    state.editor_ui.motion_timeline.selection.push(key("a", 1));
    assert_eq!(state.motion_keyframe_focus(), Some(&key("a", 1)));

    let mut summary = MotionSummary::default();
    state.fill_motion_summary(&mut summary);
    let keyframe = summary.keyframe.expect("keyframe mode");
    assert_eq!((keyframe.keyframe, keyframe.count), (1, 3));
    assert_eq!(keyframe.time_ms, 200.0);
    assert_eq!(keyframe.values, [("translateY".to_owned(), json!(4))]);
    assert!(keyframe.addable.contains(&"opacity"));
    // The screen's manual ids feed the id dropdown.
    assert_eq!(summary.manual_ids, ["close", "open"]);

    // A closed timeline leaves keyframe mode.
    state.editor_ui.motion_timeline.open = false;
    assert!(state.motion_keyframe_focus().is_none());
}
