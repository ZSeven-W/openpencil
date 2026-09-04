//! Responsive coordinate mapping for the Property Panel.
//!
//! Section code keeps its established desktop metrics. Touch panels resolve a
//! smaller logical viewport and the widget maps that viewport to the physical
//! surface in one place, keeping paint, hit-testing, popovers and scrolling in
//! the same coordinate system.

use super::PropertyPanel;
use crate::{Point2D, Rect};

/// 30pt desktop controls become 44.1pt touch controls.
pub(super) const TOUCH_DENSITY_SCALE: f32 = 1.47;

impl PropertyPanel {
    pub(crate) fn logical_length(&self, physical: f32) -> f32 {
        physical / self.density_scale
    }

    pub(crate) fn physical_length(&self, logical: f32) -> f32 {
        logical * self.density_scale
    }

    pub(crate) fn logical_rect(&self, physical: Rect) -> Rect {
        let scale = self.density_scale;
        if scale == 1.0 {
            return physical;
        }
        Rect {
            origin: physical.origin,
            size: Point2D::new(physical.size.x / scale, physical.size.y / scale),
        }
    }

    pub(crate) fn logical_point(&self, physical_rect: Rect, point: Point2D) -> Point2D {
        let scale = self.density_scale;
        if scale == 1.0 {
            return point;
        }
        Point2D::new(
            physical_rect.origin.x + (point.x - physical_rect.origin.x) / scale,
            physical_rect.origin.y + (point.y - physical_rect.origin.y) / scale,
        )
    }

    pub(crate) fn physical_rect(&self, logical_rect: Rect, rect: Rect) -> Rect {
        let scale = self.density_scale;
        if scale == 1.0 {
            return rect;
        }
        Rect {
            origin: Point2D::new(
                logical_rect.origin.x + (rect.origin.x - logical_rect.origin.x) * scale,
                logical_rect.origin.y + (rect.origin.y - logical_rect.origin.y) * scale,
            ),
            size: Point2D::new(rect.size.x * scale, rect.size.y * scale),
        }
    }

    pub(super) fn begin_density_paint(
        &self,
        backend: &mut dyn crate::RenderBackend,
        physical_rect: Rect,
    ) -> Rect {
        let logical = self.logical_rect(physical_rect);
        if self.density_scale != 1.0 {
            backend.save();
            backend.scale(
                Point2D::new(self.density_scale, self.density_scale),
                physical_rect.origin,
            );
        }
        logical
    }

    pub(super) fn end_density_paint(&self, backend: &mut dyn crate::RenderBackend) {
        if self.density_scale != 1.0 {
            backend.restore();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::property_panel_action::{CodegenAction, PropertyPanelAction};
    use crate::widgets::{PaintCx, Widget};
    use crate::{Color, RenderBackend, TextLayout};
    use op_editor_core::size_class::EditorSizeClass;
    use op_editor_core::{EditorState, PropertyTab};

    fn center(rect: Rect) -> Point2D {
        Point2D::new(
            rect.origin.x + rect.size.x / 2.0,
            rect.origin.y + rect.size.y / 2.0,
        )
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.01,
            "expected {expected}, got {actual}"
        );
    }

    fn touch_panel(class: EditorSizeClass) -> PropertyPanel {
        let mut state = EditorState::sample();
        state.editor_ui.touch = true;
        state.editor_ui.size_class = class;
        PropertyPanel::for_selection(&state).expect("sample selection")
    }

    #[test]
    fn desktop_mapping_is_exactly_identity() {
        let panel = PropertyPanel::for_selection(&EditorState::sample()).unwrap();
        let rect = Rect::xywh(17.0, 29.0, 280.0, 700.0);
        let point = Point2D::new(143.0, 211.0);
        assert_eq!(panel.density_scale, 1.0);
        assert_eq!(panel.logical_rect(rect), rect);
        assert_eq!(panel.logical_point(rect, point), point);
        assert_eq!(panel.physical_rect(rect, rect), rect);
        assert_eq!(panel.logical_length(37.0), 37.0);
        assert_eq!(panel.physical_length(37.0), 37.0);
    }

    #[test]
    fn every_touch_size_class_uses_readable_control_density() {
        for class in [
            EditorSizeClass::Compact,
            EditorSizeClass::Medium,
            EditorSizeClass::Expanded,
        ] {
            let panel = touch_panel(class);
            assert_eq!(panel.density_scale, TOUCH_DENSITY_SCALE);
            assert!(
                crate::widgets::property_panel_inputs::INPUT_HEIGHT * panel.density_scale >= 44.0
            );
        }
    }

    #[test]
    fn touch_rect_point_and_length_mapping_round_trip() {
        let panel = touch_panel(EditorSizeClass::Compact);
        let physical = Rect::xywh(31.0, 47.0, 411.6, 1029.0);
        let logical = panel.logical_rect(physical);
        assert_close(logical.size.x, 280.0);
        assert_close(logical.size.y, 700.0);

        let logical_child = Rect::xywh(51.0, 87.0, 120.0, 30.0);
        let physical_child = panel.physical_rect(logical, logical_child);
        let mapped_origin = panel.logical_point(physical, physical_child.origin);
        assert_close(mapped_origin.x, logical_child.origin.x);
        assert_close(mapped_origin.y, logical_child.origin.y);
        assert_close(
            physical_child.size.x,
            logical_child.size.x * TOUCH_DENSITY_SCALE,
        );
        assert_close(physical_child.size.y, 44.1);
    }

    #[test]
    fn touch_content_height_and_scroll_use_physical_persistence() {
        let desktop = PropertyPanel::for_selection(&EditorState::sample()).unwrap();
        let mut touch_state = EditorState::sample();
        touch_state.editor_ui.touch = true;
        touch_state.editor_ui.size_class = EditorSizeClass::Compact;
        touch_state.editor_ui.property_panel_scroll.offset = 73.5;
        let touch = PropertyPanel::for_selection(&touch_state).unwrap();
        let desktop_rect = Rect::xywh(0.0, 0.0, 280.0, 700.0);
        let touch_rect = Rect::xywh(
            0.0,
            0.0,
            desktop_rect.size.x * TOUCH_DENSITY_SCALE,
            desktop_rect.size.y * TOUCH_DENSITY_SCALE,
        );

        let touch_logical_height = touch.content_height(touch_rect) / TOUCH_DENSITY_SCALE;
        let desktop_height = desktop.content_height(desktop_rect);
        let size_rows = if touch.visible_sections().clip_content {
            3.0
        } else {
            2.0
        };
        let expected_extra = (crate::widgets::property_panel_inputs::TOUCH_SIZE_CHECK_ROW_HEIGHT
            - crate::widgets::property_panel_inputs::SIZE_CHECK_ROW_HEIGHT)
            * size_rows
            + if touch.visible_sections().text {
                (crate::widgets::property_panel_text::text_button_height(true)
                    - crate::widgets::property_panel_text::text_button_height(false))
                    * 3.0
            } else {
                0.0
            };
        assert_close(touch_logical_height, desktop_height + expected_extra);
        let scrolled = touch.scrolled_rect(touch.logical_rect(touch_rect));
        assert_close(scrolled.origin.y, -50.0);
    }

    #[test]
    fn scaled_input_hit_matches_the_painted_physical_rect() {
        let panel = touch_panel(EditorSizeClass::Compact);
        let physical = Rect::xywh(10.0, 20.0, 411.6, 1029.0);
        let logical = panel.logical_rect(physical);
        let (focus, input) = crate::widgets::property_panel_sections::editable_input_rects(
            panel.scrolled_rect(logical),
            panel.visible_sections(),
            &panel.snapshot.fills,
            &panel.snapshot.effects,
        )
        .into_iter()
        .next()
        .expect("sample panel has an editable input");
        let physical_input = panel.physical_rect(logical, input);

        assert_close(physical_input.size.y, 44.1);
        assert_eq!(
            panel.hit_test(physical, center(physical_input)),
            Some(focus)
        );
    }

    #[test]
    fn touch_size_checkbox_rows_are_full_44pt_targets() {
        let panel = touch_panel(EditorSizeClass::Compact);
        let physical = Rect::xywh(10.0, 20.0, 411.6, 1029.0);
        let logical = panel.logical_rect(physical);
        let rows = crate::widgets::property_panel_sections::action_button_rects(
            panel.scrolled_rect(logical),
            panel.visible_sections(),
            &panel.snapshot.effects,
            &panel.snapshot.fills,
            &panel.snapshot.interactions,
        );
        let size_rows: Vec<(PropertyPanelAction, Rect)> = rows
            .into_iter()
            .filter(|(action, _)| {
                matches!(
                    action,
                    PropertyPanelAction::ToggleSizeFillWidth
                        | PropertyPanelAction::ToggleSizeFillHeight
                        | PropertyPanelAction::ToggleSizeHugWidth
                        | PropertyPanelAction::ToggleSizeHugHeight
                        | PropertyPanelAction::ToggleSizeClipContent
                )
            })
            .map(|(action, rect)| (action, panel.physical_rect(logical, rect)))
            .collect();
        assert!(size_rows.len() >= 4);
        for (action, rect) in &size_rows {
            assert!(rect.size.y >= 44.0);
            assert_eq!(
                panel.hit_test_action(physical, center(*rect)),
                Some(action.clone())
            );
        }
        assert_close(
            crate::widgets::property_panel_inputs::TOUCH_SIZE_CHECK_BOX_SIZE * TOUCH_DENSITY_SCALE,
            20.58,
        );
    }

    #[test]
    fn touch_fill_and_effect_actions_are_full_44pt_targets() {
        let panel = touch_panel(EditorSizeClass::Compact);
        let physical = Rect::xywh(10.0, 20.0, 411.6, 1029.0);
        let logical = panel.logical_rect(physical);
        let actions = crate::widgets::property_panel_sections::action_button_rects(
            panel.scrolled_rect(logical),
            panel.visible_sections(),
            &panel.snapshot.effects,
            &panel.snapshot.fills,
            &panel.snapshot.interactions,
        );
        let targets: Vec<_> = actions
            .iter()
            .filter(|(action, _)| {
                matches!(
                    action,
                    PropertyPanelAction::AddFill
                        | PropertyPanelAction::RemoveFill(_)
                        | PropertyPanelAction::ToggleEffectAddPicker
                )
            })
            .map(|(action, rect)| (action.clone(), panel.physical_rect(logical, *rect)))
            .collect();
        assert!(targets.len() >= 2);
        for (action, rect) in targets {
            assert!(rect.size.x >= 44.0, "{action:?} width was {}", rect.size.x);
            assert!(rect.size.y >= 44.0, "{action:?} height was {}", rect.size.y);
        }
        let effect = crate::widgets::property_panel_effects::effect_row_rects(
            logical.origin.x,
            logical.origin.y,
            logical.size.x,
            true,
        );
        for rect in [effect.eye, effect.remove] {
            let physical_rect = panel.physical_rect(logical, rect);
            assert!(physical_rect.size.x >= 44.0);
            assert!(physical_rect.size.y >= 44.0);
        }
    }

    #[test]
    fn remaining_touch_property_targets_reach_44pt_without_changing_desktop() {
        let scale = TOUCH_DENSITY_SCALE;
        let mut touch_actions = Vec::new();
        crate::widgets::property_panel_flex::push_flex_action_rects(
            &mut touch_actions,
            0.0,
            0.0,
            280.0,
            op_editor_core::FlexLayout::Horizontal,
            super::super::LayoutJustifyValue::Start,
            false,
            true,
        );
        let alignment: Vec<_> = touch_actions
            .iter()
            .filter(|(action, _)| {
                matches!(
                    action,
                    PropertyPanelAction::SetLayoutAlignment { .. }
                        | PropertyPanelAction::SetLayoutAlign(_)
                )
            })
            .collect();
        assert_eq!(alignment.len(), 9);
        assert!(alignment
            .iter()
            .all(|(_, rect)| rect.size.y * scale >= 44.0));
        let gap_modes: Vec<_> = touch_actions
            .iter()
            .filter(|(action, _)| matches!(action, PropertyPanelAction::SetLayoutJustify(_)))
            .collect();
        assert_eq!(gap_modes.len(), 3);
        assert!(gap_modes
            .iter()
            .all(|(_, rect)| { rect.size.x * scale >= 44.0 && rect.size.y * scale >= 44.0 }));

        let text_touch =
            crate::widgets::property_panel_text::text_action_rects(0.0, 0.0, 248.0, true);
        for (action, rect) in text_touch.iter().filter(|(action, _)| {
            matches!(
                action,
                PropertyPanelAction::SetTextGrowth(_)
                    | PropertyPanelAction::SetTextAlign(_)
                    | PropertyPanelAction::SetTextVerticalAlign(_)
            )
        }) {
            assert!(rect.size.y * scale >= 44.0, "{action:?}: {rect:?}");
        }

        let effect =
            crate::widgets::property_panel_effects::effect_row_rects(0.0, 0.0, 280.0, true);
        assert!(effect.slider.size.y * scale >= 44.0);
        assert_close(
            crate::widgets::property_panel_inputs::color_swatch_action_width(true) * scale,
            44.1,
        );

        let mut desktop_actions = Vec::new();
        crate::widgets::property_panel_flex::push_flex_action_rects(
            &mut desktop_actions,
            0.0,
            0.0,
            280.0,
            op_editor_core::FlexLayout::Horizontal,
            super::super::LayoutJustifyValue::Start,
            false,
            false,
        );
        let desktop_alignment = desktop_actions
            .iter()
            .find(|(action, _)| matches!(action, PropertyPanelAction::SetLayoutAlignment { .. }))
            .unwrap()
            .1;
        assert_close(desktop_alignment.size.y, 22.0);
        let desktop_gap = desktop_actions
            .iter()
            .find(|(action, _)| matches!(action, PropertyPanelAction::SetLayoutJustify(_)))
            .unwrap()
            .1;
        assert_close(desktop_gap.size.y, 20.0);
        let desktop_text =
            crate::widgets::property_panel_text::text_action_rects(0.0, 0.0, 248.0, false);
        assert_close(desktop_text[0].1.size.y, 28.0);
        assert_close(
            crate::widgets::property_panel_effects::effect_row_rects(0.0, 0.0, 280.0, false)
                .slider
                .size
                .y,
            20.0,
        );
        assert_close(
            crate::widgets::property_panel_inputs::color_swatch_action_width(false),
            28.0,
        );
    }

    #[test]
    fn desktop_fill_and_effect_action_geometry_is_unchanged() {
        let fill = crate::widgets::property_panel_fill::fill_head_rects(0.0, 0.0, 256.0, false);
        assert_close(fill.move_up.size.x, 20.0);
        assert_close(fill.move_down.size.x, 20.0);
        assert_close(fill.remove.size.x, 22.0);
        let effect =
            crate::widgets::property_panel_effects::effect_row_rects(0.0, 0.0, 256.0, false);
        assert_close(effect.eye.size.x, 24.0);
        assert_close(effect.remove.size.x, 24.0);
        let add = crate::widgets::property_panel_inputs::section_add_target(0.0, 0.0, 256.0, false);
        assert_close(add.size.x, 28.0);
        assert_close(add.size.y, 24.0);
    }

    #[test]
    fn touch_mode_popover_rows_are_full_44pt_targets() {
        let gear = Rect::xywh(200.0, 40.0, 30.0, 30.0);
        let popup = crate::widgets::property_panel_mode_popover::mode_popover_rect_from_gear(
            gear, 280.0, true,
        );
        let rows = crate::widgets::property_panel_mode_popover::mode_popover_rows(popup, true);
        for row in rows {
            assert!(row.size.y * TOUCH_DENSITY_SCALE >= 44.0);
        }
    }

    #[test]
    fn clipped_property_rows_cannot_hit_below_the_panel() {
        let panel = touch_panel(EditorSizeClass::Medium);
        let physical = Rect::xywh(420.0, 56.0, 360.0, 900.0);
        let below = Point2D::new(
            physical.origin.x + physical.size.x / 2.0,
            physical.origin.y + physical.size.y + 1.0,
        );
        assert_eq!(panel.hit_test_action(physical, below), None);
        assert_eq!(panel.hit_test(physical, below), None);
    }

    #[test]
    fn effect_menu_rect_rows_and_hit_share_the_same_mapping() {
        let mut state = EditorState::sample();
        state.editor_ui.touch = true;
        state.editor_ui.size_class = EditorSizeClass::Medium;
        state.editor_ui.effect_add_picker_open = true;
        let panel = PropertyPanel::for_selection(&state).unwrap();
        let physical = Rect::xywh(420.0, 56.0, 360.0, 900.0);
        let logical_panel = panel.logical_rect(physical);
        let physical_menu = panel
            .effect_add_menu_rect(physical)
            .expect("open menu rect");
        let logical_menu = Rect {
            origin: panel.logical_point(physical, physical_menu.origin),
            size: Point2D::new(
                panel.logical_length(physical_menu.size.x),
                panel.logical_length(physical_menu.size.y),
            ),
        };
        let (_, first_row) =
            crate::widgets::property_panel_effects::effect_add_menu_row_rects(logical_menu)
                .into_iter()
                .next()
                .unwrap();
        let physical_row = panel.physical_rect(logical_panel, first_row);

        assert!(panel.effect_add_menu_contains(physical, center(physical_row)));
        assert!(matches!(
            panel.effect_add_menu_hit(physical, center(physical_row)),
            super::super::EffectAddMenuHit::Row(_)
        ));
        assert_eq!(
            panel.effect_add_menu_row_at(physical, center(physical_row)),
            Some(0)
        );
    }

    #[test]
    fn code_tab_normalizes_scroll_and_hits_scaled_actions() {
        let mut state = EditorState::sample();
        state.editor_ui.touch = true;
        state.editor_ui.size_class = EditorSizeClass::Expanded;
        state.editor_ui.property_tab = PropertyTab::Code;
        state.codegen.framework_scroll.offset = 147.0;
        state.codegen.code_scroll.offset = 294.0;
        let panel = PropertyPanel::for_selection(&state).unwrap();
        assert_close(panel.codegen.framework_scroll.offset, 100.0);
        assert_close(panel.codegen.code_scroll.offset, 200.0);

        let physical = Rect::xywh(600.0, 56.0, 411.6, 882.0);
        let logical = panel.logical_rect(physical);
        let (action, action_rect) =
            crate::widgets::property_panel_code::code_action_rects_in_panel(
                logical,
                &panel.codegen,
            )
            .into_iter()
            .find(|(action, _)| matches!(action, CodegenAction::Generate))
            .expect("idle Code tab has Generate action");
        let physical_action = panel.physical_rect(logical, action_rect);
        assert_eq!(
            panel.hit_test_action(physical, center(physical_action)),
            Some(PropertyPanelAction::Codegen(action))
        );
    }

    #[test]
    fn medium_tablet_code_framework_targets_are_at_least_44pt() {
        let mut state = EditorState::sample();
        state.editor_ui.touch = true;
        state.editor_ui.size_class = EditorSizeClass::Medium;
        state.editor_ui.property_tab = PropertyTab::Code;
        let panel = PropertyPanel::for_selection(&state).unwrap();
        let physical = Rect::xywh(0.0, 56.0, 411.6, 882.0);
        let logical = panel.logical_rect(physical);
        let targets =
            crate::widgets::property_panel_code::code_action_rects_in_panel_with_locale_for_touch(
                logical,
                &panel.codegen,
                panel.locale,
                true,
            );
        let framework_targets: Vec<_> = targets
            .into_iter()
            .filter(|(action, _)| {
                matches!(
                    action,
                    CodegenAction::SelectFramework(_)
                        | CodegenAction::ScrollFrameworksLeft
                        | CodegenAction::ScrollFrameworksRight
                )
            })
            .map(|(action, rect)| (action, panel.physical_rect(logical, rect)))
            .collect();
        assert!(framework_targets.len() >= 4);
        for (action, rect) in framework_targets {
            assert!(rect.size.x >= 44.0, "{action:?} width was {rect:?}");
            assert!(rect.size.y >= 44.0, "{action:?} height was {rect:?}");
            assert_eq!(
                panel.hit_test_action(physical, center(rect)),
                Some(PropertyPanelAction::Codegen(action))
            );
        }
    }

    #[test]
    fn desktop_code_framework_metrics_remain_compact() {
        let state = EditorState::sample();
        let panel_rect = Rect::xywh(0.0, 56.0, 280.0, 700.0);
        let targets = crate::widgets::property_panel_code::code_action_rects_in_panel(
            panel_rect,
            &state.codegen,
        );
        let chevrons: Vec<_> = targets
            .iter()
            .filter(|(action, _)| {
                matches!(
                    action,
                    CodegenAction::ScrollFrameworksLeft | CodegenAction::ScrollFrameworksRight
                )
            })
            .collect();
        assert_eq!(chevrons.len(), 2);
        for (_, rect) in chevrons {
            assert_close(rect.size.x, 18.0);
            assert_close(rect.size.y, 22.0);
        }
    }

    #[derive(Default)]
    struct TransformBackend {
        depth: usize,
        scales: Vec<(Point2D, Point2D)>,
        texts: Vec<String>,
    }

    impl RenderBackend for TransformBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, layout: &TextLayout, _: Point2D) {
            self.texts
                .extend(layout.runs().iter().map(|run| run.content.clone()));
        }
        fn clip_rect(&mut self, _: Rect) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn save(&mut self) {
            self.depth += 1;
        }
        fn restore(&mut self) {
            self.depth -= 1;
        }
        fn translate(&mut self, _: Point2D) {}
        fn scale(&mut self, scale: Point2D, pivot: Point2D) {
            self.scales.push((scale, pivot));
        }
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    #[test]
    fn touch_paint_applies_one_balanced_panel_transform() {
        let panel = touch_panel(EditorSizeClass::Compact);
        let rect = Rect::xywh(12.0, 56.0, 360.0, 700.0);
        let mut backend = TransformBackend::default();
        panel.paint(
            &mut PaintCx {
                backend: &mut backend,
            },
            rect,
        );

        assert_eq!(backend.depth, 0);
        assert_eq!(backend.scales.len(), 1);
        assert_eq!(backend.scales[0].1, rect.origin);
        assert_close(backend.scales[0].0.x, TOUCH_DENSITY_SCALE);
        assert_close(backend.scales[0].0.y, TOUCH_DENSITY_SCALE);
    }

    #[test]
    fn medium_touch_opacity_label_uses_the_free_second_column() {
        let mut state = EditorState::sample();
        state.editor_ui.touch = true;
        state.editor_ui.size_class = EditorSizeClass::Medium;
        state.editor_ui.locale = op_editor_core::Locale::ZhCn;
        let panel = PropertyPanel::for_selection(&state).unwrap();
        let rect = Rect::xywh(420.0, 56.0, 360.0, 900.0);
        let logical = panel.logical_rect(rect);
        assert_close(
            crate::widgets::property_panel_layer::opacity_input_width(
                logical.size.x,
                panel.snapshot.polygon_sides.is_some(),
                true,
            ),
            logical.size.x - crate::widgets::property_panel_inputs::PAD_X * 2.0,
        );

        let mut backend = TransformBackend::default();
        panel.paint(
            &mut PaintCx {
                backend: &mut backend,
            },
            rect,
        );
        assert!(backend.texts.iter().any(|text| text == "不透明度"));
        assert!(!backend.texts.iter().any(|text| text.starts_with("不透…")));
    }

    #[test]
    fn desktop_opacity_keeps_the_two_column_geometry() {
        let panel_width = 256.0;
        assert_close(
            crate::widgets::property_panel_layer::opacity_input_width(panel_width, false, false),
            (panel_width - crate::widgets::property_panel_inputs::PAD_X * 2.0 - 8.0) / 2.0,
        );
    }
}
