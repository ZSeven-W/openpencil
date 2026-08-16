//! Thin web dispatch arm for the shared Scene Template Center press flow.
//!
//! The template request is drained here, in the same call that raised it,
//! rather than from a frame pump. It can be: bringing a template in is pure
//! `EditorState` work on this host — the boards and their variables are
//! embedded, so there is no loader, no file dialog, and nothing to await. The
//! desktop drains from its event loop because its starter path also unbinds a
//! file path and rewrites the window title.

use op_editor_core::scene_template_append::template_boards;
use op_editor_core::scene_template_catalog::{
    scene_template_by_id, scene_template_document, scene_template_document_route,
};
use op_editor_ui::widgets::press_flow;
use op_editor_ui::Point2D;

use super::WidgetHost;

impl WidgetHost {
    pub(in crate::widget_host) fn dispatch_scene_template_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let Some(panel_rect) = self.scene_template_panel_rect(viewport_width, viewport_height)
        else {
            return false;
        };
        let Some(changed) = press_flow::press_scene_template_center(
            &mut self.editor_state,
            panel_rect,
            Point2D::new(x, y),
            self.now_ms,
        ) else {
            return false;
        };
        let brought_in = self.drain_pending_scene_template(viewport_width, viewport_height);
        self.discard_style_persistence_requests();
        if changed || brought_in {
            self.mark_dirty();
        }
        true
    }

    /// Turn a picked `DESIGN.md` into a style guide.
    ///
    /// The file drain reads bytes and stops there; this is where they meet the
    /// shared flow — the same one the paste box runs — so a guide picked as a
    /// file and the same guide pasted into the box are the same guide. It
    /// lives here rather than in the drain because `widget_host` is the only
    /// module on this host allowed to reach the widget layer at all.
    ///
    /// `fallback_name` is used only when the document names nothing; the
    /// caller passes the file stem, so an unnamed guide is still called after
    /// the file the user went and found.
    pub(crate) fn import_picked_style_guide(&mut self, markdown: &str, fallback_name: &str) {
        press_flow::import_style_guide_text(&mut self.editor_state, markdown, fallback_name);
        // Nothing here can write the guide down; the request the shared flow
        // raised is dropped for the same reason the paste route drops it.
        self.editor_state
            .editor_ui
            .scene_template_center
            .take_pending_style_persist();
        self.mark_dirty();
    }

    /// Report a picked file whose bytes did not survive the read.
    pub(crate) fn report_unreadable_style_import(&mut self) {
        press_flow::fail_style_import_unreadable(&mut self.editor_state);
        self.mark_dirty();
    }

    /// Drop the persist / delete requests the shared import flow raises.
    ///
    /// An imported style guide is live the moment it is parsed — the runtime
    /// catalogue is memory, and memory is all this host has. The requests
    /// exist for a host with a disk; here they are drained rather than left to
    /// accumulate, because nothing else will ever take them. The consequence
    /// is the documented M1 boundary: an imported style lasts the session and
    /// is gone after a reload.
    fn discard_style_persistence_requests(&mut self) {
        let center = &mut self.editor_state.editor_ui.scene_template_center;
        center.take_pending_style_persist();
        center.take_pending_style_delete();
        // Saved-template deletes are dropped for the same reason: this host
        // shows no user templates (its registry is always empty — the M1
        // boundary), so a delete request can never be raised, and the drain
        // keeps the queue from accumulating if one ever were.
        center.take_pending_template_delete();
        // `pending_file_pick` is deliberately NOT drained here. It is the one
        // style-import request this host can honour, and the mount's post-press
        // drain opens the browser file dialog with it — taking it here would
        // make the "choose file" button do nothing.
    }

    /// Instantiate a template whose document arrived after the click.
    ///
    /// Called once per frame by the CanvasKit mount. On native this is a
    /// no-op — the document is in the binary, so the press path already
    /// finished the job and nothing is ever left pending.
    pub(crate) fn retry_pending_scene_template(&mut self) -> bool {
        if self
            .editor_state
            .editor_ui
            .scene_template_center
            .pending_open
            .is_none()
        {
            return false;
        }
        let (w, h) = (self.last_viewport_w, self.last_viewport_h);
        if w <= 0.0 || h <= 0.0 {
            return false;
        }
        let brought_in = self.drain_pending_scene_template(w, h);
        if brought_in {
            self.mark_dirty();
        }
        brought_in
    }

    /// Bring a chosen template into the document. Returns whether it changed.
    fn drain_pending_scene_template(&mut self, viewport_w: f32, viewport_h: f32) -> bool {
        let Some(template_id) = self
            .editor_state
            .editor_ui
            .scene_template_center
            .take_pending_open()
        else {
            return false;
        };
        let Some(source) = scene_template_document(&template_id) else {
            // Web only: the ~1.1 MB of template documents are not in the wasm
            // bundle, so the first use of a template fetches it. Unknown ids
            // have no route and simply drop, exactly as before.
            let Some(route) = scene_template_document_route(&template_id) else {
                return false;
            };
            match op_editor_core::web_assets::state(route) {
                op_editor_core::web_assets::WebAssetState::Failed => {
                    // Say so instead of leaving a click that did nothing. The
                    // request stays retryable, so clicking the card again asks
                    // the daemon again.
                    self.editor_state.editor_ui.show_toast(
                        "sceneTemplate.documentUnavailable",
                        Vec::new(),
                        op_editor_core::editor_toast::EditorToastLevel::Warn,
                        self.now_ms,
                    );
                }
                _ => {
                    // Re-arm the request so the per-frame retry
                    // (`retry_pending_scene_template`) instantiates it the
                    // moment the bytes land.
                    op_editor_core::web_assets::request(route);
                    self.editor_state
                        .editor_ui
                        .scene_template_center
                        .request_open(template_id);
                }
            }
            return false;
        };
        let Some(boards) = template_boards(source, &template_id) else {
            return false;
        };
        // `adopt` picks replace-vs-append from what is on the page: an
        // untouched starter is taken over, anything else is added beside.
        if !self.editor_state.adopt_template_boards(boards) {
            return false;
        }
        // Only when the document has no scenario yet — a template dropped
        // next to other work does not redefine what the file is.
        if self.editor_state.editor_ui.scenario.is_none() {
            self.editor_state.editor_ui.scenario =
                scene_template_by_id(&template_id).map(|template| template.scene);
        }
        self.fit_content_to_viewport(viewport_w, viewport_h);
        self.force_rotate_layer_panel_owner();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::WidgetHost;

    const OCHRE: &str =
        "---\nname: Studio Ochre\n---\n\n# Studio Ochre\n\nWarm ochre. Accent #C77D3A.\n";

    /// A picked file lands on the shared flow: registered, pinned, box closed
    /// — the same three things a paste produces. The persist request the flow
    /// raises is dropped, because this host has nowhere to write it.
    #[test]
    fn a_picked_file_registers_and_pins_through_the_shared_flow() {
        let mut host = WidgetHost::new();
        host.editor_state
            .editor_ui
            .scene_template_center
            .import
            .open = true;
        host.import_picked_style_guide(OCHRE, "Studio Ochre");

        assert_eq!(
            host.editor_state.editor_ui.pinned_style_guide.as_deref(),
            Some("user:studio-ochre")
        );
        assert!(
            !host
                .editor_state
                .editor_ui
                .scene_template_center
                .import
                .open
        );
        assert!(host
            .editor_state
            .editor_ui
            .scene_template_center
            .take_pending_style_persist()
            .is_empty());
    }

    /// A press on "choose file" must survive the press handler. It is the one
    /// style-import request this host can honour, and draining it alongside
    /// the persist / delete pair is exactly how the button did nothing.
    #[test]
    fn a_file_pick_request_survives_the_press_drain() {
        let mut host = WidgetHost::new();
        host.editor_state
            .editor_ui
            .scene_template_center
            .request_style_import_file();
        host.discard_style_persistence_requests();

        assert!(host
            .editor_state
            .editor_ui
            .scene_template_center
            .take_pending_style_import_file());
    }

    #[test]
    fn an_unreadable_file_reports_in_the_box() {
        let mut host = WidgetHost::new();
        host.editor_state
            .editor_ui
            .scene_template_center
            .import
            .open = true;
        host.report_unreadable_style_import();

        assert!(
            host.editor_state
                .editor_ui
                .scene_template_center
                .import
                .open
        );
        assert_eq!(
            host.editor_state
                .editor_ui
                .scene_template_center
                .import
                .error_key,
            Some("assetCenter.style.importNotText")
        );
    }
}
