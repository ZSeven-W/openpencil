//! Browser-IO regression tests (task: web IME commit, paste-text
//! routing, canonical Save serialization, Figma-node paste). Gated on
//! `codegen` because the serialize / ingest helpers live behind that
//! feature; everything here exercises the pure host / helper layer —
//! no DOM.

use super::WidgetHost;

#[test]
fn ime_commit_lands_in_focused_chat_input() {
    let mut host = WidgetHost::new();
    host.editor_state.chat.focused = true;
    let evt = crate::event::ime::composition_end("你好".to_string());
    assert!(host.apply_ime(&evt));
    assert_eq!(host.editor_state.chat.input, "你好");
}

#[test]
fn ime_preedit_updates_do_not_mutate_the_input() {
    let mut host = WidgetHost::new();
    host.editor_state.chat.focused = true;
    let start = crate::event::ime::composition_start();
    let update = crate::event::ime::composition_update("ni".to_string(), None);
    assert!(!host.apply_ime(&start));
    assert!(!host.apply_ime(&update));
    assert!(host.editor_state.chat.input.is_empty());
}

#[test]
fn ime_commit_without_any_focused_input_is_a_no_op() {
    let mut host = WidgetHost::new();
    let evt = crate::event::ime::composition_end("漢字".to_string());
    assert!(!host.apply_ime(&evt));
    assert!(host.editor_state.chat.input.is_empty());
}

#[test]
fn paste_text_routes_to_focused_rename() {
    let mut host = WidgetHost::new();
    // The starter document selects the blank starter frame; begin an
    // inline rename on it like a layer-row double-click would.
    let id = host.editor_state.selection.anchor.clone();
    assert!(host.editor_state.start_rename_layer(id));
    // Select-all so the paste replaces the seeded name deterministically.
    host.editor_state
        .ui
        .layer_rename
        .as_mut()
        .expect("rename active")
        .select_all = true;
    assert!(host.apply_paste_text("Hero Section"));
    assert_eq!(
        host.editor_state
            .ui
            .layer_rename
            .as_ref()
            .expect("rename still active")
            .draft,
        "Hero Section"
    );
    // The paste went into the rename draft, NOT the chat input.
    assert!(host.editor_state.chat.input.is_empty());
}

#[test]
fn paste_text_routes_to_focused_chat_input() {
    let mut host = WidgetHost::new();
    host.editor_state.chat.focused = true;
    assert!(host.apply_paste_text("hello web"));
    assert_eq!(host.editor_state.chat.input, "hello web");
}

#[test]
fn paste_text_without_any_focused_input_is_a_no_op() {
    let mut host = WidgetHost::new();
    assert!(!host.apply_paste_text("nowhere to go"));
}

#[test]
fn save_serializes_the_canonical_document_json() {
    let host = WidgetHost::new();
    let json =
        crate::file_actions::serialize_document(host.editor_state()).expect("serialize succeeds");
    // The Save payload is the canonical document JSON — exactly what
    // the desktop's `persistence::save_to_path` writes (pretty-printed
    // `state.doc`), with a string `version` marker.
    let expected =
        serde_json::to_string_pretty(&host.editor_state().doc).expect("serde serializes");
    assert_eq!(json, expected);
    assert!(json.contains("\"version\""));
    // And it round-trips through the same canonical loader the web
    // Open path uses.
    let ingested = crate::file_actions::ingest_op_source(&json, host.editor_state())
        .expect("canonical round-trip parses");
    assert_eq!(
        ingested.state.doc.children.len(),
        host.editor_state().doc.children.len()
    );
}

#[test]
fn ingest_preserves_app_preferences_from_the_previous_state() {
    use op_editor_core::editor_ui_state::ThemeMode;
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.theme_mode = ThemeMode::Light;
    host.editor_state.editor_ui.locale = op_editor_core::Locale::ZhCn;
    let json =
        crate::file_actions::serialize_document(host.editor_state()).expect("serialize succeeds");
    let ingested = crate::file_actions::ingest_op_source(&json, host.editor_state())
        .expect("canonical parse succeeds");
    assert_eq!(ingested.state.editor_ui.theme_mode, ThemeMode::Light);
    assert_eq!(
        ingested.state.editor_ui.locale,
        op_editor_core::Locale::ZhCn
    );
}

#[test]
fn save_file_name_falls_back_to_untitled() {
    let mut host = WidgetHost::new();
    assert_eq!(
        crate::file_actions::save_file_name(host.editor_state()),
        "untitled.op"
    );
    host.editor_state.editor_ui.file_name_display = Some("login.op".to_string());
    assert_eq!(
        crate::file_actions::save_file_name(host.editor_state()),
        "login.op"
    );
}

#[test]
fn drop_kind_classifies_by_extension_case_insensitively() {
    use crate::file_actions::{drop_kind, DropKind};
    assert_eq!(drop_kind("design.op"), DropKind::Document);
    assert_eq!(drop_kind("Design.PEN"), DropKind::Document);
    assert_eq!(drop_kind("Mockup.FIG"), DropKind::Figma);
    assert_eq!(drop_kind("icon.svg"), DropKind::Svg);
    assert_eq!(drop_kind("photo.JPEG"), DropKind::Image);
    assert_eq!(drop_kind("notes.txt"), DropKind::Unsupported);
}

#[test]
fn paste_figma_nodes_inserts_clones_and_selects_them() {
    let mut host = WidgetHost::new();
    let before = host.editor_state.doc.children.len();
    // Donor nodes — the starter frame from a second state stands in
    // for a decoded Figma clipboard payload (same `PenNode` type).
    let donor = op_editor_core::EditorState::starter();
    let nodes = donor.doc.children.clone();
    assert!(!nodes.is_empty(), "starter doc has a frame to donate");
    assert!(host.paste_figma_nodes(nodes, 960.0, 640.0));
    assert_eq!(host.editor_state.doc.children.len(), before + 1);
    assert!(host.editor_state.selection.anchor.is_real());
}

#[test]
fn paste_figma_nodes_with_no_nodes_is_a_no_op() {
    let mut host = WidgetHost::new();
    let before = host.editor_state.doc.children.len();
    assert!(!host.paste_figma_nodes(Vec::new(), 960.0, 640.0));
    assert_eq!(host.editor_state.doc.children.len(), before);
}

#[test]
fn install_ingested_state_preserves_live_chrome_and_clears_progress() {
    use op_editor_core::editor_ui_state::ThemeMode;
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.theme_mode = ThemeMode::Light;
    host.editor_state.editor_ui.figma_import_in_progress = true;
    host.editor_state.editor_ui.file_name_display = Some("old.op".to_string());

    let mut incoming = op_editor_core::EditorState::starter();
    incoming.editor_ui.preserve_authored_geometry = true;
    host.install_ingested_state(incoming);

    let ui = &host.editor_state.editor_ui;
    assert_eq!(ui.theme_mode, ThemeMode::Light, "live theme survives");
    assert!(!ui.figma_import_in_progress, "progress overlay clears");
    assert!(ui.preserve_authored_geometry, "import geometry flag wins");
    assert_eq!(
        ui.file_name_display, None,
        "imported documents start untitled"
    );
}

#[test]
fn export_target_maps_formats_onto_browser_encoders() {
    use crate::file_actions::export_target;
    use op_editor_core::editor_ui_state::ExportFormat;
    assert_eq!(export_target(ExportFormat::Png), ("image/png", "png", true));
    assert_eq!(
        export_target(ExportFormat::Jpeg),
        ("image/jpeg", "jpg", true)
    );
    assert_eq!(
        export_target(ExportFormat::Webp),
        ("image/webp", "webp", true)
    );
    // SVG / PDF degrade to a PNG raster on web (no encoder yet).
    assert_eq!(
        export_target(ExportFormat::Svg),
        ("image/png", "png", false)
    );
    assert_eq!(
        export_target(ExportFormat::Pdf),
        ("image/png", "png", false)
    );
}
