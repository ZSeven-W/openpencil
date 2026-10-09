use super::{test_measure, PreviewSession};
use crate::{PreviewInput, PreviewInputEnvelope};
use jian_core::gesture::pointer::Modifiers;
use op_editor_core::table_pagination::Spec;
use serde_json::json;
fn fixture() -> jian_ops_schema::PenDocument {
    let mut rows = vec![
        json!({"type":"frame","id":"header","height":30,"width":"fill_container","layout":"horizontal","children":[{"type":"text","id":"name-head","content":"Name"},{"type":"text","id":"status-head","content":"Status"}]}),
    ];
    for i in 0..23 {
        rows.push(json!({"type":"frame","id":format!("row-{i}"),"width":"fill_container","height":40,"layout":"horizontal","children":[{"type":"text","id":format!("name-{i}"),"content":format!("User {i:02}"),"width":150,"height":20},{"type":"text","id":format!("status-{i}"),"content":"Active","width":80,"height":20}]}));
    }
    let button = |id: &str, name: &str, label: &str| json!({"type":"frame","id":id,"name":name,"role":"button","width":32,"height":32,"children":[{"type":"text","id":format!("{id}-label"),"content":label}]});
    let doc=serde_json::from_value(json!({"version":"1.1","children":[{"type":"frame","id":"root","x":80,"y":40,"width":800,"height":900,"layout":"vertical","children":[
        {"type":"text_input","id":"query","name":"Search users","width":240,"height":40,"value":""},
        {"type":"frame","id":"table","name":"Users Table","width":"fill_container","height":"fit_content","layout":"vertical","children":rows},
        {"type":"frame","id":"footer","name":"Pagination Footer","layout":"horizontal","children":[
            {"type":"text","id":"counter","content":"Showing 23 rows"},
            {"type":"select","id":"size","name":"Rows Per Page Select","width":100,"height":32,"value":"10","options":[{"value":"5","label":"5 / page"},{"value":"10","label":"10 / page"},{"value":"25","label":"25 / page"}]},
            {"type":"frame","id":"pager","name":"Page Buttons","layout":"horizontal","gap":4,"children":[button("prev","Prev Button","<"),button("one","Page 1 Button","1"),button("two","Page 2 Button","2"),button("next","Next Button",">")]}
        ]}
    ]}]})).unwrap();
    let mut state = op_editor_core::EditorState::from_document(doc);
    op_orchestrator::apply_loop_finalize(&mut state);
    state.doc
}
fn rows(s: &PreviewSession) -> Vec<String> {
    s.preview_scene_for_test()
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
fn click(s: &mut PreviewSession, id: &str) {
    let b = s
        .preview_scene_for_test()
        .active_page()
        .unwrap()
        .find(id)
        .unwrap()
        .bounds;
    s.dispatch_tap(b.origin.x + b.size.x / 2.0, b.origin.y + b.size.y / 2.0);
}
fn spec(doc: &jian_ops_schema::PenDocument) -> Spec {
    let mut out = vec![];
    op_editor_core::table_pagination::specs(&serde_json::to_value(doc).unwrap(), &mut out);
    out.remove(0)
}
#[test]
fn real_page_buttons_and_size_changes_reflow_without_losing_records() {
    let doc = fixture();
    let original = doc.clone();
    let paging = spec(&doc);
    let mut s = PreviewSession::enter(
        &doc,
        (1200.0, 1100.0),
        &Default::default(),
        0,
        false,
        false,
        test_measure(),
        0,
    )
    .unwrap();
    assert_eq!(
        rows(&s),
        (0..10).map(|i| format!("row-{i}")).collect::<Vec<_>>()
    );
    click(&mut s, "next");
    assert_eq!(
        rows(&s),
        (10..20).map(|i| format!("row-{i}")).collect::<Vec<_>>()
    );
    click(&mut s, "next");
    assert_eq!(rows(&s), vec!["row-20", "row-21", "row-22"]);
    click(&mut s, "next");
    assert_eq!(rows(&s).len(), 3);
    click(&mut s, "pager-local-page-1");
    assert_eq!(rows(&s).len(), 10);
    assert!(s.focus_node_for_test("size"));
    s.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Key {
        key: "ArrowUp".into(),
        code: "ArrowUp".into(),
        repeat: false,
        modifiers: Modifiers::default(),
    }));
    assert_eq!(rows(&s).len(), 5);
    assert_eq!(
        s.app_state_value_for_test(&paging.pages).unwrap().as_i64(),
        Some(5)
    );
    click(&mut s, "pager-local-page-5");
    assert_eq!(rows(&s), vec!["row-20", "row-21", "row-22"]);
    assert_eq!(doc, original);
}
#[test]
fn search_resets_page_and_saved_out_of_range_page_is_clamped() {
    let doc = fixture();
    let paging = spec(&doc);
    let mut s = PreviewSession::enter(
        &doc,
        (1200.0, 1100.0),
        &Default::default(),
        0,
        false,
        false,
        test_measure(),
        0,
    )
    .unwrap();
    click(&mut s, "pager-local-page-3");
    assert!(s.focus_node_for_test("query"));
    s.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Text(
        "User 00".into(),
    )));
    assert_eq!(rows(&s), vec!["row-0"]);
    assert_eq!(
        s.app_state_value_for_test(&paging.page).unwrap().as_i64(),
        Some(1)
    );
    s.set_state(&paging.page, json!(999));
    assert_eq!(
        s.app_state_value_for_test(&paging.page).unwrap().as_i64(),
        Some(1)
    );
    assert_eq!(rows(&s), vec!["row-0"]);
}
