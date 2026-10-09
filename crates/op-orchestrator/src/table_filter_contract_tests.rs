use super::*;
use op_editor_core::{EditorState, PenNodeExt};

fn text(id: &str, content: &str) -> Value {
    json!({"type":"text","id":id,"content":content,"fontSize":14,"width":100,"height":20})
}
fn fixture() -> Value {
    let mut rows = vec![
        json!({"type":"frame","id":"header","layout":"horizontal","children":[text("role-head","ROLE"),text("status-head","STATUS")]}),
    ];
    for (i, (role, status)) in [
        ("Admin", "Active"),
        ("Editor", "Suspended"),
        ("Viewer", "Invited"),
        ("Editor", "Active"),
    ]
    .iter()
    .enumerate()
    {
        rows.push(json!({"type":"frame","id":format!("row-{i}"),"height":40,"layout":"horizontal","children":[text(&format!("role-{i}"),role),text(&format!("status-{i}"),status)]}));
    }
    json!({"version":"1.1","children":[{"type":"frame","id":"root","width":600,"height":500,"layout":"vertical","children":[
        {"type":"select","id":"role","name":"Role filter","value":"all","options":[{"value":"all","label":"All roles"},{"value":"admin","label":"Admin"},{"value":"editor","label":"Editor"},{"value":"viewer","label":"Viewer"}]},
        {"type":"select","id":"status","name":"Status filter","value":"active","options":[{"value":"active","label":"Active"},{"value":"suspended","label":"Suspended"},{"value":"invited","label":"Invited"}]},
        {"type":"frame","id":"table","name":"Users Table","layout":"vertical","children":rows},
        {"type":"frame","id":"pagination","name":"Pagination footer","children":[text("count","Showing 1–4 of 84 users")]},
        text("other","Showing example instructions")
    ]}]})
}
fn apply(value: Value) -> EditorState {
    let mut state = EditorState::from_document(serde_json::from_value(value).unwrap());
    wire(&mut crate::loop_finalize::StateDocSink { state: &mut state });
    state
}
fn node(state: &EditorState, id: &str) -> Value {
    serde_json::to_value(
        op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(id)).unwrap(),
    )
    .unwrap()
}
#[test]
fn closed_enums_are_bound_without_rewriting_table_data() {
    let original = serde_json::to_value(
        serde_json::from_value::<jian_ops_schema::PenDocument>(fixture()).unwrap(),
    )
    .unwrap();
    let mut state = apply(original.clone());
    assert_eq!(node(&state, "status")["value"], "__op_all");
    assert_eq!(node(&state, "role")["value"], "all");
    assert_eq!(
        node(&state, "status")["options"][0]["label"],
        "All statuses"
    );
    assert_eq!(state.doc.state.as_ref().unwrap().len(), 2);
    assert!(node(&state, "row-0")["bindings"]["visible"]
        .as_str()
        .unwrap()
        .contains("&&"));
    for i in 0..4 {
        let row = node(&state, &format!("row-{i}"));
        assert_eq!(
            row["children"],
            original["children"][0]["children"][2]["children"][i + 1]["children"]
        );
    }
    assert_eq!(node(&state, "count")["content"], "Showing 1–4 of 84 users");
    assert!(node(&state, "count")["bindings"]["content"]
        .as_str()
        .unwrap()
        .contains("Rows on this page"));
    assert!(node(&state, "other")["bindings"].is_null());
    let settled = state.doc.clone();
    wire(&mut crate::loop_finalize::StateDocSink { state: &mut state });
    assert_eq!(state.doc, settled);
}
#[test]
fn authored_behavior_and_ambiguous_controls_are_preserved() {
    for field in ["bindings", "events", "state"] {
        let mut value = fixture();
        value["children"][0]["children"][0][field] = match field {
            "bindings" => json!({"bind:value":"$app.custom"}),
            "events" => json!({"onChange":[{"set":{"$app.custom":"$event.value"}}]}),
            _ => json!({"custom":{"type":"string","default":"all"}}),
        };
        let before = value["children"][0]["children"][0].clone();
        let state = apply(value);
        let after = node(&state, "role");
        assert_eq!(after[field], before[field]);
        assert!(after["explain"].is_null());
        assert!(!node(&state, "row-0")["bindings"]["visible"]
            .as_str()
            .unwrap()
            .contains(&key("role")));
    }
    let mut value = fixture();
    let mut second = value["children"][0]["children"][1].clone();
    second["id"] = json!("status-other");
    value["children"][0]["children"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let state = apply(value);
    assert!(node(&state, "status")["bindings"].is_null());
    assert!(node(&state, "status-other")["bindings"].is_null());
}
#[test]
fn unmatched_data_and_existing_row_bindings_are_never_overridden() {
    let mut value = fixture();
    value["children"][0]["children"][2]["children"][1]["children"][1]["content"] =
        json!("Pending review");
    let state = apply(value);
    assert!(node(&state, "status")["bindings"].is_null());
    assert_eq!(node(&state, "status-0")["content"], "Pending review");
    let mut value = fixture();
    value["children"][0]["children"][2]["children"][1]["bindings"] =
        json!({"visible":"$app.custom"});
    let state = apply(value.clone());
    assert_eq!(
        serde_json::to_value(state.doc).unwrap()["children"],
        serde_json::to_value(
            serde_json::from_value::<jian_ops_schema::PenDocument>(value).unwrap()
        )
        .unwrap()["children"]
    );
}
#[test]
fn multiple_tables_and_reserved_state_collisions_are_skipped() {
    let mut value = fixture();
    let mut second = value["children"][0]["children"][2].clone();
    second["id"] = json!("second-table");
    // Unique subtree ids are not needed for recognition of the ambiguity.
    value["children"][0]["children"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let state = apply(value);
    assert!(state.doc.state.is_none());
    let mut value = fixture();
    value["state"] = json!({key("status"):{"type":"string","default":"private"}});
    let state = apply(value);
    assert!(node(&state, "status")["bindings"].is_null());
    assert_eq!(
        state.doc.state.as_ref().unwrap()[&key("status")].default,
        Some(json!("private"))
    );
}
#[test]
#[ignore = "requires OPENPENCIL_QA_TABLE_INPUT and OPENPENCIL_QA_TABLE_OUTPUT"]
fn replay_retained_console_filters() {
    let source =
        std::fs::read_to_string(std::env::var("OPENPENCIL_QA_TABLE_INPUT").unwrap()).unwrap();
    let loaded = op_pen_loader::payload::load_canonical(&source).unwrap();
    let mut state = EditorState::from_document(loaded.value);
    let before = state.doc.clone();
    wire(&mut crate::loop_finalize::StateDocSink { state: &mut state });
    let mut ids = BTreeSet::new();
    op_editor_core::table_filter_contract::row_ids(
        &serde_json::to_value(&state.doc).unwrap(),
        &mut ids,
    );
    assert_eq!(ids.len(), 10);
    let before_state = EditorState::from_document(before.clone());
    for id in &ids {
        assert_eq!(
            node(&state, id)["children"],
            node(&before_state, id)["children"]
        );
    }
    for id in ["n1094", "n890", "n1096"] {
        let mut after = node(&state, id);
        let mut before = node(&before_state, id);
        after.as_object_mut().unwrap().remove("bindings");
        before.as_object_mut().unwrap().remove("bindings");
        assert_eq!(after, before);
    }
    assert_eq!(node(&state, "n887")["value"], "__op_all");
    assert_eq!(node(&state, "n884")["value"], "all");
    assert_eq!(node(&state, "n890")["value"], "7d");
    let settled = state.doc.clone();
    wire(&mut crate::loop_finalize::StateDocSink { state: &mut state });
    assert_eq!(state.doc, settled);
    let output = std::path::PathBuf::from(std::env::var("OPENPENCIL_QA_TABLE_OUTPUT").unwrap());
    std::fs::create_dir_all(output.parent().unwrap()).unwrap();
    jian_ops_schema::image_table::write_document_with_extension(
        &mut std::fs::File::create(&output).unwrap(),
        &state.doc,
        &jian_ops_schema::image_thumbs::capture_snapshot(),
        "editorMeta",
        &op_pen_loader::EditorMeta::from_state(&state),
    )
    .unwrap();
    std::fs::write(output.with_extension("receipt.json"),serde_json::to_vec_pretty(&json!({"rows":ids,"model_calls":0,"input_changed":false,
        "geometry_flags":crate::geometry_validation::geometry_diagnostics(&state),"root_size_preserved":before.children[0].height_px()==state.doc.children[0].height_px()})).unwrap()).unwrap();
}

#[test]
fn a_consistent_explicit_default_is_preserved() {
    let mut value = fixture();
    for row in &mut value["children"][0]["children"][2]["children"]
        .as_array_mut()
        .unwrap()[1..]
    {
        row["children"][1]["content"] = json!("Active");
    }
    let state = apply(value);
    assert_eq!(node(&state, "status")["value"], "active");
    assert_eq!(
        state.doc.state.as_ref().unwrap()[&key("status")].default,
        Some(json!("active"))
    );
}
