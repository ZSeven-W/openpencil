//! Editor-UI composition paint pass on `WidgetHostNative`.
//! Pulled out of `widget_host.rs` to keep the spine file under
//! the 800-line ceiling.

use super::frame_backend::NativeFrameBackend;
use super::helpers::{STATUS_INSET, TOOLBAR_INSET_X, TOOLBAR_INSET_Y};
use super::WidgetHostNative;
use openpencil_shell_core::widgets::{
    AIChatPlaceholder, AlignToolbar, CanvasViewport, LayerPanel, LayoutCx, LocalePicker, PaintCx,
    PropertyPanel, ShapePicker, StatusBar, Toolbar, TopBar, Widget, STATUS_BAR_HEIGHT,
    STATUS_BAR_WIDTH, TOOLBAR_WIDTH, TOP_BAR_HEIGHT,
};
use openpencil_shell_core::{Point2D, Rect, RenderBackend};

impl WidgetHostNative {
    /// Paint the editor-UI composition.
    pub fn paint(
        &self,
        frame: &mut NativeFrameBackend<'_>,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        // 1. Background fill so previous-frame pixels never bleed.
        frame.fill_rect(
            Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(viewport_width, viewport_height),
            },
            self.theme.background,
        );

        let dpi = frame.dpi_scale();

        // 2. TopBar.
        let top_bar = TopBar::for_document(&self.document);
        let top_bar_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_width, TOP_BAR_HEIGHT),
        };
        {
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            top_bar.paint(&mut cx, top_bar_rect);
        }

        // 3. LayerPanel — skipped when the sidebar is collapsed.
        if self.document.ui.sidebar_open {
            // Compute the active drop target so the panel can paint
            // the drop-indicator line during a drag-to-reorder.
            let layer_panel_rect = Rect {
                origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
                size: Point2D::new(
                    self.document.ui.layer_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            // Build the panel for paint. While a drag is active,
            // exclude the source's subtree so the rendered row stack
            // mirrors the post-commit layout — both the visible rows
            // and the drop-indicator y the user sees are then exactly
            // what `reorder_before/after` produces on release.
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
                // Floating ghost — keeps the source visible mid-drag.
                if let Some(item) = LayerPanel::ghost_item_for(&self.document, d.source) {
                    layer_panel.drag_ghost = Some((item, d.current_y));
                }
            }
            layer_panel.now_ms = self.now_ms;
            layer_panel.caret_anchor_ms = self.document.ui.rename_caret_anchor_ms;
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            layer_panel.paint(&mut cx, layer_panel_rect);
        }

        // 4. PropertyPanel — only when selection.
        let property_panel = PropertyPanel::for_selection_at(&self.document, self.now_ms);
        let has_property = property_panel.is_some();
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
                backend: &mut *frame,
            };
            panel.paint(&mut cx, property_rect);
        }

        // 5. CanvasViewport — middle band, respects sidebar
        //    collapse state.
        let (canvas_left, _canvas_y, canvas_w, canvas_h) =
            self.canvas_region(viewport_width, viewport_height);
        let _ = has_property;
        let canvas_rect = Rect {
            origin: Point2D::new(canvas_left, TOP_BAR_HEIGHT),
            size: Point2D::new(canvas_w, canvas_h),
        };
        if canvas_w > 0.0 && canvas_h > 0.0 {
            let mut canvas = CanvasViewport::from_document(&self.document);
            canvas.now_ms = self.now_ms;
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            canvas.paint(&mut cx, canvas_rect);
        }

        // 6. Toolbar — floating column.
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
                backend: &mut *frame,
            };
            toolbar.paint(&mut cx, toolbar_rect);
        }

        // 7. AIChatPlaceholder — painted LAST so it sits on top
        //    of the toolbar in any overlap region (matches the
        //    user's requested z-order: chat above toolbar).
        if let Some(chat_rect) = self.ai_chat_rect(viewport_width, viewport_height) {
            let chat = AIChatPlaceholder::from_document_at(&self.document, self.now_ms);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            chat.paint(&mut cx, chat_rect);
        }

        // 8. StatusBar — floating bottom-right.
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
                backend: &mut *frame,
            };
            status.paint(&mut cx, status_rect);
        }

        // 8.4. Floating align/distribute toolbar — visible whenever
        //      2+ nodes are selected. Sits above the canvas but
        //      below status / modal overlays.
        let canvas_region = Rect {
            origin: Point2D::new(canvas_left, TOP_BAR_HEIGHT),
            size: Point2D::new(canvas_w, canvas_h),
        };
        if let Some(toolbar) = AlignToolbar::for_canvas_region(canvas_region, &self.document) {
            toolbar.paint(
                &mut *frame,
                &self.theme,
                self.document.ui.align_toolbar_hover,
            );
        }

        // 8.5. Marquee selection rect — painted above canvas but
        //      below the floating pickers / status. Visible only
        //      while the user is dragging a rect-select on empty
        //      canvas (Select tool).
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
                // 10% primary-tinted fill so the rect reads as a
                // selection band without obscuring the canvas.
                let fill = openpencil_shell_core::Color {
                    r: primary.r,
                    g: primary.g,
                    b: primary.b,
                    a: primary.a * 0.12,
                };
                frame.fill_rect(rect, fill);
                frame.stroke_rect(rect, primary, 1.0);
            }
        }

        // 9. ShapePicker — anchored to the right of the toolbar
        //    shape slot; same z-priority as the locale picker.
        if self.document.ui.shape_picker_open {
            let picker_rect = self.shape_picker_rect(viewport_width, viewport_height);
            let picker = ShapePicker::for_document(&self.document);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            picker.paint(&mut cx, picker_rect);
        }

        // 10. LocalePicker — top-most overlay so it covers chat /
        //     toolbar / status when open.
        if self.document.ui.locale_picker_open {
            let picker_rect = self.locale_picker_rect(viewport_width);
            let picker = LocalePicker::for_document(&self.document);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            picker.paint(&mut cx, picker_rect);
        }

        // 10b. File-menu dropdown — anchored under TopBar's
        //      folder+chevron button.
        if self.document.ui.file_menu_open {
            use openpencil_shell_core::widgets::file_menu::FileMenu;
            use openpencil_shell_core::widgets::top_bar::TopBar;
            let top_bar_rect = Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(viewport_width, openpencil_shell_core::widgets::TOP_BAR_HEIGHT),
            };
            let anchor = TopBar::file_menu_rect(top_bar_rect);
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let menu = FileMenu::from_document(&self.document, now_secs);
            let menu_rect = menu.rect_at(anchor);
            let mut cx = PaintCx { backend: &mut *frame };
            menu.paint(&mut cx, menu_rect);
        }

        // 10c. Figma import modal — full-viewport scrim + centred card.
        if self.document.ui.figma_import_open {
            use openpencil_shell_core::widgets::figma_import::FigmaImportModal;
            frame.fill_rect(
                Rect {
                    origin: Point2D::new(0.0, 0.0),
                    size: Point2D::new(viewport_width, viewport_height),
                },
                openpencil_shell_core::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.45 },
            );
            let modal = FigmaImportModal::for_document(&self.document);
            let modal_rect = modal.rect(viewport_width, viewport_height);
            let mut cx = PaintCx { backend: &mut *frame };
            modal.paint(&mut cx, modal_rect);
        }

        // 10d. Export dialog — full-viewport scrim + centred card.
        if self.document.ui.export_dialog_open {
            use openpencil_shell_core::widgets::ExportDialog;
            frame.fill_rect(
                Rect {
                    origin: Point2D::new(0.0, 0.0),
                    size: Point2D::new(viewport_width, viewport_height),
                },
                openpencil_shell_core::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.45 },
            );
            let dlg = ExportDialog::centered(viewport_width, viewport_height);
            dlg.paint(&mut *frame, &self.theme, &self.document);
        }

        // 10a. Agent-settings modal — top-most overlay when open.
        if self.document.ui.agent_settings_open {
            use openpencil_shell_core::widgets::agent_settings_panel::AgentSettingsPanel;
            // Dim scrim across the full viewport.
            let scrim_color = openpencil_shell_core::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.45,
            };
            frame.fill_rect(
                Rect {
                    origin: Point2D::new(0.0, 0.0),
                    size: Point2D::new(viewport_width, viewport_height),
                },
                scrim_color,
            );
            let panel = AgentSettingsPanel::for_document(&self.document);
            let panel_rect = panel.rect(viewport_width, viewport_height);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            panel.paint(&mut cx, panel_rect);
        }

        // 10b. Color picker — floating overlay near the right rail.
        if let Some(state) = self.document.ui.color_picker.clone() {
            use openpencil_shell_core::widgets::color_picker::ColorPicker;
            let picker = ColorPicker::for_state(&self.document, state);
            let picker_rect = picker.rect(viewport_width, viewport_height);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            picker.paint(&mut cx, picker_rect);
        }

        // 11. Layer context menu — right-click overlay above
        //     everything else.
        if let Some(state) = self.document.ui.layer_context_menu {
            use openpencil_shell_core::widgets::layer_context_menu::LayerContextMenu;
            let menu = LayerContextMenu::for_state(&self.document, state);
            let menu_rect = menu.rect();
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            menu.paint(&mut cx, menu_rect);
        }
    }
}
