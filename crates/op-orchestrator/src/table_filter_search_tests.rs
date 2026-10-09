use super::*;
use op_editor_core::EditorState;
fn fixture() -> Value {
    let rows: Vec<_> = [
        ("ALICE \"LEE\"", "Active"),
        ("张三", "Invited"),
        ("Béatrice", "Active"),
    ]
    .iter()
    .enumerate()
    .map(|(i, (name, status))| {
        json!({
        "type":"frame","id":format!("row-{i}"),"layout":"horizontal","height":40,"children":[
            {"type":"text","id":format!("name-{i}"),"content":name},
            {"type":"text","id":format!("status-{i}"),"content":status}]})
    })
    .collect();
    let mut kids = vec![
        json!({"type":"frame","id":"header","layout":"horizontal","children":[
        {"type":"text","id":"head-name","content":"Name"},{"type":"text","id":"head-status","content":"Status"}]}),
    ];
    kids.extend(rows);
    json!({"version":"1.1","children":[{"type":"frame","id":"root","width":600,"height":400,"children":[
        {"type":"text_input","id":"query","name":"Search Input","value":"","placeholder":"Search users"},
        {"type":"select","id":"status","name":"Select Status","value":"all","options":[{"value":"all","label":"All statuses"},{"value":"active","label":"Active"},{"value":"invited","label":"Invited"}]},
        {"type":"frame","id":"table","name":"Users Table","layout":"vertical","children":kids},
        {"type":"frame","id":"footer","name":"Pagination footer","children":[{"type":"text","id":"count","content":"Showing 3 users"}]}
    ]}]})
}
fn apply(value: Value) -> EditorState {
    let mut state = EditorState::from_document(serde_json::from_value(value).unwrap());
    super::super::wire(&mut crate::loop_finalize::StateDocSink { state: &mut state });
    state
}
fn node(state: &EditorState, id: &str) -> Value {
    serde_json::to_value(
        op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(id)).unwrap(),
    )
    .unwrap()
}
#[test]
fn new_search_composes_with_enums_and_preserves_the_original_copy() {
    let mut state = apply(fixture());
    assert_eq!(
        node(&state, "query")["bindings"]["bind:value"],
        format!("$state.{}", key("query"))
    );
    let row = node(&state, "row-0");
    let expr = row["bindings"]["visible"].as_str().unwrap();
    assert!(expr.contains("contains(") && expr.contains("trim(") && expr.contains(&key("status")));
    assert!(expr.contains(r#"\"lee\""#));
    assert_eq!(row["children"][0]["content"], "ALICE \"LEE\"");
    assert_eq!(node(&state, "name-1")["content"], "张三");
    assert!(node(&state, "count")["bindings"]["content"]
        .as_str()
        .unwrap()
        .contains(&key("query")));
    let settled = state.doc.clone();
    super::super::wire(&mut crate::loop_finalize::StateDocSink { state: &mut state });
    assert_eq!(state.doc, settled);
}
#[test]
fn legacy_owned_tables_can_gain_search_once_without_resetting_filters() {
    let mut value = fixture();
    value["children"][0]["children"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    let legacy = apply(value);
    let before = node(&legacy, "row-0")["bindings"]["visible"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut value = serde_json::to_value(&legacy.doc).unwrap();
    value["children"][0]["children"][2]["children"][0]["bindings"]["opacity"] = json!("0.75");
    value["children"][0]["children"]
        .as_array_mut()
        .unwrap()
        .push(fixture()["children"][0]["children"][0].clone());
    let mut state = apply(value);
    assert!(node(&state, "row-0")["bindings"]["visible"]
        .as_str()
        .unwrap()
        .contains(&before));
    assert!(node(&state, "count")["bindings"]["content"]
        .as_str()
        .unwrap()
        .contains(&key("query")));
    assert_eq!(node(&state, "status"), node(&legacy, "status"));
    assert_eq!(node(&state, "count")["bindings"]["opacity"], "0.75");
    let settled = state.doc.clone();
    super::super::wire(&mut crate::loop_finalize::StateDocSink { state: &mut state });
    assert_eq!(state.doc, settled);
}
#[test]
fn custom_search_nonempty_defaults_and_ambiguity_are_preserved() {
    for patch in [
        json!({"bindings":{"bind:value":"$state.custom"}}),
        json!({"events":{"onChange":[{"set":{"$app.custom":"$event.value"}}]}}),
        json!({"value":"Alice"}),
        json!({"state":{"q":{"type":"string","default":""}}}),
    ] {
        let mut value = fixture();
        for (k, v) in patch.as_object().unwrap() {
            value["children"][0]["children"][0][k] = v.clone();
        }
        let before = value["children"][0]["children"][0].clone();
        let state = apply(value);
        assert_eq!(node(&state, "query")["bindings"], before["bindings"]);
        assert!(!node(&state, "row-0")["bindings"]["visible"]
            .as_str()
            .unwrap()
            .contains(&key("query")));
    }
    let mut value = fixture();
    let mut duplicate = value["children"][0]["children"][0].clone();
    duplicate["id"] = json!("other-query");
    value["children"][0]["children"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    let state = apply(value);
    assert!(node(&state, "query")["bindings"].is_null());
}
#[test]
fn search_without_enum_filters_and_custom_counter_preservation() {
    let mut value = fixture();
    value["children"][0]["children"]
        .as_array_mut()
        .unwrap()
        .remove(1);
    value["children"][0]["children"][2]["children"][0]["bindings"] =
        json!({"content":"$app.custom_count"});
    let state = apply(value);
    assert!(node(&state, "row-0")["bindings"]["visible"]
        .as_str()
        .unwrap()
        .contains("contains("));
    assert_eq!(
        node(&state, "count")["bindings"]["content"],
        "$app.custom_count"
    );
}

#[test]
#[ignore = "requires OPENPENCIL_QA_SEARCH_INPUT and OPENPENCIL_QA_SEARCH_OUTPUT"]
fn replay_retained_console_search_and_status_tints() {
    let source =
        std::fs::read_to_string(std::env::var("OPENPENCIL_QA_SEARCH_INPUT").unwrap()).unwrap();
    let doc = op_pen_loader::payload::load_canonical(&source)
        .unwrap()
        .value;
    let mut state = EditorState::from_document(doc.clone());
    {
        let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
        super::super::wire(&mut sink);
        super::super::repair_status_tints(&mut sink);
    }
    let before = EditorState::from_document(doc);
    for (id, color) in [
        ("n959", "#DC26261A"),
        ("n1031", "#DC26261A"),
        ("n995", "#D977061A"),
        ("n1067", "#D977061A"),
    ] {
        assert_eq!(node(&state, id)["fill"][0]["color"], color);
    }
    fn snapshot(mut value: Value) -> Value {
        if let Some(object) = value.as_object_mut() {
            for key in ["bindings", "explain", "state"] {
                object.remove(key);
            }
            if ["n959", "n1031", "n995", "n1067"]
                .iter()
                .any(|id| object.get("id").is_some_and(|v| v == id))
            {
                object.remove("fill");
            }
            if let Some(kids) = object.get_mut("children").and_then(Value::as_array_mut) {
                for child in kids.iter_mut() {
                    *child = snapshot(child.clone());
                }
            }
        }
        value
    }
    assert_eq!(
        snapshot(serde_json::to_value(&before.doc).unwrap()),
        snapshot(serde_json::to_value(&state.doc).unwrap())
    );
    assert_eq!(
        node(&state, "n843")["bindings"]["bind:value"],
        format!("$state.{}", key("n843"))
    );
    assert_eq!(node(&state, "n890"), node(&before, "n890"));
    assert_eq!(node(&state, "n1096"), node(&before, "n1096"));
    let settled = state.doc.clone();
    {
        let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
        super::super::wire(&mut sink);
        super::super::repair_status_tints(&mut sink);
    }
    assert_eq!(state.doc, settled);
    let output = std::path::PathBuf::from(std::env::var("OPENPENCIL_QA_SEARCH_OUTPUT").unwrap());
    std::fs::create_dir_all(output.parent().unwrap()).unwrap();
    jian_ops_schema::image_table::write_document_with_extension(
        &mut std::fs::File::create(&output).unwrap(),
        &state.doc,
        &jian_ops_schema::image_thumbs::capture_snapshot(),
        "editorMeta",
        &op_pen_loader::EditorMeta::from_state(&state),
    )
    .unwrap();
    std::fs::write(output.with_extension("receipt.json"),serde_json::to_vec_pretty(&json!({"additional_model_calls":0,"source_unchanged":true,"copy_geometry_fonts_and_images_preserved":true,"tint_changes":4,"geometry_flags":crate::geometry_validation::geometry_diagnostics(&state)})).unwrap()).unwrap();
}

#[test]
fn styled_visible_copy_is_searchable_but_dynamic_copy_is_not_snapshotted() {
    let mut value = fixture();
    value["children"][0]["children"][2]["children"][1]["children"][0]["content"] =
        json!([{ "text":"ALICE ","fontWeight":700},{"text":"LEE"}]);
    let state = apply(value);
    assert!(node(&state, "row-0")["bindings"]["visible"]
        .as_str()
        .unwrap()
        .contains("alice lee"));
    assert!(node(&state, "name-0")["content"].is_array());
    let mut value = fixture();
    value["children"][0]["children"][2]["children"][1]["children"][0]["bindings"] =
        json!({"content":"$app.live_name"});
    let state = apply(value);
    assert!(node(&state, "query")["bindings"].is_null());
}
