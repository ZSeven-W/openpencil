//! Agent/settings modal input commits.

use super::WidgetHostNative;
use op_editor_core::host_settings_commit::{commit_settings_focus, SettingsCommitScope};

impl WidgetHostNative {
    /// Commit any focused settings-modal input.
    ///
    /// The walk itself is shared with the web host
    /// (`op_editor_core::host_settings_commit`); this host commits as the
    /// desktop OPERATOR, so a draft landing on a browser-pushed
    /// credential snapshot transfers ownership to the local settings
    /// file.
    pub(in crate::widget_host) fn commit_settings_focus_if_any(&mut self) {
        if commit_settings_focus(
            &mut self.editor_state,
            SettingsCommitScope::Operator,
            self.now_ms,
        ) {
            self.mark_dirty();
        }
    }
}
