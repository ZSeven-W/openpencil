//! Geometry + cursor-affordance methods on `WidgetHostNative`.
//! Pure-math helpers that map (viewport_w, viewport_h, x, y) into
//! the rects + cursor hints the host serves.
//!
//! Scalar / chrome reads go straight to `editor_state`; node-tree
//! hit-tests run against the layout-resolved `LayoutScene`. The
//! input-dispatch contract keeps `layout_scene` fresh before any
//! hit-testing input event (see `widget_host.rs`).

use super::helpers::{PANEL_RESIZE_GUTTER, STATUS_INSET};
use super::{cursor_for_handle, CursorHint, PanelResizeKind, WidgetHostNative};
use op_editor_ui::widgets::{
    rotation_corner_at_point, selection_handle_at_point, AIChatHit, AIChatPlaceholder,
    ChatResizeEdge, STATUS_BAR_HEIGHT, STATUS_BAR_WIDTH, TOP_BAR_HEIGHT,
};
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
        let (cx0, cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        x >= cx0 && x <= cx0 + cw && y >= cy0 && y <= cy0 + ch
    }

    /// True when the cursor is over either resize gutter — used by
    /// the runner to set `CursorIcon::EwResize`. None = no gutter.
    pub fn panel_resize_hover(&self, x: f32, y: f32, viewport_w: f32) -> Option<PanelResizeKind> {
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

    fn chat_resize_cursor(edge: ChatResizeEdge) -> CursorHint {
        match edge {
            ChatResizeEdge::E | ChatResizeEdge::W => CursorHint::ResizeEw,
            ChatResizeEdge::N | ChatResizeEdge::S => CursorHint::ResizeNs,
            ChatResizeEdge::Nw | ChatResizeEdge::Se => CursorHint::ResizeNwse,
            ChatResizeEdge::Ne | ChatResizeEdge::Sw => CursorHint::ResizeNesw,
        }
    }

    /// Clear every lower-overlay hover highlight — file menu, locale
    /// / shape pickers, align toolbar, layer-context menu. Called
    /// when the cursor moves over the top-most Design-MD panel so a
    /// highlight set just before does not linger beneath it. Returns
    /// `true` if anything changed.
    pub(in crate::widget_host) fn clear_lower_overlay_hover(&mut self) -> bool {
        let mut changed = false;
        {
            let ui = &mut self.editor_state.editor_ui;
            changed |= ui.canvas_hover_node.take().is_some();
            changed |= ui.hovered_layer_id.take().is_some();
            changed |= ui.hovered_page_index.take().is_some();
            changed |= ui.file_menu.hover.take().is_some();
            changed |= ui.locale_picker.hover.take().is_some();
            changed |= ui.shape_picker.hover.take().is_some();
            changed |= ui.fill_type_picker.hover.take().is_some();
            changed |= ui.toolbar_hover.take().is_some();
            changed |= ui.align_toolbar_hover.take().is_some();
            changed |= ui.chat_model_picker.hover.take().is_some();
            changed |= ui.chat_design_block_hover.take().is_some();
            changed |= ui.chat_footer_hover.take().is_some();
            changed |= ui.export_picker_hover.take().is_some();
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
    ) -> bool {
        use op_editor_ui::widgets::{PropertyPanel, TOP_BAR_HEIGHT};
        use op_editor_ui::{Point2D, Rect};
        if !self.editor_state.editor_ui.export_scale_picker_open
            && !self.editor_state.editor_ui.export_format_picker_open
        {
            return false;
        }
        self.refresh_layout_scene();
        let Some(panel) = PropertyPanel::for_selection(&self.editor_state) else {
            return false;
        };
        let property_rect = Rect {
            origin: Point2D::new(
                viewport_w - self.editor_state.editor_ui.property_panel_width,
                TOP_BAR_HEIGHT,
            ),
            size: Point2D::new(
                self.editor_state.editor_ui.property_panel_width,
                (viewport_h - TOP_BAR_HEIGHT).max(0.0),
            ),
        };
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
    ) -> bool {
        use op_editor_ui::widgets::{PropertyPanel, TOP_BAR_HEIGHT};
        use op_editor_ui::{Point2D, Rect};
        if !self.editor_state.editor_ui.effect_add_picker_open {
            return false;
        }
        self.refresh_layout_scene();
        let Some(panel) = PropertyPanel::for_selection(&self.editor_state) else {
            return false;
        };
        let property_rect = Rect {
            origin: Point2D::new(
                viewport_w - self.editor_state.editor_ui.property_panel_width,
                TOP_BAR_HEIGHT,
            ),
            size: Point2D::new(
                self.editor_state.editor_ui.property_panel_width,
                (viewport_h - TOP_BAR_HEIGHT).max(0.0),
            ),
        };
        let new_hover = panel.effect_add_menu_row_at(property_rect, Point2D::new(x, y));
        if new_hover != self.editor_state.editor_ui.effect_add_menu_hover {
            self.editor_state.editor_ui.effect_add_menu_hover = new_hover;
            self.mark_dirty();
            true
        } else {
            false
        }
    }

    /// Update the layer-panel hover id from the current cursor
    /// position. Returns `true` if the hover state changed.
    pub fn update_layer_hover(&mut self, x: f32, y: f32, viewport_w: f32, viewport_h: f32) -> bool {
        use op_editor_ui::widgets::{LayerPanel, LayerPanelHit};
        let sidebar_open = self.editor_state.editor_ui.sidebar_open;
        let panel_w = self.editor_state.editor_ui.layer_panel_width;
        // A top-most floating panel covers the layer rail when dragged
        // over it — no row highlights underneath it.
        let blocked_by_overlay = self.over_topmost_panel(x, y, viewport_w, viewport_h)
            || self.over_dropdown_overlay(x, y, viewport_w, viewport_h);
        let (new_layer, new_page) = if sidebar_open
            && !blocked_by_overlay
            && y >= TOP_BAR_HEIGHT
            && x >= 0.0
            && x <= panel_w
        {
            let layer_rect = Rect {
                origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
                size: Point2D::new(panel_w, (viewport_h - TOP_BAR_HEIGHT).max(0.0)),
            };
            let panel = LayerPanel::from_editor(&self.editor_state);
            match panel.hit_test(layer_rect, Point2D::new(x, y)) {
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
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_w, viewport_h);
        let canvas_local = Point2D::new(x - cx0, y - cy0);
        let doc_point = self.editor_state.viewport.to_document(canvas_local);
        let zoom = self.editor_state.viewport.zoom;
        self.layout_scene
            .node_at_doc_point(doc_point, zoom)
            .is_some()
    }

    /// True while a node-drag is in flight.
    pub fn is_dragging_node(&self) -> bool {
        self.node_drag.is_some()
    }

    pub fn cursor_hint(&self, x: f32, y: f32, viewport_w: f32, viewport_h: f32) -> CursorHint {
        use op_editor_core::Tool;
        // Modal overlays — keep the pointer the OS default.
        if self.editor_state.editor_ui.agent_settings_open
            || self.editor_state.ui.color_picker.is_some()
        {
            return CursorHint::Default;
        }
        // The disabled empty-state Init card shows a not-allowed cursor
        // (TS `cursor-not-allowed`) — checked before the overlay block
        // so it wins over the Git popover's neutral Default.
        if self.over_disabled_init_card(x, y, viewport_w, viewport_h) {
            return CursorHint::NotAllowed;
        }
        if let Some(resize) = self.chat_resize {
            return Self::chat_resize_cursor(resize.edge);
        }
        if let Some(edge) = self.chat_resize_hover(x, y, viewport_w, viewport_h) {
            return Self::chat_resize_cursor(edge);
        }
        // Floating VariablesPanel resize affordances (TS ew/ns/nwse
        // cursor strips) — in-flight drag first, then edge hover.
        if let Some(edge) = self.variables_resize.or_else(|| {
            self.variables_panel_rect(viewport_w, viewport_h)
                .and_then(|rect| {
                    use op_editor_ui::widgets::variables_panel::VariablesPanel;
                    VariablesPanel::for_editor(&self.editor_state)
                        .resize_edge_at(rect, Point2D::new(x, y))
                })
        }) {
            use op_editor_ui::widgets::variables_panel::VariablesResizeEdge;
            return match edge {
                VariablesResizeEdge::Right => CursorHint::ResizeEw,
                VariablesResizeEdge::Bottom => CursorHint::ResizeNs,
                VariablesResizeEdge::Corner => CursorHint::ResizeNwse,
            };
        }
        if let Some(chat_rect) = self.ai_chat_rect(viewport_w, viewport_h) {
            let panel = AIChatPlaceholder::from_editor(&self.editor_state);
            if let Some(AIChatHit::SelectInputText(_) | AIChatHit::SelectTranscriptText(_, _)) =
                panel.hit_test(chat_rect, Point2D::new(x, y))
            {
                return CursorHint::Text;
            }
        }
        // Any floating overlay (panels, Git popover, Toolbar /
        // StatusBar / chat, open dropdowns) — a neutral cursor over
        // them, never a canvas action cursor (Move / Crosshair)
        // bleeding through from a node underneath.
        if self.over_floating_overlay(x, y, viewport_w, viewport_h) {
            return CursorHint::Default;
        }
        // The floating Git panel paints on top of the right-rail
        // resize gutter (and in diff mode is wide enough to cover
        // it), so don't show the resize cursor over the panel.
        let over_git_panel = self
            .git_panel_outer_rect(viewport_w, viewport_h)
            .is_some_and(|r| (r).contains(Point2D::new(x, y)));
        if self.is_resizing_panel()
            || (!over_git_panel && self.panel_resize_hover(x, y, viewport_w).is_some())
        {
            return CursorHint::ResizeEw;
        }
        if self.is_dragging_node() {
            return CursorHint::Default;
        }
        if self.rotate_drag.is_some() {
            return CursorHint::Rotate;
        }
        if let Some(handle) = self.handle_drag.map(|d| d.handle) {
            return cursor_for_handle(handle);
        }
        if !self.over_canvas(x, y, viewport_w, viewport_h) {
            return CursorHint::Default;
        }
        let (cx0, cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        let canvas_local = Point2D::new(x - cx0, y - cy0);
        let doc_point = self.editor_state.viewport.to_document(canvas_local);
        let zoom = self.editor_state.viewport.zoom;
        let over_canvas_node = self
            .layout_scene
            .node_at_doc_point(doc_point, zoom)
            .is_some();
        match self.editor_state.tool {
            Tool::Hand => CursorHint::Grab,
            // Shapes / Frame / Form-widget tools all place a node on
            // click — a placement crosshair reads the same for each.
            Tool::Rect
            | Tool::Ellipse
            | Tool::Polygon
            | Tool::Line
            | Tool::Pen
            | Tool::Frame
            | Tool::TextInput
            | Tool::TextArea
            | Tool::NumberInput
            | Tool::Select_
            | Tool::RadioGroup
            | Tool::Switch
            | Tool::Checkbox
            | Tool::Slider
            | Tool::Progress
            | Tool::Tabs => {
                if over_canvas_node {
                    CursorHint::Default
                } else {
                    CursorHint::Crosshair
                }
            }
            Tool::Text => CursorHint::Text,
            Tool::Select => {
                let canvas_rect = Rect {
                    origin: Point2D::new(cx0, cy0),
                    size: Point2D::new(cw, ch),
                };
                let point = Point2D::new(x, y);
                if let Some(handle) = selection_handle_at_point(
                    canvas_rect,
                    &self.layout_scene,
                    &self.editor_state,
                    point,
                ) {
                    return cursor_for_handle(handle);
                }
                if rotation_corner_at_point(
                    canvas_rect,
                    &self.layout_scene,
                    &self.editor_state,
                    point,
                )
                .is_some()
                {
                    return CursorHint::Rotate;
                }
                if over_canvas_node {
                    return CursorHint::Default;
                }
                CursorHint::Default
            }
        }
    }

    /// Canvas origin (logical px).
    pub(in crate::widget_host) fn canvas_origin(&self) -> (f32, f32) {
        let cx0 = if self.editor_state.editor_ui.sidebar_open {
            self.editor_state.editor_ui.layer_panel_width
        } else {
            0.0
        };
        (cx0, TOP_BAR_HEIGHT)
    }

    /// Canvas region (logical px, viewport-relative).
    pub(in crate::widget_host) fn canvas_region(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> (f32, f32, f32, f32) {
        let canvas_left = if self.editor_state.editor_ui.sidebar_open {
            self.editor_state.editor_ui.layer_panel_width
        } else {
            0.0
        };
        let rail_occupied = self.editor_state.right_rail_visible();
        let canvas_right = if rail_occupied {
            viewport_w - self.editor_state.editor_ui.property_panel_width
        } else {
            viewport_w
        };
        let canvas_w = (canvas_right - canvas_left).max(0.0);
        let canvas_h = (viewport_h - TOP_BAR_HEIGHT).max(0.0);
        (canvas_left, TOP_BAR_HEIGHT, canvas_w, canvas_h)
    }

    /// Bottom-right floating StatusBar rect — mirrors the placement in
    /// `widget_host/paint.rs` §8. `None` when the canvas is too narrow
    /// to float the pill (matching the paint guard).
    pub(in crate::widget_host) fn status_bar_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<Rect> {
        let (canvas_left, _top, canvas_w, canvas_h) = self.canvas_region(viewport_w, viewport_h);
        if canvas_w <= STATUS_BAR_WIDTH + STATUS_INSET * 2.0 {
            return None;
        }
        let canvas_right = canvas_left + canvas_w;
        Some(Rect {
            origin: Point2D::new(
                canvas_right - STATUS_BAR_WIDTH - STATUS_INSET,
                TOP_BAR_HEIGHT + canvas_h - STATUS_BAR_HEIGHT - STATUS_INSET,
            ),
            size: Point2D::new(STATUS_BAR_WIDTH, STATUS_BAR_HEIGHT),
        })
    }

    /// Step the canvas zoom from a StatusBar `[-]` / `[+]` click,
    /// anchored at the canvas-region centre so the visible content
    /// scales in place. `zoom_in` picks the sign; the magnitude maps
    /// to ≈ ±20 % per click through `Viewport::zoom_at`.
    pub(in crate::widget_host) fn status_bar_zoom(
        &mut self,
        zoom_in: bool,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        // `zoom_at` factor is `exp(delta * 0.0015)`; ±120 ≈ ±20 %.
        const STEP: f32 = 120.0;
        // `Viewport::zoom_at` works in CANVAS-LOCAL coordinates (every
        // other call feeds it `window_pt - canvas_origin`), so the
        // anchor is the canvas-region centre relative to the canvas
        // origin — NOT the window-space centre.
        let (_cx0, _cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        let center = op_editor_ui::Point2D::new(cw / 2.0, ch / 2.0);
        self.editor_state
            .viewport
            .zoom_at(center, if zoom_in { STEP } else { -STEP });
        self.mark_dirty();
    }

    /// Zoom + pan so the active page's content is framed within the
    /// canvas region (the StatusBar search action). No-op for an empty
    /// page.
    pub(in crate::widget_host) fn zoom_to_fit(&mut self, viewport_w: f32, viewport_h: f32) {
        self.refresh_layout_scene();
        let Some(content) = self.layout_scene.content_bounds() else {
            return;
        };
        let (_l, _t, canvas_w, canvas_h) = self.canvas_region(viewport_w, viewport_h);
        self.editor_state
            .viewport
            .fit_to_with_max_zoom(content, canvas_w, canvas_h, 64.0, 1.0);
        self.mark_dirty();
    }

    /// When the active selection is a single Path node + the Pen OR
    /// Select tool is active (TS edits path controls with Select —
    /// `skia-hit-handlers.ts::hitTestPathControl`), hit-test whether
    /// `(x, y)` lands on an anchor or one of its bezier handles.
    /// Handles are checked before anchors (TS order); returns the
    /// node id, anchor index, and which target.
    pub(in crate::widget_host) fn path_anchor_hit(
        &self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<(String, usize, super::AnchorDragTarget)> {
        use super::AnchorDragTarget;
        use op_editor_core::pen::PathHandleSide;
        use op_editor_ui::layout_scene::NodeKind;
        use op_editor_ui::widgets::path_handle_positions;
        if !matches!(
            self.editor_state.tool,
            op_editor_core::Tool::Pen | op_editor_core::Tool::Select
        ) {
            return None;
        }
        if self.editor_state.selection_count() != 1 {
            return None;
        }
        let sel = self.editor_state.selection.anchor.as_str().to_string();
        let node = self.layout_scene.active_page()?.find(&sel)?;
        if !matches!(node.kind, NodeKind::Path) {
            return None;
        }
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_w, viewport_h);
        let zoom = self.editor_state.viewport.zoom.max(0.0001);
        let canvas_local = Point2D::new(x - cx0, y - cy0);
        let mut doc = self.editor_state.viewport.to_document(canvas_local);
        // Un-rotate the cursor into the node's local frame — handle
        // positions are stored unrotated but the path paints rotated.
        if node.rotation.abs() > f32::EPSILON {
            let b = node.aggregate_bounds();
            let centre = Point2D::new(b.origin.x + b.size.x / 2.0, b.origin.y + b.size.y / 2.0);
            doc = op_editor_ui::widgets::rotate_point(doc, centre, -node.rotation);
        }
        // 8 screen-px grab radius (TS `PATH_CONTROL_HIT_RADIUS`),
        // expressed in doc space.
        let r2 = 64.0 / (zoom * zoom);
        let hit = |p: Point2D| (doc.x - p.x).powi(2) + (doc.y - p.y).powi(2) <= r2;
        // Handles hit-test before anchors — TS `hitTestPathControl`
        // walks handleOut → handleIn across all anchors first
        // (`skia-hit-handlers.ts:136-159`). Ghost (unset) handles are
        // grabbable only with the Pen tool — the Select-tool editor
        // shows existing handles only (TS overlay parity).
        let pen_tool = matches!(self.editor_state.tool, op_editor_core::Tool::Pen);
        for (i, a) in node.path_anchors.iter().enumerate() {
            let (hin, hout) = path_handle_positions(a, zoom);
            if (a.handle_out.is_some() || pen_tool) && hit(hout) {
                return Some((
                    sel.clone(),
                    i,
                    AnchorDragTarget::Handle(PathHandleSide::Out),
                ));
            }
            if (a.handle_in.is_some() || pen_tool) && hit(hin) {
                return Some((sel.clone(), i, AnchorDragTarget::Handle(PathHandleSide::In)));
            }
        }
        for (i, a) in node.path_anchors.iter().enumerate() {
            if hit(a.pos) {
                return Some((sel.clone(), i, AnchorDragTarget::Anchor));
            }
        }
        // Paths without resolved anchor data fall back to `points`.
        if node.path_anchors.is_empty() {
            for (i, p) in node.points.iter().enumerate() {
                if hit(*p) {
                    return Some((sel.clone(), i, AnchorDragTarget::Anchor));
                }
            }
        }
        None
    }

    /// When a single Ellipse is selected with the Select tool,
    /// hit-test whether `(x, y)` lands on one of its three arc
    /// handles. Returns the node id + which handle on a hit.
    pub(in crate::widget_host) fn arc_handle_hit(
        &self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
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
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_w, viewport_h);
        let zoom = self.editor_state.viewport.zoom.max(0.0001);
        let canvas_local = Point2D::new(x - cx0, y - cy0);
        let mut doc_point = self.editor_state.viewport.to_document(canvas_local);
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
        use op_editor_ui::widgets::AlignToolbar;
        let (cx, _, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        let canvas_region = Rect {
            origin: Point2D::new(cx, TOP_BAR_HEIGHT),
            size: Point2D::new(cw, ch),
        };
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
