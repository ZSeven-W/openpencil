use super::{test_measure, PreviewSession};
use crate::{PreviewInput, PreviewInputEnvelope};
use jian_core::gesture::pointer::Modifiers;
use op_editor_core::table_filter_contract::{ROW_MARKER, TABLE_MARKER};
use serde_json::json;
fn fixture() -> jian_ops_schema::PenDocument {
    let mut rows = vec![json!({"type":"frame","id":"header","width":"fill_container","height":30})];
    for (i, (name, status)) in [
        ("Alice Lee", "active"),
        ("张三", "invited"),
        ("Béatrice", "active"),
    ]
    .iter()
    .enumerate()
    {
        let corpus = format!("{name} {status}").to_lowercase();
        rows.push(json!({"type":"frame","id":format!("row-{i}"),"width":"fill_container","height":40,"layout":"horizontal","explain":ROW_MARKER,
            "bindings":{"visible":format!("($app.status == 'all' || $app.status == '{status}') && contains({},lower(trim($app.query)))",json!(corpus))},
            "children":[{"type":"text","id":format!("name-{i}"),"content":name,"width":200,"height":20}]}));
    }
    serde_json::from_value(json!({"version":"1.1","state":{"query":{"type":"string","default":""},"status":{"type":"string","default":"all"}},"children":[
        {"type":"frame","id":"root","x":80,"y":40,"width":500,"height":400,"layout":"vertical","children":[
            {"type":"text_input","id":"query","width":240,"height":40,"value":"","placeholder":"Search users","bindings":{"bind:value":"$state.query"}},
            {"type":"select","id":"status","width":180,"height":40,"value":"all","options":[{"value":"all","label":"All"},{"value":"active","label":"Active"},{"value":"invited","label":"Invited"}],"bindings":{"bind:value":"$state.status"}},
            {"type":"frame","id":"table","width":"fill_container","height":"fit_content","layout":"vertical","explain":TABLE_MARKER,"children":rows}
        ]}]})).unwrap()
}
fn type_text(session: &mut PreviewSession, text: &str) {
    session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Text(text.into())));
}
fn rows(session: &PreviewSession) -> Vec<String> {
    session
        .preview_scene_for_test()
        .active_page()
        .unwrap()
        .find("table")
        .unwrap()
        .children
        .iter()
        .filter(|n| n.id.starts_with("row-"))
        .map(|n| n.id.clone())
        .collect()
}
fn clear(session: &mut PreviewSession) {
    session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Key {
        key: "a".into(),
        code: "KeyA".into(),
        repeat: false,
        modifiers: Modifiers::CMD,
    }));
    session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Key {
        key: "Backspace".into(),
        code: "Backspace".into(),
        repeat: false,
        modifiers: Modifiers::default(),
    }));
}
#[test]
fn typing_retains_caret_through_reflows_and_clear_restores_all_rows() {
    let doc = fixture();
    let before = doc.clone();
    let mut session = PreviewSession::enter(
        &doc,
        (800.0, 600.0),
        &Default::default(),
        0,
        false,
        false,
        test_measure(),
        0,
    )
    .unwrap();
    assert!(session.focus_node_for_test("query"));
    for text in [" A", "L", "I", "C", "E "] {
        type_text(&mut session, text);
    }
    assert_eq!(session.widget_text_for_test("query"), " ALICE ");
    assert_eq!(rows(&session), vec!["row-0"]);
    clear(&mut session);
    assert_eq!(rows(&session).len(), 3);
    type_text(&mut session, "张三");
    assert_eq!(rows(&session), vec!["row-1"]);
    clear(&mut session);
    type_text(&mut session, "nobody");
    assert!(rows(&session).is_empty());
    clear(&mut session);
    assert_eq!(rows(&session).len(), 3);
    assert_eq!(doc, before);
}
#[test]
fn text_search_and_enum_filters_compose_and_restore_the_empty_state() {
    let mut session = PreviewSession::enter(
        &fixture(),
        (800.0, 600.0),
        &Default::default(),
        0,
        false,
        false,
        test_measure(),
        0,
    )
    .unwrap();
    assert!(session.focus_node_for_test("query"));
    type_text(&mut session, "张三");
    session.set_state("status", json!("active"));
    assert!(rows(&session).is_empty());
    session.set_state("status", json!("invited"));
    assert_eq!(rows(&session), vec!["row-1"]);
    clear(&mut session);
    assert_eq!(rows(&session), vec!["row-1"]);
    session.set_state("status", json!("all"));
    assert_eq!(rows(&session).len(), 3);
}
