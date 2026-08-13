//! `PropertyPanel` accessors for the searchable font-family picker
//! and the image Search / Generate popovers — host-facing contains /
//! hover / scroll helpers, split out of `property_panel.rs` for the
//! 800-line cap (same `impl PropertyPanel`).

use std::rc::Rc;

use crate::widgets::property_panel::PropertyPanel;
use crate::widgets::{property_panel_image_assets, property_panel_typography};
use crate::{Point2D, Rect};
use jian_widgets::components::select::SelectHit;

impl PropertyPanel {
    fn point_in_design_popup_clip(&self, panel_rect: Rect, point: Point2D) -> bool {
        !self.is_multi
            && matches!(self.tab, op_editor_core::PropertyTab::Design)
            && panel_rect.contains(point)
            && point.y >= panel_rect.origin.y + crate::widgets::property_panel_inputs::TAB_HEIGHT
    }

    fn popup_action_rects(
        &self,
        panel_rect: Rect,
    ) -> Vec<(crate::widgets::property_panel::PropertyPanelAction, Rect)> {
        crate::widgets::property_panel_sections::action_button_rects_with_fill_picker(
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
    }

    /// Whether `point` falls inside either open Export scale / format
    /// popup, including the popup chrome around its option rows.
    pub fn export_picker_contains(&self, panel_rect: Rect, point: Point2D) -> bool {
        self.export_picker_contains_logical(
            self.logical_rect(panel_rect),
            self.logical_point(panel_rect, point),
        )
    }

    fn export_picker_contains_logical(&self, panel_rect: Rect, point: Point2D) -> bool {
        use crate::widgets::property_panel::PropertyPanelAction as A;

        if (!self.export_scale_picker_open && !self.export_format_picker_open)
            || !self.point_in_design_popup_clip(panel_rect, point)
        {
            return false;
        }
        let actions = self.popup_action_rects(panel_rect);
        let scale_contains = self.export_scale_picker_open
            && crate::widgets::property_panel_export::export_picker_popup_rect(
                actions.iter().filter_map(|(action, rect)| {
                    matches!(action, A::SetExportScale(_)).then_some(*rect)
                }),
            )
            .is_some_and(|popup| popup.contains(point));
        let format_contains = self.export_format_picker_open
            && crate::widgets::property_panel_export::export_picker_popup_rect(
                actions.iter().filter_map(|(action, rect)| {
                    matches!(action, A::SetExportFormat(_)).then_some(*rect)
                }),
            )
            .is_some_and(|popup| popup.contains(point));
        scale_contains || format_contains
    }

    /// Whether `point` falls inside the open font-weight popup,
    /// including its vertical chrome padding.
    pub fn font_weight_picker_contains(&self, panel_rect: Rect, point: Point2D) -> bool {
        self.font_weight_picker_contains_logical(
            self.logical_rect(panel_rect),
            self.logical_point(panel_rect, point),
        )
    }

    fn font_weight_picker_contains_logical(&self, panel_rect: Rect, point: Point2D) -> bool {
        self.font_weight_picker_open
            && self.point_in_design_popup_clip(panel_rect, point)
            && crate::widgets::property_panel_text::font_weight_picker_rect(
                self.scrolled_rect(panel_rect),
                self.visible_sections(),
            )
            .is_some_and(|popup| popup.contains(point))
    }

    /// Whether `point` falls inside the open padding-mode gear popup,
    /// including its title and padded chrome.
    pub fn padding_mode_popover_contains(&self, panel_rect: Rect, point: Point2D) -> bool {
        self.padding_mode_popover_contains_logical(
            self.logical_rect(panel_rect),
            self.logical_point(panel_rect, point),
        )
    }

    fn padding_mode_popover_contains_logical(&self, panel_rect: Rect, point: Point2D) -> bool {
        use crate::widgets::property_panel::PropertyPanelAction as A;

        if !self.padding_mode_popover_open || !self.point_in_design_popup_clip(panel_rect, point) {
            return false;
        }
        self.popup_action_rects(panel_rect)
            .into_iter()
            .find_map(|(action, gear)| {
                matches!(action, A::TogglePaddingModePopover).then_some(gear)
            })
            .map(|gear| {
                crate::widgets::property_panel_mode_popover::mode_popover_rect_from_gear(
                    gear,
                    panel_rect.size.x,
                    self.density_scale > 1.0,
                )
            })
            .is_some_and(|popup| popup.contains(point))
    }

    /// Whether `point` falls inside the open stroke-mode gear popup,
    /// including its title and padded chrome.
    pub fn stroke_mode_popover_contains(&self, panel_rect: Rect, point: Point2D) -> bool {
        self.stroke_mode_popover_contains_logical(
            self.logical_rect(panel_rect),
            self.logical_point(panel_rect, point),
        )
    }

    fn stroke_mode_popover_contains_logical(&self, panel_rect: Rect, point: Point2D) -> bool {
        use crate::widgets::property_panel::PropertyPanelAction as A;

        if !self.stroke_mode_popover_open || !self.point_in_design_popup_clip(panel_rect, point) {
            return false;
        }
        self.popup_action_rects(panel_rect)
            .into_iter()
            .find_map(|(action, gear)| matches!(action, A::ToggleStrokeModePopover).then_some(gear))
            .map(|gear| {
                crate::widgets::property_panel_mode_popover::mode_popover_rect_from_gear(
                    gear,
                    panel_rect.size.x,
                    self.density_scale > 1.0,
                )
            })
            .is_some_and(|popup| popup.contains(point))
    }

    /// Variable name bound to whichever colour target the picker is
    /// open for (`None` when nothing is bound or the picker is shut).
    pub fn bound_color_variable_ref(&self) -> Option<&str> {
        match self.color_variable_picker_open? {
            op_editor_core::ColorTarget::Fill => self.fill_variable_ref.as_deref(),
            op_editor_core::ColorTarget::Stroke => self.stroke_variable_ref.as_deref(),
            _ => None,
        }
    }

    /// The `{}` trigger the colour-variable popup hangs off, in the
    /// inspector's scrolled space so the popup tracks its own row.
    fn color_variable_anchor_rect(
        &self,
        panel_rect: Rect,
        target: op_editor_core::ColorTarget,
    ) -> Option<Rect> {
        use crate::widgets::property_panel::PropertyPanelAction as A;

        self.popup_action_rects(panel_rect)
            .into_iter()
            .find_map(|(action, rect)| match action {
                A::ToggleColorVariablePicker(t) if t == target => Some(rect),
                _ => None,
            })
    }

    /// Resolved geometry of the open colour-variable popup. `None`
    /// while it is closed or has no rows to show.
    pub fn color_variable_picker_layout(
        &self,
        panel_rect: Rect,
    ) -> Option<crate::widgets::property_panel_color_variables::ColorVariablePickerLayout> {
        let logical = self.logical_rect(panel_rect);
        self.color_variable_picker_layout_logical(logical)
            .map(|layout| layout.scaled_about(logical.origin, self.density_scale))
    }

    pub(super) fn color_variable_picker_layout_logical(
        &self,
        panel_rect: Rect,
    ) -> Option<crate::widgets::property_panel_color_variables::ColorVariablePickerLayout> {
        // Same surfaces `paint` and `hit_test_action` bail on: the
        // multi-select aggregate is inert, and the Code / page
        // inspectors paint no Design sections to anchor against.
        if self.is_multi || self.page_only || matches!(self.tab, op_editor_core::PropertyTab::Code)
        {
            return None;
        }
        let target = self.color_variable_picker_open?;
        let anchor = self.color_variable_anchor_rect(panel_rect, target)?;
        crate::widgets::property_panel_color_variables::color_variable_picker_layout(
            anchor,
            crate::widgets::property_panel_fill::fill_type_picker_viewport(panel_rect),
            self.color_variables.len(),
            self.bound_color_variable_ref().is_some(),
            self.color_variable_picker_scroll / self.density_scale,
        )
    }

    /// Whether `point` falls anywhere on the open colour-variable
    /// popup — the host swallows such presses instead of dismissing it.
    pub fn color_variable_picker_contains(&self, panel_rect: Rect, point: Point2D) -> bool {
        self.color_variable_picker_layout_logical(self.logical_rect(panel_rect))
            .is_some_and(|layout| layout.contains(self.logical_point(panel_rect, point)))
    }

    /// Bind / unbind action under `point`, honoring the popup's own
    /// scroll so a row scrolled out of view can't be clicked.
    pub fn color_variable_picker_action_at(
        &self,
        panel_rect: Rect,
        point: Point2D,
    ) -> Option<crate::widgets::property_panel::PropertyPanelAction> {
        use crate::widgets::property_panel::PropertyPanelAction as A;
        use crate::widgets::property_panel_color_variables::ColorVariableRow;

        let target = self.color_variable_picker_open?;
        let row = self
            .color_variable_picker_layout_logical(self.logical_rect(panel_rect))?
            .row_at(self.logical_point(panel_rect, point))?;
        Some(match row {
            ColorVariableRow::Unbind => A::UnbindColorVariable(target),
            ColorVariableRow::Variable(index) => A::BindColorVariable { target, index },
        })
    }

    /// Row slot under `point` for hover tracking — the same scrolled
    /// geometry [`Self::color_variable_picker_action_at`] presses
    /// against, so a hovered row is always a clickable one.
    pub fn color_variable_picker_row_at(&self, panel_rect: Rect, point: Point2D) -> Option<usize> {
        self.color_variable_picker_layout_logical(self.logical_rect(panel_rect))?
            .row_slot_at(self.logical_point(panel_rect, point))
    }

    /// Scroll range of the popup's list (host wheel handler clamp).
    pub fn color_variable_picker_max_scroll(&self, panel_rect: Rect) -> f32 {
        self.color_variable_picker_layout_logical(self.logical_rect(panel_rect))
            .map_or(0.0, |layout| layout.max_scroll * self.density_scale)
    }

    pub fn fill_type_picker_hit(&self, panel_rect: Rect, point: Point2D) -> SelectHit {
        self.fill_type_picker_hit_logical(
            self.logical_rect(panel_rect),
            self.logical_point(panel_rect, point),
        )
    }

    pub(super) fn fill_type_picker_hit_logical(
        &self,
        panel_rect: Rect,
        point: Point2D,
    ) -> SelectHit {
        if !self.fill_type_picker.open {
            return SelectHit::Outside;
        }
        let Some(action_rect) =
            crate::widgets::property_panel_sections::fill_type_toggle_action_rect(
                self.scrolled_rect(panel_rect),
                self.visible_sections(),
                &self.snapshot.effects,
                &self.snapshot.fills,
                self.fill_type_picker_index,
            )
        else {
            return SelectHit::Outside;
        };
        crate::widgets::property_panel_fill::fill_type_picker_hit(
            &self.fill_type_picker,
            action_rect,
            crate::widgets::property_panel_fill::fill_type_picker_viewport(panel_rect),
            point,
            &self.theme,
        )
    }

    pub fn fill_type_picker_row_at(&self, panel_rect: Rect, point: Point2D) -> Option<usize> {
        match self.fill_type_picker_hit_logical(
            self.logical_rect(panel_rect),
            self.logical_point(panel_rect, point),
        ) {
            SelectHit::Row(idx) => Some(idx),
            SelectHit::Inside | SelectHit::Outside => None,
        }
    }

    /// Visible font-picker entries for the current search + host
    /// enumeration — paint, hit-test, and the host dispatch resolve
    /// `SetFontFamilyIndex` against this same list. Returns the cache's
    /// shared `Rc` (see `font_picker_cache`) so the panel's several
    /// per-frame accessors don't each pay for a deep clone.
    pub fn font_picker_entries(&self) -> Rc<Vec<property_panel_typography::FontPickerEntry>> {
        property_panel_typography::font_picker_entries(
            &self.imported_font_families,
            &self.bundled_font_families,
            &self.system_font_families,
            &self.font_picker_search,
        )
    }

    /// Whether `point` falls inside the open font-family picker —
    /// the host swallows such presses without closing the popup.
    pub fn font_picker_contains(&self, panel_rect: Rect, point: Point2D) -> bool {
        if self.is_multi || !self.font_picker.open {
            return false;
        }
        let entries = self.font_picker_entries();
        property_panel_typography::font_picker_contains(
            &self.font_picker,
            self.scrolled_rect(self.logical_rect(panel_rect)),
            self.visible_sections(),
            &entries,
            self.font_import_supported,
            self.logical_point(panel_rect, point),
        )
    }

    /// Font-picker entry index under `point` (hover tracking).
    pub fn font_picker_entry_index_at(&self, panel_rect: Rect, point: Point2D) -> Option<usize> {
        if self.is_multi || !self.font_picker.open {
            return None;
        }
        let entries = self.font_picker_entries();
        property_panel_typography::font_picker_entry_index_at(
            self.scrolled_rect(self.logical_rect(panel_rect)),
            self.visible_sections(),
            &entries,
            self.font_import_supported,
            &self.font_picker,
            self.logical_point(panel_rect, point),
        )
    }

    /// Whether `point` is over the picker's bottom "Import font…"
    /// (`ImportAction`) row (host import-row hover tracking).
    pub fn font_picker_import_action_at(&self, panel_rect: Rect, point: Point2D) -> bool {
        if self.is_multi || !self.font_picker.open || !self.font_import_supported {
            return false;
        }
        let entries = self.font_picker_entries();
        property_panel_typography::font_picker_import_action_at(
            self.scrolled_rect(self.logical_rect(panel_rect)),
            self.visible_sections(),
            &entries,
            self.font_import_supported,
            &self.font_picker,
            self.logical_point(panel_rect, point),
        )
    }

    /// Max scroll of the font-picker list (host wheel handler clamp).
    pub fn font_picker_max_scroll(&self, panel_rect: Rect) -> f32 {
        let entries = self.font_picker_entries();
        property_panel_typography::font_picker_max_scroll(
            self.scrolled_rect(self.logical_rect(panel_rect)),
            self.visible_sections(),
            &entries,
            self.font_import_supported,
        ) * self.density_scale
    }

    /// Whether `point` is inside an open image Search / Generate
    /// popover (host outside-click swallow).
    pub fn image_popovers_contain(&self, panel_rect: Rect, point: Point2D) -> bool {
        if self.is_multi {
            return false;
        }
        property_panel_image_assets::image_popovers_contain(
            self.scrolled_rect(self.logical_rect(panel_rect)),
            self.visible_sections(),
            &self.image_panel,
            self.image_gen_profile.as_ref(),
            self.logical_point(panel_rect, point),
        )
    }

    pub fn image_popover_input_at(
        &self,
        panel_rect: Rect,
        point: Point2D,
    ) -> Option<(property_panel_image_assets::ImagePopoverInputKind, usize)> {
        if self.is_multi {
            return None;
        }
        property_panel_image_assets::image_popover_input_at(
            self.scrolled_rect(self.logical_rect(panel_rect)),
            self.visible_sections(),
            &self.image_panel,
            self.image_gen_profile.as_ref(),
            self.logical_point(panel_rect, point),
        )
    }

    pub fn image_popover_input_drag_offset_at(
        &self,
        panel_rect: Rect,
        kind: property_panel_image_assets::ImagePopoverInputKind,
        point: Point2D,
    ) -> Option<usize> {
        if self.is_multi {
            return None;
        }
        property_panel_image_assets::image_popover_input_drag_offset_at(
            self.scrolled_rect(self.logical_rect(panel_rect)),
            self.visible_sections(),
            &self.image_panel,
            self.image_gen_profile.as_ref(),
            kind,
            self.logical_point(panel_rect, point),
        )
    }

    pub fn image_popover_input_caret_rect(&self, panel_rect: Rect) -> Option<Rect> {
        if self.is_multi {
            return None;
        }
        let logical = self.logical_rect(panel_rect);
        property_panel_image_assets::image_popover_input_caret_rect(
            self.scrolled_rect(logical),
            self.visible_sections(),
            &self.image_panel,
            self.image_gen_profile.as_ref(),
        )
        .map(|rect| self.physical_rect(logical, rect))
    }

    pub fn image_popover_input_geometry(
        &self,
        panel_rect: Rect,
        backend: &mut dyn crate::RenderBackend,
    ) -> Option<property_panel_image_assets::ImagePopoverInputGeometry> {
        if self.is_multi {
            return None;
        }
        let logical = self.logical_rect(panel_rect);
        property_panel_image_assets::image_popover_input_geometry(
            self.scrolled_rect(logical),
            self.visible_sections(),
            &self.image_panel,
            self.image_gen_profile.as_ref(),
            backend,
        )
        .map(|geometry| geometry.with_coordinate_scale(logical.origin, self.density_scale))
    }

    #[cfg(test)]
    pub(crate) fn visible_sections_for_test(
        &self,
    ) -> crate::widgets::property_panel_layout::VisibleSections {
        self.visible_sections()
    }
}
