//! Recent-files updater — the host-coupled sliver of settings I/O.
//!
//! The headless settings persistence (load / save / fingerprint /
//! save_if_changed + the on-disk payload DTOs + id allocators + locale
//! detection) lives in [`op_host_services::settings_io`]; this residual
//! keeps only `touch_recent`, which writes through the live
//! `WidgetHostNative` (orphan rule — it takes the host type).

use op_host_native::WidgetHostNative;
use op_host_services::settings_io::{fingerprint, save_if_changed};

/// Push `path` to the head of the recent-files list on the host's
/// `EditorState`, dedupe by path, cap at 10. Called by `persistence`
/// after every successful Save / Save As / Open.
pub fn touch_recent(host: &mut WidgetHostNative, path: &std::path::Path) {
    let before = fingerprint(host.editor_state());
    let path_s = path.to_string_lossy().into_owned();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    host.editor_state_mut()
        .editor_ui
        .touch_recent_file(path_s, now);
    host.mark_editor_state_dirty();
    // Save completion also runs outside the input-event dispatch whose
    // before/after snapshot normally persists settings. Own this mutation's
    // persistence so a completed Save/Open survives a cold restart.
    save_if_changed(host.editor_state(), before);
}

#[cfg(test)]
#[path = "settings_io_tests.rs"]
mod tests;
