//! Editor-UI composition paint pass on `WidgetHostNative`.
//! Pulled out of `widget_host.rs` to keep the spine file under
//! the 800-line ceiling.

use super::helpers::{GIT_PANEL_CARET_H, GIT_PANEL_CARET_HALF};
use super::WidgetHostNative;
use crate::backend::NativeFrameBackend;
use op_editor_ui::widgets::editor_state_ext::theme_for;
use op_editor_ui::widgets::host_canvas_geometry as canvas_geometry;
use op_editor_ui::widgets::{
    variables_panel::VariablesPanel, AIChatPlaceholder, AlignToolbar, CanvasViewport, GitPanel,
    LayoutCx, LocalePicker, PaintCx, PropertyPanel, ShapePicker, StatusBar, Toolbar, TopBar,
    Widget, TOOLBAR_WIDTH, TOP_BAR_HEIGHT,
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
        // Presenting a deck hides the EDITING chrome — rails, tool column,
        // chat, status bar. It is paint-side policy only: no panel state is
        // touched, so leaving the presentation restores exactly the layout
        // the user had without anything having to remember it. The TopBar
        // stays: it carries the preview toggle, so there is always a visible
        // way out besides Esc and the toolbar's own exit.
        let presenting = self.preview_slideshow_active();
        if self.device_mode_active() && self.preview_device_frame.is_none() {
            // Paint owns authoritative viewport dimensions; enter-time
            // cached dimensions can still be zero on a fresh host.
            self.recompute_device_frame(viewport_width, viewport_height);
        }
        if self.preview_slideshow_active() {
            // Same reason: the presented board is framed against the stage
            // paint actually has, so a slide advanced by a key press or a
            // window resized mid-presentation both land framed.
            let stage = self.preview_canvas_rect(viewport_width, viewport_height);
            self.frame_slideshow_board((stage.size.x, stage.size.y));
        }
        // 2. TopBar — painted only on the desktop; mobile layout replaces
        //    it with the floating action cluster below.
        let top_bar = TopBar::for_editor_ui(&self.editor_state.editor_ui);
        let top_bar_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_width, TOP_BAR_HEIGHT),
        };
        if !self.editor_state.editor_ui.touch_chrome() {
            {
                let mut cx = PaintCx {
                    backend: &mut *frame,
                };
                top_bar.paint(&mut cx, top_bar_rect);
            }
        }

        // 3. The left rail — painted BEFORE the canvas on the desktop
        //    (it pushes the canvas) and AFTER it in mobile layout (it
        //    overlays the canvas).
        let persistent_layers = !self.editor_state.editor_ui.touch_chrome()
            || (self.editor_state.editor_ui.expanded_touch_layout()
                && self.editor_state.editor_ui.sidebar_open);
        if persistent_layers {
            self.paint_left_rail(frame, viewport_width, viewport_height);
        }

        // 4. CanvasViewport — middle band, respects sidebar
        //    collapse state. It paints before the right rail so
        //    PropertyPanel popovers can extend into the canvas.
        let canvas_rect =
            canvas_geometry::canvas_rect(&self.editor_state, viewport_width, viewport_height);
        let canvas_w = canvas_rect.size.x;
        let canvas_h = canvas_rect.size.y;
        if canvas_w > 0.0 && canvas_h > 0.0 {
            if self.preview.is_some() {
                // PREVIEW path — paint the canvas background, then the
                // live document rendered through the SAME design-canvas
                // scene painter (`paint_scene`), with widget runtime
                // state overlaid + a focus caret. Passing `layout_scene`
                // (the untouched design scene) makes preview
                // pixel-identical to design plus live. The editor's
                // selection / handles / grid do NOT paint in preview.
                if presenting {
                    // A deck is PRESENTED: one board, letterboxed, no device
                    // silhouette and no switchers — neither a phone bezel nor
                    // a screen router says anything about a slide. The stage
                    // is the full width under the TopBar, since the rails
                    // that would have bounded it are not painted.
                    let stage = self.preview_canvas_rect(viewport_width, viewport_height);
                    frame.fill_rect(stage, self.theme.canvas_surface);
                    self.paint_slideshow(&mut *frame, stage);
                } else {
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
                }
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

        self.paint_mobile_sheet_scrim(frame, viewport_width, viewport_height);

        // Touch overlay: Layers paints after the canvas. Expanded's
        // persistent rail already painted before it and pushed the canvas.
        if self.editor_state.editor_ui.touch_chrome()
            && self.editor_state.editor_ui.mobile_sheet
                == Some(op_editor_core::size_class::MobileSheetKind::Layers)
        {
            self.paint_mobile_layers_sheet(frame, viewport_width, viewport_height);
        }

        // 5. PropertyPanel — only when selection.
        let property_panel = PropertyPanel::for_selection_at_with_scene(
            &self.editor_state,
            &self.layout_scene,
            self.now_ms,
        );
        let touch_layout = self.editor_state.editor_ui.touch_chrome();
        let properties_open = !touch_layout
            || self.editor_state.editor_ui.expanded_touch_layout()
            || self.editor_state.editor_ui.mobile_sheet
                == Some(op_editor_core::size_class::MobileSheetKind::Properties);
        if let Some(panel) = property_panel
            .as_ref()
            .filter(|_| !presenting && properties_open)
        {
            let property_rect = canvas_geometry::property_panel_rect(
                &self.editor_state,
                viewport_width,
                viewport_height,
            );
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            if self.editor_state.editor_ui.compact_layout()
                || self.editor_state.editor_ui.medium_layout()
            {
                crate::widget_host::paint_mobile::paint_property_sheet(
                    &self.editor_state,
                    panel,
                    &mut cx,
                    property_rect,
                );
            } else {
                panel.paint(&mut cx, property_rect);
            }
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
        let touch_layout = self.editor_state.editor_ui.touch_chrome();
        if canvas_geometry::toolbar_fits(canvas_w) && !presenting && !touch_layout {
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            toolbar.paint(&mut cx, toolbar_rect);
        }

        // 7. AIChatPlaceholder — painted LAST so it sits on top
        //    of the toolbar in any overlap region (matches the
        //    user's requested z-order: chat above toolbar).
        let chat_open = !touch_layout
            || self.editor_state.editor_ui.mobile_sheet
                == Some(op_editor_core::size_class::MobileSheetKind::Ai);
        if let Some(chat_rect) = self
            .ai_chat_rect(viewport_width, viewport_height)
            .filter(|_| !presenting && chat_open)
        {
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
                .filter(|_| !presenting && !touch_layout)
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
        let canvas_region = canvas_rect;
        if self.preview.is_none()
            && !touch_layout
            && self.editor_state.editor_ui.mobile_sheet.is_none()
        {
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
        let properties_open = !touch_layout
            || self.editor_state.editor_ui.expanded_touch_layout()
            || self.editor_state.editor_ui.mobile_sheet
                == Some(op_editor_core::size_class::MobileSheetKind::Properties);
        if let Some(panel) = property_panel
            .as_ref()
            .filter(|_| !presenting && properties_open)
        {
            let property_rect = canvas_geometry::property_panel_rect(
                &self.editor_state,
                viewport_width,
                viewport_height,
            );
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            panel.paint_overlays(&mut cx, property_rect);
            self.image_input_geometry =
                panel.image_popover_input_geometry(property_rect, &mut *frame);
        }

        // Touch chrome sits above the editor canvas and rails, but below
        // pickers, dialogs, settings, and diagnostics painted afterwards.
        self.paint_mobile_chrome(frame, viewport_width, viewport_height);

        // 8.65. TopBar hover tooltip — over the rails, under every menu.
        self.paint_top_bar_tooltip_overlay(frame, &top_bar, top_bar_rect, viewport_width);

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
            if let Some(btn_cx) = self.topbar_git_button_center_x(&top_bar, top_bar_rect) {
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

        let ui = &self.editor_state.editor_ui;

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
        if let Some(panel) = op_editor_ui::widgets::CollabPanel::for_editor_ui_at(
            &self.editor_state.editor_ui,
            self.now_ms,
        ) {
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

        // 10b. TopBar dropdowns — the File menu and the export quick menu
        //      (bodies in `paint_chrome_menus.rs`). Only one is open at a
        //      time; the file menu paints first so a stale flag can never
        //      hide the shortcut behind it.
        if ui.file_menu_open {
            self.paint_file_menu_overlay(frame, viewport_width);
        }
        if ui.export_quick_menu_open {
            self.paint_export_quick_menu_overlay(frame, viewport_width);
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

        self.paint_floating_panels(frame, viewport_width, viewport_height);

        // 13. File-drop overlay — top-most layer, above every panel and
        //     modal, while a file is dragged over the window.
        if self.editor_state.editor_ui.file_drop_active {
            let drop_rect =
                canvas_geometry::canvas_rect(&self.editor_state, viewport_width, viewport_height);
            let target = self
                .editor_state
                .editor_ui
                .file_drop_target
                .clone()
                .and_then(|id| self.node_screen_rect(&id, viewport_width, viewport_height));
            op_editor_ui::widgets::file_drop_overlay::paint_file_drop_overlay(
                &mut *frame,
                &self.theme,
                self.editor_state.editor_ui.effective_locale(),
                drop_rect,
                target,
            );
        }

        // Top-most overlay band — the diagnostics notice, the toast banner and
        // the missing-font modal, in that z-order. Split into a sibling at the
        // 800-line cap; pure code motion.
        self.paint_topmost_overlays(frame, viewport_width, viewport_height);
    }
}
