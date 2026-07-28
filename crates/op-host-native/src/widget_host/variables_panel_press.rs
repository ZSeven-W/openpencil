//! Floating VariablesPanel press dispatch — hit-test + delegation only.
//!
//! Every arm body lives in the shared
//! `op_editor_core::host_variables_transitions` (the web host's twin
//! drives the same functions); row-cell presses stay host-side in
//! `variables_panel_row_press.rs` and the draft commits in
//! `variables_panel_commit.rs`.

use super::WidgetHostNative;
use jian_ops_schema::variable::{VariableKind, VariableScalar};
use op_editor_core::host_variables_transitions as vars_flow;
use op_editor_ui::widgets::variables_panel::{VariablesPanel, VariablesPanelHit};
use op_editor_ui::Point2D;

impl WidgetHostNative {
    pub(in crate::widget_host) fn dispatch_variables_panel_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        if !self.editor_state.editor_ui.variables_panel_open {
            return false;
        }
        let Some(vars_rect) = self.variables_panel_rect(viewport_width, viewport_height) else {
            return false;
        };
        let point = Point2D::new(x, y);
        if !(vars_rect).contains(point) {
            return false;
        }
        let vars = VariablesPanel::for_editor(&self.editor_state);
        let Some(hit) = vars.hit_test(vars_rect, point) else {
            // Blank press inside the panel — commit + blur every
            // text input (row / header drafts included) and fold the
            // open menus away.
            self.blur_text_inputs_on_blank_press();
            self.close_variable_menus();
            self.mark_dirty();
            return true;
        };
        self.editor_state.editor_ui.pressed_button = vars
            .hover_at(vars_rect, point)
            .map(op_editor_core::ButtonPressTarget::VariablesPanel);
        // Any press inside the panel commits the pending property /
        // effect-param / header / row draft first (DOM parity: a
        // mousedown outside an input blurs and commits it). One call up
        // front covers every arm below, including the resize + close
        // arms that used to drop the draft on this host.
        self.commit_property_focus_if_any();
        let edit_intent = matches!(
            &hit,
            VariablesPanelHit::RowMenuRename(_)
                | VariablesPanelHit::RowMenuDelete(_)
                | VariablesPanelHit::ColorSwatch { .. }
                | VariablesPanelHit::ThemeMenuRename(_)
                | VariablesPanelHit::ThemeMenuDelete(_)
                | VariablesPanelHit::AddTheme
                | VariablesPanelHit::AddVariant
                | VariablesPanelHit::VariantMenuRename(_)
                | VariablesPanelHit::VariantMenuDelete(_)
                | VariablesPanelHit::AddVariableColor
                | VariablesPanelHit::AddVariableNumber
                | VariablesPanelHit::AddVariableString
                | VariablesPanelHit::NameCell(_)
                | VariablesPanelHit::ValueCell { .. }
                | VariablesPanelHit::Row(_)
        );
        if edit_intent
            && !self.collab_allows_document_mutation(
                op_editor_core::CollabDocumentMutation::Unsupported(
                    op_editor_core::CollabUnsupportedFeature::VariablesThemes,
                ),
            )
        {
            self.close_variable_menus();
            return true;
        }
        match hit {
            VariablesPanelHit::Resize(edge) => {
                // Edge press arms a resize drag; cursor moves write the
                // live size, release ends it (TS pointer capture).
                self.variables_resize = Some(edge);
                true
            }
            VariablesPanelHit::SearchBox => {
                self.editor_state.editor_ui.variables_search_focus = true;
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
                self.close_variable_menus();
                self.mark_dirty();
                true
            }
            VariablesPanelHit::RowMenuToggle(idx) => self.toggle_variable_row_menu(idx),
            VariablesPanelHit::RowMenuRename(idx) => self.start_variable_row_rename(idx),
            VariablesPanelHit::RowMenuDelete(idx) => self.delete_variable_row(idx),
            VariablesPanelHit::ColorSwatch { row, variant } => {
                self.press_variable_color_swatch(row, variant, x, y)
            }
            VariablesPanelHit::Close => {
                self.editor_state.editor_ui.variables_panel_open = false;
                self.editor_state.editor_ui.variables_panel_hover = None;
                self.close_variable_menus();
                self.mark_dirty();
                true
            }
            VariablesPanelHit::ThemeTab(axis) => {
                vars_flow::select_variable_axis(&mut self.editor_state, axis);
                self.mark_dirty();
                true
            }
            VariablesPanelHit::ToggleThemeMenu(axis) => {
                vars_flow::toggle_theme_menu(&mut self.editor_state.editor_ui, axis);
                self.mark_dirty();
                true
            }
            VariablesPanelHit::ThemeMenuRename(axis) => {
                vars_flow::start_theme_rename(&mut self.editor_state, axis, self.now_ms);
                self.mark_dirty();
                true
            }
            VariablesPanelHit::ThemeMenuDelete(axis) => {
                vars_flow::delete_theme_axis(&mut self.editor_state, axis);
                self.mark_dirty();
                true
            }
            VariablesPanelHit::AddTheme => {
                vars_flow::add_variable_theme(&mut self.editor_state);
                self.mark_dirty();
                true
            }
            VariablesPanelHit::TogglePresetMenu => {
                vars_flow::toggle_preset_menu(&mut self.editor_state.editor_ui);
                self.mark_dirty();
                true
            }
            VariablesPanelHit::AddVariant => {
                vars_flow::add_variable_variant(&mut self.editor_state);
                self.mark_dirty();
                true
            }
            VariablesPanelHit::ToggleVariantMenu(value) => {
                vars_flow::toggle_variant_menu(&mut self.editor_state.editor_ui, value);
                self.mark_dirty();
                true
            }
            VariablesPanelHit::VariantMenuRename(value) => {
                vars_flow::start_variant_rename(&mut self.editor_state, value, self.now_ms);
                self.mark_dirty();
                true
            }
            VariablesPanelHit::VariantMenuDelete(value) => {
                vars_flow::delete_variant_value(&mut self.editor_state, value);
                self.mark_dirty();
                true
            }
            VariablesPanelHit::ToggleAddVariableMenu => {
                vars_flow::toggle_add_variable_menu(&mut self.editor_state.editor_ui);
                self.mark_dirty();
                true
            }
            VariablesPanelHit::AddVariableColor => self.add_variable(
                "color",
                VariableKind::Color,
                VariableScalar::Str("#000000".into()),
            ),
            VariablesPanelHit::AddVariableNumber => {
                self.add_variable("number", VariableKind::Number, VariableScalar::Num(0.0))
            }
            VariablesPanelHit::AddVariableString => self.add_variable(
                "string",
                VariableKind::String,
                VariableScalar::Str("string".into()),
            ),
            VariablesPanelHit::NameCell(idx) => self.press_variable_name_cell(idx),
            VariablesPanelHit::ValueCell { row, variant } => {
                self.press_variable_value_cell(row, variant, x, y)
            }
            VariablesPanelHit::Row(idx) => self.press_variable_row(idx, x, y),
            VariablesPanelHit::AxisChip(idx) => {
                vars_flow::toggle_variable_axis(&mut self.editor_state, idx);
                self.mark_dirty();
                true
            }
            VariablesPanelHit::AxisDropdownItem { axis, value } => {
                vars_flow::select_axis_value(&mut self.editor_state, &axis, &value);
                self.mark_dirty();
                true
            }
        }
    }

    fn add_variable(&mut self, base: &str, kind: VariableKind, default: VariableScalar) -> bool {
        if !self.collab_allows_variables_mutation() {
            return true;
        }
        vars_flow::add_variable(&mut self.editor_state, base, kind, default, self.now_ms);
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn close_variable_menus(&mut self) {
        vars_flow::close_variable_menus(&mut self.editor_state.editor_ui);
    }
}
