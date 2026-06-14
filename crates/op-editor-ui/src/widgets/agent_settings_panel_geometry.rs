//! Geometry helpers shared by the agent-settings panel paint and hit-test paths.

use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::agent_settings_panel::{
    CARD_GAP, CARD_HEIGHT, CONNECT_BTN_H, CONNECT_BTN_W, NAV_ITEM_HEIGHT, NAV_ITEM_STEP, NAV_TOP,
    PAD, SECTION_GAP, SIDEBAR_WIDTH,
};
use crate::widgets::{agent_settings_acp, agent_settings_builtin};
use crate::{Point2D, Rect};
use op_editor_core::agent_settings::{AgentSettings, AgentSettingsTab};
use op_editor_core::editor_ui_state::EditorUiState;

const DISCONNECT_BTN_W: f32 = 96.0;

pub(super) fn tab_i18n_label(ui: &EditorUiState, tab: AgentSettingsTab) -> &'static str {
    match tab {
        AgentSettingsTab::Agents => t_settings(ui, "settings.tab.agents"),
        AgentSettingsTab::Mcp => t_settings(ui, "settings.tab.mcp"),
        AgentSettingsTab::Images => t_settings(ui, "settings.tab.images"),
        AgentSettingsTab::System => t_settings(ui, "settings.tab.system"),
    }
}

pub(super) fn disconnect_btn_rect_at(card: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            card.origin.x + card.size.x - 16.0 - DISCONNECT_BTN_W,
            card.origin.y + (CARD_HEIGHT - CONNECT_BTN_H) / 2.0,
        ),
        size: Point2D::new(DISCONNECT_BTN_W, CONNECT_BTN_H),
    }
}

pub(super) fn content_rect(panel: Rect) -> Rect {
    Rect {
        origin: Point2D::new(panel.origin.x + SIDEBAR_WIDTH + PAD, panel.origin.y + PAD),
        size: Point2D::new(
            panel.size.x - SIDEBAR_WIDTH - PAD * 2.0,
            panel.size.y - PAD * 2.0,
        ),
    }
}

pub(super) fn nav_item_rect(panel: Rect, i: usize) -> Rect {
    let y = panel.origin.y + NAV_TOP + i as f32 * NAV_ITEM_STEP;
    Rect {
        origin: Point2D::new(panel.origin.x + 8.0, y),
        size: Point2D::new(SIDEBAR_WIDTH - 16.0, NAV_ITEM_HEIGHT),
    }
}

pub(super) fn close_rect(panel: Rect) -> Rect {
    let s = 16.0;
    Rect {
        origin: Point2D::new(
            panel.origin.x + panel.size.x - 16.0 - s,
            panel.origin.y + 16.0,
        ),
        size: Point2D::new(s, s),
    }
}

pub(super) fn acp_section_y(content: Rect, settings: &AgentSettings) -> f32 {
    content.origin.y + 12.0 + agent_settings_builtin::content_height(settings) + SECTION_GAP
}

pub(super) fn agent_card_rect_at(x: f32, y: f32, w: f32) -> Rect {
    Rect {
        origin: Point2D::new(x, y),
        size: Point2D::new(w, CARD_HEIGHT),
    }
}

pub(super) fn connect_btn_rect_at(card: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            card.origin.x + card.size.x - 16.0 - CONNECT_BTN_W,
            card.origin.y + (CARD_HEIGHT - CONNECT_BTN_H) / 2.0,
        ),
        size: Point2D::new(CONNECT_BTN_W, CONNECT_BTN_H),
    }
}

pub(super) fn agent_card_rect_in(panel: Rect, index: usize, settings: &AgentSettings) -> Rect {
    let content = content_rect(panel);
    let builtin_block = agent_settings_builtin::content_height(settings) + SECTION_GAP;
    let acp_block = agent_settings_acp::content_height(settings) + SECTION_GAP;
    let mut y = content.origin.y + 12.0 + builtin_block + acp_block + 32.0;
    for i in 0..index {
        y += CARD_HEIGHT + CARD_GAP;
        if i == 0 && settings.connected[0] {
            y += 28.0;
        }
    }
    agent_card_rect_at(content.origin.x, y, content.size.x)
}
