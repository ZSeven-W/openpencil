//! Editor-UI composition paint pass for the web `WidgetHost`.
//! Pulled out of `widget_host.rs` to keep that file under the
//! 800-line ceiling. Mirrors the structure used by
//! `op-host-native/src/widget_host/paint.rs`.
//!
//! `paint` takes `&mut self`: it rebuilds the layout-resolved
//! `LayoutScene` (`refresh_layout_scene`) at the top of the pass,
//! then every widget builder reads `editor_state` directly and the
//! canvas reads the render scene.

use super::WidgetHost;
use op_editor_ui::widgets::host_canvas_geometry as canvas_geometry;
use op_editor_ui::widgets::variables_panel::VariablesPanel;
use op_editor_ui::widgets::{
    AIChatPlaceholder, CanvasViewport, LayerPanel, LayoutCx, LocalePicker, PaintCx, PropertyPanel,
    ShapePicker, StatusBar, Toolbar, Widget, TOOLBAR_WIDTH, TOP_BAR_HEIGHT,
};
use op_editor_ui::{Point2D, Rect, RenderBackend};

impl WidgetHost {
    /// Backend-generic public paint entry (used by the CanvasKit host).
    /// Delegates to the same composition pass.
    pub fn paint_dyn(
        &mut self,
        backend: &mut dyn RenderBackend,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        self.paint_editor(backend, viewport_width, viewport_height);
    }

    /// Backend-generic composition pass. Layer order matches the
    /// native shell so paint output is cross-platform identical:
    ///   1. background fill (+ document-import progress early-return)
    ///   2. TopBar
    ///   3. LayerPanel (left rail, sidebar-gated)
    ///   4. CanvasViewport (center band)
    ///   5. PropertyPanel (right rail, selection-gated) + variables rail
    ///   6. Toolbar (floating column)
    ///   7. AIChatPlaceholder (floating, painted late so it sits
    ///      on top of toolbar)
    ///   8. StatusBar / AlignToolbar / marquee / property overlays
    ///   9. ShapePicker / LocalePicker / FileMenu dropdowns
    ///  10. FigmaImport + Export + Variables + AgentSettings modals
    ///  11. ColorPicker + LayerContextMenu
    ///  12. ComponentBrowser / IconPicker / DesignMd floating panels
    ///  13. file-drop overlay (top-most)
    // glue:
    pub(in crate::widget_host) fn paint_editor(
        &mut self,
        backend: &mut dyn RenderBackend,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        self.image_input_geometry = None;
        // Rotate the transcript-cache owner if the active chat session changed,
        // BEFORE any resolve stores under it (mirrors native paint).
        self.rotate_chat_owner_if_session_changed();
        self.sync_theme_from_editor();
        // Pump the preview router BEFORE painting, mirroring the native
        // host: `navigate_to_screen` only queues `router.replace(path)`,
        // and `reconcile` is what actually swaps the mounted screen. Doing
        // it here means a switched screen paints this same frame instead of
        // one frame late — and without it a pill tap looks like a no-op.
        #[cfg(feature = "canvaskit")]
        if self.preview.is_some() {
            // The viewport arrives as a paint parameter on this host, so a
            // resize has no event of its own. Rebuild the cached device
            // frame when it changes — otherwise the frame keeps the `fit`
            // it was solved with and the content paints squashed against a
            // canvas region that is no longer that size.
            let vp = (viewport_width, viewport_height);
            if self.preview_frame_viewport != Some(vp) {
                self.preview_frame_viewport = Some(vp);
                // Keep the hit-test's cached viewport in step: hover and
                // switcher hit-tests read these and would otherwise stay on
                // the pre-resize geometry until the next press.
                self.last_viewport_w = viewport_width;
                self.last_viewport_h = viewport_height;
                let (_cx, _cy, cw, ch) = self.canvas_region(viewport_width, viewport_height);
                if let Some(preview) = self.preview.as_mut() {
                    preview.resize((cw, ch));
                }
                self.recompute_device_frame(viewport_width, viewport_height);
            }
        }
        #[cfg(feature = "canvaskit")]
        {
            // Track M-1: finalize a finished canvas ↔ device-frame merge
            // animation BEFORE anything below reads `self.preview` /
            // `device_mode_active()` this frame — an Exit whose 220ms
            // window just elapsed drops the runtime here, so the rest of
            // this pass sees the settled (real) mode immediately rather
            // than one stale frame late.
            self.settle_mode_transition();

            let mut switched = false;
            if let Some(preview) = self.preview.as_mut() {
                // Advance the session clock FIRST. `dispatch_pointer_phase`
                // has no clock parameter of its own — it reads the value last
                // handed to `set_now_ms` to decide whether a screen
                // transition is still running, and suppresses input while one
                // is. Never advancing it freezes the session at the first
                // transition, so every tap after the first screen switch is
                // silently dropped.
                preview.set_now_ms(self.now_ms);
                let outcome = preview.reconcile(self.now_ms);
                if outcome.repaint {
                    let warnings = preview.warnings().to_vec();
                    self.editor_state.editor_ui.preview.warnings = warnings;
                }
                switched = outcome.switched;
            }
            if switched {
                if self.device_mode_active() {
                    // The new screen has its own root, nav strip and scroll
                    // extent, so the cached frame is stale.
                    self.on_preview_screen_switched(viewport_width, viewport_height);
                } else {
                    self.center_canvas_on_preview_root(viewport_width, viewport_height);
                }
            }
        }
        backend.fill_rect(
            Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(viewport_width, viewport_height),
            },
            self.theme.background,
        );

        let dpi = backend.dpi_scale();

        // During a document import, keep the frame path independent from
        // document layout/canvas paint (mirrors native — the parser is
        // CPU-heavy and repainting the old scene reads as frozen).
        if self.editor_state.editor_ui.figma_import_in_progress {
            use op_editor_ui::widgets::figma_import_progress::ImportProgressOverlay;
            backend.fill_rect(
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
                backend: &mut *backend,
            };
            overlay.paint(&mut cx, rect);
            return;
        }

        // Rebuild the layout-resolved render scene ONCE for the whole
        // paint pass. Every widget builder below reads `editor_state`
        // directly; the canvas reads `self.layout_scene`.
        self.refresh_layout_scene();
        // The rail's slides tab, resolved ahead of the long immutable
        // borrow below because deriving it needs `&mut self` (mirrors
        // native paint.rs), and it OWNS the rail when it is on show.
        let rail_open = self.editor_state.editor_ui.sidebar_open;
        let slides_panel = if rail_open {
            self.slides_panel_frame(viewport_width, viewport_height)
        } else {
            None
        };

        // Frame the slideshow board BEFORE the canvas painting so the camera
        // is ready.
        #[cfg(feature = "canvaskit")]
        if self.preview_slideshow_active() {
            self.frame_slideshow_board((viewport_width, viewport_height));
        }

        let top_bar = self.top_bar();
        let top_bar_rect = self.top_bar_rect(viewport_width);
        {
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            top_bar.paint(&mut cx, top_bar_rect);
        }

        // 3a. Slides tab — the same rail showing the deck's page
        //     navigator instead of the layer tree.
        if let Some(slides) = &slides_panel {
            self.paint_slides_panel(&mut *backend, slides);
        }
        if rail_open && slides_panel.is_none() {
            let layer_panel_rect = self.layer_panel_rect(viewport_height);
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
                // Per-frame paint: resolve the row model through the
                // owner-scoped cache so idle / streaming / hover repaints
                // that don't touch the layer tree skip the walk + measure.
                self.layer_panel()
            };

            // Auto-reveal selected node: if the selection changed and differs
            // from the last-revealed anchor, expand ancestors and reveal.
            // This covers MCP set_selection, undo/redo, and programmatic
            // selection changes (not just canvas clicks).
            if active_drag.is_none() {
                // Only auto-reveal when not dragging; explicit drag interactions
                // take precedence and manual collapse should be respected.
                let should_reveal = match (
                    &self.editor_state.selection.anchor,
                    &self.editor_state.editor_ui.last_revealed_layer_anchor,
                ) {
                    (anchor, last) if anchor.is_real() => Some(anchor) != last.as_ref(),
                    _ => false,
                };
                if should_reveal {
                    op_editor_ui::widgets::scroll_flow::reveal_layer_panel_selection(
                        &mut self.editor_state,
                        &layer_panel,
                        layer_panel_rect,
                    );
                    // Rebuild the panel after reveal to reflect any expanded ancestors.
                    layer_panel = self.layer_panel();
                }
            }

            if let Some(d) = &active_drag {
                layer_panel.drop_target = layer_panel
                    .drop_target_at(layer_panel_rect, Point2D::new(d.current_x, d.current_y));
                if let Some(item) = LayerPanel::ghost_item_for(&self.editor_state, &d.source) {
                    layer_panel.drag_ghost = Some((item, d.current_y));
                }
            }
            layer_panel.now_ms = self.now_ms;
            {
                let mut cx = PaintCx {
                    backend: &mut *backend,
                };
                layer_panel.paint(&mut cx, layer_panel_rect);
            }
            // The tab row heads the rail in BOTH tabs — it is how the
            // user gets back to the slides.
            if let Some(tabs) = self.slides_tab_row(viewport_height) {
                self.paint_slides_tab_row(&mut *backend, &tabs);
            }
        }

        let (canvas_left, _canvas_y, canvas_w, canvas_h) =
            self.canvas_region(viewport_width, viewport_height);
        let canvas_rect = Rect {
            origin: Point2D::new(canvas_left, TOP_BAR_HEIGHT),
            size: Point2D::new(canvas_w, canvas_h),
        };
        if canvas_w > 0.0 && canvas_h > 0.0 {
            // Check if preview mode is active and we have a session
            #[cfg(feature = "canvaskit")]
            if self.editor_state.editor_ui.preview.mode && self.preview.is_some() {
                // PREVIEW PAINT path. Clear first: neither presentation
                // necessarily covers the whole canvas rect, and the previous
                // frame's design content must not show through around it.
                backend.fill_rect(canvas_rect, self.theme.canvas_surface);

                // Slideshow presentation paints the board letterboxed in the full viewport.
                if self.preview_slideshow_active() {
                    let viewport_rect = Rect {
                        origin: Point2D::new(0.0, 0.0),
                        size: Point2D::new(viewport_width, viewport_height),
                    };
                    self.paint_slideshow(backend, viewport_rect);
                } else if self.device_mode_active() {
                    // Phone / Desktop — present inside the device silhouette,
                    // with the screen's bottom nav and top status bar pinned
                    // out of the scroll flow.
                    self.paint_device_frame(backend, canvas_rect);
                } else if let Some(session) = self.preview.as_ref() {
                    // Canvas segment — the raw scene at the editor's own
                    // pan/zoom. Painting at a hard-coded identity transform
                    // would draw the document at its own origin instead of
                    // where the user is looking.
                    let viewport = self.editor_state.viewport;
                    session.paint_scene(
                        backend,
                        canvas_rect,
                        (viewport.pan_x, viewport.pan_y),
                        viewport.zoom,
                        self.now_ms,
                    );
                }
                // Chrome paints over the presentation in every segment.
                self.paint_preview_switcher(backend, canvas_rect);
                self.paint_screen_switcher(backend, canvas_rect);
            } else {
                // NORMAL PAINT path — the canvas reads editor state + the
                // layout-resolved render scene (`refresh_layout_scene`).
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
                canvas.set_node_drag_active(self.node_drag.as_ref().is_some_and(|drag| drag.moved));
                canvas.set_node_drag_overlay(self.node_drag_overlay_for_paint());
                let mut cx = PaintCx {
                    backend: &mut *backend,
                };
                canvas.paint(&mut cx, canvas_rect);
            }

            #[cfg(not(feature = "canvaskit"))]
            {
                // NORMAL PAINT path (non-canvaskit build)
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
                canvas.set_node_drag_active(self.node_drag.as_ref().is_some_and(|drag| drag.moved));
                canvas.set_node_drag_overlay(self.node_drag_overlay_for_paint());
                let mut cx = PaintCx {
                    backend: &mut *backend,
                };
                canvas.paint(&mut cx, canvas_rect);
            }
        }

        let property_panel = PropertyPanel::for_selection_at_with_scene(
            &self.editor_state,
            &self.layout_scene,
            self.now_ms,
        );
        if let Some(panel) = property_panel.as_ref() {
            let property_rect = canvas_geometry::property_panel_rect(
                &self.editor_state,
                viewport_width,
                viewport_height,
            );
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            panel.paint(&mut cx, property_rect);
        }

        // 5b. VariablesPanel — mirrors TS' `{}` toolbar toggle as a
        //     floating canvas overlay next to the toolbar (#21: same
        //     interactive grid as the native host; the old read-only
        //     right-rail copy is gone).
        if let Some(vars_rect) = self.variables_panel_rect(viewport_width, viewport_height) {
            let vars = VariablesPanel::for_editor_at(&self.editor_state, self.now_ms);
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            vars.paint(&mut cx, vars_rect);
        }

        // 5b-1. Theme-preset dropdown (#20) — painted after the panel so the
        //       functional menu covers the panel's static stub rows
        //       (variables_preset_press.rs owns the geometry).
        if let Some((preset_menu, preset_menu_rect)) =
            self.variables_preset_menu_with_rect(viewport_width, viewport_height)
        {
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            preset_menu.paint(&mut cx, preset_menu_rect);
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
        let toolbar_rect = canvas_geometry::toolbar_rect(&self.editor_state, toolbar_h);
        if canvas_geometry::toolbar_fits(canvas_w) {
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            toolbar.paint(&mut cx, toolbar_rect);
        }

        if let Some(chat_rect) = self.ai_chat_rect(viewport_width, viewport_height) {
            // Owner-stamp so paint stores the canonical build under THIS host's
            // owner (mirrors native).
            let chat = AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                .owned_by(self.chat_panel_owner);
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            chat.paint(&mut cx, chat_rect);
        }

        if let Some(status_rect) =
            canvas_geometry::status_bar_rect(&self.editor_state, viewport_width, viewport_height)
        {
            let status = StatusBar::for_editor(&self.editor_state);
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
        if let Some(rect) = self
            .marquee_drag
            .as_ref()
            .and_then(canvas_geometry::marquee_rect)
        {
            let primary = self.theme.primary;
            // 12% primary-tinted fill so the rect reads as a selection
            // band without obscuring the canvas.
            let fill = op_editor_ui::Color {
                r: primary.r,
                g: primary.g,
                b: primary.b,
                a: primary.a * 0.12,
            };
            backend.fill_rect(rect, fill);
            backend.stroke_rect(rect, primary, 1.0);
        }

        // PropertyPanel overlays — painted after canvas floating
        // controls so the image-fill popover can cover the zoom
        // status pill when it extends into the canvas.
        if let Some(panel) = property_panel.as_ref() {
            let property_rect = canvas_geometry::property_panel_rect(
                &self.editor_state,
                viewport_width,
                viewport_height,
            );
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            panel.paint_overlays(&mut cx, property_rect);
            self.image_input_geometry =
                panel.image_popover_input_geometry(property_rect, &mut *backend);
        }

        // TopBar hover tooltip — hangs off a chrome button into whatever
        // is under the bar, so it paints after the rails and canvas but
        // below every dropdown and modal (native §8.65).
        {
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            op_editor_ui::widgets::top_bar_tooltip::paint_top_bar_tooltip(
                &mut cx,
                &self.editor_state.editor_ui,
                &top_bar,
                top_bar_rect,
                viewport_width,
                self.now_ms,
            );
        }

        // Bound here rather than at the top of the pass: the rail section
        // above needs `&mut self.editor_state` to reveal the selection, and a
        // chrome-wide immutable borrow would outlive it.
        let ui = &self.editor_state.editor_ui;

        // ShapePicker — anchored to the right of the toolbar shape
        // slot; same z-priority as the locale picker (native §9).
        if ui.shape_picker.open {
            let picker_rect = self.shape_picker_rect(viewport_width, viewport_height);
            let picker = ShapePicker::for_editor_ui(&self.editor_state.editor_ui);
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            picker.paint(&mut cx, picker_rect);
        }

        if ui.import_menu_open {
            use op_editor_ui::widgets::ImportMenu;
            let (anchor, menu_viewport) = self.import_menu_anchor(viewport_width, viewport_height);
            let menu = ImportMenu::for_editor_ui(&self.editor_state.editor_ui);
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            menu.paint_select(&mut cx, anchor, menu_viewport);
        }

        if ui.locale_picker.open {
            let picker_rect = self.locale_picker_rect(viewport_width);
            let picker = LocalePicker::for_editor_ui(&self.editor_state.editor_ui);
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            picker.paint(&mut cx, picker_rect);
        }

        // Shared collaboration popover. Web builds normally keep the
        // capability unavailable for M1, but the real surface is present for
        // future satellite hosts and never reaches native transport APIs.
        if let Some(panel) = op_editor_ui::widgets::CollabPanel::for_editor_ui_at(
            &self.editor_state.editor_ui,
            self.now_ms,
        ) {
            let top_bar =
                op_editor_ui::widgets::TopBar::for_editor_ui(&self.editor_state.editor_ui)
                    .with_traffic_controls(false);
            let anchor = top_bar.collaboration_chip_rect_estimated(top_bar_rect);
            let panel_rect = panel.rect_at(
                anchor,
                Rect::xywh(0.0, 0.0, viewport_width, viewport_height),
            );
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            panel.paint(&mut cx, panel_rect);
        }

        // Menus, modals, floating panels and the top-most notices —
        // z-order above everything painted so far. Split into its own
        // method purely to keep this file under the repo's 800-line cap;
        // the call sits exactly where the moved block used to run.
        self.paint_menu_modal_and_panel_layers(&mut *backend, viewport_width, viewport_height);

        // Keep the frame loop alive while EITHER preview animation plays.
        // This host repaints only on request, so an animation that stops
        // asking for frames freezes mid-way and then jumps whenever some
        // unrelated event (a sync poll) happens to repaint.
        //
        // Both must be checked: `preview_mode_transition` is the canvas <->
        // device-frame merge on enter/exit, while the app-mode screen slide
        // lives inside the session and is what a nav tap starts. Checking
        // only the former is why switching screens rendered as one or two
        // discrete jumps, leaving a half-composited nav strip on screen
        // until the next poll repainted over it.
        #[cfg(feature = "canvaskit")]
        {
            let mode_animating = self
                .preview_mode_transition
                .as_ref()
                .is_some_and(|t| t.is_active(self.now_ms));
            let screen_animating = self
                .preview
                .as_ref()
                .is_some_and(|session| session.transition_active());
            if mode_animating || screen_animating {
                crate::repaint_coalescer::request();
            }
        }
    }
}
