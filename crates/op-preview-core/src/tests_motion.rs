#![cfg(test)]

use super::{test_measure, PreviewSession};
use jian_core::gesture::pointer::{PointerKind, PointerPhase};
use op_editor_ui::layout_scene::SceneNode;

fn session_for(source: serde_json::Value) -> PreviewSession {
    let doc = jian_ops_schema::load_str(&source.to_string())
        .expect("motion fixture")
        .value;
    PreviewSession::enter(
        &doc,
        (400.0, 400.0),
        &std::collections::BTreeMap::new(),
        0,
        false,
        false,
        test_measure(),
    )
    .expect("preview session")
}

fn node(session: &PreviewSession, id: &str) -> SceneNode {
    session
        .preview_scene_for_test()
        .active_page()
        .and_then(|page| page.find(id))
        .cloned()
        .expect("scene node")
}

fn color(session: &PreviewSession, id: &str) -> op_editor_ui::Color {
    node(session, id).fill.expect("fill")
}

fn opacity(session: &PreviewSession, id: &str) -> f32 {
    node(session, id).opacity
}

#[test]
fn mount_animation_samples_keyframe_stops_across_pumped_frames() {
    let mut session = session_for(serde_json::json!({
        "version":"1.1","formatVersion":"1.1",
        "children":[{"type":"rectangle","id":"hero","width":100,"height":100,
            "opacity":1,"fill":[{"type":"solid","color":"#ffffff"}],
            "animations":[{"trigger":"mount","keyframes":[
                {"offset":0,"values":{"opacity":0,"translateY":24}},
                {"offset":0.5,"values":{"opacity":0.5,"translateY":12}},
                {"offset":1,"values":{"opacity":1,"translateY":0}}
            ],"durationMs":100,"easing":"linear"}]}]
    }));
    assert_eq!(session.active_animation_track_count(), 2);
    session.pump(0);
    assert!((opacity(&session, "hero") - 0.0).abs() < 0.001);
    session.pump(50);
    assert!((opacity(&session, "hero") - 0.5).abs() < 0.001);
    session.pump(100);
    assert!((opacity(&session, "hero") - 1.0).abs() < 0.001);
    assert_eq!(session.active_animation_track_count(), 0);
}

#[test]
fn in_view_animation_fires_once_after_page_scroll_reveals_node() {
    let mut session = session_for(serde_json::json!({
        "version":"1.1","formatVersion":"1.1",
        "children":[{"type":"frame","id":"screen","width":200,"height":100,
            "children":[{"type":"rectangle","id":"card","x":0,"y":180,"width":100,"height":40,
                "opacity":1,"fill":[{"type":"solid","color":"#ffffff"}],
                "animations":[{"trigger":"inView","keyframes":[
                    {"offset":0,"values":{"opacity":0}},
                    {"offset":1,"values":{"opacity":1}}
                ],"durationMs":100}]}]}]
    }));
    session.pump(0);
    assert_eq!(session.active_animation_track_count(), 0);
    assert!(session.set_page_scroll(140.0, 180.0, -140.0));
    assert!((opacity(&session, "card") - 0.0).abs() < 0.001);
    assert_eq!(session.active_animation_track_count(), 1);
    session.preview_scene_for_test();
    assert_eq!(session.active_animation_track_count(), 1);
}

#[test]
fn hover_transition_tweens_fill_and_retargets_from_the_sampled_value() {
    let mut session = session_for(serde_json::json!({
        "version":"1.1","formatVersion":"1.1",
        "children":[{"type":"switch","id":"button","width":100,"height":40,
            "fill":[{"type":"solid","color":"#ff0000"}],
            "states":{"hover":{"fill":[{"type":"solid","color":"#0000ff"}]}},
            "transition":{"durationMs":200,"easing":"linear","properties":["fill"]}}]
    }));
    session.dispatch_pointer_for_id_at(1, PointerKind::Mouse, 20.0, 20.0, PointerPhase::Hover, 0);
    let _ = color(&session, "button");
    assert_eq!(session.active_animation_track_count(), 1);
    session.pump(100);
    let mid = color(&session, "button");
    assert!(
        mid.r > 0.45 && mid.b > 0.45,
        "mid-transition color: {mid:?}"
    );
    session.dispatch_pointer_for_id_at(
        1,
        PointerKind::Mouse,
        200.0,
        200.0,
        PointerPhase::Hover,
        100,
    );
    let _ = color(&session, "button");
    session.pump(150);
    let retargeted = color(&session, "button");
    assert!(
        retargeted.r > mid.r,
        "retarget must start from the sampled fill"
    );
    assert!(
        retargeted.b < mid.b,
        "retarget must move toward the authored base"
    );
}

#[test]
fn reduced_motion_is_instant_and_forwards_mount_paints_final_without_a_track() {
    let mut session = session_for(serde_json::json!({
        "version":"1.1","formatVersion":"1.1",
        "children":[{"type":"rectangle","id":"hero","width":100,"height":100,
            "opacity":1,"fill":[{"type":"solid","color":"#ffffff"}],
            "animations":[{"trigger":"mount","keyframes":[
                {"offset":0,"values":{"opacity":0}},
                {"offset":1,"values":{"opacity":0.7}}
            ],"durationMs":400,"fillMode":"forwards"}],
            "transition":{"durationMs":200,"properties":["opacity"]}}]
    }));
    session.set_motion_preference(jian_ops_schema::MotionPreference::Reduced);
    assert_eq!(session.active_animation_track_count(), 0);
    assert!((opacity(&session, "hero") - 0.7).abs() < 0.001);
    assert_eq!(
        session.effective_motion_preference(),
        jian_ops_schema::MotionPreference::Reduced
    );
}

#[test]
fn non_compositor_motion_properties_are_reported_during_session_load() {
    let session = session_for(serde_json::json!({
        "version":"1.1",
        "children":[{"type":"rectangle","id":"hero","width":100,"height":100,
            "animations":[{"trigger":"mount","keyframes":[
                {"offset":0,"values":{"x":0}},
                {"offset":1,"values":{"x":20}}
            ],"durationMs":200}]}]
    }));
    assert!(session
        .warnings()
        .iter()
        .any(|warning| warning.contains("x") && warning.contains("ignored")));
}

#[test]
fn document_reduced_motion_wins_over_a_full_host_preference() {
    let mut session = session_for(serde_json::json!({
        "version":"1.1","motion":"reduced",
        "children":[{"type":"rectangle","id":"hero","width":100,"height":100,
            "opacity":1,"fill":[{"type":"solid","color":"#ffffff"}],
            "animations":[{"trigger":"mount","keyframes":[
                {"offset":0,"values":{"opacity":0}},
                {"offset":1,"values":{"opacity":1}}
            ],"durationMs":400,"fillMode":"forwards"}]}]
    }));
    session.set_motion_preference(jian_ops_schema::MotionPreference::Full);
    assert_eq!(
        session.effective_motion_preference(),
        jian_ops_schema::MotionPreference::Reduced
    );
    assert_eq!(session.active_animation_track_count(), 0);
}

#[test]
fn reduced_host_motion_applies_hover_state_without_a_transition_track() {
    let mut session = session_for(serde_json::json!({
        "version":"1.1",
        "children":[{"type":"switch","id":"button","width":100,"height":40,
            "fill":[{"type":"solid","color":"#ff0000"}],
            "states":{"hover":{"fill":[{"type":"solid","color":"#0000ff"}]}},
            "transition":{"durationMs":200,"properties":["fill"]}}]
    }));
    session.set_motion_preference(jian_ops_schema::MotionPreference::Reduced);
    session.dispatch_pointer_for_id_at(1, PointerKind::Mouse, 20.0, 20.0, PointerPhase::Hover, 0);
    assert_eq!(session.active_animation_track_count(), 0);
    let fill = color(&session, "button");
    assert!(fill.b > 0.9 && fill.r < 0.1, "reduced hover fill: {fill:?}");
}
