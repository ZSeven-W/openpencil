//! Non-modal font fallback notice for the normal workspace and touch reader.

use crate::widgets::editor_state_ext::{theme_for, translate};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{text_metrics, PaintCx};
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::EditorState;

pub const NOTICE_HEIGHT: f32 = 52.0;
const HIT: f32 = 44.0;
const MANAGE_W: f32 = 84.0;
const FONT_SIZE: f32 = 12.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Manage,
    Dismiss,
    Inside,
    Outside,
}

pub struct MissingFontsNotice<'a> {
    state: &'a EditorState,
}

impl<'a> MissingFontsNotice<'a> {
    pub fn for_editor(state: &'a EditorState) -> Option<Self> {
        let ui = &state.editor_ui;
        (ui.workspace.visible
            && !ui.missing_fonts_modal_open
            && !ui.missing_fonts_notice_dismissed
            && !ui.agent_settings_open
            && ui
                .missing_fonts_prompt
                .as_ref()
                .is_some_and(|p| !p.entries.is_empty()))
        .then_some(Self { state })
    }
    pub fn rect(&self, viewport_w: f32, viewport_h: f32) -> Option<Rect> {
        let canvas =
            crate::widgets::host_canvas_geometry::canvas_rect(self.state, viewport_w, viewport_h);
        let width = (canvas.size.x - 24.0).min(560.0);
        (width >= 216.0 && canvas.size.y >= NOTICE_HEIGHT + 24.0).then_some(Rect::xywh(
            canvas.origin.x + (canvas.size.x - width) / 2.0,
            canvas.origin.y + 12.0,
            width,
            NOTICE_HEIGHT,
        ))
    }
    pub fn manage_rect(rect: Rect) -> Rect {
        Rect::xywh(
            rect.origin.x + rect.size.x - 6.0 - HIT - 4.0 - MANAGE_W,
            rect.origin.y + (NOTICE_HEIGHT - HIT) / 2.0,
            MANAGE_W,
            HIT,
        )
    }
    pub fn dismiss_rect(rect: Rect) -> Rect {
        Rect::xywh(
            rect.origin.x + rect.size.x - 6.0 - HIT,
            rect.origin.y + (NOTICE_HEIGHT - HIT) / 2.0,
            HIT,
            HIT,
        )
    }
    pub fn hit_test(rect: Rect, point: Point2D) -> Hit {
        if !rect.contains(point) {
            Hit::Outside
        } else if Self::dismiss_rect(rect).contains(point) {
            Hit::Dismiss
        } else if Self::manage_rect(rect).contains(point) {
            Hit::Manage
        } else {
            Hit::Inside
        }
    }
    pub fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        let ui = &self.state.editor_ui;
        let theme = theme_for(ui);
        cx.backend.fill_round_rect(rect, 8.0, theme.popover);
        cx.backend.stroke_round_rect(rect, 8.0, theme.border, 1.0);
        let manage = Self::manage_rect(rect);
        let available = (manage.origin.x - rect.origin.x - 20.0).max(0.0);
        let message = text_metrics::fit_chrome(
            cx.backend,
            translate(ui, "missingFonts.fallbackNotice"),
            available,
            FONT_SIZE,
        );
        let label = text_metrics::fit_chrome(
            cx.backend,
            translate(ui, "missingFonts.manage"),
            MANAGE_W - 16.0,
            FONT_SIZE,
        );
        let baseline = jian_widgets::centered_text_baseline_y(rect, FONT_SIZE);
        let text = TextLayout::single_run(
            &message,
            "system-ui",
            FONT_SIZE,
            theme.popover_foreground.to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&text, Point2D::new(rect.origin.x + 12.0, baseline));
        cx.backend.fill_round_rect(manage, 6.0, theme.secondary);
        let label_width = text_metrics::measure_chrome(cx.backend, &label, FONT_SIZE);
        let label_text = TextLayout::single_run(
            &label,
            "system-ui",
            FONT_SIZE,
            theme.secondary_foreground.to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &label_text,
            Point2D::new(manage.origin.x + (MANAGE_W - label_width) / 2.0, baseline),
        );
        let close = Self::dismiss_rect(rect);
        draw_icon(
            cx.backend,
            Icon::Close,
            Point2D::new(close.origin.x + 16.0, close.origin.y + 16.0),
            12.0,
            theme.muted_foreground,
            1.4,
        );
    }
}

pub fn paint(cx: &mut PaintCx<'_>, state: &EditorState, viewport_w: f32, viewport_h: f32) {
    if let Some(notice) = MissingFontsNotice::for_editor(state) {
        if let Some(rect) = notice.rect(viewport_w, viewport_h) {
            notice.paint(cx, rect);
        }
    }
}

/// The camera's available area while the notice is visible. Keep the real
/// canvas origin/clip unchanged so input and document coordinates never move.
/// Only automatic fits reserve room above the artwork for the notice.
pub fn fit_canvas_rect(state: &EditorState, viewport_w: f32, viewport_h: f32) -> Rect {
    let mut canvas =
        crate::widgets::host_canvas_geometry::canvas_rect(state, viewport_w, viewport_h);
    if let Some(rect) =
        MissingFontsNotice::for_editor(state).and_then(|notice| notice.rect(viewport_w, viewport_h))
    {
        let inset = (rect.origin.y + rect.size.y + 12.0 - canvas.origin.y)
            .min((canvas.size.y - 1.0).max(0.0));
        canvas.origin.y += inset;
        canvas.size.y -= inset;
    }
    canvas
}

/// Only the notice itself consumes input; reading and canvas actions keep working.
pub fn press(state: &mut EditorState, point: Point2D, viewport_w: f32, viewport_h: f32) -> bool {
    let hit = MissingFontsNotice::for_editor(state)
        .and_then(|notice| notice.rect(viewport_w, viewport_h))
        .map(|rect| MissingFontsNotice::hit_test(rect, point));
    match hit {
        Some(Hit::Manage) => {
            state.editor_ui.missing_fonts_modal_open = true;
            state.editor_ui.missing_fonts_scroll.offset = 0.0;
            true
        }
        Some(Hit::Dismiss) => {
            state.editor_ui.missing_fonts_notice_dismissed = true;
            true
        }
        Some(Hit::Inside) => true,
        Some(Hit::Outside) | None => false,
    }
}

#[cfg(test)]
#[path = "missing_fonts_notice_tests.rs"]
mod tests;
