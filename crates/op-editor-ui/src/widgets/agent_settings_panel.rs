//! Multi-tab settings modal opened via `Cmd+,`.

use crate::theme::Theme;
use crate::widgets::agent_settings_acp::{self, AcpHit};
use crate::widgets::agent_settings_builtin::{self, BuiltinHit};
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::agent_settings_images::{self, ImagesHit};
use crate::widgets::agent_settings_mcp::{self, McpHit};
use crate::widgets::agent_settings_panel_card::paint_agent_card;
use crate::widgets::agent_settings_panel_geometry::{
    acp_section_y, agent_card_rect_at, agent_card_rect_in, close_rect, connect_btn_rect_at,
    content_rect, disconnect_btn_rect_at, nav_item_rect, tab_i18n_label,
};
use crate::widgets::agent_settings_system::{self, SystemHit};
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{PaintCx, Widget, WidgetId};
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::agent_settings::{
    AcpAgentField, AgentProvider, AgentSettings, AgentSettingsTab, BuiltinAgentField,
    ImageGenField, ImageSearchField, McpCli,
};
use op_editor_core::editor_ui_state::EditorUiState;
use op_editor_core::BuiltinAgentPresetKey;
use op_editor_core::EditorState;
use op_editor_core::{AgentSettingsButton, ButtonPressTarget};

pub const PANEL_WIDTH: f32 = 720.0;
pub const PANEL_HEIGHT: f32 = 720.0;
pub(super) const SIDEBAR_WIDTH: f32 = 200.0;
pub(super) const PAD: f32 = 24.0;
pub(super) const NAV_ITEM_STEP: f32 = 30.0;
pub(super) const NAV_ITEM_HEIGHT: f32 = 28.0;
pub(super) const NAV_TOP: f32 = 56.0;
pub(super) const SECTION_GAP: f32 = 28.0;
pub(super) const CARD_HEIGHT: f32 = 56.0;
pub(super) const CARD_GAP: f32 = 8.0;
// 56x28 left the centered "Connect" label ~3px from each edge (measured
// on the Agents cards); 76x30 restores a comfortable button inset.
pub(super) const CONNECT_BTN_W: f32 = 76.0;
pub(super) const CONNECT_BTN_H: f32 = 30.0;
pub(super) const AVATAR_SIZE: f32 = 28.0;
pub(super) const AVATAR_ICON: f32 = 16.0;
pub(super) const NAME_FONT: f32 = 13.0;
pub(super) const SUB_FONT: f32 = 11.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSettingsPanelMode {
    Full,
    WebBuiltinOnly,
}

impl AgentSettingsPanelMode {
    fn visible_tabs(self) -> &'static [AgentSettingsTab] {
        match self {
            AgentSettingsPanelMode::Full => &AgentSettingsTab::ALL,
            AgentSettingsPanelMode::WebBuiltinOnly => &[
                AgentSettingsTab::Agents,
                AgentSettingsTab::Images,
                AgentSettingsTab::System,
            ],
        }
    }

    fn active_tab(self, settings: &AgentSettings) -> AgentSettingsTab {
        if self.visible_tabs().contains(&settings.tab) {
            settings.tab
        } else {
            AgentSettingsTab::Agents
        }
    }

    fn shows_external_agents(self) -> bool {
        matches!(self, AgentSettingsPanelMode::Full)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSettingsHit {
    Close,
    SelectTab(AgentSettingsTab),
    Connect(AgentProvider),
    AddProvider,
    FocusBuiltinAgent {
        index: usize,
        field: BuiltinAgentField,
    },
    FocusBuiltinAgentDraft(BuiltinAgentField),
    ToggleBuiltinAgentKind(usize),
    ToggleBuiltinAgentDraftKind,
    ToggleBuiltinAgentPresetMenu(Option<usize>),
    SelectBuiltinAgentPreset {
        index: Option<usize>,
        preset: BuiltinAgentPresetKey,
    },
    SaveBuiltinAgentDraft,
    CancelBuiltinAgentDraft,
    ToggleBuiltinAgentEnabled(usize),
    EditBuiltinAgent(usize),
    RemoveBuiltinAgent(usize),
    AddAcpAgent,
    FocusAcpAgent {
        index: usize,
        field: AcpAgentField,
    },
    FocusAcpAgentDraft(AcpAgentField),
    ToggleAcpConnectionType(usize),
    ToggleAcpDraftConnectionType,
    SaveAcpAgentDraft,
    CancelAcpAgentDraft,
    EditAcpAgent(usize),
    RemoveAcpAgent(usize),
    ToggleAcpConnected(usize),
    ToggleMcpServer,
    ToggleMcpCli(McpCli),
    CopyMcpClientConfig,
    ToggleImagesAdvanced,
    FocusSearchField(ImageSearchField),
    OpenImageRegisterLink,
    TestImageSearch,
    AddGenConfig,
    ToggleGenConfigEditor(usize),
    SetActiveGenConfig(usize),
    RemoveGenConfig(usize),
    TestGenConfig(usize),
    ToggleGenProviderMenu(usize),
    SelectGenProvider {
        index: usize,
        provider: op_editor_core::agent_settings::ImageGenProvider,
    },
    FocusGenConfig {
        index: usize,
        field: ImageGenField,
    },
    ToggleAutoUpdate,
    ToggleExperimental,
    FocusMcpPort,
    Outside,
    Inside,
}

pub struct AgentSettingsPanel<'a> {
    pub id: WidgetId,
    pub theme: Theme,
    pub settings: AgentSettings,
    pub now_ms: u64,
    mode: AgentSettingsPanelMode,
    ui: &'a EditorUiState,
}

impl<'a> AgentSettingsPanel<'a> {
    pub fn for_editor(state: &'a EditorState) -> Self {
        Self::for_editor_at(state, 0)
    }

    pub fn for_editor_at(state: &'a EditorState, now_ms: u64) -> Self {
        Self {
            id: WidgetId::new(5200),
            theme: theme_for(&state.editor_ui),
            settings: state.editor_ui.agent_settings.clone(),
            now_ms,
            mode: AgentSettingsPanelMode::Full,
            ui: &state.editor_ui,
        }
    }

    pub fn for_web_editor(state: &'a EditorState) -> Self {
        Self::for_web_editor_at(state, 0)
    }

    pub fn for_web_editor_at(state: &'a EditorState, now_ms: u64) -> Self {
        Self {
            id: WidgetId::new(5200),
            theme: theme_for(&state.editor_ui),
            settings: state.editor_ui.agent_settings.clone(),
            now_ms,
            mode: AgentSettingsPanelMode::WebBuiltinOnly,
            ui: &state.editor_ui,
        }
    }

    pub fn rect(&self, viewport_w: f32, viewport_h: f32) -> Rect {
        let x = ((viewport_w - PANEL_WIDTH) / 2.0).max(8.0);
        let y = ((viewport_h - PANEL_HEIGHT) / 2.0).max(crate::widgets::TOP_BAR_HEIGHT + 8.0);
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(PANEL_WIDTH, PANEL_HEIGHT),
        }
    }

    pub fn hit_test(&self, panel: Rect, point: Point2D) -> AgentSettingsHit {
        if !(panel).contains(point) {
            return AgentSettingsHit::Outside;
        }
        if (close_rect(panel)).contains(point) {
            return AgentSettingsHit::Close;
        }
        for (i, tab) in self.mode.visible_tabs().iter().enumerate() {
            if (nav_item_rect(panel, i)).contains(point) {
                return AgentSettingsHit::SelectTab(*tab);
            }
        }
        // Translate the cursor into the scrolled content frame
        // for hit-tests over scrollable rows.
        let scrolled = Point2D::new(point.x, point.y + self.settings.scroll_y.offset);
        match self.mode.active_tab(&self.settings) {
            AgentSettingsTab::Agents => {
                match agent_settings_builtin::hit_test(
                    content_rect(panel),
                    &self.settings,
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
                if !self.mode.shows_external_agents() {
                    return AgentSettingsHit::Inside;
                }
                let content = content_rect(panel);
                let acp_y = acp_section_y(content, &self.settings);
                match agent_settings_acp::hit_test(content, &self.settings, scrolled, acp_y) {
                    AcpHit::AddAgent => return AgentSettingsHit::AddAcpAgent,
                    AcpHit::Focus { index, field } => {
                        return AgentSettingsHit::FocusAcpAgent { index, field };
                    }
                    AcpHit::FocusDraft(field) => {
                        return AgentSettingsHit::FocusAcpAgentDraft(field)
                    }
                    AcpHit::ToggleConnectionType(index) => {
                        return AgentSettingsHit::ToggleAcpConnectionType(index);
                    }
                    AcpHit::ToggleDraftConnectionType => {
                        return AgentSettingsHit::ToggleAcpDraftConnectionType;
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
                for (i, provider) in AgentProvider::ALL.iter().enumerate() {
                    let card = agent_card_rect_in(panel, i, &self.settings);
                    if !(card).contains(scrolled) {
                        continue;
                    }
                    if self.settings.provider_verified_connected_at(i) {
                        // Connected card — only the disconnect button
                        // (visible on hover) toggles disconnection.
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
                match agent_settings_mcp::hit_test(content_rect(panel), &self.settings, scrolled) {
                    McpHit::ToggleServer => return AgentSettingsHit::ToggleMcpServer,
                    McpHit::ToggleCli(cli) => return AgentSettingsHit::ToggleMcpCli(cli),
                    McpHit::CopyClientConfig => return AgentSettingsHit::CopyMcpClientConfig,
                    McpHit::FocusPort => return AgentSettingsHit::FocusMcpPort,
                    McpHit::None => {}
                }
            }
            AgentSettingsTab::Images => {
                match agent_settings_images::hit_test(content_rect(panel), &self.settings, scrolled)
                {
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
            AgentSettingsTab::System => {
                match agent_settings_system::hit_test(content_rect(panel), scrolled) {
                    SystemHit::ToggleAutoUpdate => return AgentSettingsHit::ToggleAutoUpdate,
                    SystemHit::ToggleExperimental => return AgentSettingsHit::ToggleExperimental,
                    SystemHit::None => {}
                }
            }
        }
        AgentSettingsHit::Inside
    }

    /// Return the sidebar tab under `point`, or `None` if the cursor
    /// is not on a nav row. Geometry mirrors `paint_sidebar`'s
    /// `nav_item_rect` walk so the hover tint aligns with the click
    /// target.
    pub fn nav_at(&self, panel: Rect, point: Point2D) -> Option<AgentSettingsTab> {
        for (i, tab) in self.mode.visible_tabs().iter().enumerate() {
            if (nav_item_rect(panel, i)).contains(point) {
                return Some(*tab);
            }
        }
        None
    }

    /// Return the index of the provider card under `point` (in
    /// screen-space, NOT scrolled), or `None` when the cursor sits
    /// outside every card. Used for hover state.
    pub fn card_at(&self, panel: Rect, point: Point2D) -> Option<usize> {
        if !(panel).contains(point) {
            return None;
        }
        if !self.mode.shows_external_agents() {
            return None;
        }
        let scrolled = Point2D::new(point.x, point.y + self.settings.scroll_y.offset);
        (0..AgentProvider::ALL.len())
            .find(|&i| (agent_card_rect_in(panel, i, &self.settings)).contains(scrolled))
    }

    pub fn builtin_card_at(&self, panel: Rect, point: Point2D) -> Option<usize> {
        if !(panel).contains(point) {
            return None;
        }
        let scrolled = Point2D::new(point.x, point.y + self.settings.scroll_y.offset);
        agent_settings_builtin::card_at(content_rect(panel), &self.settings, scrolled)
    }

    pub fn acp_card_at(&self, panel: Rect, point: Point2D) -> Option<usize> {
        if !(panel).contains(point) {
            return None;
        }
        if !self.mode.shows_external_agents() {
            return None;
        }
        let content = content_rect(panel);
        let scrolled = Point2D::new(point.x, point.y + self.settings.scroll_y.offset);
        let section_y = acp_section_y(content, &self.settings);
        agent_settings_acp::card_at(content, &self.settings, scrolled, section_y)
    }

    pub fn builtin_preset_hover_at(
        &self,
        panel: Rect,
        point: Point2D,
    ) -> Option<BuiltinAgentPresetKey> {
        if !(panel).contains(point) {
            return None;
        }
        let scrolled = Point2D::new(point.x, point.y + self.settings.scroll_y.offset);
        agent_settings_builtin::preset_hover_at(content_rect(panel), &self.settings, scrolled)
    }

    pub fn builtin_preset_scroll_max_at(&self, panel: Rect, point: Point2D) -> Option<f32> {
        if !(panel).contains(point) {
            return None;
        }
        let scrolled = Point2D::new(point.x, point.y + self.settings.scroll_y.offset);
        agent_settings_builtin::preset_scroll_max_at(content_rect(panel), &self.settings, scrolled)
    }

    pub fn image_search_test_button_hover_at(&self, panel: Rect, point: Point2D) -> bool {
        if !(panel).contains(point) {
            return false;
        }
        let scrolled = Point2D::new(point.x, point.y + self.settings.scroll_y.offset);
        agent_settings_images::search_test_button_hover_at(
            content_rect(panel),
            &self.settings,
            scrolled,
        )
    }

    pub fn image_gen_add_button_hover_at(&self, panel: Rect, point: Point2D) -> bool {
        if !(panel).contains(point) {
            return false;
        }
        let scrolled = Point2D::new(point.x, point.y + self.settings.scroll_y.offset);
        agent_settings_images::add_gen_button_hover_at(
            content_rect(panel),
            &self.settings,
            scrolled,
        )
    }

    pub fn image_gen_profile_test_button_hover_at(
        &self,
        panel: Rect,
        point: Point2D,
    ) -> Option<usize> {
        if !(panel).contains(point) {
            return None;
        }
        let scrolled = Point2D::new(point.x, point.y + self.settings.scroll_y.offset);
        agent_settings_images::profile_test_button_hover_at(
            content_rect(panel),
            &self.settings,
            scrolled,
        )
    }

    /// Total content height for the active tab. Host uses this to
    /// clamp `scroll_y` so the bottom of the list never floats
    /// above the panel bottom.
    pub fn content_total_height(&self) -> f32 {
        match self.mode.active_tab(&self.settings) {
            AgentSettingsTab::Agents => agents_content_height(&self.settings, self.mode),
            AgentSettingsTab::Mcp => agent_settings_mcp::content_height(&self.settings),
            AgentSettingsTab::Images => agent_settings_images::content_height(&self.settings),
            AgentSettingsTab::System => agent_settings_system::content_height(),
        }
    }
}

fn agents_content_height(settings: &AgentSettings, mode: AgentSettingsPanelMode) -> f32 {
    let builtin = 12.0 + agent_settings_builtin::content_height(settings);
    if !mode.shows_external_agents() {
        return builtin + 24.0;
    }
    builtin
        + SECTION_GAP
        + (agent_settings_acp::content_height(settings) + SECTION_GAP)
        + 32.0
        + 5.0 * (CARD_HEIGHT + CARD_GAP)
        + 28.0
        + 24.0
}

impl<'a> Widget for AgentSettingsPanel<'a> {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _cx: &crate::widgets::LayoutCx) -> crate::widgets::LayoutBox {
        crate::widgets::LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(PANEL_WIDTH, PANEL_HEIGHT),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        paint_panel(
            cx,
            &self.theme,
            &self.settings,
            rect,
            self.ui,
            self.now_ms,
            self.mode,
        );
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Dialog);
        node.set_label("Settings");
        node
    }
}

fn paint_panel(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    panel: Rect,
    _ui: &EditorUiState,
    now_ms: u64,
    mode: AgentSettingsPanelMode,
) {
    cx.backend.fill_round_rect(panel, 14.0, theme.card);
    cx.backend.stroke_round_rect(panel, 14.0, theme.border, 1.0);
    paint_sidebar(cx, theme, settings, _ui, panel, mode);
    let content_rect = content_rect(panel);
    cx.backend.save();
    cx.backend.clip_rect(content_rect);
    cx.backend
        .translate(Point2D::new(0.0, -settings.scroll_y.offset));
    match mode.active_tab(settings) {
        AgentSettingsTab::Agents => {
            paint_agents_tab(cx, theme, settings, _ui, content_rect, now_ms, mode)
        }
        AgentSettingsTab::Mcp => {
            agent_settings_mcp::paint_mcp_tab(cx, theme, settings, _ui, content_rect, now_ms)
        }
        AgentSettingsTab::Images => {
            agent_settings_images::paint_images_tab(cx, theme, settings, _ui, content_rect, now_ms)
        }
        AgentSettingsTab::System => {
            agent_settings_system::paint_system_tab(cx, theme, settings, _ui, content_rect)
        }
    }
    cx.backend.restore();
    paint_close(cx, theme, settings, _ui, panel);
}

fn paint_sidebar(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    panel: Rect,
    mode: AgentSettingsPanelMode,
) {
    let sidebar = Rect {
        origin: panel.origin,
        size: Point2D::new(SIDEBAR_WIDTH, panel.size.y),
    };
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(sidebar.origin.x + sidebar.size.x - 1.0, sidebar.origin.y),
            size: Point2D::new(1.0, sidebar.size.y),
        },
        theme.border,
    );
    let title = TextLayout::single_run(
        t_settings(ui, "settings.title"),
        "system-ui",
        15.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &title,
        Point2D::new(panel.origin.x + 16.0, panel.origin.y + 31.0),
    );
    let active_tab = mode.active_tab(settings);
    for (i, tab) in mode.visible_tabs().iter().enumerate() {
        let r = nav_item_rect(panel, i);
        let selected = *tab == active_tab;
        let hovered = !selected && settings.hover_nav == Some(*tab);
        if selected {
            cx.backend.fill_round_rect(r, 8.0, theme.muted);
        } else if hovered {
            cx.backend.fill_round_rect(r, 8.0, theme.accent);
        }
        let icon = match tab {
            AgentSettingsTab::Agents => Icon::Pen,
            AgentSettingsTab::Mcp => Icon::Terminal,
            AgentSettingsTab::Images => Icon::Image,
            AgentSettingsTab::System => Icon::Settings,
        };
        let icon_color = if selected {
            theme.foreground
        } else {
            theme.muted_foreground
        };
        draw_icon(
            cx.backend,
            icon,
            Point2D::new(r.origin.x + 12.0, r.origin.y + 7.0),
            14.0,
            icon_color,
            1.6,
        );
        let label = TextLayout::single_run(
            tab_i18n_label(ui, *tab),
            "system-ui",
            13.0,
            (icon_color).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&label, Point2D::new(r.origin.x + 38.0, r.origin.y + 18.0));
    }
}

fn paint_close(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    panel: Rect,
) {
    let close = close_rect(panel);
    let pressed = ui.button_pressed(ButtonPressTarget::AgentSettings(AgentSettingsButton::Close));
    crate::widgets::button::paint_ghost_button_feedback(
        cx.backend,
        theme,
        close,
        settings.hover_agent_settings_close,
        pressed,
    );
    draw_icon(
        cx.backend,
        Icon::Close,
        close.origin,
        close.size.x,
        theme.foreground,
        2.0,
    );
}

fn paint_agents_tab(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    content: Rect,
    now_ms: u64,
    mode: AgentSettingsPanelMode,
) {
    let mut y = content.origin.y + 12.0;
    y = agent_settings_builtin::paint_builtin_section(cx, theme, settings, ui, content, y, now_ms);
    if !mode.shows_external_agents() {
        return;
    }
    y += SECTION_GAP;
    y = agent_settings_acp::paint_acp_section(cx, theme, settings, ui, content, y, now_ms);
    y += SECTION_GAP;

    y = paint_section_header(
        cx,
        theme,
        t_settings(ui, "settings.agents.title"),
        "",
        content.origin.x,
        y,
        content.size.x,
    );
    for (i, provider) in AgentProvider::ALL.iter().enumerate() {
        let card = agent_card_rect_at(content.origin.x, y, content.size.x);
        paint_agent_card(cx, theme, settings, ui, *provider, card, i);
        y += CARD_HEIGHT + CARD_GAP;
        if matches!(provider, AgentProvider::ClaudeCode)
            && settings.provider_verified_connected_at(i)
        {
            let hint = TextLayout::single_run(
                t_settings(ui, "settings.agents.claudeHint"),
                "system-ui",
                12.0,
                (theme.muted_foreground).to_jian(),
                Point2D::new(0.0, 0.0),
            );
            cx.backend
                .draw_text(&hint, Point2D::new(content.origin.x, y + 8.0));
            y += 28.0;
        }
    }
}

fn paint_section_header(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    title: &str,
    action: &str,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    paint_section_header_inset(cx, theme, title, action, x, y, w, 0.0)
}

/// `right_inset` reserves space on the right edge for an
/// overlapping chrome element — currently the panel's close X
/// which sits over the top-of-content row.
// Paint-context + geometry args threaded through; a struct adds no gain.
#[allow(clippy::too_many_arguments)]
fn paint_section_header_inset(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    title: &str,
    action: &str,
    x: f32,
    y: f32,
    w: f32,
    right_inset: f32,
) -> f32 {
    let layout = TextLayout::single_run(
        title,
        "system-ui",
        15.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&layout, Point2D::new(x, y + 18.0));
    if !action.is_empty() {
        let action_w = cx.backend.measure_text(action, 12.0);
        let act = TextLayout::single_run(
            action,
            "system-ui",
            12.0,
            (theme.primary).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&act, Point2D::new(x + w - right_inset - action_w, y + 18.0));
    }
    y + 28.0
}

pub fn drag_for_hit(
    _hit: AgentSettingsHit,
) -> Option<op_editor_core::agent_settings::AgentSettingsDrag> {
    None
}
