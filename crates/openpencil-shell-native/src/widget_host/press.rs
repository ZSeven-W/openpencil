//! Mouse-press dispatcher.
use super::helpers::color_to_hex;
use super::helpers::{rect_contains, TOOLBAR_INSET_X, TOOLBAR_INSET_Y};
use super::{
    ChatDragState, CreateDragState, DragState, HandleDragState, NodeDragState, PanelResize,
    PanelResizeKind, RotateDragState, WidgetHostNative,
};
use openpencil_shell_core::widgets::{
    rotation_corner_at_point, selection_handle_at_point, AIChatHit, AIChatPlaceholder, LayoutCx,
    LocalePicker, PropertyPanel, ShapeChoice, ShapePicker, Toolbar, TopBar, TopBarHit, Widget,
    TOOLBAR_WIDTH, TOP_BAR_HEIGHT,
};
use openpencil_shell_core::{Point2D, Rect};

impl WidgetHostNative {
    fn dispatch_layer_context_action(
        &mut self,
        action: openpencil_shell_core::widgets::layer_context_menu::LayerContextAction,
        target: openpencil_shell_core::document::LayerContextTarget,
    ) {
        use openpencil_shell_core::document::LayerContextTarget as T;
        use openpencil_shell_core::widgets::layer_context_menu::LayerContextAction as A;
        match (action, target) {
            (A::Duplicate, T::Layer(id)) => {
                self.document.set_single_selection(id);
                self.document.commit_history();
                let _ = self
                    .document
                    .duplicate_selected(&mut self.next_node_id, 10.0);
            }
            (A::Delete, T::Layer(id)) => {
                self.document.set_single_selection(id);
                self.document.commit_history();
                let _ = self.document.delete_selected();
            }
            (A::ToggleLock, T::Layer(id)) => {
                self.document.toggle_node_locked(id);
            }
            (A::ToggleVisibility, T::Layer(id)) => {
                self.document.toggle_node_hidden(id);
            }
            (A::CreateComponent, T::Layer(_)) => {} // stub
            (A::DuplicatePage, T::Page(idx)) => {
                let _ = self.document.duplicate_page(idx);
            }
            (A::MovePageUp, T::Page(idx)) => {
                let _ = self.document.move_page_up(idx);
            }
            (A::MovePageDown, T::Page(idx)) => {
                let _ = self.document.move_page_down(idx);
            }
            (A::DeletePage, T::Page(idx)) => {
                let _ = self.document.remove_page(idx);
            }
            (A::RenamePage, T::Page(idx)) => {
                if self.document.start_rename_page(idx) {
                    self.document.ui.rename_caret_anchor_ms = self.now_ms;
                }
            }
            (A::RenameLayer, T::Layer(id)) => {
                if self.document.start_rename_layer(id) {
                    self.document.ui.rename_caret_anchor_ms = self.now_ms;
                }
            }
            _ => {} // mismatched action/target — no-op
        }
    }

    /// Right-click handler — opens the LayerPanel context menu on
    /// a layer row OR page row.
    pub fn apply_right_press(&mut self, x: f32, y: f32, _viewport_w: f32, viewport_h: f32) -> bool {
        if !self.document.ui.sidebar_open {
            return false;
        }
        use openpencil_shell_core::document::{LayerContextMenuState, LayerContextTarget};
        use openpencil_shell_core::widgets::{LayerPanel, LayerPanelHit};
        let layer_rect = Rect {
            origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
            size: Point2D::new(
                self.document.ui.layer_panel_width,
                (viewport_h - TOP_BAR_HEIGHT).max(0.0),
            ),
        };
        let panel = LayerPanel::from_document(&self.document);
        match panel.hit_test(layer_rect, Point2D::new(x, y)) {
            Some(LayerPanelHit::Layer(id)) => {
                self.document.set_single_selection(id);
                self.document.ui.layer_context_menu = Some(LayerContextMenuState {
                    target: LayerContextTarget::Layer(id),
                    anchor_x: x,
                    anchor_y: y,
                    hovered_row: None,
                });
                return true;
            }
            Some(LayerPanelHit::Page(idx)) => {
                self.document.ui.layer_context_menu = Some(LayerContextMenuState {
                    target: LayerContextTarget::Page(idx),
                    anchor_x: x,
                    anchor_y: y,
                    hovered_row: None,
                });
                return true;
            }
            _ => {}
        }
        if self.document.ui.layer_context_menu.take().is_some() {
            return true;
        }
        false
    }

    /// Mouse-press handler. Returns whether anything visible changed.
    pub fn apply_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        self.last_viewport_w = viewport_width;
        self.last_viewport_h = viewport_height;
        // Blur-commit rename + text-edit; track to repaint on click.
        let rename_committed =
            self.document.ui.layer_rename.is_some() && self.document.rename_commit();
        let text_edit_was_active = self.document.ui.text_editing.is_some();
        let text_edit_committed = self.document.text_edit_commit();
        if self.document.ui.agent_settings_open
            && self.dispatch_agent_settings_press(x, y, viewport_width, viewport_height)
        {
            return true;
        }
        // 0-color. Color picker overlay — top-most when open.
        if let Some(state) = self.document.ui.color_picker.clone() {
            use openpencil_shell_core::widgets::color_picker::{
                drag_for_hit, ColorPicker, ColorPickerHit,
            };
            let picker = ColorPicker::for_state(&self.document, state.clone());
            let panel = picker.rect(viewport_width, viewport_height);
            let point = Point2D::new(x, y);
            match picker.hit_test(panel, point) {
                Some(ColorPickerHit::Close) => {
                    let _ = self.document.close_color_picker();
                    return true;
                }
                Some(ColorPickerHit::Eyedropper) | Some(ColorPickerHit::Inside) => {
                    return true;
                }
                Some(hit @ (ColorPickerHit::SvBox | ColorPickerHit::HueSlider)) => {
                    if let Some(kind) = drag_for_hit(hit) {
                        // Live-apply once for the press point.
                        match hit {
                            ColorPickerHit::SvBox => {
                                let (s, v) = picker.sv_at(panel, point);
                                let _ = self.document.color_picker_set_hsv(state.hue, s, v);
                            }
                            ColorPickerHit::HueSlider => {
                                let h = picker.hue_at(panel, point);
                                let _ = self.document.color_picker_set_hsv(h, state.sat, state.val);
                            }
                            _ => {}
                        }
                        self.document.color_picker_set_drag(Some(kind));
                    }
                    return true;
                }
                None => {
                    // Press outside the panel — close it; the click
                    // continues to the next overlay so the user can
                    // immediately interact with whatever they aimed
                    // at.
                    let _ = self.document.close_color_picker();
                }
            }
        }

        if let Some(state) = self.document.ui.layer_context_menu {
            use openpencil_shell_core::widgets::layer_context_menu::LayerContextMenu;
            let menu = LayerContextMenu::for_state(&self.document, state);
            if let Some(action) = menu.hit_test(Point2D::new(x, y)) {
                self.dispatch_layer_context_action(action, state.target);
                self.document.ui.layer_context_menu = None;
                return true;
            }
            self.document.ui.layer_context_menu = None;
            return true;
        }
        // 0aa. Commit-on-blur for property-panel inputs.
        if self.document.ui.property_focus.is_some() {
            let property_left = if self.document.property_panel_visible() {
                viewport_width - self.document.ui.property_panel_width
            } else {
                viewport_width
            };
            if x < property_left {
                self.commit_property_focus_if_any();
            }
        }

        // 0z. Panel-resize gutter — ±4 px from rail edges.
        if y >= TOP_BAR_HEIGHT {
            if let Some(kind) = self.panel_resize_hover(x, y, viewport_width) {
                let start_width = match kind {
                    PanelResizeKind::LayerRight => self.document.ui.layer_panel_width,
                    PanelResizeKind::PropertyLeft => self.document.ui.property_panel_width,
                };
                self.panel_resize = Some(PanelResize {
                    kind,
                    start_x: x,
                    start_width,
                });
                return true;
            }
        }

        // 0ab. Shape picker overlay.
        if self.document.ui.shape_picker_open {
            let panel_rect = self.shape_picker_rect(viewport_width, viewport_height);
            let picker = ShapePicker::for_document(&self.document);
            if let Some(choice) = picker.hit_test(panel_rect, Point2D::new(x, y)) {
                match choice {
                    ShapeChoice::Tool(tool) => {
                        let _ = self.document.finish_pen_path();
                        self.document.ui.shape_tool = tool;
                        self.document.tool = tool;
                    }
                    ShapeChoice::OpenIconPicker | ShapeChoice::ImportImageOrSvg => {
                        // Host-side dispatch lands when the icon
                        // picker / file dialog widgets ship.
                    }
                }
                self.document.ui.shape_picker_open = false; self.document.ui.shape_picker_hover = None;
                return true;
            }
            self.document.ui.shape_picker_open = false; self.document.ui.shape_picker_hover = None;
            return true;
        }

        if self.document.ui.file_menu_open { self.dispatch_file_menu_press(x, y, viewport_width); return true; }
        if self.document.ui.export_dialog_open { self.dispatch_export_dialog_press(x, y, viewport_width, viewport_height); return true; }
        if self.document.ui.figma_import_open { self.dispatch_figma_import_press(x, y, viewport_width, viewport_height); return true; }

        // 0a. Locale picker overlay — top-most when open; click outside closes.
        if self.document.ui.locale_picker_open {
            let panel_rect = self.locale_picker_rect(viewport_width);
            let picker = LocalePicker::for_document(&self.document);
            if let Some(locale) = picker.hit_test(panel_rect, Point2D::new(x, y)) {
                self.document.ui.locale = locale;
                self.document.ui.locale_picker_open = false; self.document.ui.locale_picker_hover = None;
                return true;
            }
            self.document.ui.locale_picker_open = false; self.document.ui.locale_picker_hover = None;
            return true;
        }

        // 0b. TopBar — sidebar toggle button + theme + locale picker.
        let top_bar_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_width, TOP_BAR_HEIGHT),
        };
        let top_bar = TopBar::for_document(&self.document);
        if let Some(hit) = top_bar.hit_test(top_bar_rect, Point2D::new(x, y)) {
            match hit {
                TopBarHit::ToggleSidebar => {
                    self.document.ui.sidebar_open = !self.document.ui.sidebar_open;
                    return true;
                }
                TopBarHit::ToggleTheme => {
                    self.document.ui.theme_mode = self.document.ui.theme_mode.flipped();
                    return true;
                }
                TopBarHit::ToggleLocale => {
                    self.document.ui.locale_picker_open = !self.document.ui.locale_picker_open;
                    return true;
                }
                TopBarHit::OpenAgentSettings => { self.document.ui.agent_settings_open = true; return true; }
                TopBarHit::ToggleFileMenu => { self.document.ui.file_menu_open ^= true; return true; }
                TopBarHit::OpenFigmaImport => { self.document.ui.figma_import_open = true; return true; }
            }
        }
        if rect_contains(top_bar_rect, Point2D::new(x, y)) {
            // Other top-bar gaps eat clicks but don't act.
            return rename_committed || text_edit_committed;
        }

        // 0c0. Fill-type picker — outside-click dismiss.
        if self.document.ui.fill_type_picker_open {
            if let Some(panel) = PropertyPanel::for_selection(&self.document) {
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
                if let Some(action) = panel.hit_test_action(property_rect, Point2D::new(x, y)) {
                    if matches!(
                        action,
                        openpencil_shell_core::widgets::PropertyPanelAction::SetFillType(_)
                            | openpencil_shell_core::widgets::PropertyPanelAction::ToggleFillTypePicker
                    ) {
                        self.apply_property_action(action);
                        return true;
                    }
                }
            }
            // Anywhere else — close + swallow.
            self.document.ui.fill_type_picker_open = false;
            return true;
        }

        // 0c. PropertyPanel input row — focus the row + seed the
        //     edit draft from the snapshot value. Any other click
        //     (canvas, chat, toolbar, layer panel) commits + clears
        //     the focused input via the catch-all branches below.
        if let Some(panel) = PropertyPanel::for_selection(&self.document) {
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
            // Button / checkbox click first (flex modes + size flags).
            if let Some(action) = panel.hit_test_action(property_rect, Point2D::new(x, y)) {
                self.commit_property_focus_if_any();
                if let openpencil_shell_core::widgets::PropertyPanelAction::OpenColorPicker(
                    target,
                ) = action
                {
                    let _ = self.document.open_color_picker(target, y);
                } else {
                    self.apply_property_action(action);
                }
                return true;
            }
            if let Some(focus) = panel.hit_test(property_rect, Point2D::new(x, y)) {
                self.commit_property_focus_if_any();
                let initial = match focus {
                    openpencil_shell_core::document::PropertyFocus::PositionX => {
                        panel.snapshot.x.to_string()
                    }
                    openpencil_shell_core::document::PropertyFocus::PositionY => {
                        panel.snapshot.y.to_string()
                    }
                    openpencil_shell_core::document::PropertyFocus::SizeW => {
                        panel.snapshot.width.to_string()
                    }
                    openpencil_shell_core::document::PropertyFocus::SizeH => {
                        panel.snapshot.height.to_string()
                    }
                    openpencil_shell_core::document::PropertyFocus::Rotation => {
                        (panel.snapshot.rotation_deg.round() as i32).to_string()
                    }
                    openpencil_shell_core::document::PropertyFocus::PositionR => {
                        (panel.snapshot.corner_radius.round() as i32).to_string()
                    }
                    openpencil_shell_core::document::PropertyFocus::Opacity => "100".to_string(),
                    openpencil_shell_core::document::PropertyFocus::FillHex => panel
                        .snapshot
                        .fill
                        .map(color_to_hex)
                        .unwrap_or_else(|| "#FFFFFF".to_string()),
                    openpencil_shell_core::document::PropertyFocus::StrokeHex => panel
                        .snapshot
                        .stroke
                        .map(|s| color_to_hex(s.color))
                        .unwrap_or_else(|| "#000000".to_string()),
                    openpencil_shell_core::document::PropertyFocus::StrokeWidth => panel
                        .snapshot
                        .stroke
                        .map(|s| format!("{}", s.width.round() as i32))
                        .unwrap_or_else(|| "1".to_string()),
                };
                self.document.ui.property_focus = Some(focus);
                self.document.ui.property_input_draft = initial;
                self.document.ui.property_caret_anchor_ms = self.now_ms;
                // No select-all-on-focus.
                self.document.ui.property_draft_select_all = false;
                self.document.chat.focused = false;
                return true;
            }
        }

        // 0d. VariablesPanel row click — body in property_dispatch.
        if self.dispatch_variables_panel_press(x, y, viewport_width, viewport_height) {
            return true;
        }

        // 1. AI chat panel — sits on top of the toolbar in paint
        //    order, so any click inside its rect is consumed
        //    here. DragHandle starts a chat drag; other AI hits
        //    defer to apply_click for focus/send/example/toggle.
        if let Some(chat_rect) = self.ai_chat_rect(viewport_width, viewport_height) {
            let panel = AIChatPlaceholder::from_document(&self.document);
            if let Some(hit) = panel.hit_test(chat_rect, Point2D::new(x, y)) {
                if matches!(hit, AIChatHit::DragHandle) {
                    self.chat_drag = Some(ChatDragState {
                        grab_dx: x - chat_rect.origin.x,
                        grab_dy: y - chat_rect.origin.y,
                        pos_x: chat_rect.origin.x,
                        pos_y: chat_rect.origin.y,
                    });
                    self.document.chat.focused = false;
                    return true;
                }
                // Non-drag chat hit: route through apply_click +
                // short-circuit so we never fall through to the
                // toolbar (which is below the chat panel).
                let _ = self.apply_click(x, y, viewport_width, viewport_height);
                return true;
            }
        }

        // 2. Toolbar — second-highest overlay. A click anywhere
        //    inside its bounding rect is consumed (gaps + padding
        //    too) so it never falls through to the canvas. The
        //    toolbar's x anchor follows `canvas_region`, so when
        //    the sidebar is collapsed it shifts left along with
        //    the canvas (codex stop-hook fix: "collapsed-sidebar
        //    interactions break").
        let (cx0, _cy0, _cw, _ch) = self.canvas_region(viewport_width, viewport_height);
        let toolbar = Toolbar::for_document(&self.document);
        let toolbar_h = toolbar
            .layout(&LayoutCx {
                available_width: TOOLBAR_WIDTH,
                dpi: 1.0,
            })
            .rect
            .size
            .y;
        let toolbar_rect = Rect {
            origin: Point2D::new(cx0 + TOOLBAR_INSET_X, TOP_BAR_HEIGHT + TOOLBAR_INSET_Y),
            size: Point2D::new(TOOLBAR_WIDTH, toolbar_h),
        };
        if rect_contains(toolbar_rect, Point2D::new(x, y)) {
            if let Some(hit) = toolbar.hit_test(toolbar_rect, Point2D::new(x, y)) {
                match hit {
                    openpencil_shell_core::widgets::ToolbarHit::Tool(tool) => {
                        let _ = self.document.finish_pen_path();
                        self.document.tool = tool;
                        self.document.ui.shape_picker_open = false; self.document.ui.shape_picker_hover = None;
                        return true;
                    }
                    openpencil_shell_core::widgets::ToolbarHit::Action(action) => {
                        use openpencil_shell_core::widgets::ToolbarAction;
                        self.document.ui.shape_picker_open = false; self.document.ui.shape_picker_hover = None;
                        let acted = match action {
                            ToolbarAction::Undo => self.document.undo(),
                            ToolbarAction::Redo => self.document.redo(),
                            _ => false,
                        };
                        return acted || rename_committed || text_edit_committed;
                    }
                    openpencil_shell_core::widgets::ToolbarHit::ToggleShapePicker => {
                        self.document.ui.shape_picker_open = !self.document.ui.shape_picker_open;
                        return true;
                    }
                }
            }
            return rename_committed || text_edit_committed;
        }

        if let Some(a) = self.align_toolbar_hit(x, y, viewport_width, viewport_height) {
            self.document.align_selected(a);
            return true;
        }
        // 3. apply_click — LayerPanel + chat-defocus.
        //    Before forwarding to apply_click, peek at the LayerPanel
        //    hit-test ourselves: if the press lands on a Layer row
        //    (with the sidebar open), seed a `layer_drag` candidate
        //    so a subsequent cursor_move past the activation
        //    threshold can promote it to a drag-to-reorder. The
        //    candidate doesn't change apply_click semantics — the
        //    row still selects on press; release decides whether to
        //    fall back to click or commit the reorder.
        if self.document.ui.sidebar_open {
            use openpencil_shell_core::widgets::{LayerPanel, LayerPanelHit, TOP_BAR_HEIGHT};
            let layer_rect = openpencil_shell_core::Rect {
                origin: openpencil_shell_core::Point2D::new(0.0, TOP_BAR_HEIGHT),
                size: openpencil_shell_core::Point2D::new(
                    self.document.ui.layer_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            let panel = LayerPanel::from_document(&self.document);
            if let Some(LayerPanelHit::Layer(node_id)) =
                panel.hit_test(layer_rect, openpencil_shell_core::Point2D::new(x, y))
            {
                self.layer_drag = Some(crate::widget_host::LayerDragState {
                    source: node_id,
                    start_y: y,
                    current_x: x,
                    current_y: y,
                    active: false,
                });
            }
        }
        let consumed = self.apply_click(x, y, viewport_width, viewport_height);
        if consumed {
            return true;
        }

        // 4. Canvas click — branch on the active tool.
        //    - Hand: pan-drag.
        //    - Select: handle-drag → node-drag → pan/deselect.
        //    - Shape tools: spawn a new node + drag-to-size.
        if self.over_canvas(x, y, viewport_width, viewport_height) {
            use openpencil_shell_core::document::Tool;
            if matches!(self.document.tool, Tool::Hand) {
                self.drag = Some(DragState {
                    last_x: x,
                    last_y: y,
                });
                return rename_committed || text_edit_committed;
            }
            let (cx0, cy0, cw, ch) = self.canvas_region(viewport_width, viewport_height);
            let canvas_rect = Rect {
                origin: Point2D::new(cx0, cy0),
                size: Point2D::new(cw, ch),
            };
            let canvas_local = Point2D::new(x - cx0, y - cy0);
            let doc_point = self.document.viewport.to_document(canvas_local);

            if matches!(self.document.tool, Tool::Select) {
                if let Some(handle) =
                    selection_handle_at_point(canvas_rect, &self.document, Point2D::new(x, y))
                {
                    if let Some(node) = self.document.selected_node() {
                        // Handles are painted from the node's
                        // aggregate bounds (so container handles
                        // surround the child union). For nodes
                        // with real `bounds` (leaves + bounded
                        // Frames) we resize against their own
                        // rect — `set_selected_bounds` writes
                        // straight to `node.bounds`. Unbounded
                        // containers (`Rect::ZERO`) would have
                        // their semantics flipped from "use
                        // children's bounds" to a concrete rect
                        // on the very first resize, and we don't
                        // yet recursively scale descendants — so
                        // skip handle-drag on those for now and
                        // fall through to node-drag / pan.
                        let raw = node.bounds;
                        if raw.size.x > 0.0 || raw.size.y > 0.0 {
                            self.document.commit_history();
                            self.handle_drag = Some(HandleDragState {
                                handle,
                                start_screen_x: x,
                                start_screen_y: y,
                                start_bounds: raw,
                            });
                            return true;
                        }
                    }
                }
                if rotation_corner_at_point(canvas_rect, &self.document, Point2D::new(x, y))
                    .is_some()
                {
                    if let Some(node) = self.document.selected_node() {
                        let bounds = node.aggregate_bounds();
                        let cx_doc = bounds.origin.x + bounds.size.x / 2.0;
                        let cy_doc = bounds.origin.y + bounds.size.y / 2.0;
                        let center_screen_x = canvas_rect.origin.x
                            + self.document.viewport.pan_x
                            + cx_doc * self.document.viewport.zoom;
                        let center_screen_y = canvas_rect.origin.y
                            + self.document.viewport.pan_y
                            + cy_doc * self.document.viewport.zoom;
                        let start_cursor_angle = (y - center_screen_y).atan2(x - center_screen_x);
                        let start_rotation = node.rotation;
                        self.document.commit_history();
                        self.rotate_drag = Some(RotateDragState {
                            center_screen_x,
                            center_screen_y,
                            start_cursor_angle,
                            start_rotation,
                        });
                        return true;
                    }
                }
                if let Some(node_id) = self.document.node_at_doc_point(doc_point) {
                    // Canvas double-click: 400 ms same-node → enter
                    // text-edit on Text nodes (no-op on others).
                    let is_double = matches!(
                        self.document.ui.last_canvas_click,
                        Some((prev, t)) if prev == node_id && self.now_ms.saturating_sub(t) < 400
                    );
                    self.document.ui.last_canvas_click = Some((node_id, self.now_ms));
                    if is_double && !text_edit_was_active && self.document.start_text_edit(node_id)
                    {
                        self.document.ui.text_edit_caret_anchor_ms = self.now_ms;
                        return true;
                    }
                    if self.shift_held {
                        // Shift+click toggles set membership.
                        // Only start a node-drag if the click
                        // ADDED the node (so the user can
                        // immediately drag the new selection
                        // together with the existing set).
                        let was_in_set = self.document.is_selected(node_id);
                        self.document.toggle_selection(node_id);
                        if !was_in_set {
                            self.node_drag = Some(NodeDragState {
                                last_screen_x: x,
                                last_screen_y: y,
                            });
                        }
                        return true;
                    }
                    // Plain click: when the click lands on an
                    // already-selected node within a multi-set,
                    // KEEP the set (TS parity — clicking inside
                    // a selection starts a multi-node drag).
                    // Otherwise collapse to single-select.
                    let already_in_set = self.document.is_selected(node_id);
                    if !already_in_set || self.document.selection_count() == 1 {
                        self.document.set_single_selection(node_id);
                    }
                    self.document.commit_history();
                    self.node_drag = Some(NodeDragState {
                        last_screen_x: x,
                        last_screen_y: y,
                    });
                    return true;
                }
                // Empty canvas press. Shift+click on empty does
                // NOT clear the set (TS parity — only plain
                // empty-click deselects).
                // Empty canvas with Select tool — start a marquee.
                // Plain-press clears the existing selection up front
                // so the marquee starts from empty; shift-press
                // preserves the existing set and toggles on release.
                // A zero-area marquee (no drag, just click) cleanly
                // collapses to "deselect-on-click".
                let cleared_now = if !self.shift_held {
                    let was_set = !self.document.selected_set.is_empty();
                    if was_set {
                        self.document.clear_selection();
                    }
                    was_set
                } else {
                    false
                };
                self.marquee_drag = Some(super::MarqueeDragState {
                    start_screen_x: x,
                    start_screen_y: y,
                    current_screen_x: x,
                    current_screen_y: y,
                    additive: self.shift_held,
                });
                return cleared_now;
            }

            // Pen tool: click on an existing anchor of the selected
            // Path → start anchor drag (edit). Otherwise click-add
            // anchors to an in-progress Path (author).
            if matches!(self.document.tool, Tool::Pen) {
                if self.document.ui.pen_in_progress.is_none() {
                    if let Some((node_id, anchor_index)) =
                        self.path_anchor_hit(x, y, viewport_width, viewport_height)
                    {
                        // Capture starting position so release knows
                        // whether the anchor actually moved (codex
                        // CONCERN — a no-motion click was polluting
                        // the undo stack).
                        let start_doc = self
                            .document
                            .active_page()
                            .and_then(|p| p.find(node_id))
                            .and_then(|n| n.points.get(anchor_index).copied())
                            .unwrap_or(doc_point);
                        let pre = self.document.snapshot_for_history();
                        self.path_anchor_drag = Some(super::PathAnchorDragState {
                            node_id,
                            anchor_index,
                            start_doc,
                            moved: false,
                            pre_drag_snapshot: pre,
                        });
                        return true;
                    }
                }
                if self.document.ui.pen_in_progress.is_some() {
                    self.document.add_pen_point(doc_point);
                } else {
                    let _ = self
                        .document
                        .start_pen_path(&mut self.next_node_id, doc_point);
                }
                return true;
            }

            // Shape / Frame / Text tool: spawn a new node at the
            // press point and drag-resize via cursor-move.
            let pre_create = self.document.snapshot_for_history();
            if let Some(node_id) = self.create_node_for_active_tool(doc_point) {
                self.document.history_push_past(pre_create);
                self.document.set_single_selection(node_id);
                self.create_drag = Some(CreateDragState {
                    start_doc_x: doc_point.x,
                    start_doc_y: doc_point.y,
                });
                return true;
            }

            // Tool didn't accept this point — fall back to pan.
            self.drag = Some(DragState {
                last_x: x,
                last_y: y,
            });
            return rename_committed || text_edit_committed;
        }
        rename_committed || text_edit_committed
    }

    /// Spawn a fresh document node for the active shape/frame/text
    /// tool at `doc_point`. Returns the new node's id when the
    /// tool maps to a creatable kind; `None` for Select / Hand.
    pub(in crate::widget_host) fn create_node_for_active_tool(
        &mut self,
        doc_point: Point2D,
    ) -> Option<openpencil_shell_core::document::NodeId> {
        use openpencil_shell_core::document::{Node, NodeId, NodeKind, Tool};
        use openpencil_shell_core::Color;
        let body_fill = Color {
            r: 0.74,
            g: 0.78,
            b: 0.85,
            a: 1.0,
        };
        let (kind, name, fill, stroke): (NodeKind, &str, Option<Color>, Option<(Color, f32)>) =
            match self.document.tool {
                Tool::Rect => (NodeKind::Rect, "Rectangle", Some(body_fill), None),
                Tool::Ellipse => (NodeKind::Ellipse, "Ellipse", Some(body_fill), None),
                Tool::Polygon => (NodeKind::Polygon, "Polygon", Some(body_fill), None),
                Tool::Line => (NodeKind::Line, "Line", None, Some((Color::BLACK, 2.0))),
                Tool::Pen => (NodeKind::Line, "Path", None, Some((Color::BLACK, 2.0))),
                Tool::Frame => (NodeKind::Frame, "Frame", Some(Color::WHITE), None),
                Tool::Text => (NodeKind::Text, "Text", None, None),
                _ => return None,
            };
        // Allocator-collision guard — mirror the duplicate path so
        // a document loaded with ids ≥ next_node_id (or near
        // u64::MAX) can't silently mint a colliding id.
        let safe = self.document.max_node_id().checked_add(1)?;
        self.next_node_id = self.next_node_id.max(safe);
        let id = NodeId::new(self.next_node_id);
        self.next_node_id = self.next_node_id.checked_add(1)?;
        // Sensible default size for click-create (no drag). Text
        // gets enough room to render its placeholder "Text" string;
        // shape tools start at 1 px so a real drag overrides
        // immediately. (codex stop-hook fix: text was inheriting
        // the shape 1×1 default, painting an unselectable speck.)
        let (init_w, init_h) = if matches!(kind, NodeKind::Text) {
            (96.0_f32, 24.0_f32)
        } else {
            (1.0, 1.0)
        };
        let mut node = Node::leaf(id.raw(), kind, name).with_bounds(Rect::xywh(
            doc_point.x,
            doc_point.y,
            init_w,
            init_h,
        ));
        if let Some(c) = fill {
            node = node.with_fill(c);
        }
        if let Some((sc, sw)) = stroke {
            node = node.with_stroke(sc, sw);
        }
        if matches!(self.document.tool, Tool::Text) {
            node = node.with_text("Text");
        }
        let page = self
            .document
            .pages
            .get_mut(self.document.active_page_index)?;
        page.children.push(node);
        Some(id)
    }

    fn dispatch_agent_settings_press(&mut self, x: f32, y: f32, vw: f32, vh: f32) -> bool {
        use openpencil_shell_core::widgets::agent_settings_panel::{
            AgentSettingsHit, AgentSettingsPanel,
        };
        let panel = AgentSettingsPanel::for_document(&self.document);
        let panel_rect = panel.rect(vw, vh);
        let point = Point2D::new(x, y);
        match panel.hit_test(panel_rect, point) {
            AgentSettingsHit::Close | AgentSettingsHit::Outside => {
                self.commit_settings_focus_if_any();
                self.document.ui.agent_settings_open = false;
                self.document.ui.agent_settings_drag = None;
            }
            AgentSettingsHit::SelectTab(t) => {
                self.commit_settings_focus_if_any();
                self.document.ui.agent_settings.tab = t;
                self.document.ui.agent_settings.scroll_y = 0.0;
            }
            AgentSettingsHit::Connect(p) => {
                let idx = openpencil_shell_core::document::AgentProvider::ALL.iter().position(|x| *x == p).unwrap_or(0);
                self.document.ui.agent_settings.connected[idx] ^= true;
            }
            AgentSettingsHit::ToggleMcpServer => {
                self.document.ui.agent_settings.mcp_server.running ^= true;
            }
            AgentSettingsHit::ToggleMcpCli(cli) => {
                let idx = openpencil_shell_core::document::McpCli::ALL.iter().position(|x| *x == cli).unwrap_or(0);
                self.document.ui.agent_settings.mcp_cli_enabled[idx] ^= true;
            }
            AgentSettingsHit::ToggleImagesAdvanced => {
                self.document.ui.agent_settings.images_advanced_open ^= true;
            }
            AgentSettingsHit::FocusMcpPort => {
                self.commit_settings_focus_if_any();
                self.document.ui.agent_settings.focus = Some(openpencil_shell_core::document::SettingsFocus::McpPort);
                self.document.ui.settings_input_draft = self.document.ui.agent_settings.mcp_server.port.to_string();
            }
            AgentSettingsHit::AddProvider | AgentSettingsHit::AddAcpAgent | AgentSettingsHit::TestImageSearch | AgentSettingsHit::AddGenConfig | AgentSettingsHit::Inside => {}
        }
        true
    }
}
