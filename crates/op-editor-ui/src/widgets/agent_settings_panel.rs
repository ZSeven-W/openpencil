//! Multi-tab settings modal opened via `Cmd+,`. Left sidebar
//! nav (Agents / MCP / Images / System) + scrollable right pane.
//! Visual parity with the TS app's settings panel.

use crate::theme::Theme;
use crate::widgets::agent_settings_builtin::{self, BuiltinHit};
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::agent_settings_images::{self, ImagesHit};
use crate::widgets::agent_settings_mcp::{self, McpHit};
use crate::widgets::agent_settings_system::{self, SystemHit};
use crate::widgets::brand_icons::{paint_brand_logo, paint_opencode_logo, BrandLogo};
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::agent_settings::{
    AgentProvider, AgentSettings, AgentSettingsTab, BuiltinAgentField, McpCli,
};
use op_editor_core::editor_ui_state::EditorUiState;
use op_editor_core::EditorState;

pub const PANEL_WIDTH: f32 = 720.0;
pub const PANEL_HEIGHT: f32 = 720.0;
const SIDEBAR_WIDTH: f32 = 200.0;
const PAD: f32 = 24.0;
const NAV_ITEM_STEP: f32 = 30.0;
const NAV_ITEM_HEIGHT: f32 = 28.0;
const NAV_TOP: f32 = 56.0;
const SECTION_GAP: f32 = 28.0;
const CARD_HEIGHT: f32 = 56.0;
const CARD_GAP: f32 = 8.0;
const CONNECT_BTN_W: f32 = 56.0;
const CONNECT_BTN_H: f32 = 28.0;
const AVATAR_SIZE: f32 = 28.0;
const AVATAR_ICON: f32 = 16.0;
const NAME_FONT: f32 = 13.0;
const SUB_FONT: f32 = 11.0;

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
    ToggleBuiltinAgentKind(usize),
    ToggleBuiltinAgentEnabled(usize),
    EditBuiltinAgent(usize),
    RemoveBuiltinAgent(usize),
    AddAcpAgent,
    ToggleMcpServer,
    ToggleMcpCli(McpCli),
    ToggleImagesAdvanced,
    TestImageSearch,
    AddGenConfig,
    SetActiveGenConfig(usize),
    RemoveGenConfig(usize),
    ToggleAutoUpdate,
    FocusMcpPort,
    Outside,
    Inside,
}

pub struct AgentSettingsPanel<'a> {
    pub id: WidgetId,
    pub theme: Theme,
    pub settings: AgentSettings,
    ui: &'a EditorUiState,
}

impl<'a> AgentSettingsPanel<'a> {
    pub fn for_editor(state: &'a EditorState) -> Self {
        Self {
            id: WidgetId::new(5200),
            theme: theme_for(&state.editor_ui),
            settings: state.editor_ui.agent_settings.clone(),
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
        if !rect_contains(panel, point) {
            return AgentSettingsHit::Outside;
        }
        if rect_contains(close_rect(panel), point) {
            return AgentSettingsHit::Close;
        }
        for (i, tab) in AgentSettingsTab::ALL.iter().enumerate() {
            if rect_contains(nav_item_rect(panel, i), point) {
                return AgentSettingsHit::SelectTab(*tab);
            }
        }
        // Translate the cursor into the scrolled content frame
        // for hit-tests over scrollable rows.
        let scrolled = Point2D::new(point.x, point.y + self.settings.scroll_y);
        match self.settings.tab {
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
                    BuiltinHit::ToggleKind(index) => {
                        return AgentSettingsHit::ToggleBuiltinAgentKind(index);
                    }
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
                if rect_contains(add_acp_rect(panel, &self.settings), scrolled) {
                    return AgentSettingsHit::AddAcpAgent;
                }
                for (i, provider) in AgentProvider::ALL.iter().enumerate() {
                    let card = agent_card_rect_in(panel, i, &self.settings);
                    if !rect_contains(card, scrolled) {
                        continue;
                    }
                    if self.settings.connected[i] {
                        // Connected card — only the disconnect button
                        // (visible on hover) toggles disconnection.
                        let disc = disconnect_btn_rect_at(card);
                        if rect_contains(disc, scrolled) {
                            return AgentSettingsHit::Connect(*provider);
                        }
                    } else if rect_contains(connect_btn_rect_at(card), scrolled) {
                        return AgentSettingsHit::Connect(*provider);
                    }
                }
            }
            AgentSettingsTab::Mcp => {
                match agent_settings_mcp::hit_test(content_rect(panel), &self.settings, scrolled) {
                    McpHit::ToggleServer => return AgentSettingsHit::ToggleMcpServer,
                    McpHit::ToggleCli(cli) => return AgentSettingsHit::ToggleMcpCli(cli),
                    McpHit::FocusPort => return AgentSettingsHit::FocusMcpPort,
                    McpHit::None => {}
                }
            }
            AgentSettingsTab::Images => {
                match agent_settings_images::hit_test(content_rect(panel), &self.settings, scrolled)
                {
                    ImagesHit::ToggleAdvanced => return AgentSettingsHit::ToggleImagesAdvanced,
                    ImagesHit::TestSearch => return AgentSettingsHit::TestImageSearch,
                    ImagesHit::AddGenConfig => return AgentSettingsHit::AddGenConfig,
                    ImagesHit::SetActiveGenConfig(index) => {
                        return AgentSettingsHit::SetActiveGenConfig(index);
                    }
                    ImagesHit::RemoveGenConfig(index) => {
                        return AgentSettingsHit::RemoveGenConfig(index);
                    }
                    ImagesHit::None => {}
                }
            }
            AgentSettingsTab::System => {
                match agent_settings_system::hit_test(content_rect(panel), scrolled) {
                    SystemHit::ToggleAutoUpdate => return AgentSettingsHit::ToggleAutoUpdate,
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
        for (i, tab) in AgentSettingsTab::ALL.iter().enumerate() {
            if rect_contains(nav_item_rect(panel, i), point) {
                return Some(*tab);
            }
        }
        None
    }

    /// Return the index of the provider card under `point` (in
    /// screen-space, NOT scrolled), or `None` when the cursor sits
    /// outside every card. Used for hover state.
    pub fn card_at(&self, panel: Rect, point: Point2D) -> Option<usize> {
        if !rect_contains(panel, point) {
            return None;
        }
        let scrolled = Point2D::new(point.x, point.y + self.settings.scroll_y);
        (0..AgentProvider::ALL.len())
            .find(|&i| rect_contains(agent_card_rect_in(panel, i, &self.settings), scrolled))
    }

    pub fn builtin_card_at(&self, panel: Rect, point: Point2D) -> Option<usize> {
        if !rect_contains(panel, point) {
            return None;
        }
        let scrolled = Point2D::new(point.x, point.y + self.settings.scroll_y);
        agent_settings_builtin::card_at(content_rect(panel), &self.settings, scrolled)
    }

    /// Total content height for the active tab. Host uses this to
    /// clamp `scroll_y` so the bottom of the list never floats
    /// above the panel bottom.
    pub fn content_total_height(&self) -> f32 {
        match self.settings.tab {
            AgentSettingsTab::Agents => agents_content_height(&self.settings),
            AgentSettingsTab::Mcp => agent_settings_mcp::content_height(),
            AgentSettingsTab::Images => agent_settings_images::content_height(&self.settings),
            AgentSettingsTab::System => agent_settings_system::content_height(),
        }
    }
}

fn agents_content_height(settings: &AgentSettings) -> f32 {
    // header 32 + subtitle 28 + built-in list + GAP, then ACP empty block, then Agents
    // header 32 + 5 cards (CARD_HEIGHT + CARD_GAP) + Claude-Code hint 28.
    12.0 + (agent_settings_builtin::content_height(settings) + SECTION_GAP)
        + (32.0 + 28.0 + 64.0 + SECTION_GAP)
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
        paint_panel(cx, &self.theme, &self.settings, rect, self.ui);
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
) {
    cx.backend.fill_round_rect(panel, 14.0, theme.card);
    cx.backend.stroke_round_rect(panel, 14.0, theme.border, 1.0);
    paint_sidebar(cx, theme, settings, _ui, panel);
    paint_close(cx, theme, panel);
    let content_rect = content_rect(panel);
    cx.backend.save();
    cx.backend.clip_rect(content_rect);
    cx.backend.translate(Point2D::new(0.0, -settings.scroll_y));
    match settings.tab {
        AgentSettingsTab::Agents => paint_agents_tab(cx, theme, settings, _ui, content_rect),
        AgentSettingsTab::Mcp => {
            agent_settings_mcp::paint_mcp_tab(cx, theme, settings, _ui, content_rect)
        }
        AgentSettingsTab::Images => {
            agent_settings_images::paint_images_tab(cx, theme, settings, _ui, content_rect)
        }
        AgentSettingsTab::System => {
            agent_settings_system::paint_system_tab(cx, theme, settings, _ui, content_rect)
        }
    }
    cx.backend.restore();
}

fn paint_sidebar(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    panel: Rect,
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
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &title,
        Point2D::new(panel.origin.x + 16.0, panel.origin.y + 31.0),
    );
    for (i, tab) in AgentSettingsTab::ALL.iter().enumerate() {
        let r = nav_item_rect(panel, i);
        let selected = *tab == settings.tab;
        let hovered = !selected && settings.hover_nav == Some(*tab);
        if selected {
            cx.backend.fill_round_rect(r, 8.0, theme.muted);
        } else if hovered {
            cx.backend.fill_round_rect(r, 8.0, theme.accent);
        }
        let icon = match tab {
            AgentSettingsTab::Agents => Icon::Pencil,
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
            to_jian(icon_color),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&label, Point2D::new(r.origin.x + 38.0, r.origin.y + 18.0));
    }
}

fn paint_close(cx: &mut PaintCx<'_>, theme: &Theme, panel: Rect) {
    let close = close_rect(panel);
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
) {
    let mut y = content.origin.y + 12.0;
    y = agent_settings_builtin::paint_builtin_section(cx, theme, settings, ui, content, y);
    y += SECTION_GAP;

    y = paint_section_header_inset(
        cx,
        theme,
        t_settings(ui, "settings.agents.acp"),
        t_settings(ui, "settings.agents.addAcp"),
        content.origin.x,
        y,
        content.size.x,
        TOP_HEADER_RIGHT_INSET,
    );
    y = paint_section_subtitle(
        cx,
        theme,
        t_settings(ui, "settings.agents.acpSubtitle"),
        content.origin.x,
        y,
    );
    y = paint_empty_hint(
        cx,
        theme,
        t_settings(ui, "settings.agents.acpEmpty"),
        content.origin.x,
        y,
        content.size.x,
    );
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
        if matches!(provider, AgentProvider::ClaudeCode) && settings.connected[i] {
            let hint = TextLayout::single_run(
                t_settings(ui, "settings.agents.claudeHint"),
                "system-ui",
                12.0,
                to_jian(theme.muted_foreground),
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
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&layout, Point2D::new(x, y + 18.0));
    if !action.is_empty() {
        let action_w = cx.backend.measure_text(action, 12.0);
        let act = TextLayout::single_run(
            action,
            "system-ui",
            12.0,
            to_jian(theme.primary),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&act, Point2D::new(x + w - right_inset - action_w, y + 18.0));
    }
    y + 28.0
}

fn paint_section_subtitle(cx: &mut PaintCx<'_>, theme: &Theme, text: &str, x: f32, y: f32) -> f32 {
    let layout = TextLayout::single_run(
        text,
        "system-ui",
        12.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&layout, Point2D::new(x, y + 16.0));
    y + 28.0
}

fn paint_empty_hint(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    text: &str,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let text_w = cx.backend.measure_text(text, 13.0);
    let layout = TextLayout::single_run(
        text,
        "system-ui",
        13.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&layout, Point2D::new(x + (w - text_w) / 2.0, y + 44.0));
    y + 64.0
}

fn paint_agent_card(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    provider: AgentProvider,
    card: Rect,
    index: usize,
) {
    let outlined = matches!(provider, AgentProvider::ClaudeCode);
    let card_hovered = settings.hover_provider == index;
    if outlined {
        cx.backend.fill_round_rect(card, 10.0, theme.muted);
    } else if card_hovered {
        // Subtle wash on hover so the whole row reads as "I'm
        // pointing at this card" rather than leaving the hover
        // signal entirely on the trailing button.
        cx.backend.fill_round_rect(card, 10.0, theme.accent);
    }
    cx.backend.stroke_round_rect(card, 10.0, theme.border, 1.0);
    let avatar = Rect {
        origin: Point2D::new(
            card.origin.x + 12.0,
            card.origin.y + (CARD_HEIGHT - AVATAR_SIZE) / 2.0,
        ),
        size: Point2D::new(AVATAR_SIZE, AVATAR_SIZE),
    };
    cx.backend.fill_round_rect(avatar, 6.0, theme.background);
    let icon_top_left = Point2D::new(
        avatar.origin.x + (AVATAR_SIZE - AVATAR_ICON) / 2.0,
        avatar.origin.y + (AVATAR_SIZE - AVATAR_ICON) / 2.0,
    );
    match provider {
        AgentProvider::ClaudeCode => paint_brand_logo(
            cx.backend,
            BrandLogo::Claude,
            icon_top_left,
            AVATAR_ICON,
            theme.foreground,
        ),
        AgentProvider::CodexCli => paint_brand_logo(
            cx.backend,
            BrandLogo::OpenAI,
            icon_top_left,
            AVATAR_ICON,
            theme.foreground,
        ),
        AgentProvider::OpenCode => {
            paint_opencode_logo(cx.backend, icon_top_left, AVATAR_ICON, theme.foreground)
        }
        AgentProvider::GithubCopilot => paint_brand_logo(
            cx.backend,
            BrandLogo::Copilot,
            icon_top_left,
            AVATAR_ICON,
            theme.foreground,
        ),
        AgentProvider::GeminiCli => paint_brand_logo(
            cx.backend,
            BrandLogo::Gemini,
            icon_top_left,
            AVATAR_ICON,
            theme.foreground,
        ),
    }
    let text_x = card.origin.x + 12.0 + AVATAR_SIZE + 12.0;
    let name = TextLayout::single_run(
        provider.name(),
        "system-ui",
        NAME_FONT,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&name, Point2D::new(text_x, card.origin.y + 22.0));
    let connected = settings.connected[index];
    let sub_localized = t_settings(ui, provider.subtitle_key());
    let (sub_color, sub_text) = if connected {
        (
            Color {
                r: 0.34,
                g: 0.78,
                b: 0.45,
                a: 1.0,
            },
            format!("✓ {}", sub_localized),
        )
    } else {
        (theme.muted_foreground, sub_localized.to_string())
    };
    let sub = TextLayout::single_run(
        &sub_text,
        "system-ui",
        SUB_FONT,
        to_jian(sub_color),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&sub, Point2D::new(text_x, card.origin.y + 38.0));
    let hovered = settings.hover_provider == index;
    if connected {
        if hovered {
            let btn = disconnect_btn_rect_at(card);
            let red = Color {
                r: 0.93,
                g: 0.30,
                b: 0.30,
                a: 1.0,
            };
            cx.backend.fill_round_rect(btn, 6.0, theme.muted);
            cx.backend.stroke_round_rect(btn, 6.0, red, 1.0);
            let label = t_settings(ui, "settings.agents.disconnect");
            let lw = cx.backend.measure_text(label, 12.0);
            let layout = TextLayout::single_run(
                label,
                "system-ui",
                12.0,
                to_jian(red),
                Point2D::new(0.0, 0.0),
            );
            cx.backend.draw_text(
                &layout,
                Point2D::new(btn.origin.x + (btn.size.x - lw) / 2.0, btn.origin.y + 18.0),
            );
        }
    } else {
        let btn = connect_btn_rect_at(card);
        cx.backend.fill_round_rect(btn, 5.0, theme.primary);
        let label = t_settings(ui, "settings.agents.connect");
        let lw = cx.backend.measure_text(label, 12.0);
        let layout = TextLayout::single_run(
            label,
            "system-ui",
            12.0,
            to_jian(theme.primary_foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &layout,
            Point2D::new(btn.origin.x + (btn.size.x - lw) / 2.0, btn.origin.y + 18.0),
        );
    }
}

const DISCONNECT_BTN_W: f32 = 96.0;
const TOP_HEADER_RIGHT_INSET: f32 = 12.0;

fn tab_i18n_label(ui: &EditorUiState, tab: AgentSettingsTab) -> &'static str {
    match tab {
        AgentSettingsTab::Agents => t_settings(ui, "settings.tab.agents"),
        AgentSettingsTab::Mcp => t_settings(ui, "settings.tab.mcp"),
        AgentSettingsTab::Images => t_settings(ui, "settings.tab.images"),
        AgentSettingsTab::System => t_settings(ui, "settings.tab.system"),
    }
}

fn disconnect_btn_rect_at(card: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            card.origin.x + card.size.x - 16.0 - DISCONNECT_BTN_W,
            card.origin.y + (CARD_HEIGHT - CONNECT_BTN_H) / 2.0,
        ),
        size: Point2D::new(DISCONNECT_BTN_W, CONNECT_BTN_H),
    }
}

fn content_rect(panel: Rect) -> Rect {
    Rect {
        origin: Point2D::new(panel.origin.x + SIDEBAR_WIDTH + PAD, panel.origin.y + PAD),
        size: Point2D::new(
            panel.size.x - SIDEBAR_WIDTH - PAD * 2.0,
            panel.size.y - PAD * 2.0,
        ),
    }
}

fn nav_item_rect(panel: Rect, i: usize) -> Rect {
    let y = panel.origin.y + NAV_TOP + i as f32 * NAV_ITEM_STEP;
    Rect {
        origin: Point2D::new(panel.origin.x + 8.0, y),
        size: Point2D::new(SIDEBAR_WIDTH - 16.0, NAV_ITEM_HEIGHT),
    }
}

fn close_rect(panel: Rect) -> Rect {
    let s = 16.0;
    Rect {
        origin: Point2D::new(
            panel.origin.x + panel.size.x - 16.0 - s,
            panel.origin.y + 16.0,
        ),
        size: Point2D::new(s, s),
    }
}

fn add_acp_rect(panel: Rect, settings: &AgentSettings) -> Rect {
    let content = content_rect(panel);
    let y =
        content.origin.y + 12.0 + agent_settings_builtin::content_height(settings) + SECTION_GAP;
    let text_w = 96.0;
    Rect {
        origin: Point2D::new(
            content.origin.x + content.size.x - TOP_HEADER_RIGHT_INSET - text_w,
            y,
        ),
        size: Point2D::new(text_w, 24.0),
    }
}

fn agent_card_rect_at(x: f32, y: f32, w: f32) -> Rect {
    Rect {
        origin: Point2D::new(x, y),
        size: Point2D::new(w, CARD_HEIGHT),
    }
}

fn connect_btn_rect_at(card: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            card.origin.x + card.size.x - 16.0 - CONNECT_BTN_W,
            card.origin.y + (CARD_HEIGHT - CONNECT_BTN_H) / 2.0,
        ),
        size: Point2D::new(CONNECT_BTN_W, CONNECT_BTN_H),
    }
}

fn agent_card_rect_in(panel: Rect, index: usize, settings: &AgentSettings) -> Rect {
    let content = content_rect(panel);
    let builtin_block = agent_settings_builtin::content_height(settings) + SECTION_GAP;
    let acp_block = 32.0 + 28.0 + 64.0 + SECTION_GAP;
    let mut y = content.origin.y + 12.0 + builtin_block + acp_block + 32.0;
    for i in 0..index {
        y += CARD_HEIGHT + CARD_GAP;
        // The Claude env-var hint paints only when ClaudeCode is
        // connected; mirror that here so the rect chain stays in
        // sync with paint.
        if i == 0 && settings.connected[0] {
            y += 28.0;
        }
    }
    agent_card_rect_at(content.origin.x, y, content.size.x)
}

fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.y >= r.origin.y
        && p.x <= r.origin.x + r.size.x
        && p.y <= r.origin.y + r.size.y
}

fn to_jian(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}

pub fn drag_for_hit(
    _hit: AgentSettingsHit,
) -> Option<op_editor_core::agent_settings::AgentSettingsDrag> {
    None
}
