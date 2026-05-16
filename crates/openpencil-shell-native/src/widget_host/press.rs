//! Mouse-press dispatcher.
//!
//! `EditorState` is the host's source of truth. Every hit-test runs
//! against the derived paint `Document` (`paint_doc`, refreshed at
//! the top of `apply_press`); the shell-core hit results are
//! translated into op-editor-core types before feeding mutators.
//! Press-helper methods (`create_node_for_active_tool`,
//! `dispatch_agent_settings_press`) live in `press_helpers.rs`.

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
        target: op_editor_core::ui_draft::LayerContextTarget,
    ) {
        use op_editor_core::ui_draft::LayerContextTarget as T;
        use openpencil_shell_core::widgets::layer_context_menu::LayerContextAction as A;
        match (action, target) {
            (A::Duplicate, T::Layer(id)) => {
                self.editor_state.set_single_selection(id);
                self.editor_state.commit_history();
                let _ = self
                    .editor_state
                    .duplicate_selected(&mut self.next_node_id, 10.0);
            }
            (A::Delete, T::Layer(id)) => {
                self.editor_state.set_single_selection(id);
                self.editor_state.commit_history();
                let _ = self.editor_state.delete_selected();
            }
            (A::ToggleLock, T::Layer(id)) => {
                self.editor_state.toggle_node_locked(&id);
            }
            (A::ToggleVisibility, T::Layer(id)) => {
                self.editor_state.toggle_node_hidden(&id);
            }
            (A::CreateComponent, T::Layer(_)) => {} // stub
            (A::DuplicatePage, T::Page(idx)) => {
                let _ = self.editor_state.duplicate_page(idx);
            }
            (A::MovePageUp, T::Page(idx)) => {
                let _ = self.editor_state.move_page_up(idx);
            }
            (A::MovePageDown, T::Page(idx)) => {
                let _ = self.editor_state.move_page_down(idx);
            }
            (A::DeletePage, T::Page(idx)) => {
                let _ = self.editor_state.remove_page(idx);
            }
            (A::RenamePage, T::Page(idx)) => {
                if self.editor_state.start_rename_page(idx) {
                    self.editor_state.editor_ui.rename_caret_anchor_ms = self.now_ms;
                }
            }
            (A::RenameLayer, T::Layer(id)) => {
                if self.editor_state.start_rename_layer(id) {
                    self.editor_state.editor_ui.rename_caret_anchor_ms = self.now_ms;
                }
            }
            _ => {} // mismatched action/target — no-op
        }
        self.mark_dirty();
    }

    /// Right-click handler — opens the LayerPanel context menu on
    /// a layer row OR page row.
    pub fn apply_right_press(&mut self, x: f32, y: f32, _viewport_w: f32, viewport_h: f32) -> bool {
        // Codex stop-gate: right-click outside the variables panel
        // must commit any pending row focus first.
        self.commit_variable_row_focus_if_any();
        if !self.editor_state.editor_ui.sidebar_open {
            return false;
        }
        use op_editor_core::editor_ui_state::LayerContextMenuState;
        use op_editor_core::ui_draft::LayerContextTarget;
        use openpencil_shell_core::widgets::{LayerPanel, LayerPanelHit};
        self.refresh_paint_doc();
        let layer_rect = Rect {
            origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
            size: Point2D::new(
                self.editor_state.editor_ui.layer_panel_width,
                (viewport_h - TOP_BAR_HEIGHT).max(0.0),
            ),
        };
        let panel = LayerPanel::from_document(&self.paint_doc);
        match panel.hit_test(layer_rect, Point2D::new(x, y)) {
            Some(LayerPanelHit::Layer(id)) => {
                let ec_id = op_pen_loader::rev::node_id(&id);
                self.editor_state.set_single_selection(ec_id.clone());
                self.editor_state.editor_ui.layer_context_menu = Some(LayerContextMenuState {
                    target: LayerContextTarget::Layer(ec_id),
                    anchor_x: x,
                    anchor_y: y,
                    hovered_row: None,
                });
                self.mark_dirty();
                return true;
            }
            Some(LayerPanelHit::Page(idx)) => {
                self.editor_state.editor_ui.layer_context_menu = Some(LayerContextMenuState {
                    target: LayerContextTarget::Page(idx),
                    anchor_x: x,
                    anchor_y: y,
                    hovered_row: None,
                });
                self.mark_dirty();
                return true;
            }
            _ => {}
        }
        if self
            .editor_state
            .editor_ui
            .layer_context_menu
            .take()
            .is_some()
        {
            self.mark_dirty();
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
        let rename_committed = self.editor_state.ui.layer_rename.is_some()
            && self.editor_state.rename_commit();
        let text_edit_was_active = self.editor_state.ui.text_editing.is_some();
        let text_edit_committed = self.editor_state.text_edit_commit();
        if rename_committed || text_edit_committed {
            self.mark_dirty();
        }
        if self.editor_state.editor_ui.agent_settings_open
            && self.dispatch_agent_settings_press(x, y, viewport_width, viewport_height)
        {
            return true;
        }
        // 0-color. Color picker overlay — top-most when open.
        if let Some(state) = self.editor_state.ui.color_picker.clone() {
            use openpencil_shell_core::widgets::color_picker::{
                drag_for_hit, ColorPicker, ColorPickerHit,
            };
            let picker = ColorPicker::for_state(&self.editor_state, state.clone());
            let panel = picker.rect(viewport_width, viewport_height);
            let point = Point2D::new(x, y);
            match picker.hit_test(panel, point) {
                Some(ColorPickerHit::Close) => {
                    let _ = self.editor_state.close_color_picker();
                    self.mark_dirty();
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
                                let _ = self
                                    .editor_state
                                    .color_picker_set_hsv(state.hue, s, v);
                            }
                            ColorPickerHit::HueSlider => {
                                let h = picker.hue_at(panel, point);
                                let _ = self
                                    .editor_state
                                    .color_picker_set_hsv(h, state.sat, state.val);
                            }
                            _ => {}
                        }
                        // `drag_for_hit` returns op-editor-core's
                        // `ColorPickerDrag` — no translation needed.
                        self.editor_state.color_picker_set_drag(Some(kind));
                    }
                    self.mark_dirty();
                    return true;
                }
                None => {
                    // Press outside the panel — close it; the click
                    // continues to the next overlay.
                    let _ = self.editor_state.close_color_picker();
                    self.mark_dirty();
                }
            }
        }

        if let Some(state) = self.editor_state.editor_ui.layer_context_menu.clone() {
            use openpencil_shell_core::widgets::layer_context_menu::LayerContextMenu;
            let menu = LayerContextMenu::for_state(&self.editor_state, state.clone());
            if let Some(action) = menu.hit_test(Point2D::new(x, y)) {
                self.dispatch_layer_context_action(action, state.target.clone());
                self.editor_state.editor_ui.layer_context_menu = None;
                self.mark_dirty();
                return true;
            }
            self.editor_state.editor_ui.layer_context_menu = None;
            self.mark_dirty();
            return true;
        }
        // 0aa. Commit-on-blur for property-panel inputs +
        // variable-row inline editor.
        self.commit_variable_row_focus_if_any();
        if self.editor_state.ui.property_focus.is_some() {
            let property_left = if self.editor_state.property_panel_visible() {
                viewport_width - self.editor_state.editor_ui.property_panel_width
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
                    PanelResizeKind::LayerRight => {
                        self.editor_state.editor_ui.layer_panel_width
                    }
                    PanelResizeKind::PropertyLeft => {
                        self.editor_state.editor_ui.property_panel_width
                    }
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
        if self.editor_state.editor_ui.shape_picker_open {
            self.refresh_paint_doc();
            let panel_rect = self.shape_picker_rect(viewport_width, viewport_height);
            let picker = ShapePicker::for_editor_ui(&self.editor_state.editor_ui);
            if let Some(choice) = picker.hit_test(panel_rect, Point2D::new(x, y)) {
                match choice {
                    ShapeChoice::Tool(tool) => {
                        let _ = self.editor_state.finish_pen_path();
                        let ec_tool = op_pen_loader::rev::tool(tool);
                        self.editor_state.editor_ui.shape_tool = ec_tool;
                        self.editor_state.tool = ec_tool;
                    }
                    ShapeChoice::OpenIconPicker | ShapeChoice::ImportImageOrSvg => {
                        // Host-side dispatch lands when the icon
                        // picker / file dialog widgets ship.
                    }
                }
                self.editor_state.editor_ui.shape_picker_open = false;
                self.editor_state.editor_ui.shape_picker_hover = None;
                self.mark_dirty();
                return true;
            }
            self.editor_state.editor_ui.shape_picker_open = false;
            self.editor_state.editor_ui.shape_picker_hover = None;
            self.mark_dirty();
            return true;
        }

        if self.editor_state.editor_ui.file_menu_open {
            self.dispatch_file_menu_press(x, y, viewport_width);
            return true;
        }
        if self.editor_state.editor_ui.export_dialog_open {
            self.dispatch_export_dialog_press(x, y, viewport_width, viewport_height);
            return true;
        }
        if self.editor_state.editor_ui.figma_import_open {
            self.dispatch_figma_import_press(x, y, viewport_width, viewport_height);
            return true;
        }

        // 0a. Locale picker overlay — top-most when open.
        if self.editor_state.editor_ui.locale_picker_open {
            self.refresh_paint_doc();
            let panel_rect = self.locale_picker_rect(viewport_width);
            let picker = LocalePicker::for_editor_ui(&self.editor_state.editor_ui);
            if let Some(locale) = picker.hit_test(panel_rect, Point2D::new(x, y)) {
                self.editor_state.editor_ui.locale = op_pen_loader::rev::locale(locale);
                self.editor_state.editor_ui.locale_picker_open = false;
                self.editor_state.editor_ui.locale_picker_hover = None;
                self.mark_dirty();
                return true;
            }
            self.editor_state.editor_ui.locale_picker_open = false;
            self.editor_state.editor_ui.locale_picker_hover = None;
            self.mark_dirty();
            return true;
        }

        // 0b. TopBar — sidebar toggle button + theme + locale picker.
        self.refresh_paint_doc();
        let top_bar_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_width, TOP_BAR_HEIGHT),
        };
        let top_bar = TopBar::for_editor_ui(&self.editor_state.editor_ui);
        if let Some(hit) = top_bar.hit_test(top_bar_rect, Point2D::new(x, y)) {
            match hit {
                TopBarHit::ToggleSidebar => {
                    let v = &mut self.editor_state.editor_ui.sidebar_open;
                    *v = !*v;
                    self.mark_dirty();
                    return true;
                }
                TopBarHit::ToggleTheme => {
                    self.editor_state.editor_ui.theme_mode =
                        self.editor_state.editor_ui.theme_mode.flipped();
                    self.mark_dirty();
                    return true;
                }
                TopBarHit::ToggleLocale => {
                    let v = &mut self.editor_state.editor_ui.locale_picker_open;
                    *v = !*v;
                    self.mark_dirty();
                    return true;
                }
                TopBarHit::OpenAgentSettings => {
                    self.editor_state.editor_ui.agent_settings_open = true;
                    self.mark_dirty();
                    return true;
                }
                TopBarHit::ToggleFileMenu => {
                    self.editor_state.editor_ui.file_menu_open ^= true;
                    self.mark_dirty();
                    return true;
                }
                TopBarHit::OpenFigmaImport => {
                    self.editor_state.editor_ui.figma_import_open = true;
                    self.mark_dirty();
                    return true;
                }
            }
        }
        if rect_contains(top_bar_rect, Point2D::new(x, y)) {
            // Other top-bar gaps eat clicks but don't act.
            return rename_committed || text_edit_committed;
        }

        // 0c0. Fill-type picker — outside-click dismiss.
        if self.editor_state.editor_ui.fill_type_picker_open {
            self.refresh_paint_doc();
            if let Some(panel) = PropertyPanel::for_selection(&self.paint_doc) {
                let property_rect = Rect {
                    origin: Point2D::new(
                        viewport_width - self.editor_state.editor_ui.property_panel_width,
                        TOP_BAR_HEIGHT,
                    ),
                    size: Point2D::new(
                        self.editor_state.editor_ui.property_panel_width,
                        (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                    ),
                };
                if let Some(action) =
                    panel.hit_test_action(property_rect, Point2D::new(x, y))
                {
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
            self.editor_state.editor_ui.fill_type_picker_open = false;
            self.mark_dirty();
            return true;
        }

        // 0b1. VariablesPanel — tested before PropertyPanel.
        if self.dispatch_variables_panel_press(x, y, viewport_width, viewport_height) {
            return true;
        }

        // 0c. PropertyPanel input row.
        self.refresh_paint_doc();
        if let Some(panel) = PropertyPanel::for_selection(&self.paint_doc) {
            let property_rect = Rect {
                origin: Point2D::new(
                    viewport_width - self.editor_state.editor_ui.property_panel_width,
                    TOP_BAR_HEIGHT,
                ),
                size: Point2D::new(
                    self.editor_state.editor_ui.property_panel_width,
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
                    let _ = self.editor_state.open_color_picker(
                        super::press_helpers::color_target(target),
                        y,
                    );
                    self.mark_dirty();
                } else {
                    self.apply_property_action(action);
                }
                return true;
            }
            if let Some(focus) = panel.hit_test(property_rect, Point2D::new(x, y)) {
                self.commit_property_focus_if_any();
                let initial =
                    super::press_helpers::property_focus_initial(focus, &panel);
                // shell-core `PropertyFocus` → op-editor-core.
                self.editor_state.ui.property_focus =
                    Some(op_pen_loader::rev::property_focus(focus));
                self.editor_state.ui.property_input_draft = initial;
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
                self.editor_state.ui.property_draft_select_all = false;
                self.editor_state.chat.focused = false;
                self.mark_dirty();
                return true;
            }
        }

        // 1. AI chat panel — sits on top of the toolbar in paint
        //    order. DragHandle starts a chat drag; other AI hits
        //    defer to apply_click.
        if let Some(chat_rect) = self.ai_chat_rect(viewport_width, viewport_height) {
            let panel = AIChatPlaceholder::from_editor(&self.editor_state);
            if let Some(hit) = panel.hit_test(chat_rect, Point2D::new(x, y)) {
                if matches!(hit, AIChatHit::DragHandle) {
                    self.chat_drag = Some(ChatDragState {
                        grab_dx: x - chat_rect.origin.x,
                        grab_dy: y - chat_rect.origin.y,
                        pos_x: chat_rect.origin.x,
                        pos_y: chat_rect.origin.y,
                    });
                    self.editor_state.chat.focused = false;
                    self.mark_dirty();
                    return true;
                }
                let _ = self.apply_click(x, y, viewport_width, viewport_height);
                return true;
            }
        }

        // 2. Toolbar — second-highest overlay.
        let (cx0, _cy0, _cw, _ch) = self.canvas_region(viewport_width, viewport_height);
        let toolbar = Toolbar::for_editor(&self.editor_state);
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
                        let _ = self.editor_state.finish_pen_path();
                        self.editor_state.tool = op_pen_loader::rev::tool(tool);
                        self.editor_state.editor_ui.shape_picker_open = false;
                        self.editor_state.editor_ui.shape_picker_hover = None;
                        self.mark_dirty();
                        return true;
                    }
                    openpencil_shell_core::widgets::ToolbarHit::Action(action) => {
                        use openpencil_shell_core::widgets::ToolbarAction;
                        self.editor_state.editor_ui.shape_picker_open = false;
                        self.editor_state.editor_ui.shape_picker_hover = None;
                        let acted = match action {
                            ToolbarAction::Undo => self.editor_state.undo(),
                            ToolbarAction::Redo => self.editor_state.redo(),
                            _ => false,
                        };
                        self.mark_dirty();
                        return acted || rename_committed || text_edit_committed;
                    }
                    openpencil_shell_core::widgets::ToolbarHit::ToggleShapePicker => {
                        let v = &mut self.editor_state.editor_ui.shape_picker_open;
                        *v = !*v;
                        self.mark_dirty();
                        return true;
                    }
                }
            }
            return rename_committed || text_edit_committed;
        }

        if let Some(a) = self.align_toolbar_hit(x, y, viewport_width, viewport_height) {
            self.editor_state.align_selected(op_pen_loader::rev::align_action(a));
            self.mark_dirty();
            return true;
        }
        // 3. apply_click — LayerPanel + chat-defocus. Peek the
        //    LayerPanel hit-test for a drag-to-reorder candidate.
        if self.editor_state.editor_ui.sidebar_open {
            use openpencil_shell_core::widgets::{LayerPanel, LayerPanelHit};
            let layer_rect = Rect {
                origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
                size: Point2D::new(
                    self.editor_state.editor_ui.layer_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            let panel = LayerPanel::from_document(&self.paint_doc);
            if let Some(LayerPanelHit::Layer(node_id)) =
                panel.hit_test(layer_rect, Point2D::new(x, y))
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
        if self.over_canvas(x, y, viewport_width, viewport_height) {
            use op_editor_core::Tool;
            self.refresh_paint_doc();
            if matches!(self.editor_state.tool, Tool::Hand) {
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
            let doc_point = self.editor_state.viewport.to_document(canvas_local);

            if matches!(self.editor_state.tool, Tool::Select) {
                if let Some(handle) =
                    selection_handle_at_point(canvas_rect, &self.paint_doc, Point2D::new(x, y))
                {
                    if let Some(node) = self.paint_doc.selected_node() {
                        // Only handle-drag on nodes with real bounds.
                        let raw = node.bounds;
                        if raw.size.x > 0.0 || raw.size.y > 0.0 {
                            self.editor_state.commit_history();
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
                if rotation_corner_at_point(
                    canvas_rect,
                    &self.paint_doc,
                    Point2D::new(x, y),
                )
                .is_some()
                {
                    if let Some(node) = self.paint_doc.selected_node() {
                        let bounds = node.aggregate_bounds();
                        let cx_doc = bounds.origin.x + bounds.size.x / 2.0;
                        let cy_doc = bounds.origin.y + bounds.size.y / 2.0;
                        let center_screen_x = canvas_rect.origin.x
                            + self.editor_state.viewport.pan_x
                            + cx_doc * self.editor_state.viewport.zoom;
                        let center_screen_y = canvas_rect.origin.y
                            + self.editor_state.viewport.pan_y
                            + cy_doc * self.editor_state.viewport.zoom;
                        let start_cursor_angle =
                            (y - center_screen_y).atan2(x - center_screen_x);
                        let start_rotation = node.rotation;
                        self.editor_state.commit_history();
                        self.rotate_drag = Some(RotateDragState {
                            center_screen_x,
                            center_screen_y,
                            start_cursor_angle,
                            start_rotation,
                        });
                        return true;
                    }
                }
                if let Some(node_id) = self.paint_doc.node_at_doc_point(doc_point) {
                    let ec_id = op_pen_loader::rev::node_id(&node_id);
                    // Canvas double-click: 400 ms same-node → enter
                    // text-edit on Text nodes.
                    let is_double = matches!(
                        &self.editor_state.editor_ui.last_canvas_click,
                        Some((prev, t)) if *prev == ec_id
                            && self.now_ms.saturating_sub(*t) < 400
                    );
                    self.editor_state.editor_ui.last_canvas_click =
                        Some((ec_id.clone(), self.now_ms));
                    if is_double
                        && !text_edit_was_active
                        && self.editor_state.start_text_edit(ec_id.clone())
                    {
                        self.editor_state.ui.text_edit_caret_anchor_ms = self.now_ms;
                        self.mark_dirty();
                        return true;
                    }
                    if self.shift_held {
                        // Shift+click toggles set membership.
                        let was_in_set = self.editor_state.is_selected(&ec_id);
                        self.editor_state.toggle_selection(ec_id.clone());
                        if !was_in_set {
                            self.node_drag = Some(NodeDragState {
                                last_screen_x: x,
                                last_screen_y: y,
                            });
                        }
                        self.mark_dirty();
                        return true;
                    }
                    // Plain click: keep a multi-set when clicking
                    // inside it (TS parity), else single-select.
                    let already_in_set = self.editor_state.is_selected(&ec_id);
                    if !already_in_set || self.editor_state.selection_count() == 1 {
                        self.editor_state.set_single_selection(ec_id);
                    }
                    self.editor_state.commit_history();
                    self.node_drag = Some(NodeDragState {
                        last_screen_x: x,
                        last_screen_y: y,
                    });
                    self.mark_dirty();
                    return true;
                }
                // Empty canvas press — start a marquee.
                let cleared_now = if !self.shift_held {
                    let was_set = !self.editor_state.selection.set.is_empty();
                    if was_set {
                        self.editor_state.clear_selection();
                        self.mark_dirty();
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

            // Pen tool: anchor edit / author.
            if matches!(self.editor_state.tool, Tool::Pen) {
                if self.editor_state.ui.pen_in_progress.is_none() {
                    if let Some((node_id, anchor_index)) =
                        self.path_anchor_hit(x, y, viewport_width, viewport_height)
                    {
                        let ec_id = op_pen_loader::rev::node_id(&node_id);
                        // Capture starting position for the
                        // history-pollution guard.
                        let start_doc = self
                            .paint_doc
                            .active_page()
                            .and_then(|p| p.find(&node_id))
                            .and_then(|n| n.points.get(anchor_index).copied())
                            .unwrap_or(doc_point);
                        let pre = self.editor_state.snapshot_for_history();
                        self.path_anchor_drag = Some(super::PathAnchorDragState {
                            node_id: ec_id,
                            anchor_index,
                            start_doc,
                            moved: false,
                            pre_drag_snapshot: pre,
                        });
                        return true;
                    }
                }
                if self.editor_state.ui.pen_in_progress.is_some() {
                    self.editor_state
                        .add_pen_point((doc_point.x as f64, doc_point.y as f64));
                } else {
                    let _ = self.editor_state.start_pen_path(
                        &mut self.next_node_id,
                        (doc_point.x as f64, doc_point.y as f64),
                    );
                }
                self.mark_dirty();
                return true;
            }

            // Shape / Frame / Text tool: spawn a new node + drag.
            let pre_create = self.editor_state.snapshot_for_history();
            if let Some(node_id) = self.create_node_for_active_tool(doc_point) {
                self.editor_state.history_push_past(pre_create);
                self.editor_state.set_single_selection(node_id);
                self.create_drag = Some(CreateDragState {
                    start_doc_x: doc_point.x,
                    start_doc_y: doc_point.y,
                });
                self.mark_dirty();
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
}

