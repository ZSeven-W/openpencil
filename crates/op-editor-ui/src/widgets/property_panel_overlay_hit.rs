//! `PropertyPanel` accessors for the searchable font-family picker
//! and the image Search / Generate popovers — host-facing contains /
//! hover / scroll helpers, split out of `property_panel.rs` for the
//! 800-line cap (same `impl PropertyPanel`).

use crate::widgets::property_panel::PropertyPanel;
use crate::widgets::{property_panel_image_assets, property_panel_typography};
use crate::{Point2D, Rect};
use jian_widgets::components::select::SelectHit;

impl PropertyPanel {
    pub fn fill_type_picker_hit(&self, panel_rect: Rect, point: Point2D) -> SelectHit {
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
            point,
            &self.theme,
        )
    }

    pub fn fill_type_picker_row_at(&self, panel_rect: Rect, point: Point2D) -> Option<usize> {
        match self.fill_type_picker_hit(panel_rect, point) {
            SelectHit::Row(idx) => Some(idx),
            SelectHit::Inside | SelectHit::Outside => None,
        }
    }

    /// Visible font-picker entries for the current search + host
    /// enumeration — paint, hit-test, and the host dispatch resolve
    /// `SetFontFamilyIndex` against this same list.
    pub fn font_picker_entries(&self) -> Vec<property_panel_typography::FontPickerEntry<'_>> {
        property_panel_typography::font_picker_entries(
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
            self.scrolled_rect(panel_rect),
            self.visible_sections(),
            &entries,
            point,
        )
    }

    /// Font-picker entry index under `point` (hover tracking).
    pub fn font_picker_entry_index_at(&self, panel_rect: Rect, point: Point2D) -> Option<usize> {
        if self.is_multi || !self.font_picker.open {
            return None;
        }
        let entries = self.font_picker_entries();
        property_panel_typography::font_picker_entry_index_at(
            self.scrolled_rect(panel_rect),
            self.visible_sections(),
            &entries,
            &self.font_picker,
            point,
        )
    }

    /// Max scroll of the font-picker list (host wheel handler clamp).
    pub fn font_picker_max_scroll(&self, panel_rect: Rect) -> f32 {
        let entries = self.font_picker_entries();
        property_panel_typography::font_picker_max_scroll(
            self.scrolled_rect(panel_rect),
            self.visible_sections(),
            &entries,
        )
    }

    /// Whether `point` is inside an open image Search / Generate
    /// popover (host outside-click swallow).
    pub fn image_popovers_contain(&self, panel_rect: Rect, point: Point2D) -> bool {
        if self.is_multi {
            return false;
        }
        property_panel_image_assets::image_popovers_contain(
            self.scrolled_rect(panel_rect),
            self.visible_sections(),
            &self.image_panel,
            self.image_gen_profile.as_ref(),
            point,
        )
    }

    #[cfg(test)]
    pub(crate) fn visible_sections_for_test(
        &self,
    ) -> crate::widgets::property_panel_layout::VisibleSections {
        self.visible_sections()
    }
}
