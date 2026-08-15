//! Agent-settings modal hover tracking for `WidgetHostNative`.
//!
//! Carved out of `geometry.rs` at the 800-line cap: the settings
//! panel's hover state is a wide tuple sweep (nav / provider cards /
//! MCP rows / image-search rows / preset rows), all written back to
//! `editor_state.editor_ui.agent_settings` with a single dirty mark.

use super::WidgetHostNative;
use op_editor_ui::Point2D;

impl WidgetHostNative {
    pub fn update_agent_settings_hover(&mut self, x: f32, y: f32) -> bool {
        use op_editor_core::AgentSettingsTab;
        self.refresh_layout_scene();
        let point = Point2D::new(x, y);
        let (
            new_nav,
            new_card,
            new_builtin,
            new_acp,
            new_acp_preset,
            new_preset_hover,
            new_model_hover,
            new_close_hover,
            new_server_hover,
            new_copy_hover,
            new_add_provider_hover,
            new_add_acp_hover,
            new_image_search_test_hover,
            new_image_search_register_link_hover,
            new_image_add_hover,
            new_image_profile_header_hover,
            new_image_profile_remove_hover,
            new_image_profile_provider_hover,
            new_image_profile_test_hover,
            new_image_provider_option_hover,
        ) = {
            let (panel, panel_rect) =
                self.agent_settings_geometry(self.last_viewport_w, self.last_viewport_h);
            let nav = panel.nav_at(panel_rect, point);
            let tab = self.editor_state.editor_ui.agent_settings.tab;
            let is_agents = matches!(tab, AgentSettingsTab::Agents);
            let is_images = matches!(tab, AgentSettingsTab::Images);
            let card = if is_agents {
                panel.card_at(panel_rect, point).unwrap_or(usize::MAX)
            } else {
                usize::MAX
            };
            let builtin = if is_agents {
                panel
                    .builtin_card_at(panel_rect, point)
                    .unwrap_or(usize::MAX)
            } else {
                usize::MAX
            };
            let acp = if is_agents {
                panel.acp_card_at(panel_rect, point).unwrap_or(usize::MAX)
            } else {
                usize::MAX
            };
            let acp_preset = if is_agents {
                panel.acp_preset_at(panel_rect, point)
            } else {
                None
            };
            let preset_hover = if is_agents {
                panel.builtin_preset_hover_at(panel_rect, point)
            } else {
                None
            };
            let model_hover = if is_agents {
                panel.builtin_model_hover_at(panel_rect, point)
            } else {
                None
            };
            let hit = panel.hit_test(panel_rect, point);
            let close_hover = matches!(
                hit,
                op_editor_ui::widgets::agent_settings_panel::AgentSettingsHit::Close
            );
            let copy_hover = matches!(tab, AgentSettingsTab::Mcp)
                && matches!(
                    hit,
                    op_editor_ui::widgets::agent_settings_panel::AgentSettingsHit::CopyMcpClientConfig
                );
            let server_hover = matches!(tab, AgentSettingsTab::Mcp)
                && matches!(
                    hit,
                    op_editor_ui::widgets::agent_settings_panel::AgentSettingsHit::ToggleMcpServer
                );
            let add_provider_hover = is_agents
                && matches!(
                    hit,
                    op_editor_ui::widgets::agent_settings_panel::AgentSettingsHit::AddProvider
                );
            let add_acp_hover = is_agents
                && matches!(
                    hit,
                    op_editor_ui::widgets::agent_settings_panel::AgentSettingsHit::AddAcpAgent
                );
            let image_search_test_hover =
                is_images && panel.image_search_test_button_hover_at(panel_rect, point);
            let image_search_register_link_hover = is_images
                && matches!(
                    hit,
                    op_editor_ui::widgets::agent_settings_panel::AgentSettingsHit::OpenImageRegisterLink
                );
            let image_add_hover =
                is_images && panel.image_gen_add_button_hover_at(panel_rect, point);
            let image_profile_header_hover = if is_images {
                match hit {
                    op_editor_ui::widgets::agent_settings_panel::AgentSettingsHit::ToggleGenConfigEditor(
                        index,
                    ) => Some(index),
                    _ => None,
                }
            } else {
                None
            };
            let image_profile_remove_hover = if is_images {
                match hit {
                    op_editor_ui::widgets::agent_settings_panel::AgentSettingsHit::RemoveGenConfig(
                        index,
                    ) => Some(index),
                    _ => None,
                }
            } else {
                None
            };
            let image_profile_provider_hover = if is_images {
                match hit {
                    op_editor_ui::widgets::agent_settings_panel::AgentSettingsHit::ToggleGenProviderMenu(
                        index,
                    ) => Some(index),
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
                    op_editor_ui::widgets::agent_settings_panel::AgentSettingsHit::SelectGenProvider {
                        index,
                        provider,
                    } => Some((index, provider)),
                    _ => None,
                }
            } else {
                None
            };
            (
                nav,
                card,
                builtin,
                acp,
                acp_preset,
                preset_hover,
                model_hover,
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
            )
        };
        let mut changed = false;
        if new_nav != self.editor_state.editor_ui.agent_settings.hover_nav {
            self.editor_state.editor_ui.agent_settings.hover_nav = new_nav;
            changed = true;
        }
        if new_card != self.editor_state.editor_ui.agent_settings.hover_provider {
            self.editor_state.editor_ui.agent_settings.hover_provider = new_card;
            changed = true;
        }
        if new_builtin
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_builtin_agent
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_builtin_agent = new_builtin;
            changed = true;
        }
        if new_acp != self.editor_state.editor_ui.agent_settings.hover_acp_agent {
            self.editor_state.editor_ui.agent_settings.hover_acp_agent = new_acp;
            changed = true;
        }
        if new_acp_preset != self.editor_state.editor_ui.agent_settings.hover_acp_preset {
            self.editor_state.editor_ui.agent_settings.hover_acp_preset = new_acp_preset;
            changed = true;
        }
        if new_preset_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .builtin_preset_menu_hover
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .builtin_preset_menu_hover = new_preset_hover;
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
        if new_close_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_agent_settings_close
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_agent_settings_close = new_close_hover;
            changed = true;
        }
        if new_copy_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_mcp_client_config_copy
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_mcp_client_config_copy = new_copy_hover;
            changed = true;
        }
        if new_server_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_mcp_server_button
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_mcp_server_button = new_server_hover;
            changed = true;
        }
        if new_add_provider_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_add_provider
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_add_provider = new_add_provider_hover;
            changed = true;
        }
        if new_add_acp_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_add_acp_agent
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_add_acp_agent = new_add_acp_hover;
            changed = true;
        }
        if new_image_search_test_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_search_test_button
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_search_test_button = new_image_search_test_hover;
            changed = true;
        }
        if new_image_search_register_link_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_search_register_link
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_search_register_link = new_image_search_register_link_hover;
            changed = true;
        }
        if new_image_add_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_add_button
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_add_button = new_image_add_hover;
            changed = true;
        }
        if new_image_profile_header_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_profile_header
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_profile_header = new_image_profile_header_hover;
            changed = true;
        }
        if new_image_profile_remove_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_profile_remove
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_profile_remove = new_image_profile_remove_hover;
            changed = true;
        }
        if new_image_profile_provider_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_profile_provider
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_profile_provider = new_image_profile_provider_hover;
            changed = true;
        }
        if new_image_profile_test_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_profile_test
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_profile_test = new_image_profile_test_hover;
            changed = true;
        }
        if new_image_provider_option_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_provider_option
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_provider_option = new_image_provider_option_hover;
            changed = true;
        }
        let (new_missing_hover, font_picker_hover, font_import_hover) = if matches!(
            self.editor_state.editor_ui.agent_settings.tab,
            op_editor_core::AgentSettingsTab::Fonts
        ) {
            use op_editor_ui::widgets::agent_settings_fonts::FontsHit;
            use op_editor_ui::widgets::agent_settings_panel::AgentSettingsHit;
            let (panel, panel_rect) =
                self.agent_settings_geometry(self.last_viewport_w, self.last_viewport_h);
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
