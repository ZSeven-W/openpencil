//! Retained .op save/reload and the shared reader/export/preview scene contract.
use op_editor_core::EditorState;
use serde_json::json;
#[test]
#[ignore = "requires OPENPENCIL_QA_SNAPSHOT_INPUT and OPENPENCIL_QA_SNAPSHOT_OUTPUT"]
fn retained_reader_exports_match_initial_preview_after_save_and_reload() {
    let input = std::path::PathBuf::from(std::env::var("OPENPENCIL_QA_SNAPSHOT_INPUT").unwrap());
    let out = std::path::PathBuf::from(std::env::var("OPENPENCIL_QA_SNAPSHOT_OUTPUT").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    let doc = op_pen_loader::payload::load_canonical(&std::fs::read_to_string(&input).unwrap())
        .unwrap()
        .value;
    let original = doc.clone();
    let mut state = EditorState::from_document(doc.clone());
    state.editor_ui.workspace.visible = true;
    let display = op_pen_loader::editor_state_to_active_page_layout_scene(&state);
    let export = op_pen_loader::editor_state_to_active_page_export_layout_scene(&state);
    let mut session = op_preview_core::PreviewSession::enter(
        &doc,
        (1600.0, 1200.0),
        &Default::default(),
        0,
        false,
        false,
        std::rc::Rc::new(jian_skia::SkiaMeasure::new()),
        0,
    )
    .unwrap();
    session.begin_lifecycle(0);
    session.pump(1000);
    let _ = session.preview_scene_for_test();
    session.pump(2000);
    let preview = session.preview_scene_for_test();
    for scene in [&display, &export, &preview] {
        assert_eq!(
            scene
                .active_page()
                .unwrap()
                .find("n1094")
                .unwrap()
                .text
                .as_deref(),
            Some("Local records 1–10 / 10")
        );
    }
    let encode = |scene: &op_editor_ui::layout_scene::LayoutScene| {
        crate::export::render_node_raster_bytes(
            scene,
            "n799",
            crate::export::RasterFormat::Png,
            1.0,
        )
        .unwrap()
    };
    let a = encode(&display);
    let b = encode(&export);
    let c = encode(&preview);
    std::fs::write(out.join("reader.png"), &a).unwrap();
    std::fs::write(out.join("export.png"), &b).unwrap();
    std::fs::write(out.join("preview.png"), &c).unwrap();
    assert!(a == b, "reader and default export differ");
    assert!(
        a == c,
        "the settled initial reader and preview must produce identical pixels"
    );
    let saved = out.join("roundtrip.op");
    crate::doc_io::save_to_path(&state, &saved).unwrap();
    let loaded =
        crate::doc_io::load_editor_state_with_report(&saved, op_i18n::Locale::ZhCn).unwrap();
    let mut reopened = loaded.state;
    reopened.editor_ui.workspace.visible = true;
    assert_eq!(reopened.doc, original);
    assert_eq!(
        encode(&op_pen_loader::editor_state_to_active_page_layout_scene(
            &reopened
        )),
        a
    );
    crate::export::export_node_svg(&export, "n799", &out.join("export.svg")).unwrap();
    crate::export_pdf::export_pdf(&export, &out.join("export.pdf")).unwrap();
    assert_eq!(state.doc, original);
    std::fs::write(out.join("receipt.json"),serde_json::to_vec_pretty(&json!({"reader_export_preview_pixel_identical":true,"canonical_data_preserved":true,"save_reload_pixels_identical":true,"svg_and_pdf_written":true,"interactive_preview_state_not_saved_as_author_data":true})).unwrap()).unwrap();
}
