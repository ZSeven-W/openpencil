//! Interaction paths through `PreviewSession`: role promotion, focus/caret
//! seeding, hover enter, a slider drag, and wheel routing.
//!
//! Kept separate from `tests.rs` so both files remain below the repository's
//! 800-line limit (same split convention as `tests_caret.rs`).

#![cfg(test)]

use super::{test_measure, PreviewSession};
use jian_core::gesture::pointer::PointerPhase;
use jian_core::widget_state::WidgetState;
use op_editor_ui::layout_scene::{LayoutScene, SceneNode};

/// no transient axis selection, so resolution falls back to the
/// document's default theme (first value per axis).
fn default_theme() -> std::collections::BTreeMap<String, String> {
    std::collections::BTreeMap::new()
}

/// Find a node by id on the scene's active page.
fn find<'a>(scene: &'a LayoutScene, id: &str) -> Option<&'a SceneNode> {
    scene.active_page().and_then(|p| p.find(id))
}

/// A minimal `.op` document with a single focusable text_input node.
fn text_input_doc() -> jian_ops_schema::PenDocument {
    let src = r##"{
        "version": "1.1",
        "formatVersion": "1.1",
        "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "children": [
            { "type": "text_input", "id": "field", "width": 200, "height": 40,
              "fill": [{ "type": "solid", "color": "#ffffff" }] }
        ]
    }"##;
    let loaded = jian_ops_schema::load_str(src).expect("parse test doc");
    loaded.value
}

#[test]
fn legacy_role_promotion_is_recorded_as_warning() {
    // A legacy role-frame should be promoted (promote=true) and the
    // promotion surfaced as a warning for the editor diagnostics.
    let src = r##"{
        "version": "1.1",
        "formatVersion": "1.1",
        "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "children": [
            { "type": "frame", "id": "legacy", "width": 200, "height": 40,
              "meta": { "role": "input" } }
        ]
    }"##;
    let doc = jian_ops_schema::load_str(src)
        .expect("parse legacy doc")
        .value;

    let session = PreviewSession::enter(
        &doc,
        (800.0, 600.0),
        &default_theme(),
        0,
        false,
        false,
        test_measure(),
    )
    .expect("enter preview on legacy doc");
    for w in session.warnings() {
        assert!(!w.is_empty());
    }
}

#[test]
fn focus_seeds_widget_state_for_caret() {
    // CONCERN fix: Tab-focusing a text input (without typing) must seed
    // its runtime state so the caret can paint immediately —
    // `Runtime::focus_next` alone only moves the focus pointer.
    let doc = text_input_doc();
    let mut session = PreviewSession::enter(
        &doc,
        (800.0, 600.0),
        &default_theme(),
        0,
        false,
        false,
        test_measure(),
    )
    .expect("enter preview");
    session.set_now_ms(0);
    session.focus_next();
    assert!(
        session.runtime().widget_states.get("field").is_some(),
        "focusing a text input should seed its widget state for caret paint"
    );
}

#[test]
fn preview_promotes_role_input_frame_to_interactive_field() {
    // The screenshot bug end-to-end: a generated `role=input` mockup
    // frame (box + leading mail icon + muted placeholder) must become a
    // live `text_input` in PREVIEW — rendered as a widget (icon kept) AND
    // clickable + typeable. Before the rework the preview scene showed
    // the un-promoted frame, so typing was invisible.
    let doc = jian_ops_schema::load_str(
        r##"{"version":"1.1","formatVersion":"1.1","id":"x",
        "app":{"name":"x","version":"1","id":"x"},
        "children":[{"type":"frame","id":"screen","width":300,"height":300,"children":[
          {"type":"frame","id":"email","role":"input","x":20,"y":20,"width":260,"height":48,
           "cornerRadius":12,"fill":[{"type":"solid","color":"#f3f4f6"}],"children":[
             {"type":"icon_font","id":"i","iconFontName":"mail","width":20,"height":20},
             {"type":"text","id":"t","content":"you@example.com",
              "fill":[{"type":"solid","color":"#9ca3af"}]}
           ]}]}]}"##,
    )
    .expect("parse role=input doc")
    .value;
    let mut session = PreviewSession::enter(
        &doc,
        (800.0, 600.0),
        &default_theme(),
        0,
        false,
        false,
        test_measure(),
    )
    .expect("enter preview");
    session.set_now_ms(0);

    // The promoted field renders as a text_input widget in preview's scene.
    let scene = session.preview_scene_for_test();
    let field = find(&scene, "email").expect("promoted field node");
    let w = field
        .widget
        .as_ref()
        .expect("role=input frame promoted to a widget in the preview scene");
    assert_eq!(w.kind, "text_input");
    assert_eq!(w.leading_icon.as_deref(), Some("mail"));
    assert_eq!(w.placeholder.as_deref(), Some("you@example.com"));

    // Tap its centre → focus → type → overlaid scene shows the value.
    let (x, y, wd, h) = session
        .node_rect("email")
        .expect("runtime rect for promoted field");
    assert!(
        session.dispatch_tap(x + wd / 2.0, y + h / 2.0),
        "tap should hit the promoted input"
    );
    session.dispatch_text("a@b");
    let after = session.preview_scene_for_test();
    assert_eq!(
        find(&after, "email")
            .unwrap()
            .widget
            .as_ref()
            .unwrap()
            .value_str
            .as_deref(),
        Some("a@b"),
        "typed text must surface in the overlaid preview scene"
    );
}

/// A frame with an onHoverEnter action writing `$app.hovered`.
fn hover_doc() -> jian_ops_schema::PenDocument {
    let src = r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "state": { "hovered": { "type": "bool", "default": false } },
        "children": [
            { "type": "frame", "id": "card", "width": 200, "height": 100,
              "fill": [{ "type": "solid", "color": "#ffffff" }],
              "events": { "onHoverEnter": [ { "set": { "$app.hovered": "true" } } ] } }
        ]
    }"##;
    jian_ops_schema::load_str(src)
        .expect("parse hover doc")
        .value
}

#[test]
fn hover_move_fires_on_hover_enter() {
    let doc = hover_doc();
    let mut session = PreviewSession::enter(
        &doc,
        (800.0, 600.0),
        &default_theme(),
        0,
        false,
        false,
        test_measure(),
    )
    .expect("enter");
    session.set_now_ms(0);
    session.dispatch_pointer_phase(20.0, 20.0, PointerPhase::Hover);
    let v = session
        .runtime()
        .state
        .app_get("hovered")
        .expect("hovered seeded from doc state");
    assert_eq!(
        v.0,
        serde_json::json!(true),
        "onHoverEnter must fire on Hover move"
    );
}

/// A slider so a Down→Move→Up drag can be asserted.
fn slider_doc() -> jian_ops_schema::PenDocument {
    let src = r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "children": [
            { "type": "slider", "id": "vol", "width": 200, "height": 24,
              "min": 0, "max": 100, "value": 50 }
        ]
    }"##;
    jian_ops_schema::load_str(src)
        .expect("parse slider doc")
        .value
}

#[test]
fn slider_drag_moves_value() {
    let doc = slider_doc();
    let mut session = PreviewSession::enter(
        &doc,
        (800.0, 600.0),
        &default_theme(),
        0,
        false,
        false,
        test_measure(),
    )
    .expect("enter");
    session.set_now_ms(0);
    let (x, y, w, h) = session.node_rect("vol").expect("slider rect");
    let cy = y + h / 2.0;
    session.dispatch_pointer_phase(x + w * 0.5, cy, PointerPhase::Down);
    session.dispatch_pointer_phase(x + w * 0.9, cy, PointerPhase::Move);
    session.dispatch_pointer_phase(x + w * 0.9, cy, PointerPhase::Up);
    match session.runtime().widget_states.get("vol") {
        Some(WidgetState::Slider { value, .. }) => {
            assert!(
                *value > 60.0,
                "drag to 90% must move the value past 60, got {value}"
            );
        }
        other => panic!("expected Slider state, got {other:?}"),
    }
}

#[test]
fn wheel_routes_only_to_on_scroll_handler() {
    // With an onScroll handler → consumed; without → not consumed
    // (the host then falls back to canvas pan/zoom).
    let src = r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "state": { "scrolled": { "type": "bool", "default": false } },
        "children": [
            { "type": "frame", "id": "list", "width": 200, "height": 100,
              "events": { "onScroll": [ { "set": { "$app.scrolled": "true" } } ] } }
        ]
    }"##;
    let doc = jian_ops_schema::load_str(src).expect("parse").value;
    let mut session = PreviewSession::enter(
        &doc,
        (800.0, 600.0),
        &default_theme(),
        0,
        false,
        false,
        test_measure(),
    )
    .expect("enter");
    assert!(
        session.dispatch_wheel(20.0, 20.0, 0.0, -12.0),
        "onScroll node must consume"
    );

    let plain = text_input_doc();
    let mut plain_session = PreviewSession::enter(
        &plain,
        (800.0, 600.0),
        &default_theme(),
        0,
        false,
        false,
        test_measure(),
    )
    .expect("enter");
    assert!(
        !plain_session.dispatch_wheel(20.0, 20.0, 0.0, -12.0),
        "no handler → not consumed → host may pan/zoom"
    );
}
