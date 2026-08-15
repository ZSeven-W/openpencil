//! Agent-settings modal hover tracking on the web `WidgetHost` — the
//! per-card / per-row hover state the modal paints its washes from.
//!
//! Split out of `cursor_input.rs` to keep every file under the repo's
//! 800-line cap.

use op_editor_ui::Point2D;

use super::WidgetHost;

impl WidgetHost {
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
            new_model_hover,
            acp_preset_hover,
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
            let new_model_hover = panel.builtin_model_hover_at(panel_rect, point);
            // The whole quick-add row is one hit target, so the press hit
            // already names the hovered row — no second geometry walk.
            let acp_preset_hover = match hit {
                AgentSettingsHit::AddAcpPreset(index) if is_agents => Some(index),
                _ => None,
            };
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
                new_model_hover,
                acp_preset_hover,
            )
        };
        let mut changed = false;
        if acp_preset_hover != self.editor_state.editor_ui.agent_settings.hover_acp_preset {
            self.editor_state.editor_ui.agent_settings.hover_acp_preset = acp_preset_hover;
            changed = true;
        }
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
        if new_model_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .builtin_model_menu_hover
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .builtin_model_menu_hover = new_model_hover;
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
        let (new_missing_hover, font_picker_hover, font_import_hover) = if matches!(
            self.editor_state.editor_ui.agent_settings.tab,
            AgentSettingsTab::Fonts
        ) {
            use op_editor_ui::widgets::agent_settings_fonts::FontsHit;
            let panel = AgentSettingsPanel::for_editor(&self.editor_state);
            let panel_rect = panel.rect(self.last_viewport_w, self.last_viewport_h);
            match panel.hit_test(panel_rect, point) {
                AgentSettingsHit::Fonts(FontsHit::ChooseFont(row)) => (
                    Some(op_editor_core::missing_fonts::MissingFontsHover::ChooseFile(row)),
                    None,
                    false,
                ),
                AgentSettingsHit::Fonts(FontsHit::RemoveImportedFont(row)) => (
                    Some(op_editor_core::missing_fonts::MissingFontsHover::RemoveImported(row)),
                    None,
                    false,
                ),
                AgentSettingsHit::Fonts(FontsHit::SelectFont(index)) => (None, Some(index), false),
                AgentSettingsHit::Fonts(FontsHit::ImportFont(_)) => (None, None, true),
                _ => (None, None, false),
            }
        } else {
            (None, None, false)
        };
        if new_missing_hover != self.editor_state.editor_ui.missing_fonts_hover {
            self.editor_state.editor_ui.missing_fonts_hover = new_missing_hover;
            changed = true;
        }
        if font_picker_hover != self.editor_state.editor_ui.font_picker.hover {
            self.editor_state.editor_ui.font_picker.hover = font_picker_hover;
            changed = true;
        }
        if font_import_hover != self.editor_state.editor_ui.font_picker_import_hover {
            self.editor_state.editor_ui.font_picker_import_hover = font_import_hover;
            changed = true;
        }
        if changed {
            self.mark_dirty();
        }
        changed
    }
}
