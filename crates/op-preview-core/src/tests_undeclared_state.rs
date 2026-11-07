//! A design whose actions write `$app` keys it never declared (the
//! shape of the QA's AI-generated product page) must still repaint the
//! written value — never "set ok" in the trace with `null` on screen.

#![cfg(test)]

use super::{test_measure, PreviewSession};
use op_editor_ui::layout_scene::{LayoutScene, SceneNode};

fn find<'a>(scene: &'a LayoutScene, id: &str) -> Option<&'a SceneNode> {
    scene.active_page().and_then(|p| p.find(id))
}

fn stepper_doc(state: &str) -> jian_ops_schema::PenDocument {
    let src = format!(
        r##"{{
        "version": "1.1", "formatVersion": "1.1", "id": "x",
        "app": {{ "name": "x", "version": "1", "id": "x" }},
        {state}
        "children": [
            {{ "type": "frame", "id": "root", "width": 400, "height": 300, "children": [
                {{ "type": "frame", "id": "minus", "x": 0, "y": 0, "width": 40, "height": 40,
                  "events": {{ "onTap": [ {{ "set": {{ "$app.qty": "$app.qty > 1 ? $app.qty - 1 : 1" }} }} ] }} }},
                {{ "type": "frame", "id": "plus", "x": 100, "y": 0, "width": 40, "height": 40,
                  "events": {{ "onTap": [ {{ "set": {{ "$app.qty": "$app.qty + 1" }} }} ] }} }},
                {{ "type": "text", "id": "qty", "x": 0, "y": 60, "content": "-", "width": 100, "height": 24,
                  "bindings": {{ "content": "to_str($app.qty)" }} }},
                {{ "type": "text", "id": "total", "x": 0, "y": 100, "content": "-", "width": 100, "height": 24,
                  "bindings": {{ "content": "'¥' + to_str(24 * $app.qty)" }} }}
            ]}}
        ]
    }}"##
    );
    jian_ops_schema::load_str(&src).expect("parse").value
}

fn enter(doc: &jian_ops_schema::PenDocument) -> PreviewSession {
    let mut s = PreviewSession::enter(
        doc,
        (800.0, 600.0),
        &std::collections::BTreeMap::new(),
        0,
        false,
        false,
        test_measure(),
        0,
    )
    .expect("enter");
    s.set_now_ms(0);
    s
}

fn tap(session: &mut PreviewSession, id: &str) {
    let (x, y, w, h) = session.node_rect(id).expect("rect");
    session.dispatch_tap(x + w / 2.0, y + h / 2.0);
}

fn texts(session: &PreviewSession) -> (String, String) {
    let scene = session.preview_scene_for_test();
    (
        find(&scene, "qty")
            .unwrap()
            .text
            .clone()
            .unwrap_or_default(),
        find(&scene, "total")
            .unwrap()
            .text
            .clone()
            .unwrap_or_default(),
    )
}

#[test]
fn undeclared_app_key_written_by_an_action_repaints() {
    let mut session = enter(&stepper_doc(""));
    tap(&mut session, "minus");
    assert_eq!(texts(&session), ("1".into(), "¥24".into()));
    tap(&mut session, "plus");
    assert_eq!(texts(&session), ("2".into(), "¥48".into()));
}

#[test]
fn declared_quantity_steps_and_clamps() {
    let mut session = enter(&stepper_doc(
        r#""state": { "qty": { "type": "int", "default": 1 } },"#,
    ));
    assert_eq!(texts(&session), ("1".into(), "¥24".into()));
    let mut seen = Vec::new();
    for id in ["plus", "plus", "minus", "minus", "minus"] {
        tap(&mut session, id);
        seen.push(texts(&session));
    }
    let want: Vec<(String, String)> = [
        ("2", "¥48"),
        ("3", "¥72"),
        ("2", "¥48"),
        ("1", "¥24"),
        ("1", "¥24"),
    ]
    .iter()
    .map(|(a, b)| (a.to_string(), b.to_string()))
    .collect();
    assert_eq!(seen, want);
}

/// Bound inputs show their app-state value on the very first frame —
/// before any focus, keystroke or screen switch — and painting fires no
/// `onChange`.
#[test]
fn bound_inputs_paint_their_state_on_the_first_frame() {
    let src = r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "state": {
            "name": { "type": "string", "default": "Ann" },
            "bio": { "type": "string", "default": "Hello" },
            "age": { "type": "int", "default": 30 },
            "changes": { "type": "int", "default": 0 }
        },
        "children": [
            { "type": "frame", "id": "root", "width": 400, "height": 400, "children": [
                { "type": "text_input", "id": "name", "width": 200, "height": 40,
                  "placeholder": "Your name",
                  "bindings": { "bind:value": "$app.name" },
                  "events": { "onChange": [ { "set": { "$app.changes": "$app.changes + 1" } } ] } },
                { "type": "text_area", "id": "bio", "width": 200, "height": 80,
                  "bindings": { "bind:value": "$state.bio" } },
                { "type": "number_input", "id": "age", "width": 200, "height": 40,
                  "bindings": { "bind:value": "$app.age" } }
            ]}
        ]
    }"##;
    let doc = jian_ops_schema::load_str(src).expect("parse").value;
    let session = enter(&doc);
    let scene = session.preview_scene_for_test();
    let value = |id: &str| {
        find(&scene, id)
            .and_then(|n| n.widget.as_ref())
            .and_then(|w| w.value_str.clone())
    };
    assert_eq!(value("name").as_deref(), Some("Ann"));
    assert_eq!(value("bio").as_deref(), Some("Hello"));
    assert_eq!(value("age").as_deref(), Some("30"));
    assert_eq!(
        session
            .runtime
            .state
            .app_get("changes")
            .and_then(|v| v.as_i64()),
        Some(0),
        "painting is read-only"
    );
}
