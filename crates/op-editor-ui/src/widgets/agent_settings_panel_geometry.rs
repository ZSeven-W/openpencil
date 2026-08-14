//! Geometry helpers shared by the agent-settings panel paint and hit-test paths.

use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::agent_settings_panel::{
    AGENTS_HERO_HEIGHT, CARD_GAP, CARD_HEIGHT, CONNECT_BTN_H, CONNECT_BTN_W, CONTENT_BOTTOM_PAD,
    HEADER_HEIGHT, PAD, SECTION_GAP, STATUS_PILL_HEIGHT, STATUS_PILL_SLOT, TAB_BAR_TOP, TAB_GAP,
    TAB_HEIGHT, TAB_MAX_WIDTH, TAB_MIN_WIDTH,
};
use crate::widgets::{agent_settings_acp, agent_settings_builtin};
use crate::{Point2D, Rect};
use op_editor_core::agent_settings::{AgentSettings, AgentSettingsTab};
use op_editor_core::editor_ui_state::EditorUiState;

const DISCONNECT_BTN_W: f32 = 88.0;
/// Inner inset of a provider row (avatar on the left, status pill on the
/// right sit this far in from the row edges).
const ROW_INSET: f32 = crate::widgets::agent_settings_metrics::ROW_PAD_X;
/// Gap between the row's action button and the status-pill slot.
const ROW_ACTION_GAP: f32 = 12.0;
/// The external-provider list's own section header, between the ACP
/// section and the first provider row. Paint and the row ladder below
/// both step over exactly this — they used to disagree by 4 px, which put
/// every provider row's hit box below the row you could see.
pub(super) const PROVIDER_SECTION_HEADER_H: f32 =
    crate::widgets::agent_settings_metrics::SECTION_HEADER_H;
/// Muted hint under a connected Claude Code row.
pub(super) const CLAUDE_HINT_H: f32 = 24.0;
/// Space kept clear at the right of the left-aligned tab strip so a wide
/// strip can never slide under the close button.
const TAB_STRIP_SIDE_RESERVE: f32 = 72.0;

const FULL_TABS_WITHOUT_ACCOUNT: [AgentSettingsTab; 5] = [
    AgentSettingsTab::Agents,
    AgentSettingsTab::Mcp,
    AgentSettingsTab::Images,
    AgentSettingsTab::Fonts,
    AgentSettingsTab::System,
];

pub(super) fn full_settings_tabs(account_available: bool) -> &'static [AgentSettingsTab] {
    if account_available {
        &AgentSettingsTab::ALL
    } else {
        &FULL_TABS_WITHOUT_ACCOUNT
    }
}

pub(super) fn tab_i18n_label(ui: &EditorUiState, tab: AgentSettingsTab) -> &'static str {
    match tab {
        AgentSettingsTab::Agents => t_settings(ui, "settings.tab.agents"),
        AgentSettingsTab::Mcp => t_settings(ui, "settings.tab.mcp"),
        AgentSettingsTab::Images => t_settings(ui, "settings.tab.images"),
        AgentSettingsTab::Fonts => t_settings(ui, "settings.tab.fonts"),
        AgentSettingsTab::System => t_settings(ui, "settings.tab.system"),
        AgentSettingsTab::Account => t_settings(ui, "settings.tab.account"),
    }
}

/// Action button on a provider row. It sits to the LEFT of the status
/// pill slot, so the pill stays visible while Connect / Disconnect is
/// offered.
fn row_action_rect(card: Rect, w: f32) -> Rect {
    Rect {
        origin: Point2D::new(
            card.origin.x + card.size.x - ROW_INSET - STATUS_PILL_SLOT - ROW_ACTION_GAP - w,
            card.origin.y + (CARD_HEIGHT - CONNECT_BTN_H) / 2.0,
        ),
        size: Point2D::new(w, CONNECT_BTN_H),
    }
}

pub(super) fn disconnect_btn_rect_at(card: Rect) -> Rect {
    row_action_rect(card, DISCONNECT_BTN_W)
}

/// Fixed slot the status pill is right-aligned inside. Paint hugs the
/// pill to its label within this rect; hit-test never needs the pill
/// (it is a status readout, not a control).
pub(super) fn status_pill_slot_rect_at(card: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            card.origin.x + card.size.x - ROW_INSET - STATUS_PILL_SLOT,
            card.origin.y + (CARD_HEIGHT - STATUS_PILL_HEIGHT) / 2.0,
        ),
        size: Point2D::new(STATUS_PILL_SLOT, STATUS_PILL_HEIGHT),
    }
}

pub(super) fn content_rect(panel: Rect) -> Rect {
    Rect {
        origin: Point2D::new(panel.origin.x + PAD, panel.origin.y + HEADER_HEIGHT),
        size: Point2D::new(
            panel.size.x - PAD * 2.0,
            panel.size.y - HEADER_HEIGHT - CONTENT_BOTTOM_PAD,
        ),
    }
}

/// Paint-only clip for the scrollable settings body. Full-width cards sit
/// exactly on the content edges, while their 1px outlines are centered on
/// those edges. Leave one horizontal pixel for the outer half of each stroke
/// (and its antialiasing) without relaxing the vertical scroll clip.
pub(super) fn content_paint_clip_rect(panel: Rect) -> Rect {
    let content = content_rect(panel);
    Rect {
        origin: Point2D::new(content.origin.x - 1.0, content.origin.y),
        size: Point2D::new(content.size.x + 2.0, content.size.y),
    }
}

/// One pill in the left-aligned horizontal tab strip. All pills share a
/// width derived from the panel width and the visible-tab `count`, so the
/// geometry needs no text measurement; paint centres icon + label inside.
pub(super) fn nav_item_rect(panel: Rect, i: usize, count: usize) -> Rect {
    let count = count.max(1);
    let available = (panel.size.x - PAD - TAB_STRIP_SIDE_RESERVE).max(TAB_MIN_WIDTH);
    let tab_w = ((available - TAB_GAP * (count - 1) as f32) / count as f32)
        .clamp(TAB_MIN_WIDTH, TAB_MAX_WIDTH);
    let x = panel.origin.x + PAD + i as f32 * (tab_w + TAB_GAP);
    Rect {
        origin: Point2D::new(x, panel.origin.y + TAB_BAR_TOP),
        size: Point2D::new(tab_w, TAB_HEIGHT),
    }
}

pub(super) fn close_rect(panel: Rect) -> Rect {
    let s = 16.0;
    Rect {
        origin: Point2D::new(
            panel.origin.x + panel.size.x - 16.0 - s,
            panel.origin.y + TAB_BAR_TOP + (TAB_HEIGHT - s) / 2.0,
        ),
        size: Point2D::new(s, s),
    }
}

/// First y of the Agents tab body — everything below the tab intro
/// (section-sized title + one muted line). Single source for paint,
/// hit-test, and the content-height walk.
pub(super) fn agents_body_top(content: Rect) -> f32 {
    content.origin.y + AGENTS_HERO_HEIGHT
}

/// Images / Fonts / Account take the same intro every other tab does,
/// with one muted line.
pub(super) const SECONDARY_HEADING_HAS_DESC: bool = true;

/// Body rect handed to those tabs: the content viewport pushed down past
/// the intro the panel paints for them. Keeping the shift here means
/// their own geometry stays written against "the top of my body", so
/// adding the intro needed no edits inside them.
pub(super) fn hero_body_rect(content: Rect) -> Rect {
    hero_body_rect_for_ui(content, false)
}

pub(super) fn hero_body_rect_for_ui(content: Rect, touch: bool) -> Rect {
    let dy = crate::widgets::agent_settings_rows::tab_intro_height_for_ui(
        SECONDARY_HEADING_HAS_DESC,
        touch,
    );
    Rect {
        origin: Point2D::new(content.origin.x, content.origin.y + dy),
        size: Point2D::new(content.size.x, (content.size.y - dy).max(0.0)),
    }
}

pub(super) fn acp_section_y(content: Rect, settings: &AgentSettings) -> f32 {
    agents_body_top(content) + agent_settings_builtin::content_height(settings) + SECTION_GAP
}

pub(super) fn agent_card_rect_at(x: f32, y: f32, w: f32) -> Rect {
    Rect {
        origin: Point2D::new(x, y),
        size: Point2D::new(w, CARD_HEIGHT),
    }
}

pub(super) fn connect_btn_rect_at(card: Rect) -> Rect {
    row_action_rect(card, CONNECT_BTN_W)
}

/// Y of the first external-provider row: past the built-in and ACP
/// sections and past the provider list's own header.
pub(super) fn provider_rows_top(content: Rect, settings: &AgentSettings) -> f32 {
    acp_section_y(content, settings)
        + agent_settings_acp::content_height(settings)
        + SECTION_GAP
        + PROVIDER_SECTION_HEADER_H
}

pub(super) fn agent_card_rect_in(panel: Rect, index: usize, settings: &AgentSettings) -> Rect {
    let content = content_rect(panel);
    let mut y = provider_rows_top(content, settings);
    for i in 0..index {
        y += CARD_HEIGHT + CARD_GAP;
        if i == 0 && settings.provider_verified_connected_at(0) {
            y += CLAUDE_HINT_H;
        }
    }
    agent_card_rect_at(content.origin.x, y, content.size.x)
}
