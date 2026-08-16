//! Settings-modal draft commit on the web host — split from
//! `keyboard.rs` to honor the 800-line cap.

use super::WidgetHost;
use op_editor_core::host_settings_commit::{commit_settings_focus, SettingsCommitScope};

impl WidgetHost {
    /// Commit the in-progress settings-modal input draft (MCP port,
    /// agent / image-gen fields, Openverse credentials).
    ///
    /// The walk itself is shared with the native host
    /// (`op_editor_core::host_settings_commit`); the browser commits in
    /// `Browser` scope, so it leaves the `web-credential:` id scoping and
    /// the `openverse_credential_owner` tag alone — it is already the
    /// owner of its own snapshot, and the daemon-side merge in
    /// `op-host-services::web_credentials` is what reads those fields.
    pub(super) fn commit_settings_focus(&mut self) {
        if commit_settings_focus(
            &mut self.editor_state,
            SettingsCommitScope::Browser,
            self.now_ms,
        ) {
            self.mark_dirty();
        }
    }
}
