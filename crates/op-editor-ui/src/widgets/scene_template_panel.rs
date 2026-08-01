//! Platform-neutral Scene Template Center geometry, filtering, and hit testing.
//!
//! Same contract as the Prompt Center: hosts supply the panel rect and route
//! the returned hit through their shared press flow, and the widget reads only
//! [`EditorState`] so both the native and wasm hosts can use it.
//!
//! The two panels look alike on purpose — a user who has met one should not
//! have to learn the other — but they answer different questions. A prompt
//! ends up in the chat input; a template opens as a document. That is why the
//! only card action here is "open", and why the panel carries no save form.

use op_editor_core::scene_template_catalog::{
    scene_template_catalogue, SceneTemplateDefinition, TemplateScene,
};
use op_editor_core::{ButtonPressTarget, EditorState, Locale, SceneFilter};

use super::prompt_center_panel::estimated_text_width;
use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::{Point2D, Rect};

/// Scene Template Center width in logical pixels.
pub const SCENE_TEMPLATE_PANEL_W: f32 = 720.0;
/// Scene Template Center height in logical pixels.
pub const SCENE_TEMPLATE_PANEL_H: f32 = 520.0;

/// Hover token for the close button.
pub const SCENE_TEMPLATE_CLOSE_HOVER: usize = usize::MAX;

const FILTER_HOVER_BASE: usize = usize::MAX - 32;

pub(super) const PAD: f32 = 16.0;
pub(super) const HEADER_H: f32 = 46.0;
pub(super) const SEARCH_ROW_H: f32 = 42.0;
pub(super) const FILTER_ROW_H: f32 = 40.0;
pub(super) const CLOSE_BTN: f32 = 26.0;
const SEARCH_H: f32 = 30.0;
pub(super) const SEARCH_TEXT_SIZE: f32 = 12.0;
pub(super) const CHIP_H: f32 = 24.0;
const CHIP_GAP: f32 = 6.0;
const CARD_COLS: usize = 2;
const CARD_GAP: f32 = 12.0;
pub(super) const CARD_H: f32 = 262.0;
pub(super) const CARD_PREVIEW_INSET: f32 = 8.0;
pub(super) const CARD_PREVIEW_ASPECT: f32 = 16.0 / 10.0;

/// A hover token for the filter chip at `index`.
pub(super) fn filter_hover_token(index: usize) -> usize {
    FILTER_HOVER_BASE + index
}

/// What a press inside the panel resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneTemplateHit {
    Close,
    FocusSearch(usize),
    SelectFilter(SceneFilter),
    /// Open this template as a new document.
    SelectTemplate(String),
    /// Inside the panel but not on a control — swallows the press so it
    /// cannot fall through to the canvas underneath.
    Inside,
}

/// Floating Scene Template Center view model.
pub struct SceneTemplatePanel<'a> {
    pub(super) state: &'a EditorState,
    pub(super) theme: Theme,
    pub(super) locale: Locale,
    pub(super) now_ms: u64,
}

impl<'a> SceneTemplatePanel<'a> {
    /// Build the panel when it is open.
    pub fn for_editor(state: &'a EditorState) -> Option<Self> {
        Self::for_editor_at(state, 0)
    }

    /// Build the panel with a frame clock for caret blinking.
    pub fn for_editor_at(state: &'a EditorState, now_ms: u64) -> Option<Self> {
        state.editor_ui.scene_template_center.open.then(|| Self {
            state,
            theme: theme_for(&state.editor_ui),
            locale: state.editor_ui.locale,
            now_ms,
        })
    }

    pub(super) fn now_ms(&self) -> u64 {
        self.now_ms
    }

    pub(super) fn is_pressed(&self, token: usize) -> bool {
        matches!(
            self.state.editor_ui.pressed_button,
            Some(ButtonPressTarget::SceneTemplate(pressed)) if pressed == token
        )
    }

    /// Templates surviving the scene filter and the search query.
    pub fn filtered(&self) -> Vec<&'static SceneTemplateDefinition> {
        let center = &self.state.editor_ui.scene_template_center;
        let query = center.search.text().trim();
        scene_template_catalogue()
            .iter()
            .filter(|template| match center.filter {
                SceneFilter::All => true,
                SceneFilter::Scene(scene) => template.scene == scene,
            })
            .filter(|template| template.matches_query(self.locale, query))
            .collect()
    }

    /// The chip row: "All" plus every scene, in catalogue order.
    pub(super) fn filters(&self) -> Vec<SceneFilter> {
        let mut filters = vec![SceneFilter::All];
        filters.extend(TemplateScene::ALL.map(SceneFilter::Scene));
        filters
    }

    /// Label for one chip.
    pub(super) fn filter_label(&self, filter: SceneFilter) -> &'static str {
        match filter {
            SceneFilter::All => {
                let translated = op_i18n::translate(self.locale, "sceneTemplate.filter.all");
                if translated == "sceneTemplate.filter.all" {
                    "全部"
                } else {
                    translated
                }
            }
            SceneFilter::Scene(scene) => {
                let translated = op_i18n::translate(self.locale, scene.title_key());
                if translated == scene.title_key() {
                    scene.title_fallback()
                } else {
                    translated
                }
            }
        }
    }

    pub fn close_rect(panel: Rect) -> Rect {
        Rect::xywh(
            panel.origin.x + panel.size.x - PAD - CLOSE_BTN,
            panel.origin.y + (HEADER_H - CLOSE_BTN) / 2.0,
            CLOSE_BTN,
            CLOSE_BTN,
        )
    }

    pub fn search_rect(panel: Rect) -> Rect {
        Rect::xywh(
            panel.origin.x + PAD,
            panel.origin.y + HEADER_H + (SEARCH_ROW_H - SEARCH_H) / 2.0,
            panel.size.x - PAD * 2.0,
            SEARCH_H,
        )
    }

    pub(super) fn filter_chip_rects(&self, panel: Rect) -> Vec<(Rect, SceneFilter)> {
        let top = panel.origin.y + HEADER_H + SEARCH_ROW_H + (FILTER_ROW_H - CHIP_H) / 2.0;
        let mut x = panel.origin.x + PAD;
        self.filters()
            .into_iter()
            .map(|filter| {
                let width = chip_width(self.filter_label(filter));
                let rect = Rect::xywh(x, top, width, CHIP_H);
                x += width + CHIP_GAP;
                (rect, filter)
            })
            .collect()
    }

    pub(super) fn cards_top(&self, panel: Rect) -> f32 {
        panel.origin.y + HEADER_H + SEARCH_ROW_H + FILTER_ROW_H
    }

    pub fn cards_viewport(&self, panel: Rect) -> Rect {
        let top = self.cards_top(panel);
        Rect::xywh(
            panel.origin.x + PAD,
            top,
            panel.size.x - PAD * 2.0,
            (panel.origin.y + panel.size.y - PAD - top).max(0.0),
        )
    }

    fn rows_for_count(count: usize) -> usize {
        count.div_ceil(CARD_COLS)
    }

    pub(super) fn content_height_for_count(count: usize) -> f32 {
        let rows = Self::rows_for_count(count);
        if rows == 0 {
            0.0
        } else {
            rows as f32 * CARD_H + (rows - 1) as f32 * CARD_GAP
        }
    }

    /// Largest legal scroll offset for the current result set.
    pub fn max_scroll(&self, panel: Rect) -> f32 {
        self.max_scroll_for_count(panel, self.filtered().len())
    }

    pub(super) fn max_scroll_for_count(&self, panel: Rect, count: usize) -> f32 {
        let viewport = self.cards_viewport(panel);
        (Self::content_height_for_count(count) - viewport.size.y).max(0.0)
    }

    pub(super) fn card_rects_for_count(&self, panel: Rect, count: usize) -> Vec<(usize, Rect)> {
        let viewport = self.cards_viewport(panel);
        let card_w = (viewport.size.x - CARD_GAP) / CARD_COLS as f32;
        let scroll = self
            .state
            .editor_ui
            .scene_template_center
            .scroll
            .offset
            .clamp(0.0, self.max_scroll_for_count(panel, count));
        (0..count)
            .map(|index| {
                let row = index / CARD_COLS;
                let column = index % CARD_COLS;
                let rect = Rect::xywh(
                    viewport.origin.x + column as f32 * (card_w + CARD_GAP),
                    viewport.origin.y + row as f32 * (CARD_H + CARD_GAP) - scroll,
                    card_w,
                    CARD_H,
                );
                (index, rect)
            })
            .collect()
    }

    pub(super) fn card_rects(&self, panel: Rect) -> Vec<(usize, Rect)> {
        self.card_rects_for_count(panel, self.filtered().len())
    }

    /// Resolve a pointer to a hover token shared with paint.
    pub fn hover_at(&self, panel: Rect, point: Point2D) -> Option<usize> {
        if !panel.contains(point) {
            return None;
        }
        if Self::close_rect(panel).contains(point) {
            return Some(SCENE_TEMPLATE_CLOSE_HOVER);
        }
        for (index, (rect, _)) in self.filter_chip_rects(panel).into_iter().enumerate() {
            if rect.contains(point) {
                return Some(filter_hover_token(index));
            }
        }
        // A card scrolled out of the viewport must not hover: its rect is
        // still computed (paint clips it), so the viewport check is what
        // keeps a pointer below the panel from lighting up a hidden row.
        let viewport = self.cards_viewport(panel);
        if !viewport.contains(point) {
            return None;
        }
        self.card_rects(panel)
            .into_iter()
            .find(|(_, rect)| rect.contains(point))
            .map(|(index, _)| index)
    }

    /// Hit-test panel chrome and cards. Outside presses return `None` so the
    /// caller can treat them as dismiss.
    pub fn hit_test(&self, panel: Rect, point: Point2D) -> Option<SceneTemplateHit> {
        if !panel.contains(point) {
            return None;
        }
        if Self::close_rect(panel).contains(point) {
            return Some(SceneTemplateHit::Close);
        }
        let search = Self::search_rect(panel);
        if search.contains(point) {
            let caret = self.search_caret_at(search, point);
            return Some(SceneTemplateHit::FocusSearch(caret));
        }
        for (rect, filter) in self.filter_chip_rects(panel) {
            if rect.contains(point) {
                return Some(SceneTemplateHit::SelectFilter(filter));
            }
        }
        let viewport = self.cards_viewport(panel);
        if viewport.contains(point) {
            let cards = self.filtered();
            for (index, rect) in self.card_rects_for_count(panel, cards.len()) {
                if rect.contains(point) {
                    return Some(SceneTemplateHit::SelectTemplate(cards[index].id.clone()));
                }
            }
        }
        Some(SceneTemplateHit::Inside)
    }

    /// Caret index for a press inside the search field.
    fn search_caret_at(&self, search: Rect, point: Point2D) -> usize {
        let text = self.state.editor_ui.scene_template_center.search.text();
        let relative = (point.x - (search.origin.x + 32.0)).max(0.0);
        let mut width = 0.0;
        for (index, character) in text.char_indices() {
            let advance = estimated_text_width(&character.to_string(), SEARCH_TEXT_SIZE);
            if relative < width + advance / 2.0 {
                return index;
            }
            width += advance;
        }
        text.len()
    }
}

/// Chip label size, shared by the rect math here and the paint pass.
pub(super) const CHIP_LABEL_SIZE: f32 = 11.0;

fn chip_width(label: &str) -> f32 {
    // Reuses the Prompt Center's estimate on purpose: the two chip rows sit
    // in identically sized panels, and a second width model would drift them
    // apart for the same label.
    estimated_text_width(label, CHIP_LABEL_SIZE) + 20.0
}

#[cfg(test)]
#[path = "scene_template_panel_tests.rs"]
mod scene_template_panel_tests;
