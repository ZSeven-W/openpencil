//! AI chat floating-panel geometry.

use super::helpers::{AICHAT_INSET_BOTTOM, AICHAT_INSET_LEFT};
use super::WidgetHostNative;
use op_editor_core::ChatAnchor;
use op_editor_ui::widgets::{
    AI_CHAT_COLLAPSED_HEIGHT, AI_CHAT_COLLAPSED_WIDTH, AI_CHAT_HEIGHT, AI_CHAT_WIDTH,
};
use op_editor_ui::{Point2D, Rect};

impl WidgetHostNative {
    pub(in crate::widget_host) fn ai_chat_size(&self) -> (f32, f32) {
        if self.editor_state.chat.collapsed {
            (AI_CHAT_COLLAPSED_WIDTH, AI_CHAT_COLLAPSED_HEIGHT)
        } else {
            (AI_CHAT_WIDTH, AI_CHAT_HEIGHT)
        }
    }

    pub(in crate::widget_host) fn ai_chat_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<Rect> {
        let (cx0, cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        let (panel_w, panel_h) = self.ai_chat_size();
        if self.editor_state.chat.maximized && !self.editor_state.chat.collapsed {
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
