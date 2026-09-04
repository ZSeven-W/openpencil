//! R9 pause/resume/reset and effect-continuation controls.

#![cfg(test)]

use super::input_event::{PreviewInput, PreviewInputEnvelope};
use super::{test_measure, PreviewHostCapabilities, PreviewSession};
use op_preview_contracts::{PreviewEffect, PreviewEffectResult, PreviewRunState, PreviewTraceKind};

fn theme() -> std::collections::BTreeMap<String, String> {
    std::collections::BTreeMap::new()
}

fn control_doc() -> jian_ops_schema::PenDocument {
    let source = r##"{
        "version":"1.1","formatVersion":"1.1","id":"controls",
        "app":{"name":"controls","version":"1","id":"controls",
               "capabilities":["clipboard"]},
        "state":{"after":{"type":"int","default":0}},
        "children":[
            {"type":"frame","id":"button","width":200,"height":200,
             "events":{"onTap":[
                 {"parallel":[
                     [{"delay":{"ms":100}},{"set":{"$state.after":"1"}}],
                     [{"animate":{
                         "target":"button","property":"opacity","from":1,"to":0,
                         "durationMs":100,"fillMode":"forwards"
                     }}],
                     [{"copy":{"text":"'pause-secret'"}}]
                 ]}
             ]}},
            {"type":"text_input","id":"field","x":220,"width":120,"height":32}
        ]
    }"##;
    jian_ops_schema::load_str(source)
        .expect("parse control doc")
        .value
}

fn confirm_doc() -> jian_ops_schema::PenDocument {
    let source = r##"{
        "version":"1.1","formatVersion":"1.1","id":"confirm",
        "app":{"name":"confirm","version":"1","id":"confirm",
               "capabilities":["notifications"]},
        "state":{
            "confirmed":{"type":"int","default":0},
            "cancelled":{"type":"int","default":0}
        },
        "children":[
            {"type":"frame","id":"button","width":200,"height":200,
             "events":{"onTap":[
                 {"confirm":{
                     "title":"'Question'","message":"'Continue?'",
                     "on_confirm":[{"set":{"$state.confirmed":"$state.confirmed + 1"}}],
                     "on_cancel":[{"set":{"$state.cancelled":"$state.cancelled + 1"}}]
                 }}
             ]}}
        ]
    }"##;
    jian_ops_schema::load_str(source)
        .expect("parse confirm doc")
        .value
}

fn enter(
    document: &jian_ops_schema::PenDocument,
    capabilities: PreviewHostCapabilities,
) -> PreviewSession {
    PreviewSession::enter_with_capabilities(
        document,
        (800.0, 600.0),
        &theme(),
        0,
        false,
        false,
        test_measure(),
        capabilities,
    )
    .expect("enter")
}

fn tap(session: &mut PreviewSession) {
    let mut down = jian_core::gesture::PointerEvent::simple_at(
        1,
        jian_core::gesture::pointer::PointerPhase::Down,
        jian_core::geometry::point(100.0, 100.0),
        0,
    );
    down.kind = jian_core::gesture::pointer::PointerKind::Touch;
    let _ = session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Pointer(down)));
    let mut up = jian_core::gesture::PointerEvent::simple_at(
        1,
        jian_core::gesture::pointer::PointerPhase::Up,
        jian_core::geometry::point(100.0, 100.0),
        0,
    );
    up.kind = jian_core::gesture::pointer::PointerKind::Touch;
    let _ = session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Pointer(up)));
}

#[test]
fn pause_freezes_every_queue_and_resume_runs_elapsed_work_once() {
    let document = control_doc();
    let mut session = enter(
        &document,
        PreviewHostCapabilities {
            clipboard: true,
            ..PreviewHostCapabilities::none()
        },
    );
    tap(&mut session);
    let before = session.debug_snapshot();
    assert!(before.queues.action_tasks > 0);
    assert_eq!(before.queues.effects, 1);
    assert_eq!(before.queues.animations, 1);

    session.pause();
    assert_eq!(session.debug_snapshot().run_state, PreviewRunState::Paused);
    assert_eq!(session.next_wake_deadline_ms(), None);
    let frozen = session.debug_snapshot();
    let _ = session.pump(1000);
    let _ = session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::FocusNext));
    assert!(session.drain_effects().is_empty());
    let still_frozen = session.debug_snapshot();
    assert_eq!(still_frozen.state, frozen.state);
    assert_eq!(still_frozen.queues, frozen.queues);

    session.resume();
    assert_eq!(session.debug_snapshot().run_state, PreviewRunState::Running);
    let deadline = session.next_wake_deadline_ms().expect("re-armed wake");
    assert!(deadline > 1000, "paused wall time is shifted, not replayed");
    let _ = session.pump(deadline);
    let _ = session.pump(1100);
    assert_eq!(
        session
            .app_state_value_for_test("after")
            .and_then(|value| value.as_i64()),
        Some(1)
    );
    let _ = session.pump(1200);
    assert_eq!(
        session
            .app_state_value_for_test("after")
            .and_then(|value| value.as_i64()),
        Some(1),
        "resume continuation is exactly once"
    );
    assert_eq!(session.drain_effects().len(), 1);
    assert!(session
        .trace_entries()
        .iter()
        .any(|entry| entry.kind == PreviewTraceKind::Control));
}

#[test]
fn queued_confirm_completion_resumes_the_exact_branch_after_pause() {
    for (result, confirmed, cancelled) in [
        (PreviewEffectResult::Success, 1, 0),
        (PreviewEffectResult::Cancelled, 0, 1),
    ] {
        let document = confirm_doc();
        let mut session = enter(
            &document,
            PreviewHostCapabilities {
                notifications: true,
                ..PreviewHostCapabilities::none()
            },
        );
        tap(&mut session);
        let effects = session.drain_effects();
        assert!(matches!(
            effects.as_slice(),
            [PreviewEffect::Confirm { .. }]
        ));
        let id = effects[0].id();
        session.pause();
        assert!(session.complete_effect(id, result.clone()));
        let _ = session.pump(1000);
        assert_eq!(
            session
                .app_state_value_for_test("confirmed")
                .and_then(|value| value.as_i64()),
            Some(0)
        );
        assert_eq!(
            session
                .app_state_value_for_test("cancelled")
                .and_then(|value| value.as_i64()),
            Some(0)
        );
        session.resume();
        let deadline = session.next_wake_deadline_ms().expect("completion wake");
        let _ = session.pump(deadline);
        assert_eq!(
            session
                .app_state_value_for_test("confirmed")
                .and_then(|value| value.as_i64()),
            Some(confirmed)
        );
        assert_eq!(
            session
                .app_state_value_for_test("cancelled")
                .and_then(|value| value.as_i64()),
            Some(cancelled)
        );
        let _ = session.pump(deadline + 1);
        assert!(
            !session.complete_effect(id, result),
            "completion is exactly once"
        );
    }
}

#[test]
fn reset_rebuilds_defaults_and_clears_all_runtime_ownership() {
    let document = control_doc();
    let before = serde_json::to_string(&document).unwrap();
    let mut session = enter(
        &document,
        PreviewHostCapabilities {
            clipboard: true,
            ..PreviewHostCapabilities::none()
        },
    );
    tap(&mut session);
    let mut down = jian_core::gesture::PointerEvent::simple_at(
        9,
        jian_core::gesture::pointer::PointerPhase::Down,
        jian_core::geometry::point(100.0, 100.0),
        10,
    );
    down.kind = jian_core::gesture::pointer::PointerKind::Touch;
    let _ = session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Pointer(down)));
    session.pause();
    session.reset().expect("reset");

    let snapshot = session.debug_snapshot();
    assert_eq!(snapshot.run_state, PreviewRunState::Running);
    assert_eq!(snapshot.queues.action_tasks, 0);
    assert_eq!(snapshot.queues.effects, 0);
    assert_eq!(snapshot.queues.animations, 0);
    assert!(snapshot.captured_pointers.is_empty());
    assert_eq!(snapshot.active_gestures, 0);
    assert!(snapshot.focused_node.is_none());
    assert_eq!(snapshot.current_screen.as_deref(), Some("/"));
    assert_eq!(
        session
            .app_state_value_for_test("after")
            .and_then(|value| value.as_i64()),
        Some(0)
    );
    assert_eq!(serde_json::to_string(&document).unwrap(), before);
}
