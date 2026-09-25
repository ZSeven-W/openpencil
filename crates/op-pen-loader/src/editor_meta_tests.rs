//! `editor_meta` tests — split out at the 800-line file cap.

use super::*;

#[test]
fn extracts_camel_and_legacy_snake_case_fields() {
    assert_eq!(
        extract_editor_meta(
            r#"{"version":"1","editorMeta":{"activePageIndex":3,"preserveAuthoredGeometry":true},"children":[]}"#,
        ),
        Some(EditorMeta {
            active_page_index: 3,
            preserve_authored_geometry: true,
            scenario: None,
            pinned_style_guide: None,
            share_recipe: None,
            imported_from: None,
        })
    );
    assert_eq!(
        extract_editor_meta(
            r#"{"editorMeta":{"active_page_index":2,"preserve_authored_geometry":true}}"#,
        ),
        Some(EditorMeta {
            active_page_index: 2,
            preserve_authored_geometry: true,
            scenario: None,
            pinned_style_guide: None,
            share_recipe: None,
            imported_from: None,
        })
    );
}

#[test]
fn absent_preserve_field_keeps_legacy_false_semantics() {
    assert_eq!(
        extract_editor_meta(r#"{"editorMeta":{"activePageIndex":7},"children":[]}"#),
        Some(EditorMeta {
            active_page_index: 7,
            preserve_authored_geometry: false,
            scenario: None,
            pinned_style_guide: None,
            share_recipe: None,
            imported_from: None,
        })
    );
}

#[test]
fn missing_preserve_field_recovers_figma_geometry_from_canonical_page_id() {
    assert_eq!(
        extract_editor_meta(
            r#"{"version":"1","pages":[{"id":"figma-page-12","name":"Imported","children":[]}],"children":[],"editorMeta":{"activePageIndex":0}}"#,
        ),
        Some(EditorMeta {
            active_page_index: 0,
            preserve_authored_geometry: true,
            scenario: None,
            pinned_style_guide: None,
            share_recipe: None,
            imported_from: None,
        })
    );
    assert_eq!(
        extract_editor_meta(
            r#"{"editorMeta":{"active_page_index":0},"pages":[{"id":"figma-page-0","name":"Imported","children":[]}],"children":[]}"#,
        ),
        Some(EditorMeta {
            active_page_index: 0,
            preserve_authored_geometry: true,
            scenario: None,
            pinned_style_guide: None,
            share_recipe: None,
            imported_from: None,
        })
    );
}

#[test]
fn explicit_false_overrides_figma_page_migration_for_both_spellings() {
    for meta in [
        r#"{"activePageIndex":0,"preserveAuthoredGeometry":false}"#,
        r#"{"active_page_index":0,"preserve_authored_geometry":false}"#,
    ] {
        let src = format!(
            r#"{{"version":"1","pages":[{{"id":"figma-page-0","name":"Imported","children":[]}}],"children":[],"editorMeta":{meta}}}"#
        );
        assert_eq!(
            extract_editor_meta(&src),
            Some(EditorMeta {
                active_page_index: 0,
                preserve_authored_geometry: false,
                scenario: None,
                pinned_style_guide: None,
                share_recipe: None,
                imported_from: None,
            })
        );
    }
}

#[test]
fn missing_metadata_does_not_migrate_even_with_a_figma_page_id() {
    let src = r#"{"version":"1","pages":[{"id":"figma-page-0","name":"Imported","children":[]}],"children":[]}"#;
    let document = jian_ops_schema::load_str(src).expect("fixture").value;
    let mut state = op_editor_core::EditorState::from_document(document);
    state.editor_ui.preserve_authored_geometry = true;

    let meta = extract_editor_meta(src);
    apply_editor_meta_or_legacy_fallback(&mut state, meta.clone());

    assert_eq!(meta, None);
    assert!(!state.editor_ui.preserve_authored_geometry);
}

#[test]
fn figma_like_but_noncanonical_page_id_does_not_trigger_migration() {
    assert_eq!(
        extract_editor_meta(
            r#"{"version":"1","pages":[{"id":"figma-page-preview","name":"Ordinary","children":[]}],"children":[],"editorMeta":{"activePageIndex":0}}"#,
        ),
        Some(EditorMeta {
            active_page_index: 0,
            preserve_authored_geometry: false,
            scenario: None,
            pinned_style_guide: None,
            share_recipe: None,
            imported_from: None,
        })
    );
}

#[test]
fn applying_metadata_clamps_page_and_restores_geometry_mode() {
    let document = jian_ops_schema::load_str(
        r#"{"version":"1","pages":[{"id":"p1","name":"One","children":[]},{"id":"p2","name":"Two","children":[]}],"children":[]}"#,
    )
    .expect("fixture")
    .value;
    let mut state = op_editor_core::EditorState::from_document(document);

    apply_editor_meta(
        &mut state,
        EditorMeta {
            active_page_index: 99,
            preserve_authored_geometry: true,
            scenario: None,
            pinned_style_guide: None,
            share_recipe: None,
            imported_from: None,
        },
    );

    assert_eq!(state.ui.active_page_index, 1);
    assert!(state.editor_ui.preserve_authored_geometry);
}

#[test]
fn absent_metadata_resets_preserve_and_opens_first_nonempty_page() {
    let document = jian_ops_schema::load_str(
        r#"{"version":"1","pages":[
          {"id":"p1","name":"Empty","children":[]},
          {"id":"p2","name":"Content","children":[
            {"type":"rectangle","id":"visible","width":10,"height":10}
          ]}
        ],"children":[]}"#,
    )
    .expect("fixture")
    .value;
    let mut state = op_editor_core::EditorState::from_document(document);
    state.editor_ui.preserve_authored_geometry = true;

    apply_editor_meta_or_legacy_fallback(&mut state, None);

    assert_eq!(state.ui.active_page_index, 1);
    assert!(!state.editor_ui.preserve_authored_geometry);
}

#[test]
fn escaped_strings_nested_values_and_duplicate_keys_are_bounded_correctly() {
    let src = r#"{
      "note":"quoted \"editorMeta\" and braces } ]",
      "plugin":{"editorMeta":{"preserveAuthoredGeometry":false}},
      "editorMeta":{"activePageIndex":1},
      "editor\u004deta":{"activePageIndex":4,"preserveAuthoredGeometry":true},
      "children":[]
    }"#;
    assert_eq!(
        extract_editor_meta(src),
        Some(EditorMeta {
            active_page_index: 4,
            preserve_authored_geometry: true,
            scenario: None,
            pinned_style_guide: None,
            share_recipe: None,
            imported_from: None,
        })
    );
}

#[test]
fn nested_absent_and_invalid_metadata_are_ignored() {
    assert_eq!(
        extract_editor_meta(
            r#"{"plugin":{"editorMeta":{"preserveAuthoredGeometry":true}},"children":[]}"#,
        ),
        None
    );
    assert_eq!(extract_editor_meta(r#"{"children":[]}"#), None);
    assert_eq!(extract_editor_meta(r#"{"editorMeta":"invalid"}"#), None);
}

#[test]
fn streaming_rewrite_preserves_legacy_document_bytes_outside_editor_meta() {
    let src = concat!(
        "{\n",
        "  \"version\":\"0.8.0\",\n",
        "  \"futureExtension\":{\"keep\":true},\n",
        "  \"editorMeta\":{\"activePageIndex\":1,\"oldField\":\"kept-nowhere\"},\n",
        "  \"children\":[]\n",
        "}\n"
    );
    let old_meta = r#"{"activePageIndex":1,"oldField":"kept-nowhere"}"#;
    let mut output = Vec::new();

    write_source_with_editor_meta(
        &mut output,
        src,
        EditorMeta {
            active_page_index: 7,
            preserve_authored_geometry: true,
            scenario: None,
            pinned_style_guide: None,
            share_recipe: None,
            imported_from: None,
        },
    )
    .expect("streaming metadata rewrite");

    let output = String::from_utf8(output).expect("UTF-8 output");
    let expected = src.replace(
        old_meta,
        r#"{"activePageIndex":7,"preserveAuthoredGeometry":true}"#,
    );
    assert_eq!(output, expected, "only the metadata value may change");
    assert_eq!(
        extract_editor_meta(&output),
        Some(EditorMeta {
            active_page_index: 7,
            preserve_authored_geometry: true,
            scenario: None,
            pinned_style_guide: None,
            share_recipe: None,
            imported_from: None,
        })
    );
}

#[test]
fn streaming_rewrite_appends_metadata_to_empty_and_nonempty_roots() {
    for (src, expected_prefix) in [
        (
            "{}\n",
            r#"{"editorMeta":{"activePageIndex":2,"preserveAuthoredGeometry":false}}"#,
        ),
        (
            "{\"version\":\"1\",\"children\":[]}\n",
            r#"{"version":"1","children":[],"editorMeta":{"activePageIndex":2,"preserveAuthoredGeometry":false}}"#,
        ),
    ] {
        let mut output = Vec::new();
        write_source_with_editor_meta(
            &mut output,
            src,
            EditorMeta {
                active_page_index: 2,
                preserve_authored_geometry: false,
                scenario: None,
                pinned_style_guide: None,
                share_recipe: None,
                imported_from: None,
            },
        )
        .expect("append metadata");
        let output = String::from_utf8(output).expect("UTF-8 output");
        assert_eq!(output, format!("{expected_prefix}\n"));
    }
}

#[test]
fn streaming_rewrite_rejects_non_object_sources() {
    let mut output = Vec::new();
    assert!(write_source_with_editor_meta(&mut output, "[]", EditorMeta::default()).is_err());
    assert!(output.is_empty());
}

#[test]
fn current_schema_rewrite_preserves_nested_unknown_fields() {
    let source = r#"{"version":"2.8","formatVersion":"1.0","children":[{"type":"rectangle","id":"r","futureNodeField":{"mustSurvive":true}}],"editorMeta":{"activePageIndex":9}}"#;
    let mut output = Vec::new();

    write_source_with_current_schema(
        &mut output,
        source,
        EditorMeta {
            active_page_index: 2,
            preserve_authored_geometry: true,
            scenario: None,
            pinned_style_guide: None,
            share_recipe: None,
            imported_from: None,
        },
    )
    .expect("current-schema rewrite");

    let parsed: serde_json::Value = serde_json::from_slice(&output).expect("valid JSON");
    assert_eq!(
        parsed["formatVersion"],
        jian_ops_schema::version::FORMAT_VERSION_CURRENT
    );
    assert_eq!(
        parsed["children"][0]["futureNodeField"]["mustSurvive"],
        true
    );
    assert_eq!(parsed["editorMeta"]["activePageIndex"], 2);
    assert_eq!(
        parsed["editorMeta"]["preserveAuthoredGeometry"],
        serde_json::Value::Bool(true)
    );
}

#[test]
fn pinned_style_guide_survives_a_save_load_round_trip() {
    let mut state = op_editor_core::EditorState::new();
    state.editor_ui.pinned_style_guide = Some("nordic-frost-light".to_string());

    let meta = EditorMeta::from_state(&state);
    assert_eq!(
        meta.pinned_style_guide.as_deref(),
        Some("nordic-frost-light")
    );

    let mut output = Vec::new();
    write_source_with_editor_meta(&mut output, "{\"version\":\"1\",\"children\":[]}\n", meta)
        .expect("append metadata");
    let output = String::from_utf8(output).expect("UTF-8 output");
    assert!(
        output.contains(r#""pinnedStyleGuide":"nordic-frost-light""#),
        "the pin must reach the file: {output}"
    );

    let read_back = extract_editor_meta(&output).expect("metadata round-trips");
    assert_eq!(
        read_back.pinned_style_guide.as_deref(),
        Some("nordic-frost-light")
    );

    let mut reopened = op_editor_core::EditorState::new();
    apply_editor_meta(&mut reopened, read_back);
    assert_eq!(
        reopened.editor_ui.pinned_style_guide.as_deref(),
        Some("nordic-frost-light")
    );
}

#[test]
fn unpinning_omits_the_field_entirely() {
    // An absent pin must not be written as `null`: old readers and the
    // byte-comparison rewrite tests both treat the key as optional.
    let state = op_editor_core::EditorState::new();
    let mut output = Vec::new();
    write_source_with_editor_meta(
        &mut output,
        "{\"version\":\"1\",\"children\":[]}\n",
        EditorMeta::from_state(&state),
    )
    .expect("append metadata");
    let output = String::from_utf8(output).expect("UTF-8 output");

    assert!(!output.contains("pinnedStyleGuide"), "{output}");
    assert_eq!(
        extract_editor_meta(&output)
            .expect("metadata present")
            .pinned_style_guide,
        None
    );
}

#[test]
fn malformed_pins_read_back_as_none_instead_of_failing_the_load() {
    for value in ["7", "null", "{}", "[]", r#""   ""#] {
        let src = format!(
            "{{\"version\":\"1\",\"children\":[],\"editorMeta\":{{\"activePageIndex\":0,\
             \"pinnedStyleGuide\":{value}}}}}"
        );
        let meta = extract_editor_meta(&src)
            .unwrap_or_else(|| panic!("a bad pin must not fail the load: {value}"));
        assert_eq!(meta.pinned_style_guide, None, "value {value}");
    }
}

#[test]
fn a_pin_the_registry_no_longer_carries_is_kept_on_disk() {
    // Only the generation path can judge a name against the registry, and
    // it falls back to automatic ranking. Dropping it here would silently
    // unpin a document whose guide was merely renamed in a later release.
    let src = r#"{"version":"1","children":[],"editorMeta":{"activePageIndex":0,"pinnedStyleGuide":"retired-guide-v0"}}"#;
    assert_eq!(
        extract_editor_meta(src)
            .expect("metadata present")
            .pinned_style_guide
            .as_deref(),
        Some("retired-guide-v0")
    );
}

#[test]
fn a_file_without_metadata_reopens_unpinned() {
    let src = r#"{"version":"1","pages":[{"id":"p","name":"Page","children":[]}],"children":[]}"#;
    let document = jian_ops_schema::load_str(src).expect("fixture").value;
    let mut state = op_editor_core::EditorState::from_document(document);
    state.editor_ui.pinned_style_guide = Some("stale-from-the-previous-file".to_string());

    apply_editor_meta_or_legacy_fallback(&mut state, extract_editor_meta(src));

    assert_eq!(state.editor_ui.pinned_style_guide, None);
}
