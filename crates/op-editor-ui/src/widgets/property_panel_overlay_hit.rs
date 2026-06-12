//! `PropertyPanel` accessors for the searchable font-family picker
//! and the image Search / Generate popovers — host-facing contains /
//! hover / scroll helpers, split out of `property_panel.rs` for the
//! 800-line cap (same `impl PropertyPanel`).

use crate::widgets::property_panel::PropertyPanel;
use crate::widgets::{property_panel_image_assets, property_panel_typography};
use crate::{Point2D, Rect};

impl PropertyPanel {
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
        if self.is_multi || !self.font_family_picker_open {
            return false;
        }
        let entries = self.font_picker_entries();
        property_panel_typography::font_picker_contains(
            self.scrolled_rect(panel_rect),
            self.visible_sections(),
            &entries,
            self.font_picker_scroll,
            point,
        )
    }

    /// Font-picker entry index under `point` (hover tracking).
    pub fn font_picker_entry_index_at(&self, panel_rect: Rect, point: Point2D) -> Option<usize> {
        if self.is_multi || !self.font_family_picker_open {
            return None;
        }
        let entries = self.font_picker_entries();
        property_panel_typography::font_picker_entry_index_at(
            self.scrolled_rect(panel_rect),
            self.visible_sections(),
            &entries,
            self.font_picker_scroll,
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
