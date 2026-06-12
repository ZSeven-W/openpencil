//! Floating export dialog — opens on File → Export image (or
//! Cmd/Ctrl+Shift+P). Mirrors the TS Electron `ExportDialog`
//! (`apps/web/src/components/shared/export-dialog.tsx`):
//!   - Format radio group: PNG / JPEG / WEBP / SVG / PDF
//!   - Scale radio group: 1× / 2× / 3×
//!   - Cancel / Export buttons
//!
//! Selection writes back to `Document.ui.export_format` /
//! `export_scale`; Export dispatches `FileAction::ExportImage` so
//! the host's save dialog uses the chosen format + scale.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::doc_export_format;
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use op_editor_core::editor_ui_state::EditorUiState;

pub const DIALOG_WIDTH: f32 = 420.0;
pub const DIALOG_HEIGHT: f32 = 220.0;
const PAD: f32 = 16.0;
const ROW_GAP: f32 = 14.0;
const PILL_HEIGHT: f32 = 28.0;
const PILL_GAP: f32 = 6.0;
const PILL_RADIUS: f32 = 6.0;
const BUTTON_HEIGHT: f32 = 30.0;
const BUTTON_WIDTH: f32 = 80.0;
const HEADER_HEIGHT: f32 = 40.0;
const CORNER: f32 = 10.0;
const FONT_FAMILY: &str = "system-ui";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Png,
    Jpeg,
    Webp,
    Svg,
    Pdf,
}

impl ExportFormat {
    pub fn label(self) -> &'static str {
        match self {
            ExportFormat::Png => "PNG",
            ExportFormat::Jpeg => "JPEG",
            ExportFormat::Webp => "WEBP",
            ExportFormat::Svg => "SVG",
            ExportFormat::Pdf => "PDF",
        }
    }
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Png => "png",
            ExportFormat::Jpeg => "jpg",
            ExportFormat::Webp => "webp",
            ExportFormat::Svg => "svg",
            ExportFormat::Pdf => "pdf",
        }
    }
    pub const ALL: [ExportFormat; 5] = [
        ExportFormat::Png,
        ExportFormat::Jpeg,
        ExportFormat::Webp,
        ExportFormat::Svg,
        ExportFormat::Pdf,
    ];
    /// Whether the format has a working export backend. All five
    /// formats are now implemented (PDF landed via skia's built-in
    /// `pdf::new_document` backend in Phase 4); kept as a hook so a
    /// future format can be added in a disabled state.
    pub fn is_implemented(self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportDialogHit {
    Format(ExportFormat),
    Scale(u8),
    Cancel,
    Export,
}

pub struct ExportDialog {
    rect: Rect,
}

impl ExportDialog {
    pub fn centered(viewport_w: f32, viewport_h: f32) -> Self {
        let x = ((viewport_w - DIALOG_WIDTH) / 2.0).max(0.0);
        let y = ((viewport_h - DIALOG_HEIGHT) / 2.0).max(0.0);
        Self {
            rect: Rect::xywh(x, y, DIALOG_WIDTH, DIALOG_HEIGHT),
        }
    }

    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn paint(&self, backend: &mut dyn RenderBackend, theme: &Theme, ui: &EditorUiState) {
        backend.fill_round_rect(self.rect, CORNER, theme.popover);
        backend.stroke_round_rect(self.rect, CORNER, theme.border, 1.0);

        // Header "Export" label.
        let header = TextLayout::single_run(
            "Export",
            FONT_FAMILY,
            15.0,
            to_jian(theme.foreground),
            Point2D::new(0.0, 0.0),
        );
        backend.draw_text(
            &header,
            Point2D::new(self.rect.origin.x + PAD, self.rect.origin.y + 14.0),
        );

        let body_y = self.rect.origin.y + HEADER_HEIGHT;

        // Format row label.
        let fmt_label = TextLayout::single_run(
            "Format",
            FONT_FAMILY,
            12.0,
            to_jian(theme.muted_foreground),
            Point2D::new(0.0, 0.0),
        );
        backend.draw_text(&fmt_label, Point2D::new(self.rect.origin.x + PAD, body_y));
        for (i, &fmt) in ExportFormat::ALL.iter().enumerate() {
            let rect = self.format_pill_rect(i);
            let selected = fmt == doc_export_format(ui.export_format);
            if !fmt.is_implemented() {
                self.paint_disabled_pill(backend, theme, rect, fmt.label());
            } else {
                let hovered = ui.export_dialog_hover
                    == Some(op_editor_core::ExportDialogButton::Format(
                        crate::widgets::editor_state_ext::export_format(fmt),
                    ));
                self.paint_pill(backend, theme, rect, fmt.label(), selected, hovered);
            }
        }

        // Scale row label + pills.
        let scale_label_y = body_y + 16.0 + PILL_HEIGHT + ROW_GAP;
        let scale_label = TextLayout::single_run(
            "Scale",
            FONT_FAMILY,
            12.0,
            to_jian(theme.muted_foreground),
            Point2D::new(0.0, 0.0),
        );
        backend.draw_text(
            &scale_label,
            Point2D::new(self.rect.origin.x + PAD, scale_label_y),
        );
        for (i, lbl) in ["1x", "2x", "3x"].iter().enumerate() {
            let s = (i as u8) + 1;
            let selected = scale_index(ui.export_scale) == s;
            let hovered =
                ui.export_dialog_hover == Some(op_editor_core::ExportDialogButton::Scale(s));
            self.paint_pill(
                backend,
                theme,
                self.scale_pill_rect(i),
                lbl,
                selected,
                hovered,
            );
        }

        // Footer buttons.
        let (cancel, export) = self.button_rects();
        backend.fill_round_rect(cancel, 6.0, theme.muted);
        if ui.export_dialog_hover == Some(op_editor_core::ExportDialogButton::Cancel) {
            backend.fill_round_rect(cancel, 6.0, theme.button_hover);
        }
        backend.stroke_round_rect(cancel, 6.0, theme.border, 1.0);
        paint_centered_label(backend, "Cancel", 13.0, theme.foreground, cancel);
        backend.fill_round_rect(export, 6.0, theme.primary);
        if ui.export_dialog_hover == Some(op_editor_core::ExportDialogButton::Export) {
            backend.fill_round_rect(export, 6.0, theme.button_hover);
        }
        paint_centered_label(backend, "Export", 13.0, theme.primary_foreground, export);
    }

    fn paint_disabled_pill(
        &self,
        backend: &mut dyn RenderBackend,
        theme: &Theme,
        rect: Rect,
        label: &str,
    ) {
        backend.fill_round_rect(rect, PILL_RADIUS, theme.muted);
        backend.stroke_round_rect(rect, PILL_RADIUS, theme.border, 1.0);
        // Muted-foreground signals "not pickable" without a hard
        // disabled overlay. Honest UI: still shows what's coming.
        paint_centered_label(backend, label, 12.0, theme.muted_foreground, rect);
    }

    fn paint_pill(
        &self,
        backend: &mut dyn RenderBackend,
        theme: &Theme,
        rect: Rect,
        label: &str,
        selected: bool,
        hovered: bool,
    ) {
        let bg = if selected { theme.primary } else { theme.muted };
        backend.fill_round_rect(rect, PILL_RADIUS, bg);
        // Hover brightens the pill (TS `hover:bg-accent` / `hover:bg-primary/90`).
        if hovered {
            backend.fill_round_rect(rect, PILL_RADIUS, theme.button_hover);
        }
        backend.stroke_round_rect(rect, PILL_RADIUS, theme.border, 1.0);
        let color = if selected {
            theme.primary_foreground
        } else {
            theme.foreground
        };
        paint_centered_label(backend, label, 12.0, color, rect);
    }

    pub fn hit_test(&self, point: Point2D) -> Option<ExportDialogHit> {
        if !(self.rect).contains(point) {
            return None;
        }
        for (i, &fmt) in ExportFormat::ALL.iter().enumerate() {
            if (self.format_pill_rect(i)).contains(point) {
                // Disabled formats swallow the click without changing
                // the active format. Same as TS — visible but inert.
                if !fmt.is_implemented() {
                    return None;
                }
                return Some(ExportDialogHit::Format(fmt));
            }
        }
        for i in 0..3 {
            if (self.scale_pill_rect(i)).contains(point) {
                return Some(ExportDialogHit::Scale((i as u8) + 1));
            }
        }
        let (cancel, export) = self.button_rects();
        if (cancel).contains(point) {
            return Some(ExportDialogHit::Cancel);
        }
        if (export).contains(point) {
            return Some(ExportDialogHit::Export);
        }
        None
    }

    /// True iff the point lands inside the dialog rect (inclusive
    /// of the right + bottom edges, unlike `rect_contains` which
    /// uses half-open bounds — codex NIT: edge-pixel clicks must
    /// be treated as in-dialog dead space, not outside-click).
    pub fn contains(&self, point: Point2D) -> bool {
        let r = self.rect;
        point.x >= r.origin.x
            && point.x <= r.origin.x + r.size.x
            && point.y >= r.origin.y
            && point.y <= r.origin.y + r.size.y
    }

    fn format_pill_rect(&self, i: usize) -> Rect {
        let body_y = self.rect.origin.y + HEADER_HEIGHT;
        let pills_y = body_y + 16.0;
        let total_w = self.rect.size.x - PAD * 2.0;
        let pill_w = (total_w - PILL_GAP * 4.0) / 5.0;
        let x = self.rect.origin.x + PAD + (pill_w + PILL_GAP) * i as f32;
        Rect::xywh(x, pills_y, pill_w, PILL_HEIGHT)
    }

    fn scale_pill_rect(&self, i: usize) -> Rect {
        let body_y = self.rect.origin.y + HEADER_HEIGHT;
        let scale_pills_y = body_y + 16.0 + PILL_HEIGHT + ROW_GAP + 16.0;
        let pill_w = 56.0;
        let x = self.rect.origin.x + PAD + (pill_w + PILL_GAP) * i as f32;
        Rect::xywh(x, scale_pills_y, pill_w, PILL_HEIGHT)
    }

    fn button_rects(&self) -> (Rect, Rect) {
        let y = self.rect.origin.y + self.rect.size.y - BUTTON_HEIGHT - PAD;
        let export_x = self.rect.origin.x + self.rect.size.x - PAD - BUTTON_WIDTH;
        let cancel_x = export_x - 8.0 - BUTTON_WIDTH;
        (
            Rect::xywh(cancel_x, y, BUTTON_WIDTH, BUTTON_HEIGHT),
            Rect::xywh(export_x, y, BUTTON_WIDTH, BUTTON_HEIGHT),
        )
    }
}

fn paint_centered_label(
    backend: &mut dyn RenderBackend,
    text: &str,
    size: f32,
    color: Color,
    rect: Rect,
) {
    let w = backend.measure_text(text, size);
    let layout = TextLayout::single_run(
        text,
        FONT_FAMILY,
        size,
        to_jian(color),
        Point2D::new(0.0, 0.0),
    );
    let y_offset = (rect.size.y - size) / 2.0 + size * 0.75;
    backend.draw_text(
        &layout,
        Point2D::new(
            rect.origin.x + (rect.size.x - w) / 2.0,
            rect.origin.y + y_offset,
        ),
    );
}

fn to_jian(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}

/// Map a stored `export_scale` (1.0 / 2.0 / 3.0) to a UI index
/// (1 / 2 / 3). Values outside the supported set round to 2 (default).
pub fn scale_index(scale: f32) -> u8 {
    match scale {
        s if (s - 1.0).abs() < 0.01 => 1,
        s if (s - 3.0).abs() < 0.01 => 3,
        _ => 2,
    }
}

pub fn scale_from_index(index: u8) -> f32 {
    match index {
        1 => 1.0,
        3 => 3.0,
        _ => 2.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_centers_in_viewport() {
        let dlg = ExportDialog::centered(1000.0, 800.0);
        let r = dlg.rect();
        assert!((r.origin.x - (500.0 - DIALOG_WIDTH / 2.0)).abs() < 0.5);
        assert!((r.origin.y - (400.0 - DIALOG_HEIGHT / 2.0)).abs() < 0.5);
        assert_eq!(r.size.x, DIALOG_WIDTH);
        assert_eq!(r.size.y, DIALOG_HEIGHT);
    }

    #[test]
    fn hit_test_resolves_each_implemented_format_pill() {
        let dlg = ExportDialog::centered(1000.0, 800.0);
        for (i, &fmt) in ExportFormat::ALL.iter().enumerate() {
            let r = dlg.format_pill_rect(i);
            let center = Point2D::new(r.origin.x + r.size.x / 2.0, r.origin.y + r.size.y / 2.0);
            if fmt.is_implemented() {
                assert_eq!(dlg.hit_test(center), Some(ExportDialogHit::Format(fmt)));
            } else {
                // Disabled formats swallow the click; no action queued.
                assert_eq!(dlg.hit_test(center), None);
            }
        }
    }

    #[test]
    fn every_format_is_implemented() {
        // Phase 4 enabled PDF via skia's built-in backend. All five
        // formats are now selectable + dispatched to a real encoder.
        for fmt in ExportFormat::ALL {
            assert!(fmt.is_implemented(), "{fmt:?} should be implemented");
        }
    }

    #[test]
    fn hit_test_resolves_each_scale_pill() {
        let dlg = ExportDialog::centered(1000.0, 800.0);
        for i in 0..3 {
            let r = dlg.scale_pill_rect(i);
            let center = Point2D::new(r.origin.x + r.size.x / 2.0, r.origin.y + r.size.y / 2.0);
            assert_eq!(
                dlg.hit_test(center),
                Some(ExportDialogHit::Scale((i as u8) + 1))
            );
        }
    }

    #[test]
    fn hit_test_resolves_buttons() {
        let dlg = ExportDialog::centered(1000.0, 800.0);
        let (c, e) = dlg.button_rects();
        let cc = Point2D::new(c.origin.x + c.size.x / 2.0, c.origin.y + c.size.y / 2.0);
        let ec = Point2D::new(e.origin.x + e.size.x / 2.0, e.origin.y + e.size.y / 2.0);
        assert_eq!(dlg.hit_test(cc), Some(ExportDialogHit::Cancel));
        assert_eq!(dlg.hit_test(ec), Some(ExportDialogHit::Export));
    }

    #[test]
    fn hit_test_outside_dialog_returns_none() {
        let dlg = ExportDialog::centered(1000.0, 800.0);
        assert_eq!(dlg.hit_test(Point2D::new(0.0, 0.0)), None);
        assert_eq!(dlg.hit_test(Point2D::new(999.0, 799.0)), None);
    }

    #[test]
    fn scale_index_round_trip() {
        assert_eq!(scale_index(1.0), 1);
        assert_eq!(scale_index(2.0), 2);
        assert_eq!(scale_index(3.0), 3);
        assert_eq!(scale_index(0.5), 2);
        assert_eq!(scale_index(4.0), 2);
        assert_eq!(scale_from_index(1), 1.0);
        assert_eq!(scale_from_index(2), 2.0);
        assert_eq!(scale_from_index(3), 3.0);
        assert_eq!(scale_from_index(99), 2.0);
    }

    #[test]
    fn format_labels_match_ts_strings() {
        assert_eq!(ExportFormat::Png.label(), "PNG");
        assert_eq!(ExportFormat::Jpeg.label(), "JPEG");
        assert_eq!(ExportFormat::Webp.label(), "WEBP");
        assert_eq!(ExportFormat::Svg.label(), "SVG");
        assert_eq!(ExportFormat::Pdf.label(), "PDF");
        assert_eq!(ExportFormat::Jpeg.extension(), "jpg");
    }
}
