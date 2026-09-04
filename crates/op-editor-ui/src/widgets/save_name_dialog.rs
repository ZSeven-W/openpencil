//! Modal file-name prompt for the mobile Save / Save As flow.
//!
//! Touch shells keep documents inside the app sandbox, so saving needs a
//! file name but no directory picker. The card sits near the top of the
//! viewport so the software keyboard (which covers the lower half of a
//! phone screen) never hides the field being edited. Desktop never opens
//! it — its Save flow keeps the native `rfd` picker.

use crate::widgets::text_metrics;
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use op_editor_core::editor_ui_state::EditorUiState;

pub const DIALOG_WIDTH: f32 = 360.0;
pub const DIALOG_HEIGHT: f32 = 168.0;
const PAD: f32 = 20.0;
const HEADER_HEIGHT: f32 = 48.0;
const INPUT_HEIGHT: f32 = 40.0;
const BUTTON_HEIGHT: f32 = 36.0;
const BUTTON_WIDTH: f32 = 96.0;
const CORNER: f32 = 14.0;
const TOP_MARGIN: f32 = 24.0;
const FONT_FAMILY: &str = "system-ui";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveNameDialogHit {
    /// The file-name field — focuses it (it owns the IME already).
    Input,
    /// The primary Save button.
    Confirm,
    /// The Cancel button.
    Cancel,
}

pub struct SaveNameDialog {
    rect: Rect,
}

impl SaveNameDialog {
    /// Anchored top-center: below the touch app bar, clear of the software
    /// keyboard. `app_bar_bottom` is the y where chrome content may start.
    pub fn anchored(viewport_w: f32, app_bar_bottom: f32) -> Self {
        let width = DIALOG_WIDTH.min((viewport_w - 24.0).max(0.0));
        let x = ((viewport_w - width) / 2.0).max(12.0);
        Self {
            rect: Rect::xywh(x, app_bar_bottom + TOP_MARGIN, width, DIALOG_HEIGHT),
        }
    }

    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn contains(&self, point: Point2D) -> bool {
        self.rect.contains(point)
    }

    pub fn input_rect(&self) -> Rect {
        Rect::xywh(
            self.rect.origin.x + PAD,
            self.rect.origin.y + HEADER_HEIGHT,
            self.rect.size.x - PAD * 2.0,
            INPUT_HEIGHT,
        )
    }

    fn button_rects(&self) -> (Rect, Rect) {
        let y = self.rect.origin.y + self.rect.size.y - BUTTON_HEIGHT - 16.0;
        let confirm_x = self.rect.origin.x + self.rect.size.x - PAD - BUTTON_WIDTH;
        let cancel_x = confirm_x - 8.0 - BUTTON_WIDTH;
        (
            Rect::xywh(cancel_x, y, BUTTON_WIDTH, BUTTON_HEIGHT),
            Rect::xywh(confirm_x, y, BUTTON_WIDTH, BUTTON_HEIGHT),
        )
    }

    pub fn hit_test(&self, point: Point2D) -> Option<SaveNameDialogHit> {
        if !self.contains(point) {
            return None;
        }
        if self.input_rect().contains(point) {
            return Some(SaveNameDialogHit::Input);
        }
        let (cancel, confirm) = self.button_rects();
        if cancel.contains(point) {
            return Some(SaveNameDialogHit::Cancel);
        }
        if confirm.contains(point) {
            return Some(SaveNameDialogHit::Confirm);
        }
        None
    }

    pub fn paint(&self, cx: &mut crate::widgets::PaintCx<'_>, ui: &EditorUiState, now_ms: u64) {
        let theme = crate::widgets::editor_state_ext::theme_for(ui);
        let tr = |key: &'static str| op_i18n::translate(ui.effective_locale(), key);
        let dialog = &ui.save_name_dialog;

        cx.backend.fill_round_rect(self.rect, CORNER, theme.popover);
        cx.backend
            .stroke_round_rect(self.rect, CORNER, theme.border, 1.0);

        // Title.
        let title = tr("dialog.pickerSaveTitle");
        let title_layout = TextLayout::single_run(
            title,
            FONT_FAMILY,
            15.0,
            theme.foreground.to_jian(),
            Point2D::ZERO,
        );
        cx.backend.draw_text(
            &title_layout,
            Point2D::new(self.rect.origin.x + PAD, self.rect.origin.y + 30.0),
        );

        // File-name field — always focused while the modal is open.
        let input = self.input_rect();
        cx.backend.fill_round_rect(input, 8.0, theme.input);
        cx.backend.stroke_round_rect(input, 8.0, theme.ring, 1.0);
        crate::widgets::property_panel_text_input::paint_text_input_view(
            cx,
            &theme,
            &dialog.input,
            input,
            14.0,
            12.0,
            jian_widgets::centered_text_baseline_y(input, 14.0),
            now_ms,
            tr("common.untitled"),
            true,
        );

        // Footer: Cancel (outline) + Save (primary; muted when blank).
        let (cancel, confirm) = self.button_rects();
        cx.backend.fill_round_rect(cancel, 8.0, theme.card);
        cx.backend.stroke_round_rect(cancel, 8.0, theme.border, 1.0);
        paint_centered_label(
            cx.backend,
            tr("common.cancel"),
            13.0,
            theme.foreground,
            cancel,
        );
        let enabled = dialog.confirm_enabled();
        let (confirm_bg, confirm_fg) = if enabled {
            (theme.primary, theme.primary_foreground)
        } else {
            (theme.muted, theme.muted_foreground)
        };
        cx.backend.fill_round_rect(confirm, 8.0, confirm_bg);
        cx.backend
            .stroke_round_rect(confirm, 8.0, theme.border, 1.0);
        paint_centered_label(cx.backend, tr("common.save"), 13.0, confirm_fg, confirm);
    }
}

fn paint_centered_label(
    backend: &mut dyn RenderBackend,
    text: &str,
    size: f32,
    color: Color,
    rect: Rect,
) {
    let w = text_metrics::measure_chrome(backend, text, size);
    let layout = TextLayout::single_run(text, FONT_FAMILY, size, color.to_jian(), Point2D::ZERO);
    backend.draw_text(
        &layout,
        Point2D::new(
            rect.origin.x + (rect.size.x - w) / 2.0,
            jian_widgets::centered_text_baseline_y(rect, size),
        ),
    );
}

/// Scrim behind the modal — matches the mobile sheet scrim.
pub fn save_name_scrim_color() -> Color {
    Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.42,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_targets_stay_inside_the_card_and_apart() {
        let dialog = SaveNameDialog::anchored(390.0, 52.0);
        let rect = dialog.rect();
        assert!(rect.origin.y >= 52.0);
        assert!(rect.origin.x + rect.size.x <= 390.0);

        let input = dialog.input_rect();
        let center = Point2D::new(
            input.origin.x + input.size.x / 2.0,
            input.origin.y + input.size.y / 2.0,
        );
        assert_eq!(dialog.hit_test(center), Some(SaveNameDialogHit::Input));

        let (cancel, confirm) = dialog.button_rects();
        for (target, expected) in [
            (cancel, SaveNameDialogHit::Cancel),
            (confirm, SaveNameDialogHit::Confirm),
        ] {
            assert!(target.size.y >= 36.0);
            let center = Point2D::new(
                target.origin.x + target.size.x / 2.0,
                target.origin.y + target.size.y / 2.0,
            );
            assert_eq!(dialog.hit_test(center), Some(expected));
        }
        assert!(dialog
            .hit_test(Point2D::new(rect.origin.x + 1.0, rect.origin.y + 1.0))
            .is_none());
        assert!(dialog.hit_test(Point2D::new(-1.0, -1.0)).is_none());
    }

    #[test]
    fn narrow_phones_keep_a_margin() {
        let dialog = SaveNameDialog::anchored(320.0, 52.0);
        assert!(dialog.rect().size.x <= 320.0 - 24.0);
        assert!(dialog.rect().origin.x >= 12.0);
    }
}
