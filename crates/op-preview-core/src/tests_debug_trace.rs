//! R9 bounded trace, provenance, ordering, and redaction.

#![cfg(test)]

use super::input_event::{PreviewInput, PreviewInputEnvelope};
use super::{test_measure, PreviewHostCapabilities, PreviewSession};
use op_preview_contracts::{PreviewStateScope, PreviewTraceKind, UserActivationId};

fn theme() -> std::collections::BTreeMap<String, String> {
    std::collections::BTreeMap::new()
}

fn trace_doc() -> jian_ops_schema::PenDocument {
    let source = r##"{
        "version":"1.1","formatVersion":"1.1","id":"trace",
        "app":{"name":"trace","version":"1","id":"trace",
               "capabilities":["clipboard","network"]},
        "state":{"alias":{"type":"int","default":0}},
        "children":[
            {"type":"frame","id":"home","name":"Home","screen":"/",
             "width":200,"height":200,"opacity":1,
             "events":{"onTap":[
                 {"set":{
                     "$app.global":"1",
                     "$app.apiToken":"'credential-secret'",
                     "$state.alias":"1",
                     "$page.local":"1",
                     "$page.sessionToken":"'page-secret'",
                     "$self.own":"1",
                     "$self.password":"'self-secret'"
                 }},
                 {"push":"'\/detail'"},
                 {"animate":{
                     "target":"home","property":"opacity","from":1,"to":0,
                     "durationMs":100,"fillMode":"forwards"
                 }},
                 {"copy":{"text":"'clipboard-secret'"}},
                 {"share":{"text":"'private-share-secret'"}}
             ]}},
            {"type":"frame","id":"detail","name":"Detail","screen":"/detail",
             "width":200,"height":200}
        ]
    }"##;
    jian_ops_schema::load_str(source)
        .expect("parse trace doc")
        .value
}

fn enter() -> PreviewSession {
    let document = trace_doc();
    PreviewSession::enter_with_capabilities(
        &document,
        (800.0, 600.0),
        &theme(),
        0,
        false,
        false,
        test_measure(),
        PreviewHostCapabilities {
            clipboard: true,
            share: true,
            ..PreviewHostCapabilities::none()
        },
    )
    .expect("enter")
}

fn tap(session: &mut PreviewSession, activation: Option<UserActivationId>) {
    let mut down = jian_core::gesture::PointerEvent::simple_at(
        7,
        jian_core::gesture::pointer::PointerPhase::Down,
        jian_core::geometry::point(100.0, 100.0),
        0,
    );
    down.kind = jian_core::gesture::pointer::PointerKind::Touch;
    let _ = session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Pointer(down)));
    let mut up = jian_core::gesture::PointerEvent::simple_at(
        7,
        jian_core::gesture::pointer::PointerPhase::Up,
        jian_core::geometry::point(100.0, 100.0),
        10,
    );
    up.kind = jian_core::gesture::pointer::PointerKind::Touch;
    let _ = session.dispatch_input(PreviewInputEnvelope {
        input: PreviewInput::Pointer(up),
        activation,
    });
}

#[test]
fn trace_ring_keeps_the_newest_256_entries() {
    let mut session = enter();
    for _ in 0..257 {
        let _ = session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::FocusNext));
    }
    let entries = session.trace_entries();
    assert_eq!(entries.len(), 256);
    assert_eq!(
        entries.first().unwrap().sequence + 255,
        entries.last().unwrap().sequence
    );
    assert!(
        entries.first().unwrap().sequence > 1,
        "oldest entries are evicted, sequence never rewinds"
    );
}

#[test]
fn interaction_trace_order_provenance_and_redaction_are_exact() {
    let mut session = enter();
    tap(&mut session, Some(UserActivationId::from_raw(987_654_321)));
    let entries = session.trace_entries();
    let position = |kind| {
        entries
            .iter()
            .position(|entry| entry.kind == kind)
            .unwrap_or_else(|| panic!("missing trace kind {kind:?}"))
    };
    let ordered = [
        PreviewTraceKind::Input,
        PreviewTraceKind::SemanticEvent,
        PreviewTraceKind::Action,
        PreviewTraceKind::StateDiff,
        PreviewTraceKind::Route,
        PreviewTraceKind::Animation,
        PreviewTraceKind::Effect,
    ];
    let positions: Vec<_> = ordered.into_iter().map(position).collect();
    assert!(
        positions.windows(2).all(|window| window[0] < window[1]),
        "trace order is exact: {positions:?}"
    );

    let effects = session.drain_effects();
    assert_eq!(effects.len(), 2);
    for effect in effects {
        assert!(session.complete_effect(
            effect.id(),
            op_preview_contracts::PreviewEffectResult::Success
        ));
    }
    assert!(session
        .trace_entries()
        .iter()
        .any(|entry| entry.kind == PreviewTraceKind::EffectResult));

    let snapshot = session.debug_snapshot();
    for (scope, key) in [
        (PreviewStateScope::App, "global"),
        (PreviewStateScope::State, "alias"),
        (PreviewStateScope::Page, "local"),
        (PreviewStateScope::SelfNode, "own"),
    ] {
        let row = snapshot
            .state
            .iter()
            .find(|row| row.scope == scope && row.key == key)
            .unwrap_or_else(|| panic!("missing {scope:?}.{key}"));
        let provenance = row.provenance.as_ref().expect("last writer");
        assert_eq!(provenance.node_id.as_deref(), Some("home"));
        assert_eq!(provenance.event.as_deref(), Some("onTap"));
        assert_eq!(provenance.action.as_deref(), Some("set"));
        assert!(provenance.sequence > 0);
    }

    let encoded = serde_json::to_string(&session.trace_entries()).unwrap();
    for private in [
        "clipboard-secret",
        "private-share-secret",
        "credential-secret",
        "page-secret",
        "self-secret",
        "987654321",
    ] {
        assert!(
            !encoded.contains(private),
            "trace must redact private value {private}"
        );
    }
    assert!(snapshot
        .state
        .iter()
        .find(|row| row.key == "apiToken")
        .is_some_and(|row| row.value == serde_json::json!("<redacted>")));
    for key in ["sessionToken", "password"] {
        assert!(snapshot
            .state
            .iter()
            .find(|row| row.key == key)
            .is_some_and(|row| row.value == serde_json::json!("<redacted>")));
    }
}
