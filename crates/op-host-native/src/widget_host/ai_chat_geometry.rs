//! AI chat floating-panel geometry.

use super::helpers::{AICHAT_INSET_BOTTOM, AICHAT_INSET_LEFT};
use super::WidgetHostNative;
use op_editor_core::ChatAnchor;
use op_editor_ui::widgets::{AI_CHAT_MINIMIZED_HEIGHT, AI_CHAT_MIN_HEIGHT, AI_CHAT_MIN_WIDTH};
use op_editor_ui::{Point2D, Rect};

impl WidgetHostNative {
    /// Bottom edge available to a touch AI surface while one of its inputs
    /// owns the software keyboard. The editor's primary viewport stays stable;
    /// only this focused surface consumes the host-local occlusion channel.
    fn touch_ai_visible_bottom(&self, viewport_h: f32) -> f32 {
        if self.editor_state.chat.focused || self.editor_state.editor_ui.chat_model_picker.open {
            self.keyboard_visible_bottom(viewport_h)
        } else {
            viewport_h
        }
    }

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
        // Native touch chrome uses a modal bottom sheet on phones and a
        // non-modal bounded trailing assistant on tablets. The latter keeps
        // the canvas legible and interactive while the chat remains open.
        if self.editor_state.editor_ui.touch_chrome() {
            if self.editor_state.editor_ui.mobile_sheet
                != Some(op_editor_core::size_class::MobileSheetKind::Ai)
            {
                return None;
            }
            let visible_bottom = self.touch_ai_visible_bottom(viewport_h);
            if self.editor_state.editor_ui.compact_layout() {
                let max_h = (visible_bottom
                    - op_editor_ui::widgets::host_canvas_geometry::touch_app_bar_height(
                        &self.editor_state,
                    ))
                .max(0.0);
                let min_h = 280.0_f32.min(max_h);
                let sheet_h = (viewport_h * 0.58).clamp(min_h, max_h);
                return Some(Rect {
                    origin: Point2D::new(0.0, visible_bottom - sheet_h),
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
                size: Point2D::new(panel_w, (visible_bottom - top - inset).max(0.0)),
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

#[cfg(test)]
mod keyboard_avoidance_tests {
    use super::*;
    use op_editor_core::size_class::{EditorSizeClass, MobileSheetKind};
    use op_editor_ui::widgets::{host_canvas_geometry, AIChatPlaceholder};

    fn touch_ai_host(size_class: EditorSizeClass) -> WidgetHostNative {
        let mut host = WidgetHostNative::new();
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.touch = true;
        ui.size_class = size_class;
        ui.sidebar_open = size_class.is_expanded();
        ui.mobile_sheet = Some(MobileSheetKind::Ai);
        host
    }

    fn bottom(rect: Rect) -> f32 {
        rect.origin.y + rect.size.y
    }

    #[test]
    fn compact_focused_chat_sheet_tracks_keyboard_without_moving_main_chrome() {
        let (viewport_w, viewport_h) = (390.0, 844.0);
        let mut host = touch_ai_host(EditorSizeClass::Compact);
        let resting = host
            .ai_chat_rect(viewport_w, viewport_h)
            .expect("touch AI sheet");
        let app_bar = host_canvas_geometry::touch_app_bar_rect(host.editor_state(), viewport_w);
        let canvas = host_canvas_geometry::canvas_rect(host.editor_state(), viewport_w, viewport_h);
        let dock =
            host_canvas_geometry::touch_dock_rect(host.editor_state(), viewport_w, viewport_h);

        host.editor_state_mut().chat.focused = true;
        assert!(host.set_keyboard_occlusion(300.0));
        let visible_bottom = host.keyboard_visible_bottom(viewport_h);
        let raised = host
            .ai_chat_rect(viewport_w, viewport_h)
            .expect("keyboard-aware touch AI sheet");

        assert_eq!(bottom(raised), visible_bottom);
        assert_eq!(raised.origin.x, resting.origin.x);
        assert_eq!(raised.size, resting.size);
        assert!(raised.origin.y < resting.origin.y);
        let chat = AIChatPlaceholder::from_editor(host.editor_state());
        assert!(bottom(chat.input_caret_rect(raised)) <= visible_bottom);
        assert_eq!(
            host_canvas_geometry::touch_app_bar_rect(host.editor_state(), viewport_w),
            app_bar
        );
        assert_eq!(
            host_canvas_geometry::canvas_rect(host.editor_state(), viewport_w, viewport_h),
            canvas
        );
        assert_eq!(
            host_canvas_geometry::touch_dock_rect(host.editor_state(), viewport_w, viewport_h),
            dock
        );

        assert!(host.set_keyboard_occlusion(0.0));
        assert_eq!(host.ai_chat_rect(viewport_w, viewport_h), Some(resting));
    }

    #[test]
    fn tablet_ai_surface_caps_only_its_bottom_for_each_keyboard_owner() {
        for (size_class, viewport_w, viewport_h) in [
            (EditorSizeClass::Medium, 834.0, 1112.0),
            (EditorSizeClass::Expanded, 1194.0, 834.0),
        ] {
            for model_picker_open in [false, true] {
                let mut host = touch_ai_host(size_class);
                host.editor_state_mut().chat.focused = !model_picker_open;
                host.editor_state_mut().editor_ui.chat_model_picker.open = model_picker_open;
                let resting = host
                    .ai_chat_rect(viewport_w, viewport_h)
                    .expect("tablet AI surface");

                assert!(host.set_keyboard_occlusion(320.0));
                let visible_bottom = host.keyboard_visible_bottom(viewport_h);
                let capped = host
                    .ai_chat_rect(viewport_w, viewport_h)
                    .expect("keyboard-aware tablet AI surface");

                assert_eq!(capped.origin, resting.origin);
                assert_eq!(capped.size.x, resting.size.x);
                assert_eq!(bottom(capped), visible_bottom - 12.0);
                assert!(capped.size.y < resting.size.y);
                let chat = AIChatPlaceholder::from_editor(host.editor_state());
                assert!(bottom(chat.input_caret_rect(capped)) <= visible_bottom);

                assert!(host.set_keyboard_occlusion(0.0));
                assert_eq!(host.ai_chat_rect(viewport_w, viewport_h), Some(resting));
            }
        }
    }

    #[test]
    fn touch_ai_surface_ignores_keyboard_without_an_ai_input_owner() {
        let (viewport_w, viewport_h) = (834.0, 1112.0);
        let mut host = touch_ai_host(EditorSizeClass::Medium);
        let resting = host
            .ai_chat_rect(viewport_w, viewport_h)
            .expect("tablet AI surface");

        assert!(host.set_keyboard_occlusion(320.0));
        assert_eq!(host.ai_chat_rect(viewport_w, viewport_h), Some(resting));
    }
}
