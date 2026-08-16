use crate::theme::Theme;
use crate::widgets::agent_settings_account::{self, AccountTabHit};
use crate::widgets::agent_settings_acp::{self, AcpHit};
use crate::widgets::agent_settings_builtin::{self, BuiltinHit};
use crate::widgets::agent_settings_fonts::{self, FontsHit};
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::agent_settings_images::{self, ImagesHit};
use crate::widgets::agent_settings_mcp::{self, McpHit};
use crate::widgets::agent_settings_panel_card::paint_agent_card;
use crate::widgets::agent_settings_panel_geometry::{
    acp_section_y, agent_card_rect_at, agent_card_rect_in, agents_body_top, close_rect,
    connect_btn_rect_at, content_rect, disconnect_btn_rect_at, full_settings_tabs, hero_body_rect,
    hero_body_rect_for_ui, provider_rows_top, tab_i18n_label, CLAUDE_HINT_H,
    PROVIDER_SECTION_HEADER_H,
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

/// Modal size ceiling. The dialog is a centred workspace rather than a
/// fixed small window: [`AgentSettingsPanel::rect`] shrinks it to fit the
/// viewport, so these are maxima, not fixed dimensions.
///
/// They came down with the row scale. A 1100×850 shell around 54 px rows
/// is mostly empty column — the settings read as a poster instead of a
/// list, which is the "too big" the shipped build showed.
pub const PANEL_WIDTH: f32 = 960.0;
pub const PANEL_HEIGHT: f32 = 760.0;
/// Floor for the shrink-to-fit clamp — below this the tab strip and the
/// provider rows stop being readable, so the modal overflows a tiny
/// viewport instead of collapsing.
const PANEL_MIN_WIDTH: f32 = 620.0;
const PANEL_MIN_HEIGHT: f32 = 420.0;
/// Breathing room kept between the modal and the viewport edges.
const VIEWPORT_MARGIN_X: f32 = 32.0;
const VIEWPORT_MARGIN_BOTTOM: f32 = 48.0;

// The modal's spacing scale lives in `agent_settings_metrics`; the names
// below are that scale under the spellings this file's callers already
// use, plus the few control sizes that belong to a specific control.
use crate::widgets::agent_settings_metrics as metrics;

pub(super) const PAD: f32 = metrics::CONTENT_PAD_X;
pub(super) const CONTENT_BOTTOM_PAD: f32 = metrics::CONTENT_PAD_BOTTOM;
/// Horizontal tab strip: inset from the panel top, pill height, and the
/// per-pill width band. Pills share one width so the hit-test geometry
/// stays measurement-free (paint centres icon + label inside).
pub(super) const TAB_BAR_TOP: f32 = metrics::TAB_BAR_TOP;
pub(super) const TAB_HEIGHT: f32 = metrics::TAB_HEIGHT;
pub(super) const TAB_GAP: f32 = metrics::TAB_GAP;
pub(super) const TAB_MAX_WIDTH: f32 = 116.0;
pub(super) const TAB_MIN_WIDTH: f32 = 72.0;
/// Fixed header band: `TAB_BAR_TOP` + `TAB_HEIGHT` + the gap down to the
/// scrollable body.
pub(super) const HEADER_HEIGHT: f32 = TAB_BAR_TOP + TAB_HEIGHT + metrics::TAB_TO_CONTENT;
/// Total vertical inset between the panel rect and its scrollable
/// content viewport. Hosts subtract this from the panel height to derive
/// the scroll viewport (`content_rect` height) instead of hardcoding it.
pub const CONTENT_VERTICAL_INSET: f32 = HEADER_HEIGHT + CONTENT_BOTTOM_PAD;
pub(super) const SECTION_GAP: f32 = metrics::SECTION_GAP;
pub(super) const CONTENT_TAIL_PAD: f32 = metrics::CONTENT_TAIL_PAD;
/// Provider rows carry a name over a status subtitle, so they take the
/// modal's two-line row box — the same one the MCP server row and the
/// System auto-update row use. Hairline-separated list rows sit flush
/// against each other: the separator IS the gap.
pub(super) const CARD_HEIGHT: f32 = metrics::ROW_H_TWO_LINE;
pub(super) const CARD_GAP: f32 = 0.0;
pub(super) const CONNECT_BTN_W: f32 = 84.0;
pub(super) const CONNECT_BTN_H: f32 = 28.0;
pub(super) const AVATAR_SIZE: f32 = metrics::ROW_AVATAR;
pub(super) const AVATAR_ICON: f32 = 20.0;
pub(super) const NAME_FONT: f32 = crate::widgets::agent_settings_rows::ROW_LABEL_FONT;
pub(super) const SUB_FONT: f32 = crate::widgets::agent_settings_rows::ROW_DESC_FONT;
/// Width reserved at the right edge of a provider row for the status
/// pill. The painted pill hugs its label and is right-aligned inside the
/// slot; the slot itself is fixed so hit-test geometry needs no text
/// measurement.
pub(super) const STATUS_PILL_SLOT: f32 = 140.0;
pub(super) const STATUS_PILL_HEIGHT: f32 = 24.0;
pub(super) const STATUS_PILL_FONT: f32 = 12.0;
/// Agents-tab intro block (title + provider roll) above the first
/// section — the vertical offset from the content viewport's top to the
/// first section header. Public so host tests can anchor to it; it must
/// stay equal to `agent_settings_rows::tab_intro_height(true)`, which a
/// unit test asserts.
pub const AGENTS_HERO_HEIGHT: f32 = crate::widgets::agent_settings_rows::tab_intro_height(true);

/// Scrollable body viewport inside `panel` — everything below the top
/// tab strip, inset by the modal's content padding. Exported so hosts and
/// their tests read the body geometry from its one definition instead of
/// re-deriving the insets.
pub fn content_viewport(panel: Rect) -> Rect {
    content_rect(panel)
}

/// Body viewport for the tabs the panel paints a hero on behalf of
/// (Images / Fonts / Account): [`content_viewport`] pushed down past that
/// hero. Those tabs' own geometry is written against the top of their
/// body, so this is the rect they — and anything testing them — must use.
pub fn secondary_tab_body(panel: Rect) -> Rect {
    hero_body_rect(content_rect(panel))
}

/// Agents tab: the external-provider row boxes, paired with their line
/// count — walked by the traversal layout audit.
#[cfg(test)]
pub(super) fn provider_row_boxes(
    panel: Rect,
) -> Vec<(Rect, crate::widgets::agent_settings_rows::RowLines)> {
    let settings = AgentSettings::default();
    (0..AgentProvider::ALL.len())
        .map(|i| {
            (
                agent_card_rect_in(panel, i, &settings),
                crate::widgets::agent_settings_rows::RowLines::Two,
            )
        })
        .collect()
}

/// MCP tab: the server Start/Stop button.
pub fn mcp_server_button(panel: Rect) -> Rect {
    agent_settings_mcp::server_button_rect(content_rect(panel))
}

/// MCP tab: the "Copy MCP config" action in the custom-configuration
/// section header.
pub fn mcp_copy_config_button(panel: Rect) -> Rect {
    agent_settings_mcp::client_config_copy_button_rect(content_rect(panel))
}

/// System tab: the switch on the Auto-update row.
pub fn system_auto_update_switch(panel: Rect) -> Rect {
    agent_settings_system::auto_update_switch_rect(content_rect(panel))
}

/// Close button rect in the header band, exported alongside
/// [`content_viewport`] so hosts and their tests target it by geometry
/// rather than by a copied offset.
pub fn close_button_rect(panel: Rect) -> Rect {
    close_rect(panel)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSettingsPanelMode {
    Full,
    /// `Full` plus the Account tab — selected when the host enabled the
    /// runtime account gate (`EditorUiState::account_ui_available`).
    FullWithAccount,
    WebBuiltinOnly,
    /// `WebBuiltinOnly` plus the Account tab (web with a daemon-side
    /// auth backend).
    WebBuiltinOnlyWithAccount,
    McpOnly,
}

impl AgentSettingsPanelMode {
    fn visible_tabs(self, ui: &EditorUiState) -> &'static [AgentSettingsTab] {
        if ui.touch_chrome() && self != AgentSettingsPanelMode::McpOnly {
            return if matches!(
                self,
                AgentSettingsPanelMode::FullWithAccount
                    | AgentSettingsPanelMode::WebBuiltinOnlyWithAccount
            ) {
                &TOUCH_TABS_WITH_ACCOUNT
            } else {
                &TOUCH_TABS
            };
        }
        match self {
            AgentSettingsPanelMode::Full => full_settings_tabs(false),
            AgentSettingsPanelMode::FullWithAccount => full_settings_tabs(true),
            AgentSettingsPanelMode::WebBuiltinOnly => &[
                AgentSettingsTab::Agents,
                AgentSettingsTab::Images,
                AgentSettingsTab::Fonts,
                AgentSettingsTab::System,
            ],
            AgentSettingsPanelMode::WebBuiltinOnlyWithAccount => &[
                AgentSettingsTab::Agents,
                AgentSettingsTab::Images,
                AgentSettingsTab::Fonts,
                AgentSettingsTab::System,
                AgentSettingsTab::Account,
            ],
            AgentSettingsPanelMode::McpOnly => &[AgentSettingsTab::Mcp],
        }
    }

    fn active_tab(self, settings: &AgentSettings, ui: &EditorUiState) -> AgentSettingsTab {
        if self.visible_tabs(ui).contains(&settings.tab) {
            settings.tab
        } else {
            self.visible_tabs(ui)[0]
        }
    }

    fn shows_external_agents(self, ui: &EditorUiState) -> bool {
        !ui.touch_chrome()
            && matches!(
                self,
                AgentSettingsPanelMode::Full | AgentSettingsPanelMode::FullWithAccount
            )
    }
}

const TOUCH_TABS: [AgentSettingsTab; 4] = [
    AgentSettingsTab::Agents,
    AgentSettingsTab::Images,
    AgentSettingsTab::Fonts,
    AgentSettingsTab::System,
];

const TOUCH_TABS_WITH_ACCOUNT: [AgentSettingsTab; 5] = [
    AgentSettingsTab::Agents,
    AgentSettingsTab::Images,
    AgentSettingsTab::Fonts,
    AgentSettingsTab::System,
    AgentSettingsTab::Account,
];

fn mode_for_ui(ui: &EditorUiState, base: AgentSettingsPanelMode) -> AgentSettingsPanelMode {
    // Capability controls whether this build can begin a new login; a restored
    // signed-in profile still needs its Account settings even if that runtime
    // gate has not been advertised yet.
    let account_visible = ui.account_ui_available || ui.account.is_signed_in();
    if ui.embed == op_editor_core::EmbedHost::VsCode {
        AgentSettingsPanelMode::McpOnly
    } else if base == AgentSettingsPanelMode::Full && account_visible {
        AgentSettingsPanelMode::FullWithAccount
    } else if base == AgentSettingsPanelMode::WebBuiltinOnly && account_visible {
        AgentSettingsPanelMode::WebBuiltinOnlyWithAccount
    } else {
        base
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
    ToggleBuiltinModelMenu(Option<usize>),
    /// Model-dropdown row pressed — positional index into the card's
    /// runtime catalog, resolved to the model id by the press arm so
    /// this enum stays `Copy`.
    SelectBuiltinModel {
        index: Option<usize>,
        row: usize,
    },
    SaveBuiltinAgentDraft,
    CancelBuiltinAgentDraft,
    ToggleBuiltinAgentEnabled(usize),
    EditBuiltinAgent(usize),
    RemoveBuiltinAgent(usize),
    AddAcpAgent,
    /// Quick-add row pressed — positional index into
    /// `AgentSettings::visible_acp_presets` for the same frame.
    AddAcpPreset(usize),
    FocusAcpAgent {
        index: usize,
        field: AcpAgentField,
    },
    FocusAcpAgentDraft(AcpAgentField),
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
    Fonts(FontsHit),
    ToggleAutoUpdate,
    ToggleExperimental,
    SelectThemeMode(op_editor_core::editor_ui_state::ThemeMode),
    SelectPencilCursor(op_editor_core::PencilCursorStyle),
    FocusMcpPort,
    OpenLoginModal,
    SignOutAccount,
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
            mode: mode_for_ui(&state.editor_ui, AgentSettingsPanelMode::Full),
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
            mode: mode_for_ui(&state.editor_ui, AgentSettingsPanelMode::WebBuiltinOnly),
            ui: &state.editor_ui,
        }
    }

    /// Centred modal rect, clamped to the viewport. Width and height both
    /// shrink below their ceiling when the window is small, so the dialog
    /// never runs off screen or under the top bar. In mobile layout the
    /// settings become a responsive touch surface: full-screen on phone,
    /// inset on tablet portrait, and a bounded rail workspace on tablet
    /// landscape.
    pub fn rect(&self, viewport_w: f32, viewport_h: f32) -> Rect {
        if self.ui.compact_layout() {
            return Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(viewport_w.max(0.0), viewport_h.max(0.0)),
            };
        }
        if self.ui.medium_layout() {
            let inset = 12.0;
            return Rect {
                origin: Point2D::new(inset, inset),
                size: Point2D::new(
                    (viewport_w - inset * 2.0).max(0.0),
                    (viewport_h - inset * 2.0).max(0.0),
                ),
            };
        }
        if self.ui.expanded_touch_layout() {
            let w = PANEL_WIDTH.min((viewport_w - 32.0).max(0.0));
            let h = 800.0_f32.min((viewport_h - 32.0).max(0.0));
            return Rect {
                origin: Point2D::new((viewport_w - w) / 2.0, (viewport_h - h) / 2.0),
                size: Point2D::new(w, h),
            };
        }
        let top_limit = crate::widgets::TOP_BAR_HEIGHT + 8.0;
        let w = PANEL_WIDTH
            .min(viewport_w - VIEWPORT_MARGIN_X * 2.0)
            .max(PANEL_MIN_WIDTH);
        let h = PANEL_HEIGHT
            .min(viewport_h - crate::widgets::TOP_BAR_HEIGHT - VIEWPORT_MARGIN_BOTTOM)
            .max(PANEL_MIN_HEIGHT);
        let x = ((viewport_w - w) / 2.0).max(8.0);
        let y = ((viewport_h - h) / 2.0).max(top_limit);
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(w, h),
        }
    }

    /// Resolved responsive geometry shared by paint, hit-testing, scrolling,
    /// and host gesture code.
    pub fn resolved_layout(&self, panel: Rect) -> AgentSettingsLayout {
        AgentSettingsLayout::resolve(panel, self.ui, self.navigation_tabs().len())
    }

    pub fn resolved_content_viewport(&self, panel: Rect) -> Rect {
        self.resolved_layout(panel).content
    }

    pub fn max_scroll(&self, panel: Rect) -> f32 {
        (self.content_total_height() - self.resolved_content_viewport(panel).size.y).max(0.0)
    }

    pub fn effective_scroll(&self, panel: Rect) -> f32 {
        self.settings
            .scroll_y
            .offset
            .clamp(0.0, self.max_scroll(panel))
    }

    pub fn navigation_tabs(&self) -> &'static [AgentSettingsTab] {
        self.mode.visible_tabs(self.ui)
    }

    pub fn active_tab(&self) -> AgentSettingsTab {
        self.mode.active_tab(&self.settings, self.ui)
    }
}

mod hero;
mod hit_test;
mod layout;
mod paint;
mod tabs;

pub use layout::{AgentSettingsLayout, AgentSettingsSurfaceKind};
pub use paint::drag_for_hit;
