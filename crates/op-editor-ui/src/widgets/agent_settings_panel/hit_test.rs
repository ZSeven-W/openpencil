//! Hit-testing + hover / geometry queries for [`AgentSettingsPanel`] —
//! `hit_test` (the tab-dispatching click router) plus the per-surface
//! `*_at` hover probes and content-height maths. Carved off
//! `agent_settings_panel.rs` to keep every file under the 800-line cap.

use super::paint::agents_content_height;
use super::*;
use crate::widgets::agent_settings_panel_geometry::SECONDARY_HEADING_HAS_DESC;
use op_editor_core::agent_settings::SettingsFocus;

fn secondary_hero_height(touch: bool) -> f32 {
    crate::widgets::agent_settings_rows::tab_intro_height_for_ui(SECONDARY_HEADING_HAS_DESC, touch)
}

impl AgentSettingsPanel<'_> {
    /// Unscrolled document-space rect for the active editable field.
    /// Hosts subtract [`Self::effective_scroll`] before comparing it with the
    /// visible content viewport (for example when avoiding a soft keyboard).
    pub fn focused_input_rect(&self, panel: Rect) -> Option<Rect> {
        crate::widgets::agent_settings_focus_geometry::focused_input_rect(
            self.resolved_content_viewport(panel),
            &self.settings,
            self.active_tab(),
            self.ui.touch_chrome(),
        )
    }

    /// Unscrolled document-space rect for the model dropdown owned by the
    /// focused built-in Model field. Native touch hosts use this alongside the
    /// focused input rect so the software keyboard does not hide the menu that
    /// became the active interaction surface.
    pub fn focused_model_menu_rect(&self, panel: Rect) -> Option<Rect> {
        crate::widgets::agent_settings_focus_geometry::focused_model_menu_rect(
            self.resolved_content_viewport(panel),
            &self.settings,
            self.active_tab(),
            self.ui.touch_chrome(),
        )
    }

    /// Resolve a click/tap in the currently focused multiline provider Model
    /// field to its UTF-8 byte offset. The screen-space input rect applies the
    /// same outer body scroll as paint and hit-testing.
    pub fn focused_model_text_byte_offset_at(&self, panel: Rect, point: Point2D) -> Option<usize> {
        if !matches!(
            self.settings.focus,
            Some(SettingsFocus::BuiltinAgent {
                field: BuiltinAgentField::Model,
                ..
            }) | Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::Model))
        ) {
            return None;
        }
        let mut input = self.focused_input_rect(panel)?;
        input.origin.y -= self.effective_scroll(panel);
        crate::widgets::agent_settings_caret::model_text_byte_offset_at(
            self.ui,
            input,
            point,
            self.ui.touch_chrome(),
        )
    }

    pub fn hit_test(&self, panel: Rect, point: Point2D) -> AgentSettingsHit {
        if !panel.contains(point) {
            return AgentSettingsHit::Outside;
        }
        let layout = self.resolved_layout(panel);
        if layout.close_target.contains(point) {
            return AgentSettingsHit::Close;
        }
        let tabs = self.mode.visible_tabs(self.ui);
        for (i, tab) in tabs.iter().enumerate() {
            if layout.nav_item_rect(i, tabs.len()).contains(point) {
                return AgentSettingsHit::SelectTab(*tab);
            }
        }
        let active_tab = self.mode.active_tab(&self.settings, self.ui);
        let scroll = self.effective_scroll(panel);
        // The Fonts popup deliberately escapes the body clip and is painted
        // as a modal child. Route it before the body gate; close and tab
        // controls above already retained priority.
        if active_tab == AgentSettingsTab::Fonts && self.font_picker_layout(panel).is_some() {
            let hit = agent_settings_fonts::hit_test(
                panel,
                hero_body_rect_for_ui(layout.content, self.ui.touch_chrome()),
                self.ui,
                point,
                scroll,
            );
            if hit != FontsHit::None {
                return AgentSettingsHit::Fonts(hit);
            }
        }
        // Painted tab bodies are clipped to this viewport. Gate dispatch by
        // the same rect so a scrolled-off control cannot fire through the
        // touch header or navigation strip.
        if !layout.content.contains(point) {
            return AgentSettingsHit::Inside;
        }
        let scrolled = Point2D::new(point.x, point.y + scroll);
        let content = layout.content;
        match active_tab {
            AgentSettingsTab::Agents => {
                match agent_settings_builtin::hit_test_for_ui(
                    content,
                    &self.settings,
                    self.ui,
                    scrolled,
                ) {
                    BuiltinHit::AddProvider => return AgentSettingsHit::AddProvider,
                    BuiltinHit::Focus { index, field } => {
                        return AgentSettingsHit::FocusBuiltinAgent { index, field };
                    }
                    BuiltinHit::FocusDraft(field) => {
                        return AgentSettingsHit::FocusBuiltinAgentDraft(field);
                    }
                    BuiltinHit::ToggleKind(index) => {
                        return AgentSettingsHit::ToggleBuiltinAgentKind(index);
                    }
                    BuiltinHit::ToggleDraftKind => {
                        return AgentSettingsHit::ToggleBuiltinAgentDraftKind;
                    }
                    BuiltinHit::TogglePresetMenu(index) => {
                        return AgentSettingsHit::ToggleBuiltinAgentPresetMenu(index);
                    }
                    BuiltinHit::SelectPreset { index, preset } => {
                        return AgentSettingsHit::SelectBuiltinAgentPreset { index, preset };
                    }
                    BuiltinHit::ToggleModelMenu(index) => {
                        return AgentSettingsHit::ToggleBuiltinModelMenu(index);
                    }
                    BuiltinHit::SelectModel { index, row } => {
                        return AgentSettingsHit::SelectBuiltinModel { index, row };
                    }
                    BuiltinHit::SaveDraft => return AgentSettingsHit::SaveBuiltinAgentDraft,
                    BuiltinHit::CancelDraft => return AgentSettingsHit::CancelBuiltinAgentDraft,
                    BuiltinHit::ToggleEnabled(index) => {
                        return AgentSettingsHit::ToggleBuiltinAgentEnabled(index);
                    }
                    BuiltinHit::Edit(index) => {
                        return AgentSettingsHit::EditBuiltinAgent(index);
                    }
                    BuiltinHit::Remove(index) => {
                        return AgentSettingsHit::RemoveBuiltinAgent(index);
                    }
                    BuiltinHit::None => {}
                }
                if !self.mode.shows_external_agents(self.ui) {
                    return AgentSettingsHit::Inside;
                }
                let acp_y = acp_section_y(content, &self.settings);
                match agent_settings_acp::hit_test(content, &self.settings, scrolled, acp_y) {
                    AcpHit::AddAgent => return AgentSettingsHit::AddAcpAgent,
                    AcpHit::AddPreset(index) => {
                        return AgentSettingsHit::AddAcpPreset(index);
                    }
                    AcpHit::Focus { index, field } => {
                        return AgentSettingsHit::FocusAcpAgent { index, field };
                    }
                    AcpHit::FocusDraft(field) => {
                        return AgentSettingsHit::FocusAcpAgentDraft(field)
                    }
                    AcpHit::SaveDraft => return AgentSettingsHit::SaveAcpAgentDraft,
                    AcpHit::CancelDraft => return AgentSettingsHit::CancelAcpAgentDraft,
                    AcpHit::Edit(index) => return AgentSettingsHit::EditAcpAgent(index),
                    AcpHit::Remove(index) => return AgentSettingsHit::RemoveAcpAgent(index),
                    AcpHit::ToggleConnected(index) => {
                        return AgentSettingsHit::ToggleAcpConnected(index);
                    }
                    AcpHit::None => {}
                }
                for (row, provider) in AgentProvider::DISPLAY.iter().enumerate() {
                    let card = agent_card_rect_in(panel, row, &self.settings);
                    if !(card).contains(scrolled) {
                        continue;
                    }
                    // Connect state is indexed by AgentProvider::ALL
                    // (the append-only persistence layout), not by the
                    // DISPLAY row position.
                    let index = AgentSettings::provider_index(*provider);
                    if self.settings.provider_verified_connected_at(index) {
                        let disc = disconnect_btn_rect_at(card);
                        if (disc).contains(scrolled) {
                            return AgentSettingsHit::Connect(*provider);
                        }
                    } else if (connect_btn_rect_at(card)).contains(scrolled) {
                        return AgentSettingsHit::Connect(*provider);
                    }
                }
            }
            AgentSettingsTab::Mcp => {
                match agent_settings_mcp::hit_test(content, &self.settings, scrolled) {
                    McpHit::ToggleServer => return AgentSettingsHit::ToggleMcpServer,
                    McpHit::ToggleCli(cli) => return AgentSettingsHit::ToggleMcpCli(cli),
                    McpHit::CopyClientConfig => return AgentSettingsHit::CopyMcpClientConfig,
                    McpHit::FocusPort => return AgentSettingsHit::FocusMcpPort,
                    McpHit::None => {}
                }
            }
            AgentSettingsTab::Images => {
                match agent_settings_images::hit_test_for_ui(
                    hero_body_rect_for_ui(content, self.ui.touch_chrome()),
                    &self.settings,
                    scrolled,
                    self.ui.touch_chrome(),
                ) {
                    ImagesHit::ToggleAdvanced => return AgentSettingsHit::ToggleImagesAdvanced,
                    ImagesHit::FocusSearchField(field) => {
                        return AgentSettingsHit::FocusSearchField(field);
                    }
                    ImagesHit::OpenRegisterLink => {
                        return AgentSettingsHit::OpenImageRegisterLink;
                    }
                    ImagesHit::TestSearch => return AgentSettingsHit::TestImageSearch,
                    ImagesHit::AddGenConfig => return AgentSettingsHit::AddGenConfig,
                    ImagesHit::ToggleGenConfigEditor(index) => {
                        return AgentSettingsHit::ToggleGenConfigEditor(index);
                    }
                    ImagesHit::SetActiveGenConfig(index) => {
                        return AgentSettingsHit::SetActiveGenConfig(index);
                    }
                    ImagesHit::RemoveGenConfig(index) => {
                        return AgentSettingsHit::RemoveGenConfig(index);
                    }
                    ImagesHit::TestGenConfig(index) => {
                        return AgentSettingsHit::TestGenConfig(index);
                    }
                    ImagesHit::ToggleGenProviderMenu(index) => {
                        return AgentSettingsHit::ToggleGenProviderMenu(index);
                    }
                    ImagesHit::SelectGenProvider { index, provider } => {
                        return AgentSettingsHit::SelectGenProvider { index, provider };
                    }
                    ImagesHit::FocusGenConfig { index, field } => {
                        return AgentSettingsHit::FocusGenConfig { index, field };
                    }
                    ImagesHit::None => {}
                }
            }
            AgentSettingsTab::Fonts => {
                let hit = agent_settings_fonts::hit_test(
                    panel,
                    hero_body_rect_for_ui(content, self.ui.touch_chrome()),
                    self.ui,
                    point,
                    scroll,
                );
                if hit != FontsHit::None {
                    return AgentSettingsHit::Fonts(hit);
                }
            }
            AgentSettingsTab::System => match agent_settings_system::hit_test(content, scrolled) {
                SystemHit::SelectThemeMode(mode) => return AgentSettingsHit::SelectThemeMode(mode),
                SystemHit::ToggleAutoUpdate => return AgentSettingsHit::ToggleAutoUpdate,
                SystemHit::ToggleExperimental => return AgentSettingsHit::ToggleExperimental,
                SystemHit::SelectPencilCursor(style) => {
                    return AgentSettingsHit::SelectPencilCursor(style)
                }
                SystemHit::None => {}
            },
            AgentSettingsTab::Account => {
                match agent_settings_account::hit_test(
                    hero_body_rect_for_ui(content, self.ui.touch_chrome()),
                    self.ui,
                    scrolled,
                ) {
                    AccountTabHit::SignIn => return AgentSettingsHit::OpenLoginModal,
                    AccountTabHit::SignOut => return AgentSettingsHit::SignOutAccount,
                    AccountTabHit::None => {}
                }
            }
        }
        AgentSettingsHit::Inside
    }

    pub fn nav_at(&self, panel: Rect, point: Point2D) -> Option<AgentSettingsTab> {
        let layout = self.resolved_layout(panel);
        let tabs = self.mode.visible_tabs(self.ui);
        for (i, tab) in tabs.iter().enumerate() {
            if layout.nav_item_rect(i, tabs.len()).contains(point) {
                return Some(*tab);
            }
        }
        None
    }

    pub fn card_at(&self, panel: Rect, point: Point2D) -> Option<usize> {
        let layout = self.resolved_layout(panel);
        if !layout.content.contains(point) {
            return None;
        }
        if !self.mode.shows_external_agents(self.ui) {
            return None;
        }
        let scrolled = Point2D::new(point.x, point.y + self.effective_scroll(panel));
        // Returns the ALL storage index of the hovered card — rows walk
        // DISPLAY order, state reads/writes ALL order.
        AgentProvider::DISPLAY
            .iter()
            .enumerate()
            .find(|(row, _)| (agent_card_rect_in(panel, *row, &self.settings)).contains(scrolled))
            .map(|(_, provider)| AgentSettings::provider_index(*provider))
    }

    pub fn builtin_card_at(&self, panel: Rect, point: Point2D) -> Option<usize> {
        let content = self.resolved_content_viewport(panel);
        if !content.contains(point) {
            return None;
        }
        let scrolled = Point2D::new(point.x, point.y + self.effective_scroll(panel));
        agent_settings_builtin::card_at_for_ui(
            content,
            &self.settings,
            scrolled,
            self.ui.touch_chrome(),
        )
    }

    pub fn acp_card_at(&self, panel: Rect, point: Point2D) -> Option<usize> {
        let content = self.resolved_content_viewport(panel);
        if !content.contains(point) {
            return None;
        }
        if !self.mode.shows_external_agents(self.ui) {
            return None;
        }
        let scrolled = Point2D::new(point.x, point.y + self.effective_scroll(panel));
        let section_y = acp_section_y(content, &self.settings);
        agent_settings_acp::card_at(content, &self.settings, scrolled, section_y)
    }

    /// Visible quick-add preset row under `point`, for the hosts' hover
    /// ladder. Same geometry contract as [`Self::acp_card_at`].
    pub fn acp_preset_at(&self, panel: Rect, point: Point2D) -> Option<usize> {
        let content = self.resolved_content_viewport(panel);
        if !content.contains(point) {
            return None;
        }
        if !self.mode.shows_external_agents(self.ui) {
            return None;
        }
        let scrolled = Point2D::new(point.x, point.y + self.effective_scroll(panel));
        let section_y = acp_section_y(content, &self.settings);
        agent_settings_acp::preset_row_at(content, &self.settings, scrolled, section_y)
    }

    pub fn builtin_preset_hover_at(
        &self,
        panel: Rect,
        point: Point2D,
    ) -> Option<BuiltinAgentPresetKey> {
        let content = self.resolved_content_viewport(panel);
        if !content.contains(point) {
            return None;
        }
        let scrolled = Point2D::new(point.x, point.y + self.effective_scroll(panel));
        agent_settings_builtin::preset_hover_at_for_ui(
            content,
            &self.settings,
            scrolled,
            self.ui.touch_chrome(),
        )
    }

    pub fn builtin_preset_scroll_max_at(&self, panel: Rect, point: Point2D) -> Option<f32> {
        let content = self.resolved_content_viewport(panel);
        if !content.contains(point) {
            return None;
        }
        let scrolled = Point2D::new(point.x, point.y + self.effective_scroll(panel));
        agent_settings_builtin::preset_scroll_max_at_for_ui(
            content,
            &self.settings,
            scrolled,
            self.ui.touch_chrome(),
        )
    }

    /// Hovered model-dropdown row under `point`, mirroring
    /// [`Self::builtin_preset_hover_at`] for the built-in form's Model
    /// combobox menu.
    pub fn builtin_model_hover_at(&self, panel: Rect, point: Point2D) -> Option<usize> {
        let content = self.resolved_content_viewport(panel);
        if !content.contains(point) {
            return None;
        }
        let scrolled = Point2D::new(point.x, point.y + self.effective_scroll(panel));
        agent_settings_builtin::model_hover_at_for_ui(
            content,
            &self.settings,
            scrolled,
            self.ui.touch_chrome(),
        )
    }

    /// Wheel-scroll cap for the model dropdown under `point`; `None`
    /// when the cursor is not over an open model menu.
    pub fn builtin_model_scroll_max_at(&self, panel: Rect, point: Point2D) -> Option<f32> {
        let content = self.resolved_content_viewport(panel);
        if !content.contains(point) {
            return None;
        }
        let scrolled = Point2D::new(point.x, point.y + self.effective_scroll(panel));
        agent_settings_builtin::model_scroll_max_at_for_ui(
            content,
            &self.settings,
            scrolled,
            self.ui.touch_chrome(),
        )
    }

    pub fn image_search_test_button_hover_at(&self, panel: Rect, point: Point2D) -> bool {
        let content = self.resolved_content_viewport(panel);
        if !content.contains(point) {
            return false;
        }
        let scrolled = Point2D::new(point.x, point.y + self.effective_scroll(panel));
        agent_settings_images::search_test_button_hover_at_for_ui(
            hero_body_rect_for_ui(content, self.ui.touch_chrome()),
            &self.settings,
            scrolled,
            self.ui.touch_chrome(),
        )
    }

    pub fn image_gen_add_button_hover_at(&self, panel: Rect, point: Point2D) -> bool {
        let content = self.resolved_content_viewport(panel);
        if !content.contains(point) {
            return false;
        }
        let scrolled = Point2D::new(point.x, point.y + self.effective_scroll(panel));
        agent_settings_images::add_gen_button_hover_at_for_ui(
            hero_body_rect_for_ui(content, self.ui.touch_chrome()),
            &self.settings,
            scrolled,
            self.ui.touch_chrome(),
        )
    }

    pub fn image_gen_profile_test_button_hover_at(
        &self,
        panel: Rect,
        point: Point2D,
    ) -> Option<usize> {
        let content = self.resolved_content_viewport(panel);
        if !content.contains(point) {
            return None;
        }
        let scrolled = Point2D::new(point.x, point.y + self.effective_scroll(panel));
        agent_settings_images::profile_test_button_hover_at_for_ui(
            hero_body_rect_for_ui(content, self.ui.touch_chrome()),
            &self.settings,
            scrolled,
            self.ui.touch_chrome(),
        )
    }

    pub fn content_total_height(&self) -> f32 {
        match self.mode.active_tab(&self.settings, self.ui) {
            AgentSettingsTab::Agents => agents_content_height(&self.settings, self.mode, self.ui),
            AgentSettingsTab::Mcp => agent_settings_mcp::content_height(&self.settings),
            AgentSettingsTab::Images => {
                let content_width = if self.ui.compact_layout() {
                    390.0
                } else if self.ui.touch_chrome() {
                    680.0
                } else {
                    896.0
                };
                secondary_hero_height(self.ui.touch_chrome())
                    + agent_settings_images::responsive_content_height(
                        &self.settings,
                        content_width,
                        self.ui.touch_chrome(),
                    )
            }
            AgentSettingsTab::Fonts => {
                secondary_hero_height(self.ui.touch_chrome())
                    + agent_settings_fonts::content_height(self.ui)
            }
            AgentSettingsTab::System => agent_settings_system::content_height(),
            AgentSettingsTab::Account => {
                secondary_hero_height(self.ui.touch_chrome())
                    + agent_settings_account::content_height()
            }
        }
    }

    pub fn font_picker_layout(
        &self,
        panel: Rect,
    ) -> Option<crate::widgets::property_panel_typography::FontPickerLayout> {
        agent_settings_fonts::picker_layout(
            panel,
            hero_body_rect_for_ui(
                self.resolved_content_viewport(panel),
                self.ui.touch_chrome(),
            ),
            self.ui,
            self.effective_scroll(panel),
        )
    }
}
