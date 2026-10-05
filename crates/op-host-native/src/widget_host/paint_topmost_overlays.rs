//! The native host's top-most overlay band: the post-import diagnostics
//! notice, the transient toast banner, and the missing-font modal.
//!
//! Split off `paint.rs` at the 800-line cap; pure code motion, so the z-order
//! here IS the order the press ladder mirrors in reverse (see
//! `press_overlay_tiers.rs`).

use crate::backend::NativeFrameBackend;
use op_editor_ui::widgets::{PaintCx, Widget};
use op_editor_ui::{Point2D, Rect, RenderBackend};

use super::WidgetHostNative;

impl WidgetHostNative {
    pub(in crate::widget_host) fn paint_topmost_overlays(
        &mut self,
        frame: &mut NativeFrameBackend<'_>,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        // Post-import HTML diagnostics notice — painted above every panel
        // but under the missing-font modal, mirroring its press tier.
        if let Some(panel) =
            op_editor_ui::widgets::HtmlImportDiagnosticsPanel::for_editor(&self.editor_state)
        {
            let panel_rect = panel.rect(viewport_width, viewport_height);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            panel.paint(&mut cx, panel_rect);
        }

        // The normal workspace keeps font fallback visible without blocking reading.
        {
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            op_editor_ui::widgets::missing_fonts_notice::paint(
                &mut cx,
                &self.editor_state,
                viewport_width,
                viewport_height,
            );
        }

        // Transient notice banner — painted above every panel and the
        // diagnostics notice, but under the missing-font modal, so its press
        // tier sits in exactly the same place (hit-test is reverse paint
        // order). The rect is cached because the width is measured in the font
        // it paints with, which only this pass can do.
        self.toast_rect = {
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            op_editor_ui::widgets::editor_toast_flow::paint(
                &mut cx,
                &self.editor_state,
                viewport_width,
                viewport_height,
                self.now_ms,
            )
        };

        // Missing-font prompt — absolute top-most modal after every other
        // overlay, matching its first-tier press routing.
        if let Some(panel) =
            op_editor_ui::widgets::MissingFontsPanel::for_editor(&self.editor_state)
        {
            frame.fill_rect(
                Rect {
                    origin: Point2D::new(0.0, 0.0),
                    size: Point2D::new(viewport_width, viewport_height),
                },
                op_editor_ui::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.5,
                },
            );
            let panel_rect = panel.rect(viewport_width, viewport_height);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            panel.paint(&mut cx, panel_rect);
            panel.paint_picker(
                &mut cx,
                panel_rect,
                Rect {
                    origin: Point2D::new(0.0, 0.0),
                    size: Point2D::new(viewport_width, viewport_height),
                },
                self.now_ms,
            );
        }
    }
}
