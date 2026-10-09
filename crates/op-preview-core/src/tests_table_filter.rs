use super::{test_measure, PreviewSession};
use crate::{PreviewInput, PreviewInputEnvelope};
use jian_core::gesture::pointer::Modifiers;
use op_editor_core::table_filter_contract::{ROW_MARKER, TABLE_MARKER};
use serde_json::json;

fn fixture() -> jian_ops_schema::PenDocument {
    let mut rows = vec![
        json!({"type":"frame","id":"header","height":30,"width":"fill_container","children":[]}),
    ];
    for (i, status) in ["active", "suspended", "active", "invited"]
        .iter()
        .enumerate()
    {
        rows.push(json!({"type":"frame","id":format!("row-{i}"),"explain":ROW_MARKER,"height":40,"width":"fill_container","layout":"horizontal",
            "bindings":{"visible":format!("$app.status == 'all' || $app.status == '{status}'")},
            "children":[{"type":"text","id":format!("text-{i}"),"width":120,"height":20,"content":status}]}));
    }
    serde_json::from_value(json!({"version":"1.1","state":{"status":{"type":"string","default":"all"}},"children":[
        {"type":"frame","id":"root","x":80,"y":40,"width":400,"height":400,"layout":"vertical","children":[
            {"type":"select","id":"filter","width":180,"height":40,"value":"all","bindings":{"bind:value":"$state.status"},
                "options":[{"value":"all","label":"All statuses"},{"value":"active","label":"Active"},{"value":"suspended","label":"Suspended"},{"value":"invited","label":"Invited"},{"value":"none","label":"Empty"}]},
            {"type":"frame","id":"viewport","role":"scroll-area","width":"fill_container","height":100,"clipContent":true,"layout":"vertical",
                "events":{"onScroll":[{"set":{"$app.scrolled":"true"}}]},"children":[
                    {"type":"frame","id":"table","name":"Table","explain":format!("{TABLE_MARKER} lang=en"),"width":"fill_container","height":"fit_content","layout":"vertical","children":rows}]},
            {"type":"text","id":"footer","content":"Original count","height":24,"width":240}
        ]}]})).unwrap()
}
fn key(session: &mut PreviewSession) {
    session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Key {
        key: "ArrowDown".into(),
        code: "ArrowDown".into(),
        repeat: false,
        modifiers: Modifiers::default(),
    }));
}
fn scene_rows(session: &PreviewSession) -> Vec<String> {
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
#[test]
fn real_select_keys_reflow_rows_restore_all_and_show_empty_state() {
    let doc = fixture();
    let original = serde_json::to_value(&doc).unwrap();
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
    assert_eq!(scene_rows(&session).len(), 4);
    assert!(session.focus_node_for_test("filter"));
    key(&mut session); // All -> Active.
    assert_eq!(
        session.app_state_value_for_test("status").unwrap().0,
        json!("active")
    );
    assert_eq!(scene_rows(&session), vec!["row-0", "row-2"]);
    let scene = session.preview_scene_for_test();
    let page = scene.active_page().unwrap();
    let a = page.find("row-0").unwrap().bounds;
    let b = page.find("row-2").unwrap().bounds;
    assert!(
        (b.origin.y - a.origin.y - a.size.y).abs() < 1.0,
        "{a:?} {b:?}"
    );
    let mut runtime_ids = std::collections::BTreeSet::new();
    op_editor_core::table_filter_contract::row_ids(
        &serde_json::to_value(&session.runtime.document.as_ref().unwrap().schema).unwrap(),
        &mut runtime_ids,
    );
    assert_eq!(runtime_ids.len(), 2);
    key(&mut session);
    assert_eq!(scene_rows(&session), vec!["row-1"]);
    key(&mut session);
    assert_eq!(scene_rows(&session), vec!["row-3"]);
    key(&mut session);
    assert!(scene_rows(&session).is_empty());
    let scene = session.preview_scene_for_test();
    assert_eq!(
        scene
            .active_page()
            .unwrap()
            .find("table-filter-empty-label")
            .unwrap()
            .text
            .as_deref(),
        Some("No matching rows")
    );
    key(&mut session);
    assert_eq!(scene_rows(&session).len(), 4);
    assert_eq!(serde_json::to_value(&doc).unwrap(), original);
}
#[test]
fn filtering_clamps_the_old_scroll_offset_to_the_shorter_table() {
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
    session.dispatch_wheel(200.0, 100.0, 0.0, -1000.0);
    session.set_state("status", json!("suspended"));
    let scene = session.preview_scene_for_test();
    let page = scene.active_page().unwrap();
    let row = page.find("row-1").unwrap().bounds;
    let viewport = page.find("viewport").unwrap().bounds;
    assert!(row.origin.y >= viewport.origin.y, "{row:?} {viewport:?}");
    assert!(row.origin.y + row.size.y <= viewport.origin.y + viewport.size.y);
}
#[test]
fn unmarked_hidden_rows_keep_the_authored_layout_semantics() {
    let mut value = serde_json::to_value(fixture()).unwrap();
    let rows = value["children"][0]["children"][1]["children"][0]["children"]
        .as_array_mut()
        .unwrap();
    for row in rows {
        row.as_object_mut().unwrap().remove("explain");
    }
    let doc = serde_json::from_value(value).unwrap();
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
    assert_eq!(
        session.set_state("status", json!("suspended")),
        crate::InvalidationKind::HitTest
    );
    assert_eq!(scene_rows(&session).len(), 4);
}

#[test]
fn initial_filter_and_routed_screen_return_materialize_the_saved_selection() {
    let mut value = serde_json::to_value(fixture()).unwrap();
    value["state"]["status"]["default"] = json!("suspended");
    value["children"][0]["screen"] = json!("/");
    value["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"frame","id":"detail","screen":"/detail","width":400,"height":400}));
    let doc = serde_json::from_value(value).unwrap();
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
    assert!(session.is_app_mode());
    assert_eq!(scene_rows(&session), vec!["row-1"]);
    session.runtime.nav.push("/detail");
    session.reconcile(1);
    session.runtime.nav.push("/");
    session.reconcile(2);
    assert_eq!(scene_rows(&session), vec!["row-1"]);
    assert!(session.focus_node_for_test("filter"));
    session.set_logical_now_ms(1000);
    key(&mut session);
    assert_eq!(scene_rows(&session), vec!["row-3"]);
}

#[test]
fn composed_filter_count_expression_matches_individual_predicates() {
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
    session.set_state("status", json!("active"));
    let expression="(to_num($app.status == 'all' || $app.status == 'active') + to_num($app.status == 'all' || $app.status == 'suspended') + to_num($app.status == 'all' || $app.status == 'active')) + ' rows'";
    let compiled = jian_core::expression::Expression::compile(expression).unwrap();
    let (value, warnings) = compiled.eval(&session.runtime.state, None, None);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(value.0, json!("2 rows"));
}
