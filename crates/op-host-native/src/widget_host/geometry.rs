//! Geometry + cursor-affordance methods on `WidgetHostNative`.
//! Pure-math helpers that map (viewport_w, viewport_h, x, y) into
//! the rects + cursor hints the host serves.
//!
//! Scalar / chrome reads go straight to `editor_state`; node-tree
//! hit-tests run against the layout-resolved `LayoutScene`. The
//! input-dispatch contract keeps `layout_scene` fresh before any
//! hit-testing input event (see `widget_host.rs`).

use super::helpers::PANEL_RESIZE_GUTTER;
use super::{PanelResizeKind, WidgetHostNative};
use op_editor_ui::widgets::host_canvas_geometry as canvas_geometry;
use op_editor_ui::widgets::host_overlay_geometry as overlay_geometry;
use op_editor_ui::widgets::{AIChatPlaceholder, ChatResizeEdge, TOP_BAR_HEIGHT};
use op_editor_ui::{Point2D, Rect};

impl WidgetHostNative {
    /// Hit-test which screen region the cursor is over. Used by
    /// the wheel + drag handlers so wheel-zoom + Hand-pan only
    /// fire when the cursor is over the canvas (not over a panel).
    pub(in crate::widget_host) fn over_canvas(
        &self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        canvas_geometry::over_canvas(&self.editor_state, x, y, viewport_w, viewport_h)
    }

    /// True when the cursor is over either resize gutter — used by
    /// the runner to set `CursorIcon::EwResize`. None = no gutter.
    pub fn panel_resize_hover(&self, x: f32, y: f32, viewport_w: f32) -> Option<PanelResizeKind> {
        if self.editor_state.editor_ui.touch_chrome() {
            return None;
        }
        if y < TOP_BAR_HEIGHT {
            return None;
        }
        if self.editor_state.editor_ui.sidebar_open {
            let edge = self.editor_state.editor_ui.layer_panel_width;
            if (x - edge).abs() <= PANEL_RESIZE_GUTTER {
                return Some(PanelResizeKind::LayerRight);
            }
        }
        if self.editor_state.property_panel_visible() {
            let edge = viewport_w - self.editor_state.editor_ui.property_panel_width;
            if (x - edge).abs() <= PANEL_RESIZE_GUTTER {
                return Some(PanelResizeKind::PropertyLeft);
            }
        }
        None
    }

    /// Whether a panel resize is in progress. Runner uses this to
    /// keep the resize cursor active even when the cursor briefly
    /// leaves the gutter mid-drag.
    pub fn is_resizing_panel(&self) -> bool {
        self.panel_resize.is_some()
    }

    pub fn chat_resize_hover(
        &self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<ChatResizeEdge> {
        let rect = self.ai_chat_rect(viewport_w, viewport_h)?;
        let panel = AIChatPlaceholder::from_editor(&self.editor_state);
        panel.resize_edge_at(rect, Point2D::new(x, y))
    }

    /// Whether the ordinary Chat panel owns a screen point. This mirrors the
    /// non-model-picker surface covered by `AIChatPlaceholder::cursor_probe`:
    /// every point inside the painted card plus its invisible resize gutter.
    /// Keep this probe transcript-free so LayerPanel/Variables pre-dispatch can
    /// respect Chat's z-order without hashing the chat log a second time.
    pub(in crate::widget_host) fn chat_panel_surface_contains(
        &self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        let Some(rect) = self.ai_chat_rect(viewport_w, viewport_h) else {
            return false;
        };
        let point = Point2D::new(x, y);
        if rect.contains(point) {
            return true;
        }
        AIChatPlaceholder::from_editor(&self.editor_state)
            .resize_edge_at(rect, point)
            .is_some()
    }

    /// Clear every lower-overlay hover highlight — file menu, locale
    /// / shape pickers, align toolbar, layer-context menu. Called
    /// when the cursor moves over the top-most Design-MD panel so a
    /// highlight set just before does not linger beneath it. Returns
    /// `true` if anything changed.
    pub(in crate::widget_host) fn clear_lower_overlay_hover(&mut self) -> bool {
        self.clear_lower_overlay_hover_impl(true)
    }

    /// Clear hover feedback painted below the collaboration popover while
    /// preserving the popover control resolved for this cursor event.
    pub(in crate::widget_host) fn clear_hover_below_collab_panel(&mut self) -> bool {
        let mut changed = false;
        {
            let ui = &mut self.editor_state.editor_ui;
            changed |= ui.canvas_hover_node.take().is_some();
            changed |= ui.hovered_layer_id.take().is_some();
            changed |= ui.hovered_page_index.take().is_some();
            changed |= ui.file_menu.hover.take().is_some();
            changed |= ui.export_quick_menu_hover.take().is_some();
            changed |= ui.locale_picker.hover.take().is_some();
            changed |= ui.shape_picker.hover.take().is_some();
            changed |= ui.fill_type_picker.hover.take().is_some();
            changed |= ui.compositing_picker.hover.take().is_some();
            changed |= ui.toolbar_hover.take().is_some();
            changed |= ui.align_toolbar_hover.take().is_some();
            changed |= ui.statusbar_hover.take().is_some();
            changed |= ui.topbar_button_hover.take().is_some();
            changed |= std::mem::take(&mut ui.topbar_traffic_hover);
            changed |= ui.chat_model_picker.hover.take().is_some();
            changed |= ui.chat_header_hover.take().is_some();
            changed |= ui.chat_tab_hover.take().is_some();
            changed |= ui.chat_design_block_hover.take().is_some();
            changed |= ui.chat_footer_hover.take().is_some();
            changed |= ui.clear_chat_style_chip_hover();
            changed |= ui.chat_example_hover.take().is_some();
            changed |= ui.parallel_agents_picker_hover.take().is_some();
            changed |= ui.export_picker_hover.take().is_some();
            changed |= ui.variables_panel_hover.take().is_some();
            changed |= ui.variables_preset_menu_hover.take().is_some();
            changed |= ui.property_action_hover.take().is_some();
            changed |= ui.property_tab_hover.take().is_some();
            let git = &mut ui.git_panel;
            changed |= git.empty_hovered_card.take().is_some();
            changed |= std::mem::take(&mut git.branch_button_hovered);
            changed |= git.button_hover.take().is_some();
            changed |= git.tracked_picker.hover.take().is_some();
            changed |= git.branch_picker_menu.hover.take().is_some();
            changed |= git.overflow_menu.hover.take().is_some();
        }
        changed |= self.editor_state.codegen.framework_hover.take().is_some();
        changed |= self.editor_state.codegen.action_hover.take().is_some();
        self.last_hover_probe = None;
        if changed {
            self.mark_dirty();
        }
        changed
    }

    /// Clear hover state below the open chat model picker while preserving
    /// the picker's own active row.
    pub(in crate::widget_host) fn clear_hover_below_chat_model_picker(&mut self) -> bool {
        let changed = self.clear_lower_overlay_hover_impl(false);
        // Opening/owning the picker truncates canvas dispatch. Force the first
        // point after it closes to run a fresh tree walk even when it is within
        // the canvas hover jitter radius. Resetting the probe is not itself a
        // visual change, so preserve the helper's repaint result.
        self.last_hover_probe = None;
        changed
    }

    /// Clear hover feedback painted below the ordinary Chat panel while
    /// preserving every Chat-owned hover resolved for the same cursor event.
    /// Surfaces painted above Chat (StatusBar, AlignToolbar, property popovers,
    /// Git/context menus) are intentionally absent from this list.
    pub(in crate::widget_host) fn clear_hover_below_chat_panel(&mut self) -> bool {
        let mut changed = false;
        {
            let ui = &mut self.editor_state.editor_ui;
            changed |= ui.canvas_hover_node.take().is_some();
            changed |= ui.hovered_layer_id.take().is_some();
            changed |= ui.hovered_page_index.take().is_some();
            changed |= ui.toolbar_hover.take().is_some();
            changed |= ui.variables_panel_hover.take().is_some();
            changed |= ui.variables_preset_menu_hover.take().is_some();
            changed |= ui.property_action_hover.take().is_some();
            changed |= ui.property_tab_hover.take().is_some();
            changed |= ui.fill_type_picker.hover.take().is_some();
            changed |= ui.topbar_button_hover.take().is_some();
            changed |= std::mem::take(&mut ui.topbar_traffic_hover);
        }
        changed |= self.editor_state.codegen.framework_hover.take().is_some();
        changed |= self.editor_state.codegen.action_hover.take().is_some();
        // The next point back on canvas must run a fresh tree walk even when it
        // is less than the 3 px jitter threshold from the pre-Chat probe.
        self.last_hover_probe = None;
        if changed {
            self.mark_dirty();
        }
        changed
    }

    /// Clear Chat's own hover feedback plus every surface painted below Chat,
    /// while preserving the hover owned by whichever higher surface triggered
    /// the call (StatusBar, AlignToolbar, context/Git, or a property popover).
    /// This is the transition helper for a single cursor move from Chat into an
    /// overlapping higher-painted footprint.
    pub(in crate::widget_host) fn clear_chat_and_lower_hover(&mut self) -> bool {
        let mut changed = false;
        {
            let ui = &mut self.editor_state.editor_ui;
            changed |= ui.collab.panel.hover.take().is_some();
            changed |= ui.chat_model_picker.hover.take().is_some();
            changed |= ui.chat_header_hover.take().is_some();
            changed |= ui.chat_tab_hover.take().is_some();
            changed |= ui.chat_design_block_hover.take().is_some();
            changed |= ui.chat_footer_hover.take().is_some();
            changed |= ui.clear_chat_style_chip_hover();
            changed |= ui.chat_example_hover.take().is_some();
            changed |= ui.parallel_agents_picker_hover.take().is_some();
        }
        let lower_changed = self.clear_hover_below_chat_panel();
        if changed {
            self.mark_dirty();
        }
        changed || lower_changed
    }

    fn clear_lower_overlay_hover_impl(&mut self, clear_chat_model_picker: bool) -> bool {
        let mut changed = false;
        {
            let ui = &mut self.editor_state.editor_ui;
            changed |= ui.canvas_hover_node.take().is_some();
            changed |= ui.hovered_layer_id.take().is_some();
            changed |= ui.hovered_page_index.take().is_some();
            changed |= ui.file_menu.hover.take().is_some();
            changed |= ui.export_quick_menu_hover.take().is_some();
            changed |= ui.locale_picker.hover.take().is_some();
            changed |= ui.shape_picker.hover.take().is_some();
            changed |= ui.fill_type_picker.hover.take().is_some();
            changed |= ui.compositing_picker.hover.take().is_some();
            changed |= ui.toolbar_hover.take().is_some();
            changed |= ui.align_toolbar_hover.take().is_some();
            changed |= ui.topbar_button_hover.take().is_some();
            changed |= std::mem::take(&mut ui.topbar_traffic_hover);
            changed |= ui.collab.panel.hover.take().is_some();
            if clear_chat_model_picker {
                changed |= ui.chat_model_picker.hover.take().is_some();
            }
            changed |= ui.chat_header_hover.take().is_some();
            changed |= ui.chat_tab_hover.take().is_some();
            changed |= ui.chat_design_block_hover.take().is_some();
            changed |= ui.chat_footer_hover.take().is_some();
            changed |= ui.clear_chat_style_chip_hover();
            changed |= ui.chat_example_hover.take().is_some();
            changed |= ui.parallel_agents_picker_hover.take().is_some();
            changed |= ui.export_picker_hover.take().is_some();
            changed |= ui.variables_panel_hover.take().is_some();
            changed |= ui.variables_preset_menu_hover.take().is_some();
            // Right-rail panel hovers — cleared so a wash doesn't linger
            // under a floating panel covering the rail.
            changed |= ui.property_action_hover.take().is_some();
            changed |= ui.property_tab_hover.take().is_some();
            if let Some(menu) = ui.layer_context_menu.as_mut() {
                changed |= menu.menu.hover.take().is_some();
            }
        }
        if let Some(menu) = self.editor_state.ui.path_anchor_menu.as_mut() {
            changed |= menu.menu.hover.take().is_some();
        }
        changed |= self.editor_state.codegen.framework_hover.take().is_some();
        changed |= self.editor_state.codegen.action_hover.take().is_some();
        if changed {
            self.mark_dirty();
        }
        changed
    }

    pub(in crate::widget_host) fn clear_layer_panel_hover(&mut self) -> bool {
        let ui = &mut self.editor_state.editor_ui;
        let cleared_layer = ui.hovered_layer_id.take().is_some();
        let cleared_page = ui.hovered_page_index.take().is_some();
        let changed = cleared_layer || cleared_page;
        if changed {
            self.mark_dirty();
        }
        changed
    }

    /// Update the file-menu / locale-picker / shape-picker dropdown
    /// hover highlights from the cursor. At most one of the three is
    /// open at a time. `over_topmost` suppresses updates when a
    /// floating panel covers the chrome. Returns `true` on change.
    pub(in crate::widget_host) fn update_dropdown_hover(
        &mut self,
        x: f32,
        y: f32,
        over_topmost: bool,
    ) -> bool {
        use op_editor_ui::{Point2D, Rect};
        if over_topmost
            && !self.over_dropdown_overlay(x, y, self.last_viewport_w, self.last_viewport_h)
        {
            return false;
        }
        if self.editor_state.editor_ui.file_menu_open {
            use op_editor_ui::widgets::file_menu::FileMenu;
            use op_editor_ui::widgets::top_bar::TopBar;
            self.refresh_layout_scene();
            let top_bar_rect = Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(self.last_viewport_w, op_editor_ui::widgets::TOP_BAR_HEIGHT),
            };
            let anchor =
                TopBar::file_menu_rect(top_bar_rect, self.editor_state.editor_ui.window_fullscreen);
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let menu = FileMenu::from_editor_ui(&self.editor_state.editor_ui, now_secs);
            let panel = menu.rect_at(anchor);
            let new_hover = menu.hovered_at(panel, Point2D::new(x, y));
            if new_hover != self.editor_state.editor_ui.file_menu.hover {
                self.editor_state.editor_ui.file_menu.hover = new_hover;
                self.mark_dirty();
                return true;
            }
        }
        if self.editor_state.editor_ui.export_quick_menu_open {
            use op_editor_ui::widgets::ExportQuickMenu;
            self.refresh_layout_scene();
            let panel = self.export_quick_menu_rect(self.last_viewport_w);
            let menu = ExportQuickMenu::for_editor_ui(&self.editor_state.editor_ui);
            let new_hover = menu.hovered_at(panel, Point2D::new(x, y));
            if new_hover != self.editor_state.editor_ui.export_quick_menu_hover {
                self.editor_state.editor_ui.export_quick_menu_hover = new_hover;
                self.mark_dirty();
                return true;
            }
        }
        if self.editor_state.editor_ui.import_menu_open {
            use op_editor_ui::widgets::ImportMenu;
            self.refresh_layout_scene();
            let (anchor, viewport) =
                self.import_menu_anchor(self.last_viewport_w, self.last_viewport_h);
            let menu = ImportMenu::for_editor_ui(&self.editor_state.editor_ui);
            let new_hover = match menu.hit(anchor, viewport, Point2D::new(x, y)) {
                op_editor_ui::widgets::import_menu::SelectHit::Row(idx) => Some(idx),
                op_editor_ui::widgets::import_menu::SelectHit::Inside
                | op_editor_ui::widgets::import_menu::SelectHit::Outside => None,
            };
            if new_hover != self.editor_state.editor_ui.import_menu.hover {
                self.editor_state.editor_ui.import_menu.hover = new_hover;
                self.mark_dirty();
                return true;
            }
        }
        if self.editor_state.editor_ui.locale_picker.open {
            use op_editor_ui::widgets::locale_picker::LocalePicker;
            self.refresh_layout_scene();
            let panel = self.locale_picker_rect(self.last_viewport_w);
            let picker = LocalePicker::for_editor_ui(&self.editor_state.editor_ui);
            let new_hover = match picker.hit_popup(panel, Point2D::new(x, y)) {
                op_editor_ui::widgets::locale_picker::SelectHit::Row(idx) => Some(idx),
                op_editor_ui::widgets::locale_picker::SelectHit::Inside
                | op_editor_ui::widgets::locale_picker::SelectHit::Outside => None,
            };
            if new_hover != self.editor_state.editor_ui.locale_picker.hover {
                self.editor_state.editor_ui.locale_picker.hover = new_hover;
                self.mark_dirty();
                return true;
            }
        }
        if self.editor_state.editor_ui.shape_picker.open {
            use op_editor_ui::widgets::shape_picker::ShapePicker;
            self.refresh_layout_scene();
            let panel = self.shape_picker_rect(self.last_viewport_w, self.last_viewport_h);
            let picker = ShapePicker::for_editor_ui(&self.editor_state.editor_ui);
            let new_hover = match picker.hit_popup(panel, Point2D::new(x, y)) {
                op_editor_ui::widgets::shape_picker::SelectHit::Row(idx) => Some(idx),
                op_editor_ui::widgets::shape_picker::SelectHit::Inside
                | op_editor_ui::widgets::shape_picker::SelectHit::Outside => None,
            };
            if new_hover != self.editor_state.editor_ui.shape_picker.hover {
                self.editor_state.editor_ui.shape_picker.hover = new_hover;
                self.mark_dirty();
                return true;
            }
        }
        false
    }

    /// Update the Export-section select-popup row hover from the
    /// current cursor position. A no-op (returns `false`) when no
    /// export popup is open. Returns `true` if the hover changed.
    pub(in crate::widget_host) fn update_export_picker_hover(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
        panel: Option<&op_editor_ui::widgets::PropertyPanel>,
    ) -> bool {
        use op_editor_ui::Point2D;
        if !self.editor_state.editor_ui.export_scale_picker_open
            && !self.editor_state.editor_ui.export_format_picker_open
        {
            return false;
        }
        let Some(panel) = panel else {
            return false;
        };
        let property_rect = self.property_rect(viewport_w, viewport_h);
        let new_hover = panel.export_picker_row_at(property_rect, Point2D::new(x, y));
        if new_hover != self.editor_state.editor_ui.export_picker_hover {
            self.editor_state.editor_ui.export_picker_hover = new_hover;
            self.mark_dirty();
            true
        } else {
            false
        }
    }

    /// Update the Effects "+" add-menu hovered row from the cursor
    /// position (mirrors [`Self::update_export_picker_hover`]). Returns
    /// `true` when the hover changed.
    pub(in crate::widget_host) fn update_effect_add_menu_hover(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
        panel: Option<&op_editor_ui::widgets::PropertyPanel>,
    ) -> bool {
        use op_editor_ui::Point2D;
        if !self.editor_state.editor_ui.effect_add_picker_open {
            return false;
        }
        let Some(panel) = panel else {
            return false;
        };
        let property_rect = self.property_rect(viewport_w, viewport_h);
        let new_hover = panel.effect_add_menu_row_at(property_rect, Point2D::new(x, y));
        if new_hover != self.editor_state.editor_ui.effect_add_menu_hover {
            self.editor_state.editor_ui.effect_add_menu_hover = new_hover;
            self.mark_dirty();
            true
        } else {
            false
        }
    }

    /// Track the Interactions section's Navigate/Back/Remove popover
    /// row under the cursor so it paints a hover wash. No-op when the
    /// popover is closed. Mirrors [`Self::update_effect_add_menu_hover`].
    pub(in crate::widget_host) fn update_interaction_menu_hover(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
        panel: Option<&op_editor_ui::widgets::PropertyPanel>,
    ) -> bool {
        use op_editor_ui::Point2D;
        if !self.editor_state.editor_ui.interaction_menu_open {
            return false;
        }
        let Some(panel) = panel else {
            return false;
        };
        let property_rect = self.property_rect(viewport_w, viewport_h);
        let new_hover = panel.interaction_menu_row_at(property_rect, Point2D::new(x, y));
        if new_hover != self.editor_state.editor_ui.interaction_menu_hover {
            self.editor_state.editor_ui.interaction_menu_hover = new_hover;
            self.mark_dirty();
            true
        } else {
            false
        }
    }

    /// Update the layer-panel hover id from the current cursor
    /// position. Returns `true` if the hover state changed.
    pub fn update_layer_hover(&mut self, x: f32, y: f32, viewport_w: f32, viewport_h: f32) -> bool {
        use op_editor_ui::widgets::LayerPanelHit;
        let sidebar_open = self.editor_state.editor_ui.sidebar_open;
        let panel_w = self.editor_state.editor_ui.layer_panel_width;
        let point = Point2D::new(x, y);
        // A top-most overlay covers the layer rail when dragged over it — no
        // row highlights underneath it.
        let blocked_by_overlay = self
            .chat_model_picker_rect(viewport_w, viewport_h)
            .is_some()
            || self.chat_panel_surface_contains(x, y, viewport_w, viewport_h)
            || self.over_topmost_panel(x, y, viewport_w, viewport_h)
            || self.over_dropdown_overlay(x, y, viewport_w, viewport_h)
            || self
                .layer_context_menu_rect()
                .is_some_and(|rect| rect.contains(point));
        let (new_layer, new_page) = if sidebar_open
            && !blocked_by_overlay
            && y >= TOP_BAR_HEIGHT
            && x >= 0.0
            && x <= panel_w
        {
            let layer_rect = self.layers_content_rect(viewport_w, viewport_h);
            let panel = self.layer_panel();
            match panel.hit_test(layer_rect, point) {
                Some(LayerPanelHit::Layer(id))
                | Some(LayerPanelHit::ToggleHidden(id))
                | Some(LayerPanelHit::ToggleLocked(id))
                | Some(LayerPanelHit::ToggleCollapsed(id)) => (Some(id), None),
                Some(LayerPanelHit::Page(idx)) | Some(LayerPanelHit::DeletePage(idx)) => {
                    (None, Some(idx))
                }
                _ => (None, None),
            }
        } else {
            (None, None)
        };
        // shell-core hit-test returns shell-core `NodeId`s; translate
        // to op-editor-core ids for storage on `editor_ui`.
        let new_layer_ec = new_layer.clone();
        let changed = new_layer_ec != self.editor_state.editor_ui.hovered_layer_id
            || new_page != self.editor_state.editor_ui.hovered_page_index;
        if changed {
            self.editor_state.editor_ui.hovered_layer_id = new_layer_ec;
            self.editor_state.editor_ui.hovered_page_index = new_page;
        }
        changed
    }

    pub fn cursor_over_layer_panel(
        &self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        self.editor_state.editor_ui.sidebar_open
            && self
                .chat_model_picker_rect(viewport_w, viewport_h)
                .is_none()
            && !self.chat_panel_surface_contains(x, y, viewport_w, viewport_h)
            && !self.over_topmost_panel(x, y, viewport_w, viewport_h)
            && !self.over_dropdown_overlay(x, y, viewport_w, viewport_h)
            && y >= TOP_BAR_HEIGHT
            && x >= 0.0
            && x <= self.editor_state.editor_ui.layer_panel_width
    }

    pub fn layer_drag_in_progress(&self) -> bool {
        self.layer_drag.is_some()
    }

    /// True when the cursor is over a draggable node inside the
    /// canvas region (used by the runner to flip cursor → Move).
    pub fn cursor_over_node(&self, x: f32, y: f32, viewport_w: f32, viewport_h: f32) -> bool {
        if matches!(self.editor_state.tool, op_editor_core::Tool::Hand) {
            return false;
        }
        if !self.over_canvas(x, y, viewport_w, viewport_h) {
            return false;
        }
        let doc_point = canvas_geometry::canvas_doc_point_unclamped(&self.editor_state, x, y);
        let zoom = self.editor_state.viewport.zoom;
        self.layout_scene
            .node_at_doc_point(doc_point, zoom)
            .is_some()
    }

    /// True while a node-drag is in flight.
    pub fn is_dragging_node(&self) -> bool {
        self.node_drag.is_some()
    }

    /// Canvas region (logical px, viewport-relative). The math is
    /// single-sourced with the web host — see the coordinate invariant in
    /// `op_editor_ui::widgets::host_canvas_geometry`.
    pub(in crate::widget_host) fn canvas_region(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> (f32, f32, f32, f32) {
        canvas_geometry::canvas_region(&self.editor_state, viewport_w, viewport_h)
    }

    /// Bottom-right floating StatusBar rect — mirrors the placement in
    /// `widget_host/paint.rs` §8. `None` when the canvas is too narrow
    /// to float the pill (matching the paint guard).
    pub(in crate::widget_host) fn status_bar_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<Rect> {
        if self.editor_state.editor_ui.touch_chrome() {
            return None;
        }
        canvas_geometry::status_bar_rect(&self.editor_state, viewport_w, viewport_h)
    }

    /// Step the canvas zoom from a StatusBar `[-]` / `[+]` click,
    /// anchored at the canvas-region centre so the visible content
    /// scales in place. `zoom_in` picks the sign; the magnitude maps
    /// to ≈ ±20 % per click through `Viewport::zoom_at`.
    /// Step the canvas zoom from a StatusBar `[-]` / `[+]` click.
    /// Shared with the web host — see
    /// `op_editor_ui::widgets::host_overlay_geometry::status_bar_zoom`.
    pub(in crate::widget_host) fn status_bar_zoom(
        &mut self,
        zoom_in: bool,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        overlay_geometry::status_bar_zoom(&mut self.editor_state, zoom_in, viewport_w, viewport_h);
        self.mark_dirty();
    }

    /// Zoom + pan so the active page's content is framed within the
    /// canvas region (the StatusBar search action). No-op for an empty
    /// page.
    pub(in crate::widget_host) fn zoom_to_fit(&mut self, viewport_w: f32, viewport_h: f32) {
        self.refresh_layout_scene();
        overlay_geometry::zoom_to_fit(
            &mut self.editor_state,
            &self.layout_scene,
            viewport_w,
            viewport_h,
        );
        self.mark_dirty();
    }

    /// Anchor / bezier-handle hit-test for the selected Path node.
    /// The math is shared with the web host — see
    /// `op_editor_ui::widgets::host_canvas_geometry::path_anchor_hit`.
    pub(in crate::widget_host) fn path_anchor_hit(
        &self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<(String, usize, super::AnchorDragTarget)> {
        canvas_geometry::path_anchor_hit(
            &self.editor_state,
            &self.layout_scene,
            x,
            y,
            viewport_w,
            viewport_h,
        )
    }

    /// When a single Ellipse is selected with the Select tool,
    /// hit-test whether `(x, y)` lands on one of its three arc
    /// handles. Returns the node id + which handle on a hit.
    pub(in crate::widget_host) fn arc_handle_hit(
        &self,
        x: f32,
        y: f32,
        _viewport_w: f32,
        _viewport_h: f32,
    ) -> Option<(String, op_editor_ui::widgets::ArcHandle)> {
        if !matches!(self.editor_state.tool, op_editor_core::Tool::Select) {
            return None;
        }
        if self.editor_state.selection_count() != 1 {
            return None;
        }
        let sel = self.editor_state.selection.anchor.as_str().to_string();
        let node = self.layout_scene.active_page()?.find(&sel)?;
        let handles = op_editor_ui::widgets::arc_handle_positions(node)?;
        let zoom = self.editor_state.viewport.zoom.max(0.0001);
        let mut doc_point = canvas_geometry::canvas_doc_point_unclamped(&self.editor_state, x, y);
        // Un-rotate the cursor into the ellipse's local frame.
        if node.rotation.abs() > f32::EPSILON {
            let b = node.bounds;
            let centre = Point2D::new(b.origin.x + b.size.x / 2.0, b.origin.y + b.size.y / 2.0);
            doc_point = op_editor_ui::widgets::rotate_point(doc_point, centre, -node.rotation);
        }
        // ~7 screen-px grab radius, expressed in doc space.
        let r2 = 49.0 / (zoom * zoom);
        // Reverse paint order so the topmost-painted handle wins —
        // on a full-sweep ellipse the Start + Sweep handles coincide,
        // and Sweep is painted last, so it must hit-test first.
        for (handle, p) in handles.into_iter().rev() {
            let dx = doc_point.x - p.x;
            let dy = doc_point.y - p.y;
            if dx * dx + dy * dy <= r2 {
                return Some((sel, handle));
            }
        }
        None
    }

    /// Resolve a screen point to an align/distribute action or boolean
    /// operation if it lands on the floating selection toolbar.
    pub(in crate::widget_host) fn selection_toolbar_hit(
        &self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<op_editor_ui::widgets::AlignToolbarHit> {
        if self.editor_state.editor_ui.touch_chrome() {
            return None;
        }
        use op_editor_ui::widgets::AlignToolbar;
        let canvas_region =
            canvas_geometry::canvas_rect(&self.editor_state, viewport_w, viewport_h);
        AlignToolbar::for_canvas_region(canvas_region, &self.editor_state)?
            .hit_test_action(Point2D::new(x, y))
    }

    /// Resolve a screen point to an `AlignAction` if it lands on the
    /// floating align toolbar (visible when 2+ selected).
    pub(in crate::widget_host) fn align_toolbar_hit(
        &self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<op_editor_core::AlignAction> {
        match self.selection_toolbar_hit(x, y, viewport_w, viewport_h) {
            Some(op_editor_ui::widgets::AlignToolbarHit::Align(action)) => Some(action),
            _ => None,
        }
    }
}
