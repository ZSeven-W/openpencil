use super::*;
use crate::layout_scene::{NodeKind, SceneNode, ScenePage};
use crate::Rect;
use serde_json::json;

/// `card` fades 0 → 1 over 400 ms; `other` sits on another screen with
/// its own fade, which the timeline never touches.
fn state() -> EditorState {
    let fade = json!({
        "trigger": "mount", "durationMs": 400, "easing": "linear",
        "keyframes": [
            {"offset": 0, "values": {"opacity": 0}},
            {"offset": 1, "values": {"opacity": 1}}
        ]
    });
    let doc = jian_ops_schema::load_str(
        &json!({
            "version": "1.0",
            "children": [
                {"type": "frame", "id": "home", "width": 390, "height": 844, "children": [
                    {"type": "rectangle", "id": "card", "width": 10, "height": 10,
                     "animations": [fade.clone()]}
                ]},
                {"type": "frame", "id": "away", "width": 390, "height": 844, "children": [
                    {"type": "rectangle", "id": "other", "width": 10, "height": 10,
                     "animations": [fade]}
                ]}
            ]
        })
        .to_string(),
    )
    .expect("document")
    .value;
    EditorState::from_document(doc)
}

fn scene() -> LayoutScene {
    let leaf = |id: &str| {
        let mut node = SceneNode::leaf(id, NodeKind::Rect);
        node.bounds = Rect::xywh(0.0, 0.0, 10.0, 10.0);
        node.opacity = 1.0;
        node
    };
    let mut home = SceneNode::leaf("home", NodeKind::Frame);
    home.children = vec![leaf("card")];
    let mut away = SceneNode::leaf("away", NodeKind::Frame);
    away.children = vec![leaf("other")];
    LayoutScene {
        pages: vec![ScenePage {
            id: "p".into(),
            name: "Page".into(),
            children: vec![home, away],
        }],
        active_page_index: 0,
    }
}

fn opacity(scene: &LayoutScene, id: &str) -> f32 {
    scene
        .active_page()
        .and_then(|page| page.find(id))
        .expect("node")
        .opacity
}

#[test]
fn the_canvas_shows_the_scope_at_the_playhead_and_nothing_else() {
    let mut state = state();
    let base = scene();
    assert!(
        canvas_paint_scene(&state, &base, None, 0).is_none(),
        "a closed timeline paints the scene as it is"
    );
    state.toggle_motion_timeline();
    state.scrub_motion_playhead(100.0, 0);
    let painted = canvas_paint_scene(&state, &base, None, 0).expect("an overridden scene");
    assert!((opacity(&painted, "card") - 0.25).abs() < 1e-3);
    assert_eq!(opacity(&painted, "other"), 1.0, "other screens stay static");
    assert_eq!(opacity(&base, "card"), 1.0, "the layout scene is untouched");

    // At the clip's end the fade is back at its base value: nothing to apply.
    state.scrub_motion_playhead(400.0, 0);
    assert!(canvas_paint_scene(&state, &base, None, 0).is_none());
}

#[test]
fn only_a_playing_timeline_asks_for_frames() {
    let mut state = state();
    assert!(!motion_timeline_playing(&state));
    state.toggle_motion_timeline();
    state.toggle_motion_playback(0);
    assert!(motion_timeline_playing(&state));
    state.editor_ui.preview.mode = true;
    assert!(!motion_timeline_playing(&state), "Preview hides the panel");
}
