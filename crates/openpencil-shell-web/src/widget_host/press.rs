//! Web `apply_press` + `apply_click` — extracted from
//! `widget_host.rs` so the spine stays under the 800-line cap.
//! Mirrors the native `widget_host/press.rs` split.

use openpencil_shell_core::document::Tool;
use openpencil_shell_core::widgets::{
    AIChatHit, AIChatPlaceholder, LayerPanel, LayerPanelHit, LocalePicker, PropertyPanel, Toolbar,
    TopBar, TopBarHit, AI_CHAT_COLLAPSED_HEIGHT, AI_CHAT_HEIGHT, LOCALE_PICKER_WIDTH,
    TOOLBAR_WIDTH, TOP_BAR_HEIGHT,
};
use openpencil_shell_core::{Point2D, Rect};

use super::{
    rect_contains, ChatDragState, DragState, LayerDragState, MarqueeDragState, WidgetHost,
};

impl WidgetHost {
    /// Right-click handler — opens the LayerPanel context menu on
    /// a layer or page row.
    pub fn apply_right_press(&mut self, x: f32, y: f32, _viewport_w: f32, viewport_h: f32) -> bool {
        if !self.document.ui.sidebar_open {
            return false;
        }
        use openpencil_shell_core::document::{LayerContextMenuState, LayerContextTarget};
        let layer_rect = self.layer_panel_rect(viewport_h);
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
            (A::ToggleLock, T::Layer(id)) => self.document.toggle_node_locked(id),
            (A::ToggleVisibility, T::Layer(id)) => self.document.toggle_node_hidden(id),
            (A::CreateComponent, T::Layer(_)) => {}
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
                let _ = self.document.start_rename_page(idx);
            }
            (A::RenameLayer, T::Layer(id)) => {
                let _ = self.document.start_rename_layer(id);
            }
            _ => {}
        }
    }

    pub fn apply_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        // 0-pre. Commit any in-flight rename + canvas text-edit on
        // first press anywhere. Tracked so the final return reports
        // the visible change.
        let rename_committed =
            self.document.ui.layer_rename.is_some() && self.document.rename_commit();
        let text_edit_was_active = self.document.ui.text_editing.is_some();
        let text_edit_committed = self.document.text_edit_commit();
        if self.document.ui.agent_settings_open
            && self.dispatch_agent_settings_press(x, y, viewport_width, viewport_height)
        {
            return true;
        }
        // 0. Layer context menu — top-most overlay when open.
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
        // 0a. Locale picker overlay — top-most when open. Row hit
        //     sets locale + closes; ANY other hit (including the
        //     Globe button itself) closes the picker AND swallows
        //     the click so the same press doesn't re-toggle open.
        if self.document.ui.locale_picker_open {
            let panel_rect = self.locale_picker_rect(viewport_width);
            let picker = LocalePicker::for_document(&self.document);
            if let Some(locale) = picker.hit_test(panel_rect, Point2D::new(x, y)) {
                self.document.ui.locale = locale;
                self.document.ui.locale_picker_open = false;
                return true;
            }
            self.document.ui.locale_picker_open = false;
            return true;
        }

        // 0b. TopBar — sidebar toggle button. Mirrors the native
        //     host so web + native behave identically.
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
                TopBarHit::OpenAgentSettings => {
                    self.document.ui.agent_settings_open = true;
                    return true;
                }
                TopBarHit::ToggleFileMenu => {
                    self.document.ui.file_menu_open ^= true;
                    return true;
                }
                TopBarHit::OpenFigmaImport => {
                    self.document.ui.figma_import_open = true;
                    return true;
                }
            }
        }
        if rect_contains(top_bar_rect, Point2D::new(x, y)) {
            return rename_committed || text_edit_committed;
        }

        // 0c. PropertyPanel button / checkbox — flex modes + size
        //     flags. Runs AFTER locale picker + TopBar so the
        //     dropdown overlays still win (codex stop-hook fix:
        //     "web property-panel action hit-test intercepts the
        //     locale picker").
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
                self.apply_property_action(action);
                return true;
            }
        }

        // 1. AI chat panel — painted on top of toolbar so a
        //    click inside its rect is consumed here, even when
        //    that point lies inside the toolbar rect underneath.
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
                let _ = self.apply_click(x, y, viewport_width, viewport_height);
                return true;
            }
        }

        // 2. Toolbar — second-highest overlay. Bounding rect
        //    consumes all clicks (gaps + padding too) so it
        //    never falls through to the canvas for tool gaps
        //    that lie outside the chat panel.
        let toolbar_rect = self.toolbar_rect(viewport_width);
        let toolbar = Toolbar::for_document(&self.document);
        if rect_contains(toolbar_rect, Point2D::new(x, y)) {
            if let Some(hit) = toolbar.hit_test(toolbar_rect, Point2D::new(x, y)) {
                match hit {
                    openpencil_shell_core::widgets::ToolbarHit::Tool(tool) => {
                        self.document.tool = tool;
                        self.document.ui.shape_picker_open = false;
                        return true;
                    }
                    openpencil_shell_core::widgets::ToolbarHit::Action(action) => {
                        use openpencil_shell_core::widgets::ToolbarAction;
                        self.document.ui.shape_picker_open = false;
                        let acted = match action {
                            ToolbarAction::Undo => self.document.undo(),
                            ToolbarAction::Redo => self.document.redo(),
                            _ => false,
                        };
                        return acted || rename_committed;
                    }
                    openpencil_shell_core::widgets::ToolbarHit::ToggleShapePicker => {
                        self.document.ui.shape_picker_open = !self.document.ui.shape_picker_open;
                        return true;
                    }
                }
            }
            return rename_committed || text_edit_committed;
        }

        // 3. apply_click — LayerPanel + chat-defocus.
        //    Pre-seed a `layer_drag` candidate when the press lands
        //    on a Layer row so a subsequent move past the threshold
        //    promotes the gesture to a drag-to-reorder (mirrors
        //    native; see `widget_host/press.rs`).
        if self.document.ui.sidebar_open {
            let layer_rect = self.layer_panel_rect(viewport_height);
            let panel = LayerPanel::from_document(&self.document);
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
        if self.apply_click(x, y, viewport_width, viewport_height) {
            return true;
        }

        // 4. Canvas click — branch on tool.
        //    - Hand: pan-drag.
        //    - Select + node hit: set/toggle selection.
        //    - Select + empty: marquee.
        if self.over_canvas(x, y, viewport_width, viewport_height) {
            if matches!(self.document.tool, Tool::Hand) {
                self.drag = Some(DragState {
                    last_x: x,
                    last_y: y,
                });
                return rename_committed || text_edit_committed;
            }
            if matches!(self.document.tool, Tool::Select) {
                // Convert screen → doc to ask which node (if any)
                // is under the cursor.
                let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_width, viewport_height);
                let canvas_local = Point2D::new(x - cx0, y - cy0);
                let doc_point = self.document.viewport.to_document(canvas_local);
                if let Some(node_id) = self.document.node_at_doc_point(doc_point) {
                    // Canvas double-click: 400 ms same-node → enter
                    // text-edit on Text nodes.
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
                        self.document.toggle_selection(node_id);
                    } else {
                        let already_in_set = self.document.is_selected(node_id);
                        if !already_in_set || self.document.selection_count() == 1 {
                            self.document.set_single_selection(node_id);
                        }
                    }
                    return true;
                }
                // Empty canvas with Select → marquee.
                let cleared_now = if !self.shift_held {
                    let was_set = !self.document.selected_set.is_empty();
                    if was_set {
                        self.document.clear_selection();
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
        if let Some(chat_rect) = self.ai_chat_rect(viewport_w, viewport_h) {
            let panel = AIChatPlaceholder::from_document(&self.document);
            if let Some(hit) = panel.hit_test(chat_rect, Point2D::new(x, y)) {
                match hit {
                    AIChatHit::FocusInput => {
                        self.document.chat.focused = true;
                        return true;
                    }
                    AIChatHit::Send => {
                        self.document.chat.send();
                        return true;
                    }
                    AIChatHit::Example(text) => {
                        self.document.chat.input = text;
                        self.document.chat.focused = true;
                        return true;
                    }
                    AIChatHit::DragHandle => {
                        return false;
                    }
                    AIChatHit::ToggleCollapse => {
                        self.document.chat.collapsed = !self.document.chat.collapsed;
                        return true;
                    }
                }
            }
        }
        let was_focused = self.document.chat.focused;
        self.document.chat.focused = false;

        let toolbar_rect = self.toolbar_rect(viewport_w);
        let toolbar = Toolbar::for_document(&self.document);
        if let Some(hit) = toolbar.hit_test(toolbar_rect, Point2D::new(x, y)) {
            match hit {
                openpencil_shell_core::widgets::ToolbarHit::Tool(tool) => {
                    self.document.tool = tool;
                    return true;
                }
                openpencil_shell_core::widgets::ToolbarHit::Action(action) => {
                    use openpencil_shell_core::widgets::ToolbarAction;
                    return match action {
                        ToolbarAction::Undo => self.document.undo(),
                        ToolbarAction::Redo => self.document.redo(),
                        _ => false,
                    };
                }
                openpencil_shell_core::widgets::ToolbarHit::ToggleShapePicker => {
                    self.document.ui.shape_picker_open = !self.document.ui.shape_picker_open;
                    return true;
                }
            }
        }
        if !self.document.ui.sidebar_open {
            return was_focused;
        }
        let layer_rect = self.layer_panel_rect(viewport_h);
        let panel = LayerPanel::from_document(&self.document);
        if let Some(hit) = panel.hit_test(layer_rect, Point2D::new(x, y)) {
            match hit {
                openpencil_shell_core::widgets::LayerPanelHit::Page(idx) => {
                    self.document.active_page_index = idx;
                    self.document.clear_selection();
                    return true;
                }
                openpencil_shell_core::widgets::LayerPanelHit::Layer(node_id) => {
                    if self.shift_held {
                        self.document.toggle_selection(node_id);
                    } else {
                        self.document.set_single_selection(node_id);
                    }
                    return true;
                }
                openpencil_shell_core::widgets::LayerPanelHit::ToggleHidden(node_id) => {
                    self.document.toggle_node_hidden(node_id);
                    return true;
                }
                openpencil_shell_core::widgets::LayerPanelHit::ToggleLocked(node_id) => {
                    self.document.toggle_node_locked(node_id);
                    return true;
                }
                openpencil_shell_core::widgets::LayerPanelHit::ToggleCollapsed(node_id) => {
                    self.document.toggle_node_collapsed(node_id);
                    return true;
                }
                openpencil_shell_core::widgets::LayerPanelHit::AddPage => {
                    let _ = self.document.add_page();
                    return true;
                }
                openpencil_shell_core::widgets::LayerPanelHit::DeletePage(idx) => {
                    let _ = self.document.remove_page(idx);
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
        use openpencil_shell_core::widgets::agent_settings_panel::{AgentSettingsHit, AgentSettingsPanel};
        let panel = AgentSettingsPanel::for_document(&self.document);
        let panel_rect = panel.rect(vw, vh);
        match panel.hit_test(panel_rect, Point2D::new(x, y)) {
            AgentSettingsHit::Close | AgentSettingsHit::Outside => {
                self.commit_settings_focus();
                self.document.ui.agent_settings_open = false;
            }
            AgentSettingsHit::SelectTab(t) => {
                self.commit_settings_focus();
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
                self.commit_settings_focus();
                self.document.ui.agent_settings.focus = Some(openpencil_shell_core::document::SettingsFocus::McpPort);
                self.document.ui.settings_input_draft = self.document.ui.agent_settings.mcp_server.port.to_string();
            }
            AgentSettingsHit::AddProvider | AgentSettingsHit::AddAcpAgent | AgentSettingsHit::TestImageSearch | AgentSettingsHit::AddGenConfig | AgentSettingsHit::Inside => {}
        }
        true
    }
}
