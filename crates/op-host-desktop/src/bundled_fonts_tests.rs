//! Exercise the production bundled-font startup path without modifying the
//! process-global registry used by other rendering tests.

#[test]
fn desktop_bundled_fonts_reopen_saved_featured_template_without_missing_fonts() {
    const CHILD: &str = "OPENPENCIL_DESKTOP_BUNDLED_FONT_TEST_CHILD";
    const FULL_TEST: &str = concat!(
        module_path!(),
        "::desktop_bundled_fonts_reopen_saved_featured_template_without_missing_fonts"
    );
    let test = FULL_TEST.split_once("::").expect("qualified test name").1;
    if std::env::var(CHILD).as_deref() != Ok(test) {
        let output = std::process::Command::new(std::env::current_exe().expect("test executable"))
            .args(["--exact", test, "--nocapture"])
            .env(CHILD, test)
            .output()
            .expect("run isolated desktop font regression");
        assert!(
            output.status.success()
                && String::from_utf8_lossy(&output.stdout).contains("1 passed;"),
            "isolated desktop font regression failed:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        return;
    }

    super::register();
    let source = include_str!("../../op-editor-core/assets/scene_templates/coffee-order-app.op");
    let document = op_pen_loader::load_canonical(source)
        .expect("featured coffee template")
        .value;
    let state = op_editor_core::EditorState::from_document(document);
    let path = std::env::temp_dir().join(format!(
        "openpencil-desktop-font-reopen-{}.op",
        std::process::id()
    ));
    op_host_services::doc_io::save_to_path(&state, &path).expect("save featured work");
    let loaded = op_host_services::doc_io::load_editor_state(&path, op_i18n::Locale::ZhCn);
    std::fs::remove_file(&path).expect("remove saved test work");
    let mut host = op_host_native::WidgetHostNative::new();
    assert!(host
        .install_open_document(
            loaded.expect("reopen saved featured work").doc,
            None,
            Some("Coffee.op".into()),
        )
        .is_ok());
    let ui = &host.editor_state().editor_ui;
    for family in ["Inter", "Noto Sans SC", "Plus Jakarta Sans"] {
        assert!(
            ui.bundled_font_families.iter().any(|name| name == family),
            "desktop must ship the actual {family} family"
        );
    }
    assert!(ui.missing_fonts_prompt.is_none());
    assert!(!ui.missing_fonts_modal_open);
}
