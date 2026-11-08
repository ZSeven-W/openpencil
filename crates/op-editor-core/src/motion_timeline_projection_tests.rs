use super::*;
use serde_json::json;

fn screen(children: serde_json::Value) -> PenNode {
    serde_json::from_value(json!({
        "type": "frame", "id": "screen", "width": 390, "height": 844,
        "children": children
    }))
    .expect("screen")
}

fn node(id: &str, animations: serde_json::Value) -> serde_json::Value {
    json!({"type": "rectangle", "id": id, "width": 10, "height": 10, "animations": animations})
}

fn fade(trigger: &str, easing: &str, delay: u64, extra: serde_json::Value) -> serde_json::Value {
    let mut animation = json!({
        "trigger": trigger, "easing": easing, "durationMs": 400, "delayMs": delay,
        "keyframes": [
            {"offset": 0, "values": {"opacity": 0}},
            {"offset": 0.25, "values": {"opacity": 0.5}},
            {"offset": 1, "values": {"opacity": 1}}
        ]
    });
    if let serde_json::Value::Object(extra) = extra {
        animation.as_object_mut().expect("object").extend(extra);
    }
    animation
}

#[test]
fn linear_keyframes_sit_at_their_offsets() {
    let root = screen(json!([node(
        "a",
        json!([fade("mount", "linear", 100, json!({}))])
    )]));
    let projection = project(&root, &ClipTrigger::Mount);
    let strip = &projection.strips[0];
    assert_eq!((strip.node_id.as_str(), strip.animation), ("a", 0));
    assert_eq!((strip.start_ms, strip.iteration_ms), (100.0, 400.0));
    assert_eq!(strip.end_ms, Some(500.0));
    for (actual, expected) in strip.keyframe_ms.iter().zip([100.0, 200.0, 500.0]) {
        assert!((actual - expected).abs() < 0.01, "{:?}", strip.keyframe_ms);
    }
    assert_eq!(projection.clip_ms, 500.0);
}

#[test]
fn eased_keyframes_sit_where_preview_reaches_them() {
    // easeIn is p², so eased progress 0.25 is reached half way through.
    let root = screen(json!([node(
        "a",
        json!([fade("mount", "easeIn", 0, json!({}))])
    )]));
    let strip = &project(&root, &ClipTrigger::Mount).strips[0];
    assert!(
        (strip.keyframe_ms[1] - 200.0).abs() < 0.01,
        "{:?}",
        strip.keyframe_ms
    );
    assert_eq!(strip.keyframe_ms[2], 400.0);
}

#[test]
fn tabs_split_by_trigger_and_manual_tabs_gather_one_id() {
    let root = screen(json!([
        node(
            "a",
            json!([
                fade("mount", "linear", 0, json!({})),
                fade("inView", "linear", 0, json!({})),
                fade("manual", "linear", 0, json!({"id": "intro"}))
            ])
        ),
        node(
            "b",
            json!([
                fade("manual", "linear", 60, json!({"id": "intro"})),
                fade("manual", "linear", 0, json!({"id": "outro"}))
            ])
        )
    ]));
    assert_eq!(project(&root, &ClipTrigger::Mount).strips.len(), 1);
    assert_eq!(project(&root, &ClipTrigger::InView).strips[0].animation, 1);
    let intro = project(&root, &ClipTrigger::Manual("intro".into()));
    let placed: Vec<_> = intro
        .strips
        .iter()
        .map(|strip| (strip.node_id.as_str(), strip.animation, strip.start_ms))
        .collect();
    assert_eq!(placed, [("a", 2, 0.0), ("b", 0, 60.0)]);
    assert_eq!(intro.clip_ms, 460.0);
    assert_eq!(manual_ids(&root), ["intro", "outro"]);
}

#[test]
fn looping_animations_size_the_clip_by_two_iterations() {
    let root = screen(json!([
        node(
            "a",
            json!([fade(
                "mount",
                "linear",
                0,
                json!({"iterations": "infinite"})
            )])
        ),
        node(
            "b",
            json!([fade(
                "mount",
                "linear",
                100,
                json!({"iterations": "infinite"})
            )])
        )
    ]));
    let projection = project(&root, &ClipTrigger::Mount);
    assert!(projection.strips.iter().all(|strip| strip.end_ms.is_none()));
    assert_eq!(projection.clip_ms, 900.0);

    // One finite animation decides the clip on its own.
    let mixed = screen(json!([
        node(
            "a",
            json!([fade(
                "mount",
                "linear",
                0,
                json!({"iterations": "infinite"})
            )])
        ),
        node(
            "b",
            json!([fade("mount", "linear", 0, json!({"iterations": 2}))])
        )
    ]));
    assert_eq!(project(&mixed, &ClipTrigger::Mount).clip_ms, 800.0);
}

#[test]
fn an_empty_screen_projects_nothing() {
    let projection = project(&screen(json!([])), &ClipTrigger::Mount);
    assert!(projection.strips.is_empty());
    assert_eq!(projection.clip_ms, 0.0);
}
