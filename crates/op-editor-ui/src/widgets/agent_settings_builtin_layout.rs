use crate::widgets::agent_settings_builtin_parts;
use crate::widgets::agent_settings_header_action::header_action_rect;
use crate::widgets::agent_settings_switch::{SETTINGS_SWITCH_H, SETTINGS_SWITCH_W};
use crate::{Point2D, Rect};
use op_editor_core::agent_settings::{AgentSettings, SettingsFocus};

pub(super) const HEADER_HEIGHT: f32 = 28.0;
pub(super) const SUBTITLE_HEIGHT: f32 = 28.0;
pub(super) const EMPTY_HEIGHT: f32 = 64.0;
const COMPACT_CARD_HEIGHT: f32 = 60.0;
const EXPANDED_CARD_HEIGHT: f32 = 196.0;
const DRAFT_ACTION_HEIGHT: f32 = 36.0;
pub(super) const CARD_GAP: f32 = 8.0;
const FIELD_LABEL_W: f32 = 68.0;
const FIELD_H: f32 = 24.0;
const ACTION_W: f32 = 24.0;

pub(super) fn is_editing(settings: &AgentSettings, index: usize) -> bool {
    matches!(
        settings.focus,
        Some(SettingsFocus::BuiltinAgent { index: i, .. }) if i == index
    )
}

pub(super) fn card_height(settings: &AgentSettings, index: usize) -> f32 {
    if is_editing(settings, index) {
        expanded_card_height(settings, Some(index))
    } else {
        COMPACT_CARD_HEIGHT
    }
}

pub(super) fn expanded_card_height(settings: &AgentSettings, index: Option<usize>) -> f32 {
    EXPANDED_CARD_HEIGHT + agent_settings_builtin_parts::preset_menu_height(settings, index)
}

pub(super) fn draft_card_height(settings: &AgentSettings) -> f32 {
    expanded_card_height(settings, None) + DRAFT_ACTION_HEIGHT
}

pub(super) fn add_provider_rect(content: Rect, y: f32) -> Rect {
    header_action_rect(content, y, 0.0)
}

pub(super) fn card_rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect {
        origin: Point2D::new(x, y),
        size: Point2D::new(w, h),
    }
}

pub(super) fn compact_switch_rect(card: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            card.origin.x + card.size.x - 12.0 - SETTINGS_SWITCH_W - 8.0 - ACTION_W * 2.0 - 4.0,
            card.origin.y + (card.size.y - SETTINGS_SWITCH_H) / 2.0,
        ),
        size: Point2D::new(SETTINGS_SWITCH_W, SETTINGS_SWITCH_H),
    }
}

pub(super) fn compact_edit_rect(card: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            compact_switch_rect(card).origin.x + SETTINGS_SWITCH_W + 8.0,
            card.origin.y + (card.size.y - ACTION_W) / 2.0,
        ),
        size: Point2D::new(ACTION_W, ACTION_W),
    }
}

pub(super) fn compact_remove_rect(card: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            compact_edit_rect(card).origin.x + ACTION_W + 4.0,
            card.origin.y + (card.size.y - ACTION_W) / 2.0,
        ),
        size: Point2D::new(ACTION_W, ACTION_W),
    }
}

pub(super) fn field_input_rect(
    settings: &AgentSettings,
    card: Rect,
    index: Option<usize>,
    row: usize,
) -> Rect {
    let menu_h = agent_settings_builtin_parts::preset_menu_height(settings, index);
    Rect {
        origin: Point2D::new(
            card.origin.x + 12.0 + FIELD_LABEL_W,
            card.origin.y + 76.0 + menu_h + row as f32 * 28.0,
        ),
        size: Point2D::new(card.size.x - 24.0 - FIELD_LABEL_W, FIELD_H),
    }
}
