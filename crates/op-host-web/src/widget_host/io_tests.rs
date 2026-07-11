//! Browser-IO regression tests (task: web IME commit, paste-text
//! routing, canonical Save serialization, Figma-node paste). Gated on
//! `codegen` because the serialize / ingest helpers live behind that
//! feature; everything here exercises the pure host / helper layer —
//! no DOM.

use super::WidgetHost;
use crate::repaint_ctx::RepaintContext;
use op_editor_core::editor_ui_state::{DesignMdRequest, FileAction, RecentFile};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsValue;

struct TestRepaintContext {
    host: WidgetHost,
    repaint_count: usize,
}

impl TestRepaintContext {
    fn new(host: WidgetHost) -> Self {
        Self {
            host,
            repaint_count: 0,
        }
    }
}

impl RepaintContext for TestRepaintContext {
    fn host(&self) -> &WidgetHost {
        &self.host
    }

    fn host_mut(&mut self) -> &mut WidgetHost {
        &mut self.host
    }

    fn viewport_size(&self) -> (f32, f32) {
        (1200.0, 800.0)
    }

    fn register_system_font(&mut self, _family: &str, _bytes: &[u8]) -> bool {
        false
    }

    fn register_imported_font(&mut self, _family: &str, _bytes: &[u8]) -> bool {
        false
    }

    fn register_imported_font_from_bytes(&mut self, _bytes: &[u8]) -> Option<String> {
        None
    }

    fn imported_family_list(&self) -> Vec<String> {
        Vec::new()
    }

    fn remove_imported_font(&mut self, _family: &str) {}

    fn repaint(&mut self) -> Result<(), JsValue> {
        self.repaint_count += 1;
        Ok(())
    }
}

#[test]
fn ime_commit_lands_in_focused_chat_input() {
    let mut host = WidgetHost::new();
    host.editor_state.chat.focused = true;
    let evt = crate::event::ime::composition_end("你好".to_string());
    assert!(host.apply_ime(&evt));
    assert_eq!(host.editor_state.chat.input.text(), "你好");
}

#[test]
fn ime_preedit_updates_do_not_mutate_the_input() {
    let mut host = WidgetHost::new();
    host.editor_state.chat.focused = true;
    let start = crate::event::ime::composition_start();
    let update = crate::event::ime::composition_update("ni".to_string(), None);
    assert!(!host.apply_ime(&start));
    assert!(!host.apply_ime(&update));
    assert!(host.editor_state.chat.input.text().is_empty());
}

#[test]
fn ime_commit_without_any_focused_input_is_a_no_op() {
    let mut host = WidgetHost::new();
    let evt = crate::event::ime::composition_end("漢字".to_string());
    assert!(!host.apply_ime(&evt));
    assert!(host.editor_state.chat.input.text().is_empty());
}

#[test]
fn paste_text_routes_to_focused_rename() {
    let mut host = WidgetHost::new();
    // Begin an inline rename on the blank starter frame like a
    // layer-row double-click would.
    host.editor_state
        .set_single_selection(op_editor_core::NodeId::new("n10"));
    let id = host.editor_state.selection.anchor.clone();
    assert!(host.editor_state.start_rename_layer(id));
    // Select-all so the paste replaces the seeded name deterministically.
    host.editor_state
        .ui
        .layer_rename
        .as_mut()
        .expect("rename active")
        .input
        .select_all();
    assert!(host.apply_paste_text("Hero Section"));
    assert_eq!(
        host.editor_state
            .ui
            .layer_rename
            .as_ref()
            .expect("rename still active")
            .input
            .text(),
        "Hero Section"
    );
    // The paste went into the rename draft, NOT the chat input.
    assert!(host.editor_state.chat.input.text().is_empty());
}

#[test]
fn paste_text_routes_to_focused_chat_input() {
    let mut host = WidgetHost::new();
    host.editor_state.chat.focused = true;
    assert!(host.apply_paste_text("hello web"));
    assert_eq!(host.editor_state.chat.input.text(), "hello web");
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
fn drain_file_action_clear_recent_matches_desktop_state_change() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.recent_files = vec![
        RecentFile {
            path: "/tmp/a.op".to_string(),
            modified_at: 1,
        },
        RecentFile {
            path: "/tmp/b.op".to_string(),
            modified_at: 2,
        },
    ];
    host.editor_state.editor_ui.pending_file_action = Some(FileAction::ClearRecent);
    host.editor_state_dirty = false;
    let inner = Rc::new(RefCell::new(TestRepaintContext::new(host)));

    crate::dom_io::drain_pending_file_action(&inner);

    let borrowed = inner.borrow();
    assert!(borrowed.host.editor_state.editor_ui.recent_files.is_empty());
    assert!(borrowed
        .host
        .editor_state
        .editor_ui
        .pending_file_action
        .is_none());
    assert!(borrowed.host.editor_state_dirty);
    assert_eq!(borrowed.repaint_count, 1);
}

#[test]
fn drain_design_md_auto_generate_consumes_empty_document_request() {
    let mut host = WidgetHost::new();
    host.editor_state.doc.children.clear();
    host.editor_state.editor_ui.design_md_request = Some(DesignMdRequest::AutoGenerate);
    host.editor_state.editor_ui.design_md_generating = false;
    let inner = Rc::new(RefCell::new(TestRepaintContext::new(host)));

    crate::web_design_md::drain_design_md_action(&inner);

    let borrowed = inner.borrow();
    assert!(borrowed
        .host
        .editor_state
        .editor_ui
        .design_md_request
        .is_none());
    assert!(!borrowed.host.editor_state.editor_ui.design_md_generating);
    assert_eq!(borrowed.repaint_count, 0);
}

#[test]
fn open_recent_success_response_touches_local_recent_entry() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.recent_files = vec![
        RecentFile {
            path: "/tmp/a.op".to_string(),
            modified_at: 1,
        },
        RecentFile {
            path: "/tmp/b.op".to_string(),
            modified_at: 2,
        },
    ];

    let changed = crate::file_actions::apply_open_recent_response(
        host.editor_state_mut(),
        "/tmp/b.op",
        r#"{"ok":true,"version":3}"#,
        99,
    );

    assert!(changed);
    assert_eq!(
        host.editor_state.editor_ui.recent_files[0].path,
        "/tmp/b.op"
    );
    assert_eq!(host.editor_state.editor_ui.recent_files[0].modified_at, 99);
    assert_eq!(host.editor_state.editor_ui.recent_files.len(), 2);
    assert_eq!(
        host.editor_state.editor_ui.file_name_display.as_deref(),
        Some("b.op")
    );
}

#[test]
fn open_recent_stale_response_prunes_local_recent_entry() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.recent_files = vec![RecentFile {
        path: "/tmp/missing.op".to_string(),
        modified_at: 1,
    }];

    let changed = crate::file_actions::apply_open_recent_response(
        host.editor_state_mut(),
        "/tmp/missing.op",
        r#"{"ok":false,"pruned":true,"error":"missing"}"#,
        99,
    );

    assert!(changed);
    assert!(host.editor_state.editor_ui.recent_files.is_empty());
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
fn export_svg_document_returns_vector_markup() {
    let host = WidgetHost::new();
    let svg = crate::file_actions::export_svg_document(&host.editor_state)
        .expect("starter document exports svg");

    assert!(svg.starts_with("<svg "), "missing SVG root: {svg}");
    assert!(
        svg.contains("<rect ") || svg.contains("<text "),
        "starter export should contain vector elements: {svg}"
    );
    assert!(svg.ends_with("</svg>"), "missing SVG close: {svg}");
}

#[test]
fn export_svg_document_uses_single_selection() {
    let mut state = op_editor_core::EditorState::sample();
    state.set_single_selection(op_editor_core::NodeId::new("n11"));

    let svg = crate::file_actions::export_svg_document(&state).expect("selected svg");

    assert!(svg.contains("n11"), "selected node missing: {svg}");
    assert!(!svg.contains("n13"), "unselected sibling leaked: {svg}");
}
