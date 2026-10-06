use super::{load_editor_state_from_source, save_to_path};
use op_editor_core::{EditorState, HomeFamily, Locale, TaskDraft};

#[test]
fn every_home_work_kind_survives_save_and_open_without_publishing_the_brief() {
    for family in HomeFamily::ALL {
        let doc = jian_ops_schema::load_str(r#"{"version":"1.0.0","children":[{"type":"frame","id":"work","width":1080,"height":1440,"children":[]}]}"#).unwrap().value;
        let mut state = EditorState::from_document(doc);
        state.editor_ui.open_workspace_for_generation(
            family,
            "PRIVATE BRIEF MUST NOT BE SAVED",
            TaskDraft::default(),
            0,
            1,
            None,
        );
        let path = std::env::temp_dir().join(format!(
            "openpencil-work-kind-{}-{}.op",
            std::process::id(),
            family.id()
        ));
        save_to_path(&state, &path).unwrap();
        let source = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert!(!source.contains("PRIVATE BRIEF"));
        let json: serde_json::Value = serde_json::from_str(&source).unwrap();
        assert_eq!(json["editorMeta"]["workFamily"], family.id());
        assert!(json["editorMeta"].get("shareRecipe").is_none());
        let mut reopened = load_editor_state_from_source(&source, Locale::ZhCn).unwrap();
        op_editor_core::work_identity::open_for_reading(&mut reopened, 2);
        assert_eq!(reopened.editor_ui.workspace.family, family);
        assert!(reopened.editor_ui.workspace.family_known);
        assert!(reopened.editor_ui.workspace.brief.is_empty());
        assert_eq!(reopened.doc, state.doc);
    }
}

#[test]
fn legacy_unknown_and_malformed_work_tags_remain_viewable_and_unclassified() {
    for tag in ["null", "7", "\"future-kind\"", "{\"family\":\"poster\"}"] {
        let source =
            format!(r#"{{"version":"1.0.0","children":[],"editorMeta":{{"workFamily":{tag}}}}}"#);
        let mut state = load_editor_state_from_source(&source, Locale::ZhCn).unwrap();
        op_editor_core::work_identity::open_for_reading(&mut state, 1);
        assert!(state.editor_ui.home.work_family.is_none());
        assert!(!state.editor_ui.workspace.family_known);
    }
}
