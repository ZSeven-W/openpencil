//! `PropertyPanel` hit-testing — action dispatch, focusable input
//! rows, hover indices, and the popover-priority ladder that lets
//! floating surfaces win over the scrolling panel body.
//!
//! Split out of `property_panel.rs` to keep both files under the
//! openpencil 800-line cap.

use super::{EffectAddMenuHit, PropertyPanel, PropertyPanelAction};
use crate::widgets::property_panel_interactions::InteractionMenuHit;
use crate::widgets::property_panel_sections as sections;
use crate::{Point2D, Rect};
use jian_widgets::components::select::SelectHit;
use op_editor_core::PropertyFocus;

impl PropertyPanel {
    /// Whether a physical point is in the Code tab's horizontal framework
    /// strip. Native touch arbitration uses this one bounded query instead of
    /// duplicating the panel's density coordinate mapping.
    pub fn code_framework_strip_contains(&self, panel_rect: Rect, point: Point2D) -> bool {
        if !matches!(self.tab, op_editor_core::PropertyTab::Code) || !panel_rect.contains(point) {
            return false;
        }
        let logical_rect = self.logical_rect(panel_rect);
        let logical_point = self.logical_point(panel_rect, point);
        let (top, bottom) = crate::widgets::property_panel_code::framework_row_band_for(
            logical_rect.origin.y,
            self.density_scale > 1.0,
        );
        logical_point.y >= top && logical_point.y <= bottom
    }

    /// Map a physical surface point over the Code tab to its hover state.
    /// The panel-owned Codegen snapshot already carries logical scroll offsets.
    pub(crate) fn code_hover_at(
        &self,
        panel_rect: Rect,
        point: Point2D,
    ) -> (
        Option<op_editor_core::codegen::Framework>,
        Option<op_editor_core::codegen::CodegenHover>,
    ) {
        if !matches!(self.tab, op_editor_core::PropertyTab::Code) {
            return (None, None);
        }
        crate::widgets::property_panel_code::code_hover_at_with_locale_for_touch(
            self.logical_rect(panel_rect),
            &self.codegen,
            self.logical_point(panel_rect, point),
            self.locale,
            self.density_scale > 1.0,
        )
    }

    /// Map a physical surface point inside the generated-code preview to a
    /// byte offset. Hosts use this for caret placement and selection drags.
    pub fn code_text_offset_at(&self, panel_rect: Rect, point: Point2D) -> Option<usize> {
        if !matches!(self.tab, op_editor_core::PropertyTab::Code) {
            return None;
        }
        crate::widgets::property_panel_code::code_text_offset_at(
            self.logical_rect(panel_rect),
            &self.codegen,
            self.logical_point(panel_rect, point),
        )
    }

    /// Hit-test the flex / size buttons + checkboxes. Returns the
    /// action the host should dispatch, or `None` if the cursor
    /// missed every clickable shape. Called AFTER `hit_test` so
    /// text inputs win over the action rects they overlap with.
    pub fn hit_test_action(&self, panel_rect: Rect, point: Point2D) -> Option<PropertyPanelAction> {
        self.hit_test_action_logical(
            self.logical_rect(panel_rect),
            self.logical_point(panel_rect, point),
        )
    }

    fn hit_test_action_logical(
        &self,
        panel_rect: Rect,
        point: Point2D,
    ) -> Option<PropertyPanelAction> {
        // Design / Code tab strip — clickable on either tab, incl. multi-select.
        if let Some(tab) = sections::tab_strip_hit(
            &self.labels,
            panel_rect.origin.x,
            panel_rect.origin.y,
            point,
            self.snapshot.widget.is_some(),
            self.density_scale > 1.0,
        ) {
            return Some(PropertyPanelAction::SetPropertyTab(tab));
        }
        if self.is_multi {
            // Multi-select inputs / toggles are inert in v1.
            return None;
        }
        if matches!(self.tab, op_editor_core::PropertyTab::Code) {
            return crate::widgets::property_panel_code::code_action_hit_with_locale_for_touch(
                panel_rect,
                &self.codegen,
                point,
                self.locale,
                self.density_scale > 1.0,
            );
        }
        if self.page_only {
            return self.page_action_at(panel_rect, point);
        }
        if self.image_fill_popover_open {
            if let Some(action) = sections::image_fill_popover_action_at(
                self.scrolled_rect(panel_rect),
                self.visible_sections(),
                &self.snapshot,
                point,
            ) {
                return Some(action);
            }
        }
        if self.compositing_picker.open {
            match self.compositing_picker_hit_logical(panel_rect, point) {
                SelectHit::Row(index) => {
                    if let Some(target) = self.compositing_picker_target {
                        return crate::widgets::property_panel_compositing::action_for_row(
                            target, index,
                        );
                    }
                }
                SelectHit::Inside => return None,
                SelectHit::Outside => {}
            }
        }
        // Image Search / Generate popovers — overlay controls win
        // over everything beneath them (they extend out of the rail).
        if self.image_panel.search_open || self.image_panel.generate_open {
            if let Some(action) =
                crate::widgets::property_panel_image_assets::image_popover_action_at(
                    self.scrolled_rect(panel_rect),
                    self.visible_sections(),
                    &self.image_panel,
                    self.image_gen_profile.as_ref(),
                    point,
                )
            {
                return Some(action);
            }
        }
        // Font-family picker rows (searchable overlay).
        if self.font_picker.open {
            let entries = self.font_picker_entries();
            if let Some(action) = crate::widgets::property_panel_typography::font_picker_action_at(
                self.scrolled_rect(panel_rect),
                self.visible_sections(),
                &entries,
                self.font_import_supported,
                &self.font_picker,
                point,
            ) {
                return Some(action);
            }
        }
        if self.fill_type_picker.open {
            match self.fill_type_picker_hit_logical(panel_rect, point) {
                SelectHit::Row(idx) => {
                    if let Some(fill_type) = crate::widgets::property_panel_fill::fill_type_at(idx)
                    {
                        return Some(PropertyPanelAction::SetFillType {
                            index: self.fill_type_picker_index,
                            fill_type,
                        });
                    }
                }
                SelectHit::Inside => return None,
                SelectHit::Outside => {}
            }
        }
        // Effects "+" add-menu: when open, its rows win over the panel
        // body; clicks inside its chrome are swallowed.
        if self.effect_add_picker_open {
            match self.effect_add_menu_hit_logical(panel_rect, point) {
                EffectAddMenuHit::Row(action) => return Some(action),
                EffectAddMenuHit::Inside => return None,
                EffectAddMenuHit::Outside => {}
            }
        }
        if self.interaction_menu_open {
            match self.interaction_menu_hit_logical(panel_rect, point) {
                InteractionMenuHit::Row(action) => return Some(action),
                InteractionMenuHit::Inside => return None,
                InteractionMenuHit::Outside => {}
            }
        }
        if !self.point_in_section_viewport(panel_rect, point) {
            return None;
        }
        let scrolled = self.scrolled_rect(panel_rect);
        let component_block_y = scrolled.origin.y
            + crate::widgets::property_panel_inputs::TAB_HEIGHT
            + crate::widgets::property_panel_inputs::HEADER_HEIGHT;
        if let Some(index) = crate::widgets::property_panel_instance::option_index_at(
            scrolled.origin.x,
            component_block_y,
            scrolled.size.x,
            self.visible_sections().component_button,
            point,
        ) {
            if let Some(option) = self.instance_component_options.get(index) {
                return Some(PropertyPanelAction::SetInstanceComponent(option.id.clone()));
            }
        }
        let rects = sections::action_button_rects_with_fill_picker(
            scrolled,
            self.visible_sections(),
            &self.snapshot.effects,
            &self.snapshot.fills,
            &self.snapshot.interactions,
            self.fill_type_picker.open,
            self.fill_type_picker_index,
            self.font_picker.open,
            self.font_weight_picker_open,
            self.export_scale_picker_open,
            self.export_format_picker_open,
            self.padding_mode_popover_open,
        );
        // Picker rows live in `rects` AFTER the dropdown rect, so
        // a row hit takes priority — `rev()` makes the picker rows
        // tested first and short-circuits before the dropdown
        // toggle, otherwise clicking a row would just re-toggle.
        for (action, rect) in rects.into_iter().rev() {
            if (rect).contains(point) {
                if let PropertyPanelAction::AdjustEffectParam { effect, field, .. } = &action {
                    return Some(PropertyPanelAction::AdjustEffectParam {
                        effect: *effect,
                        field: *field,
                        new_value: crate::widgets::property_panel_effects::slider_value(
                            rect, point.x,
                        ),
                    });
                }
                return Some(action);
            }
        }
        None
    }

    /// Row index of the open Export select popup under `point`, or
    /// `None` when no popup is open / the cursor is off every row.
    /// The index counts only the option rows (`SetExportScale` /
    /// `SetExportFormat`), matching `paint_select_popup`'s row walk,
    /// so it can drive the popup's hover highlight.
    pub fn export_picker_row_at(&self, panel_rect: Rect, point: Point2D) -> Option<usize> {
        self.export_picker_row_at_logical(
            self.logical_rect(panel_rect),
            self.logical_point(panel_rect, point),
        )
    }

    fn export_picker_row_at_logical(&self, panel_rect: Rect, point: Point2D) -> Option<usize> {
        if !self.export_scale_picker_open && !self.export_format_picker_open {
            return None;
        }
        if !self.point_in_section_viewport(panel_rect, point) {
            return None;
        }
        sections::action_button_rects_with_fill_picker(
            self.scrolled_rect(panel_rect),
            self.visible_sections(),
            &self.snapshot.effects,
            &self.snapshot.fills,
            &self.snapshot.interactions,
            self.fill_type_picker.open,
            self.fill_type_picker_index,
            self.font_picker.open,
            self.font_weight_picker_open,
            self.export_scale_picker_open,
            self.export_format_picker_open,
            self.padding_mode_popover_open,
        )
        .into_iter()
        .filter(|(a, _)| {
            matches!(
                a,
                PropertyPanelAction::SetExportScale(_) | PropertyPanelAction::SetExportFormat(_)
            )
        })
        .position(|(_, rect)| (rect).contains(point))
    }

    pub fn image_adjustment_drag_action(
        &self,
        panel_rect: Rect,
        field: op_editor_core::ImageAdjustmentField,
        x: f32,
    ) -> Option<PropertyPanelAction> {
        let point = self.logical_point(panel_rect, Point2D::new(x, panel_rect.origin.y));
        let panel_rect = self.logical_rect(panel_rect);
        if self.is_multi || !self.image_fill_popover_open {
            return None;
        }
        sections::image_fill_popover_adjustment_action_for_drag(
            self.scrolled_rect(panel_rect),
            self.visible_sections(),
            &self.snapshot,
            field,
            point.x,
        )
    }

    pub fn effect_radius_drag_action(
        &self,
        panel_rect: Rect,
        effect_index: usize,
        x: f32,
    ) -> Option<PropertyPanelAction> {
        let point = self.logical_point(panel_rect, Point2D::new(x, panel_rect.origin.y));
        let panel_rect = self.logical_rect(panel_rect);
        if self.is_multi {
            return None;
        }
        sections::action_button_rects_with_fill_picker(
            self.scrolled_rect(panel_rect),
            self.visible_sections(),
            &self.snapshot.effects,
            &self.snapshot.fills,
            &self.snapshot.interactions,
            self.fill_type_picker.open,
            self.fill_type_picker_index,
            self.font_picker.open,
            self.font_weight_picker_open,
            self.export_scale_picker_open,
            self.export_format_picker_open,
            self.padding_mode_popover_open,
        )
        .into_iter()
        .find_map(|(action, rect)| match action {
            PropertyPanelAction::AdjustEffectParam { effect, field, .. }
                if effect == effect_index =>
            {
                Some(PropertyPanelAction::AdjustEffectParam {
                    effect,
                    field,
                    new_value: crate::widgets::property_panel_effects::slider_value(rect, point.x),
                })
            }
            _ => None,
        })
    }

    pub fn image_fill_popover_contains(&self, panel_rect: Rect, point: Point2D) -> bool {
        self.image_fill_popover_contains_logical(
            self.logical_rect(panel_rect),
            self.logical_point(panel_rect, point),
        )
    }

    fn image_fill_popover_contains_logical(&self, panel_rect: Rect, point: Point2D) -> bool {
        !self.is_multi
            && self.image_fill_popover_open
            && sections::image_fill_popover_contains(
                self.scrolled_rect(panel_rect),
                self.visible_sections(),
                &self.snapshot,
                point,
            )
    }

    /// Focusable Tile-scale input inside the floating image-fill
    /// editor. Hosts call this before the rail's regular input walker
    /// because the popover can extend left over the canvas.
    pub fn image_fill_popover_input_at(
        &self,
        panel_rect: Rect,
        point: Point2D,
    ) -> Option<PropertyFocus> {
        let point = self.logical_point(panel_rect, point);
        let panel_rect = self.logical_rect(panel_rect);
        if self.is_multi || !self.image_fill_popover_open {
            return None;
        }
        sections::image_fill_popover_input_at(
            self.scrolled_rect(panel_rect),
            self.visible_sections(),
            &self.snapshot,
            point,
        )
    }

    // Font-picker / image-popover overlay accessors (entries,
    // contains, hover index, max scroll) live in
    // `property_panel_overlay_hit.rs` — same `impl PropertyPanel`,
    // split for the 800-line cap.

    /// Hit-test the panel at `point` and return which input row
    /// (if any) contains the click. The layout walk mirrors the
    /// per-kind section filtering applied in `paint`, so rects
    /// after a skipped section don't drift out of alignment.
    pub fn hit_test(&self, panel_rect: Rect, point: Point2D) -> Option<PropertyFocus> {
        let point = self.logical_point(panel_rect, point);
        let panel_rect = self.logical_rect(panel_rect);
        if self.is_multi {
            // Inputs inert in v1 multi-select aggregate view.
            return None;
        }
        if matches!(self.tab, op_editor_core::PropertyTab::Code) {
            // The Code tab paints no Design input rows — a click must
            // not focus an invisible input (paint + hit-test agree).
            return None;
        }
        if self.page_only {
            return self.page_input_at(panel_rect, point);
        }
        if !self.point_in_section_viewport(panel_rect, point) {
            return None;
        }
        for (focus, rect) in sections::editable_input_rects(
            self.scrolled_rect(panel_rect),
            self.visible_sections(),
            &self.snapshot.fills,
            &self.snapshot.effects,
        ) {
            if (rect).contains(point) {
                return Some(focus);
            }
        }
        None
    }

    /// Index into `action_button_rects_with_fill_picker` of the action
    /// button under `point`, or `None`. Design-tab single-select only —
    /// drives the per-button `theme.button_hover` wash. Shares the
    /// walker geometry with `hit_test_action` + paint so it can't drift.
    pub fn action_hover_index(&self, panel_rect: Rect, point: Point2D) -> Option<usize> {
        self.action_hover_index_logical(
            self.logical_rect(panel_rect),
            self.logical_point(panel_rect, point),
        )
    }

    fn action_hover_index_logical(&self, panel_rect: Rect, point: Point2D) -> Option<usize> {
        if self.is_multi || matches!(self.tab, op_editor_core::PropertyTab::Code) {
            return None;
        }
        if self.page_only {
            return None;
        }
        if self.fill_type_picker.open
            && !matches!(
                self.fill_type_picker_hit_logical(panel_rect, point),
                SelectHit::Outside
            )
        {
            return None;
        }
        if self.compositing_picker_contains_logical(panel_rect, point) {
            return None;
        }
        if !self.point_in_section_viewport(panel_rect, point) {
            return None;
        }
        sections::action_button_rects_with_fill_picker(
            self.scrolled_rect(panel_rect),
            self.visible_sections(),
            &self.snapshot.effects,
            &self.snapshot.fills,
            &self.snapshot.interactions,
            self.fill_type_picker.open,
            self.fill_type_picker_index,
            self.font_picker.open,
            self.font_weight_picker_open,
            self.export_scale_picker_open,
            self.export_format_picker_open,
            self.padding_mode_popover_open,
        )
        .iter()
        .position(|(_, r)| (*r).contains(point))
    }

    /// Pinned Design / Code tab under the cursor.
    pub fn tab_hover_at(
        &self,
        panel_rect: Rect,
        point: Point2D,
    ) -> Option<op_editor_core::PropertyTab> {
        let point = self.logical_point(panel_rect, point);
        let panel_rect = self.logical_rect(panel_rect);
        sections::tab_strip_hit(
            &self.labels,
            panel_rect.origin.x,
            panel_rect.origin.y,
            point,
            self.snapshot.widget.is_some(),
            self.density_scale > 1.0,
        )
    }
}
