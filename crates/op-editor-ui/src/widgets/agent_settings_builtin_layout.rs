use crate::widgets::agent_settings_builtin_model_menu;
use crate::widgets::agent_settings_builtin_parts;
use crate::widgets::agent_settings_header_action::header_action_rect;
use crate::widgets::agent_settings_switch::{SETTINGS_SWITCH_H, SETTINGS_SWITCH_W};
use crate::{Point2D, Rect};
use op_editor_core::agent_settings::{AgentSettings, SettingsFocus};

use crate::widgets::agent_settings_metrics::{
    EMPTY_BLOCK_H, ROW_H_TWO_LINE, ROW_PAD_X, SECTION_HEADER_H, SECTION_SUBTITLE_H,
};

pub(super) const HEADER_HEIGHT: f32 = SECTION_HEADER_H;
pub(super) const SUBTITLE_HEIGHT: f32 = SECTION_SUBTITLE_H;
pub(super) const SYNC_ERROR_HEIGHT: f32 = 22.0;

/// Extra row reserved under the subtitle when the browser→daemon credential
/// sync reported a failure. Every y-walk (paint + hit-test + heights) must
/// add this so cards stay aligned while the status row shows.
pub(super) fn sync_error_height(settings: &AgentSettings) -> f32 {
    if settings.web_credential_sync_error.is_some() {
        SYNC_ERROR_HEIGHT
    } else {
        0.0
    }
}
pub(super) const EMPTY_HEIGHT: f32 = EMPTY_BLOCK_H;
/// A saved agent that is not being edited is a LIST ROW, not a card: the
/// modal's two-line row box, a hairline under it, and a hover wash. It
/// used to be a bordered, tinted 60 px card, which is what made three
/// configured agents fill half the tab.
const COMPACT_CARD_HEIGHT: f32 = ROW_H_TWO_LINE;
const EXPANDED_CARD_HEIGHT: f32 = 196.0;
const TOUCH_EXPANDED_CARD_HEIGHT: f32 = 376.0;
const DRAFT_ACTION_HEIGHT: f32 = 36.0;
const TOUCH_DRAFT_ACTION_HEIGHT: f32 = 60.0;
/// Rows sit flush — the hairline IS the gap. Only the expanded edit form,
/// which is still a card, wants air around it, and it gets that from the
/// row it replaces being taller.
pub(super) const CARD_GAP: f32 = 0.0;
const FIELD_LABEL_W: f32 = 68.0;
const FIELD_H: f32 = 24.0;
const MODEL_EDITOR_H: f32 = 68.0;
const TOUCH_FIELD_LABEL_W: f32 = 84.0;
const TOUCH_FIELD_H: f32 = 44.0;
const TOUCH_MODEL_EDITOR_H: f32 = 104.0;
const TOUCH_FIELD_START_Y: f32 = 160.0;
const TOUCH_FIELD_ROW_H: f32 = 52.0;
const ACTION_W: f32 = 24.0;
const TOUCH_TARGET: f32 = 44.0;
pub(super) const TOUCH_EMPTY_CARD_H: f32 = 72.0;
pub(super) const TOUCH_EMPTY_CTA_W: f32 = 132.0;

pub(super) fn is_editing(settings: &AgentSettings, index: usize) -> bool {
    matches!(
        settings.focus,
        Some(SettingsFocus::BuiltinAgent { index: i, .. }) if i == index
    )
}

pub(super) fn card_height_for_ui(settings: &AgentSettings, index: usize, touch: bool) -> f32 {
    if is_editing(settings, index) {
        expanded_card_height_for_ui(settings, Some(index), touch)
    } else {
        COMPACT_CARD_HEIGHT
    }
}

pub(super) fn expanded_card_height_for_ui(
    settings: &AgentSettings,
    index: Option<usize>,
    touch: bool,
) -> f32 {
    let base = if touch {
        TOUCH_EXPANDED_CARD_HEIGHT
    } else {
        EXPANDED_CARD_HEIGHT
    };
    // Editing an existing provider carries a delete row at the bottom of
    // the form (a new draft has nothing to delete yet).
    let delete = if index.is_some() {
        delete_row_height(touch)
    } else {
        0.0
    };
    base + delete
        + model_editor_extra_height(settings, index, touch)
        + agent_settings_builtin_parts::preset_menu_height(settings, index, touch)
        + agent_settings_builtin_model_menu::model_menu_height(settings, index, touch)
}

pub(super) fn delete_row_height(touch: bool) -> f32 {
    if touch {
        TOUCH_TARGET + 8.0
    } else {
        32.0
    }
}

/// Bottom-of-form "Delete provider" target while editing a saved card.
pub(super) fn editing_delete_rect(card: Rect, touch: bool) -> Rect {
    let row = delete_row_height(touch);
    let pad = if touch { 16.0 } else { 12.0 };
    Rect {
        origin: Point2D::new(card.origin.x + pad, card.origin.y + card.size.y - row),
        size: Point2D::new(card.size.x - pad * 2.0, row - 6.0),
    }
}

fn model_editor_expanded(settings: &AgentSettings, index: Option<usize>) -> bool {
    match index {
        Some(index) => matches!(
            settings.focus,
            Some(SettingsFocus::BuiltinAgent {
                index: focused,
                field: op_editor_core::agent_settings::BuiltinAgentField::Model,
            }) if focused == index
        ),
        None => matches!(
            settings.focus,
            Some(SettingsFocus::BuiltinAgentDraft(
                op_editor_core::agent_settings::BuiltinAgentField::Model,
            ))
        ),
    }
}

fn model_editor_extra_height(settings: &AgentSettings, index: Option<usize>, touch: bool) -> f32 {
    if !model_editor_expanded(settings, index) {
        return 0.0;
    }
    if touch {
        TOUCH_MODEL_EDITOR_H - TOUCH_FIELD_H
    } else {
        MODEL_EDITOR_H - FIELD_H
    }
}

pub(super) fn draft_card_height_for_ui(settings: &AgentSettings, touch: bool) -> f32 {
    expanded_card_height_for_ui(settings, None, touch)
        + if touch {
            TOUCH_DRAFT_ACTION_HEIGHT
        } else {
            DRAFT_ACTION_HEIGHT
        }
}

pub(super) fn add_provider_rect(content: Rect, y: f32) -> Rect {
    header_action_rect(content, y)
}

pub(super) fn add_provider_touch_target(content: Rect, y: f32) -> Rect {
    let visual = add_provider_rect(content, y);
    Rect {
        origin: visual.origin,
        size: Point2D::new(visual.size.x, TOUCH_TARGET),
    }
}

pub(super) fn touch_empty_card_rect(content: Rect, y: f32) -> Rect {
    Rect {
        origin: Point2D::new(content.origin.x, y),
        size: Point2D::new(content.size.x, TOUCH_EMPTY_CARD_H),
    }
}

pub(super) fn touch_empty_cta_rect(content: Rect, y: f32) -> Rect {
    let card = touch_empty_card_rect(content, y);
    Rect {
        origin: Point2D::new(
            card.origin.x + card.size.x - TOUCH_EMPTY_CTA_W - 12.0,
            card.origin.y + (card.size.y - TOUCH_TARGET) / 2.0,
        ),
        size: Point2D::new(TOUCH_EMPTY_CTA_W, TOUCH_TARGET),
    }
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
            card.origin.x + card.size.x
                - ROW_PAD_X
                - SETTINGS_SWITCH_W
                - 8.0
                - ACTION_W * 2.0
                - 4.0,
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

fn trailing_touch_target(card: Rect, slot_from_right: usize) -> Rect {
    Rect {
        origin: Point2D::new(
            card.origin.x + card.size.x - (slot_from_right + 1) as f32 * TOUCH_TARGET,
            card.origin.y + (card.size.y - TOUCH_TARGET) / 2.0,
        ),
        size: Point2D::new(TOUCH_TARGET, TOUCH_TARGET),
    }
}

pub(super) fn compact_touch_switch_target(card: Rect) -> Rect {
    trailing_touch_target(card, 2)
}

pub(super) fn compact_touch_edit_target(card: Rect) -> Rect {
    trailing_touch_target(card, 1)
}

pub(super) fn compact_touch_remove_target(card: Rect) -> Rect {
    trailing_touch_target(card, 0)
}

pub(super) fn compact_touch_switch_rect(card: Rect) -> Rect {
    let target = compact_touch_switch_target(card);
    Rect {
        origin: Point2D::new(
            target.origin.x + (target.size.x - SETTINGS_SWITCH_W) / 2.0,
            target.origin.y + (target.size.y - SETTINGS_SWITCH_H) / 2.0,
        ),
        size: Point2D::new(SETTINGS_SWITCH_W, SETTINGS_SWITCH_H),
    }
}

pub(super) fn compact_touch_edit_rect(card: Rect) -> Rect {
    centered_action_rect(compact_touch_edit_target(card))
}

pub(super) fn compact_touch_remove_rect(card: Rect) -> Rect {
    centered_action_rect(compact_touch_remove_target(card))
}

fn centered_action_rect(target: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            target.origin.x + (target.size.x - ACTION_W) / 2.0,
            target.origin.y + (target.size.y - ACTION_W) / 2.0,
        ),
        size: Point2D::new(ACTION_W, ACTION_W),
    }
}

pub(super) fn field_input_rect_for_ui(
    settings: &AgentSettings,
    card: Rect,
    index: Option<usize>,
    row: usize,
    touch: bool,
) -> Rect {
    // The provider-preset menu pushes every field down; the model menu
    // only pushes the rows below the Model field it is anchored to.
    let preset_menu_h = agent_settings_builtin_parts::preset_menu_height(settings, index, touch);
    let below_model = row > agent_settings_builtin_model_menu::MODEL_FIELD_ROW;
    let model_editor_h = if below_model {
        model_editor_extra_height(settings, index, touch)
    } else {
        0.0
    };
    let model_menu_h = if below_model {
        agent_settings_builtin_model_menu::model_menu_height(settings, index, touch)
    } else {
        0.0
    };
    let (pad_x, label_w, start_y, row_h, field_h) = if touch {
        (
            16.0,
            TOUCH_FIELD_LABEL_W,
            TOUCH_FIELD_START_Y,
            TOUCH_FIELD_ROW_H,
            TOUCH_FIELD_H,
        )
    } else {
        (12.0, FIELD_LABEL_W, 76.0, 28.0, FIELD_H)
    };
    let field_h = if row == agent_settings_builtin_model_menu::MODEL_FIELD_ROW
        && model_editor_expanded(settings, index)
    {
        if touch {
            TOUCH_MODEL_EDITOR_H
        } else {
            MODEL_EDITOR_H
        }
    } else {
        field_h
    };
    Rect {
        origin: Point2D::new(
            card.origin.x + pad_x + label_w,
            card.origin.y
                + start_y
                + preset_menu_h
                + model_editor_h
                + model_menu_h
                + row as f32 * row_h,
        ),
        size: Point2D::new(card.size.x - pad_x * 2.0 - label_w, field_h),
    }
}
