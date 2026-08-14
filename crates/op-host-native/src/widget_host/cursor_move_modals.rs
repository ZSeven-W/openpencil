//! `apply_cursor_move` tier 1 — modal / top-most overlay cursor owners.
//!
//! Runs before every other tier: while one of these surfaces is up it owns
//! the cursor outright, so lower-layer hover must not fire beneath its
//! scrim. Order inside the helper is the paint Z-order and is load-bearing.

use super::WidgetHostNative;
use op_editor_ui::widgets::cursor_hover_flow as hover_flow;
use op_editor_ui::Point2D;

impl WidgetHostNative {
    /// `None` — no modal owns the cursor, fall through to the next tier.
    pub(in crate::widget_host) fn cursor_move_modal_tiers(
        &mut self,
        x: f32,
        y: f32,
    ) -> Option<bool> {
        // In-flight VariablesPanel edge resize — owns the cursor.
        if self.variables_resize.is_some()
            && self.apply_variables_panel_resize(x, y, self.last_viewport_w, self.last_viewport_h)
        {
            return Some(true);
        }
        // Missing-fonts modal — owns the cursor while open. Hover the
        // per-row choose-file buttons + the dismiss action.
        if self.editor_state.editor_ui.missing_fonts_modal_open {
            let changed = hover_flow::missing_fonts_modal_hover(
                &mut self.editor_state,
                self.last_viewport_w,
                self.last_viewport_h,
                Point2D::new(x, y),
            );
            if changed {
                self.mark_dirty();
            }
            return Some(changed);
        }
        // Post-import HTML diagnostics notice — tints its own buttons and
        // owns the cursor only while it is under the card, so hovers below
        // keep working around it.
        if self.editor_state.editor_ui.html_import_diagnostics_open
            && self.update_html_import_diagnostics_hover(
                x,
                y,
                self.last_viewport_w,
                self.last_viewport_h,
            )
        {
            return Some(true);
        }
        if self.editor_state.editor_ui.agent_settings_open {
            return Some(self.update_agent_settings_hover(x, y));
        }
        // Modal export dialog — owns the cursor while open. Update its
        // per-button hover wash (format / scale / cancel / export) and
        // swallow the move so lower-layer hovers don't fire beneath the
        // scrim.
        if self.editor_state.editor_ui.export_dialog_open {
            use op_editor_ui::widgets::ExportDialog;
            let dlg = ExportDialog::centered(self.last_viewport_w, self.last_viewport_h);
            let new_hover = dlg
                .hit_test(Point2D::new(x, y))
                .map(op_editor_ui::widgets::editor_state_ext::export_dialog_button);
            let changed = new_hover != self.editor_state.editor_ui.export_dialog_hover;
            if changed {
                self.editor_state.editor_ui.export_dialog_hover = new_hover;
                self.mark_dirty();
            }
            return Some(changed);
        }
        // Modal Figma-import dialog — owns the cursor while open. Hover
        // the close `✕` + the browse drop-zone.
        if self.editor_state.editor_ui.figma_import_open {
            use op_editor_ui::widgets::figma_import::FigmaImportModal;
            let modal = FigmaImportModal::for_editor(&self.editor_state);
            let panel = modal.rect(self.last_viewport_w, self.last_viewport_h);
            let new_hover = op_editor_ui::widgets::editor_state_ext::figma_import_button(
                modal.hit_test(panel, Point2D::new(x, y)),
            );
            let changed = new_hover != self.editor_state.editor_ui.figma_import_hover;
            if changed {
                self.editor_state.editor_ui.figma_import_hover = new_hover;
                self.mark_dirty();
            }
            return Some(changed);
        }
        // Sign-in modal — owns the cursor while open. Hover the close
        // `✕` + the primary sign-in button.
        if (self.editor_state.editor_ui.account_ui_available
            || self.editor_state.editor_ui.touch_chrome())
            && self.editor_state.editor_ui.login_modal_open
        {
            let changed = hover_flow::login_modal_hover(
                &mut self.editor_state,
                self.last_viewport_w,
                self.last_viewport_h,
                Point2D::new(x, y),
            );
            if changed {
                self.mark_dirty();
            }
            return Some(changed);
        }
        // Signed-in account dropdown — owns the cursor while open.
        if self.editor_state.editor_ui.account_ui_available
            && self.editor_state.editor_ui.account_menu_open
        {
            let changed = hover_flow::account_menu_hover(
                &mut self.editor_state,
                self.last_viewport_w,
                Point2D::new(x, y),
            );
            if changed {
                self.mark_dirty();
            }
            return Some(changed);
        }
        if let Some(state) = self.editor_state.ui.color_picker.clone() {
            if let Some(kind) = state.drag {
                if !self.collab_allows_color_picker_mutation() {
                    self.editor_state.color_picker_set_drag(None);
                    return Some(true);
                }
                use op_editor_core::ui_draft::ColorPickerDrag;
                use op_editor_ui::widgets::color_picker::ColorPicker;
                let picker = ColorPicker::for_state(&self.editor_state, state.clone());
                let panel = picker.rect(self.last_viewport_w, self.last_viewport_h);
                let point = Point2D::new(x, y);
                // Instance-write redirect (GAP #10) — picker drags on
                // a Ref anchor route the live colour into descendants.
                let instance_scope = self.editor_state.begin_instance_write_for_anchor();
                let is_instance = instance_scope.is_some();
                match kind {
                    ColorPickerDrag::SvBox => {
                        let (s, v) = picker.sv_at(panel, point);
                        let _ = self.editor_state.color_picker_set_hsv(state.hue, s, v);
                    }
                    ColorPickerDrag::HueSlider => {
                        let h = picker.hue_at(panel, point);
                        let _ = self
                            .editor_state
                            .color_picker_set_hsv(h, state.sat, state.val);
                    }
                }
                if let Some(scope) = instance_scope {
                    self.editor_state.finish_instance_write(scope);
                }
                // A solid Fill/Stroke change on a concrete anchor touches
                // no layout, so patch the resolved scene paint in place
                // rather than re-running taffy + reshape on every drag
                // frame (mirrors the node-drag `translate_nodes` fast
                // path). Variable-mode, instance redirects, gradient-stop /
                // effect targets, and new strokes fall back to a rebuild.
                if !self.try_patch_color_drag(is_instance) {
                    self.mark_dirty();
                }
                return Some(true);
            }
            // Picker open but not dragging — it is a top-most overlay, so a move
            // over its panel must be swallowed (canvas hover must not bleed
            // through underneath it). Matches the design-md / modal behaviour.
            let picker = op_editor_ui::widgets::color_picker::ColorPicker::for_state(
                &self.editor_state,
                state.clone(),
            );
            let panel = picker.rect(self.last_viewport_w, self.last_viewport_h);
            if panel.contains(Point2D::new(x, y)) {
                self.clear_lower_overlay_hover();
                return Some(true);
            }
        }
        // Live preview owns canvas cursor moves (hover + drag into the
        // runtime). Runs below the modal guards (they own the cursor
        // while open); floating overlays are excluded inside
        // `preview_dispatch_move` via `over_topmost_panel`, and
        // off-canvas moves fall through so top-bar hover still works
        // while previewing.
        if self.preview.is_some() {
            // Neither switcher paints while a deck is presenting, so only the
            // presenting toolbar tracks the cursor there.
            if self.preview_slideshow_active() {
                self.slideshow_toolbar_hover(x, y, self.last_viewport_w, self.last_viewport_h);
            } else {
                self.screen_switcher_hover(x, y, self.last_viewport_w, self.last_viewport_h);
                self.preview_switcher_hover(x, y, self.last_viewport_w, self.last_viewport_h);
            }
            if self.preview_dispatch_move(x, y) {
                return Some(true);
            }
        }
        None
    }
}
