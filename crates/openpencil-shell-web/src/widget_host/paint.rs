//! Editor-UI composition paint pass for the web `WidgetHost`.
//! Pulled out of `widget_host.rs` to keep that file under the
//! 800-line ceiling. Mirrors the structure used by
//! `openpencil-shell-native/src/widget_host/paint.rs`.

use super::WidgetHost;
use crate::backend::WebBackend;
use openpencil_shell_core::widgets::{
    AIChatPlaceholder, CanvasViewport, LayerPanel, LayoutCx, LocalePicker, PaintCx, PropertyPanel,
    StatusBar, Toolbar, TopBar, Widget, STATUS_BAR_HEIGHT, STATUS_BAR_WIDTH, TOOLBAR_WIDTH,
    TOP_BAR_HEIGHT,
};
use openpencil_shell_core::{Point2D, Rect, RenderBackend};

use super::{STATUS_INSET, TOOLBAR_INSET_X, TOOLBAR_INSET_Y};

impl WidgetHost {
    /// Paint the full editor-UI composition. Layer order matches
    /// the native shell so paint output is cross-platform identical:
    ///   1. background fill
    ///   2. TopBar
    ///   3. LayerPanel (left rail, sidebar-gated)
    ///   4. PropertyPanel (right rail, selection-gated)
    ///   5. CanvasViewport (center band)
    ///   6. Toolbar (floating column)
    ///   7. AIChatPlaceholder (floating, painted late so it sits
    ///      on top of toolbar)
    ///   8. StatusBar (floating bottom-right)
    ///   9. LocalePicker (top-most overlay)
    // glue:
    pub fn paint(&self, backend: &mut WebBackend, viewport_width: f32, viewport_height: f32) {
        backend.fill_rect(
            Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(viewport_width, viewport_height),
            },
            self.theme.background,
        );

        let dpi = backend.dpi_scale();

        let top_bar = TopBar::for_document(&self.document);
        let top_bar_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_width, TOP_BAR_HEIGHT),
        };
        {
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            top_bar.paint(&mut cx, top_bar_rect);
        }

        if self.document.ui.sidebar_open {
            let layer_panel_rect = self.layer_panel_rect(viewport_height);
            // While a drag is active, paint against a panel with the
            // source's subtree excluded — see native paint.rs.
            let active_drag = self.layer_drag.filter(|d| {
                d.active
                    && self
                        .document
                        .active_page()
                        .map(|p| p.find(d.source).is_some())
                        .unwrap_or(false)
            });
            let mut layer_panel = if let Some(d) = active_drag {
                LayerPanel::from_document_with_drag_source(&self.document, d.source)
            } else {
                LayerPanel::from_document(&self.document)
            };
            if let Some(d) = active_drag {
                layer_panel.drop_target = layer_panel
                    .drop_target_at(layer_panel_rect, Point2D::new(d.current_x, d.current_y));
                if let Some(item) = LayerPanel::ghost_item_for(&self.document, d.source) {
                    layer_panel.drag_ghost = Some((item, d.current_y));
                }
            }
            // Web has no per-frame time source; caret paints solid
            // (now == anchor == 0 ⇒ blink_visible returns true).
            layer_panel.now_ms = 0;
            layer_panel.caret_anchor_ms = 0;
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            layer_panel.paint(&mut cx, layer_panel_rect);
        }

        let property_panel = PropertyPanel::for_selection(&self.document);
        if let Some(panel) = property_panel.as_ref() {
            let property_rect = Rect {
                origin: Point2D::new(
                    viewport_width - self.document.ui.property_panel_width,
                    TOP_BAR_HEIGHT,
                ),
                size: Point2D::new(
                    self.document.ui.property_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            panel.paint(&mut cx, property_rect);
        }

        let (canvas_left, _canvas_y, canvas_w, canvas_h) =
            self.canvas_region(viewport_width, viewport_height);
        let canvas_rect = Rect {
            origin: Point2D::new(canvas_left, TOP_BAR_HEIGHT),
            size: Point2D::new(canvas_w, canvas_h),
        };
        if canvas_w > 0.0 && canvas_h > 0.0 {
            let canvas = CanvasViewport::from_document(&self.document);
            // Web has no per-frame clock; caret stays solid.
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            canvas.paint(&mut cx, canvas_rect);
        }

        let toolbar = Toolbar::for_document(&self.document);
        let toolbar_h = toolbar
            .layout(&LayoutCx {
                available_width: TOOLBAR_WIDTH,
                dpi,
            })
            .rect
            .size
            .y;
        let toolbar_rect = Rect {
            origin: Point2D::new(
                canvas_left + TOOLBAR_INSET_X,
                TOP_BAR_HEIGHT + TOOLBAR_INSET_Y,
            ),
            size: Point2D::new(TOOLBAR_WIDTH, toolbar_h),
        };
        if canvas_w > TOOLBAR_WIDTH + TOOLBAR_INSET_X * 2.0 {
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            toolbar.paint(&mut cx, toolbar_rect);
        }

        if let Some(chat_rect) = self.ai_chat_rect(viewport_width, viewport_height) {
            let chat = AIChatPlaceholder::from_document(&self.document);
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            chat.paint(&mut cx, chat_rect);
        }

        let canvas_right = canvas_left + canvas_w;
        if canvas_w > STATUS_BAR_WIDTH + STATUS_INSET * 2.0 {
            let status = StatusBar::for_document(&self.document);
            let status_rect = Rect {
                origin: Point2D::new(
                    canvas_right - STATUS_BAR_WIDTH - STATUS_INSET,
                    TOP_BAR_HEIGHT + canvas_h - STATUS_BAR_HEIGHT - STATUS_INSET,
                ),
                size: Point2D::new(STATUS_BAR_WIDTH, STATUS_BAR_HEIGHT),
            };
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            status.paint(&mut cx, status_rect);
        }

        // Marquee selection rect — between StatusBar and the
        // floating pickers in z-order, only while a marquee
        // drag is active.
        if let Some(m) = self.marquee_drag {
            let x0 = m.start_screen_x.min(m.current_screen_x);
            let y0 = m.start_screen_y.min(m.current_screen_y);
            let w = (m.current_screen_x - m.start_screen_x).abs();
            let h = (m.current_screen_y - m.start_screen_y).abs();
            if w >= 1.0 && h >= 1.0 {
                let rect = Rect {
                    origin: Point2D::new(x0, y0),
                    size: Point2D::new(w, h),
                };
                let primary = self.theme.primary;
                let fill = openpencil_shell_core::Color {
                    r: primary.r,
                    g: primary.g,
                    b: primary.b,
                    a: primary.a * 0.12,
                };
                backend.fill_rect(rect, fill);
                backend.stroke_rect(rect, primary, 1.0);
            }
        }

        if self.document.ui.locale_picker_open {
            let picker_rect = self.locale_picker_rect(viewport_width);
            let picker = LocalePicker::for_document(&self.document);
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            picker.paint(&mut cx, picker_rect);
        }

        // Layer context menu — right-click overlay, top of stack.
        if let Some(state) = self.document.ui.layer_context_menu {
            use openpencil_shell_core::widgets::layer_context_menu::LayerContextMenu;
            let menu = LayerContextMenu::for_state(&self.document, state);
            let menu_rect = menu.rect();
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            menu.paint(&mut cx, menu_rect);
        }

        // Settings modal — Cmd+, overlay, top-most.
        if self.document.ui.agent_settings_open {
            use openpencil_shell_core::widgets::agent_settings_panel::AgentSettingsPanel;
            let panel = AgentSettingsPanel::for_document(&self.document);
            let panel_rect = panel.rect(viewport_width, viewport_height);
            // Dim scrim behind the modal so the underlying canvas
            // reads as "blocked." Matches the native shell's chrome.
            backend.fill_rect(
                Rect {
                    origin: Point2D::new(0.0, 0.0),
                    size: Point2D::new(viewport_width, viewport_height),
                },
                openpencil_shell_core::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.5 },
            );
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            panel.paint(&mut cx, panel_rect);
        }
    }
}
