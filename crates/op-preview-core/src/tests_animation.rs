//! R7 structured Preview animation timeline.

#![cfg(test)]

use super::input_event::{PreviewInput, PreviewInputEnvelope};
use super::{test_measure, InvalidationKind, PreviewSession};
use jian_core::action::services::{
    AnimationDirection, AnimationFillMode, AnimationOutcome, AnimationProperty, AnimationRequest,
    AnimationSink, Easing,
};

fn document() -> jian_ops_schema::PenDocument {
    let source = r##"{
        "version":"1.1","formatVersion":"1.1","id":"animation",
        "app":{"name":"animation","version":"1","id":"animation"},
        "children":[
            {"type":"rectangle","id":"box","x":0,"y":0,"width":100,"height":100,
             "opacity":0,
             "fill":[{"type":"solid","color":"#000000"}],
             "events":{"onTap":[
                 {"animate":{
                     "target":"box","property":"opacity",
                     "from":0,"to":1,"durationMs":100,"delayMs":0,
                     "easing":"linear","iterations":1,
                     "direction":"normal","fillMode":"forwards"
                 }}
             ]}}
        ]
    }"##;
    jian_ops_schema::load_str(source).expect("parse").value
}

fn opacity(session: &PreviewSession) -> f32 {
    session
        .preview_scene_for_test()
        .active_page()
        .and_then(|page| page.find("box"))
        .expect("box")
        .opacity
}

fn tap(session: &mut PreviewSession) {
    let mut down = jian_core::gesture::PointerEvent::simple_at(
        1,
        jian_core::gesture::pointer::PointerPhase::Down,
        jian_core::geometry::point(50.0, 50.0),
        0,
    );
    down.kind = jian_core::gesture::pointer::PointerKind::Touch;
    let _ = session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Pointer(down)));
    let mut up = jian_core::gesture::PointerEvent::simple_at(
        1,
        jian_core::gesture::pointer::PointerPhase::Up,
        jian_core::geometry::point(50.0, 50.0),
        0,
    );
    up.kind = jian_core::gesture::pointer::PointerKind::Touch;
    let _ = session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Pointer(up)));
}

fn opacity_session(actions: Vec<serde_json::Value>, authored: f64) -> PreviewSession {
    let source = serde_json::json!({
        "version":"1.1","formatVersion":"1.1","id":"timeline",
        "app":{"name":"timeline","version":"1","id":"timeline"},
        "children":[{
            "type":"rectangle","id":"box","x":0,"y":0,"width":100,"height":100,
            "opacity":authored,
            "fill":[{"type":"solid","color":"#ff0000"}],
            "events":{"onTap":actions}
        }]
    });
    let document = jian_ops_schema::load_str(&source.to_string())
        .expect("parse timeline")
        .value;
    PreviewSession::enter(
        &document,
        (800.0, 600.0),
        &std::collections::BTreeMap::new(),
        0,
        false,
        false,
        test_measure(),
    )
    .expect("enter")
}

fn opacity_action(
    from: Option<f64>,
    to: f64,
    duration_ms: u64,
    delay_ms: u64,
    iterations: u32,
    direction: &str,
    fill_mode: &str,
) -> serde_json::Value {
    let mut body = serde_json::json!({
        "target":"box","property":"opacity","to":to,
        "durationMs":duration_ms,"delayMs":delay_ms,
        "easing":"linear","iterations":iterations,
        "direction":direction,"fillMode":fill_mode
    });
    if let Some(from) = from {
        body["from"] = serde_json::json!(from);
    }
    serde_json::json!({ "animate": body })
}

#[test]
fn opacity_animation_reaches_exact_midpoint_and_terminal_fill() {
    let document = document();
    let mut session = PreviewSession::enter(
        &document,
        (800.0, 600.0),
        &std::collections::BTreeMap::new(),
        0,
        false,
        false,
        test_measure(),
    )
    .expect("enter");
    tap(&mut session);
    let _ = session.pump(50);
    assert!((opacity(&session) - 0.5).abs() < 0.001);
    let _ = session.pump(100);
    assert!((opacity(&session) - 1.0).abs() < 0.001);
}

#[test]
fn host_clock_push_ticks_the_same_session_timeline() {
    let mut session = opacity_session(
        vec![opacity_action(
            Some(0.0),
            1.0,
            100,
            0,
            1,
            "normal",
            "forwards",
        )],
        0.0,
    );
    tap(&mut session);
    session.set_now_ms(50);
    assert!((opacity(&session) - 0.5).abs() < 0.001);
}

#[test]
fn delay_boundaries_and_all_fill_modes_are_exact() {
    for (fill_mode, during_delay, after_end) in [
        ("none", 0.25, 0.25),
        ("forwards", 0.25, 1.0),
        ("backwards", 0.0, 0.25),
        ("both", 0.0, 1.0),
    ] {
        let mut session = opacity_session(
            vec![opacity_action(
                Some(0.0),
                1.0,
                100,
                20,
                1,
                "normal",
                fill_mode,
            )],
            0.25,
        );
        tap(&mut session);
        let _ = session.pump(19);
        assert!(
            (opacity(&session) - during_delay).abs() < 0.001,
            "{fill_mode} before delay boundary"
        );
        let _ = session.pump(20);
        assert!((opacity(&session) - 0.0).abs() < 0.001);
        let _ = session.pump(70);
        assert!((opacity(&session) - 0.5).abs() < 0.001);
        let _ = session.pump(120);
        assert!(
            (opacity(&session) - after_end).abs() < 0.001,
            "{fill_mode} after terminal boundary"
        );
    }
}

#[test]
fn reverse_alternate_and_alternate_reverse_iterations_are_exact() {
    let mut reverse = opacity_session(
        vec![opacity_action(
            Some(0.0),
            1.0,
            100,
            0,
            1,
            "reverse",
            "forwards",
        )],
        0.0,
    );
    tap(&mut reverse);
    let _ = reverse.pump(0);
    assert!((opacity(&reverse) - 1.0).abs() < 0.001);
    let _ = reverse.pump(50);
    assert!((opacity(&reverse) - 0.5).abs() < 0.001);
    let _ = reverse.pump(100);
    assert!((opacity(&reverse) - 0.0).abs() < 0.001);

    let mut alternate = opacity_session(
        vec![opacity_action(
            Some(0.0),
            1.0,
            100,
            0,
            2,
            "alternate",
            "forwards",
        )],
        0.0,
    );
    tap(&mut alternate);
    for (time, expected) in [(0, 0.0), (50, 0.5), (100, 1.0), (150, 0.5), (200, 0.0)] {
        let _ = alternate.pump(time);
        assert!((opacity(&alternate) - expected).abs() < 0.001);
    }

    let mut alternate_reverse = opacity_session(
        vec![opacity_action(
            Some(0.0),
            1.0,
            100,
            0,
            2,
            "alternate_reverse",
            "forwards",
        )],
        0.0,
    );
    tap(&mut alternate_reverse);
    for (time, expected) in [(0, 1.0), (50, 0.5), (100, 0.0), (150, 0.5), (200, 1.0)] {
        let _ = alternate_reverse.pump(time);
        assert!((opacity(&alternate_reverse) - expected).abs() < 0.001);
    }
}

#[test]
fn omitted_from_samples_current_value_and_replacement_is_bounded() {
    let mut sampled = opacity_session(
        vec![opacity_action(None, 1.0, 100, 0, 1, "normal", "forwards")],
        0.25,
    );
    tap(&mut sampled);
    let _ = sampled.pump(0);
    assert!((opacity(&sampled) - 0.25).abs() < 0.001);
    let _ = sampled.pump(50);
    assert!((opacity(&sampled) - 0.625).abs() < 0.001);

    let mut replaced = opacity_session(
        vec![
            opacity_action(Some(0.0), 1.0, 100, 0, 1, "normal", "forwards"),
            opacity_action(Some(0.0), 0.5, 200, 0, 1, "normal", "forwards"),
        ],
        0.0,
    );
    tap(&mut replaced);
    assert_eq!(replaced.active_animation_track_count(), 1);
    let _ = replaced.pump(100);
    assert!((opacity(&replaced) - 0.25).abs() < 0.001);
    assert!(replaced.cancel_animation("box", "opacity"));
    assert_eq!(replaced.active_animation_track_count(), 0);
    assert_eq!(opacity(&replaced), 0.0);
}

#[test]
fn one_deadline_serves_many_tracks_and_layout_tracks_are_discrete() {
    let actions = vec![
        opacity_action(Some(0.0), 1.0, 100, 0, 1, "normal", "forwards"),
        serde_json::json!({ "animate": {
            "target":"box","property":"x","from":0,"to":100,
            "durationMs":100,"easing":"linear"
        }}),
    ];
    let mut session = opacity_session(actions, 0.0);
    tap(&mut session);
    assert_eq!(session.active_animation_track_count(), 2);
    assert_eq!(session.next_wake_deadline_ms(), Some(16));

    let width = vec![serde_json::json!({ "animate": {
        "target":"box","property":"width","from":100,"to":200,
        "durationMs":100,"easing":"linear","fillMode":"forwards"
    }})];
    let mut discrete = opacity_session(width, 0.0);
    tap(&mut discrete);
    assert_eq!(
        discrete.next_wake_deadline_ms(),
        Some(100),
        "discrete Relayout has no 60Hz wake"
    );
    let _ = discrete.pump(50);
    assert_eq!(
        discrete.runtime_rect("box").map(|rect| rect.size.x),
        Some(100.0)
    );
    assert_ne!(
        discrete.last_animation_invalidation(),
        InvalidationKind::Relayout,
        "continuous midpoint cannot relayout"
    );
    let _ = discrete.pump(100);
    assert_eq!(
        discrete.runtime_rect("box").map(|rect| rect.size.x),
        Some(200.0)
    );
    assert_eq!(
        discrete.last_animation_invalidation(),
        InvalidationKind::Relayout
    );
}

#[test]
fn every_builtin_property_applies_through_r6_invalidation() {
    let properties = [
        ("opacity", serde_json::json!(0), serde_json::json!(1)),
        ("x", serde_json::json!(0), serde_json::json!(100)),
        ("y", serde_json::json!(0), serde_json::json!(80)),
        ("rotation", serde_json::json!(0), serde_json::json!(90)),
        ("scaleX", serde_json::json!(1), serde_json::json!(2)),
        ("scaleY", serde_json::json!(1), serde_json::json!(2)),
        (
            "fill",
            serde_json::json!("#ff0000"),
            serde_json::json!("#0000ff"),
        ),
        (
            "stroke",
            serde_json::json!("#000000"),
            serde_json::json!("#ffffff"),
        ),
        ("cornerRadius", serde_json::json!(0), serde_json::json!(10)),
        ("width", serde_json::json!(100), serde_json::json!(200)),
        ("height", serde_json::json!(100), serde_json::json!(160)),
    ];
    let actions = properties
        .iter()
        .map(|(property, from, to)| {
            serde_json::json!({ "animate": {
                "target":"box","property":property,"from":from,"to":to,
                "durationMs":100,"easing":"linear","fillMode":"forwards"
            }})
        })
        .collect();
    let mut session = opacity_session(actions, 0.0);
    tap(&mut session);
    let _ = session.pump(50);
    let midpoint = session.preview_scene_for_test();
    let box_node = midpoint
        .active_page()
        .and_then(|page| page.find("box"))
        .expect("box");
    assert!((box_node.opacity - 0.5).abs() < 0.001);
    assert!((box_node.rotation - std::f32::consts::FRAC_PI_4).abs() < 0.001);
    assert_eq!(box_node.corner_radius, 5.0);
    assert_eq!(
        box_node.bounds.origin,
        op_editor_ui::Point2D::new(25.0, 15.0)
    );
    assert_eq!(
        box_node.bounds.size,
        op_editor_ui::Point2D::new(150.0, 150.0)
    );
    let fill = box_node.fill.expect("animated fill");
    assert!((fill.r - 0.5).abs() < 0.01 && (fill.b - 0.5).abs() < 0.01);
    let stroke = box_node.stroke.expect("animated stroke");
    assert!(
        (stroke.color.r - 0.5).abs() < 0.01
            && (stroke.color.g - 0.5).abs() < 0.01
            && (stroke.color.b - 0.5).abs() < 0.01
    );
    assert_eq!(
        session.last_animation_invalidation(),
        InvalidationKind::HitTest,
        "discrete width/height do not contaminate continuous frames"
    );

    let _ = session.pump(100);
    let terminal = session.preview_scene_for_test();
    let box_node = terminal
        .active_page()
        .and_then(|page| page.find("box"))
        .expect("box");
    assert_eq!(box_node.bounds.origin, op_editor_ui::Point2D::new(0.0, 0.0));
    assert_eq!(
        box_node.bounds.size,
        op_editor_ui::Point2D::new(400.0, 320.0)
    );
    assert_eq!(
        session.last_animation_invalidation(),
        InvalidationKind::Relayout
    );
}

#[test]
fn timeline_rejects_the_257th_unique_track() {
    let session = opacity_session(Vec::new(), 0.0);
    for index in 0..256 {
        let outcome = session.animation.request(&AnimationRequest {
            target: format!("node-{index}"),
            property: AnimationProperty::Opacity,
            from: Some(serde_json::json!(0)),
            to: serde_json::json!(1),
            duration_ms: 100,
            delay_ms: 0,
            easing: Easing::Linear,
            iterations: 1,
            direction: AnimationDirection::Normal,
            fill_mode: AnimationFillMode::None,
            requested_at_ms: 0,
        });
        assert_eq!(outcome, AnimationOutcome::Accepted);
    }
    let overflow = session.animation.request(&AnimationRequest {
        target: "overflow".to_owned(),
        property: AnimationProperty::Opacity,
        from: Some(serde_json::json!(0)),
        to: serde_json::json!(1),
        duration_ms: 100,
        delay_ms: 0,
        easing: Easing::Linear,
        iterations: 1,
        direction: AnimationDirection::Normal,
        fill_mode: AnimationFillMode::None,
        requested_at_ms: 0,
    });
    assert_eq!(
        overflow,
        AnimationOutcome::Rejected("animation timeline is full".to_owned())
    );
}
