//! Background `.fig` import session — moves the multi-second
//! `parse_fig_binary` call off the main thread so the editor UI keeps
//! repainting (cursor moves, overlay animation) while a large Figma
//! file decodes.
//!
//! Lifecycle, mirrors `chat_session`:
//!   1. `spawn` — kick off file read + parse on a `std::thread`,
//!      returning a session handle holding the receiver.
//!   2. `pump` — called every `RedrawRequested`; non-blocking
//!      `try_recv` on the channel. Returns whether the host state
//!      changed (so the caller can mark the next frame dirty).
//!   3. `is_pending` — true while the worker is still running. The
//!      app handler reads this to schedule periodic wakes so the
//!      "正在解析…" overlay keeps repainting under `WaitUntil` flow.
//!
//! On result land:
//!   - `Ok(import)` → swap `EditorState` to the imported document and
//!     clear `figma_import_in_progress`.
//!   - `Err(e)` → show the native error dialog + clear the flag.

use op_editor_core::EditorState;
use op_host_native::WidgetHostNative;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use crate::persistence::{show_error_dialog_public, ErrorKind};

/// Successful worker output. Keep this Skia-free: building
/// `LayoutScene` runs text measurement, which can contend with the
/// main-thread painter and freeze the progress overlay.
pub struct PreparedImport {
    pub state: EditorState,
    pub warnings: Vec<String>,
}

/// One in-flight `.fig` parse — the source path (for the error
/// dialog) plus the worker-thread receiver.
pub struct FigmaImportSession {
    path: PathBuf,
    rx: Receiver<Result<PreparedImport, String>>,
}

impl FigmaImportSession {
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Spawn a worker thread that reads `path`, parses it with
/// `op_figma::parse_fig_binary` in `Preserve` mode, and posts the
/// result back through a channel. Returns the session handle.
pub fn spawn(host: &mut WidgetHostNative, path: PathBuf) -> FigmaImportSession {
    let (tx, rx) = mpsc::channel();
    // Flip the overlay flag so paint shows "正在解析…" feedback as
    // soon as the next frame fires. We deliberately do NOT call
    // `mark_editor_state_dirty()` here: that would set
    // `editor_state_dirty=true`, which triggers
    // `refresh_layout_scene` on the next paint and rebuilds the
    // layout against the OLD document — wasted work since the
    // import is about to replace `editor_state` whole-cloth. The
    // overlay widget reads `editor_ui.figma_import_in_progress`
    // directly, not through `layout_scene`, so the cached layout
    // from before the spawn is fine to keep painting underneath.
    host.editor_state_mut().editor_ui.figma_import_in_progress = true;

    let path_for_thread = path.clone();
    thread::Builder::new()
        .name("op-figma-import".into())
        .spawn(move || {
            let result = parse_path(&path_for_thread);
            // Recv side may be gone if the user closed the app
            // mid-parse; tolerate the SendError silently.
            let _ = tx.send(result);
        })
        .expect("spawn op-figma-import worker");

    FigmaImportSession { path, rx }
}

fn parse_path(path: &Path) -> Result<PreparedImport, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let file_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Figma Import");
    let import = op_figma::parse_fig_binary(&bytes, file_name, op_figma::FigLayoutMode::Preserve)
        .map_err(|e| e.to_string())?;
    let mut state = EditorState::from_document(import.document);
    state.editor_ui.preserve_authored_geometry = true;
    Ok(PreparedImport {
        state,
        warnings: import.warnings,
    })
}

/// Returns true when the session resolved and the host state changed
/// (caller should mark the next frame dirty). Returns false when the
/// worker is still running or no session is active.
///
/// On success this drains the receiver, applies the imported
/// document, refreshes the host title, and clears the
/// in-progress flag. On failure it pops the native error dialog. In
/// either case the `*session` slot becomes `None`.
pub fn pump(
    host: &mut WidgetHostNative,
    session: &mut Option<FigmaImportSession>,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> PumpOutcome {
    let Some(sess) = session.as_mut() else {
        return PumpOutcome::Idle;
    };
    match sess.rx.try_recv() {
        Ok(Ok(prepared)) => {
            for warning in &prepared.warnings {
                eprintln!("[import-figma] warning: {warning}");
            }
            // Swap in the parsed state. The worker deliberately did
            // not build a LayoutScene because that touches Skia /
            // FontMgr and can block the main-thread progress overlay.
            host.install_imported_state(prepared.state);
            // Imported docs have no `.op` path; next Save routes via
            // Save As — matches the synchronous import behaviour.
            *current_path = None;
            refresh_title(current_path, window);
            *session = None;
            PumpOutcome::CompletedOk
        }
        Ok(Err(e)) => {
            eprintln!("[import-figma] {e}");
            show_error_dialog_public(host, ErrorKind::Open, Some(&sess.path), &e);
            host.editor_state_mut().editor_ui.figma_import_in_progress = false;
            host.mark_editor_state_dirty();
            *session = None;
            PumpOutcome::CompletedErr
        }
        Err(TryRecvError::Empty) => PumpOutcome::StillPending,
        Err(TryRecvError::Disconnected) => {
            // Worker thread panicked or dropped without sending —
            // pop the same native dialog the explicit error path
            // uses so the user gets feedback instead of a silently
            // vanishing progress overlay.
            eprintln!("[import-figma] worker thread terminated without sending a result");
            let detail = "Figma import worker exited unexpectedly";
            show_error_dialog_public(host, ErrorKind::Open, Some(&sess.path), detail);
            host.editor_state_mut().editor_ui.figma_import_in_progress = false;
            host.mark_editor_state_dirty();
            *session = None;
            PumpOutcome::CompletedErr
        }
    }
}

/// Drop the active session (if any) and clear the in-progress flag —
/// called when another document-replacing action runs while a Figma
/// import is still parsing (File→New, File→Open, another File→Import
/// Figma). Without this guard, a stale worker would later overwrite
/// the user's freshly-opened document in `pump`.
///
/// The worker thread keeps running until it tries to `tx.send`; that
/// send becomes a no-op once we drop the receiver here. The thread
/// is short-lived (one parse + layout pass) so leaking it briefly is
/// fine.
pub fn cancel(host: &mut WidgetHostNative, session: &mut Option<FigmaImportSession>) {
    if session.is_some() {
        eprintln!("[import-figma] cancelling in-flight session — superseded");
        *session = None;
        if host.editor_state().editor_ui.figma_import_in_progress {
            host.editor_state_mut().editor_ui.figma_import_in_progress = false;
            host.mark_editor_state_dirty();
        }
    }
}

/// Outcome of `pump` — used by the caller to decide whether to mark
/// the next frame dirty and whether to reset the document-path state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PumpOutcome {
    /// No active session.
    Idle,
    /// Worker thread still running.
    StillPending,
    /// Worker finished and the document was applied.
    CompletedOk,
    /// Worker finished with an error (dialog already shown).
    CompletedErr,
}

fn refresh_title(current_path: &Option<PathBuf>, window: Option<&winit::window::Window>) {
    let Some(window) = window else { return };
    let title = match current_path.as_ref().and_then(|p| p.file_name()) {
        Some(name) => format!("{} — OpenPencil", name.to_string_lossy()),
        None => "OpenPencil".to_string(),
    };
    window.set_title(&title);
}
