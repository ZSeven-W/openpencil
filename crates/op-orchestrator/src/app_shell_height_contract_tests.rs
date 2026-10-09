use super::*;
use op_editor_core::EditorState;

fn fixture(table_height: f64) -> Value {
    json!({"type":"frame","id":"root","name":"Admin Console","width":1440,"height":900,"layout":"horizontal","children":[
      {"type":"frame","id":"sidebar","name":"Sidebar","width":260,"height":"fill_container","layout":"vertical","children":[
        {"type":"frame","id":"section","name":"Grouped Navigation Sidebar","width":"fill_container","height":"fit_content","layout":"vertical","children":[
          {"type":"frame","id":"surface","name":"Navigation rail","width":"fill_container","height":"fill_container","layout":"vertical","justifyContent":"space_between","children":[
            {"type":"frame","id":"nav","height":220,"children":[{"type":"text","id":"nav-copy","content":"Users"}]},
            {"type":"frame","id":"profile","name":"Account footer","height":60,"children":[{"type":"text","id":"profile-copy","content":"Test account"}]}
          ]}
        ]}
      ]},
      {"type":"frame","id":"main","name":"Main Content","width":"fill_container","height":"fill_container","layout":"vertical","padding":32,"gap":16,"children":[
        {"type":"frame","id":"header","name":"Header","height":72},
        {"type":"frame","id":"filters","name":"Filters","height":48},
        {"type":"frame","id":"table-section","name":"Users Table","width":"fill_container","height":"fit_content","layout":"vertical","children":[
          {"type":"frame","id":"table","name":"Table Container","width":"fill_container","height":table_height,"layout":"vertical","children":[
            {"type":"frame","id":"row-1","name":"Row","layout":"horizontal","height":48,"children":[{"type":"text","id":"name-1","content":"Alice"},{"type":"text","id":"status-1","content":"Active"}]},
            {"type":"frame","id":"row-2","name":"Row","layout":"horizontal","height":48,"children":[{"type":"text","id":"name-2","content":"Bob"},{"type":"text","id":"status-2","content":"Active"}]}
          ]}
        ]},
        {"type":"frame","id":"pagination","name":"Pagination Footer","width":"fill_container","height":48,"children":[{"type":"text","id":"pagination-copy","content":"1–10 of 128"}]}
      ]}
    ]})
}

fn state(root: Value) -> EditorState {
    EditorState::from_document(
        serde_json::from_value(json!({"version":"1.0.0","children":[root]})).unwrap(),
    )
}

fn bounds(state: &EditorState, id: &str) -> (f64, f64, f64, f64) {
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
    let b = scene.active_page().unwrap().find(id).unwrap().bounds;
    (
        b.origin.x as f64,
        b.origin.y as f64,
        b.size.x as f64,
        b.size.y as f64,
    )
}

#[test]
fn nested_sidebar_surface_fills_the_shell_and_pagination_reaches_the_bottom() {
    let mut state = state(fixture(400.0));
    assert!(bounds(&state, "surface").3 < 400.0);
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    assert!(repair_height_contract(&mut sink, "root"));
    assert_eq!(bounds(sink.state(), "surface").3, 900.0);
    let footer = bounds(sink.state(), "pagination");
    assert!((footer.1 + footer.3 - 868.0).abs() < 1.0);
    let before = sink.state().doc.clone();
    assert!(!repair_height_contract(&mut sink, "root"));
    assert_eq!(sink.state().doc, before);
}

#[test]
fn long_table_is_preserved_in_a_scrollable_middle_with_a_visible_footer() {
    let mut state = state(fixture(1200.0));
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    repair_height_contract(&mut sink, "root");
    let footer = bounds(sink.state(), "pagination");
    assert!((footer.1 + footer.3 - 868.0).abs() < 1.0);
    assert_eq!(bounds(sink.state(), "table").3, 1200.0);
    let main =
        serde_json::to_value(&sink.state().doc).unwrap()["children"][0]["children"][1].clone();
    let viewport = &main["children"][2];
    assert_eq!(viewport["role"], "scroll-area");
    assert_eq!(viewport["clipContent"], true);
    assert!(!viewport["events"]["onScroll"]
        .as_array()
        .unwrap()
        .is_empty());
    for id in ["name-1", "name-2", "pagination-copy"] {
        let _ = bounds(sink.state(), id);
    }
}

#[test]
fn near_fit_rows_spend_whitespace_without_shrinking_text_or_losing_rows() {
    let mut root = fixture(400.0);
    let table = &mut root["children"][1]["children"][2]["children"][0];
    table["height"] = json!("fit_content");
    table["children"] = Value::Array((0..10).map(|i| json!({
        "type":"frame","id":format!("data-row-{i}"),"name":"User Row","width":"fill_container","layout":"horizontal","padding":[12,20],"children":[
            {"type":"frame","id":format!("user-info-{i}"),"layout":"vertical","gap":2,"children":[
                {"type":"text","id":format!("user-name-{i}"),"fontSize":14,"content":"Alice"},
                {"type":"text","id":format!("user-email-{i}"),"fontSize":12,"content":"alice@example.test"}
            ]},
            {"type":"text","id":format!("user-status-{i}"),"fontSize":14,"content":"Active"}
        ]
    })).collect());
    let mut state = state(root);
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    repair_height_contract(&mut sink, "root");
    let value = serde_json::to_value(&sink.state().doc).unwrap();
    let viewport = &value["children"][0]["children"][1]["children"][2];
    let vp = bounds(sink.state(), viewport["id"].as_str().unwrap());
    let last = bounds(sink.state(), "data-row-9");
    assert!(last.1 + last.3 <= vp.1 + vp.3 + 1.0);
    let table = &viewport["children"][0]["children"][0];
    assert_eq!(children(table).len(), 10);
    assert_eq!(
        table["children"][0]["children"][0]["children"][0]["fontSize"],
        14.0
    );
    assert!(last.3 >= 44.0);
}

#[test]
fn complete_cleanup_keeps_the_shell_height_contract() {
    let mut state = state(fixture(400.0));
    let root = state.active_children()[0].clone();
    let mut plan = crate::loop_finalize::synthesize_plan(std::slice::from_ref(&root), 1440.0);
    plan.root_frame.height = 900.0;
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    crate::cleanup::run_cleanup_passes(&mut sink, &plan, &["root"]);
    let value = serde_json::to_value(&sink.state().doc).unwrap();
    let mut issues = Vec::new();
    height_contract_diagnostics(
        &value["children"][0],
        |id| Some(bounds(sink.state(), id)),
        &mut issues,
    );
    assert!(issues.is_empty(), "{issues:?}");
}

#[test]
fn diagnostics_catch_short_sidebar_and_floating_pagination() {
    let root = fixture(400.0);
    let state = state(root.clone());
    let mut out = Vec::new();
    height_contract_diagnostics(&root, |id| Some(bounds(&state, id)), &mut out);
    assert!(out
        .iter()
        .any(|x| x.contains("desktop-shell-sidebar-height")));
    assert!(out
        .iter()
        .any(|x| x.contains("desktop-shell-pagination-bottom")));
}

#[test]
fn diagnostics_use_the_fixed_board_when_a_dense_main_column_grows_past_it() {
    let root = fixture(1200.0);
    let state = state(root.clone());
    let mut out = Vec::new();
    height_contract_diagnostics(&root, |id| Some(bounds(&state, id)), &mut out);
    assert!(out
        .iter()
        .any(|x| x.contains("desktop-shell-pagination-bottom")));
}

#[test]
fn mobile_and_non_table_editorial_pages_are_unchanged() {
    for width in [390.0, 1440.0] {
        let mut root = fixture(400.0);
        root["width"] = json!(width);
        if width == 1440.0 {
            root["children"][1]["children"] =
                json!([{"type":"text","id":"article","content":"Editorial article"}]);
        }
        let mut state = state(root);
        let before = state.doc.clone();
        let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
        assert!(!repair_height_contract(&mut sink, "root"));
        assert_eq!(sink.state().doc, before);
    }
}

#[test]
fn incomplete_roots_do_not_panic_or_acquire_shell_layout() {
    for kids in [json!([]), json!([{"type":"frame","id":"only-child"}])] {
        let mut state = state(
            json!({"type":"frame","id":"root","width":1440,"height":900,"layout":"horizontal","children":kids}),
        );
        let before = state.doc.clone();
        let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
        assert!(!repair_height_contract(&mut sink, "root"));
        assert_eq!(sink.state().doc, before);
    }
}

#[test]
fn chinese_labels_and_explicit_roles_keep_the_same_desktop_contract() {
    for semantic in [false, true] {
        let mut root = fixture(400.0);
        root["children"][0]["name"] = json!(if semantic { "Panel A" } else { "侧边栏" });
        if semantic {
            root["children"][0]["role"] = json!("sidebar");
        }
        root["children"][1]["children"][2]["children"][0]["name"] = json!("用户表格");
        root["children"][1]["children"][3]["name"] = json!("分页页脚");
        let mut state = state(root);
        let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
        assert!(repair_height_contract(&mut sink, "root"));
        assert_eq!(bounds(sink.state(), "surface").3, 900.0);
        let footer = bounds(sink.state(), "pagination");
        assert!((footer.1 + footer.3 - 868.0).abs() < 1.0);
    }
}

#[test]
#[ignore = "requires a retained .op and output directory"]
fn replay_saved_console_preserves_copy_and_pins_the_shell_chrome() {
    fn text(n: &Value, out: &mut std::collections::BTreeMap<String, Value>) {
        if n.get("type").and_then(Value::as_str) == Some("text") {
            out.insert(n["id"].as_str().unwrap().to_owned(), n["content"].clone());
        }
        for c in children(n) {
            text(c, out);
        }
    }
    let input = std::env::var("OPENPENCIL_REPLAY_INPUT").unwrap();
    let dir = std::path::PathBuf::from(std::env::var("OPENPENCIL_REPLAY_DIR").unwrap());
    std::fs::create_dir_all(&dir).unwrap();
    let doc = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
    let mut state = EditorState::from_document(doc);
    let before = serde_json::to_value(&state.doc).unwrap();
    let root = before["children"][0].clone();
    let root_id = root["id"].as_str().unwrap();
    let (_, main) = shell(&root).unwrap();
    let (_, footer) = table_and_footer(main).unwrap();
    let footer_id = footer["id"].as_str().unwrap();
    let mut issues_before = Vec::new();
    height_contract_diagnostics(&root, |id| Some(bounds(&state, id)), &mut issues_before);
    let footer_before = bounds(&state, footer_id);
    let mut original = std::collections::BTreeMap::new();
    text(&before, &mut original);
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    assert!(repair_height_contract(&mut sink, root_id));
    let after = serde_json::to_value(&sink.state().doc).unwrap();
    let mut preserved = std::collections::BTreeMap::new();
    text(&after, &mut preserved);
    assert_eq!(original, preserved);
    let mut issues_after = Vec::new();
    height_contract_diagnostics(
        &after["children"][0],
        |id| Some(bounds(sink.state(), id)),
        &mut issues_after,
    );
    assert!(issues_after.is_empty(), "{issues_after:?}");
    let footer_after = bounds(sink.state(), footer_id);
    std::fs::write(
        dir.join("after.op"),
        serde_json::to_vec_pretty(&sink.state().doc).unwrap(),
    )
    .unwrap();
    std::fs::write(
        dir.join("layout-receipt.json"),
        serde_json::to_vec_pretty(&json!({
            "issues_before":issues_before,"issues_after":issues_after,
            "footer_before":footer_before,"footer_after":footer_after,
            "source_text_ids_and_contents_preserved":true,"extra_model_calls":0
        }))
        .unwrap(),
    )
    .unwrap();
}
