//! AI chat floating-panel geometry for the web host.

use super::{WidgetHost, AICHAT_INSET_BOTTOM, AICHAT_INSET_LEFT};
use op_editor_ui::widgets::{AI_CHAT_HEIGHT, AI_CHAT_MINIMIZED_HEIGHT, AI_CHAT_WIDTH};
use op_editor_ui::{Point2D, Rect};

impl WidgetHost {
    /// The panel's width in BOTH states — minimizing changes the height
    /// only, so the bar and the panel read this one function.
    pub(in crate::widget_host) fn ai_chat_panel_width(&self) -> f32 {
        AI_CHAT_WIDTH
    }

    pub(in crate::widget_host) fn ai_chat_size(&self) -> (f32, f32) {
        let height = if self.editor_state.chat.is_minimized() {
            AI_CHAT_MINIMIZED_HEIGHT
        } else {
            AI_CHAT_HEIGHT
        };
        (self.ai_chat_panel_width(), height)
    }

    pub(in crate::widget_host) fn ai_chat_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<Rect> {
        if self.editor_state.editor_ui.embed == op_editor_core::EmbedHost::VsCode {
            // The VS Code plugin is MCP-driven; the in-editor chat is hidden.
            return None;
        }
        // The chat is PINNED into a left column — the workspace's dock,
        // or the editor's Chat tab — whenever one owns it, through the
        // ONE shared helper the native host routes through as well. A
        // pinned-but-closed column means no chat at all, never a
        // floating fallback; touch (the sheet branch below) is never
        // pinned.
        use op_editor_ui::widgets::host_canvas_geometry::{pinned_chat, PinnedChat};
        match pinned_chat(&self.editor_state, viewport_w, viewport_h) {
            Some(PinnedChat::At(rect)) => return Some(rect),
            Some(PinnedChat::Closed) => return None,
            Some(PinnedChat::Composer { x, bottom, width }) => {
                // Composer-only: the card's height follows the draft and
                // focus, so the widget resolves it and the card grows
                // upward from the canvas floor — same seam as native.
                let panel = op_editor_ui::widgets::AIChatPlaceholder::from_editor_at(
                    &self.editor_state,
                    self.now_ms,
                );
                let height = panel.composer_only_height(width);
                return Some(Rect {
                    origin: Point2D::new(x, bottom - height),
                    size: Point2D::new(width, height),
                });
            }
            None => {}
        }
        let (cx0, cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        if self.editor_state.chat.is_minimized() {
            return op_editor_ui::widgets::host_canvas_geometry::minimized_chat_bar_rect(
                self.editor_state.chat.anchor,
                self.ai_chat_panel_width(),
                // `None`, not `chat.panel_position`: the web host's EXPANDED
                // rect is anchor-placed and reads no stored position, so
                // honouring one here would put the bar somewhere the panel
                // never was. Native reads it because native's panel does.
                None,
                cx0,
                cy0,
                cw,
                ch,
            );
        }
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
        let (panel_w, panel_h) = self.ai_chat_size();
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
            op_editor_core::ChatAnchor::TopLeft => {
                (cx0 + AICHAT_INSET_LEFT, cy0 + AICHAT_INSET_BOTTOM)
            }
            op_editor_core::ChatAnchor::TopRight => (
                cx0 + cw - panel_w - AICHAT_INSET_BOTTOM,
                cy0 + AICHAT_INSET_BOTTOM,
            ),
            op_editor_core::ChatAnchor::BottomLeft => (
                cx0 + AICHAT_INSET_LEFT,
                cy0 + ch - panel_h - AICHAT_INSET_BOTTOM,
            ),
            op_editor_core::ChatAnchor::BottomRight => (
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
