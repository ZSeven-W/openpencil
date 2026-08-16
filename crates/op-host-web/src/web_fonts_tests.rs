use super::*;

fn state_with_text(family: &str) -> op_editor_core::EditorState {
    let doc = serde_json::from_value(serde_json::json!({
        "version": "0.8.0",
        "children": [{
            "type": "text",
            "id": "t1",
            "name": "t",
            "x": 0,
            "y": 0,
            "width": 10,
            "height": 10,
            "content": "hi",
            "fontFamily": family
        }]
    }))
    .expect("document");
    op_editor_core::EditorState::from_document(doc)
}

#[test]
fn supplied_font_with_another_family_records_a_mismatch_note() {
    let bytes = include_bytes!("../../op-host-desktop/assets/fonts/InstrumentSerif-Regular.ttf");
    let actual = crate::font_meta::parse_family(bytes).expect("fixture family");
    let mut host = crate::widget_host::WidgetHost::new();
    *host.editor_state_mut() = state_with_text("Katibeh");
    host.editor_state_mut().editor_ui.system_fonts_loaded = true;
    host.editor_state_mut().editor_ui.missing_fonts_prompt =
        Some(op_editor_core::missing_fonts::MissingFontsPrompt {
            entries: vec![op_editor_core::missing_fonts::MissingFontEntry {
                family: "Katibeh".to_string(),
                run_count: 1,
                mismatch_note: None,
                resolved: false,
            }],
        });

    host.note_missing_font_supplied(0, Some(&actual));

    let note = host
        .editor_state()
        .editor_ui
        .missing_fonts_prompt
        .as_ref()
        .unwrap()
        .entries[0]
        .mismatch_note
        .as_deref()
        .expect("mismatch note");
    assert!(note.contains("Instrument Serif"));
    assert!(note.contains("Katibeh"));
}

#[test]
fn detects_platform_emoji_font_families() {
    assert!(is_emoji_font_family("Apple Color Emoji"));
    assert!(is_emoji_font_family("Noto Color Emoji"));
    assert!(is_emoji_font_family("Segoe UI Emoji"));
    assert!(!is_emoji_font_family("PingFang SC"));
}

#[test]
fn detects_platform_cjk_fallback_font_families() {
    for family in [
        "PingFang SC",
        "PingFang TC",
        "Hiragino Sans",
        "Hiragino Sans GB",
        "Hiragino Kaku Gothic ProN",
        "Apple SD Gothic Neo",
        "Heiti SC",
        "STHeiti",
        "Yu Gothic",
        "Meiryo",
        "Noto Sans CJK SC",
        "Noto Sans JP",
        "Noto Sans KR",
        "Noto Sans TC",
        "Source Han Sans SC",
        "Microsoft YaHei",
        "Microsoft JhengHei",
        "Malgun Gothic",
        "AppleGothic",
        "Nanum Gothic",
        "SimHei",
    ] {
        assert!(
            is_cjk_fallback_font_family(family),
            "{family} should be treated as a browser system CJK fallback"
        );
    }
    assert!(!is_cjk_fallback_font_family("Roboto"));
}

#[test]
fn detects_platform_multilingual_text_fallback_font_families() {
    for family in [
        "Kohinoor Devanagari",
        "Devanagari Sangam MN",
        "ITFDevanagari",
        "MuktaMahee",
        "Noto Sans Devanagari",
        "Nirmala UI",
        "Apple SD Gothic Neo",
        "Nanum Gothic",
        "Noto Sans KR",
        "Noto Sans Cyrillic",
        "Arial Cyr",
        "SFGeorgian",
        "SFHebrew",
        "Arial Hebrew",
        "Geeza Pro",
        "Al Nile",
        "Thonburi",
        "Sukhumvit Set",
        "Noto Sans Thai",
        "Noto Sans Thai UI",
        "Arial Unicode MS",
        "Apple Color Emoji",
        "Segoe UI Emoji",
        "Noto Sans",
    ] {
        assert!(
            is_text_fallback_font_family(family),
            "{family} should be treated as a browser system text fallback"
        );
    }
    assert!(!is_text_fallback_font_family("Roboto"));
}

#[test]
fn system_font_query_runs_without_opening_font_picker() {
    assert!(should_query_system_fonts_state(false, false));
    assert!(!should_query_system_fonts_state(false, true));
    assert!(should_query_system_fonts_state(true, false));
}

#[test]
fn system_font_key_preserves_a_comma_inside_one_family_name() {
    assert_eq!(
        family_key("ACME, Display").as_deref(),
        Some("acme, display")
    );
}

#[test]
fn font_data_key_falls_back_to_the_windows_ui_alias_spelling() {
    // Issue #211: the document says `Microsoft YaHei`; the browser's
    // `queryLocalFonts` surfaced only the documented YaHei UI alias.
    // The byte lookup must resolve to that enumerated key.
    let enumerated = vec![
        "arial".to_string(),
        "microsoft yahei ui".to_string(),
        "segoe ui".to_string(),
        "yu gothic ui".to_string(),
    ];
    assert_eq!(
        alias_equivalent_font_data_key("Microsoft YaHei", "microsoft yahei", &enumerated)
            .as_deref(),
        Some("microsoft yahei ui")
    );
    // Authored UI spelling resolving against a plain-only enumeration.
    let plain_only = vec![
        "arial".to_string(),
        "microsoft yahei".to_string(),
        "segoe ui".to_string(),
    ];
    assert_eq!(
        alias_equivalent_font_data_key("Microsoft YaHei UI", "microsoft yahei ui", &plain_only)
            .as_deref(),
        Some("microsoft yahei")
    );
    // A family with no alias present stays unresolved.
    assert_eq!(
        alias_equivalent_font_data_key("Adventure Works Sans", "adventure works sans", &enumerated),
        None
    );
    // Distinct UI faces must not load the other face's bytes.
    assert_eq!(
        alias_equivalent_font_data_key("Segoe", "segoe", &enumerated),
        None
    );
    assert_eq!(
        alias_equivalent_font_data_key("Yu Gothic", "yu gothic", &enumerated),
        None
    );
}

#[test]
fn system_font_query_rejection_finishes_deferred_detection() {
    // Permission denial is a terminal empty system-font snapshot for this
    // session. This lets missing-font detection finish and exposes the import
    // fallback instead of leaving the prompt pending forever.
    assert!(should_mark_system_fonts_loaded_after_query_rejection());
}

#[test]
fn cross_page_ref_requests_the_resolved_master_text_family() {
    let doc = serde_json::from_value(serde_json::json!({
        "version": "0.8.0",
        "pages": [
            {
                "id": "components",
                "name": "Components",
                "children": [{
                    "type": "frame",
                    "id": "card",
                    "reusable": true,
                    "children": [{
                        "type": "text",
                        "id": "label",
                        "content": "Hello",
                        "fontFamily": "Master Sans"
                    }]
                }]
            },
            {
                "id": "canvas",
                "name": "Canvas",
                "children": [{"type": "ref", "id": "instance", "ref": "card"}]
            }
        ]
    }))
    .expect("document");
    let mut state = op_editor_core::EditorState::from_document(doc);
    state.ui.active_page_index = 1;

    assert_eq!(used_font_families_from_state(&state), vec!["Master Sans"]);
}

#[test]
fn active_ref_override_replaces_the_master_family_before_requesting() {
    let doc = serde_json::from_value(serde_json::json!({
        "version": "0.8.0",
        "pages": [
            {
                "id": "components",
                "name": "Components",
                "children": [{
                    "type": "frame",
                    "id": "card",
                    "reusable": true,
                    "children": [{
                        "type": "text",
                        "id": "label",
                        "content": "Hello",
                        "fontFamily": "Master Sans"
                    }]
                }]
            },
            {
                "id": "canvas",
                "name": "Canvas",
                "children": [{
                    "type": "ref",
                    "id": "instance",
                    "ref": "card",
                    "descendants": {"label": {"fontFamily": "Override Sans"}}
                }]
            }
        ]
    }))
    .expect("document");
    let mut state = op_editor_core::EditorState::from_document(doc);
    state.ui.active_page_index = 1;

    assert_eq!(used_font_families_from_state(&state), vec!["Override Sans"]);
}
