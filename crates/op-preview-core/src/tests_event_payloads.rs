//! R4 residual semantic-event payload coverage.

#![cfg(test)]

use super::input_event::{PreviewInput, PreviewInputEnvelope};
use super::{test_measure, PreviewSession};
use jian_core::geometry::point;
use jian_core::gesture::pointer::{Modifiers, PointerKind, PointerPhase};
use jian_core::gesture::PointerEvent;

fn enter() -> PreviewSession {
    let source = r##"{
        "version":"1.1","formatVersion":"1.1","id":"event-payloads",
        "app":{"name":"event-payloads","version":"1","id":"event-payloads"},
        "state":{
            "keyCode":{"type":"string","default":""},
            "keyRepeat":{"type":"bool","default":false},
            "changes":{"type":"int","default":0},
            "changedValue":{"type":"number","default":0},
            "submits":{"type":"int","default":0}
        },
        "children":[
            {"type":"frame","id":"screen","width":400,"height":240,"children":[
                {"type":"text_input","id":"field","x":20,"y":20,"width":200,"height":40,
                 "events":{
                    "onKey":[{"set":{
                        "$state.keyCode":"$event.code",
                        "$state.keyRepeat":"$event.repeat"
                    }}],
                    "onSubmit":[{"set":{"$state.submits":"$state.submits + 1"}}]
                 }},
                {"type":"slider","id":"slider","x":20,"y":100,"width":200,"height":32,
                 "min":0,"max":10,"step":1,"value":0,
                 "events":{"onChange":[{"set":{
                    "$state.changes":"$state.changes + 1",
                    "$state.changedValue":"$event.value"
                 }}]}}
            ]}
        ]
    }"##;
    let document = jian_ops_schema::load_str(source)
        .expect("parse event payload fixture")
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
    .expect("enter event payload fixture")
}

fn touch(id: u32, phase: PointerPhase, x: f32, y: f32, t_ms: u64) -> PreviewInputEnvelope {
    let mut event = PointerEvent::simple_at(id, phase, point(x, y), t_ms);
    event.kind = PointerKind::Touch;
    PreviewInputEnvelope::new(PreviewInput::Pointer(event))
}

fn tap(session: &mut PreviewSession, id: u32, x: f32, y: f32, t_ms: u64) {
    let _ = session.dispatch_input(touch(id, PointerPhase::Down, x, y, t_ms));
    let _ = session.dispatch_input(touch(id, PointerPhase::Up, x, y, t_ms + 10));
}

fn app_value(session: &PreviewSession, key: &str) -> serde_json::Value {
    session
        .app_state_value_for_test(key)
        .map(|value| value.0)
        .unwrap_or(serde_json::Value::Null)
}

#[test]
fn key_code_and_repeat_reach_the_authored_handler_payload() {
    let mut session = enter();
    tap(&mut session, 1, 120.0, 40.0, 0);
    let outcome = session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Key {
        key: "F2".to_owned(),
        code: "PhysicalF2".to_owned(),
        repeat: true,
        modifiers: Modifiers::CTRL,
    }));
    assert_eq!(outcome.semantic_handlers, vec!["onKey"]);
    assert_eq!(
        app_value(&session, "keyCode"),
        serde_json::json!("PhysicalF2")
    );
    assert_eq!(app_value(&session, "keyRepeat"), serde_json::json!(true));
}

#[test]
fn slider_change_fires_once_only_when_the_value_changes() {
    let mut session = enter();
    tap(&mut session, 2, 120.0, 116.0, 0);
    assert_eq!(app_value(&session, "changes"), serde_json::json!(1));
    assert_eq!(app_value(&session, "changedValue"), serde_json::json!(5.0));

    tap(&mut session, 3, 120.0, 116.0, 100);
    assert_eq!(
        app_value(&session, "changes"),
        serde_json::json!(1),
        "an unchanged slider value must not emit another Change"
    );
}

#[test]
fn enter_on_a_text_input_dispatches_submit() {
    let mut session = enter();
    tap(&mut session, 4, 120.0, 40.0, 0);
    let outcome = session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Key {
        key: "Enter".to_owned(),
        code: "Enter".to_owned(),
        repeat: false,
        modifiers: Modifiers::empty(),
    }));
    assert!(outcome.semantic_handlers.contains(&"onSubmit"));
    assert_eq!(app_value(&session, "submits"), serde_json::json!(1));
}
