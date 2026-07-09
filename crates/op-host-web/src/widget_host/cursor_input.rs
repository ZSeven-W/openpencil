//! Cursor-move dispatch for the web `WidgetHost` — canvas pan-drag,
//! marquee / layer / chat drags, and every hover wash (agent
//! settings, toolbar, top bar, status bar, chat, property panel,
//! code panel, align toolbar). Split out of `widget_host.rs` to keep
//! the spine under the repo's 800-line cap (mirrors the native
//! host's `widget_host/input.rs` split).

use op_editor_ui::widgets::TOP_BAR_HEIGHT;
use op_editor_ui::{Point2D, Rect};

use super::WidgetHost;

impl WidgetHost {
    // Cursor-move coalescing hint — tested + ready to wire; the CanvasKit mount
    // repaints every mousemove rather than scheduling deferred frames.
    #[allow(dead_code)]
    pub(crate) fn cursor_move_requires_immediate_frame(&self) -> bool {
        let color_picker_drag = self
            .editor_state
            .ui
            .color_picker
            .as_ref()
            .and_then(|state| state.drag)
            .is_some();
        self.variables_resize.is_some()
            || color_picker_drag
            || self.design_md_drag.is_some()
            || self.component_browser_drag.is_some()
            || self.icon_picker_drag.is_some()
            || self.code_selection_drag.is_some()
            || self.chat_input_selection_drag.is_some()
            || self.chat_text_selection_drag.is_some()
            || self.create_drag.is_some()
            || self.path_anchor_drag.is_some()
            || self.handle_drag.is_some()
            || self.node_drag.is_some()
            || self.marquee_drag.is_some()
            || self.layer_drag.is_some()
            || self.chat_drag.is_some()
            || self.image_adjustment_drag.is_some()
            || self.drag.is_some()
    }

    /// Sync every agent-settings hover flag from the cursor.
    /// Returns `true` when any hover state changed.
    pub(in crate::widget_host) fn update_agent_settings_hover(&mut self, x: f32, y: f32) -> bool {
        use op_editor_core::AgentSettingsTab;
        use op_editor_ui::widgets::agent_settings_panel::{AgentSettingsHit, AgentSettingsPanel};
        let point = Point2D::new(x, y);
        let (
            close_hover,
            server_hover,
            copy_hover,
            add_provider_hover,
            add_acp_hover,
            image_search_test_hover,
            image_search_register_link_hover,
            image_add_hover,
            image_profile_header_hover,
            image_profile_remove_hover,
            image_profile_provider_hover,
            image_profile_test_hover,
            image_provider_option_hover,
            new_hover,
        ) = {
            let panel = AgentSettingsPanel::for_web_editor(&self.editor_state);
            let panel_rect = panel.rect(self.last_viewport_w, self.last_viewport_h);
            let hit = panel.hit_test(panel_rect, point);
            let is_agents = matches!(
                self.editor_state.editor_ui.agent_settings.tab,
                AgentSettingsTab::Agents
            );
            let is_images = matches!(
                self.editor_state.editor_ui.agent_settings.tab,
                AgentSettingsTab::Images
            );
            let copy_hover = matches!(
                self.editor_state.editor_ui.agent_settings.tab,
                AgentSettingsTab::Mcp
            ) && matches!(hit, AgentSettingsHit::CopyMcpClientConfig);
            let close_hover = matches!(hit, AgentSettingsHit::Close);
            let server_hover = matches!(
                self.editor_state.editor_ui.agent_settings.tab,
                AgentSettingsTab::Mcp
            ) && matches!(hit, AgentSettingsHit::ToggleMcpServer);
            let add_provider_hover = is_agents && matches!(hit, AgentSettingsHit::AddProvider);
            let add_acp_hover = is_agents && matches!(hit, AgentSettingsHit::AddAcpAgent);
            let image_search_test_hover =
                is_images && panel.image_search_test_button_hover_at(panel_rect, point);
            let image_search_register_link_hover =
                is_images && matches!(hit, AgentSettingsHit::OpenImageRegisterLink);
            let image_add_hover =
                is_images && panel.image_gen_add_button_hover_at(panel_rect, point);
            let image_profile_header_hover = if is_images {
                match hit {
                    AgentSettingsHit::ToggleGenConfigEditor(index) => Some(index),
                    _ => None,
                }
            } else {
                None
            };
            let image_profile_remove_hover = if is_images {
                match hit {
                    AgentSettingsHit::RemoveGenConfig(index) => Some(index),
                    _ => None,
                }
            } else {
                None
            };
            let image_profile_provider_hover = if is_images {
                match hit {
                    AgentSettingsHit::ToggleGenProviderMenu(index) => Some(index),
                    _ => None,
                }
            } else {
                None
            };
            let image_profile_test_hover = if is_images {
                panel.image_gen_profile_test_button_hover_at(panel_rect, point)
            } else {
                None
            };
            let image_provider_option_hover = if is_images {
                match hit {
                    AgentSettingsHit::SelectGenProvider { index, provider } => {
                        Some((index, provider))
                    }
                    _ => None,
                }
            } else {
                None
            };
            let new_hover = panel.builtin_preset_hover_at(panel_rect, point);
            (
                close_hover,
                server_hover,
                copy_hover,
                add_provider_hover,
                add_acp_hover,
                image_search_test_hover,
                image_search_register_link_hover,
                image_add_hover,
                image_profile_header_hover,
                image_profile_remove_hover,
                image_profile_provider_hover,
                image_profile_test_hover,
                image_provider_option_hover,
                new_hover,
            )
        };
        let mut changed = false;
        if close_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_agent_settings_close
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_agent_settings_close = close_hover;
            changed = true;
        }
        if server_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_mcp_server_button
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_mcp_server_button = server_hover;
            changed = true;
        }
        if copy_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_mcp_client_config_copy
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_mcp_client_config_copy = copy_hover;
            changed = true;
        }
        if new_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .builtin_preset_menu_hover
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .builtin_preset_menu_hover = new_hover;
            changed = true;
        }
        if add_provider_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_add_provider
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_add_provider = add_provider_hover;
            changed = true;
        }
        if add_acp_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_add_acp_agent
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_add_acp_agent = add_acp_hover;
            changed = true;
        }
        if image_search_test_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_search_test_button
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_search_test_button = image_search_test_hover;
            changed = true;
        }
        if image_search_register_link_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_search_register_link
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_search_register_link = image_search_register_link_hover;
            changed = true;
        }
        if image_add_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_add_button
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_add_button = image_add_hover;
            changed = true;
        }
        if image_profile_header_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_profile_header
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_profile_header = image_profile_header_hover;
            changed = true;
        }
        if image_profile_remove_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_profile_remove
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_profile_remove = image_profile_remove_hover;
            changed = true;
        }
        if image_profile_provider_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_profile_provider
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_profile_provider = image_profile_provider_hover;
            changed = true;
        }
        if image_profile_test_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_profile_test
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_profile_test = image_profile_test_hover;
            changed = true;
        }
        if image_provider_option_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_provider_option
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_provider_option = image_provider_option_hover;
            changed = true;
        }
        if changed {
            self.mark_dirty();
        }
        changed
    }

    /// Cursor-move handler — drives canvas pan-drag, marquee /
    /// layer / chat / overlay drags, and the chrome hover washes.
    pub fn apply_cursor_move(&mut self, x: f32, y: f32) -> bool {
        // Refresh the derived paint doc once up front so every hit-test
        // below (layer context menu, layer drag, align toolbar) reads
        // current geometry, never a stale snapshot.
        self.refresh_layout_scene();
        if self.apply_path_anchor_drag_move(x, y) {
            return true;
        }
        // In-flight VariablesPanel edge resize — owns the cursor.
        if self.variables_resize.is_some()
            && self.apply_variables_panel_resize(x, y, self.last_viewport_w, self.last_viewport_h)
        {
            return true;
        }
        if self.editor_state.editor_ui.agent_settings_open && self.update_agent_settings_hover(x, y)
        {
            return true;
        }
        // Floating-overlay drags + hovers (colour picker, Design-MD /
        // Icon-picker / Component-Browser panels, open dropdowns) own
        // the cursor before lower context menus. This matches native:
        // a topmost panel covering a path-anchor / layer menu must
        // block that lower menu's hover wash.
        if self.apply_overlay_cursor_move(x, y) {
            return true;
        }
        if self.update_path_anchor_menu_hover(x, y) {
            return true;
        }
        if let Some(state) = self.editor_state.editor_ui.layer_context_menu.clone() {
            use op_editor_ui::widgets::layer_context_menu::LayerContextMenu;
            let menu = LayerContextMenu::for_state(&self.editor_state, state.clone());
            let new_hover = menu.hovered_row_at(Point2D::new(x, y));
            if new_hover != state.menu.hover {
                let mut next = state;
                next.menu.hover = new_hover;
                self.editor_state.editor_ui.layer_context_menu = Some(next);
                self.mark_dirty();
                return true;
            }
        }
        // Export-section select-popup row hover highlight.
        if self.editor_state.editor_ui.export_scale_picker_open
            || self.editor_state.editor_ui.export_format_picker_open
        {
            if let Some(panel) =
                op_editor_ui::widgets::PropertyPanel::for_selection(&self.editor_state)
            {
                let property_rect = Rect {
                    origin: Point2D::new(
                        self.last_viewport_w - self.editor_state.editor_ui.property_panel_width,
                        op_editor_ui::widgets::TOP_BAR_HEIGHT,
                    ),
                    size: Point2D::new(
                        self.editor_state.editor_ui.property_panel_width,
                        (self.last_viewport_h - op_editor_ui::widgets::TOP_BAR_HEIGHT).max(0.0),
                    ),
                };
                let new_hover = panel.export_picker_row_at(property_rect, Point2D::new(x, y));
                if new_hover != self.editor_state.editor_ui.export_picker_hover {
                    self.editor_state.editor_ui.export_picker_hover = new_hover;
                    self.mark_dirty();
                    return true;
                }
            }
        }
        if self.apply_chat_text_selection_drag_cursor_move(x, y) {
            return true;
        }
        if self.apply_chat_input_selection_drag_cursor_move(x, y) {
            return true;
        }
        if self.apply_code_selection_drag_cursor_move(x, y) {
            return true;
        }
        if let Some(consumed) = self.apply_node_drag_cursor_move(x, y) {
            return consumed;
        }
        if self.apply_selection_handle_drag_move(x, y) {
            return true;
        }
        if self.update_create_drag(x, y) {
            return true;
        }
        if let Some(m) = self.marquee_drag.as_mut() {
            m.current_screen_x = x;
            m.current_screen_y = y;
            return true;
        }
        if self.layer_drag.is_some() {
            // Drop the gesture if the source disappeared mid-drag —
            // see the native host for the rationale.
            let source_id = self.layer_drag.as_ref().unwrap().source.clone();
            let still_present = self
                .layout_scene
                .active_page()
                .map(|p| p.find(source_id.as_str()).is_some())
                .unwrap_or(false);
            if !still_present {
                self.layer_drag = None;
                return true;
            }
            let d = self.layer_drag.as_mut().unwrap();
            d.current_x = x;
            d.current_y = y;
            // VERTICAL-ONLY activation (4 px). See the native host
            // for the rationale: pure horizontal wiggle must not
            // steal click-feel from row-level gestures.
            if !d.active && (y - d.start_y).abs() > 4.0 {
                d.active = true;
            }
            return true;
        }
        if let Some(d) = self.chat_drag.as_mut() {
            d.pos_x = x - d.grab_dx;
            d.pos_y = y - d.grab_dy;
            return true;
        }
        if let Some(field) = self.image_adjustment_drag {
            if let Some(panel) =
                op_editor_ui::widgets::PropertyPanel::for_selection(&self.editor_state)
            {
                let property_rect = Rect {
                    origin: Point2D::new(
                        self.last_viewport_w - self.editor_state.editor_ui.property_panel_width,
                        op_editor_ui::widgets::TOP_BAR_HEIGHT,
                    ),
                    size: Point2D::new(
                        self.editor_state.editor_ui.property_panel_width,
                        (self.last_viewport_h - op_editor_ui::widgets::TOP_BAR_HEIGHT).max(0.0),
                    ),
                };
                if let Some(action) = panel.image_adjustment_drag_action(property_rect, field, x) {
                    self.apply_property_action(action);
                    return true;
                }
            }
        }
        if let Some(drag) = self.drag.as_mut() {
            let dx = x - drag.last_x;
            let dy = y - drag.last_y;
            drag.last_x = x;
            drag.last_y = y;
            self.editor_state.viewport.pan(dx, dy);
            // Canvas pan only translates the viewport; the layout-resolved
            // scene is document-space (camera applied at paint time), so keep
            // the layout cache intact — `mark_dirty()` here forced a full
            // serde reconversion of the document every move (matches native
            // `op-host-native/.../input.rs`). The listener still repaints off
            // this `true` return.
            return true;
        }
        // Left layer rail hover wash. CanvasKit mousemove now feeds the
        // same layer-row hover state the press / paint path already uses.
        if self.update_layer_hover(x, y, self.last_viewport_h) {
            return true;
        }
        // Toolbar per-button hover wash — AFTER drag detection so a
        // path-anchor / node / pan drag whose cursor crosses the
        // toolbar isn't intercepted by the hover update (mirrors
        // native widget_host/input.rs ordering).
        if self.update_toolbar_hover(x, y) {
            return true;
        }
        // Floating VariablesPanel hover wash — mirrors the native
        // host's hover sync against the SAME rect press dispatch uses.
        if self.editor_state.editor_ui.variables_panel_open {
            use op_editor_ui::widgets::variables_panel::VariablesPanel;
            let point = Point2D::new(x, y);
            if let Some(vars_rect) =
                self.variables_panel_rect(self.last_viewport_w, self.last_viewport_h)
            {
                if (vars_rect).contains(point) {
                    let new_hover = VariablesPanel::for_editor_at(&self.editor_state, self.now_ms)
                        .hover_at(vars_rect, point);
                    let changed = new_hover != self.editor_state.editor_ui.variables_panel_hover;
                    if changed {
                        self.editor_state.editor_ui.variables_panel_hover = new_hover;
                        self.mark_dirty();
                    }
                    return changed;
                }
            }
            if self
                .editor_state
                .editor_ui
                .variables_panel_hover
                .take()
                .is_some()
            {
                self.mark_dirty();
                return true;
            }
        } else if self
            .editor_state
            .editor_ui
            .variables_panel_hover
            .take()
            .is_some()
        {
            self.mark_dirty();
            return true;
        }
        // TopBar chrome-button hover wash (sidebar / file-menu / figma /
        // theme / locale / fullscreen / agent chip). Git and Preview are
        // compiled out on wasm32; every visible button lights up the same
        // as native. Reuses the click hit-test so paint can't drift.
        {
            let tb_rect = self.top_bar_rect(self.last_viewport_w);
            let new_hover = self
                .top_bar()
                .hit_test(tb_rect, Point2D::new(x, y))
                .map(op_editor_ui::widgets::editor_state_ext::topbar_button_hover);
            if new_hover != self.editor_state.editor_ui.topbar_button_hover {
                self.editor_state.editor_ui.topbar_button_hover = new_hover;
                self.mark_dirty();
                return true;
            }
        }
        // StatusBar control hover wash (search / zoom-out / zoom-in).
        {
            let new_hover = self
                .status_bar_rect(self.last_viewport_w, self.last_viewport_h)
                .and_then(|r| {
                    op_editor_ui::widgets::StatusBar::for_editor(&self.editor_state)
                        .control_at(r, Point2D::new(x, y))
                });
            if new_hover != self.editor_state.editor_ui.statusbar_hover {
                self.editor_state.editor_ui.statusbar_hover = new_hover;
                self.mark_dirty();
                return true;
            }
        }
        let over_topmost =
            self.over_topmost_panel(x, y, self.last_viewport_w, self.last_viewport_h);
        if self.update_chat_model_picker_hover(x, y, over_topmost) {
            return true;
        }
        // AI chat header buttons (chevron / maximize / new chat) hover.
        if let Some(chat_rect) = self.ai_chat_rect(self.last_viewport_w, self.last_viewport_h) {
            use op_editor_ui::widgets::AIChatPlaceholder;
            let new_hover = AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                .hit_test(chat_rect, Point2D::new(x, y))
                .as_ref()
                .and_then(op_editor_ui::widgets::editor_state_ext::chat_header_hover);
            if new_hover != self.editor_state.editor_ui.chat_header_hover {
                self.editor_state.editor_ui.chat_header_hover = new_hover;
                self.mark_dirty();
                return true;
            }
        }
        // AI chat tab row hover — drives the close-× visibility on each tab.
        // `tab_hover_at` returns None when collapsed or no tabs exist; the
        // else branch clears any stale index when the panel is not present.
        if let Some(chat_rect) = self.ai_chat_rect(self.last_viewport_w, self.last_viewport_h) {
            use op_editor_ui::widgets::AIChatPlaceholder;
            let new_hover = AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                .tab_hover_at(chat_rect, Point2D::new(x, y));
            if new_hover != self.editor_state.editor_ui.chat_tab_hover {
                self.editor_state.editor_ui.chat_tab_hover = new_hover;
                self.mark_dirty();
                return true;
            }
        } else if self.editor_state.editor_ui.chat_tab_hover.take().is_some() {
            self.mark_dirty();
            return true;
        }
        if let Some(chat_rect) = self.ai_chat_rect(self.last_viewport_w, self.last_viewport_h) {
            use op_editor_ui::widgets::AIChatPlaceholder;
            let new_hover = AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                .footer_hover_at(chat_rect, Point2D::new(x, y));
            if new_hover != self.editor_state.editor_ui.chat_footer_hover {
                self.editor_state.editor_ui.chat_footer_hover = new_hover;
                self.mark_dirty();
                return true;
            }
        } else if self
            .editor_state
            .editor_ui
            .chat_footer_hover
            .take()
            .is_some()
        {
            self.mark_dirty();
            return true;
        }
        // Parallel-agents picker row hover — drives the highlight wash inside the overlay.
        if let Some(chat_rect) = self.ai_chat_rect(self.last_viewport_w, self.last_viewport_h) {
            use op_editor_ui::widgets::AIChatPlaceholder;
            let new_hover = AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                .parallel_agents_picker_hover_at(chat_rect, Point2D::new(x, y));
            if new_hover != self.editor_state.editor_ui.parallel_agents_picker_hover {
                self.editor_state.editor_ui.parallel_agents_picker_hover = new_hover;
                self.mark_dirty();
                return true;
            }
        } else if self
            .editor_state
            .editor_ui
            .parallel_agents_picker_hover
            .take()
            .is_some()
        {
            self.mark_dirty();
            return true;
        }
        if let Some(chat_rect) = self.ai_chat_rect(self.last_viewport_w, self.last_viewport_h) {
            use op_editor_ui::widgets::AIChatPlaceholder;
            let new_hover = AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                .example_hover_at(chat_rect, Point2D::new(x, y));
            if new_hover != self.editor_state.editor_ui.chat_example_hover {
                self.editor_state.editor_ui.chat_example_hover = new_hover;
                self.mark_dirty();
                return true;
            }
        } else if self
            .editor_state
            .editor_ui
            .chat_example_hover
            .take()
            .is_some()
        {
            self.mark_dirty();
            return true;
        }
        if self.update_chat_design_hover(x, y, over_topmost) {
            return true;
        }
        // PropertyPanel tab/action hover wash. Shown with a selection.
        let mut property_hover_changed = false;
        let mut inside_property_panel = false;
        if self.editor_state.property_panel_visible() {
            use op_editor_ui::widgets::{PropertyPanel, TOP_BAR_HEIGHT};
            if let Some(panel) = PropertyPanel::for_selection(&self.editor_state) {
                let property_rect = Rect {
                    origin: Point2D::new(
                        self.last_viewport_w - self.editor_state.editor_ui.property_panel_width,
                        TOP_BAR_HEIGHT,
                    ),
                    size: Point2D::new(
                        self.editor_state.editor_ui.property_panel_width,
                        (self.last_viewport_h - TOP_BAR_HEIGHT).max(0.0),
                    ),
                };
                let point = Point2D::new(x, y);
                inside_property_panel = property_rect.contains(point);
                let new_tab_hover = panel.tab_hover_at(property_rect, point);
                if new_tab_hover != self.editor_state.editor_ui.property_tab_hover {
                    self.editor_state.editor_ui.property_tab_hover = new_tab_hover;
                    property_hover_changed = true;
                }
                let new_fill_type_hover = panel.fill_type_picker_row_at(property_rect, point);
                if new_fill_type_hover != self.editor_state.editor_ui.fill_type_picker.hover {
                    self.editor_state.editor_ui.fill_type_picker.hover = new_fill_type_hover;
                    property_hover_changed = true;
                }
                let new_action_hover = panel.action_hover_index(property_rect, point);
                if new_action_hover != self.editor_state.editor_ui.property_action_hover {
                    self.editor_state.editor_ui.property_action_hover = new_action_hover;
                    property_hover_changed = true;
                }
            }
        } else {
            let ui = &mut self.editor_state.editor_ui;
            property_hover_changed |= ui.property_tab_hover.take().is_some();
            property_hover_changed |= ui.fill_type_picker.hover.take().is_some();
        }
        // Code-panel hover wash. Reuses the panel's click geometry for
        // framework chips, scroll chevrons, and body actions.
        let (new_fw_hover, new_action_hover) = if self.editor_state.property_panel_visible()
            && matches!(
                self.editor_state.editor_ui.property_tab,
                op_editor_core::PropertyTab::Code
            ) {
            let pw = self.editor_state.editor_ui.property_panel_width;
            let panel_x = self.last_viewport_w - pw;
            let panel_rect = Rect {
                origin: Point2D::new(panel_x, TOP_BAR_HEIGHT),
                size: Point2D::new(pw, (self.last_viewport_h - TOP_BAR_HEIGHT).max(0.0)),
            };
            if x >= panel_x && x <= self.last_viewport_w {
                op_editor_ui::widgets::property_panel_code::code_hover_at_with_locale(
                    panel_rect,
                    &self.editor_state.codegen,
                    Point2D::new(x, y),
                    self.editor_state.editor_ui.locale,
                )
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };
        if new_fw_hover != self.editor_state.codegen.framework_hover
            || new_action_hover != self.editor_state.codegen.action_hover
        {
            self.editor_state.codegen.framework_hover = new_fw_hover;
            self.editor_state.codegen.action_hover = new_action_hover;
            self.mark_dirty();
            return true;
        }
        if inside_property_panel {
            let lower_hover_changed = self.clear_hover_below_property_panel();
            if property_hover_changed && !lower_hover_changed {
                self.mark_dirty();
            }
            return true;
        }
        if property_hover_changed {
            self.mark_dirty();
            return true;
        }
        // No drag active — sync align toolbar hover. AFTER all drag
        // branches so an active drag isn't intercepted (codex CONCERN
        // — mirrors native widget_host/input.rs ordering).
        let new_hover = if self.editor_state.selection_count() >= 2 {
            use op_editor_ui::widgets::{AlignToolbar, TOP_BAR_HEIGHT};
            let (cx, _, cw, ch) = self.canvas_region(self.last_viewport_w, self.last_viewport_h);
            let canvas_region = op_editor_ui::Rect {
                origin: Point2D::new(cx, TOP_BAR_HEIGHT),
                size: Point2D::new(cw, ch),
            };
            AlignToolbar::for_canvas_region(canvas_region, &self.editor_state)
                .and_then(|tb| tb.hit_test(Point2D::new(x, y)))
        } else {
            None
        };
        let new_hover_ec = new_hover;
        if new_hover_ec != self.editor_state.editor_ui.align_toolbar_hover {
            self.editor_state.editor_ui.align_toolbar_hover = new_hover_ec;
            self.mark_dirty();
            return true;
        }
        false
    }

    fn clear_hover_below_property_panel(&mut self) -> bool {
        let mut changed = false;
        {
            let ui = &mut self.editor_state.editor_ui;
            changed |= ui.canvas_hover_node.take().is_some();
            changed |= ui.hovered_layer_id.take().is_some();
            changed |= ui.hovered_page_index.take().is_some();
            changed |= ui.toolbar_hover.take().is_some();
            changed |= ui.align_toolbar_hover.take().is_some();
            changed |= ui.statusbar_hover.take().is_some();
            changed |= ui.chat_design_block_hover.take().is_some();
            changed |= ui.chat_footer_hover.take().is_some();
            changed |= ui.chat_example_hover.take().is_some();
            changed |= ui.chat_tab_hover.take().is_some();
            if let Some(menu) = ui.layer_context_menu.as_mut() {
                changed |= menu.menu.hover.take().is_some();
            }
        }
        if let Some(menu) = self.editor_state.ui.path_anchor_menu.as_mut() {
            changed |= menu.menu.hover.take().is_some();
        }
        if changed {
            self.mark_dirty();
        }
        changed
    }
}
