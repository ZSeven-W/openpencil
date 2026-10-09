use super::*;
use op_editor_core::EditorState;
fn text(id: &str, copy: &str) -> Value {
    json!({"type":"text","id":id,"content":copy,"width":120,"height":20})
}
fn button(id: &str, name: &str, copy: &str) -> Value {
    let mut label = text(&format!("{id}-label"), copy);
    label["width"] = json!("fit_content");
    json!({"type":"frame","id":id,"name":name,"role":"button","width":32,"height":32,"layout":"horizontal","justifyContent":"center","alignItems":"center","children":[label]})
}
fn fixture(count: usize, clock: bool) -> Value {
    let reference = 2_000_000_000_000u64;
    let mut rows = vec![
        json!({"type":"frame","id":"header","layout":"horizontal","height":30,"children":[text("name-head","Name"),text("status-head","Status")]}),
    ];
    for i in 0..count {
        rows.push(json!({"type":"frame","id":format!("row-{i}"),"name":"User row","width":"fill_container","height":40,"layout":"horizontal",
            "explain":if clock{format!("op-table-time:v1 {}",json!({"last_active_ms":if i==count-1{Value::Null}else{json!(reference-(i as u64)*86_400_000)}}))}else{String::new()},
            "children":[text(&format!("name-{i}"),&format!("User {i:02}")),text(&format!("status-{i}"),if i%2==0{"Active"}else{"Invited"})]}));
    }
    json!({"version":"1.1","children":[{"type":"frame","id":"root","width":800,"height":1400,"layout":"vertical","children":[
        {"type":"text_input","id":"query","name":"Search users","width":240,"height":40,"value":""},
        {"type":"select","id":"status","name":"Select Status","width":180,"height":40,"value":"all","options":[{"value":"all","label":"All statuses"},{"value":"active","label":"Active"},{"value":"invited","label":"Invited"}]},
        {"type":"select","id":"time","name":"Select Last active","width":180,"height":40,"value":"7d","options":[{"value":"24h","label":"Last 24 hours"},{"value":"7d","label":"Last 7 days"},{"value":"30d","label":"Last 30 days"}]},
        {"type":"frame","id":"table","name":"Users Table","explain":if clock{format!("op-table-clock:v1 {}",json!({"reference_ms":reference}))}else{String::new()},"width":"fill_container","height":"fit_content","layout":"vertical","children":rows},
        {"type":"frame","id":"footer","name":"Pagination Footer","width":"fill_container","height":40,"layout":"horizontal","children":[json!({"type":"text","id":"counter","content":"Showing 1–10 of 84 users","width":"fill_container","height":"fit_content"}),
            {"type":"select","id":"size","name":"Rows Per Page Select","width":100,"height":32,"value":"10","options":[{"value":"5","label":"5 / page"},{"value":"10","label":"10 / page"},{"value":"25","label":"25 / page"}]},
            {"type":"frame","id":"pager","name":"Page Buttons","layout":"horizontal","gap":4,"children":[button("previous","Prev Button","<"),button("first","Page 1 Button","1"),button("second","Page 2 Button","2"),button("fake","Page 9 Button","9"),button("next","Next Button",">")]}
        ]}
    ]}]})
}
fn apply(value: Value) -> EditorState {
    let mut state = EditorState::from_document(serde_json::from_value(value).unwrap());
    crate::geometry_validation::wire_interaction_backfill(
        &mut crate::loop_finalize::StateDocSink { state: &mut state },
    );
    state
}
fn node(state: &EditorState, id: &str) -> Value {
    serde_json::to_value(
        op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(id)).unwrap(),
    )
    .unwrap()
}
#[test]
fn pagination_uses_only_authored_records_and_preserves_all_rows() {
    let mut state = apply(fixture(23, false));
    let spec = op_editor_core::table_pagination::Spec::from_node(&node(&state, "table")).unwrap();
    assert_eq!(
        node(&state, "table")["children"].as_array().unwrap().len(),
        24
    );
    assert_eq!(
        node(&state, "pager")["children"].as_array().unwrap().len(),
        5
    );
    assert_eq!(
        state.doc.state.as_ref().unwrap()[&spec.pages].default,
        Some(json!(3))
    );
    assert!(node(&state, "time")["value"].is_null());
    assert_eq!(node(&state, "time")["placeholder"], "Dates unavailable");
    assert_eq!(node(&state, "time")["enabled"], false);
    assert!(node(&state, "time")["bindings"].is_null());
    let settled = state.doc.clone();
    crate::geometry_validation::wire_interaction_backfill(
        &mut crate::loop_finalize::StateDocSink { state: &mut state },
    );
    assert_eq!(state.doc, settled);
}
#[test]
fn single_page_discards_fake_page_numbers_and_custom_pagers_are_untouched() {
    let state = apply(fixture(10, false));
    assert_eq!(
        node(&state, "pager")["children"].as_array().unwrap().len(),
        3
    );
    let mut value = fixture(23, false);
    value["children"][0]["children"][4]["children"][2]["children"][4]["events"] =
        json!({"onTap":[{"set":{"$app.custom":"1"}}]});
    let state = apply(value);
    assert!(op_editor_core::table_pagination::Spec::from_node(&node(&state, "table")).is_none());
    assert_eq!(
        node(&state, "next")["events"]["onTap"][0]["set"]["$app.custom"],
        "1"
    );
}
#[test]
fn time_filter_requires_complete_precise_metadata_and_excludes_missing_activity() {
    let state = apply(fixture(23, true));
    assert_eq!(node(&state, "time")["value"], "__op_all");
    assert!(node(&state, "row-0")["bindings"]["visible"]
        .as_str()
        .unwrap()
        .contains("24h"));
    assert!(!node(&state, "row-22")["bindings"]["visible"]
        .as_str()
        .unwrap()
        .contains("24h"));
    for broken in [json!({"last_active_ms":2_000_000_000_001u64}), json!({})] {
        let mut value = fixture(23, true);
        value["children"][0]["children"][3]["children"][1]["explain"] =
            json!(format!("op-table-time:v1 {broken}"));
        let state = apply(value);
        assert!(node(&state, "time")["bindings"].is_null());
    }
}
#[test]
#[ignore = "requires OPENPENCIL_QA_PAGING_INPUT and OPENPENCIL_QA_PAGING_OUTPUT"]
fn replay_retained_local_pagination() {
    let source =
        std::fs::read_to_string(std::env::var("OPENPENCIL_QA_PAGING_INPUT").unwrap()).unwrap();
    let before = op_pen_loader::payload::load_canonical(&source)
        .unwrap()
        .value;
    let mut state = EditorState::from_document(before.clone());
    crate::geometry_validation::wire_interaction_backfill(
        &mut crate::loop_finalize::StateDocSink { state: &mut state },
    );
    let spec = op_editor_core::table_pagination::Spec::from_node(&node(&state, "n898")).unwrap();
    assert_eq!(
        state.doc.state.as_ref().unwrap()[&spec.total].default,
        Some(json!(10))
    );
    assert_eq!(
        state.doc.state.as_ref().unwrap()[&spec.pages].default,
        Some(json!(1))
    );
    let original = EditorState::from_document(before);
    for id in ["n898", "n843", "n890"] {
        let a = node(&state, id);
        let b = node(&original, id);
        if id == "n898" {
            assert_eq!(a["children"], b["children"]);
        } else if id == "n890" {
            assert_eq!(a["options"], b["options"]);
            assert_eq!(a["enabled"], false);
        } else {
            assert_eq!(a["value"], b["value"]);
        }
    }
    let output = std::path::PathBuf::from(std::env::var("OPENPENCIL_QA_PAGING_OUTPUT").unwrap());
    std::fs::create_dir_all(output.parent().unwrap()).unwrap();
    jian_ops_schema::image_table::write_document_with_extension(
        &mut std::fs::File::create(&output).unwrap(),
        &state.doc,
        &jian_ops_schema::image_thumbs::capture_snapshot(),
        "editorMeta",
        &op_pen_loader::EditorMeta::from_state(&state),
    )
    .unwrap();
    let mut demo = apply(fixture(23, true));
    crate::geometry_validation::wire_interaction_backfill(
        &mut crate::loop_finalize::StateDocSink { state: &mut demo },
    );
    jian_ops_schema::image_table::write_document_with_extension(
        &mut std::fs::File::create(output.with_file_name("23-record-demo.op")).unwrap(),
        &demo.doc,
        &Default::default(),
        "editorMeta",
        &op_pen_loader::EditorMeta::from_state(&demo),
    )
    .unwrap();
}

#[test]
fn unavailable_dates_recover_when_complete_metadata_is_added() {
    let state = apply(fixture(23, false));
    let mut value = serde_json::to_value(&state.doc).unwrap();
    let source = fixture(23, true);
    value["children"][0]["children"][3]["explain"] = json!(format!(
        "{}\n{}",
        value["children"][0]["children"][3]["explain"]
            .as_str()
            .unwrap(),
        source["children"][0]["children"][3]["explain"]
            .as_str()
            .unwrap()
    ));
    for (i, row) in value["children"][0]["children"][3]["children"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
        .skip(1)
    {
        row["explain"] = json!(format!(
            "{}\n{}",
            row["explain"].as_str().unwrap(),
            source["children"][0]["children"][3]["children"][i]["explain"]
                .as_str()
                .unwrap()
        ));
    }
    let state = apply(value);
    assert_eq!(node(&state, "time")["value"], "__op_all");
    assert!(node(&state, "time")["enabled"].is_null());
    assert!(node(&state, "time")["placeholder"].is_null());
    let paging = op_editor_core::table_pagination::Spec::from_node(&node(&state, "table")).unwrap();
    assert!(paging.filter_keys.contains(&key("time")));
}

#[test]
fn chinese_tables_localize_date_feedback_even_with_internal_english_names() {
    for clock in [false, true] {
        let mut value = fixture(23, clock);
        value["children"][0]["children"][3]["children"][0]["children"][1]["content"] =
            json!("状态");
        let state = apply(value);
        if clock {
            assert_eq!(node(&state, "time")["options"][0]["label"], "全部时间");
        } else {
            assert_eq!(node(&state, "time")["placeholder"], "日期未提供");
        }
    }
}
