//! Web arm of Preview effect delivery: the shared
//! `PreviewSession::deliver_editor_effects` presents toasts/alerts in
//! the editor toast slot; this arm performs link opening and clipboard
//! writes with the browser APIs (relayed to the parent inside the
//! VS Code embed, which blocks both in the nested iframe).

use super::WidgetHost;
use op_preview_core::PreviewPlatformRequest;

impl WidgetHost {
    /// Deliver queued Preview effects. Runs on pointer Up (inside the
    /// user gesture browsers require for popups / clipboard) and once
    /// per frame for effects a chain queues later.
    #[cfg(feature = "canvaskit")]
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
                PreviewPlatformRequest::OpenUrl(url) => open_url(&url),
                PreviewPlatformRequest::CopyText(text) => {
                    if crate::web_clipboard::is_vscode_embed() {
                        crate::web_clipboard::post_copy_to_parent(&text);
                    } else {
                        crate::web_clipboard::copy_text(&text);
                    }
                }
            }
        }
        if changed {
            self.mark_dirty();
        }
    }
}

#[cfg(feature = "canvaskit")]
fn open_url(url: &str) {
    if crate::web_clipboard::post_open_external_to_parent(url) {
        return;
    }
    if let Some(window) = web_sys::window() {
        let _ = window.open_with_url_and_target(url, "_blank");
    }
}
