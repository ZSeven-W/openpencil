//! Touch-only density for the Asset Center.
//!
//! The desktop gallery keeps its compact pointer controls. Phone and tablet
//! hosts share the same layout vocabulary, but every actionable surface is at
//! least 44 pt and the filter row wraps instead of escaping a narrow panel.

use op_editor_core::{AssetCenterTab, SceneFilter};

use super::panel_control_metrics::{CHIP_GAP, SEGMENT_TRACK_PAD};
use super::scene_template_card_actions::{card_action_rects, ACTION_GAP, ACTION_INSET};
use super::scene_template_panel::{
    chip_width, SceneTemplatePanel, CARD_GAP, CARD_MAX_W, CHIP_H, CLOSE_BTN, CONTROL_H,
    FILTER_ROW_H, GENERATE_BUTTON_W, GENERATE_INPUT_H, GENERATE_ROW_H, HEADER_H, SEARCH_ROW_H,
    STYLE_CARD_H, TAB_ROW_H,
};
use crate::Rect;

const TOUCH_TARGET: f32 = 44.0;
const TOUCH_CONTROL_H: f32 = 48.0;
const TOUCH_TAB_ROW_H: f32 = 60.0;
const TOUCH_SEARCH_ROW_H: f32 = 60.0;
const TOUCH_FILTER_ROW_H: f32 = 56.0;
const TOUCH_GENERATE_ROW_H: f32 = 84.0;
const TOUCH_GENERATE_BUTTON_W: f32 = 88.0;

#[derive(Debug, Clone, Copy)]
pub(super) struct SceneTemplateDensity {
    pub(super) chip_h: f32,
    pub(super) control_h: f32,
    pub(super) close_size: f32,
    pub(super) tab_row_h: f32,
    pub(super) search_row_h: f32,
    pub(super) filter_row_h: f32,
    pub(super) generate_row_h: f32,
    pub(super) generate_button_w: f32,
    pub(super) action_h: f32,
    pub(super) delete_size: f32,
}

impl SceneTemplatePanel<'_> {
    pub(super) fn density(&self) -> SceneTemplateDensity {
        if self.state.editor_ui.touch_chrome() {
            SceneTemplateDensity {
                chip_h: TOUCH_TARGET,
                control_h: TOUCH_CONTROL_H,
                close_size: TOUCH_TARGET,
                tab_row_h: TOUCH_TAB_ROW_H,
                search_row_h: TOUCH_SEARCH_ROW_H,
                filter_row_h: TOUCH_FILTER_ROW_H,
                generate_row_h: TOUCH_GENERATE_ROW_H,
                generate_button_w: TOUCH_GENERATE_BUTTON_W,
                action_h: TOUCH_TARGET,
                delete_size: TOUCH_TARGET,
            }
        } else {
            SceneTemplateDensity {
                chip_h: CHIP_H,
                control_h: CONTROL_H,
                close_size: CLOSE_BTN,
                tab_row_h: TAB_ROW_H,
                search_row_h: SEARCH_ROW_H,
                filter_row_h: FILTER_ROW_H,
                generate_row_h: GENERATE_ROW_H,
                generate_button_w: GENERATE_BUTTON_W,
                action_h: CHIP_H,
                delete_size: super::scene_template_style_geometry::STYLE_DELETE_BTN,
            }
        }
    }

    pub(super) fn touch_density_active(&self) -> bool {
        self.state.editor_ui.touch_chrome()
    }

    pub(super) fn compact_landscape_panel(&self, panel: Rect) -> bool {
        self.state.editor_ui.compact_layout() && panel.size.x > panel.size.y
    }

    pub(super) fn header_height_for(&self, panel: Rect) -> f32 {
        if self.compact_landscape_panel(panel) {
            48.0
        } else {
            HEADER_H
        }
    }

    pub(super) fn tab_row_height_for(&self, panel: Rect) -> f32 {
        if self.compact_landscape_panel(panel) {
            52.0
        } else {
            self.density().tab_row_h
        }
    }

    pub(super) fn search_row_height_for(&self, panel: Rect) -> f32 {
        if self.compact_landscape_panel(panel) {
            52.0
        } else {
            self.density().search_row_h
        }
    }

    pub fn close_rect_for(&self, panel: Rect) -> Rect {
        if !self.touch_density_active() {
            return Self::close_rect(panel);
        }
        let content = Self::content_rect(panel);
        let side = self.density().close_size;
        Rect::xywh(
            content.origin.x + content.size.x - side,
            panel.origin.y + (self.header_height_for(panel) - side) / 2.0,
            side,
            side,
        )
    }

    pub fn search_rect_for(&self, panel: Rect) -> Rect {
        if !self.touch_density_active() {
            return Self::search_rect(panel);
        }
        let content = Self::control_rect(panel);
        let density = self.density();
        Rect::xywh(
            content.origin.x,
            panel.origin.y
                + self.header_height_for(panel)
                + self.tab_row_height_for(panel)
                + (self.search_row_height_for(panel) - density.control_h) / 2.0,
            content.size.x,
            density.control_h,
        )
    }

    pub(super) fn tab_track_height_for(&self) -> f32 {
        if self.touch_density_active() {
            self.density().chip_h + SEGMENT_TRACK_PAD * 2.0
        } else {
            super::panel_control_metrics::SEGMENT_TRACK_H
        }
    }

    pub(super) fn filter_chip_layout(&self, panel: Rect) -> (Vec<(Rect, SceneFilter)>, f32) {
        if self.tab() != AssetCenterTab::Templates {
            return (Vec::new(), 0.0);
        }
        // Landscape phone height is the scarce axis. Search remains available;
        // the default All filter does not need a second permanent toolbar.
        if self.compact_landscape_panel(panel) {
            return (Vec::new(), 0.0);
        }
        let density = self.density();
        let content = Self::content_rect(panel);
        let left = content.origin.x;
        let right = left + content.size.x;
        let mut x = left;
        let mut row = 0_usize;
        let mut rects = Vec::new();
        for filter in self.filters() {
            let width = chip_width(self.filter_label(filter)).min(content.size.x);
            if self.touch_density_active() && x > left && x + width > right {
                row += 1;
                x = left;
            }
            let row_top = panel.origin.y
                + self.header_height_for(panel)
                + self.tab_row_height_for(panel)
                + self.search_row_height_for(panel)
                + row as f32 * density.filter_row_h;
            let rect = Rect::xywh(
                x,
                row_top + (density.filter_row_h - density.chip_h) / 2.0,
                width,
                density.chip_h,
            );
            rects.push((rect, filter));
            x += width + CHIP_GAP;
        }
        let rows = row + 1;
        (rects, rows as f32 * density.filter_row_h)
    }

    pub(super) fn filter_row_height_for(&self, panel: Rect) -> f32 {
        self.filter_chip_layout(panel).1
    }

    pub(super) fn generate_row_height_for(&self, panel: Rect) -> f32 {
        if self.generate_row_visible_in(panel) {
            self.density().generate_row_h
        } else {
            0.0
        }
    }

    pub(super) fn generate_row_visible_in(&self, panel: Rect) -> bool {
        self.generate_row_visible() && !self.compact_landscape_panel(panel)
    }

    pub(super) fn generate_input_height_for(&self) -> f32 {
        if self.touch_density_active() {
            self.density().control_h
        } else {
            GENERATE_INPUT_H
        }
    }

    pub(super) fn generate_button_width_for(&self) -> f32 {
        self.density().generate_button_w
    }

    pub(super) fn card_action_rects_for(
        &self,
        card: Rect,
        with_generate: bool,
    ) -> (Rect, Option<Rect>) {
        if !self.touch_density_active() {
            return card_action_rects(card, with_generate);
        }
        let preview = Self::card_preview_rect(card);
        let row = Rect::xywh(
            preview.origin.x + ACTION_INSET,
            preview.origin.y + preview.size.y - ACTION_INSET - self.density().action_h,
            (preview.size.x - ACTION_INSET * 2.0).max(0.0),
            self.density().action_h,
        );
        split_action_row(row, with_generate)
    }

    pub(super) fn style_delete_rect_for(&self, card: Rect) -> Rect {
        if !self.touch_density_active() {
            return Self::style_delete_rect(card);
        }
        let side = self.density().delete_size;
        Rect::xywh(
            card.origin.x + card.size.x - side - 8.0,
            card.origin.y + 8.0,
            side,
            side,
        )
    }

    pub(super) fn style_import_action_height(&self) -> f32 {
        if self.touch_density_active() {
            TOUCH_TARGET
        } else {
            32.0
        }
    }

    pub(super) fn touch_grid_columns(&self, viewport_w: f32) -> Option<usize> {
        if !self.touch_density_active() {
            return None;
        }
        self.state
            .editor_ui
            .compact_layout()
            .then_some(1_usize)
            .or_else(|| {
                self.state.editor_ui.medium_layout().then(|| {
                    (((viewport_w + CARD_GAP) / (CARD_MAX_W + CARD_GAP)).ceil() as usize).max(2)
                })
            })
    }

    pub(super) fn touch_style_card_height(&self) -> f32 {
        if self.touch_density_active() {
            STYLE_CARD_H.max(128.0)
        } else {
            STYLE_CARD_H
        }
    }
}

fn split_action_row(row: Rect, with_generate: bool) -> (Rect, Option<Rect>) {
    if !with_generate {
        return (row, None);
    }
    let half = ((row.size.x - ACTION_GAP) / 2.0).max(0.0);
    (
        Rect::xywh(row.origin.x, row.origin.y, half, row.size.y),
        Some(Rect::xywh(
            row.origin.x + half + ACTION_GAP,
            row.origin.y,
            half,
            row.size.y,
        )),
    )
}
