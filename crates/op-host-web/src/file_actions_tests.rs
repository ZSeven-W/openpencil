//! Tests for the web host's file actions — open / save round-trips,
//! export request bodies and the daemon response parsers.
//!
//! Split out of `file_actions.rs` at the repo's 800-line cap. Pure code
//! motion: the module is still `file_actions::tests`, so every test name
//! and every `use super::*` reach is unchanged.

use super::*;
use op_editor_core::PenNodeExt;

#[test]
fn metadata_free_open_uses_first_nonempty_page_and_legacy_layout_mode() {
    let previous = EditorState::new();
    let source = r#"{
      "version":"1.0.0",
      "children":[],
      "pages":[
        {"id":"empty","name":"Empty","children":[]},
        {"id":"content","name":"Content","children":[
          {"type":"rectangle","id":"visible","x":0,"y":0,"width":10,"height":10}
        ]}
      ]
    }"#;

    let ingested = ingest_op_source(source, &previous).expect("legacy document loads");

    assert_eq!(ingested.state.ui.active_page_index, 1);
    assert!(!ingested.state.editor_ui.preserve_authored_geometry);
}

#[test]
fn figma_worker_canonical_source_installs_all_pages_eagerly() {
    let source = r#"{"version":"1.0","pages":[{"id":"p1","name":"One","children":[{"type":"rectangle","id":"a"}]},{"id":"p2","name":"Two","children":[{"type":"rectangle","id":"b"}]}],"children":[]}"#;
    let ingested = ingest_figma_temp_source(source, r#"["worker warning"]"#)
        .expect("worker canonical source loads");
    let pages = ingested.state.doc.pages.as_ref().expect("pages");
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0].children.len(), 1);
    assert_eq!(pages[1].children.len(), 1);
    assert!(ingested.state.editor_ui.preserve_authored_geometry);
    assert_eq!(ingested.warnings, ["worker warning"]);
}

#[test]
fn app_preferences_preserve_runtime_font_availability() {
    let mut previous = EditorState::new();
    previous.editor_ui.font_import_supported = true;
    previous.editor_ui.system_fonts_loaded = true;
    previous.editor_ui.system_font_families = std::sync::Arc::new(vec!["PingFang SC".into()]);
    previous.editor_ui.bundled_font_families = std::sync::Arc::new(vec!["Inter".into()]);
    previous.editor_ui.imported_font_families = std::sync::Arc::new(vec!["Brand Sans".into()]);
    let mut next = EditorState::new();

    preserve_app_preferences(&previous, &mut next);

    assert!(next.editor_ui.font_import_supported);
    assert!(next.editor_ui.system_fonts_loaded);
    assert_eq!(&*next.editor_ui.system_font_families, &["PingFang SC"]);
    assert_eq!(&*next.editor_ui.bundled_font_families, &["Inter"]);
    assert_eq!(&*next.editor_ui.imported_font_families, &["Brand Sans"]);
}

#[test]
fn app_preferences_preserve_transient_embedding_theme_and_locale_separately() {
    let mut previous = EditorState::new();
    previous.editor_ui.theme_mode = op_editor_core::ThemeMode::Light;
    previous
        .editor_ui
        .set_host_theme_override(Some(op_editor_core::ThemeMode::Dark));
    previous.editor_ui.locale = op_editor_core::Locale::ZhCn;
    previous
        .editor_ui
        .set_host_locale_override(Some(op_editor_core::Locale::EnUs));
    let mut next = EditorState::new();

    preserve_app_preferences(&previous, &mut next);

    assert_eq!(next.editor_ui.theme_mode, op_editor_core::ThemeMode::Light);
    assert_eq!(
        next.editor_ui.effective_theme_mode(),
        op_editor_core::ThemeMode::Dark
    );
    assert_eq!(next.editor_ui.locale, op_editor_core::Locale::ZhCn);
    assert_eq!(
        next.editor_ui.effective_locale(),
        op_editor_core::Locale::EnUs
    );
}

#[test]
fn attachment_media_type_matches_desktop_image_extensions() {
    assert_eq!(attachment_media_type_for_name("a.png"), "image/png");
    assert_eq!(attachment_media_type_for_name("a.JPG"), "image/jpeg");
    assert_eq!(attachment_media_type_for_name("a.jpeg"), "image/jpeg");
    assert_eq!(attachment_media_type_for_name("a.gif"), "image/gif");
    assert_eq!(attachment_media_type_for_name("a.webp"), "image/webp");
    assert_eq!(attachment_media_type_for_name("a.svg"), "image/svg+xml");
    assert_eq!(
        attachment_media_type_for_name("notes.txt"),
        "application/octet-stream"
    );
}

#[test]
fn attachment_file_name_strips_path_separators() {
    assert_eq!(attachment_file_name("../a.png"), ".._a.png");
    assert_eq!(attachment_file_name("folder\\a.png"), "folder_a.png");
    assert_eq!(attachment_file_name(""), "attachment");
}

#[test]
fn export_kit_document_builds_download_name_and_json() {
    let src = r#"{"version":"1.0.0","name":"My Kit!","children":[{"type":"frame","id":"button","name":"Primary Button","reusable":true,"x":0,"y":0,"width":120,"height":40,"children":[]}]}"#;
    let doc = op_pen_loader::load_canonical(src)
        .expect("canonical doc")
        .value;
    let state = EditorState::from_document(doc);

    let export = export_kit_document(&state)
        .expect("export encodes")
        .expect("document has reusable components");

    assert_eq!(export.file_name, "My Kit.op");
    let parsed: jian_ops_schema::PenDocument =
        serde_json::from_str(&export.json).expect("kit json");
    assert_eq!(parsed.name.as_deref(), Some("My Kit!"));
    assert_eq!(parsed.children.len(), 1);
    assert_eq!(parsed.children[0].base().id, "button");
}

#[test]
fn save_request_body_embeds_document_and_active_page_index() {
    let src = r#"{"version":"1.0.0","children":[],"pages":[{"id":"p1","name":"One","children":[]},{"id":"p2","name":"Two","children":[{"type":"rectangle","id":"save-node","name":"Save Node","x":0,"y":0,"width":80,"height":40}]}]}"#;
    let doc = op_pen_loader::load_canonical(src)
        .expect("canonical doc")
        .value;
    let mut state = EditorState::from_document(doc);
    assert!(state.set_active_page(1));

    let body = save_request_body(&state).expect("request body");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json body");

    assert_eq!(
        parsed["document"]["pages"][1]["children"][0]["id"],
        "save-node"
    );
    assert_eq!(parsed["activePageIndex"], 1);
}

#[test]
fn parse_save_response_accepts_daemon_success() {
    let saved = parse_save_response(r#"{"ok":true,"version":3,"fileName":"design.op"}"#)
        .expect("save response");

    assert_eq!(saved.file_name, "design.op");
    assert_eq!(saved.version, Some(3));
}

#[test]
fn parse_save_response_surfaces_daemon_error() {
    let err = parse_save_response(r#"{"ok":false,"error":"No file path"}"#)
        .expect_err("daemon error should fail");

    assert_eq!(err.to_string(), "No file path");
}

#[test]
fn export_pdf_request_body_embeds_current_document() {
    let src = r#"{"version":"1.0.0","children":[],"pages":[{"id":"p1","name":"One","children":[]},{"id":"p2","name":"Two","children":[{"type":"rectangle","id":"pdf-node","name":"PDF Node","x":0,"y":0,"width":80,"height":40}]}]}"#;
    let doc = op_pen_loader::load_canonical(src)
        .expect("canonical doc")
        .value;
    let mut state = EditorState::from_document(doc);
    assert!(state.set_active_page(1));
    state.editor_ui.preserve_authored_geometry = true;

    let body = export_pdf_request_body(&state, None).expect("request body");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json body");

    assert_eq!(
        parsed["document"]["pages"][1]["children"][0]["id"],
        "pdf-node"
    );
    assert_eq!(
        parsed["document"]["pages"][1]["children"][0]["name"],
        "PDF Node"
    );
    assert_eq!(parsed["activePageIndex"], 1);
    assert_eq!(parsed["document"]["editorMeta"]["activePageIndex"], 1);
    assert_eq!(
        parsed["document"]["editorMeta"]["preserveAuthoredGeometry"],
        true
    );
    assert!(
        parsed.get("boards").is_none(),
        "a whole-page export must send NO boards field — an empty array \
         would read daemon-side as 'narrowed to nothing'"
    );
}

/// The slides rail's "Export selected slides" row travels as data: the
/// daemon rebuilds its `EditorState` from the posted document, and that
/// round-trip carries no selection, so the ids are the only record of it
/// that reaches the exporter.
#[test]
fn export_pdf_request_body_carries_a_board_filter_when_one_is_given() {
    let src = r#"{"version":"1.0.0","children":[{"type":"frame","id":"s1","name":"Cover","x":0,"y":0,"width":320,"height":180},{"type":"frame","id":"s2","name":"Agenda","x":400,"y":0,"width":320,"height":180}]}"#;
    let doc = op_pen_loader::load_canonical(src)
        .expect("canonical doc")
        .value;
    let state = EditorState::from_document(doc);

    let boards = vec!["s2".to_string()];
    let body = export_pdf_request_body(&state, Some(&boards)).expect("request body");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json body");

    assert_eq!(parsed["boards"], serde_json::json!(["s2"]));

    // An empty selection still sends the field — "narrowed to nothing"
    // is a different request from "not narrowed", and only the field
    // being present tells the two apart.
    let body = export_pdf_request_body(&state, Some(&[])).expect("request body");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(parsed["boards"], serde_json::json!([]));
}

#[test]
fn parse_pdf_download_response_decodes_daemon_payload() {
    let response = serde_json::json!({
        "ok": true,
        "fileName": "openpencil-export.pdf",
        "mime": "application/pdf",
        "dataBase64": base64::engine::general_purpose::STANDARD.encode(b"%PDF-test%%EOF"),
    })
    .to_string();

    let download = parse_pdf_download_response(&response).expect("pdf response");

    assert_eq!(download.file_name, "openpencil-export.pdf");
    assert_eq!(download.mime, "application/pdf");
    assert_eq!(download.bytes, b"%PDF-test%%EOF");
}

#[test]
fn parse_pdf_download_response_surfaces_daemon_error() {
    let err = parse_pdf_download_response(r#"{"ok":false,"error":"nothing to export"}"#)
        .expect_err("daemon error should fail");

    assert_eq!(err.to_string(), "nothing to export");
}

#[test]
fn export_raster_request_body_embeds_format_scale_document_and_single_selection() {
    let src = r#"{"version":"1.0.0","children":[{"type":"rectangle","id":"raster-node","name":"Raster Node","x":0,"y":0,"width":80,"height":40}]}"#;
    let doc = op_pen_loader::load_canonical(src)
        .expect("canonical doc")
        .value;
    let mut state = EditorState::from_document(doc);
    state.editor_ui.export_format = ExportFormat::Webp;
    state.editor_ui.export_scale = 3.0;
    state.editor_ui.preserve_authored_geometry = true;
    state.set_single_selection(op_editor_core::NodeId::new("raster-node"));

    let body = export_raster_request_body(&state).expect("request body");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json body");

    assert_eq!(parsed["format"], "webp");
    assert_eq!(parsed["scale"], 3.0);
    assert_eq!(parsed["selectedNodeId"], "raster-node");
    assert_eq!(parsed["activePageIndex"], 0);
    assert_eq!(parsed["document"]["children"][0]["id"], "raster-node");
    assert_eq!(
        parsed["document"]["editorMeta"]["preserveAuthoredGeometry"],
        true
    );
}

#[test]
fn parse_raster_download_response_decodes_daemon_payload() {
    let response = serde_json::json!({
        "ok": true,
        "fileName": "openpencil-export.png",
        "mime": "image/png",
        "dataBase64": base64::engine::general_purpose::STANDARD.encode(b"\x89PNG\r\n\x1a\n"),
    })
    .to_string();

    let download = parse_raster_download_response(&response).expect("raster response");

    // The daemon's generic `fileName` is present in the payload and
    // deliberately dropped: this host names the download itself, from
    // the live document and selection.
    assert_eq!(download.mime, "image/png");
    assert_eq!(download.bytes, b"\x89PNG\r\n\x1a\n");
}

#[test]
fn parse_raster_download_response_surfaces_daemon_error() {
    let err = parse_raster_download_response(r#"{"ok":false,"error":"nothing to export"}"#)
        .expect_err("daemon error should fail");

    assert_eq!(err.to_string(), "nothing to export");
}

#[test]
fn import_kit_source_extracts_components_with_supplied_id() {
    let src = r#"{"version":"1.0.0","name":"Imported System","children":[{"type":"frame","id":"card","name":"Profile Card","reusable":true,"x":0,"y":0,"width":240,"height":120,"children":[]}]}"#;

    let kit = import_kit_source(src, "web-kit-1".to_string())
        .expect("import parses")
        .expect("source has reusable components");

    assert_eq!(kit.id, "web-kit-1");
    assert_eq!(kit.name, "Imported System");
    assert_eq!(kit.components.len(), 1);
    assert_eq!(kit.components[0].id, "card");
}

#[test]
fn drop_kind_recognizes_html_and_zip() {
    assert!(matches!(drop_kind("page.html"), DropKind::Html));
    assert!(matches!(drop_kind("PAGE.HTM"), DropKind::Html));
    assert!(matches!(drop_kind("site.CSS"), DropKind::HtmlResource));
    assert!(matches!(drop_kind("ui.WOFF2"), DropKind::HtmlResource));
    assert!(matches!(drop_kind("brand.otf"), DropKind::HtmlResource));
    assert!(matches!(drop_kind("app.mjs"), DropKind::HtmlResource));
    assert!(matches!(
        drop_kind("manifest.webmanifest"),
        DropKind::HtmlResource
    ));
    assert!(matches!(drop_kind("favicon.ico"), DropKind::Image));
    assert!(matches!(drop_kind("photo.avif"), DropKind::Image));
    assert!(matches!(drop_kind("saved-page.ZIP"), DropKind::Zip));
    assert!(matches!(drop_kind("a.svg"), DropKind::Svg));
}

#[test]
fn drop_batch_plan_groups_html_with_explicit_resources() {
    assert_eq!(
        drop_batch_plan(&[
            DropKind::Html,
            DropKind::HtmlResource,
            DropKind::Image,
            DropKind::Svg,
        ]),
        DropBatchPlan::HtmlProject
    );
    assert_eq!(
        drop_batch_plan(&[DropKind::Html, DropKind::Html]),
        DropBatchPlan::HtmlProject
    );
}

#[test]
fn drop_batch_plan_rejects_html_document_figma_and_unknown_mixes() {
    for conflict in [DropKind::Document, DropKind::Figma, DropKind::Unsupported] {
        assert_eq!(
            drop_batch_plan(&[DropKind::Html, conflict]),
            DropBatchPlan::InvalidHtmlMix
        );
    }
}

#[test]
fn drop_batch_plan_rejects_zip_mixes_and_keeps_other_drops_individual() {
    assert_eq!(drop_batch_plan(&[DropKind::Zip]), DropBatchPlan::HtmlZip);
    assert_eq!(
        drop_batch_plan(&[DropKind::Zip, DropKind::Html]),
        DropBatchPlan::InvalidZipMix
    );
    assert_eq!(
        drop_batch_plan(&[DropKind::Image, DropKind::Svg]),
        DropBatchPlan::Individual
    );
    assert_eq!(
        drop_batch_plan(&[DropKind::HtmlResource]),
        DropBatchPlan::Individual
    );
}

#[test]
fn ingest_html_project_resolves_relative_stylesheets_and_images() {
    let files = vec![
        op_html::HtmlProjectFile {
            relative_path: "pages/index.html".into(),
            bytes: br#"<link rel="stylesheet" href="../assets/site.css">
                <div class="hero"></div>"#
                .to_vec(),
        },
        op_html::HtmlProjectFile {
            relative_path: "assets/site.css".into(),
            bytes: br#".hero { width: 40px; height: 30px;
                background-image: url('./hero icon.png?v=1'); }"#
                .to_vec(),
        },
        op_html::HtmlProjectFile {
            relative_path: "assets/hero icon.png".into(),
            bytes: vec![0x89, 0x50, 0x4e, 0x47, 1, 2, 3],
        },
    ];

    let ingested = ingest_html_project(&files).expect("saved-page project imports");
    let json = serde_json::to_string(&ingested.state.doc).expect("document serializes");

    assert!(json.contains("data:image/png;base64,"));
    assert!(!ingested
        .warnings
        .iter()
        .any(|warning| warning.contains("external stylesheet skipped")));
}

#[test]
fn ingest_html_project_prefers_index_and_rejects_missing_html() {
    let files = vec![
        op_html::HtmlProjectFile {
            relative_path: "other.html".into(),
            bytes: b"<h1>Other</h1>".to_vec(),
        },
        op_html::HtmlProjectFile {
            relative_path: "INDEX.HTML".into(),
            bytes: b"<h1>Index chosen</h1>".to_vec(),
        },
    ];
    let ingested = ingest_html_project(&files).expect("index candidate imports");
    let json = serde_json::to_string(&ingested.state.doc).expect("document serializes");
    assert!(json.contains("Index chosen"));
    assert!(!json.contains("Other"));

    assert!(ingest_html_project(&[op_html::HtmlProjectFile {
        relative_path: "style.css".into(),
        bytes: b"body {}".to_vec(),
    }])
    .is_err());
}

/// Web twin of the desktop's `export_dialog_default_name_joins_document_and_selected_node`:
/// the browser download and the native save dialog read the same
/// shared derivation, so an export of the same document and selection
/// lands under the same name on either host.
#[test]
fn export_download_file_name_joins_document_and_selected_node() {
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0.0","children":[
            {"type":"frame","id":"f1","name":"星图","width":100,"height":100},
            {"type":"frame","id":"f2","name":"侧栏","width":100,"height":100}
        ]}"#,
    )
    .expect("fixture JSON parses")
    .value;
    let mut state = EditorState::from_document(doc);
    state.editor_ui.file_name_display = Some("0808-k3-2.op".to_string());

    assert_eq!(export_download_file_name(&state), "0808-k3-2.png");

    state.selection.set = vec![op_editor_core::NodeId::new("f1")];
    state.selection.anchor = op_editor_core::NodeId::new("f1");
    assert_eq!(export_download_file_name(&state), "0808-k3-2-星图.png");

    // A name the filesystem cannot carry is sanitized before it
    // reaches the download attribute.
    state.editor_ui.file_name_display = Some("re:port/v2.op".to_string());
    assert_eq!(export_download_file_name(&state), "re-port-v2-星图.png");
}
