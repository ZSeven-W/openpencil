//! Theme-preset dropdown dispatch — the native seam for #20.
//!
//! The dropdown itself is the shared `ThemePresetMenu` widget
//! (op-editor-ui `widgets/variables_preset_menu.rs`). This module
//! anchors it under the variables panel's preset button and routes
//! presses into the `EditorState` preset store + the pending
//! `.optheme` IO drained by op-host-desktop `theme_preset_host.rs`.
//!
//! TS flows mirrored: `variable-theme-manager.tsx:131-164` (save /
//! load / delete / import / export) + the outside-mousedown close at
//! `:53-76`.

use super::WidgetHostNative;
use op_editor_core::editor_ui_state::ThemePresetIo;
use op_editor_core::variables_panel_state::PresetMenuButton;
use op_editor_ui::widgets::{PresetMenuHit, ThemePresetMenu};
use op_editor_ui::{Point2D, Rect};

/// Map a press hit inside the preset dropdown to the persistent hover-row enum
/// (the `Blank` swallow target has no row, so it clears the hover).
fn preset_hit_to_button(hit: PresetMenuHit) -> Option<PresetMenuButton> {
    match hit {
        PresetMenuHit::SaveCurrent => Some(PresetMenuButton::SaveCurrent),
        PresetMenuHit::NameInput => Some(PresetMenuButton::NameInput),
        PresetMenuHit::Load(i) => Some(PresetMenuButton::Load(i)),
        PresetMenuHit::Delete(i) => Some(PresetMenuButton::Delete(i)),
        PresetMenuHit::Import => Some(PresetMenuButton::Import),
        PresetMenuHit::Export => Some(PresetMenuButton::Export),
        PresetMenuHit::Blank => None,
    }
}

impl WidgetHostNative {
    /// Anchor rect of the variables panel's preset button.
    ///
    /// Replicates the panel's private geometry (provenance:
    /// op-editor-ui `widgets/variables_panel.rs` — `preset_rect`
    /// :300-306, `add_theme_rect` :293-298, `theme_tabs_end_x`
    /// :316-322, `theme_tab_advance_width` /
    /// `theme_rename_input_width`, `label_width` :732-736, and the
    /// implicit "Theme-1" tab fallback in `for_editor_at`). The
    /// duplication is hit-test-safe: a drift only offsets the popover
    /// cosmetically since both paint and press use THIS rect.
    fn variables_preset_anchor(&self, panel_rect: Rect) -> Rect {
        let renaming = self
            .editor_state
            .editor_ui
            .variables_theme_rename_axis
            .as_deref();
        let draft = &self.editor_state.ui.property_input_draft;
        let mut x = panel_rect.origin.x + 16.0; // panel PAD_X
        for label in self.panel_theme_tab_labels() {
            x += if renaming == Some(label.as_str()) {
                (panel_label_width(draft, 13.0) + 28.0).max(96.0) + 4.0
            } else {
                panel_label_width(&label, 13.0) + 24.0
            };
        }
        let add_theme_x = x + 6.0; // add-theme button, 28px wide
        Rect {
            origin: Point2D::new(add_theme_x + 28.0 + 8.0, panel_rect.origin.y + 6.0),
            size: Point2D::new(122.0, 32.0),
        }
    }

    /// Theme-axis tab labels as the panel paints them — doc axes plus
    /// the implicit "Theme-1" tab when the doc has variables but no
    /// declared axes (`VariablesPanel::for_editor_at`).
    fn panel_theme_tab_labels(&self) -> Vec<String> {
        let doc = &self.editor_state.doc;
        let mut axes: Vec<String> = doc
            .themes
            .as_ref()
            .map(|themes| themes.keys().cloned().collect())
            .unwrap_or_default();
        let has_rows = doc.variables.as_ref().is_some_and(|vars| !vars.is_empty());
        if axes.is_empty() && has_rows {
            axes.push("Theme-1".to_string());
        }
        axes
    }

    /// The open preset dropdown + its rect, or `None` while closed /
    /// no panel. Shared by paint (overlay) and press (dispatch).
    pub(in crate::widget_host) fn variables_preset_menu_with_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<(ThemePresetMenu, Rect)> {
        if !self.editor_state.editor_ui.variables_preset_menu_open {
            return None;
        }
        let panel_rect = self.variables_panel_rect(viewport_w, viewport_h)?;
        let menu = ThemePresetMenu::for_editor(&self.editor_state);
        let rect = menu.menu_rect(self.variables_preset_anchor(panel_rect));
        Some((menu, rect))
    }

    /// Update the per-row hover for the open preset dropdown on a cursor move.
    /// Returns `true` when the cursor is over the menu (the move is swallowed,
    /// like any top-most overlay).
    pub(in crate::widget_host) fn update_variables_preset_menu_hover(
        &mut self,
        x: f32,
        viewport_w: f32,
        viewport_h: f32,
        y: f32,
    ) -> bool {
        let Some((menu, menu_rect)) = self.variables_preset_menu_with_rect(viewport_w, viewport_h)
        else {
            return false;
        };
        let point = Point2D::new(x, y);
        let new_hover = menu
            .hit_test(menu_rect, point)
            .and_then(preset_hit_to_button);
        if new_hover != self.editor_state.editor_ui.variables_preset_menu_hover {
            self.editor_state.editor_ui.variables_preset_menu_hover = new_hover;
            self.mark_dirty();
        }
        menu_rect.contains(point)
    }

    /// Press routing for the open preset dropdown. Runs BEFORE
    /// `dispatch_variables_panel_press` so the functional rows win
    /// over the panel's stub `TogglePresetMenu` mapping. Returns
    /// `false` when the press should continue to lower layers (an
    /// outside press closes the menu first, mirroring the TS
    /// outside-mousedown handler, EXCEPT on the anchor button whose
    /// panel-side toggle owns the close).
    pub(in crate::widget_host) fn dispatch_variables_preset_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        let Some((menu, rect)) = self.variables_preset_menu_with_rect(viewport_w, viewport_h)
        else {
            return false;
        };
        let point = Point2D::new(x, y);
        let Some(hit) = menu.hit_test(rect, point) else {
            let panel_rect = self.variables_panel_rect(viewport_w, viewport_h);
            let on_anchor = panel_rect
                .is_some_and(|panel| (self.variables_preset_anchor(panel)).contains(point));
            if !on_anchor {
                self.close_preset_menu();
                self.mark_dirty();
            }
            return false;
        };
        match hit {
            PresetMenuHit::SaveCurrent => {
                // Swap the row to the name input
                // (variable-theme-manager.tsx:306-317).
                self.commit_property_focus_if_any();
                self.commit_variables_panel_header_focus_if_any();
                self.commit_variable_row_focus_if_any();
                self.editor_state.editor_ui.variables_preset_name_focus = true;
                self.editor_state.ui.property_input_draft.clear();
                self.editor_state.ui.property_caret_pos = 0;
                self.editor_state.ui.property_draft_select_all = false;
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
            }
            PresetMenuHit::NameInput | PresetMenuHit::Blank => {
                // Clicks inside the popover never close it (TS
                // popover swallows them).
            }
            PresetMenuHit::Load(idx) => {
                // handleLoadPreset — merge + close the menu. One
                // history snapshot per gesture, like the other
                // variables-panel mutations.
                self.commit_property_focus_if_any();
                let snap = self.editor_state.snapshot_for_history();
                if self.editor_state.load_theme_preset(idx) {
                    self.editor_state.history_push_past(snap);
                }
                self.close_preset_menu();
            }
            PresetMenuHit::Delete(idx) => {
                // deletePreset — app-level list, NOT an undoable
                // document edit (TS history never sees it). The menu
                // stays open (TS stopPropagation).
                if let Some(id) = self
                    .editor_state
                    .theme_presets
                    .get(idx)
                    .map(|p| p.id.clone())
                {
                    let _ = self.editor_state.delete_theme_preset(&id);
                }
            }
            PresetMenuHit::Import => {
                // handleImportFromFile closes the menu, then the host
                // runs the file dialog (pending IO drained desktop-side).
                self.close_preset_menu();
                self.editor_state.editor_ui.pending_theme_preset_io = Some(ThemePresetIo::Import);
            }
            PresetMenuHit::Export => {
                self.close_preset_menu();
                self.editor_state.editor_ui.pending_theme_preset_io = Some(ThemePresetIo::Export);
            }
        }
        self.mark_dirty();
        true
    }

    fn close_preset_menu(&mut self) {
        let had_name_focus = self.editor_state.editor_ui.preset_name_input_active();
        let ui = &mut self.editor_state.editor_ui;
        ui.variables_preset_menu_open = false;
        ui.variables_preset_name_focus = false;
        if had_name_focus {
            // The shared draft belonged to the name input — discard
            // (TS outside-mousedown closes without saving).
            self.editor_state.ui.property_input_draft.clear();
            self.editor_state.ui.property_caret_pos = 0;
            self.editor_state.ui.property_draft_select_all = false;
        }
    }

    /// Enter in the preset-name input — TS `handleSavePreset`
    /// (`variable-theme-manager.tsx:131-136`): blank names keep the
    /// input open; a real name saves, clears, and swaps back to the
    /// "Save Current as Preset…" row (menu stays open).
    pub(in crate::widget_host) fn commit_variables_preset_name_if_any(&mut self) -> bool {
        if !self.editor_state.editor_ui.preset_name_input_active() {
            return false;
        }
        let name = self.editor_state.ui.property_input_draft.clone();
        if name.trim().is_empty() {
            return true;
        }
        let now_ms = self.now_ms;
        let _ = self.editor_state.save_theme_preset(&name, now_ms);
        self.editor_state.editor_ui.variables_preset_name_focus = false;
        self.editor_state.ui.property_input_draft.clear();
        self.editor_state.ui.property_caret_pos = 0;
        self.mark_dirty();
        true
    }

    /// Escape in the preset-name input — closes just the input, the
    /// menu stays open (`variable-theme-manager.tsx:299`).
    pub(in crate::widget_host) fn escape_variables_preset_name(&mut self) -> bool {
        if !self.editor_state.editor_ui.preset_name_input_active() {
            return false;
        }
        self.editor_state.editor_ui.variables_preset_name_focus = false;
        self.editor_state.ui.property_input_draft.clear();
        self.editor_state.ui.property_caret_pos = 0;
        self.mark_dirty();
        true
    }
}

/// `variables_panel.rs:732-736` — ASCII chars advance 0.55em, CJK 1em.
fn panel_label_width(text: &str, size: f32) -> f32 {
    text.chars()
        .map(|c| if c.is_ascii() { size * 0.55 } else { size })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use jian_ops_schema::variable::{VariableKind, VariableScalar};
    use op_editor_core::editor_ui_state::ThemePresetIo;

    const VIEWPORT_W: f32 = 1280.0;
    const VIEWPORT_H: f32 = 900.0;

    fn host_with_open_preset_menu(preset_count: usize) -> WidgetHostNative {
        let mut host = WidgetHostNative::new();
        assert!(host.editor_state_mut().create_variable(
            "brand",
            VariableKind::Color,
            VariableScalar::Str("#ff0000".into()),
        ));
        for i in 0..preset_count {
            assert!(host
                .editor_state_mut()
                .save_theme_preset(&format!("kit-{i}"), 100 + i as u64));
        }
        host.editor_state_mut().theme_presets_dirty = false;
        host.editor_state_mut().editor_ui.variables_panel_open = true;
        host.editor_state_mut().editor_ui.variables_preset_menu_open = true;
        host
    }

    fn center(r: Rect) -> (f32, f32) {
        (r.origin.x + r.size.x / 2.0, r.origin.y + r.size.y / 2.0)
    }

    #[test]
    fn preset_menu_rect_sits_under_the_panel_header_and_covers_the_stub() {
        let host = host_with_open_preset_menu(0);
        let panel = host
            .variables_panel_rect(VIEWPORT_W, VIEWPORT_H)
            .expect("panel open");
        let (_, rect) = host
            .variables_preset_menu_with_rect(VIEWPORT_W, VIEWPORT_H)
            .expect("menu open");
        // Anchored below the 44px header + 6px button y + 32 button
        // h + 4 gap.
        assert_eq!(rect.origin.y, panel.origin.y + 6.0 + 32.0 + 4.0);
        assert!(rect.origin.x > panel.origin.x);
        assert_eq!(rect.size.x, 224.0);
        // Tall enough to fully cover the panel's 144px stub menu.
        assert!(rect.size.y > 144.0);
    }

    #[test]
    fn save_row_activates_the_name_input_and_enter_saves() {
        let mut host = host_with_open_preset_menu(0);
        let (menu, rect) = host
            .variables_preset_menu_with_rect(VIEWPORT_W, VIEWPORT_H)
            .expect("menu open");
        // Save row = first row inside the popover.
        let save_point = (rect.origin.x + 20.0, rect.origin.y + 4.0 + 18.0);
        assert_eq!(
            menu.hit_test(rect, Point2D::new(save_point.0, save_point.1)),
            Some(op_editor_ui::widgets::PresetMenuHit::SaveCurrent)
        );
        assert!(host.apply_press(save_point.0, save_point.1, VIEWPORT_W, VIEWPORT_H));
        assert!(host.editor_state().editor_ui.variables_preset_name_focus);

        for c in "Brand kit".chars() {
            assert!(host.apply_text(c));
        }
        assert_eq!(host.editor_state().ui.property_input_draft, "Brand kit");
        assert!(host.apply_send());
        let state = host.editor_state();
        assert_eq!(state.theme_presets.len(), 1);
        assert_eq!(state.theme_presets[0].name, "Brand kit");
        assert!(state.theme_presets_dirty, "persist pump must fire");
        assert!(!state.editor_ui.variables_preset_name_focus);
        assert!(
            state.editor_ui.variables_preset_menu_open,
            "menu stays open after a save (TS keeps the popover)"
        );
    }

    #[test]
    fn blank_name_enter_keeps_the_input_and_escape_discards_it() {
        let mut host = host_with_open_preset_menu(0);
        host.editor_state_mut()
            .editor_ui
            .variables_preset_name_focus = true;
        assert!(host.apply_send());
        assert!(host.editor_state().editor_ui.variables_preset_name_focus);
        assert!(host.editor_state().theme_presets.is_empty());

        assert!(host.apply_text('x'));
        assert!(host.apply_escape());
        let state = host.editor_state();
        assert!(!state.editor_ui.variables_preset_name_focus);
        assert!(state.editor_ui.variables_preset_menu_open);
        assert!(state.ui.property_input_draft.is_empty());
        assert!(state.theme_presets.is_empty());
    }

    #[test]
    fn load_row_merges_the_preset_and_pushes_one_undo_step() {
        let mut host = host_with_open_preset_menu(1);
        // Re-color the live variable so the preset differs.
        assert!(host
            .editor_state_mut()
            .set_variable_color("brand", "#0000ff"));
        let depth_before = host.editor_state().history.past.len();
        let (menu, rect) = host
            .variables_preset_menu_with_rect(VIEWPORT_W, VIEWPORT_H)
            .expect("menu open");
        let row = rect.origin.y + 4.0 + 36.0 + 9.0 + 15.0; // first list row center
        assert_eq!(
            menu.hit_test(rect, Point2D::new(rect.origin.x + 20.0, row)),
            Some(op_editor_ui::widgets::PresetMenuHit::Load(0))
        );
        assert!(host.apply_press(rect.origin.x + 20.0, row, VIEWPORT_W, VIEWPORT_H));
        let state = host.editor_state();
        assert_eq!(
            state.resolve_variable("brand"),
            Some(&VariableScalar::Str("#ff0000".into())),
            "preset value overwrote the live one"
        );
        assert_eq!(state.history.past.len(), depth_before + 1);
        assert!(!state.editor_ui.variables_preset_menu_open, "load closes");
    }

    #[test]
    fn delete_row_removes_the_preset_without_history() {
        let mut host = host_with_open_preset_menu(2);
        let depth_before = host.editor_state().history.past.len();
        let (menu, rect) = host
            .variables_preset_menu_with_rect(VIEWPORT_W, VIEWPORT_H)
            .expect("menu open");
        let row_y = rect.origin.y + 4.0 + 36.0 + 9.0 + 15.0;
        let trash_x = rect.origin.x + rect.size.x - 12.0 - 9.0;
        assert_eq!(
            menu.hit_test(rect, Point2D::new(trash_x, row_y)),
            Some(op_editor_ui::widgets::PresetMenuHit::Delete(0))
        );
        assert!(host.apply_press(trash_x, row_y, VIEWPORT_W, VIEWPORT_H));
        let state = host.editor_state();
        assert_eq!(state.theme_presets.len(), 1);
        assert_eq!(state.theme_presets[0].name, "kit-1");
        assert!(state.theme_presets_dirty);
        assert_eq!(state.history.past.len(), depth_before);
        assert!(state.editor_ui.variables_preset_menu_open, "menu stays");
    }

    #[test]
    fn import_and_export_rows_raise_pending_io_and_close_the_menu() {
        for (pick_export, expected) in [
            (false, ThemePresetIo::Import),
            (true, ThemePresetIo::Export),
        ] {
            let mut host = host_with_open_preset_menu(0);
            let (menu, rect) = host
                .variables_preset_menu_with_rect(VIEWPORT_W, VIEWPORT_H)
                .expect("menu open");
            // Rows from the bottom: export, then import above it.
            let y =
                rect.origin.y + rect.size.y - 4.0 - if pick_export { 18.0 } else { 36.0 + 18.0 };
            let hit = menu.hit_test(rect, Point2D::new(rect.origin.x + 20.0, y));
            assert_eq!(
                hit,
                Some(if pick_export {
                    op_editor_ui::widgets::PresetMenuHit::Export
                } else {
                    op_editor_ui::widgets::PresetMenuHit::Import
                })
            );
            assert!(host.apply_press(rect.origin.x + 20.0, y, VIEWPORT_W, VIEWPORT_H));
            let state = host.editor_state();
            assert_eq!(state.editor_ui.pending_theme_preset_io, Some(expected));
            assert!(!state.editor_ui.variables_preset_menu_open);
        }
    }

    #[test]
    fn outside_press_closes_the_menu_and_falls_through() {
        let mut host = host_with_open_preset_menu(0);
        let (_, rect) = host
            .variables_preset_menu_with_rect(VIEWPORT_W, VIEWPORT_H)
            .expect("menu open");
        // A point on the bare canvas well below the menu.
        let _ = host.apply_press(
            rect.origin.x + 10.0,
            rect.origin.y + rect.size.y + 500.0,
            VIEWPORT_W,
            VIEWPORT_H,
        );
        assert!(
            !host.editor_state().editor_ui.variables_preset_menu_open,
            "outside mousedown closes the popover (TS document handler)"
        );
    }

    #[test]
    fn anchor_press_does_not_double_toggle() {
        let mut host = host_with_open_preset_menu(0);
        let panel = host
            .variables_panel_rect(VIEWPORT_W, VIEWPORT_H)
            .expect("panel open");
        let anchor = host.variables_preset_anchor(panel);
        let (ax, ay) = center(anchor);
        // The press lands on the panel's own preset-button toggle —
        // it must CLOSE the menu (not close-then-reopen).
        let _ = host.apply_press(ax, ay, VIEWPORT_W, VIEWPORT_H);
        assert!(
            !host.editor_state().editor_ui.variables_preset_menu_open,
            "clicking the preset button while open closes the menu"
        );
    }
}
