//! AI chat floating-panel geometry.

use super::helpers::{AICHAT_INSET_BOTTOM, AICHAT_INSET_LEFT};
use super::WidgetHostNative;
use op_editor_core::ChatAnchor;
use op_editor_ui::widgets::{AI_CHAT_MINIMIZED_HEIGHT, AI_CHAT_MIN_HEIGHT, AI_CHAT_MIN_WIDTH};
use op_editor_ui::{Point2D, Rect};

impl WidgetHostNative {
    /// The panel's width in BOTH states — minimizing changes the height
    /// only, so the bar and the panel must read this one function or their
    /// edges drift apart as soon as the user resizes.
    pub(in crate::widget_host) fn ai_chat_panel_width(&self) -> f32 {
        self.editor_state.chat.panel_width.max(AI_CHAT_MIN_WIDTH)
    }

    pub(in crate::widget_host) fn ai_chat_size(&self) -> (f32, f32) {
        let height = if self.editor_state.chat.is_minimized() {
            AI_CHAT_MINIMIZED_HEIGHT
        } else {
            self.editor_state.chat.panel_height.max(AI_CHAT_MIN_HEIGHT)
        };
        (self.ai_chat_panel_width(), height)
    }

    pub(in crate::widget_host) fn ai_chat_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<Rect> {
        let (cx0, cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        // Native touch chrome owns a modal AI surface. Phones use a bottom
        // sheet; tablets use a bounded trailing panel so the canvas remains
        // legible and the IME cannot turn the whole screen into a black slab.
        if self.editor_state.editor_ui.touch_chrome() {
            if self.editor_state.editor_ui.mobile_sheet
                != Some(op_editor_core::size_class::MobileSheetKind::Ai)
            {
                return None;
            }
            if self.editor_state.editor_ui.compact_layout() {
                let max_h = (viewport_h
                    - op_editor_ui::widgets::host_canvas_geometry::touch_app_bar_height(
                        &self.editor_state,
                    ))
                .max(0.0);
                let min_h = 280.0_f32.min(max_h);
                let sheet_h = (viewport_h * 0.58).clamp(min_h, max_h);
                return Some(Rect {
                    origin: Point2D::new(0.0, viewport_h - sheet_h),
                    size: Point2D::new(viewport_w, sheet_h),
                });
            }
            let inset = 12.0;
            let top = op_editor_ui::widgets::host_canvas_geometry::touch_app_bar_height(
                &self.editor_state,
            ) + 8.0;
            let panel_w = 380.0_f32.min((viewport_w - inset * 2.0).max(0.0));
            return Some(Rect {
                origin: Point2D::new(viewport_w - panel_w - inset, top),
                size: Point2D::new(panel_w, (viewport_h - top - inset).max(0.0)),
            });
        }
        if self.editor_state.chat.is_minimized() {
            return op_editor_ui::widgets::host_canvas_geometry::minimized_chat_bar_rect(
                self.editor_state.chat.anchor,
                self.ai_chat_panel_width(),
                self.editor_state.chat.panel_position,
                cx0,
                cy0,
                cw,
                ch,
            );
        }
        let (panel_w, panel_h) = self.ai_chat_size();
        if self.editor_state.chat.maximized {
            let inset = 12.0;
            if cw <= inset * 2.0 + 16.0 || ch <= inset * 2.0 + 16.0 {
                return None;
            }
            return Some(Rect {
                origin: Point2D::new(cx0 + inset, cy0 + inset),
                size: Point2D::new(cw - inset * 2.0, ch - inset * 2.0),
            });
        }
        if cw <= panel_w + AICHAT_INSET_LEFT + 16.0 || ch <= panel_h + 16.0 {
            return None;
        }
        if let Some(d) = self.chat_drag {
            return Some(Rect {
                origin: Point2D::new(d.pos_x, d.pos_y),
                size: Point2D::new(panel_w, panel_h),
            });
        }
        if let Some((x, y)) = self.editor_state.chat.panel_position {
            return Some(Rect {
                origin: Point2D::new(x, y),
                size: Point2D::new(panel_w, panel_h),
            });
        }
        let (x, y) = match self.editor_state.chat.anchor {
            ChatAnchor::TopLeft => (cx0 + AICHAT_INSET_LEFT, cy0 + AICHAT_INSET_BOTTOM),
            ChatAnchor::TopRight => (
                cx0 + cw - panel_w - AICHAT_INSET_BOTTOM,
                cy0 + AICHAT_INSET_BOTTOM,
            ),
            ChatAnchor::BottomLeft => (
                cx0 + AICHAT_INSET_LEFT,
                cy0 + ch - panel_h - AICHAT_INSET_BOTTOM,
            ),
            ChatAnchor::BottomRight => (
                cx0 + cw - panel_w - AICHAT_INSET_BOTTOM,
                cy0 + ch - panel_h - AICHAT_INSET_BOTTOM,
            ),
        };
        Some(Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(panel_w, panel_h),
        })
    }
}
