//! Keep documents displaced by Home reachable from the mobile works list.

use super::write_document_state;
use super::{documents_dir, forget_current_document, sanitize_stem, unique_target_path};
use crate::error::{FfiError, FfiResult};
use crate::lifecycle::Session;
use crate::OpStatus;
use op_host_native::widget_host::ReplacedHomeDocument;
use std::path::Path;

/// Capture at every ABI boundary, before the next input can replace the
/// host's one-slot handoff. This only transfers ownership; saving is drained
/// separately so failures retain every outgoing document for a later retry.
pub(crate) fn capture_replaced_home_document(session: &mut Session) {
    let Some(replaced) = session
        .editor
        .as_mut()
        .and_then(|host| host.take_replaced_home_document())
    else {
        return;
    };
    let path = session
        .document_save
        .bound_path()
        .map(Path::to_path_buf)
        .or_else(|| {
            session
                .document_save
                .shell_binding()
                .and_then(|binding| binding.recent_copy.clone())
        });
    if let Some(path) = path.as_ref() {
        touch_recent(session, path);
    }
    // Even a clean picker-owned file needs a local copy: the engine cannot
    // reopen a SAF URI or security-scoped bookmark from its recent list.
    if replaced.had_unsaved_changes || path.is_none() {
        session.document_save.rescued.push_back(replaced);
    }
    // Invalidate pending picker callbacks and suspend resaves BEFORE the
    // incoming draft can be saved under the outgoing document's binding.
    forget_current_document(session);
    session.image_search.reset();
    session.selected = None;
    session.gesture.reset();
    session.user_interacted = false;
    if let Some(host) = session.editor.as_ref() {
        // Page/selection ABI calls use the lightweight state, while painting
        // uses the widget host. Both must observe the incoming Home draft.
        session.state = host.editor_state().clone();
        session.scene = op_pen_loader::editor_state_to_active_page_layout_scene(&session.state);
    }
    session.request_redraw();
}

/// A recovery failure must be visible without blocking Save to a different
/// destination. Keep retrying the owned snapshots, but report a continuous
/// failure once rather than flooding the shell on every frame.
pub(super) fn drain_recovery(session: &mut Session) {
    match persist_replaced_documents(session) {
        Ok(()) => session.document_save.recovery_error_reported = false,
        Err(error) => {
            if !session.document_save.recovery_error_reported {
                session.emit_runtime_error(2, &error.message, "op-engine-ffi/save");
                session.document_save.recovery_error_reported = true;
            }
        }
    }
}

fn persist_replaced_documents(session: &mut Session) -> FfiResult<()> {
    while let Some(replaced) = session.document_save.rescued.front() {
        let dir = documents_dir(&session.document_save)?.join("Autosave");
        std::fs::create_dir_all(&dir).map_err(|error| {
            FfiError::new(
                OpStatus::NotReady,
                format!("could not create the recovery directory: {error}"),
            )
        })?;
        let stem = if replaced.title.trim().is_empty() {
            super::seed_name(&replaced.state)
        } else {
            sanitize_stem(&replaced.title)
        };
        let path = unique_target_path(&dir, &stem)?;
        write_document_state(&replaced.state, &path, &replaced.thumbnails)?;
        // Do not take before writing: a full disk or unavailable directory
        // must leave the exact snapshot alive for the next drain/suspend.
        session.document_save.rescued.pop_front();
        touch_recent(session, &path);
    }
    Ok(())
}

pub(super) fn touch_recent(session: &mut Session, path: &Path) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default();
    if let Some(host) = session.editor.as_mut() {
        host.editor_state_mut()
            .editor_ui
            .touch_recent_file(path.to_string_lossy().into_owned(), now);
        host.mark_editor_state_dirty();
    }
    session.request_redraw();
}

/// Keep the exact bytes acknowledged by the picker reachable after restart.
/// Copy the staging file, not the live editor: newer edits may already exist.
pub(crate) fn cache_staged_document(
    session: &mut Session,
    staged: &Path,
    handle: &str,
    display_name: &str,
) -> FfiResult<std::path::PathBuf> {
    let dir = documents_dir(&session.document_save)?.join("Saved");
    let io_error = |error: std::io::Error| {
        FfiError::new(
            OpStatus::NotReady,
            format!("could not keep the saved document in Works: {error}"),
        )
    };
    std::fs::create_dir_all(&dir).map_err(io_error)?;
    let existing = session
        .document_save
        .shell_binding()
        .filter(|binding| binding.handle == handle)
        .and_then(|binding| binding.recent_copy.clone());
    let path = match existing {
        Some(path) => path,
        None => unique_target_path(&dir, &sanitize_stem(display_name.trim_end_matches(".op")))?,
    };
    let temp = super::sibling_temp_path(&path);
    let result = std::fs::copy(staged, &temp).and_then(|_| std::fs::rename(&temp, &path));
    if let Err(error) = result {
        let _ = std::fs::remove_file(&temp);
        return Err(io_error(error));
    }
    touch_recent(session, &path);
    super::binding::remember(&session.document_save, &path, handle, display_name)?;
    Ok(path)
}

pub(super) fn open_recent(session: &mut Session, index: usize) -> FfiResult<()> {
    let (path, outgoing) = {
        let host = session.editor_mut()?;
        host.editor_state_mut().editor_ui.pending_file_action = None;
        let path = host
            .editor_state()
            .editor_ui
            .recent_files
            .get(index)
            .map(|entry| std::path::PathBuf::from(&entry.path))
            .ok_or_else(|| FfiError::invalid("recent document no longer exists"))?;
        let state = host.editor_state_mut();
        let outgoing = state.is_dirty().then(|| ReplacedHomeDocument {
            title: state.chat.title.clone(),
            thumbnails: jian_ops_schema::image_thumbs::capture_snapshot(),
            had_unsaved_changes: true,
            state: Box::new(op_editor_core::request_snapshot::narrowed_snapshot(state)),
        });
        (path, outgoing)
    };
    let source = std::fs::read_to_string(&path).map_err(|error| {
        FfiError::new(
            OpStatus::BadDocument,
            format!("could not read the recent document: {error}"),
        )
    })?;
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    crate::editor::install_document_source(session, &source, name)?;
    session.document_save.binding = super::binding::restore(&session.document_save, &path);
    if let Some(outgoing) = outgoing {
        session.document_save.rescued.push_back(outgoing);
    }
    // Surface the opened work even when keeping the outgoing work must retry.
    touch_recent(session, &path);
    drain_recovery(session);
    touch_recent(session, &path);
    Ok(())
}
