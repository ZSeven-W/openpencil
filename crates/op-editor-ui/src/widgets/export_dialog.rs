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
use crate::widgets::editor_state_ext::{doc_export_format, export_format};
use crate::widgets::text_metrics;
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use op_editor_core::editor_ui_state::EditorUiState;

pub const DIALOG_WIDTH: f32 = 392.0;
pub const DIALOG_HEIGHT: f32 = 236.0;
const PAD: f32 = 20.0;
const ROW_GAP: f32 = 20.0;
const PILL_HEIGHT: f32 = 32.0;
const PILL_GAP: f32 = 8.0;
const PILL_RADIUS: f32 = 8.0;
const SCALE_PILL_WIDTH: f32 = 64.0;
const BUTTON_HEIGHT: f32 = 34.0;
const BUTTON_WIDTH: f32 = 88.0;
const HEADER_HEIGHT: f32 = 52.0;
const CORNER: f32 = 12.0;
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
    /// Whether the current target's shipping renderer contains this encoder.
    pub fn is_implemented(self) -> bool {
        export_format(self).is_implemented()
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
        let tr = |key: &'static str| op_i18n::translate(ui.effective_locale(), key);
        backend.fill_round_rect(self.rect, CORNER, theme.popover);
        backend.stroke_round_rect(self.rect, CORNER, theme.border, 1.0);

        // Header label.
        let header = TextLayout::single_run(
            tr("export.title"),
            FONT_FAMILY,
            15.0,
            (theme.foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        backend.draw_text(
            &header,
            Point2D::new(self.rect.origin.x + PAD, self.rect.origin.y + 28.0),
        );

        let body_y = self.rect.origin.y + HEADER_HEIGHT;

        // Format row label.
        let fmt_label = TextLayout::single_run(
            tr("export.format"),
            FONT_FAMILY,
            12.0,
            (theme.muted_foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        backend.draw_text(&fmt_label, Point2D::new(self.rect.origin.x + PAD, body_y));
        for (i, &fmt) in ExportFormat::ALL.iter().enumerate() {
            let rect = self.format_pill_rect(i);
            let selected = fmt == doc_export_format(ui.export_format);
            if !fmt.is_implemented() {
                self.paint_disabled_pill(backend, theme, rect, fmt.label());
            } else {
                let button = op_editor_core::ExportDialogButton::Format(
                    crate::widgets::editor_state_ext::export_format(fmt),
                );
                let hovered = ui.export_dialog_hover == Some(button);
                let pressed = export_dialog_pressed(ui, button);
                self.paint_pill(
                    backend,
                    theme,
                    rect,
                    fmt.label(),
                    selected,
                    hovered,
                    pressed,
                );
            }
        }

        // Scale row label + pills.
        let scale_label_y = body_y + 16.0 + PILL_HEIGHT + ROW_GAP;
        let scale_label = TextLayout::single_run(
            tr("export.scale"),
            FONT_FAMILY,
            12.0,
            (theme.muted_foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        backend.draw_text(
            &scale_label,
            Point2D::new(self.rect.origin.x + PAD, scale_label_y),
        );
        for (i, lbl) in ["1x", "2x", "3x"].iter().enumerate() {
            let s = (i as u8) + 1;
            let selected = scale_index(ui.export_scale) == s;
            let button = op_editor_core::ExportDialogButton::Scale(s);
            let hovered = ui.export_dialog_hover == Some(button);
            let pressed = export_dialog_pressed(ui, button);
            self.paint_pill(
                backend,
                theme,
                self.scale_pill_rect(i),
                lbl,
                selected,
                hovered,
                pressed,
            );
        }

        // Footer buttons — Cancel as an outline (border + hover wash), Export
        // as the primary CTA. jian Button owns the fill / border / feedback.
        let (cancel, export) = self.button_rects();
        let tokens = crate::widgets::button::tokens_from_theme(theme);
        use jian_widgets::components::button::{Button, ButtonVariant};
        use op_editor_core::ExportDialogButton;
        Button {
            label: tr("common.cancel"),
            icon_paths: None,
            variant: ButtonVariant::Outline,
            enabled: true,
            hovered: ui.export_dialog_hover == Some(ExportDialogButton::Cancel),
            pressed: export_dialog_pressed(ui, ExportDialogButton::Cancel),
            font_size: 13.0,
        }
        .paint(backend, cancel, &tokens);
        Button {
            label: tr("common.export"),
            icon_paths: None,
            variant: ButtonVariant::Primary,
            enabled: true,
            hovered: ui.export_dialog_hover == Some(ExportDialogButton::Export),
            pressed: export_dialog_pressed(ui, ExportDialogButton::Export),
            font_size: 13.0,
        }
        .paint(backend, export, &tokens);
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

    #[allow(clippy::too_many_arguments)]
    fn paint_pill(
        &self,
        backend: &mut dyn RenderBackend,
        theme: &Theme,
        rect: Rect,
        label: &str,
        selected: bool,
        hovered: bool,
        pressed: bool,
    ) {
        let bg = if selected { theme.primary } else { theme.muted };
        backend.fill_round_rect(rect, PILL_RADIUS, bg);
        crate::widgets::button::paint_button_feedback_wash(
            backend,
            theme,
            rect,
            PILL_RADIUS,
            hovered,
            pressed,
        );
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
    /// of the right + bottom edges).
    pub fn contains(&self, point: Point2D) -> bool {
        self.rect.contains(point)
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
        let x = self.rect.origin.x + PAD + (SCALE_PILL_WIDTH + PILL_GAP) * i as f32;
        Rect::xywh(x, scale_pills_y, SCALE_PILL_WIDTH, PILL_HEIGHT)
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

fn export_dialog_pressed(ui: &EditorUiState, button: op_editor_core::ExportDialogButton) -> bool {
    ui.pressed_button == Some(op_editor_core::ButtonPressTarget::ExportDialog(button))
}

fn paint_centered_label(
    backend: &mut dyn RenderBackend,
    text: &str,
    size: f32,
    color: Color,
    rect: Rect,
) {
    let w = text_metrics::measure_chrome(backend, text, size);
    let layout = TextLayout::single_run(
        text,
        FONT_FAMILY,
        size,
        (color).to_jian(),
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

    #[derive(Default)]
    struct CaptureBackend {
        round_fills: Vec<(Rect, f32, Color)>,
        texts: Vec<(String, Point2D)>,
    }

    impl RenderBackend for CaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, layout: &TextLayout, point: Point2D) {
            if let Some(run) = layout.runs().first() {
                self.texts.push((
                    run.content.clone(),
                    Point2D::new(point.x + run.origin.x, point.y + run.origin.y),
                ));
            }
        }
        fn clip_rect(&mut self, _: Rect) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
            self.round_fills.push((rect, radius, color));
        }
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    fn color_close(a: Color, b: Color) -> bool {
        (a.r - b.r).abs() < 1e-6
            && (a.g - b.g).abs() < 1e-6
            && (a.b - b.b).abs() < 1e-6
            && (a.a - b.a).abs() < 1e-6
    }

    #[test]
    fn export_dialog_layout_keeps_header_and_footer_tight() {
        let dlg = ExportDialog::centered(1000.0, 800.0);
        let rect = dlg.rect();
        let scale = dlg.scale_pill_rect(0);
        let (cancel, export) = dlg.button_rects();
        let footer_gap = cancel.origin.y - (scale.origin.y + scale.size.y);
        let mut backend = CaptureBackend::default();

        // Pin English so the "Export" title assertion below is
        // locale-independent (the default EditorUiState locale is ZhCn).
        let ui = EditorUiState {
            locale: op_editor_core::Locale::EnUs,
            ..Default::default()
        };
        dlg.paint(&mut backend, &Theme::dark(), &ui);

        let title = backend
            .texts
            .iter()
            .find(|(text, point)| text == "Export" && point.x < export.origin.x)
            .expect("dialog should paint a title");

        assert!(rect.size.x <= 400.0, "dialog should not feel oversized");
        assert!(
            (232.0..=244.0).contains(&rect.size.y),
            "dialog should have enough vertical padding without becoming tall"
        );
        assert!(
            title.1.y - rect.origin.y >= 24.0,
            "title should have clear top padding"
        );
        assert!(
            (10.0..=20.0).contains(&footer_gap),
            "scale controls and footer buttons should read as one compact dialog"
        );
        assert_eq!(cancel.size.y, export.size.y);
    }

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
    fn every_host_format_matches_the_core_capability() {
        for fmt in ExportFormat::ALL {
            assert_eq!(
                fmt.is_implemented(),
                export_format(fmt).is_implemented(),
                "{fmt:?} capability drifted"
            );
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
    fn pressed_scale_pill_uses_shared_button_feedback() {
        let dlg = ExportDialog::centered(1000.0, 800.0);
        let ui = EditorUiState {
            pressed_button: Some(op_editor_core::ButtonPressTarget::ExportDialog(
                op_editor_core::ExportDialogButton::Scale(1),
            )),
            ..Default::default()
        };
        let theme = Theme::dark();
        let expected = theme.button_hover.with_alpha(theme.button_hover.a * 1.8);
        let mut backend = CaptureBackend::default();
        let scale = dlg.scale_pill_rect(0);

        dlg.paint(&mut backend, &theme, &ui);

        assert!(
            backend.round_fills.iter().any(|(rect, radius, color)| {
                *rect == scale
                    && (*radius - PILL_RADIUS).abs() < 0.01
                    && color_close(*color, expected)
            }),
            "pressed scale pill should paint the shared pressed feedback token"
        );
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
