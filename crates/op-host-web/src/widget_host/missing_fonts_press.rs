//! Web missing-font modal dispatch and detection lifecycle.
//!
//! The modal's scroll + press dispatch and the detection/refresh
//! bookkeeping live in the shared
//! `op_editor_ui::widgets::missing_fonts_flow` (driven by the native
//! host too); this file keeps the web platform arms — the settings
//! Fonts-tab press dispatch plus the asynchronous CanvasKit
//! `queryLocalFonts` hand-off — and the `mark_dirty` tails.

use super::WidgetHost;
use op_editor_ui::widgets::missing_fonts_flow as fonts_flow;
use op_editor_ui::{Point2D, Rect};

impl WidgetHost {
    pub(in crate::widget_host) fn try_scroll_missing_fonts_picker(
        &mut self,
        x: f32,
        y: f32,
        delta_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let Some(changed) = fonts_flow::scroll_picker(
            &mut self.editor_state,
            x,
            y,
            delta_y,
            viewport_width,
            viewport_height,
        ) else {
            return false;
        };
        if changed {
            self.mark_dirty();
        }
        true
    }

    pub(in crate::widget_host) fn dispatch_missing_fonts_press(
        &mut self,
        panel_rect: Rect,
        viewport_rect: Rect,
        point: Point2D,
    ) -> bool {
        if !fonts_flow::press(&mut self.editor_state, panel_rect, viewport_rect, point) {
            return false;
        }
        self.mark_dirty();
        true
    }

    pub(crate) fn take_missing_fonts_import_row(&mut self) -> Option<usize> {
        self.editor_state.editor_ui.missing_fonts_import_row.take()
    }

    /// Arm detection and schedule the existing CanvasKit font drain, whose
    /// queryLocalFonts path supplies the asynchronous system snapshot.
    pub fn arm_missing_fonts_detection(&mut self) {
        if self.editor_state.editor_ui.system_fonts_loaded {
            fonts_flow::replace_data(&mut self.editor_state, true);
            self.mark_dirty();
            return;
        }
        fonts_flow::arm_pending_detection(&mut self.editor_state, true);
        self.mark_dirty();
        crate::repaint_coalescer::request();
    }

    pub(in crate::widget_host) fn refresh_missing_fonts_for_settings(&mut self) {
        if self.editor_state.editor_ui.system_fonts_loaded {
            fonts_flow::replace_data(&mut self.editor_state, false);
            self.mark_dirty();
            return;
        }
        fonts_flow::arm_pending_detection(&mut self.editor_state, false);
        self.mark_dirty();
        crate::repaint_coalescer::request();
    }

    pub(in crate::widget_host) fn refresh_missing_fonts_after_history_change(&mut self) {
        if self.editor_state.editor_ui.system_fonts_loaded {
            fonts_flow::clear_pending_detection(&mut self.editor_state);
            self.refresh_missing_fonts_prompt();
            return;
        }
        let open_modal = self.editor_state.editor_ui.missing_fonts_modal_open;
        fonts_flow::arm_pending_detection(&mut self.editor_state, open_modal);
        self.mark_dirty();
        crate::repaint_coalescer::request();
    }

    pub(in crate::widget_host) fn refresh_missing_fonts_after_document_change(&mut self) {
        if fonts_flow::settings_fonts_open(&self.editor_state) {
            self.refresh_missing_fonts_for_settings();
        } else {
            self.arm_missing_fonts_detection();
        }
    }

    pub(crate) fn complete_pending_missing_fonts_detection(&mut self) {
        // Detection opens a one-shot modal, so it must not run against a
        // half-built font snapshot. The bundled faces are fetched over the
        // network at mount and land moments after the system-font query; when
        // they do, `apply_bundled_font_families` clears the flag and re-enters
        // here, so nothing is dropped — only deferred.
        if self.bundled_fonts_pending {
            return;
        }
        if fonts_flow::complete_pending_detection(&mut self.editor_state) {
            self.mark_dirty();
        }
    }

    pub(crate) fn refresh_missing_fonts_prompt(&mut self) {
        fonts_flow::refresh_prompt(&mut self.editor_state);
        self.mark_dirty();
    }

    pub(crate) fn note_missing_font_supplied(&mut self, row: usize, actual_family: Option<&str>) {
        fonts_flow::note_font_supplied(&mut self.editor_state, row, actual_family);
        self.mark_dirty();
    }
}

#[cfg(test)]
mod tests {
    use op_editor_core::missing_fonts::{MissingFontEntry, MissingFontsPrompt};
    use op_editor_core::{AgentSettingsTab, EditorState, MissingFontSurface};
    use op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel;
    use op_editor_ui::widgets::MissingFontsPanel;
    use op_editor_ui::Point2D;
    use std::sync::Arc;

    use super::super::WidgetHost;

    fn host_with_missing_fonts(families: &[&str]) -> WidgetHost {
        let mut host = WidgetHost::new();
        host.editor_state.editor_ui.missing_fonts_prompt = Some(MissingFontsPrompt {
            entries: families
                .iter()
                .map(|family| MissingFontEntry {
                    family: (*family).to_string(),
                    run_count: 1,
                    mismatch_note: None,
                    resolved: false,
                })
                .collect(),
        });
        host.editor_state.editor_ui.missing_fonts_modal_open = true;
        host
    }

    fn state_with_text(family: &str) -> EditorState {
        let doc: jian_ops_schema::PenDocument = serde_json::from_str(&format!(
            r#"{{"version":"0.8.0","children":[
                {{"type":"text","id":"t1","name":"t","x":0,"y":0,"width":10,"height":10,
                  "content":"hi","fontFamily":"{family}"}}]}}"#
        ))
        .expect("document");
        EditorState::from_document(doc)
    }

    fn host_after_resolving_missing_font(missing: &str, fonts_loaded: bool) -> WidgetHost {
        let mut host = WidgetHost::new();
        host.editor_state = state_with_text(missing);
        if fonts_loaded {
            host.editor_state.editor_ui.system_fonts_loaded = true;
            host.editor_state.editor_ui.system_font_families = Arc::new(vec!["Arial".to_string()]);
        }
        host.arm_missing_fonts_detection();
        assert!(host
            .editor_state
            .apply(op_editor_core::EditorCommand::ReplaceFontFamily {
                from: missing.to_string(),
                to: "Arial".to_string(),
            },));
        if fonts_loaded {
            host.refresh_missing_fonts_prompt();
            assert!(host.editor_state.editor_ui.missing_fonts_prompt.is_none());
        }
        assert!(!host.editor_state.editor_ui.missing_fonts_modal_open);
        host
    }

    fn populate_scrollable_font_list(host: &mut WidgetHost) {
        host.editor_state.editor_ui.system_font_families = Arc::new(
            (0..40)
                .map(|index| format!("System Font {index:02}"))
                .collect(),
        );
    }

    #[test]
    fn choose_font_press_opens_the_shared_picker_for_the_row() {
        let mut host = host_with_missing_fonts(&["Katibeh"]);
        let panel = MissingFontsPanel::for_editor(&host.editor_state).expect("open prompt");
        let rect = panel.rect(1200.0, 800.0);
        let point = Point2D::new(rect.origin.x + rect.size.x - 50.0, rect.origin.y + 90.0);

        let viewport = op_editor_ui::Rect::xywh(0.0, 0.0, 1200.0, 800.0);
        assert!(host.dispatch_missing_fonts_press(rect, viewport, point));
        assert_eq!(
            host.editor_state().editor_ui.font_picker_purpose,
            Some(op_editor_core::FontPickerPurpose::MissingFont {
                row: 0,
                surface: op_editor_core::MissingFontSurface::Prompt,
            })
        );
    }

    #[test]
    fn wheel_scrolls_prompt_and_settings_font_pickers_down() {
        const VIEWPORT_W: f32 = 1200.0;
        const VIEWPORT_H: f32 = 800.0;
        let viewport = op_editor_ui::Rect::xywh(0.0, 0.0, VIEWPORT_W, VIEWPORT_H);

        for surface in [MissingFontSurface::Prompt, MissingFontSurface::Settings] {
            let mut host = host_with_missing_fonts(&["Katibeh"]);
            populate_scrollable_font_list(&mut host);
            host.editor_state
                .editor_ui
                .open_missing_font_picker(0, surface);

            let popup = match surface {
                MissingFontSurface::Prompt => {
                    let panel = MissingFontsPanel::for_editor(&host.editor_state).expect("prompt");
                    let panel_rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
                    let layout = panel
                        .picker_layout(panel_rect, viewport)
                        .expect("prompt picker");
                    assert!(layout.max_scroll > 80.0);
                    layout.popup
                }
                MissingFontSurface::Settings => {
                    let ui = &mut host.editor_state.editor_ui;
                    ui.missing_fonts_modal_open = false;
                    ui.agent_settings_open = true;
                    ui.agent_settings.tab = AgentSettingsTab::Fonts;
                    let panel = AgentSettingsPanel::for_web_editor(&host.editor_state);
                    let panel_rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
                    let layout = panel
                        .font_picker_layout(panel_rect)
                        .expect("settings picker");
                    assert!(layout.max_scroll > 80.0);
                    layout.popup
                }
            };
            let point = Point2D::new(
                popup.origin.x + popup.size.x / 2.0,
                popup.origin.y + popup.size.y / 2.0,
            );
            let zoom_before = host.editor_state.viewport.zoom;

            assert!(host.apply_wheel(point.x, point.y, -80.0, VIEWPORT_W, VIEWPORT_H,));

            assert_eq!(host.editor_state.editor_ui.font_picker.scroll.offset, 80.0);
            assert_eq!(host.editor_state.viewport.zoom, zoom_before);
        }
    }

    #[test]
    fn dismiss_keeps_prompt_data() {
        let mut host = host_with_missing_fonts(&["Katibeh"]);
        let panel = MissingFontsPanel::for_editor(&host.editor_state).expect("open prompt");
        let rect = panel.rect(1200.0, 800.0);
        let point = Point2D::new(
            rect.origin.x + rect.size.x - 70.0,
            rect.origin.y + rect.size.y - 30.0,
        );

        assert!(host.dispatch_missing_fonts_press(
            rect,
            op_editor_ui::Rect::xywh(0.0, 0.0, 1200.0, 800.0),
            point,
        ));
        assert!(!host.editor_state.editor_ui.missing_fonts_modal_open);
        assert!(host.editor_state.editor_ui.missing_fonts_prompt.is_some());
    }

    #[test]
    fn ingest_with_loaded_snapshot_detects_immediately() {
        let mut host = WidgetHost::new();
        host.editor_state.editor_ui.system_fonts_loaded = true;

        host.install_ingested_state(state_with_text("__OpenPencilWebMissingFontTest__"));

        let ui = &host.editor_state.editor_ui;
        assert!(ui.missing_fonts_modal_open);
        assert_eq!(
            ui.missing_fonts_prompt.as_ref().unwrap().entries[0].family,
            "__OpenPencilWebMissingFontTest__"
        );
    }

    #[test]
    fn ingest_without_snapshot_stays_pending_until_query_result_lands() {
        let mut host = WidgetHost::new();

        host.install_ingested_state(state_with_text("__OpenPencilWebDeferredFontTest__"));
        assert!(host.editor_state.editor_ui.missing_fonts_pending_detect);

        host.apply_browser_system_font_families(vec!["Arial".to_string()]);

        let ui = &host.editor_state.editor_ui;
        assert!(!ui.missing_fonts_pending_detect);
        assert!(ui.missing_fonts_modal_open);
        assert_eq!(
            ui.missing_fonts_prompt.as_ref().unwrap().entries[0].family,
            "__OpenPencilWebDeferredFontTest__"
        );
    }

    /// A mount that has armed the bundled-font fetch but not received it yet,
    /// holding a document whose only family is an app-bundled one.
    fn host_awaiting_bundled_fonts(family: &str) -> WidgetHost {
        let mut host = WidgetHost::new();
        host.begin_bundled_font_loading();
        host.install_ingested_state(state_with_text(family));
        assert!(host.editor_state.editor_ui.missing_fonts_pending_detect);
        host
    }

    #[test]
    fn a_system_font_query_cannot_open_the_modal_while_bundled_fonts_are_in_flight() {
        // The browser fetches the bundled faces over the network, so the
        // system-font query routinely wins the race. Completing detection then
        // would accuse Inter of being missing a frame before it registers.
        let mut host = host_awaiting_bundled_fonts("Inter");

        host.apply_browser_system_font_families(vec!["Arial".to_string()]);

        let ui = &host.editor_state.editor_ui;
        assert!(!ui.missing_fonts_modal_open);
        assert!(ui.missing_fonts_prompt.is_none());
        assert!(
            ui.missing_fonts_pending_detect,
            "detection must stay armed so the bundled fonts can complete it"
        );
    }

    #[test]
    fn bundled_fonts_landing_completes_the_held_detection_with_no_prompt() {
        let mut host = host_awaiting_bundled_fonts("Inter");
        host.apply_browser_system_font_families(vec!["Arial".to_string()]);

        host.apply_bundled_font_families(vec!["Inter".to_string()]);

        let ui = &host.editor_state.editor_ui;
        assert!(!ui.missing_fonts_pending_detect);
        assert!(ui.missing_fonts_prompt.is_none());
        assert!(!ui.missing_fonts_modal_open);
    }

    #[test]
    fn a_family_no_bundled_font_covers_still_raises_the_prompt() {
        const MISSING: &str = "__OpenPencilWebUnbundledFontTest__";
        let mut host = host_awaiting_bundled_fonts(MISSING);
        host.apply_browser_system_font_families(vec!["Arial".to_string()]);

        host.apply_bundled_font_families(vec!["Inter".to_string(), "Outfit".to_string()]);

        let ui = &host.editor_state.editor_ui;
        assert!(!ui.missing_fonts_pending_detect);
        assert!(ui.missing_fonts_modal_open);
        assert_eq!(
            ui.missing_fonts_prompt.as_ref().unwrap().entries[0].family,
            MISSING
        );
    }

    #[test]
    fn keyboard_history_navigation_refreshes_missing_fonts_without_reopening_prompt() {
        const MISSING: &str = "__OpenPencilWebUndoMissingFont__";
        let mut host = host_after_resolving_missing_font(MISSING, true);

        assert!(host.apply_undo());
        assert_eq!(
            host.editor_state
                .editor_ui
                .missing_fonts_prompt
                .as_ref()
                .and_then(|prompt| prompt.entries.first())
                .map(|entry| entry.family.as_str()),
            Some(MISSING)
        );
        assert!(!host.editor_state.editor_ui.missing_fonts_modal_open);

        assert!(host.apply_redo());
        assert!(host.editor_state.editor_ui.missing_fonts_prompt.is_none());
        assert!(!host.editor_state.editor_ui.missing_fonts_modal_open);
    }

    #[test]
    fn toolbar_undo_refreshes_missing_fonts_without_reopening_prompt() {
        const MISSING: &str = "__OpenPencilWebToolbarUndoMissingFont__";
        let mut host = host_after_resolving_missing_font(MISSING, true);

        assert!(host.dispatch_toolbar_action(op_editor_ui::widgets::ToolbarAction::Undo));
        assert_eq!(
            host.editor_state
                .editor_ui
                .missing_fonts_prompt
                .as_ref()
                .and_then(|prompt| prompt.entries.first())
                .map(|entry| entry.family.as_str()),
            Some(MISSING)
        );
        assert!(!host.editor_state.editor_ui.missing_fonts_modal_open);

        assert!(host.dispatch_toolbar_action(op_editor_ui::widgets::ToolbarAction::Redo));
        assert!(host.editor_state.editor_ui.missing_fonts_prompt.is_none());
        assert!(!host.editor_state.editor_ui.missing_fonts_modal_open);
    }

    #[test]
    fn undo_keeps_deferred_font_detection_passive_after_query_completes() {
        const MISSING: &str = "__OpenPencilWebDeferredUndoMissingFont__";
        let mut host = host_after_resolving_missing_font(MISSING, false);
        assert!(host.editor_state.editor_ui.missing_fonts_pending_detect);
        assert!(host.editor_state.editor_ui.missing_fonts_pending_open_modal);

        assert!(host.apply_undo());
        assert!(host.editor_state.editor_ui.missing_fonts_pending_detect);
        assert!(!host.editor_state.editor_ui.missing_fonts_pending_open_modal);
        assert!(!host.editor_state.editor_ui.missing_fonts_modal_open);

        host.apply_browser_system_font_families(vec!["Arial".to_string()]);

        let ui = &host.editor_state.editor_ui;
        assert!(!ui.missing_fonts_pending_detect);
        assert_eq!(
            ui.missing_fonts_prompt
                .as_ref()
                .and_then(|prompt| prompt.entries.first())
                .map(|entry| entry.family.as_str()),
            Some(MISSING)
        );
        assert!(!ui.missing_fonts_modal_open);
    }
}
