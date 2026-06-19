//! Recent-files updater — the host-coupled sliver of settings I/O.
//!
//! The headless settings persistence (load / save / fingerprint /
//! save_if_changed + the on-disk payload DTOs + id allocators + locale
//! detection) lives in [`op_web_daemon::settings_io`]; this residual
//! keeps only `touch_recent`, which writes through the live
//! `WidgetHostNative` (orphan rule — it takes the host type).

use op_host_native::WidgetHostNative;

/// Push `path` to the head of the recent-files list on the host's
/// `EditorState`, dedupe by path, cap at 10. Called by `persistence`
/// after every successful Save / Save As / Open.
pub fn touch_recent(host: &mut WidgetHostNative, path: &std::path::Path) {
    let path_s = path.to_string_lossy().into_owned();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    host.editor_state_mut()
        .editor_ui
        .touch_recent_file(path_s, now);
    host.mark_editor_state_dirty();
}
