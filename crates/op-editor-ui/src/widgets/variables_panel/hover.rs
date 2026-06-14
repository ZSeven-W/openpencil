use super::*;
use op_editor_core::variables_panel_state::VariablesPanelButton;

impl VariablesPanel {
    /// Map a cursor point to the hover key that paints the same target
    /// as the click hit-test.
    pub fn hover_at(&self, rect: Rect, point: Point2D) -> Option<VariablesPanelButton> {
        use VariablesPanelButton as B;
        match self.hit_test(rect, point)? {
            VariablesPanelHit::Close => Some(B::Close),
            VariablesPanelHit::ThemeTab(axis) | VariablesPanelHit::ToggleThemeMenu(axis) => self
                .theme_tab_labels()
                .iter()
                .position(|label| *label == axis)
                .map(B::ThemeTab),
            VariablesPanelHit::ThemeMenuRename(_) => Some(B::ThemeMenuItem(0)),
            VariablesPanelHit::ThemeMenuDelete(_) => Some(B::ThemeMenuItem(1)),
            VariablesPanelHit::AddTheme => Some(B::AddTheme),
            VariablesPanelHit::TogglePresetMenu => Some(B::PresetMenu),
            VariablesPanelHit::AddVariant => Some(B::AddVariant),
            VariablesPanelHit::ToggleVariantMenu(value) => self
                .variant_column_labels()
                .iter()
                .position(|label| *label == value)
                .map(B::VariantHeader),
            VariablesPanelHit::VariantMenuRename(_) => Some(B::VariantMenuItem(0)),
            VariablesPanelHit::VariantMenuDelete(_) => Some(B::VariantMenuItem(1)),
            VariablesPanelHit::ToggleAddVariableMenu => Some(B::AddVariable),
            VariablesPanelHit::AddVariableColor => Some(B::AddVariableMenuItem(0)),
            VariablesPanelHit::AddVariableNumber => Some(B::AddVariableMenuItem(1)),
            VariablesPanelHit::AddVariableString => Some(B::AddVariableMenuItem(2)),
            VariablesPanelHit::NameCell(i) | VariablesPanelHit::Row(i) => Some(B::Row(i)),
            VariablesPanelHit::ValueCell { row, .. } => Some(B::Row(row)),
            VariablesPanelHit::ColorSwatch { row, .. } => Some(B::Row(row)),
            VariablesPanelHit::RowMenuToggle(i) => Some(B::RowMenuButton(i)),
            // Menu rows key by position (0 = Rename, 1 = Delete) so
            // `paint_menus` can wash the hovered row.
            VariablesPanelHit::RowMenuRename(_) => Some(B::RowMenuItem(0)),
            VariablesPanelHit::RowMenuDelete(_) => Some(B::RowMenuItem(1)),
            VariablesPanelHit::AxisChip(i) => Some(B::AxisChip(i)),
            VariablesPanelHit::AxisDropdownItem { axis, value } => self
                .axis_values(&axis)
                .and_then(|values| values.iter().position(|v| *v == value))
                .map(B::DropdownItem),
            VariablesPanelHit::SearchBox => Some(B::SearchBox),
            VariablesPanelHit::Resize(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Color;
    use jian_ops_schema::variable::{VariableKind, VariableScalar};
    use op_editor_core::variables_panel_state::VariablesPanelButton;
    use op_editor_core::ButtonPressTarget;

    #[derive(Default)]
    struct HoverBackend {
        round_fills: Vec<(Rect, Color)>,
    }

    impl crate::RenderBackend for HoverBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &crate::TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, rect: Rect, _: f32, color: Color) {
            self.round_fills.push((rect, color));
        }
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    fn themed_state() -> EditorState {
        let mut state = EditorState::new();
        state
            .doc
            .themes
            .get_or_insert_with(Default::default)
            .insert("Theme-1".into(), vec!["Default".into(), "Variant-1".into()]);
        state
            .ui
            .variables
            .active_theme
            .insert("Theme-1".into(), "Default".into());
        state.create_variable("spacing", VariableKind::Number, VariableScalar::Num(8.0));
        state
    }

    #[test]
    fn hover_at_maps_clickable_header_targets() {
        let panel = VariablesPanel::for_editor(&themed_state());
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(VARIABLES_PANEL_WIDTH, panel.intrinsic_height()),
        };

        assert_eq!(
            panel.hover_at(
                rect,
                panel.theme_tab_rect(rect, 0).origin + Point2D::new(8.0, 8.0)
            ),
            Some(VariablesPanelButton::ThemeTab(0))
        );
        assert_eq!(
            panel.hover_at(
                rect,
                panel.add_theme_rect(rect).origin + Point2D::new(8.0, 8.0)
            ),
            Some(VariablesPanelButton::AddTheme)
        );
        assert_eq!(
            panel.hover_at(
                rect,
                panel.preset_rect(rect).origin + Point2D::new(8.0, 8.0)
            ),
            Some(VariablesPanelButton::PresetMenu)
        );
        assert_eq!(
            panel.hover_at(rect, add_variant_rect(rect).origin + Point2D::new(8.0, 8.0)),
            Some(VariablesPanelButton::AddVariant)
        );
    }

    #[test]
    fn clickable_header_targets_paint_hover_wash() {
        let mut state = themed_state();
        state.editor_ui.variables_panel_hover = Some(VariablesPanelButton::AddTheme);
        let panel = VariablesPanel::for_editor(&state);
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(VARIABLES_PANEL_WIDTH, panel.intrinsic_height()),
        };
        let mut backend = HoverBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        panel.paint(&mut cx, rect);

        assert!(
            backend
                .round_fills
                .iter()
                .any(|(fill, color)| *fill == panel.add_theme_rect(rect)
                    && *color == panel.theme.button_hover),
            "add-theme button should paint a hover wash"
        );
    }

    #[test]
    fn pressed_header_targets_paint_shared_feedback() {
        let mut state = themed_state();
        state.editor_ui.pressed_button = Some(ButtonPressTarget::VariablesPanel(
            VariablesPanelButton::AddTheme,
        ));
        let panel = VariablesPanel::for_editor(&state);
        assert_eq!(panel.pressed, Some(VariablesPanelButton::AddTheme));
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(VARIABLES_PANEL_WIDTH, panel.intrinsic_height()),
        };
        let expected = panel
            .theme
            .button_hover
            .with_alpha(panel.theme.button_hover.a * 1.8);
        let mut backend = HoverBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        panel.paint(&mut cx, rect);

        assert!(
            backend
                .round_fills
                .iter()
                .any(|(fill, color)| *fill == panel.add_theme_rect(rect) && *color == expected),
            "pressed add-theme button should paint shared pressed feedback"
        );
    }

    #[test]
    fn pressed_add_variable_menu_item_paints_shared_feedback() {
        let mut state = themed_state();
        state.editor_ui.variables_add_menu_open = true;
        state.editor_ui.pressed_button = Some(ButtonPressTarget::VariablesPanel(
            VariablesPanelButton::AddVariableMenuItem(1),
        ));
        let panel = VariablesPanel::for_editor(&state);
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(VARIABLES_PANEL_WIDTH, panel.intrinsic_height()),
        };
        let menu = add_variable_menu_rect(rect);
        let row_y = menu.origin.y + ADD_VARIABLE_MENU_ROW_HEIGHT;
        let expected_rect = Rect {
            origin: Point2D::new(menu.origin.x + 4.0, row_y + 3.0),
            size: Point2D::new(menu.size.x - 8.0, ADD_VARIABLE_MENU_ROW_HEIGHT - 6.0),
        };
        let expected_color = panel
            .theme
            .button_hover
            .with_alpha(panel.theme.button_hover.a * 1.8);
        let mut backend = HoverBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        panel.paint(&mut cx, rect);

        assert!(
            backend
                .round_fills
                .iter()
                .any(|(fill, color)| *fill == expected_rect && *color == expected_color),
            "pressed add-variable menu item should paint shared pressed feedback"
        );
    }
}
