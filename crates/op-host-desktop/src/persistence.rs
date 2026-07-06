//! `.pen` / `.op` file Save / Open — desktop-side dialog flow.
//!
//! The headless load / save core (the canonical serializer, embedded
//! editor view-state plus legacy `.opmeta` fallback,
//! legacy-`DocPayload` detection, and the `EditorState`
//! preference-carry helpers) lives in
//! [`op_host_services::doc_io`]; this module keeps the rfd / winit dialog
//! flow — the Save / Save-As / Open pickers, the file-menu
//! [`run_action`] router, and the native error dialog — and imports the
//! headless functions back.
//!
//! The native host's single source of truth is
//! `op_editor_core::EditorState` (built on the canonical
//! `jian_ops_schema::PenDocument`): Save serializes `editor_state.doc`
//! straight to canonical `.op` JSON and Open re-seeds `EditorState`
//! via the shared parser the TS editor / Jian apps use. See `doc_io`
//! for the on-disk format, editor metadata, and legacy-file handling.

use std::path::PathBuf;

use op_editor_core::EditorState;
use op_host_native::WidgetHostNative;
use op_host_services::doc_io::{
    active_page_bbox, load_editor_state, preserve_app_preferences, save_to_path,
    set_file_name_display, ActionOutcome, ErrorKind,
};

/// Pop a Save dialog (rfd native) and write the current document to
/// the chosen path. `Ok(Some(path))` on success, `Ok(None)` on user
/// cancel, `Err` on IO / encode failure.
pub fn save_as_dialog(state: &EditorState) -> Result<Option<PathBuf>, String> {
    let path = rfd::FileDialog::new()
        .set_title(op_i18n::translate(
            state.editor_ui.locale,
            "dialog.pickerSaveTitle",
        ))
        .add_filter("OpenPencil", &["pen", "op"])
        .set_file_name("untitled.op")
        .save_file();
    let Some(path) = path else {
        return Ok(None);
    };
    save_to_path(state, &path)?;
    Ok(Some(path))
}

/// Cmd+S — save to `current_path` if known, else fall through to
/// Save As. Updates `current_path` + window title on success.
/// Returns `true` when the document was written to disk, `false` on
/// an IO error or a cancelled Save-As dialog — so the caller can
/// tell a real save from a no-op (e.g. the unsaved-changes prompt).
pub fn handle_save(
    host: &mut WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> bool {
    if let Some(path) = current_path.clone() {
        match save_to_path(host.editor_state(), &path) {
            Err(e) => {
                eprintln!("[save] {e}");
                show_error_dialog(host, ErrorKind::Save, Some(&path), &e);
                return false;
            }
            Ok(()) => {
                crate::settings_io::touch_recent(host, &path);
                set_display_name(host, Some(&path));
                host.editor_state_mut().mark_saved_revision();
                return true;
            }
        }
    }
    handle_save_as(host, current_path, window)
}

fn set_display_name(host: &mut WidgetHostNative, path: Option<&std::path::Path>) {
    set_file_name_display(host.editor_state_mut(), path);
}

fn viewport_size_for_window(window: Option<&winit::window::Window>) -> (f32, f32) {
    window
        .map(|w| {
            let size = w.inner_size();
            let scale = w.scale_factor() as f32;
            (size.width as f32 / scale, size.height as f32 / scale)
        })
        .unwrap_or((super::INITIAL_VIEWPORT_W, super::INITIAL_VIEWPORT_H))
}

fn fit_loaded_document(host: &mut WidgetHostNative, window: Option<&winit::window::Window>) {
    let (vw, vh) = viewport_size_for_window(window);
    host.fit_content_to_viewport(vw, vh);
    host.editor_state_mut().mark_saved_revision();
    host.mark_editor_state_dirty();
}

/// Cmd+Shift+S — always pop the Save dialog.
pub fn handle_save_as(
    host: &mut WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> bool {
    match save_as_dialog(host.editor_state()) {
        Ok(Some(path)) => {
            crate::settings_io::touch_recent(host, &path);
            // Mirror handle_save: refresh the in-chrome file name too, not
            // just the OS window title — without this, first Save As writes
            // the file but the TopBar keeps showing "Untitled".
            set_display_name(host, Some(&path));
            host.editor_state_mut().mark_saved_revision();
            *current_path = Some(path);
            refresh_title(current_path, window);
            true
        }
        Ok(None) => false,
        Err(e) => {
            eprintln!("[save as] {e}");
            show_error_dialog(host, ErrorKind::Save, None, &e);
            false
        }
    }
}

/// Replace the host's `EditorState` with one loaded from `path`.
fn load_into_host(host: &mut WidgetHostNative, path: &std::path::Path) -> Result<(), String> {
    let locale = host.editor_state().editor_ui.locale;
    let mut state = load_editor_state(path, locale)?;
    preserve_app_preferences(host.editor_state(), &mut state);
    set_file_name_display(&mut state, Some(path));
    state.clear_selection();
    let bb = active_page_bbox(&state);
    eprintln!(
        "[open] {} top-level nodes; content bbox {:?}",
        state.doc.children.len(),
        bb
    );
    *host.editor_state_mut() = state;
    host.editor_state_mut().mark_saved_revision();
    host.mark_editor_state_dirty();
    Ok(())
}

/// Cmd+O — pop the Open dialog and replace the current document.
pub fn handle_open(
    host: &mut WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> bool {
    let path = match rfd::FileDialog::new()
        .set_title(op_i18n::translate(
            host.editor_state().editor_ui.locale,
            "dialog.pickerOpenTitle",
        ))
        .add_filter("OpenPencil", &["pen", "op"])
        .pick_file()
    {
        Some(p) => p,
        None => return false,
    };
    match load_into_host(host, &path) {
        Ok(()) => {
            fit_loaded_document(host, window);
            crate::settings_io::touch_recent(host, &path);
            *current_path = Some(path);
            refresh_title(current_path, window);
            true
        }
        Err(e) => {
            eprintln!("[open] {e}");
            show_error_dialog(host, ErrorKind::Open, Some(&path), &e);
            false
        }
    }
}

/// Open `path` directly — no dialog. Backs drag-and-drop drops and
/// the file-association launch path. Replaces the host's document,
/// records the file in recents and refreshes the window title.
/// Returns `true` when the document loaded (so the caller can
/// request a redraw); a load failure pops the error dialog and
/// leaves the current document untouched.
pub fn open_path(
    host: &mut WidgetHostNative,
    path: PathBuf,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> bool {
    match load_into_host(host, &path) {
        Ok(()) => {
            fit_loaded_document(host, window);
            crate::settings_io::touch_recent(host, &path);
            *current_path = Some(path);
            refresh_title(current_path, window);
            true
        }
        Err(e) => {
            eprintln!("[open] {e}");
            show_error_dialog(host, ErrorKind::Open, Some(&path), &e);
            false
        }
    }
}

/// Route a `FileAction` raised by the file-menu dispatcher to the
/// matching dialog flow. The returned [`ActionOutcome`] tells the
/// runner which post-action bookkeeping to run — see its variant
/// docs.
pub fn run_action(
    action: op_editor_core::editor_ui_state::FileAction,
    host: &mut WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> ActionOutcome {
    use op_editor_core::editor_ui_state::FileAction;
    match action {
        FileAction::New => {
            let mut state = EditorState::starter();
            preserve_app_preferences(host.editor_state(), &mut state);
            *host.editor_state_mut() = state;
            let (vw, vh) = viewport_size_for_window(window);
            host.fit_content_to_viewport(vw, vh);
            host.editor_state_mut().mark_saved_revision();
            host.mark_editor_state_dirty();
            *current_path = None;
            refresh_title(current_path, window);
            ActionOutcome::Saved
        }
        FileAction::Open => ActionOutcome::saved_or_noop(handle_open(host, current_path, window)),
        FileAction::Save => ActionOutcome::saved_or_noop(handle_save(host, current_path, window)),
        FileAction::SaveAs => {
            ActionOutcome::saved_or_noop(handle_save_as(host, current_path, window))
        }
        FileAction::ExportImage => {
            // main.rs intercepts ExportImage to open the picker; this
            // fallback keeps external callers working.
            host.editor_state_mut().editor_ui.export_dialog_open = true;
            host.mark_editor_state_dirty();
            ActionOutcome::Noop
        }
        FileAction::ExportImageConfirm => {
            use op_editor_core::editor_ui_state::ExportFormat as Fmt;
            let fmt = host.editor_state().editor_ui.export_format;
            let scale = host.editor_state().editor_ui.export_scale;
            let (filter_label, filter_exts): (&str, &[&str]) = match fmt {
                Fmt::Png => ("PNG", &["png"]),
                Fmt::Jpeg => ("JPEG", &["jpg", "jpeg"]),
                Fmt::Webp => ("WEBP", &["webp"]),
                Fmt::Svg => ("SVG", &["svg"]),
                Fmt::Pdf => ("PDF", &["pdf"]),
            };
            let default_name = format!("openpencil-export.{}", fmt.extension());
            if let Some(path) = rfd::FileDialog::new()
                .set_title(op_i18n::translate(
                    host.editor_state().editor_ui.locale,
                    "dialog.pickerExportTitle",
                ))
                .add_filter(filter_label, filter_exts)
                .set_file_name(&default_name)
                .save_file()
            {
                // The export renderers consume a layout-resolved
                // `LayoutScene` — build one from the live editor state
                // (runs jian's flex pass + `$ref` fill resolution).
                let scene = op_pen_loader::editor_state_to_layout_scene(host.editor_state());
                let scene = &scene;
                // When exactly one node is selected, raster export
                // crops to that layer (TS parity: exportLayerToRaster);
                // otherwise the whole active page is exported. SVG /
                // PDF always stay page-level.
                let single_node: Option<String> = {
                    let st = host.editor_state();
                    if st.selection_count() == 1 && st.selection.anchor.is_real() {
                        Some(st.selection.anchor.as_str().to_string())
                    } else {
                        None
                    }
                };
                let raster = |rf: op_host_services::export::RasterFormat| -> Result<(), String> {
                    match &single_node {
                        Some(id) => op_host_services::export::export_node_raster(
                            scene, id, &path, rf, scale,
                        ),
                        None => op_host_services::export::export_raster(scene, &path, rf, scale),
                    }
                };
                let result: Result<(), String> = match fmt {
                    Fmt::Png => raster(op_host_services::export::RasterFormat::Png),
                    Fmt::Jpeg => raster(op_host_services::export::RasterFormat::Jpeg),
                    Fmt::Webp => raster(op_host_services::export::RasterFormat::Webp),
                    Fmt::Svg => op_host_services::export::export_svg(scene, &path),
                    Fmt::Pdf => op_host_services::export_pdf::export_pdf(scene, &path),
                };
                if let Err(e) = result {
                    eprintln!("[export-image] {e}");
                    show_error_dialog(host, ErrorKind::Export, Some(&path), &e);
                }
            }
            ActionOutcome::Noop
        }
        FileAction::OpenRecent(i) => {
            let Some(entry) = host.editor_state().editor_ui.recent_files.get(i).cloned() else {
                return ActionOutcome::Noop;
            };
            let path = std::path::PathBuf::from(&entry.path);
            match load_into_host(host, &path) {
                Ok(()) => {
                    fit_loaded_document(host, window);
                    crate::settings_io::touch_recent(host, &path);
                    *current_path = Some(path);
                    refresh_title(current_path, window);
                    ActionOutcome::Saved
                }
                Err(e) => {
                    // File missing / parse failure → tell the user and
                    // drop the stale entry from recents.
                    eprintln!("[open-recent] {e}; pruning {}", entry.path);
                    show_error_dialog(host, ErrorKind::Open, Some(&path), &e);
                    host.editor_state_mut()
                        .editor_ui
                        .recent_files
                        .retain(|r| r.path != entry.path);
                    host.mark_editor_state_dirty();
                    ActionOutcome::Noop
                }
            }
        }
        FileAction::ClearRecent => {
            host.editor_state_mut().editor_ui.recent_files.clear();
            host.mark_editor_state_dirty();
            ActionOutcome::Noop
        }
        FileAction::ImportFigma => {
            let path = match rfd::FileDialog::new()
                .set_title(op_i18n::translate(
                    host.editor_state().editor_ui.locale,
                    "dialog.pickerOpenTitle",
                ))
                .add_filter("Figma", &["fig"])
                .pick_file()
            {
                Some(p) => p,
                None => return ActionOutcome::Noop,
            };
            // Spawn the parse on a worker thread so the UI keeps
            // repainting (a 2–3 MB .fig with hundreds of nodes takes
            // multiple seconds; running it on the main thread freezes
            // the window). The desktop runner picks up the session in
            // the next `RedrawRequested` pump and applies the result
            // when it lands.
            ActionOutcome::FigmaImportStarted(path)
        }
        FileAction::ImportImageOrSvg => {
            crate::persistence_image::handle_import_image_or_svg(host);
            ActionOutcome::Noop
        }
        FileAction::PickFillImage => {
            crate::persistence_image::handle_pick_fill_image(host);
            ActionOutcome::Noop
        }
        FileAction::RelinkImage => {
            crate::persistence_image::handle_relink_image(host, current_path.as_deref());
            ActionOutcome::Noop
        }
    }
}

// `import_figma_into_host` (synchronous parse) was retired in favour
// of `figma_import_session::spawn`, which moves the parse to a worker
// thread and pumps the result back through a channel each frame.

fn refresh_title(current_path: &Option<PathBuf>, window: Option<&winit::window::Window>) {
    let Some(window) = window else { return };
    let title = match current_path.as_ref().and_then(|p| p.file_name()) {
        Some(name) => format!("{} — OpenPencil", name.to_string_lossy()),
        None => "OpenPencil".to_string(),
    };
    window.set_title(&title);
}

/// Pop a native error dialog. Used by Open / Save / Export when the
/// underlying IO or parse step fails.
fn show_error_dialog(
    host: &WidgetHostNative,
    kind: ErrorKind,
    path: Option<&std::path::Path>,
    detail: &str,
) {
    let locale = host.editor_state().editor_ui.locale;
    let (title_key, lead_key) = match kind {
        ErrorKind::Open => ("dialog.openErrorTitle", "dialog.openErrorLead"),
        ErrorKind::Save => ("dialog.saveErrorTitle", "dialog.saveErrorLead"),
        ErrorKind::Export => ("dialog.exportErrorTitle", "dialog.exportErrorLead"),
    };
    let mut body = op_i18n::translate(locale, lead_key).to_string();
    if let Some(p) = path {
        body.push_str("\n\n");
        body.push_str(&p.display().to_string());
    }
    body.push_str("\n\n");
    body.push_str(detail);
    rfd::MessageDialog::new()
        .set_title(op_i18n::translate(locale, title_key))
        .set_description(&body)
        .set_level(rfd::MessageLevel::Error)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

/// Public re-export of the native error dialog — used by the
/// background Figma import session (`figma_import_session::pump`) to
/// pop the same OS dialog the synchronous error path uses.
pub fn show_error_dialog_public(
    host: &WidgetHostNative,
    kind: ErrorKind,
    path: Option<&std::path::Path>,
    detail: &str,
) {
    show_error_dialog(host, kind, path, detail)
}

#[cfg(test)]
mod tests {
    use super::*;
    // `save_to_path` arrives via `super::*`; `sidecar_path` is only
    // needed for legacy-sidecar cleanup in tests, so import it directly.
    use op_host_services::doc_io::sidecar_path;

    /// A unique temp path under the OS temp dir for a round-trip test.
    fn temp_op_path(tag: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        p.push(format!("openpencil-test-{tag}-{pid}-{nanos}.op"));
        p
    }

    #[test]
    fn new_file_action_resets_to_starter_frame() {
        let mut host = WidgetHostNative::new();
        host.editor_state_mut().doc.children.clear();
        host.editor_state_mut().viewport.pan_x = -5000.0;
        host.editor_state_mut().viewport.pan_y = -5000.0;
        host.editor_state_mut().viewport.zoom = 0.2;
        let mut current_path = Some(PathBuf::from("/tmp/old.op"));

        let outcome = run_action(
            op_editor_core::editor_ui_state::FileAction::New,
            &mut host,
            &mut current_path,
            None,
        );

        assert_eq!(outcome, ActionOutcome::Saved);
        assert!(current_path.is_none());
        assert_eq!(host.editor_state().doc.children.len(), 1);
        assert!(host.editor_state().selection.is_empty());
        let frame = match &host.editor_state().doc.children[0] {
            jian_ops_schema::node::PenNode::Frame(frame) => frame,
            other => panic!(
                "new file should create the blank starter frame, got {:?}",
                other
            ),
        };
        assert_eq!(frame.base.x, Some(0.0));
        assert_eq!(frame.base.y, Some(0.0));
        assert!(matches!(
            frame.container.width,
            Some(jian_ops_schema::sizing::SizingBehavior::Number(1200.0))
        ));
        assert!(matches!(
            frame.container.height,
            Some(jian_ops_schema::sizing::SizingBehavior::Number(800.0))
        ));
        let v = host.editor_state().viewport;
        assert!((v.zoom - 0.8933333).abs() < 1e-3, "zoom {}", v.zoom);
        assert!((v.pan_x - 64.0).abs() < 1e-2, "pan_x {}", v.pan_x);
        assert!((v.pan_y - 72.66669).abs() < 1e-2, "pan_y {}", v.pan_y);
    }

    #[test]
    fn new_file_action_preserves_builtin_agent_models() {
        let mut host = WidgetHostNative::new();
        let builtin_id = host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .add_builtin_agent_config(
                "DS",
                "sk-test",
                "deepseek-v4-pro",
                op_editor_core::BuiltinAgentKind::OpenAiCompat,
                "https://api.deepseek.com/v1",
            );
        host.editor_state_mut().rebuild_chat_models();
        let mut current_path = Some(PathBuf::from("/tmp/old.op"));

        let outcome = run_action(
            op_editor_core::editor_ui_state::FileAction::New,
            &mut host,
            &mut current_path,
            None,
        );

        assert_eq!(outcome, ActionOutcome::Saved);
        assert_eq!(
            host.editor_state()
                .editor_ui
                .agent_settings
                .builtin_agents
                .len(),
            1
        );
        assert!(host
            .editor_state()
            .chat
            .available_models
            .iter()
            .any(|m| m.builtin_provider_id.as_deref() == Some(builtin_id.as_str())));
    }

    #[test]
    fn opening_document_preserves_builtin_agent_models() {
        let mut host = WidgetHostNative::new();
        let builtin_id = host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .add_builtin_agent_config(
                "MINIMAX",
                "sk-test",
                "MiniMax-M2.7",
                op_editor_core::BuiltinAgentKind::OpenAiCompat,
                "https://api.minimaxi.com/v1",
            );
        host.editor_state_mut().rebuild_chat_models();
        assert!(host
            .editor_state()
            .chat
            .available_models
            .iter()
            .any(|m| m.builtin_provider_id.as_deref() == Some(builtin_id.as_str())));

        let state_to_open = EditorState::new();
        let path = temp_op_path("open-preserves-builtins");
        save_to_path(&state_to_open, &path).expect("save succeeds");
        let mut current_path = None;

        assert!(open_path(&mut host, path.clone(), &mut current_path, None));

        assert_eq!(
            host.editor_state()
                .editor_ui
                .agent_settings
                .builtin_agents
                .len(),
            1
        );
        assert!(host
            .editor_state()
            .chat
            .available_models
            .iter()
            .any(|m| m.builtin_provider_id.as_deref() == Some(builtin_id.as_str())));

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(sidecar_path(&path));
    }

    #[test]
    fn opening_document_leaves_nothing_selected() {
        let mut host = WidgetHostNative::new();
        host.editor_state_mut()
            .set_single_selection(op_editor_core::NodeId::new("n10"));

        let mut state_to_open = EditorState::starter();
        state_to_open.set_single_selection(op_editor_core::NodeId::new("n10"));
        let path = temp_op_path("open-clears-selection");
        save_to_path(&state_to_open, &path).expect("save succeeds");
        let mut current_path = None;

        assert!(open_path(&mut host, path.clone(), &mut current_path, None));

        assert!(host.editor_state().selection.is_empty());
        assert_eq!(host.editor_state().doc.children.len(), 1);

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(sidecar_path(&path));
    }

    #[test]
    fn opening_document_fits_and_centers_multiple_root_nodes() {
        let mut host = WidgetHostNative::new();
        let doc = jian_ops_schema::load_str(
            r#"{
              "version":"0.8.0",
              "children":[
                {"type":"frame","id":"left","name":"Left","x":900,"y":120,"width":240,"height":320},
                {"type":"frame","id":"right","name":"Right","x":1320,"y":220,"width":260,"height":280}
              ]
            }"#,
        )
        .expect("fixture JSON parses")
        .value;
        let state_to_open = EditorState::from_document(doc);
        let path = temp_op_path("open-centers-multi-root");
        save_to_path(&state_to_open, &path).expect("save succeeds");
        let mut current_path = None;

        assert!(open_path(&mut host, path.clone(), &mut current_path, None));

        let (min_x, min_y, max_x, max_y) =
            active_page_bbox(host.editor_state()).expect("opened content has bounds");
        let content_center_x = ((min_x + max_x) / 2.0) as f32;
        let content_center_y = ((min_y + max_y) / 2.0) as f32;
        let (canvas_w, canvas_h) = op_host_services::design_session::design_canvas_size(
            host.editor_state(),
            super::super::INITIAL_VIEWPORT_W,
            super::super::INITIAL_VIEWPORT_H,
        );
        let screen_center_x = host.editor_state().viewport.pan_x
            + content_center_x * host.editor_state().viewport.zoom;
        let screen_center_y = host.editor_state().viewport.pan_y
            + content_center_y * host.editor_state().viewport.zoom;

        assert!(
            (screen_center_x - canvas_w / 2.0).abs() < 0.5,
            "opened content should be horizontally centered: screen_center_x={screen_center_x}, canvas_w={canvas_w}"
        );
        assert!(
            (screen_center_y - canvas_h / 2.0).abs() < 0.5,
            "opened content should be vertically centered: screen_center_y={screen_center_y}, canvas_h={canvas_h}"
        );

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(sidecar_path(&path));
    }
}
