use super::*;
use op_editor_core::{
    table_filter_contract::{ROW_MARKER, TABLE_MARKER},
    table_pagination::{Spec, MARKER},
    EditorState,
};
fn fixture() -> EditorState {
    let button = |id: &str| json!({"type":"frame","id":id,"height":32,"width":32,"children":[{"type":"text","id":format!("{id}-label"),"content":"1"}]});
    let spec = Spec {
        page: "op_paging_page".into(),
        size: "op_paging_size".into(),
        total: "op_paging_total".into(),
        pages: "op_paging_pages".into(),
        start: "op_paging_start".into(),
        end: "op_paging_end".into(),
        filter_keys: vec!["op_filter_q".into()],
        sizes: vec![5],
        default_size: 5,
        items_id: "pager".into(),
        previous: button("prev"),
        next: button("next"),
        active: button("selected"),
        inactive: button("normal"),
    };
    let mut rows = vec![json!({"type":"frame","id":"header","height":30,"width":"fill_container"})];
    for i in 0..7 {
        rows.push(json!({"type":"frame","id":format!("row-{i}"),"explain":ROW_MARKER,"height":40,"width":"fill_container","bindings":{"visible":format!("contains('user {i}', $app.op_filter_q)")},"children":[{"type":"text","id":format!("name-{i}"),"content":format!("User {i}")}]}));
    }
    let mut state=EditorState::from_document(serde_json::from_value(json!({"version":"1.1","state":{
        "op_paging_page":{"type":"int","default":1},"op_paging_size":{"type":"string","default":"5"},"op_filter_q":{"type":"string","default":""},
        "op_paging_total":{"type":"int","default":7},"op_paging_pages":{"type":"int","default":2},"op_paging_start":{"type":"int","default":1},"op_paging_end":{"type":"int","default":5}},
        "children":[{"type":"frame","id":"root","width":600,"height":600,"layout":"vertical","children":[
            {"type":"frame","id":"table","width":"fill_container","height":"fit_content","layout":"vertical","explain":format!("{TABLE_MARKER}\n{MARKER}{}",serde_json::to_string(&spec).unwrap()),"children":rows},
            {"type":"text","id":"counter","content":"Old 84 users","bindings":{"content":"\"Local records \" + $app.op_paging_start + '–' + $app.op_paging_end + ' / ' + $app.op_paging_total"}},
            {"type":"text","id":"custom","content":"Authored","bindings":{"content":"'Custom ' + $app.op_paging_total"}},
            {"type":"frame","id":"pager","layout":"horizontal","children":[]}
        ]}]})).unwrap());
    state.editor_ui.workspace.visible = true;
    state
}
fn rows(scene: &jian_scene::layout_scene::LayoutScene) -> usize {
    scene
        .active_page()
        .unwrap()
        .find("table")
        .unwrap()
        .children
        .iter()
        .filter(|n| n.id.starts_with("row-"))
        .count()
}
#[test]
fn normal_display_and_exports_use_initial_page_without_changing_the_saved_dataset() {
    let mut state = fixture();
    let original = state.doc.clone();
    let scene = crate::editor_state_to_active_page_layout_scene(&state);
    assert_eq!(rows(&scene), 5);
    assert_eq!(
        scene
            .active_page()
            .unwrap()
            .find("counter")
            .unwrap()
            .text
            .as_deref(),
        Some("Local records 1–5 / 7")
    );
    assert_eq!(
        scene
            .active_page()
            .unwrap()
            .find("custom")
            .unwrap()
            .text
            .as_deref(),
        Some("Authored")
    );
    state.editor_ui.workspace.visible = false;
    let expert = crate::editor_state_to_active_page_layout_scene(&state);
    assert_eq!(rows(&expert), 7);
    let export = crate::editor_state_to_active_page_export_layout_scene(&state);
    assert_eq!(rows(&export), 5);
    assert_eq!(state.doc, original);
}
#[test]
fn normal_posture_is_a_scene_cache_input() {
    let mut state = fixture();
    let mut cache = crate::SceneBuildCache::new();
    assert_eq!(rows(&cache.maybe_rebuild(&state).unwrap()), 5);
    assert!(cache.maybe_rebuild(&state).is_none());
    state.editor_ui.workspace.visible = false;
    assert_eq!(rows(&cache.maybe_rebuild(&state).unwrap()), 7);
}

#[test]
fn a_selected_record_outside_page_one_remains_individually_exportable() {
    let mut state = fixture();
    state.apply(op_editor_core::EditorCommand::SetSelection {
        node_id: op_editor_core::NodeId::new("row-6"),
    });
    let scene = crate::editor_state_to_active_page_export_layout_scene(&state);
    assert!(scene.active_page().unwrap().find("row-6").is_some());
}
