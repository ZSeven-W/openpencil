//! R3 effect queue through the live preview session — the policy
//! allowlist, the fail-closed capability gate, URL-scheme validation,
//! ordered drains, and exactly-once completion, exercised end to end
//! through `dispatch_input`.

#![cfg(test)]

use super::input_event::{PreviewInput, PreviewInputEnvelope};
use super::{test_measure, PreviewSession};
use op_preview_contracts::{
    PreviewCapability, PreviewEffect, PreviewEffectResult, PreviewHostCapabilities,
};

/// A document whose only interaction surface is one frame with one tap
/// handler firing every effect-class action in a fixed order.
fn effects_doc() -> &'static str {
    r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "x",
        "app": { "name": "x", "version": "1", "id": "x",
                 "capabilities": ["clipboard", "network", "notifications", "haptic"] },
        "state": { "after": { "type": "int", "default": 0 } },
        "children": [
            { "type": "frame", "id": "btn", "x": 0, "y": 0, "width": 200, "height": 200,
              "events": {
                "onTap": [
                    { "open_url": { "url": "'https://openpencil.dev'" } },
                    { "copy": { "text": "'hello'" } },
                    { "haptic": { "style": "light" } },
                    { "toast": { "message": "'saved'" } },
                    { "notify": { "title": "'nope'" } },
                    { "set": { "$app.after": "$app.after + 1" } }
                ]
              } }
        ]
    }"##
}

fn default_theme() -> std::collections::BTreeMap<String, String> {
    std::collections::BTreeMap::new()
}

fn enter_with(caps: PreviewHostCapabilities) -> PreviewSession {
    let doc = jian_ops_schema::load_str(effects_doc())
        .expect("parse doc")
        .value;
    PreviewSession::enter_with_capabilities(
        &doc,
        (800.0, 600.0),
        &default_theme(),
        0,
        false,
        false,
        test_measure(),
        caps,
    )
    .expect("enter preview")
}

fn tap(session: &mut PreviewSession) -> usize {
    let mut ev = jian_core::gesture::PointerEvent::simple_at(
        1,
        jian_core::gesture::pointer::PointerPhase::Down,
        jian_core::geometry::point(100.0, 100.0),
        0,
    );
    ev.kind = jian_core::gesture::pointer::PointerKind::Touch;
    let _ = session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Pointer(ev)));
    let mut ev = jian_core::gesture::PointerEvent::simple_at(
        1,
        jian_core::gesture::pointer::PointerPhase::Up,
        jian_core::geometry::point(100.0, 100.0),
        10,
    );
    ev.kind = jian_core::gesture::pointer::PointerKind::Touch;
    session
        .dispatch_input(PreviewInputEnvelope::new(PreviewInput::Pointer(ev)))
        .effects_enqueued
}

/// One tap fires the ordered effect chain: open_url → copy → haptic →
/// toast at the queue; the policy-denied `notify` NEVER queues (it
/// surfaces as a diagnostic), and the safe trailing `set` still ran.
#[test]
fn effects_queue_in_order_and_policy_denies_are_diagnostics() {
    let caps = PreviewHostCapabilities {
        open_url: true,
        clipboard: true,
        haptics: true,
        notifications: true,
        ..PreviewHostCapabilities::none()
    };
    let mut s = enter_with(caps);
    assert_eq!(tap(&mut s), 4, "exactly the four approved effects enqueued");

    let drained = s.drain_effects();
    let kinds: Vec<&str> = drained.iter().map(|e| e.kind()).collect();
    assert_eq!(kinds, vec!["open_url", "copy", "haptic", "toast"]);
    // Ids are unique and monotonic.
    let ids: Vec<u64> = drained.iter().map(|e| e.id()).collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted, "ids ascend in FIFO order");
    // The declared capabilities ride with each effect.
    assert_eq!(drained[0].required_capability(), PreviewCapability::OpenUrl);
    assert_eq!(
        drained[1].required_capability(),
        PreviewCapability::Clipboard
    );
    // The denied `notify` left a structured diagnostic, not an effect.
    let diagnostics = s.take_action_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|d| d.contains("policy rejected action `notify`")),
        "the denied action is diagnosed: {diagnostics:?}"
    );
    assert_eq!(
        s.app_state_value_for_test("after").and_then(|v| v.as_i64()),
        Some(1),
        "the safe sibling after the denied action still ran"
    );
}

/// Fail-closed capabilities: an undeclared effect class is `Unsupported`
/// at the sink — nothing queues, nothing executes, later safe siblings
/// still run.
#[test]
fn undeclared_capabilities_never_enqueue() {
    let mut s = enter_with(PreviewHostCapabilities {
        open_url: true,
        ..PreviewHostCapabilities::none()
    });
    // Only open_url is declared: copy/haptic/toast are Unsupported.
    assert_eq!(tap(&mut s), 1, "only the declared effect class enqueued");
    assert!(matches!(
        s.drain_effects().first(),
        Some(PreviewEffect::OpenUrl { .. })
    ));
}

/// URL-scheme validation happens BEFORE enqueue: `tel`/`mailto` pass,
/// everything else is rejected with a structured diagnostic.
#[test]
fn open_url_schemes_are_validated_before_enqueue() {
    let caps = PreviewHostCapabilities {
        open_url: true,
        ..PreviewHostCapabilities::none()
    };
    let doc_src = r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "x",
        "app": { "name": "x", "version": "1", "id": "x",
                 "capabilities": ["network"] },
        "state": { "n": { "type": "int", "default": 0 } },
        "children": [
            { "type": "frame", "id": "btn", "x": 0, "y": 0, "width": 100, "height": 100,
              "events": {
                "onTap": [
                    { "open_url": { "url": "'tel:+15550100'" } },
                    { "open_url": { "url": "'ftp://nope'" } },
                    { "set": { "$app.n": "$app.n + 1" } }
                ]
              } }
        ]
    }"##;
    let doc = jian_ops_schema::load_str(doc_src).expect("parse doc").value;
    let mut s = PreviewSession::enter_with_capabilities(
        &doc,
        (800.0, 600.0),
        &default_theme(),
        0,
        false,
        false,
        test_measure(),
        caps,
    )
    .expect("enter preview");
    assert_eq!(tap(&mut s), 1, "only the tel: URL enqueued");
    let drained = s.drain_effects();
    assert!(
        matches!(drained.first(), Some(PreviewEffect::OpenUrl { url, .. }) if url == "tel:+15550100")
    );
    assert!(
        s.effect_diagnostics()
            .iter()
            .any(|d| d.contains("invalid url scheme")),
        "the bad scheme is diagnosed"
    );
    assert_eq!(
        s.app_state_value_for_test("n").and_then(|v| v.as_i64()),
        Some(1),
        "the chain continued past the rejected URL"
    );
}

/// Exactly-once completion: a double completion is refused (and
/// diagnosed) — a host bug can never resume authored continuations
/// twice.
#[test]
fn effect_completion_is_exactly_once() {
    let caps = PreviewHostCapabilities {
        clipboard: true,
        ..PreviewHostCapabilities::none()
    };
    let mut s = enter_with(caps);
    assert_eq!(tap(&mut s), 1);
    let drained = s.drain_effects();
    let id = drained[0].id();
    assert!(s.complete_effect(id, PreviewEffectResult::Success));
    assert!(!s.complete_effect(id, PreviewEffectResult::Success));
    assert!(
        s.effect_diagnostics()
            .iter()
            .any(|d| d.contains("completed more than once")),
        "the double completion is diagnosed"
    );
}
