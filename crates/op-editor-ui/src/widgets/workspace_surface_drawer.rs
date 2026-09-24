//! The narrow-window chat drawer's own chrome: the scrim over the canvas
//! and the drawer's sheet (surface, edge, shadow).
//!
//! The chat panel itself is painted by the host at the sheet's slid
//! position (the ordinary chat pass, translated); this module paints
//! what sits under it. Hit-testing always uses the SETTLED rect — only
//! the paint slides.

use super::paint::fade;
use super::{StudioPalette, WorkspaceSurface};
use crate::widgets::PaintCx;
use crate::{Color, Rect};
use op_editor_core::WORKSPACE_HEADER_H;

/// Peak scrim opacity over the canvas while the drawer is open.
const SCRIM_ALPHA: f32 = 0.32;

impl WorkspaceSurface<'_> {
    /// The drawer's slide progress (0 shut … 1 open) at this frame,
    /// honouring reduced motion.
    pub fn drawer_progress(&self) -> f32 {
        self.ui.workspace_drawer_progress(self.now_ms)
    }

    /// Horizontal offset the drawer's content paints at this frame:
    /// `-width` fully shut, `0` fully open.
    pub fn drawer_offset_x(&self, viewport_w: f32) -> f32 {
        -(1.0 - self.drawer_progress()) * self.ui.workspace_drawer_width(viewport_w)
    }

    /// Whether the drawer (or its closing slide) paints at all.
    pub fn drawer_painting(&self) -> bool {
        self.ui.workspace_drawer_active() && self.drawer_progress() > 0.0
    }

    /// Paint the scrim and the drawer's sheet at the current slide
    /// position. The host paints the chat panel over the sheet after this.
    pub fn paint_drawer_sheet(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        if !self.drawer_painting() {
            return;
        }
        let progress = self.drawer_progress();
        let palette = StudioPalette::for_mode(self.ui.effective_theme_mode());
        let below_header = Rect::xywh(
            0.0,
            WORKSPACE_HEADER_H,
            rect.size.x,
            (rect.size.y - WORKSPACE_HEADER_H).max(0.0),
        );
        cx.backend.fill_rect(
            below_header,
            fade(Color::rgb_u8(0x0B, 0x10, 0x18), SCRIM_ALPHA * progress),
        );
        let width = self.ui.workspace_drawer_width(rect.size.x);
        let sheet = Rect::xywh(
            self.drawer_offset_x(rect.size.x),
            WORKSPACE_HEADER_H,
            width,
            below_header.size.y,
        );
        cx.backend.fill_drop_shadow(
            Rect::xywh(
                sheet.origin.x + 4.0,
                sheet.origin.y,
                sheet.size.x,
                sheet.size.y,
            ),
            0.0,
            16.0,
            fade(Color::rgb_u8(0x0B, 0x10, 0x18), 0.22 * progress),
        );
        cx.backend.fill_rect(sheet, palette.panel);
        cx.backend.fill_rect(
            Rect::xywh(
                sheet.origin.x + sheet.size.x - 1.0,
                sheet.origin.y,
                1.0,
                sheet.size.y,
            ),
            palette.line,
        );
    }
}
