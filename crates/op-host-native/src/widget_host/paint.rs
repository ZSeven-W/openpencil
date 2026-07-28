//! Editor-UI composition paint pass on `WidgetHostNative`.
//! Pulled out of `widget_host.rs` to keep the spine file under
//! the 800-line ceiling.

use super::helpers::{GIT_PANEL_CARET_H, GIT_PANEL_CARET_HALF};
use super::WidgetHostNative;
use crate::backend::NativeFrameBackend;
use op_editor_ui::widgets::editor_state_ext::theme_for;
use op_editor_ui::widgets::host_canvas_geometry as canvas_geometry;
use op_editor_ui::widgets::{
    variables_panel::VariablesPanel, AIChatPlaceholder, AlignToolbar, CanvasViewport,
    ComponentBrowserPanel, DesignMdPanel, GitPanel, IconPickerPanel, LayerPanel, LayoutCx,
    LocalePicker, PaintCx, PropertyPanel, ShapePicker, StatusBar, Toolbar, TopBar, Widget,
    TOOLBAR_WIDTH, TOP_BAR_HEIGHT,
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
        self.image_input_geometry = None;
        // Rotate the transcript-cache owner if the active chat session changed
        // since the last frame, BEFORE any resolve stores under it — the new
        // tab's build is then stamped with the fresh owner.
        self.rotate_chat_owner_if_session_changed();
        self.sync_theme_from_editor();
        // 1. Background fill so previous-frame pixels never bleed.
        frame.fill_rect(
            Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(viewport_width, viewport_height),
            },
            self.theme.background,
        );

        let dpi = frame.dpi_scale();

        // During document import, keep the frame path independent from
        // document layout/canvas paint. The parser can be CPU-heavy;
        // rebuilding or painting the old scene here makes the loading
        // overlay appear frozen.
        if self.editor_state.editor_ui.figma_import_in_progress {
            use op_editor_ui::widgets::figma_import_progress::ImportProgressOverlay;
            frame.fill_rect(
                Rect {
                    origin: Point2D::new(0.0, 0.0),
                    size: Point2D::new(viewport_width, viewport_height),
                },
                op_editor_ui::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.55,
                },
            );
            let overlay = ImportProgressOverlay::for_editor(&self.editor_state, self.now_ms);
            let rect = overlay.rect(viewport_width, viewport_height);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            overlay.paint(&mut cx, rect);
            return;
        }

        // Track M-1: finalize a finished canvas ↔ device-frame merge
        // animation BEFORE anything below reads `self.preview` /
        // `device_mode_active()` this frame — an Exit whose 220ms
        // window just elapsed drops the runtime here, so the rest of
        // this pass sees the settled (real) mode immediately rather
        // than one stale frame late.
        self.settle_mode_transition();

        // APP MODE per-frame reconcile: drain rejected navs into
        // `preview.warnings` and, on a screen switch, re-center the
        // viewport on the newly-mounted screen. Runs before the `ui`
        // borrow below (and before the preview paint branch further
        // down) so the switched screen paints this same frame, not one
        // frame late, and so the `preview.warnings` write here doesn't
        // conflict with `ui`'s borrow of `self.editor_state.editor_ui`.
        let mut preview_switched = false;
        let mut switched_screen_rect = None;
        if let Some(preview) = self.preview.as_mut() {
            let outcome = preview.reconcile(self.now_ms);
            if outcome.repaint {
                self.editor_state.editor_ui.preview.warnings = preview.warnings().to_vec();
            }
            if outcome.switched {
                preview_switched = true;
                switched_screen_rect = preview.current_screen_scene_rect();
            }
        }
        if preview_switched {
            if self.device_mode_active() {
                self.on_preview_screen_switched(viewport_width, viewport_height);
            } else if let Some(rect) = switched_screen_rect {
                self.center_canvas_on(rect, viewport_width, viewport_height);
            }
        }

        // Rebuild the layout-resolved render scene ONCE for the whole
        // paint pass. Every widget builder below reads `editor_state`
        // directly; the canvas reads `self.layout_scene`.
        self.refresh_layout_scene();
        if self.device_mode_active() && self.preview_device_frame.is_none() {
            // Paint owns authoritative viewport dimensions; enter-time
            // cached dimensions can still be zero on a fresh host.
            self.recompute_device_frame(viewport_width, viewport_height);
        }
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
            let layer_panel_rect =
                canvas_geometry::layer_panel_rect(&self.editor_state, viewport_height);
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
                // Per-frame paint: resolve the row model through the
                // owner-scoped cache so idle / streaming / hover repaints
                // that don't touch the layer tree skip the walk + measure.
                self.layer_panel()
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
            if self.preview.is_some() {
                // PREVIEW path — paint the canvas background, then the
                // live document rendered through the SAME design-canvas
                // scene painter (`paint_scene`), with widget runtime
                // state overlaid + a focus caret. Passing `layout_scene`
                // (the untouched design scene) makes preview
                // pixel-identical to design plus live. The editor's
                // selection / handles / grid do NOT paint in preview.
                frame.fill_rect(canvas_rect, self.theme.canvas_surface);
                if self.device_mode_active() {
                    self.paint_device_frame(&mut *frame, canvas_rect);
                } else if let Some(preview) = self.preview.as_ref() {
                    preview.paint_scene(
                        &mut *frame,
                        canvas_rect,
                        (
                            self.editor_state.viewport.pan_x,
                            self.editor_state.viewport.pan_y,
                        ),
                        self.editor_state.viewport.zoom,
                        self.now_ms,
                    );
                }
                self.paint_preview_switcher(&mut *frame, canvas_rect);
                self.paint_screen_switcher(&mut *frame, canvas_rect);
            } else if !self.serve_canvas_from_pan_cache(frame, canvas_rect) {
                // PAINT path — the canvas reads editor state + the
                // layout-resolved render scene (`refresh_layout_scene`).
                // During a live pan gesture the frame is rendered once
                // into an offscreen layer (grown by the pan-cache
                // margin) so following pure-pan frames blit it above.
                // Zoom ticks never build: the next tick's zoom change
                // would invalidate the freshly built layer anyway.
                let build_pan_cache = self.pan_cache_usable() && !self.last_gesture_was_zoom;
                let cached_layer = {
                    let mut transition_scene = None;
                    if let Some(transition) = self.layout_transition.as_ref() {
                        if transition.is_active(self.now_ms) {
                            let mut scene = self.layout_scene.clone();
                            transition.apply_to_scene(&mut scene, self.now_ms);
                            transition_scene = Some(scene);
                        }
                    }
                    let canvas_scene = transition_scene.as_ref().unwrap_or(&self.layout_scene);
                    let mut canvas = CanvasViewport::from_editor(&self.editor_state, canvas_scene);
                    canvas.now_ms = self.now_ms;
                    canvas.fast_interaction = self.fast_interaction_active();
                    canvas.set_node_drag_active(
                        self.node_drag.as_ref().is_some_and(|drag| drag.moved),
                    );
                    canvas.set_node_drag_overlay(self.node_drag_overlay_for_paint());
                    let mut rendered_layer = None;
                    if build_pan_cache {
                        use super::canvas_pan_cache::PAN_CACHE_MARGIN as M;
                        let expanded = Rect {
                            origin: Point2D::new(
                                canvas_rect.origin.x - M,
                                canvas_rect.origin.y - M,
                            ),
                            size: Point2D::new(
                                canvas_rect.size.x + M * 2.0,
                                canvas_rect.size.y + M * 2.0,
                            ),
                        };
                        // The +M pan offset cancels the -M rect shift so
                        // the doc↔logical mapping matches an on-window
                        // paint exactly.
                        canvas.offset_paint_origin(M, M);
                        rendered_layer = frame.render_offscreen_layer(canvas_rect, M, |off| {
                            let mut cx = PaintCx { backend: off };
                            canvas.paint(&mut cx, expanded);
                        });
                        if rendered_layer.is_none() {
                            // Offscreen allocation failed — restore the
                            // mapping and paint directly.
                            canvas.offset_paint_origin(-M, -M);
                        }
                    }
                    if rendered_layer.is_none() {
                        let mut cx = PaintCx {
                            backend: &mut *frame,
                        };
                        canvas.paint(&mut cx, canvas_rect);
                    }
                    rendered_layer
                };
                if let Some(mut surface) = cached_layer {
                    use super::canvas_pan_cache::PAN_CACHE_MARGIN;
                    frame.draw_offscreen_layer(
                        &surface.image_snapshot(),
                        canvas_rect,
                        PAN_CACHE_MARGIN,
                        Point2D::new(0.0, 0.0),
                    );
                    let raster_generation = frame.raster_generation();
                    self.store_pan_cache(surface, canvas_rect, dpi, raster_generation);
                }
            }
        }

        // Re-borrow after the canvas section: the pan-cache path above
        // needs `&mut self`, which must not overlap the earlier shared
        // `ui` borrow.
        let ui = &self.editor_state.editor_ui;

        // 5. PropertyPanel — only when selection.
        let property_panel = PropertyPanel::for_selection_at_with_scene(
            &self.editor_state,
            &self.layout_scene,
            self.now_ms,
        );
        let property_panel_width = ui.property_panel_width;
        let right_rail_x = viewport_width - property_panel_width;
        if let Some(panel) = property_panel.as_ref() {
            let property_rect = canvas_geometry::property_panel_rect(
                &self.editor_state,
                viewport_width,
                viewport_height,
            );
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

        // 5b-1. Theme-preset dropdown (#20) — painted after the panel
        //       so the functional menu covers the panel's static stub
        //       rows (variables_preset_press.rs owns the geometry).
        if let Some((preset_menu, preset_menu_rect)) =
            self.variables_preset_menu_with_rect(viewport_width, viewport_height)
        {
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            preset_menu.paint(&mut cx, preset_menu_rect);
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
        let toolbar_rect = canvas_geometry::toolbar_rect(&self.editor_state, toolbar_h);
        if canvas_geometry::toolbar_fits(canvas_w) {
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            toolbar.paint(&mut cx, toolbar_rect);
        }

        // 7. AIChatPlaceholder — painted LAST so it sits on top
        //    of the toolbar in any overlap region (matches the
        //    user's requested z-order: chat above toolbar).
        if let Some(chat_rect) = self.ai_chat_rect(viewport_width, viewport_height) {
            // Owner-stamp so paint stores the canonical build under THIS host's
            // owner — the display-frame cursor hint reads it back by that owner.
            let chat = AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                .owned_by(self.chat_panel_owner);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            chat.paint(&mut cx, chat_rect);
        }

        // 8. StatusBar — floating bottom-right.
        if let Some(status_rect) =
            canvas_geometry::status_bar_rect(&self.editor_state, viewport_width, viewport_height)
        {
            let status = StatusBar::for_editor(&self.editor_state);
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
        if self.preview.is_none() {
            if let Some(toolbar) =
                AlignToolbar::for_canvas_region(canvas_region, &self.editor_state)
            {
                let hover = self.editor_state.editor_ui.align_toolbar_hover;
                toolbar.paint(&mut *frame, &self.theme, hover);
            }
        }

        // 8.5. Marquee selection rect — painted above canvas but
        //      below the floating pickers / status. Visible only
        //      while the user is dragging a rect-select on empty
        //      canvas (Select tool). Never in preview mode.
        if let Some(rect) = self
            .marquee_drag
            .filter(|_| self.preview.is_none())
            .as_ref()
            .and_then(canvas_geometry::marquee_rect)
        {
            {
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
            self.image_input_geometry =
                panel.image_popover_input_geometry(property_rect, &mut *frame);
        }

        // 8.7. Floating Git panel — a popover hanging off the TopBar
        //      Git button (centred by `git_panel_rect`). Painted here —
        //      ABOVE the align toolbar / marquee / property overlays but
        //      BELOW the shape / locale pickers and modals — so its
        //      paint z-order matches its hit-test priority (press block
        //      0.9, ahead of chat / toolbar / canvas). When it floated
        //      top-left this was moot; centred under the button it now
        //      overlaps the align toolbar, so the orders must agree.
        if let Some(panel_rect) = self.git_panel_rect(viewport_width, viewport_height) {
            let panel = GitPanel::for_editor_at(&self.editor_state, self.now_ms)
                .expect("git_panel_rect is Some, so the panel is open");
            let git_theme = theme_for(&self.editor_state.editor_ui);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            panel.paint(&mut cx, panel_rect);

            // Up-caret connecting the panel to the Git button — drawn
            // after the panel so its base covers the panel's top border
            // and reads as one continuous popover surface.
            if let Some(btn_cx) = top_bar.git_button_center_x(top_bar_rect) {
                let half = GIT_PANEL_CARET_HALF;
                let top = panel_rect.origin.y;
                let caret_x = btn_cx.clamp(
                    panel_rect.origin.x + half + 2.0,
                    panel_rect.origin.x + panel_rect.size.x - half - 2.0,
                );
                let tip = Point2D::new(caret_x, top - GIT_PANEL_CARET_H);
                let base_l = Point2D::new(caret_x - half, top + 0.5);
                let base_r = Point2D::new(caret_x + half, top + 0.5);
                cx.backend
                    .fill_polygon(&[tip, base_l, base_r], git_theme.popover);
                cx.backend.stroke_line(tip, base_l, git_theme.border, 1.0);
                cx.backend.stroke_line(tip, base_r, git_theme.border, 1.0);
            }
        }

        // 9. ShapePicker — anchored to the right of the toolbar
        //    shape slot; same z-priority as the locale picker.
        if ui.shape_picker.open {
            let picker_rect = self.shape_picker_rect(viewport_width, viewport_height);
            let picker = ShapePicker::for_editor_ui(&self.editor_state.editor_ui);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            picker.paint(&mut cx, picker_rect);
        }

        // 9z. Import dropdown — same overlay tier as the locale picker.
        if ui.import_menu_open {
            use op_editor_ui::widgets::ImportMenu;
            let (anchor, menu_viewport) = self.import_menu_anchor(viewport_width, viewport_height);
            let menu = ImportMenu::for_editor_ui(&self.editor_state.editor_ui);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            menu.paint_select(&mut cx, anchor, menu_viewport);
        }

        // 10. LocalePicker — top-most overlay so it covers chat /
        //     toolbar / status when open.
        if ui.locale_picker.open {
            let picker_rect = self.locale_picker_rect(viewport_width);
            let picker = LocalePicker::for_editor_ui(&self.editor_state.editor_ui);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            picker.paint(&mut cx, picker_rect);
        }

        // Collaboration popover — a real shared widget anchored to the
        // collaboration status chip. It consumes only sanitized UI state;
        // the native session actor drains queued actions separately.
        if let Some(panel) =
            op_editor_ui::widgets::CollabPanel::for_editor_ui(&self.editor_state.editor_ui)
        {
            let anchor = top_bar.collaboration_chip_rect_estimated(top_bar_rect);
            let panel_rect = panel.rect_at(
                anchor,
                Rect::xywh(0.0, 0.0, viewport_width, viewport_height),
            );
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            panel.paint(&mut cx, panel_rect);
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
            // Selected-frame count only names the batch-export row; it
            // leaves row geometry untouched, so the hit-test call sites
            // keep building the menu without it.
            let selected_frames =
                op_editor_core::export_batch::selected_frame_count(&self.editor_state);
            let menu = FileMenu::from_editor_ui(&self.editor_state.editor_ui, now_secs)
                .with_selected_frames(selected_frames);
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

        // 10e. Sign-in modal — full-viewport scrim + centred card.
        if ui.account_ui_available && ui.login_modal_open {
            use op_editor_ui::widgets::login_modal::LoginModal;
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
            let modal = LoginModal::for_editor(&self.editor_state);
            let modal_rect = modal.rect(viewport_width, viewport_height);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            modal.paint(&mut cx, modal_rect);
        }

        // 10f. Signed-in account dropdown — anchored under the TopBar
        //      avatar button, no scrim (same tier as the file menu /
        //      locale picker).
        if ui.account_ui_available && ui.account_menu_open {
            use op_editor_ui::widgets::account_menu::AccountMenu;
            let top_bar_rect = Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(viewport_width, TOP_BAR_HEIGHT),
            };
            let top_bar = TopBar::for_editor_ui(&self.editor_state.editor_ui);
            let anchor = top_bar.account_button_rect(top_bar_rect);
            if let Some(menu) = AccountMenu::for_editor_ui(&self.editor_state.editor_ui) {
                let menu_rect = menu.rect_at(anchor);
                let mut cx = PaintCx {
                    backend: &mut *frame,
                };
                menu.paint(&mut cx, menu_rect);
            }
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
            let panel = AgentSettingsPanel::for_editor_at(&self.editor_state, self.now_ms);
            let panel_rect = panel.rect(viewport_width, viewport_height);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            panel.paint(&mut cx, panel_rect);
        }

        // 10b. Color picker — floating overlay near the right rail.
        if let Some(state) = self.editor_state.ui.color_picker.clone() {
            use op_editor_ui::widgets::color_picker::ColorPicker;
            let picker = ColorPicker::for_state_at(&self.editor_state, state, self.now_ms);
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

        // 11a. Path-anchor context menu — Select-tool right-click on
        //      a path anchor / handle (TS `PathAnchorContextMenu`).
        if let Some(state) = self.editor_state.ui.path_anchor_menu.clone() {
            use op_editor_ui::widgets::path_anchor_context_menu::PathAnchorContextMenu;
            let menu = PathAnchorContextMenu::for_state(&self.editor_state, state);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            menu.paint(&mut cx);
        }

        // 11.5. Floating Component-Browser panel — the UIKit library
        //       browser, toggled from the View menu. Painted just
        //       below the Design-MD panel so when both are open the
        //       Design-MD panel sits absolute-top.
        if let (Some(panel), Some(panel_rect)) = (
            ComponentBrowserPanel::for_editor_at(&self.editor_state, self.now_ms),
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

        // 13. File-drop overlay — top-most layer, above every panel and
        //     modal, while a file is dragged over the window.
        if self.editor_state.editor_ui.file_drop_active {
            let (drop_left, _y, drop_w, drop_h) =
                self.canvas_region(viewport_width, viewport_height);
            let drop_rect = Rect {
                origin: Point2D::new(drop_left, TOP_BAR_HEIGHT),
                size: Point2D::new(drop_w, drop_h),
            };
            let target = self
                .editor_state
                .editor_ui
                .file_drop_target
                .clone()
                .and_then(|id| self.node_screen_rect(&id, viewport_width, viewport_height));
            op_editor_ui::widgets::file_drop_overlay::paint_file_drop_overlay(
                &mut *frame,
                &self.theme,
                self.editor_state.editor_ui.locale,
                drop_rect,
                target,
            );
        }

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
