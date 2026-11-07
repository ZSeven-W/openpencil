//! Native arm of Preview effect delivery: the shared
//! `PreviewSession::deliver_editor_effects` presents toasts/alerts in
//! the editor toast slot; this arm performs the two platform requests.
//! Copy goes through `chat.pending_copy_text`, which the desktop runner
//! already drains into the OS clipboard (the host lib has no clipboard
//! of its own — the window crate owns it).

use super::WidgetHostNative;
use op_preview_core::PreviewPlatformRequest;

impl WidgetHostNative {
    /// Deliver queued Preview effects. Called once per frame before the
    /// preview reconcile, and right after preview pointer input so a
    /// tap's copy lands while the desktop runner still drains clipboard
    /// requests for that event.
    pub(in crate::widget_host) fn deliver_preview_effects(&mut self) {
        let Some(preview) = self.preview.as_mut() else {
            return;
        };
        let toast_before = self.editor_state.editor_ui.editor_toast.clone();
        let requests =
            preview.deliver_editor_effects(&mut self.editor_state.editor_ui, self.now_ms);
        let changed =
            !requests.is_empty() || self.editor_state.editor_ui.editor_toast != toast_before;
        for request in requests {
            match request {
                PreviewPlatformRequest::OpenUrl(url) => {
                    super::press_helpers::open_external_url(&url);
                }
                PreviewPlatformRequest::CopyText(text) => {
                    self.editor_state.chat.pending_copy_text = Some(text);
                }
            }
        }
        if changed {
            self.mark_dirty();
        }
    }
}
