//! Native missing-font modal dispatch and detection lifecycle.
//!
//! The modal's scroll + press dispatch and the detection/refresh
//! bookkeeping live in the shared
//! `op_editor_ui::widgets::missing_fonts_flow` (driven by the web host
//! too); this file keeps the native platform arms — synchronous system
//! font enumeration through `ensure_system_fonts_loaded` — plus the
//! `mark_dirty` tails.

use super::WidgetHostNative;
use op_editor_ui::widgets::missing_fonts_flow as fonts_flow;
use op_editor_ui::widgets::{MissingFontsHit, MissingFontsPanel};
use op_editor_ui::{Point2D, Rect};

impl WidgetHostNative {
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

    /// Route a press to the top-most missing-font modal.
    pub(in crate::widget_host) fn dispatch_missing_fonts_press(
        &mut self,
        panel_rect: Rect,
        viewport_rect: Rect,
        point: Point2D,
    ) -> bool {
        let hit = MissingFontsPanel::for_editor(&self.editor_state)
            .map(|panel| panel.hit_test(panel_rect, viewport_rect, point));
        if matches!(hit, Some(MissingFontsHit::SelectFont(_)))
            && !self.collab_allows_document_mutation(
                op_editor_core::CollabDocumentMutation::Unsupported(
                    op_editor_core::CollabUnsupportedFeature::Typography,
                ),
            )
        {
            return true;
        }
        if !fonts_flow::press(&mut self.editor_state, panel_rect, viewport_rect, point) {
            return false;
        }
        self.mark_dirty();
        true
    }

    /// Drain the expected-family import request raised by either prompt surface.
    pub fn take_missing_fonts_import_row(&mut self) -> Option<usize> {
        self.editor_state.editor_ui.missing_fonts_import_row.take()
    }

    /// Enumerate system fonts through the property picker's canonical routine,
    /// then detect missing document families. The pending flag remains the
    /// fallback if enumeration ever becomes asynchronous.
    pub fn arm_missing_fonts_detection(&mut self) {
        if !self.editor_state.editor_ui.system_fonts_loaded {
            fonts_flow::arm_pending_detection(&mut self.editor_state, true);
            self.ensure_system_fonts_loaded();
        }
        if self.editor_state.editor_ui.system_fonts_loaded {
            fonts_flow::replace_data(&mut self.editor_state, true);
            self.mark_dirty();
        }
    }

    /// Recompute the Settings Fonts-tab data without opening the one-shot modal.
    pub(in crate::widget_host) fn refresh_missing_fonts_for_settings(&mut self) {
        if !self.editor_state.editor_ui.system_fonts_loaded {
            fonts_flow::arm_pending_detection(&mut self.editor_state, false);
            self.ensure_system_fonts_loaded();
        }
        if self.editor_state.editor_ui.system_fonts_loaded {
            fonts_flow::replace_data(&mut self.editor_state, false);
            self.mark_dirty();
        }
    }

    /// Recompute missing-font data after undo/redo without resurrecting a
    /// dismissed one-shot prompt. If the prompt is already visible, refreshing
    /// its rows leaves it visible.
    pub(in crate::widget_host) fn refresh_missing_fonts_after_history_change(&mut self) {
        if !self.editor_state.editor_ui.system_fonts_loaded {
            let open_modal = self.editor_state.editor_ui.missing_fonts_modal_open;
            fonts_flow::arm_pending_detection(&mut self.editor_state, open_modal);
            self.ensure_system_fonts_loaded();
        }
        if self.editor_state.editor_ui.system_fonts_loaded {
            fonts_flow::clear_pending_detection(&mut self.editor_state);
            self.refresh_missing_fonts_prompt();
        }
    }

    pub(in crate::widget_host) fn refresh_missing_fonts_after_document_change(&mut self) {
        if fonts_flow::settings_fonts_open(&self.editor_state) {
            self.refresh_missing_fonts_for_settings();
        } else {
            self.arm_missing_fonts_detection();
        }
    }

    /// Finish a deferred font scan using the open/silent intent captured when
    /// it was scheduled.
    pub fn complete_pending_missing_fonts_detection(&mut self) {
        if fonts_flow::complete_pending_detection(&mut self.editor_state) {
            self.mark_dirty();
        }
    }

    /// Reconcile existing rows against the latest system/imported snapshots.
    pub fn refresh_missing_fonts_prompt(&mut self) {
        fonts_flow::refresh_prompt(&mut self.editor_state);
        self.mark_dirty();
    }

    /// Record whether the supplied file declared the row's expected family,
    /// then refresh resolution from the live imported-font snapshot.
    pub fn note_missing_font_supplied(&mut self, row: usize, actual_family: Option<&str>) {
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

    use super::super::WidgetHostNative;

    fn host_with_missing_fonts(families: &[&str]) -> WidgetHostNative {
        let mut host = WidgetHostNative::new();
        host.editor_state_mut().editor_ui.missing_fonts_prompt = Some(MissingFontsPrompt {
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
        host.editor_state_mut().editor_ui.missing_fonts_modal_open = true;
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

    fn host_after_resolving_missing_font(missing: &str) -> WidgetHostNative {
        let mut state = state_with_text(missing);
        state.editor_ui.system_fonts_loaded = true;
        state.editor_ui.system_font_families = Arc::new(vec!["Arial".to_string()]);
        let mut host = WidgetHostNative::new();
        *host.editor_state_mut() = state;
        host.arm_missing_fonts_detection();
        assert!(host
            .editor_state_mut()
            .apply(op_editor_core::EditorCommand::ReplaceFontFamily {
                from: missing.to_string(),
                to: "Arial".to_string(),
            },));
        host.refresh_missing_fonts_prompt();
        assert!(host.editor_state().editor_ui.missing_fonts_prompt.is_none());
        assert!(!host.editor_state().editor_ui.missing_fonts_modal_open);
        host
    }

    fn populate_scrollable_font_list(host: &mut WidgetHostNative) {
        host.editor_state_mut().editor_ui.system_font_families = Arc::new(
            (0..40)
                .map(|index| format!("System Font {index:02}"))
                .collect(),
        );
    }

    #[test]
    fn choose_font_press_opens_the_shared_picker_for_the_row() {
        let mut host = host_with_missing_fonts(&["Katibeh"]);
        let panel = MissingFontsPanel::for_editor(host.editor_state()).expect("open prompt");
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
    fn trackpad_pan_scrolls_prompt_and_settings_font_pickers_down() {
        const VIEWPORT_W: f32 = 1200.0;
        const VIEWPORT_H: f32 = 800.0;
        let viewport = op_editor_ui::Rect::xywh(0.0, 0.0, VIEWPORT_W, VIEWPORT_H);

        for surface in [MissingFontSurface::Prompt, MissingFontSurface::Settings] {
            let mut host = host_with_missing_fonts(&["Katibeh"]);
            populate_scrollable_font_list(&mut host);
            host.editor_state_mut()
                .editor_ui
                .open_missing_font_picker(0, surface);

            let popup = match surface {
                MissingFontSurface::Prompt => {
                    let panel = MissingFontsPanel::for_editor(host.editor_state()).expect("prompt");
                    let panel_rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
                    let layout = panel
                        .picker_layout(panel_rect, viewport)
                        .expect("prompt picker");
                    assert!(layout.max_scroll > 80.0);
                    layout.popup
                }
                MissingFontSurface::Settings => {
                    let ui = &mut host.editor_state_mut().editor_ui;
                    ui.missing_fonts_modal_open = false;
                    ui.agent_settings_open = true;
                    ui.agent_settings.tab = AgentSettingsTab::Fonts;
                    let panel = AgentSettingsPanel::for_editor(host.editor_state());
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
            let pan_before = (
                host.editor_state().viewport.pan_x,
                host.editor_state().viewport.pan_y,
            );

            assert!(host.apply_pan_gesture(point.x, point.y, 0.0, -80.0, VIEWPORT_W, VIEWPORT_H,));

            assert_eq!(
                host.editor_state().editor_ui.font_picker.scroll.offset,
                80.0
            );
            assert_eq!(
                (
                    host.editor_state().viewport.pan_x,
                    host.editor_state().viewport.pan_y,
                ),
                pan_before
            );
        }
    }

    #[test]
    fn wheel_scrolls_long_prompt_rows_without_moving_the_canvas() {
        const VIEWPORT_W: f32 = 600.0;
        const VIEWPORT_H: f32 = 400.0;
        let names = (0..20).map(|index| format!("F{index}")).collect::<Vec<_>>();
        let refs = names.iter().map(String::as_str).collect::<Vec<_>>();
        let mut host = host_with_missing_fonts(&refs);
        let panel = MissingFontsPanel::for_editor(host.editor_state()).expect("prompt");
        let panel_rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
        let rows = panel.rows_rect(panel_rect);
        let point = Point2D::new(rows.origin.x + 20.0, rows.origin.y + 20.0);
        let pan_before = (
            host.editor_state().viewport.pan_x,
            host.editor_state().viewport.pan_y,
        );

        assert!(host.apply_wheel(point.x, point.y, -80.0, VIEWPORT_W, VIEWPORT_H));
        assert_eq!(
            host.editor_state().editor_ui.missing_fonts_scroll.offset,
            80.0
        );
        assert_eq!(
            (
                host.editor_state().viewport.pan_x,
                host.editor_state().viewport.pan_y
            ),
            pan_before
        );
    }

    #[test]
    fn dismiss_press_closes_modal_but_keeps_data_for_the_tab() {
        let mut host = host_with_missing_fonts(&["Katibeh"]);
        let panel = MissingFontsPanel::for_editor(host.editor_state()).expect("open prompt");
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
        assert!(!host.editor_state().editor_ui.missing_fonts_modal_open);
        assert!(host.editor_state().editor_ui.missing_fonts_prompt.is_some());
    }

    #[test]
    fn install_imported_state_enumerates_then_detects_missing_fonts() {
        let mut host = WidgetHostNative::new();
        assert!(!host.editor_state().editor_ui.system_fonts_loaded);

        host.install_imported_state(state_with_text("__OpenPencilMissingFontTest__"));

        let ui = &host.editor_state().editor_ui;
        assert!(ui.system_fonts_loaded);
        assert!(!ui.missing_fonts_pending_detect);
        assert!(ui.missing_fonts_modal_open);
        assert_eq!(
            ui.missing_fonts_prompt.as_ref().unwrap().entries[0].family,
            "__OpenPencilMissingFontTest__"
        );
    }

    #[test]
    fn settings_refresh_recomputes_data_without_opening_modal() {
        let mut host = WidgetHostNative::new();
        *host.editor_state_mut() = state_with_text("__OpenPencilSettingsMissingFontTest__");
        host.editor_state_mut().editor_ui.system_fonts_loaded = true;

        host.refresh_missing_fonts_for_settings();

        let ui = &host.editor_state().editor_ui;
        assert!(ui.missing_fonts_prompt.is_some());
        assert!(!ui.missing_fonts_modal_open);
    }

    #[test]
    fn refresh_discards_stale_css_stack_and_generic_rows() {
        let doc: jian_ops_schema::PenDocument = serde_json::from_value(serde_json::json!({
            "version": "1.0.0",
            "children": [
                {
                    "type": "text",
                    "id": "stack",
                    "content": [{
                        "text": "NOVA",
                        "fontFamily": "Inter,ui-sans-serif,system-ui,-apple-system,\"PingFang SC\",sans-serif"
                    }],
                    "fontFamily": "Inter,ui-sans-serif,system-ui,-apple-system,\"PingFang SC\",sans-serif"
                },
                {
                    "type": "text",
                    "id": "generic",
                    "content": "2",
                    "fontFamily": "sans-serif"
                }
            ]
        }))
        .expect("document");
        let mut host = WidgetHostNative::new();
        *host.editor_state_mut() = EditorState::from_document(doc);
        {
            let ui = &mut host.editor_state_mut().editor_ui;
            ui.system_fonts_loaded = true;
            ui.system_font_families = Arc::new(vec!["PingFang SC".into()]);
            ui.bundled_font_families = Arc::new(vec!["Inter".into()]);
            ui.missing_fonts_prompt = Some(MissingFontsPrompt {
                entries: vec![
                    MissingFontEntry {
                        family:
                            "Inter,ui-sans-serif,system-ui,-apple-system,\"PingFang SC\",sans-serif"
                                .into(),
                        run_count: 36,
                        mismatch_note: None,
                        resolved: false,
                    },
                    MissingFontEntry {
                        family: "sans-serif".into(),
                        run_count: 1,
                        mismatch_note: None,
                        resolved: false,
                    },
                ],
            });
            ui.missing_fonts_modal_open = true;
        }

        host.refresh_missing_fonts_prompt();

        let ui = &host.editor_state().editor_ui;
        assert!(ui.missing_fonts_prompt.is_none());
        assert!(!ui.missing_fonts_modal_open);
    }

    #[test]
    fn choosing_system_font_rewrites_document_and_is_undoable() {
        let mut state = state_with_text("__OpenPencilMissingReplacementTest__");
        state.editor_ui.system_fonts_loaded = true;
        state.editor_ui.system_font_families = std::sync::Arc::new(vec!["Arial".to_string()]);
        state.editor_ui.font_import_supported = true;
        let mut host = WidgetHostNative::new();
        *host.editor_state_mut() = state;
        host.arm_missing_fonts_detection();

        let viewport = op_editor_ui::Rect::xywh(0.0, 0.0, 1200.0, 800.0);
        let panel = MissingFontsPanel::for_editor(host.editor_state()).expect("prompt");
        let panel_rect = panel.rect(1200.0, 800.0);
        let trigger = Point2D::new(
            panel_rect.origin.x + panel_rect.size.x - 50.0,
            panel_rect.origin.y + 90.0,
        );
        assert!(host.dispatch_missing_fonts_press(panel_rect, viewport, trigger));

        let panel = MissingFontsPanel::for_editor(host.editor_state()).expect("picker");
        let entries = panel.picker_entries();
        let arial = entries
            .iter()
            .position(|entry| entry.family == "Arial")
            .expect("Arial entry");
        let layout = panel
            .picker_layout(panel_rect, viewport)
            .expect("picker layout");
        let row = layout
            .rows
            .iter()
            .find_map(|(row, rect)| matches!(row, op_editor_ui::widgets::property_panel_typography::FontPickerRow::Entry(index) if *index == arial).then_some(*rect))
            .expect("Arial row");
        let point = Point2D::new(row.origin.x + 20.0, row.origin.y + row.size.y / 2.0);
        assert!(host.dispatch_missing_fonts_press(panel_rect, viewport, point));

        let plan = jian_ops_schema::font_plan::FontPlan::scan(&host.editor_state().doc);
        assert!(plan.families().any(|(family, _)| family == "Arial"));
        assert!(host.editor_state().editor_ui.missing_fonts_prompt.is_none());
        assert!(host.apply_undo());
        let plan = jian_ops_schema::font_plan::FontPlan::scan(&host.editor_state().doc);
        assert!(plan
            .families()
            .any(|(family, _)| family == "__OpenPencilMissingReplacementTest__"));
        assert!(
            !host.editor_state().editor_ui.missing_fonts_modal_open,
            "undo must refresh missing-font data without reopening the dismissed prompt"
        );
        assert_eq!(
            host.editor_state()
                .editor_ui
                .missing_fonts_prompt
                .as_ref()
                .and_then(|prompt| prompt.entries.first())
                .map(|entry| entry.family.as_str()),
            Some("__OpenPencilMissingReplacementTest__")
        );

        assert!(host.apply_redo());
        let plan = jian_ops_schema::font_plan::FontPlan::scan(&host.editor_state().doc);
        assert!(plan.families().any(|(family, _)| family == "Arial"));
        assert!(host.editor_state().editor_ui.missing_fonts_prompt.is_none());
        assert!(!host.editor_state().editor_ui.missing_fonts_modal_open);
    }

    #[test]
    fn toolbar_history_navigation_refreshes_missing_fonts_without_reopening_prompt() {
        const MISSING: &str = "__OpenPencilToolbarUndoMissingFont__";
        let mut host = host_after_resolving_missing_font(MISSING);

        assert!(host.dispatch_toolbar_action(op_editor_ui::widgets::ToolbarAction::Undo));
        assert_eq!(
            host.editor_state()
                .editor_ui
                .missing_fonts_prompt
                .as_ref()
                .and_then(|prompt| prompt.entries.first())
                .map(|entry| entry.family.as_str()),
            Some(MISSING)
        );
        assert!(!host.editor_state().editor_ui.missing_fonts_modal_open);
        assert!(
            !host
                .editor_state()
                .editor_ui
                .missing_fonts_pending_open_modal
        );

        assert!(host.dispatch_toolbar_action(op_editor_ui::widgets::ToolbarAction::Redo));
        assert!(host.editor_state().editor_ui.missing_fonts_prompt.is_none());
        assert!(!host.editor_state().editor_ui.missing_fonts_modal_open);
    }
}
