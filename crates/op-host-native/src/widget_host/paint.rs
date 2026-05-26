//! Editor-UI composition paint pass on `WidgetHostNative`.
//! Pulled out of `widget_host.rs` to keep the spine file under
//! the 800-line ceiling.

use super::frame_backend::NativeFrameBackend;
use super::helpers::{STATUS_INSET, TOOLBAR_INSET_X, TOOLBAR_INSET_Y};
use super::WidgetHostNative;
use op_editor_ui::widgets::{
    variables_panel::VariablesPanel, AIChatPlaceholder, AlignToolbar, CanvasViewport,
    ComponentBrowserPanel, DesignMdPanel, GitPanel, IconPickerPanel, LayerPanel, LayoutCx,
    LocalePicker, PaintCx, PropertyPanel, ShapePicker, StatusBar, Toolbar, TopBar, Widget,
    GIT_PANEL_INSET, STATUS_BAR_HEIGHT, STATUS_BAR_WIDTH, TOOLBAR_WIDTH, TOP_BAR_HEIGHT,
};
use op_editor_ui::{Point2D, Rect, RenderBackend};

impl WidgetHostNative {
    /// Paint the editor-UI composition.
    pub fn paint(
        &mut self,
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

        // Rebuild the layout-resolved render scene ONCE for the whole
        // paint pass. Every widget builder below reads `editor_state`
        // directly; the canvas reads `self.layout_scene`.
        self.refresh_layout_scene();
        let ui = &self.editor_state.editor_ui;

        // 2. TopBar.
        let top_bar = TopBar::for_editor_ui(&self.editor_state.editor_ui);
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
        if ui.sidebar_open {
            // Compute the active drop target so the panel can paint
            // the drop-indicator line during a drag-to-reorder.
            let layer_panel_rect = Rect {
                origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
                size: Point2D::new(
                    ui.layer_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            // Build the panel for paint. While a drag is active,
            // exclude the source's subtree so the rendered row stack
            // mirrors the post-commit layout — both the visible rows
            // and the drop-indicator y the user sees are then exactly
            // what `reorder_before/after` produces on release.
            // The panel walks the canonical `PenNode` tree directly
            // off `EditorState`; the drag source id is shell-core's
            // `NodeId` (from the input path), losslessly accepted.
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
                // Floating ghost — keeps the source visible mid-drag.
                if let Some(item) = LayerPanel::ghost_item_for(&self.editor_state, &d.source) {
                    layer_panel.drag_ghost = Some((item, d.current_y));
                }
            }
            layer_panel.now_ms = self.now_ms;
            layer_panel.caret_anchor_ms = self.editor_state.editor_ui.rename_caret_anchor_ms;
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            layer_panel.paint(&mut cx, layer_panel_rect);
        }

        // 4. CanvasViewport — middle band, respects sidebar
        //    collapse state. It paints before the right rail so
        //    PropertyPanel popovers can extend into the canvas.
        let (canvas_left, _canvas_y, canvas_w, canvas_h) =
            self.canvas_region(viewport_width, viewport_height);
        let canvas_rect = Rect {
            origin: Point2D::new(canvas_left, TOP_BAR_HEIGHT),
            size: Point2D::new(canvas_w, canvas_h),
        };
        if canvas_w > 0.0 && canvas_h > 0.0 {
            // PAINT path — the canvas reads editor state + the
            // layout-resolved render scene (`refresh_layout_scene`).
            let mut canvas = CanvasViewport::from_editor(&self.editor_state, &self.layout_scene);
            canvas.now_ms = self.now_ms;
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            canvas.paint(&mut cx, canvas_rect);
        }

        // 5. PropertyPanel — only when selection.
        let property_panel = PropertyPanel::for_selection_at(&self.editor_state, self.now_ms);
        let property_panel_width = ui.property_panel_width;
        let right_rail_x = viewport_width - property_panel_width;
        if let Some(panel) = property_panel.as_ref() {
            let property_rect = Rect {
                origin: Point2D::new(right_rail_x, TOP_BAR_HEIGHT),
                size: Point2D::new(
                    property_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            panel.paint(&mut cx, property_rect);
        }

        // 5b. VariablesPanel — mirrors TS' `{}` toolbar toggle as a
        //     floating canvas overlay next to the toolbar.
        if let Some(vars_rect) = self.variables_panel_rect(viewport_width, viewport_height) {
            let vars = VariablesPanel::for_editor_at(&self.editor_state, self.now_ms);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            vars.paint(&mut cx, vars_rect);
        }

        // 6. Toolbar — floating column.
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
                backend: &mut *frame,
            };
            toolbar.paint(&mut cx, toolbar_rect);
        }

        // 7. AIChatPlaceholder — painted LAST so it sits on top
        //    of the toolbar in any overlap region (matches the
        //    user's requested z-order: chat above toolbar).
        if let Some(chat_rect) = self.ai_chat_rect(viewport_width, viewport_height) {
            let chat = AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            chat.paint(&mut cx, chat_rect);
        }

        // 8. StatusBar — floating bottom-right.
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
                backend: &mut *frame,
            };
            status.paint(&mut cx, status_rect);
        }

        // 8.2. Floating Git panel — read-only repository status,
        //      toggled from the View menu. Floats at the canvas's
        //      top-left, below the TopBar.
        if let Some(panel) = GitPanel::for_editor(&self.editor_state) {
            let panel_rect = Rect {
                origin: Point2D::new(
                    canvas_left + GIT_PANEL_INSET,
                    TOP_BAR_HEIGHT + GIT_PANEL_INSET,
                ),
                size: Point2D::new(panel.panel_width(), panel.height()),
            };
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            panel.paint(&mut cx, panel_rect);
        }

        // 8.4. Floating align/distribute toolbar — visible whenever
        //      2+ nodes are selected. Sits above the canvas but
        //      below status / modal overlays.
        let canvas_region = Rect {
            origin: Point2D::new(canvas_left, TOP_BAR_HEIGHT),
            size: Point2D::new(canvas_w, canvas_h),
        };
        if let Some(toolbar) = AlignToolbar::for_canvas_region(canvas_region, &self.editor_state) {
            let hover = self.editor_state.editor_ui.align_toolbar_hover;
            toolbar.paint(&mut *frame, &self.theme, hover);
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
                let fill = op_editor_ui::Color {
                    r: primary.r,
                    g: primary.g,
                    b: primary.b,
                    a: primary.a * 0.12,
                };
                frame.fill_rect(rect, fill);
                frame.stroke_rect(rect, primary, 1.0);
            }
        }

        // 8.6. PropertyPanel overlays — painted after canvas floating
        //      controls so the image-fill popover can cover the zoom
        //      status pill when it extends into the canvas.
        if let Some(panel) = property_panel.as_ref() {
            let property_rect = Rect {
                origin: Point2D::new(right_rail_x, TOP_BAR_HEIGHT),
                size: Point2D::new(
                    property_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            panel.paint_overlays(&mut cx, property_rect);
        }

        // 9. ShapePicker — anchored to the right of the toolbar
        //    shape slot; same z-priority as the locale picker.
        if ui.shape_picker_open {
            let picker_rect = self.shape_picker_rect(viewport_width, viewport_height);
            let picker = ShapePicker::for_editor_ui(&self.editor_state.editor_ui);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            picker.paint(&mut cx, picker_rect);
        }

        // 10. LocalePicker — top-most overlay so it covers chat /
        //     toolbar / status when open.
        if ui.locale_picker_open {
            let picker_rect = self.locale_picker_rect(viewport_width);
            let picker = LocalePicker::for_editor_ui(&self.editor_state.editor_ui);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            picker.paint(&mut cx, picker_rect);
        }

        // 10b. File-menu dropdown — anchored under TopBar's
        //      folder+chevron button.
        if ui.file_menu_open {
            use op_editor_ui::widgets::file_menu::FileMenu;
            use op_editor_ui::widgets::top_bar::TopBar;
            let top_bar_rect = Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(viewport_width, op_editor_ui::widgets::TOP_BAR_HEIGHT),
            };
            let anchor =
                TopBar::file_menu_rect(top_bar_rect, self.editor_state.editor_ui.window_fullscreen);
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let menu = FileMenu::from_editor_ui(&self.editor_state.editor_ui, now_secs);
            let menu_rect = menu.rect_at(anchor);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            menu.paint(&mut cx, menu_rect);
        }

        // 10c. Figma import modal — full-viewport scrim + centred card.
        if ui.figma_import_open {
            use op_editor_ui::widgets::figma_import::FigmaImportModal;
            frame.fill_rect(
                Rect {
                    origin: Point2D::new(0.0, 0.0),
                    size: Point2D::new(viewport_width, viewport_height),
                },
                op_editor_ui::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.45,
                },
            );
            let modal = FigmaImportModal::for_editor(&self.editor_state);
            let modal_rect = modal.rect(viewport_width, viewport_height);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            modal.paint(&mut cx, modal_rect);
        }

        // 10d. Export dialog — full-viewport scrim + centred card.
        if ui.export_dialog_open {
            use op_editor_ui::widgets::ExportDialog;
            frame.fill_rect(
                Rect {
                    origin: Point2D::new(0.0, 0.0),
                    size: Point2D::new(viewport_width, viewport_height),
                },
                op_editor_ui::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.45,
                },
            );
            let dlg = ExportDialog::centered(viewport_width, viewport_height);
            dlg.paint(&mut *frame, &self.theme, &self.editor_state.editor_ui);
        }

        // 10a. Agent-settings modal — top-most overlay when open.
        if ui.agent_settings_open {
            use op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel;
            // Dim scrim across the full viewport.
            let scrim_color = op_editor_ui::Color {
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
            let panel = AgentSettingsPanel::for_editor(&self.editor_state);
            let panel_rect = panel.rect(viewport_width, viewport_height);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            panel.paint(&mut cx, panel_rect);
        }

        // 10b. Color picker — floating overlay near the right rail.
        if let Some(state) = self.editor_state.ui.color_picker.clone() {
            use op_editor_ui::widgets::color_picker::ColorPicker;
            let picker = ColorPicker::for_state(&self.editor_state, state);
            let picker_rect = picker.rect(viewport_width, viewport_height);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            picker.paint(&mut cx, picker_rect);
        }

        // 11. Layer context menu — right-click overlay above
        //     everything else.
        if let Some(state) = self.editor_state.editor_ui.layer_context_menu.clone() {
            use op_editor_ui::widgets::layer_context_menu::LayerContextMenu;
            let menu = LayerContextMenu::for_state(&self.editor_state, state);
            let menu_rect = menu.rect();
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            menu.paint(&mut cx, menu_rect);
        }

        // 11.5. Floating Component-Browser panel — the UIKit library
        //       browser, toggled from the View menu. Painted just
        //       below the Design-MD panel so when both are open the
        //       Design-MD panel sits absolute-top.
        if let (Some(panel), Some(panel_rect)) = (
            ComponentBrowserPanel::for_editor(&self.editor_state),
            self.component_browser_panel_rect(viewport_width, viewport_height),
        ) {
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            panel.paint(&mut cx, panel_rect);
        }

        // 11.7. Floating Icon picker — opened from the shape-tool
        //       dropdown. It sits above the component browser and
        //       below Design-MD, matching the press routing order.
        if let (Some(panel), Some(panel_rect)) = (
            IconPickerPanel::for_editor_at(&self.editor_state, self.now_ms),
            self.icon_picker_panel_rect(viewport_width, viewport_height),
        ) {
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            panel.paint(&mut cx, panel_rect);
        }

        // 12. Floating Design-MD panel — the document's design.md
        //     brief, toggled from the View menu. Painted last so it
        //     is the top-most overlay: a deliberately-opened,
        //     draggable panel that captures clicks on its rect ahead
        //     of every lower layer (hit-test mirrors this — see
        //     `press.rs`, dispatched first).
        if let (Some(panel), Some(panel_rect)) = (
            DesignMdPanel::for_editor(&self.editor_state),
            self.design_md_panel_rect(viewport_width, viewport_height),
        ) {
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            panel.paint(&mut cx, panel_rect);
        }
    }
}
