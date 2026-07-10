//! Task C1 tests: compiling non-`bind:value` bindings at preview `enter`.
//!
//! Split from `tests.rs` (which sits at the 800-line cap) rather than
//! appended there. Carries the shared `counter_doc()` fixture both
//! Task C1 (this file — compile-time site collection) and Task C2
//! (overlay re-evaluation, added on top of the same fixture) rely on.

#![cfg(test)]

use super::PreviewSession;
use op_editor_ui::layout_scene::{LayoutScene, SceneNode};

/// The default (empty) active-theme map — mirrors `tests.rs`'s helper of
/// the same name (duplicated here rather than making it `pub(super)` in
/// `tests.rs`, since these are sibling `#[cfg(test)]` modules, not nested
/// ones, and the helper is two lines).
fn default_theme() -> std::collections::BTreeMap<String, String> {
    std::collections::BTreeMap::new()
}

/// Find a node by id on the scene's active page — mirrors `tests.rs`'s
/// helper of the same name (duplicated for the same sibling-module reason
/// as `default_theme` above).
fn find<'a>(scene: &'a LayoutScene, id: &str) -> Option<&'a SceneNode> {
    scene.active_page().and_then(|p| p.find(id))
}

/// Counter fixture: doc-root `$app.count`, a text node whose `content`
/// binds the count, and a switch whose onTap increments it — the
/// canonical "events must be visible" Spec-2 acceptance shape.
fn counter_doc() -> jian_ops_schema::PenDocument {
    let src = r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "state": { "count": { "type": "int", "default": 2 } },
        "children": [
            { "type": "frame", "id": "root", "width": 400, "height": 200, "children": [
                { "type": "text", "id": "label", "content": "Count: -",
                  "width": 200, "height": 24,
                  "bindings": { "content": "\"Count: \" + $app.count" } },
                { "type": "switch", "id": "sw", "width": 44, "height": 24,
                  "events": { "onTap": [ { "set": { "$app.count": "$app.count + 1" } } ] } }
            ]}
        ]
    }"##;
    jian_ops_schema::load_str(src)
        .expect("parse counter doc")
        .value
}

#[test]
fn enter_compiles_binding_sites() {
    let doc = counter_doc();
    let session =
        PreviewSession::enter(&doc, (800.0, 600.0), &default_theme(), 0, false).expect("enter");
    assert_eq!(
        session.binding_sites_len_for_test(),
        1,
        "one content binding compiled"
    );
}

#[test]
fn invalid_binding_becomes_warning_not_error() {
    let src = r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "children": [
            { "type": "text", "id": "bad", "content": "x", "width": 100, "height": 20,
              "bindings": { "content": "1 +" } }
        ]
    }"##;
    let doc = jian_ops_schema::load_str(src).expect("parse").value;
    let session = PreviewSession::enter(&doc, (800.0, 600.0), &default_theme(), 0, false)
        .expect("enter still ok");
    assert_eq!(session.binding_sites_len_for_test(), 0);
    assert!(
        session
            .warnings()
            .iter()
            .any(|w| w.contains("InvalidBinding")),
        "compile failure must surface as a warning, got {:?}",
        session.warnings()
    );
}

// --- Task C2: overlay re-evaluates bindings each paint -------------

#[test]
fn binding_content_resolves_on_enter() {
    // Even before any interaction, a bound text node must show the
    // expression's value over the doc-root default state.
    let doc = counter_doc();
    let session =
        PreviewSession::enter(&doc, (800.0, 600.0), &default_theme(), 0, false).expect("enter");
    let scene = session.preview_scene_for_test();
    let label = find(&scene, "label").expect("label in scene");
    assert_eq!(label.text.as_deref(), Some("Count: 2"));
}

#[test]
fn tap_event_updates_bound_text() {
    // The Spec-2 acceptance loop: tap switch → onTap set $app.count →
    // bound label repaints with the new value.
    let doc = counter_doc();
    let mut session =
        PreviewSession::enter(&doc, (800.0, 600.0), &default_theme(), 0, false).expect("enter");
    session.set_now_ms(0);
    let (x, y, w, h) = session.node_rect("sw").expect("switch rect");
    session.dispatch_tap(x + w / 2.0, y + h / 2.0);
    let scene = session.preview_scene_for_test();
    assert_eq!(
        find(&scene, "label").expect("label").text.as_deref(),
        Some("Count: 3"),
        "fired event must be visible in the overlaid scene"
    );
}

#[test]
fn bindings_do_not_mutate_document() {
    let doc = counter_doc();
    let before = serde_json::to_string(&doc).expect("before");
    {
        let mut session =
            PreviewSession::enter(&doc, (800.0, 600.0), &default_theme(), 0, false).expect("enter");
        session.set_now_ms(0);
        let (x, y, w, h) = session.node_rect("sw").expect("switch rect");
        session.dispatch_tap(x + w / 2.0, y + h / 2.0);
    }
    let after = serde_json::to_string(&doc).expect("after");
    assert_eq!(
        before, after,
        "binding overlay must never touch the saved doc"
    );
}
