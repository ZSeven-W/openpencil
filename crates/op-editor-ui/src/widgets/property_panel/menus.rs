//! Floating-menu geometry + hit-testing for the `PropertyPanel` —
//! the Effects "+" add-menu and the Interactions Navigate/Back/Remove
//! popover.
//!
//! Split out of `property_panel.rs` to keep both files under the
//! openpencil 800-line cap.

use super::{EffectAddMenuHit, PropertyPanel, PropertyPanelAction};
use crate::widgets::property_panel_interactions::InteractionMenuHit;
use crate::widgets::property_panel_sections as sections;
use crate::{Point2D, Rect};

impl PropertyPanel {
    /// Hit-test the Effects "+" add-menu against `point` (panel space).
    /// `Row` = a choice was clicked, `Inside` = swallow (keep open),
    /// `Outside` = dismiss. Only meaningful while the menu is open.
    pub fn effect_add_menu_hit(&self, panel_rect: Rect, point: Point2D) -> EffectAddMenuHit {
        self.effect_add_menu_hit_logical(
            self.logical_rect(panel_rect),
            self.logical_point(panel_rect, point),
        )
    }

    pub(super) fn effect_add_menu_hit_logical(
        &self,
        panel_rect: Rect,
        point: Point2D,
    ) -> EffectAddMenuHit {
        let Some(menu) = self.effect_add_menu_rect_logical(panel_rect) else {
            return EffectAddMenuHit::Outside;
        };
        for (action, row) in crate::widgets::property_panel_effects::effect_add_menu_row_rects(menu)
        {
            if row.contains(point) {
                return EffectAddMenuHit::Row(action);
            }
        }
        if menu.contains(point) {
            EffectAddMenuHit::Inside
        } else {
            EffectAddMenuHit::Outside
        }
    }

    /// Exact painted bounds of the open Effects add-menu, including its
    /// padded chrome. Hosts use this as the O(1) ownership/occlusion test;
    /// deriving it here keeps popup paint, hit, and hover geometry identical.
    pub fn effect_add_menu_rect(&self, panel_rect: Rect) -> Option<Rect> {
        let logical = self.logical_rect(panel_rect);
        self.effect_add_menu_rect_logical(logical)
            .map(|rect| self.physical_rect(logical, rect))
    }

    fn effect_add_menu_rect_logical(&self, panel_rect: Rect) -> Option<Rect> {
        if !self.effect_add_picker_open {
            return None;
        }
        self.effect_add_button_rect(self.scrolled_rect(panel_rect))
            .map(crate::widgets::property_panel_effects::effect_add_menu_rect)
    }

    /// Whether `point` lies anywhere inside the open Effects add-menu chrome.
    pub fn effect_add_menu_contains(&self, panel_rect: Rect, point: Point2D) -> bool {
        self.effect_add_menu_rect(panel_rect)
            .is_some_and(|menu| menu.contains(point))
    }

    /// Row index under `point` in the open Effects add-menu — drives the
    /// hover highlight (mirrors [`Self::export_picker_row_at`]).
    pub fn effect_add_menu_row_at(&self, panel_rect: Rect, point: Point2D) -> Option<usize> {
        let point = self.logical_point(panel_rect, point);
        let panel_rect = self.logical_rect(panel_rect);
        let menu = self.effect_add_menu_rect_logical(panel_rect)?;
        crate::widgets::property_panel_effects::effect_add_menu_row_rects(menu)
            .into_iter()
            .position(|(_, row)| row.contains(point))
    }

    /// The Effects section "+" button rect — `scrolled` is the already
    /// scroll-adjusted panel rect (`scrolled_rect`). The anchor the
    /// add-menu popover drops from.
    pub(crate) fn effect_add_button_rect(&self, scrolled: Rect) -> Option<Rect> {
        sections::action_button_rects_with_fill_picker(
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
        )
        .into_iter()
        .find(|(a, _)| matches!(a, PropertyPanelAction::ToggleEffectAddPicker))
        .map(|(_, r)| r)
    }

    /// Whether the Interactions popover's "Remove" row shows — only
    /// when there is an existing single `onTap` action to remove (the
    /// empty "+ Add interaction" state has nothing to remove; a
    /// multi-action `onTap` doesn't open this popover at all — see
    /// `interaction_menu_anchor_rect`).
    pub(super) fn interaction_menu_removable(&self) -> bool {
        self.snapshot.interactions.on_tap.len() == 1
    }

    /// Hit-test the Interactions section's Navigate/Back/Remove popover
    /// against `point` (panel space). Mirrors [`Self::effect_add_menu_hit`].
    pub fn interaction_menu_hit(&self, panel_rect: Rect, point: Point2D) -> InteractionMenuHit {
        self.interaction_menu_hit_logical(
            self.logical_rect(panel_rect),
            self.logical_point(panel_rect, point),
        )
    }

    pub(super) fn interaction_menu_hit_logical(
        &self,
        panel_rect: Rect,
        point: Point2D,
    ) -> InteractionMenuHit {
        let Some(anchor) = self.interaction_menu_anchor_rect(self.scrolled_rect(panel_rect)) else {
            return InteractionMenuHit::Outside;
        };
        let rows = crate::widgets::property_panel_interactions::interaction_menu_rows(
            self.locale,
            &self.screen_paths,
            self.interaction_menu_removable(),
        );
        let menu =
            crate::widgets::property_panel_interactions::interaction_menu_rect(anchor, rows.len());
        for (action, row) in
            crate::widgets::property_panel_interactions::interaction_menu_row_rects(menu, &rows)
        {
            if row.contains(point) {
                return InteractionMenuHit::Row(action);
            }
        }
        if menu.contains(point) {
            InteractionMenuHit::Inside
        } else {
            InteractionMenuHit::Outside
        }
    }

    /// Row index under `point` in the open Interactions popover — drives
    /// the hover highlight (mirrors [`Self::effect_add_menu_row_at`]).
    pub fn interaction_menu_row_at(&self, panel_rect: Rect, point: Point2D) -> Option<usize> {
        let point = self.logical_point(panel_rect, point);
        let panel_rect = self.logical_rect(panel_rect);
        let anchor = self.interaction_menu_anchor_rect(self.scrolled_rect(panel_rect))?;
        let rows = crate::widgets::property_panel_interactions::interaction_menu_rows(
            self.locale,
            &self.screen_paths,
            self.interaction_menu_removable(),
        );
        let menu =
            crate::widgets::property_panel_interactions::interaction_menu_rect(anchor, rows.len());
        crate::widgets::property_panel_interactions::interaction_menu_row_rects(menu, &rows)
            .into_iter()
            .position(|(_, row)| row.contains(point))
    }

    /// The Interactions section's clickable tap-row rect
    /// (`ToggleInteractionMenu`'s rect) — the popover drops from here.
    /// `None` when the current `onTap` list has more than one action
    /// (only "Remove all" is clickable then — no popover).
    pub(crate) fn interaction_menu_anchor_rect(&self, scrolled: Rect) -> Option<Rect> {
        sections::action_button_rects_with_fill_picker(
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
        )
        .into_iter()
        .find(|(a, _)| matches!(a, PropertyPanelAction::ToggleInteractionMenu))
        .map(|(_, r)| r)
    }
}
