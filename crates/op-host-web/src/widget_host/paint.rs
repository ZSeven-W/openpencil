//! Editor-UI composition paint pass for the web `WidgetHost`.
//! Pulled out of `widget_host.rs` to keep that file under the
//! 800-line ceiling. Mirrors the structure used by
//! `openpencil-shell-native/src/widget_host/paint.rs`.
//!
//! `paint` takes `&mut self`: it rebuilds the layout-resolved
//! `LayoutScene` (`refresh_layout_scene`) at the top of the pass,
//! then every widget builder reads `editor_state` directly and the
//! canvas reads the render scene.

use super::WidgetHost;
use crate::backend::WebBackend;
use op_editor_ui::widgets::{
    AIChatPlaceholder, CanvasViewport, LayerPanel, LayoutCx, LocalePicker, PaintCx, PropertyPanel,
    StatusBar, Toolbar, TopBar, VariablesModal, VariablesPanel, Widget, STATUS_BAR_HEIGHT,
    STATUS_BAR_WIDTH, TOOLBAR_WIDTH, TOP_BAR_HEIGHT,
};
use op_editor_ui::{Point2D, Rect, RenderBackend};

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
    pub fn paint(&mut self, backend: &mut WebBackend, viewport_width: f32, viewport_height: f32) {
        self.sync_theme_from_editor();
        backend.fill_rect(
            Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(viewport_width, viewport_height),
            },
            self.theme.background,
        );

        let dpi = backend.dpi_scale();

        // Rebuild the layout-resolved render scene ONCE for the whole
        // paint pass. Every widget builder below reads `editor_state`
        // directly; the canvas reads `self.layout_scene`.
        self.refresh_layout_scene();
        let ui = &self.editor_state.editor_ui;

        let top_bar = TopBar::for_editor_ui(&self.editor_state.editor_ui);
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

        if ui.sidebar_open {
            let layer_panel_rect = Rect {
                origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
                size: Point2D::new(
                    ui.layer_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            // While a drag is active, paint against a panel with the
            // source's subtree excluded — see native paint.rs. The
            // panel walks the canonical `PenNode` tree off
            // `EditorState`; the drag source id is shell-core's
            // `NodeId` from the input path, losslessly accepted.
            let active_drag = self.layer_drag.clone().filter(|d| {
                d.active
                    && self
                        .layout_scene
                        .active_page()
                        .map(|p| p.find(d.source.as_str()).is_some())
                        .unwrap_or(false)
            });
            let mut layer_panel = if let Some(d) = &active_drag {
                LayerPanel::from_editor_with_drag_source(&self.editor_state, &d.source)
            } else {
                LayerPanel::from_editor(&self.editor_state)
            };
            if let Some(d) = &active_drag {
                layer_panel.drop_target = layer_panel
                    .drop_target_at(layer_panel_rect, Point2D::new(d.current_x, d.current_y));
                if let Some(item) = LayerPanel::ghost_item_for(&self.editor_state, &d.source) {
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

        let (canvas_left, _canvas_y, canvas_w, canvas_h) =
            self.canvas_region(viewport_width, viewport_height);
        let canvas_rect = Rect {
            origin: Point2D::new(canvas_left, TOP_BAR_HEIGHT),
            size: Point2D::new(canvas_w, canvas_h),
        };
        if canvas_w > 0.0 && canvas_h > 0.0 {
            // PAINT path — the canvas reads editor state + the
            // layout-resolved render scene (`refresh_layout_scene`).
            // Web has no per-frame clock; caret stays solid.
            let canvas = CanvasViewport::from_editor(&self.editor_state, &self.layout_scene);
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            canvas.paint(&mut cx, canvas_rect);
        }

        let property_panel = PropertyPanel::for_selection_at(&self.editor_state, self.now_ms);
        if let Some(panel) = property_panel.as_ref() {
            let property_rect = Rect {
                origin: Point2D::new(viewport_width - ui.property_panel_width, TOP_BAR_HEIGHT),
                size: Point2D::new(
                    ui.property_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            panel.paint(&mut cx, property_rect);
        }

        let has_variable_table = self
            .editor_state
            .doc
            .variables
            .as_ref()
            .map(|v| !v.is_empty())
            .unwrap_or(false);
        let show_variables = has_variable_table && property_panel.is_none();
        if show_variables {
            let vars = VariablesPanel::for_editor(&self.editor_state);
            let vars_rect = Rect {
                origin: Point2D::new(
                    viewport_width - ui.property_panel_width,
                    TOP_BAR_HEIGHT + 8.0,
                ),
                size: Point2D::new(ui.property_panel_width, vars.intrinsic_height()),
            };
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            vars.paint(&mut cx, vars_rect);
        }

        let toolbar = Toolbar::for_editor(&self.editor_state);
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
            let chat = AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms);
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            chat.paint(&mut cx, chat_rect);
        }

        let canvas_right = canvas_left + canvas_w;
        if canvas_w > STATUS_BAR_WIDTH + STATUS_INSET * 2.0 {
            let status = StatusBar::for_editor(&self.editor_state);
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

        // Floating align/distribute toolbar — visible whenever 2+
        // nodes are selected. Sits above the canvas but below
        // marquee / pickers / modals.
        {
            use op_editor_ui::widgets::AlignToolbar;
            let canvas_region = Rect {
                origin: Point2D::new(canvas_left, TOP_BAR_HEIGHT),
                size: Point2D::new(canvas_w, canvas_h),
            };
            if let Some(tb) = AlignToolbar::for_canvas_region(canvas_region, &self.editor_state) {
                let hover = self.editor_state.editor_ui.align_toolbar_hover;
                tb.paint(&mut *backend, &self.theme, hover);
            }
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
                let fill = op_editor_ui::Color {
                    r: primary.r,
                    g: primary.g,
                    b: primary.b,
                    a: primary.a * 0.12,
                };
                backend.fill_rect(rect, fill);
                backend.stroke_rect(rect, primary, 1.0);
            }
        }

        // PropertyPanel overlays — painted after canvas floating
        // controls so the image-fill popover can cover the zoom
        // status pill when it extends into the canvas.
        if let Some(panel) = property_panel.as_ref() {
            let property_rect = Rect {
                origin: Point2D::new(viewport_width - ui.property_panel_width, TOP_BAR_HEIGHT),
                size: Point2D::new(
                    ui.property_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            panel.paint_overlays(&mut cx, property_rect);
        }

        if ui.locale_picker_open {
            let picker_rect = self.locale_picker_rect(viewport_width);
            let picker = LocalePicker::for_editor_ui(&self.editor_state.editor_ui);
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            picker.paint(&mut cx, picker_rect);
        }

        if ui.variables_panel_open {
            let modal = VariablesModal::for_editor(&self.editor_state);
            let modal_rect = modal.rect(viewport_width, viewport_height);
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            modal.paint(&mut cx, modal_rect);
        }

        // Layer context menu — right-click overlay, top of stack.
        if let Some(state) = self.editor_state.editor_ui.layer_context_menu.clone() {
            use op_editor_ui::widgets::layer_context_menu::LayerContextMenu;
            let menu = LayerContextMenu::for_state(&self.editor_state, state);
            let menu_rect = menu.rect();
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            menu.paint(&mut cx, menu_rect);
        }

        // Settings modal — Cmd+, overlay, top-most.
        if ui.agent_settings_open {
            use op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel;
            let panel = AgentSettingsPanel::for_editor_at(&self.editor_state, self.now_ms);
            let panel_rect = panel.rect(viewport_width, viewport_height);
            // Dim scrim behind the modal so the underlying canvas
            // reads as "blocked." Matches the native shell's chrome.
            backend.fill_rect(
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
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            panel.paint(&mut cx, panel_rect);
        }
    }
}
