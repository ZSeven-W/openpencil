//! The variants bar: one pill per landed design direction along the
//! bottom of the workspace canvas — `方案 A · Editorial Dark [用这个]` —
//! once a side-by-side run has settled.
//!
//! Like the other canvas banners it paints AFTER the real canvas (from
//! [`WorkspaceSurface::paint_canvas_banners`]). Only the "use this" button
//! answers a press; the rest of the pill lets presses through to the
//! canvas, so a stray click on a label never keeps a direction.

use super::paint::{fade, text, tr, SANS};
use super::{StudioPalette, WorkspaceLayout, WorkspaceSurface};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};
use op_editor_core::WorkspaceHit;

const PILL_H: f32 = 40.0;
const PILL_GAP: f32 = 10.0;
const PILL_PAD_X: f32 = 14.0;
/// Clears the canvas's own floor controls (zoom pill) underneath.
const BOTTOM_INSET: f32 = 64.0;
const BUTTON_H: f32 = 28.0;
const BUTTON_PAD_X: f32 = 12.0;
const LABEL_FONT: f32 = 13.0;
const BUTTON_FONT: f32 = 12.0;
const SIDE_MARGIN: f32 = 16.0;

/// One direction's pill and its "use this" button.
#[derive(Debug, Clone, PartialEq)]
pub struct VariantBarItem {
    /// Slot of the direction (`0` = A).
    pub index: usize,
    /// The label painted in the pill (possibly shortened to fit).
    pub label: String,
    pub pill: Rect,
    pub button: Rect,
}

/// Rough advance of `text` at `size` — CJK glyphs are about one em wide,
/// Latin about 0.56 em. Layout must not depend on a backend, so it
/// estimates; paint measures and centres inside the estimated box.
fn estimate_text_w(text: &str, size: f32) -> f32 {
    text.chars()
        .map(|c| if c.is_ascii() { 0.56 } else { 1.0 })
        .sum::<f32>()
        * size
}

/// Shorten `label` with an ellipsis until it fits `max_w`.
fn fit_label(label: &str, max_w: f32) -> String {
    if estimate_text_w(label, LABEL_FONT) <= max_w {
        return label.to_string();
    }
    let mut out: String = label.to_string();
    while !out.is_empty() && estimate_text_w(&format!("{out}…"), LABEL_FONT) > max_w {
        out.pop();
    }
    format!("{}…", out.trim_end())
}

/// Lay the pills out centred along the bottom of `canvas`, one per
/// `(index, label)`, all shortened to share the canvas width evenly when
/// they would not fit.
pub fn variant_bar_layout(
    canvas: Rect,
    labels: &[(usize, String)],
    button_label: &str,
) -> Vec<VariantBarItem> {
    if labels.is_empty() || canvas.size.x <= 0.0 {
        return Vec::new();
    }
    let button_w = estimate_text_w(button_label, BUTTON_FONT) + BUTTON_PAD_X * 2.0;
    let chrome_w = PILL_PAD_X * 2.0 + 10.0 + button_w;
    let count = labels.len() as f32;
    let available = canvas.size.x - SIDE_MARGIN * 2.0 - PILL_GAP * (count - 1.0);
    let max_label_w = (available / count - chrome_w).max(24.0);
    let fitted: Vec<(usize, String, f32)> = labels
        .iter()
        .map(|(index, label)| {
            let label = fit_label(label, max_label_w);
            let w = estimate_text_w(&label, LABEL_FONT).min(max_label_w) + chrome_w;
            (*index, label, w)
        })
        .collect();
    let total: f32 = fitted.iter().map(|(.., w)| w).sum::<f32>() + PILL_GAP * (count - 1.0);
    let y = canvas.origin.y + canvas.size.y - BOTTOM_INSET - PILL_H;
    let mut x = canvas.origin.x + ((canvas.size.x - total) / 2.0).max(SIDE_MARGIN);
    fitted
        .into_iter()
        .map(|(index, label, w)| {
            let pill = Rect::xywh(x, y, w, PILL_H);
            let button = Rect::xywh(
                pill.origin.x + pill.size.x - PILL_PAD_X / 2.0 - button_w,
                y + (PILL_H - BUTTON_H) / 2.0,
                button_w,
                BUTTON_H,
            );
            x += w + PILL_GAP;
            VariantBarItem {
                index,
                label,
                pill,
                button,
            }
        })
        .collect()
}

impl WorkspaceSurface<'_> {
    /// The variants bar, when the run's directions can be picked from.
    pub fn variant_bar(&self, layout: &WorkspaceLayout) -> Vec<VariantBarItem> {
        if !self.state.variant_pick_enabled() {
            return Vec::new();
        }
        let labels: Vec<(usize, String)> = self
            .state
            .variants
            .iter()
            .map(|variant| (variant.index, variant.label()))
            .collect();
        variant_bar_layout(
            layout.canvas,
            &labels,
            tr(self.ui.locale, "workspace.variants.use"),
        )
    }

    /// The "use this" button under `point`, if any.
    pub(super) fn variant_bar_hit(
        &self,
        layout: &WorkspaceLayout,
        point: Point2D,
    ) -> Option<WorkspaceHit> {
        self.variant_bar(layout)
            .into_iter()
            .find(|item| item.button.contains(point))
            .map(|item| WorkspaceHit::UseVariant(item.index))
    }
}

/// Paint the variants bar over the canvas.
pub(super) fn paint_variant_bar(
    surface: &WorkspaceSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &WorkspaceLayout,
    palette: StudioPalette,
) {
    let items = surface.variant_bar(layout);
    let use_label = tr(surface.ui.locale, "workspace.variants.use");
    for item in &items {
        cx.backend
            .fill_round_rect(item.pill, PILL_H / 2.0, fade(palette.panel, 0.96));
        cx.backend
            .stroke_round_rect(item.pill, PILL_H / 2.0, palette.line, 1.0);
        text(
            cx,
            &item.label,
            Point2D::new(
                item.pill.origin.x + PILL_PAD_X,
                jian_widgets::centered_text_baseline_y(item.pill, LABEL_FONT),
            ),
            LABEL_FONT,
            palette.ink,
        );
        let hovered = surface.state.hover == Some(WorkspaceHit::UseVariant(item.index));
        cx.backend.fill_round_rect(
            item.button,
            BUTTON_H / 2.0,
            if hovered {
                palette.blue_hover
            } else {
                palette.blue
            },
        );
        let label_w = cx.backend.measure_text_family(use_label, BUTTON_FONT, SANS);
        text(
            cx,
            use_label,
            Point2D::new(
                item.button.origin.x + (item.button.size.x - label_w) / 2.0,
                jian_widgets::centered_text_baseline_y(item.button, BUTTON_FONT),
            ),
            BUTTON_FONT,
            palette.panel,
        );
    }
}

#[cfg(test)]
#[path = "workspace_variants_bar_tests.rs"]
mod tests;
