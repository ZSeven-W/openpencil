//! Round-trip and tolerance tests for `editorMeta.shareRecipe`.

use crate::{
    apply_editor_meta, apply_editor_meta_or_legacy_fallback, extract_editor_meta,
    write_source_with_editor_meta, EditorMeta,
};
use op_editor_core::{
    EditorState, HomeDevice, HomeFamily, InfoKind, ShareRecipe, SlideRatio, SHARE_REDACTED,
};

fn recipe() -> ShareRecipe {
    ShareRecipe {
        brief: "做一份 5 页的咖啡品牌介绍".to_string(),
        family: HomeFamily::Presentation,
        device: HomeDevice::Desktop,
        ratio: SlideRatio::Classic43,
        info_kind: InfoKind::Flow,
        style_guide: Some("editorial-dark".to_string()),
    }
}

fn meta_with(recipe: Option<ShareRecipe>) -> EditorMeta {
    EditorMeta {
        active_page_index: 0,
        share_recipe: recipe,
        ..EditorMeta::default()
    }
}

#[test]
fn the_recipe_round_trips_through_a_saved_document() {
    let mut out = Vec::new();
    write_source_with_editor_meta(
        &mut out,
        r#"{"version":"1.0.0","children":[]}"#,
        meta_with(Some(recipe())),
    )
    .expect("write");
    let written = String::from_utf8(out).expect("utf8");
    assert!(written.contains(r#""shareRecipe":{"#), "{written}");
    assert!(written.contains(r#""family":"presentation""#), "{written}");
    assert!(written.contains(r#""ratio":"4:3""#), "{written}");
    assert!(
        written.contains(r#""styleGuide":"editorial-dark""#),
        "{written}"
    );
    let back = extract_editor_meta(&written).expect("meta");
    assert_eq!(back.share_recipe, Some(recipe()));
}

#[test]
fn a_document_without_a_recipe_writes_no_key() {
    let json = serde_json::to_string(&meta_with(None)).expect("json");
    assert!(!json.contains("shareRecipe"), "{json}");
}

#[test]
fn malformed_recipes_read_back_as_none_without_failing_the_load() {
    for bad in [
        r#"5"#,
        r#"null"#,
        r#""presentation""#,
        r#"{"family":"hologram","brief":"x"}"#,
        r#"{"brief":"no family"}"#,
    ] {
        let src = format!(r#"{{"editorMeta":{{"activePageIndex":1,"shareRecipe":{bad}}}}}"#);
        let meta = extract_editor_meta(&src).expect("the rest of the meta still loads");
        assert_eq!(meta.active_page_index, 1, "{bad}");
        assert_eq!(meta.share_recipe, None, "{bad}");
    }
}

#[test]
fn unknown_options_fall_back_to_defaults() {
    let src = r#"{"editorMeta":{"shareRecipe":{"family":"app","brief":"b","device":"watch","ratio":"21:9","infoKind":"map"}}}"#;
    let recipe = extract_editor_meta(src)
        .and_then(|meta| meta.share_recipe)
        .expect("a known family keeps the recipe");
    assert_eq!(recipe.family, HomeFamily::AppUi);
    assert_eq!(recipe.device, HomeDevice::Mobile);
    assert_eq!(recipe.ratio, SlideRatio::Wide169);
    assert_eq!(recipe.info_kind, InfoKind::Data);
    assert_eq!(recipe.style_guide, None);
}

#[test]
fn a_recipe_from_someone_elses_file_is_sanitized_on_read() {
    let src = r#"{"editorMeta":{"shareRecipe":{"family":"web","brief":"see /Users/eve/secret.txt apiKey=abc","styleGuide":"../../etc/passwd"}}}"#;
    let recipe = extract_editor_meta(src)
        .and_then(|meta| meta.share_recipe)
        .expect("recipe");
    assert!(!recipe.brief.contains("/Users/"), "{}", recipe.brief);
    assert!(!recipe.brief.contains("abc"), "{}", recipe.brief);
    assert!(recipe.brief.contains(SHARE_REDACTED), "{}", recipe.brief);
    assert_eq!(recipe.style_guide, None);
}

#[test]
fn applying_meta_installs_the_recipe_and_its_absence_clears_it() {
    let mut state = EditorState::new();
    apply_editor_meta(&mut state, meta_with(Some(recipe())));
    assert_eq!(state.editor_ui.home.recipe, Some(recipe()));
    // A saved state re-emits the recipe it was opened with.
    assert_eq!(EditorMeta::from_state(&state).share_recipe, Some(recipe()));

    apply_editor_meta_or_legacy_fallback(&mut state, None);
    assert_eq!(state.editor_ui.home.recipe, None);
}

#[test]
fn an_ordinary_save_of_a_live_run_writes_no_recipe() {
    let mut state = EditorState::new();
    state.editor_ui.open_workspace_for_generation(
        HomeFamily::Web,
        "my private brief",
        op_editor_core::TaskDraft::default(),
        0,
        1,
        None,
    );
    assert!(state.editor_ui.share_recipe().is_some());
    assert_eq!(EditorMeta::from_state(&state).share_recipe, None);
}
