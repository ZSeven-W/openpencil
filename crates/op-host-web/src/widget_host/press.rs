//! Web `apply_press` + `apply_click` + `apply_right_press` —
//! extracted from `widget_host.rs` so the spine stays under the
//! 800-line cap. Mirrors the native `widget_host/press.rs` +
//! `click.rs` split.
//!
//! `EditorState` is the host's source of truth. Every widget
//! the canvas hit-test runs against the layout-resolved `LayoutScene`,
//! refreshed at the top of each input handler); the shell-core hit
//! results (`NodeId` / hit enums) are translated into op-editor-core
//! widget hit-test results before feeding `EditorState`
//! mutators.

use op_editor_ui::widgets::{
    AIChatHit, AIChatPlaceholder, LayerPanel, LayerPanelHit, LocalePicker, PropertyPanel, Toolbar,
    TopBar, TopBarHit, TOP_BAR_HEIGHT,
};
use op_editor_ui::{Point2D, Rect};

use super::{
    rect_contains, ChatDragState, DragState, LayerDragState, MarqueeDragState, WidgetHost,
};

impl WidgetHost {
    /// Right-click handler — opens the LayerPanel context menu on
    /// a layer or page row.
    pub fn apply_right_press(&mut self, x: f32, y: f32, _viewport_w: f32, viewport_h: f32) -> bool {
        if !self.editor_state.editor_ui.sidebar_open {
            return false;
        }
        use op_editor_core::editor_ui_state::LayerContextMenuState;
        use op_editor_core::ui_draft::LayerContextTarget;
        self.refresh_layout_scene();
        let layer_rect = self.layer_panel_rect(viewport_h);
        let panel = LayerPanel::from_editor(&self.editor_state);
        match panel.hit_test(layer_rect, Point2D::new(x, y)) {
            Some(LayerPanelHit::Layer(id)) => {
                let ec_id = id.clone();
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

    fn dispatch_layer_context_action(
        &mut self,
        action: op_editor_ui::widgets::layer_context_menu::LayerContextAction,
        target: op_editor_core::ui_draft::LayerContextTarget,
    ) {
        use op_editor_core::ui_draft::LayerContextTarget as T;
        use op_editor_ui::widgets::layer_context_menu::LayerContextAction as A;
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
            (A::CreateComponent, T::Layer(_)) => {}
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
                let _ = self.editor_state.start_rename_page(idx);
            }
            (A::RenameLayer, T::Layer(id)) => {
                let _ = self.editor_state.start_rename_layer(id);
            }
            _ => {}
        }
        self.mark_dirty();
    }

    pub fn apply_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        // Cache the viewport dims so `apply_cursor_move(x, y)` (no
        // viewport params in signature) can rebuild the canvas region
        // for the floating align toolbar's hover sync. Mirrors the
        // native host's `last_viewport_w` / `_h` cache.
        self.last_viewport_w = viewport_width;
        self.last_viewport_h = viewport_height;
        // Refresh the derived paint doc once up front — every hit-test
        // below reads `&self.layout_scene`, so it must be current.
        self.refresh_layout_scene();
        // 0-pre. Commit any in-flight rename + canvas text-edit on
        // first press anywhere. Tracked so the final return reports
        // the visible change.
        let rename_committed =
            self.editor_state.ui.layer_rename.is_some() && self.editor_state.rename_commit();
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
        // 0. Layer context menu — top-most overlay when open.
        if let Some(state) = self.editor_state.editor_ui.layer_context_menu.clone() {
            use op_editor_ui::widgets::layer_context_menu::LayerContextMenu;
            let menu = LayerContextMenu::for_state(&self.editor_state, state.clone());
            if let Some(action) = menu.hit_test(Point2D::new(x, y)) {
                self.dispatch_layer_context_action(action, state.target);
                self.editor_state.editor_ui.layer_context_menu = None;
                self.mark_dirty();
                return true;
            }
            self.editor_state.editor_ui.layer_context_menu = None;
            self.mark_dirty();
            return true;
        }
        // 0a. Locale picker overlay — top-most when open. Row hit
        //     sets locale + closes; ANY other hit (including the
        //     Globe button itself) closes the picker AND swallows
        //     the click so the same press doesn't re-toggle open.
        if self.editor_state.editor_ui.locale_picker_open {
            let panel_rect = self.locale_picker_rect(viewport_width);
            let picker = LocalePicker::for_editor_ui(&self.editor_state.editor_ui);
            if let Some(locale) = picker.hit_test(panel_rect, Point2D::new(x, y)) {
                self.editor_state.editor_ui.locale = locale;
                self.editor_state.editor_ui.locale_picker_open = false;
                self.mark_dirty();
                return true;
            }
            self.editor_state.editor_ui.locale_picker_open = false;
            self.mark_dirty();
            return true;
        }

        // 0b. TopBar — sidebar toggle + chrome buttons. Mirrors the
        //     native host so web + native behave identically.
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
                }
                TopBarHit::ToggleTheme => {
                    self.editor_state.editor_ui.theme_mode =
                        self.editor_state.editor_ui.theme_mode.flipped();
                }
                TopBarHit::ToggleLocale => {
                    let v = &mut self.editor_state.editor_ui.locale_picker_open;
                    *v = !*v;
                }
                TopBarHit::OpenAgentSettings => {
                    self.editor_state.editor_ui.agent_settings_open = true;
                }
                TopBarHit::ToggleFileMenu => {
                    self.editor_state.editor_ui.file_menu_open ^= true;
                }
                TopBarHit::OpenFigmaImport => {
                    self.editor_state.editor_ui.figma_import_open = true;
                }
            }
            self.mark_dirty();
            return true;
        }
        if rect_contains(top_bar_rect, Point2D::new(x, y)) {
            return rename_committed || text_edit_committed;
        }

        // 0c0b. Export scale / format inline select popup —
        //       outside-click dismiss. A click on a popup row or a
        //       dropdown toggle is applied; any other click closes
        //       both pickers and is swallowed. Mirrors the native
        //       host's `0c0b` block.
        if self.editor_state.editor_ui.export_scale_picker_open
            || self.editor_state.editor_ui.export_format_picker_open
        {
            if let Some(panel) = PropertyPanel::for_selection(&self.editor_state) {
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
                if let Some(action) = panel.hit_test_action(property_rect, Point2D::new(x, y)) {
                    if matches!(
                        action,
                        op_editor_ui::widgets::PropertyPanelAction::SetExportScale(_)
                            | op_editor_ui::widgets::PropertyPanelAction::SetExportFormat(_)
                            | op_editor_ui::widgets::PropertyPanelAction::ToggleExportScalePicker
                            | op_editor_ui::widgets::PropertyPanelAction::ToggleExportFormatPicker
                    ) {
                        self.apply_property_action(action);
                        return true;
                    }
                }
            }
            self.editor_state.editor_ui.export_scale_picker_open = false;
            self.editor_state.editor_ui.export_format_picker_open = false;
            self.mark_dirty();
            return true;
        }

        // 0c. PropertyPanel button / checkbox — flex modes + size
        //     flags. Runs AFTER locale picker + TopBar so the
        //     dropdown overlays still win.
        if let Some(panel) = PropertyPanel::for_selection(&self.editor_state) {
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
            if let Some(action) = panel.hit_test_action(property_rect, Point2D::new(x, y)) {
                self.apply_property_action(action);
                return true;
            }
        }

        // 1. AI chat panel — painted on top of toolbar so a
        //    click inside its rect is consumed here, even when
        //    that point lies inside the toolbar rect underneath.
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

        // 2. Toolbar — second-highest overlay. Bounding rect
        //    consumes all clicks (gaps + padding too) so it
        //    never falls through to the canvas for tool gaps
        //    that lie outside the chat panel.
        let toolbar_rect = self.toolbar_rect(viewport_width);
        let toolbar = Toolbar::for_editor(&self.editor_state);
        if rect_contains(toolbar_rect, Point2D::new(x, y)) {
            if let Some(hit) = toolbar.hit_test(toolbar_rect, Point2D::new(x, y)) {
                match hit {
                    op_editor_ui::widgets::ToolbarHit::Tool(tool) => {
                        self.editor_state.tool = tool;
                        self.editor_state.editor_ui.shape_picker_open = false;
                        self.mark_dirty();
                        return true;
                    }
                    op_editor_ui::widgets::ToolbarHit::Action(action) => {
                        use op_editor_ui::widgets::ToolbarAction;
                        self.editor_state.editor_ui.shape_picker_open = false;
                        let acted = match action {
                            ToolbarAction::Undo => self.editor_state.undo(),
                            ToolbarAction::Redo => self.editor_state.redo(),
                            _ => false,
                        };
                        if acted {
                            self.mark_dirty();
                        }
                        return acted || rename_committed;
                    }
                    op_editor_ui::widgets::ToolbarHit::ToggleShapePicker => {
                        let v = &mut self.editor_state.editor_ui.shape_picker_open;
                        *v = !*v;
                        self.mark_dirty();
                        return true;
                    }
                }
            }
            return rename_committed || text_edit_committed;
        }

        // 3. apply_click — LayerPanel + chat-defocus.
        //    Pre-seed a `layer_drag` candidate when the press lands
        //    on a Layer row so a subsequent move past the threshold
        //    promotes the gesture to a drag-to-reorder.
        if self.editor_state.editor_ui.sidebar_open {
            let layer_rect = self.layer_panel_rect(viewport_height);
            let panel = LayerPanel::from_editor(&self.editor_state);
            if let Some(LayerPanelHit::Layer(node_id)) =
                panel.hit_test(layer_rect, Point2D::new(x, y))
            {
                self.layer_drag = Some(LayerDragState {
                    source: node_id,
                    start_y: y,
                    current_x: x,
                    current_y: y,
                    active: false,
                });
            }
        }
        // 2.5. Floating align/distribute toolbar — visible when
        //      2+ nodes are selected. Hit-tested before apply_click
        //      so the visible button always wins over a layer row
        //      that happens to share screen y (matches native order).
        {
            use op_editor_ui::widgets::AlignToolbar;
            let (acx, _, acw, ach) = self.canvas_region(viewport_width, viewport_height);
            let canvas_region = op_editor_ui::Rect {
                origin: Point2D::new(acx, TOP_BAR_HEIGHT),
                size: Point2D::new(acw, ach),
            };
            if let Some(action) = AlignToolbar::for_canvas_region(canvas_region, &self.editor_state)
                .and_then(|tb| tb.hit_test(Point2D::new(x, y)))
            {
                let ec_action = action;
                self.editor_state.align_selected(ec_action);
                self.mark_dirty();
                return true;
            }
        }

        if self.apply_click(x, y, viewport_width, viewport_height) {
            return true;
        }

        // 4. Canvas click — branch on tool.
        //    - Hand: pan-drag.
        //    - Select + node hit: set/toggle selection.
        //    - Select + empty: marquee.
        if self.over_canvas(x, y, viewport_width, viewport_height) {
            if matches!(self.editor_state.tool, op_editor_core::Tool::Hand) {
                self.drag = Some(DragState {
                    last_x: x,
                    last_y: y,
                });
                return rename_committed || text_edit_committed;
            }
            if matches!(self.editor_state.tool, op_editor_core::Tool::Select) {
                // Convert screen → doc to ask which node (if any)
                // is under the cursor — `node_at_doc_point` queries
                // the layout-resolved render scene.
                let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_width, viewport_height);
                let canvas_local = Point2D::new(x - cx0, y - cy0);
                let doc_point = self.editor_state.viewport.to_document(canvas_local);
                let hit = self
                    .layout_scene
                    .node_at_doc_point(doc_point, self.editor_state.viewport.zoom);
                if let Some(sc_node_id) = hit {
                    let node_id = op_editor_core::NodeId::new(&sc_node_id);
                    // Canvas double-click: 400 ms same-node → enter
                    // text-edit on Text nodes.
                    let is_double = matches!(
                        self.editor_state.editor_ui.last_canvas_click.clone(),
                        Some((prev, t))
                            if prev == node_id && self.now_ms.saturating_sub(t) < 400
                    );
                    self.editor_state.editor_ui.last_canvas_click =
                        Some((node_id.clone(), self.now_ms));
                    if is_double
                        && !text_edit_was_active
                        && self.editor_state.start_text_edit(node_id.clone())
                    {
                        self.editor_state.ui.text_edit_caret_anchor_ms = self.now_ms;
                        self.mark_dirty();
                        return true;
                    }
                    if self.shift_held {
                        self.editor_state.toggle_selection(node_id);
                    } else {
                        let already_in_set = self.editor_state.is_selected(&node_id);
                        if !already_in_set || self.editor_state.selection_count() == 1 {
                            self.editor_state.set_single_selection(node_id);
                        }
                    }
                    self.mark_dirty();
                    return true;
                }
                // Empty canvas with Select → marquee.
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
                self.marquee_drag = Some(MarqueeDragState {
                    start_screen_x: x,
                    start_screen_y: y,
                    current_screen_x: x,
                    current_screen_y: y,
                    additive: self.shift_held,
                });
                return cleared_now || rename_committed || text_edit_committed;
            }
            // Any other tool on empty canvas — fall back to pan
            // (web doesn't ship shape-creation drag yet).
            self.drag = Some(DragState {
                last_x: x,
                last_y: y,
            });
            return rename_committed || text_edit_committed;
        }
        rename_committed || text_edit_committed
    }

    pub fn apply_click(&mut self, x: f32, y: f32, viewport_w: f32, viewport_h: f32) -> bool {
        // glue:
        // Floating chat panel sits on top — check first so its
        // clicks don't fall through to the canvas.
        self.refresh_layout_scene();
        if let Some(chat_rect) = self.ai_chat_rect(viewport_w, viewport_h) {
            let panel = AIChatPlaceholder::from_editor(&self.editor_state);
            if let Some(hit) = panel.hit_test(chat_rect, Point2D::new(x, y)) {
                match hit {
                    AIChatHit::FocusInput => {
                        self.editor_state.chat.focused = true;
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::Send => {
                        // Web keeps the offline echo stub.
                        self.editor_state.chat.send();
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::Example(text) => {
                        self.editor_state.chat.input = text;
                        self.editor_state.chat.focused = true;
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::DragHandle => {
                        return false;
                    }
                    AIChatHit::ToggleCollapse => {
                        let v = &mut self.editor_state.chat.collapsed;
                        *v = !*v;
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::ToggleModelPicker => {
                        let v = &mut self.editor_state.editor_ui.chat_model_picker_open;
                        *v = !*v;
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::SelectModel(idx) => {
                        self.editor_state.select_chat_model(idx);
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::CycleThinking => {
                        self.editor_state.chat.cycle_thinking_mode();
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::CycleEffort => {
                        self.editor_state.chat.cycle_effort_level();
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::AddAttachment => {
                        // The web shell has no native file picker wired
                        // yet — staging an attachment is a desktop-only
                        // path for now. No-op so the click is consumed.
                        return true;
                    }
                    AIChatHit::RemoveAttachment(idx) => {
                        self.editor_state.chat.remove_attachment(idx);
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::ToggleThinking(idx) => {
                        self.editor_state.chat.toggle_message_thinking(idx);
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::ToggleToolCalls(idx) => {
                        self.editor_state.chat.toggle_message_tool_calls(idx);
                        self.mark_dirty();
                        return true;
                    }
                }
            }
        }
        // Click outside the chat panel closes the model picker.
        let picker_was_open = self.editor_state.editor_ui.chat_model_picker_open;
        self.editor_state.editor_ui.chat_model_picker_open = false;
        let was_focused = self.editor_state.chat.focused || picker_was_open;
        self.editor_state.chat.focused = false;
        self.mark_dirty();

        let toolbar_rect = self.toolbar_rect(viewport_w);
        let toolbar = Toolbar::for_editor(&self.editor_state);
        if let Some(hit) = toolbar.hit_test(toolbar_rect, Point2D::new(x, y)) {
            match hit {
                op_editor_ui::widgets::ToolbarHit::Tool(tool) => {
                    self.editor_state.tool = tool;
                    self.mark_dirty();
                    return true;
                }
                op_editor_ui::widgets::ToolbarHit::Action(action) => {
                    use op_editor_ui::widgets::ToolbarAction;
                    let acted = match action {
                        ToolbarAction::Undo => self.editor_state.undo(),
                        ToolbarAction::Redo => self.editor_state.redo(),
                        _ => false,
                    };
                    if acted {
                        self.mark_dirty();
                    }
                    return acted;
                }
                op_editor_ui::widgets::ToolbarHit::ToggleShapePicker => {
                    let v = &mut self.editor_state.editor_ui.shape_picker_open;
                    *v = !*v;
                    self.mark_dirty();
                    return true;
                }
            }
        }
        if !self.editor_state.editor_ui.sidebar_open {
            return was_focused;
        }
        let layer_rect = self.layer_panel_rect(viewport_h);
        let panel = LayerPanel::from_editor(&self.editor_state);
        if let Some(hit) = panel.hit_test(layer_rect, Point2D::new(x, y)) {
            match hit {
                LayerPanelHit::Page(idx) => {
                    let _ = self.editor_state.set_active_page(idx);
                    self.editor_state.clear_selection();
                    self.mark_dirty();
                    return true;
                }
                LayerPanelHit::Layer(node_id) => {
                    let ec_id = node_id.clone();
                    if self.shift_held {
                        self.editor_state.toggle_selection(ec_id);
                    } else {
                        self.editor_state.set_single_selection(ec_id);
                    }
                    self.mark_dirty();
                    return true;
                }
                LayerPanelHit::ToggleHidden(node_id) => {
                    self.editor_state.toggle_node_hidden(&node_id.clone());
                    self.mark_dirty();
                    return true;
                }
                LayerPanelHit::ToggleLocked(node_id) => {
                    self.editor_state.toggle_node_locked(&node_id.clone());
                    self.mark_dirty();
                    return true;
                }
                LayerPanelHit::ToggleCollapsed(node_id) => {
                    self.editor_state.toggle_node_collapsed(&node_id.clone());
                    self.mark_dirty();
                    return true;
                }
                LayerPanelHit::AddPage => {
                    let _ = self.editor_state.add_page();
                    self.mark_dirty();
                    return true;
                }
                LayerPanelHit::DeletePage(idx) => {
                    let _ = self.editor_state.remove_page(idx);
                    self.mark_dirty();
                    return true;
                }
            }
        }
        // Defocusing the chat input itself is a visible change —
        // the caller should still repaint to drop the caret.
        was_focused
    }

    /// Cmd+, settings modal — dispatch hit-tests on the modal.
    /// Returns true once the modal swallowed the press.
    fn dispatch_agent_settings_press(&mut self, x: f32, y: f32, vw: f32, vh: f32) -> bool {
        use op_editor_ui::widgets::agent_settings_panel::{AgentSettingsHit, AgentSettingsPanel};
        self.refresh_layout_scene();
        let panel = AgentSettingsPanel::for_editor(&self.editor_state);
        let panel_rect = panel.rect(vw, vh);
        match panel.hit_test(panel_rect, Point2D::new(x, y)) {
            AgentSettingsHit::Close | AgentSettingsHit::Outside => {
                self.commit_settings_focus();
                self.editor_state.editor_ui.agent_settings_open = false;
            }
            AgentSettingsHit::SelectTab(t) => {
                self.commit_settings_focus();
                self.editor_state.editor_ui.agent_settings.tab = t;
                self.editor_state.editor_ui.agent_settings.scroll_y = 0.0;
            }
            AgentSettingsHit::Connect(p) => {
                // `connected` is indexed by `AgentProvider::ALL` order.
                let idx = op_editor_core::agent_settings::AgentProvider::ALL
                    .iter()
                    .position(|x| *x == p)
                    .unwrap_or(0);
                self.editor_state.editor_ui.agent_settings.connected[idx] ^= true;
                // Re-derive the chat model picker for the new mask.
                self.editor_state.rebuild_chat_models();
            }
            AgentSettingsHit::ToggleMcpServer => {
                self.editor_state
                    .editor_ui
                    .agent_settings
                    .mcp_server
                    .running ^= true;
            }
            AgentSettingsHit::ToggleMcpCli(cli) => {
                // `mcp_cli_enabled` is indexed by `McpCli::ALL` order.
                let idx = op_editor_core::agent_settings::McpCli::ALL
                    .iter()
                    .position(|x| *x == cli)
                    .unwrap_or(0);
                self.editor_state.editor_ui.agent_settings.mcp_cli_enabled[idx] ^= true;
            }
            AgentSettingsHit::ToggleImagesAdvanced => {
                self.editor_state
                    .editor_ui
                    .agent_settings
                    .images_advanced_open ^= true;
            }
            AgentSettingsHit::FocusMcpPort => {
                self.commit_settings_focus();
                self.editor_state.editor_ui.agent_settings.focus =
                    Some(op_editor_core::agent_settings::SettingsFocus::McpPort);
                self.editor_state.editor_ui.settings_input_draft = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .mcp_server
                    .port
                    .to_string();
            }
            AgentSettingsHit::AddProvider
            | AgentSettingsHit::AddAcpAgent
            | AgentSettingsHit::TestImageSearch
            | AgentSettingsHit::AddGenConfig
            | AgentSettingsHit::Inside => {}
        }
        self.mark_dirty();
        true
    }
}
