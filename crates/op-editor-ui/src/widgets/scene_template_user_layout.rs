//! Templates-tab geometry for the user-saved half of the grid.
//!
//! Split from `scene_template_panel.rs` at the file-size cap, the same way
//! `scene_template_style_geometry.rs` owns the Styles tab's geometry. The
//! Templates tab was a plain uniform grid until saved templates arrived; now
//! it is two sections — what the user saved and what ships with the app —
//! and this module owns every rect that depends on that shape: the saved
//! cards' view models, and the sectioned walker that places both halves.
//!
//! With no saved templates the walker degenerates to the flat grid that
//! predates them, so the shipped-only case is untouched.

use super::asset_center_style_layout::{style_grid_layout, StyleGridLayout, StyleGridMetrics};
use super::asset_center_template_cards::{
    user_template_card_count, user_template_cards, UserTemplateCard,
};
use super::scene_template_panel::{SceneTemplatePanel, CARD_GAP};
use crate::Rect;

impl SceneTemplatePanel<'_> {
    /// Saved templates surviving the search query, in registry order.
    pub(super) fn user_cards(&self) -> Vec<UserTemplateCard> {
        user_template_cards(self.state.editor_ui.scene_template_center.search.text())
    }

    /// How many saved templates are showing. Kept separate from
    /// [`SceneTemplatePanel::filtered`] because the two halves of the grid
    /// have different ids, filters, and hover behaviour — the only thing
    /// they share is the walker that places their cards.
    pub(super) fn user_card_count(&self) -> usize {
        user_template_card_count(self.state.editor_ui.scene_template_center.search.text())
    }

    /// The Templates grid: saved-first, with a section heading when both
    /// halves are present — the same walker the Styles tab uses.
    ///
    /// With no saved templates it degenerates to the flat grid that predates
    /// them, so the shipped-only case is untouched.
    pub(super) fn template_layout(&self, panel: Rect) -> StyleGridLayout {
        let user_count = self.user_card_count();
        let builtin_count = self.filtered().len();
        let viewport = self.cards_viewport(panel);
        let (columns, card_w, card_h) = self.grid_metrics(panel);
        let metrics = StyleGridMetrics {
            columns,
            card_w,
            card_h,
            card_gap: CARD_GAP,
            user_count,
            total: user_count + builtin_count,
        };
        // Height first, unscrolled, so the offset can be clamped against a
        // limit that already counts the headings.
        let height = style_grid_layout(viewport, &metrics, 0.0).content_height;
        let scroll = self
            .state
            .editor_ui
            .scene_template_center
            .scroll
            .offset
            .clamp(0.0, (height - viewport.size.y).max(0.0));
        style_grid_layout(viewport, &metrics, scroll)
    }
}
