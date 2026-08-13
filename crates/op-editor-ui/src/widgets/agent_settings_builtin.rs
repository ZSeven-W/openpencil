//! Built-in provider section for the Agent settings panel.

use crate::theme::Theme;
use crate::widgets::agent_settings_builtin_layout::{
    add_provider_rect, add_provider_touch_target, card_height_for_ui, card_rect, compact_edit_rect,
    compact_remove_rect, compact_switch_rect, compact_touch_edit_rect, compact_touch_edit_target,
    compact_touch_remove_rect, compact_touch_remove_target, compact_touch_switch_rect,
    compact_touch_switch_target, draft_card_height_for_ui, expanded_card_height_for_ui,
    field_input_rect_for_ui, is_editing, sync_error_height, touch_empty_cta_rect, CARD_GAP,
    EMPTY_HEIGHT, HEADER_HEIGHT, SUBTITLE_HEIGHT, TOUCH_EMPTY_CARD_H,
};
use crate::widgets::agent_settings_builtin_parts;
use crate::widgets::agent_settings_form_actions::{
    cancel_button_rect_for_ui, paint_form_actions_for_ui, save_button_rect_for_ui,
};
use crate::widgets::agent_settings_header_action::{
    fit_header_copy, header_action_rect, header_action_text_baseline_y, header_action_text_x,
};
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::agent_settings_metrics::{ROW_AVATAR, ROW_PAD_X, ROW_TEXT_X};
use crate::widgets::agent_settings_panel_geometry::agents_body_top;
use crate::widgets::agent_settings_rows::{
    paint_row_hairline, paint_row_label_above_status_at, paint_row_status_line_at_fitted,
    paint_section_title,
};
use crate::widgets::agent_settings_switch::paint_settings_switch;
use crate::widgets::button::paint_ghost_button_feedback;
use crate::widgets::icons::Icon;
use crate::widgets::settings_form::{self, paint_action};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::agent_settings::{
    AgentSettings, BuiltinAgentConfig, BuiltinAgentField, BuiltinAgentPresetMenuTarget,
};
use op_editor_core::editor_ui_state::EditorUiState;
use op_editor_core::{AgentSettingsButton, BuiltinAgentPresetKey, ButtonPressTarget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinHit {
    AddProvider,
    Focus {
        index: usize,
        field: BuiltinAgentField,
    },
    FocusDraft(BuiltinAgentField),
    ToggleKind(usize),
    ToggleDraftKind,
    TogglePresetMenu(Option<usize>),
    SelectPreset {
        index: Option<usize>,
        preset: BuiltinAgentPresetKey,
    },
    SaveDraft,
    CancelDraft,
    ToggleEnabled(usize),
    Edit(usize),
    Remove(usize),
    None,
}

pub fn content_height(settings: &AgentSettings) -> f32 {
    content_height_for_ui(settings, false)
}

pub fn content_height_for_ui(settings: &AgentSettings, touch: bool) -> f32 {
    let has_draft = settings.builtin_agent_draft.is_some();
    let list_h = if settings.builtin_agents.is_empty() && !has_draft {
        if touch {
            TOUCH_EMPTY_CARD_H
        } else {
            EMPTY_HEIGHT
        }
    } else {
        let saved_h: f32 = settings
            .builtin_agents
            .iter()
            .enumerate()
            .map(|(index, _)| card_height_for_ui(settings, index, touch) + CARD_GAP)
            .sum();
        saved_h
            + if has_draft {
                draft_card_height_for_ui(settings, touch) + CARD_GAP
            } else {
                0.0
            }
    };
    HEADER_HEIGHT + SUBTITLE_HEIGHT + sync_error_height(settings) + list_h
}

pub fn hit_test(content: Rect, settings: &AgentSettings, point: Point2D) -> BuiltinHit {
    hit_test_with_touch(content, settings, point, false)
}

pub fn hit_test_for_ui(
    content: Rect,
    settings: &AgentSettings,
    ui: &EditorUiState,
    point: Point2D,
) -> BuiltinHit {
    hit_test_with_touch(content, settings, point, ui.touch_chrome())
}

fn hit_test_with_touch(
    content: Rect,
    settings: &AgentSettings,
    point: Point2D,
    touch: bool,
) -> BuiltinHit {
    let y = agents_body_top(content);
    let empty = settings.builtin_agents.is_empty() && settings.builtin_agent_draft.is_none();
    let add_target = if touch && empty {
        touch_empty_cta_rect(
            content,
            y + HEADER_HEIGHT + SUBTITLE_HEIGHT + sync_error_height(settings),
        )
    } else if touch {
        add_provider_touch_target(content, y)
    } else {
        add_provider_rect(content, y)
    };
    if add_target.contains(point) {
        return BuiltinHit::AddProvider;
    }
    let mut card_y = y + HEADER_HEIGHT + SUBTITLE_HEIGHT + sync_error_height(settings);
    for (index, agent) in settings.builtin_agents.iter().enumerate() {
        let card = card_rect(
            content.origin.x,
            card_y,
            content.size.x,
            card_height_for_ui(settings, index, touch),
        );
        if is_editing(settings, index) {
            if (agent_settings_builtin_parts::provider_select_rect(card, touch)).contains(point) {
                return BuiltinHit::TogglePresetMenu(Some(index));
            }
            if settings.builtin_preset_menu_open == Some(BuiltinAgentPresetMenuTarget::Agent(index))
            {
                if let Some(preset) = agent_settings_builtin_parts::preset_at(
                    card,
                    point,
                    settings.builtin_preset_menu_scroll.offset,
                    touch,
                ) {
                    return BuiltinHit::SelectPreset {
                        index: Some(index),
                        preset,
                    };
                }
            }
            if agent_settings_builtin_parts::kind_toggle_target(agent, card, point, touch).is_some()
            {
                return BuiltinHit::ToggleKind(index);
            }
            for (row, field) in [
                BuiltinAgentField::DisplayName,
                BuiltinAgentField::ApiKey,
                BuiltinAgentField::Model,
                BuiltinAgentField::BaseUrl,
            ]
            .into_iter()
            .enumerate()
            {
                if field == BuiltinAgentField::BaseUrl && !agent.base_url_editable() {
                    continue;
                }
                if (field_input_rect_for_ui(settings, card, Some(index), row, touch))
                    .contains(point)
                {
                    return BuiltinHit::Focus { index, field };
                }
            }
        } else if touch && compact_touch_switch_target(card).contains(point) {
            return BuiltinHit::ToggleEnabled(index);
        } else if touch && compact_touch_edit_target(card).contains(point) {
            return BuiltinHit::Edit(index);
        } else if touch && compact_touch_remove_target(card).contains(point) {
            return BuiltinHit::Remove(index);
        } else if compact_switch_rect(card).contains(point) {
            return BuiltinHit::ToggleEnabled(index);
        } else if settings.hover_builtin_agent == index && (compact_edit_rect(card)).contains(point)
        {
            return BuiltinHit::Edit(index);
        } else if settings.hover_builtin_agent == index
            && (compact_remove_rect(card)).contains(point)
        {
            return BuiltinHit::Remove(index);
        }
        card_y += card.size.y + CARD_GAP;
    }
    if let Some(agent) = settings.builtin_agent_draft.as_ref() {
        let card = card_rect(
            content.origin.x,
            card_y,
            content.size.x,
            draft_card_height_for_ui(settings, touch),
        );
        if (agent_settings_builtin_parts::provider_select_rect(card, touch)).contains(point) {
            return BuiltinHit::TogglePresetMenu(None);
        }
        if settings.builtin_preset_menu_open == Some(BuiltinAgentPresetMenuTarget::Draft) {
            if let Some(preset) = agent_settings_builtin_parts::preset_at(
                card,
                point,
                settings.builtin_preset_menu_scroll.offset,
                touch,
            ) {
                return BuiltinHit::SelectPreset {
                    index: None,
                    preset,
                };
            }
        }
        if agent_settings_builtin_parts::kind_toggle_target(agent, card, point, touch).is_some() {
            return BuiltinHit::ToggleDraftKind;
        }
        for (row, field) in [
            BuiltinAgentField::DisplayName,
            BuiltinAgentField::ApiKey,
            BuiltinAgentField::Model,
            BuiltinAgentField::BaseUrl,
        ]
        .into_iter()
        .enumerate()
        {
            if field == BuiltinAgentField::BaseUrl && !agent.base_url_editable() {
                continue;
            }
            if (field_input_rect_for_ui(settings, card, None, row, touch)).contains(point) {
                return BuiltinHit::FocusDraft(field);
            }
        }
        let form_h = expanded_card_height_for_ui(settings, None, touch);
        if (save_button_rect_for_ui(card, form_h, touch)).contains(point) {
            return BuiltinHit::SaveDraft;
        }
        if (cancel_button_rect_for_ui(card, form_h, touch)).contains(point) {
            return BuiltinHit::CancelDraft;
        }
    }
    BuiltinHit::None
}

pub fn card_at(content: Rect, settings: &AgentSettings, point: Point2D) -> Option<usize> {
    card_at_for_ui(content, settings, point, false)
}

pub fn card_at_for_ui(
    content: Rect,
    settings: &AgentSettings,
    point: Point2D,
    touch: bool,
) -> Option<usize> {
    settings_form::card_index_at(
        content.origin.x,
        content.size.x,
        agents_body_top(content) + HEADER_HEIGHT + SUBTITLE_HEIGHT + sync_error_height(settings),
        CARD_GAP,
        (0..settings.builtin_agents.len()).map(|index| card_height_for_ui(settings, index, touch)),
        point,
    )
}

pub fn preset_hover_at(
    content: Rect,
    settings: &AgentSettings,
    point: Point2D,
) -> Option<BuiltinAgentPresetKey> {
    preset_hover_at_for_ui(content, settings, point, false)
}

pub fn preset_hover_at_for_ui(
    content: Rect,
    settings: &AgentSettings,
    point: Point2D,
    touch: bool,
) -> Option<BuiltinAgentPresetKey> {
    let card = open_preset_menu_card(content, settings, point, touch)?;
    agent_settings_builtin_parts::preset_hover_at(
        card,
        point,
        settings.builtin_preset_menu_scroll.offset,
        touch,
    )
}

pub fn preset_scroll_max_at(
    content: Rect,
    settings: &AgentSettings,
    point: Point2D,
) -> Option<f32> {
    preset_scroll_max_at_for_ui(content, settings, point, false)
}

pub fn preset_scroll_max_at_for_ui(
    content: Rect,
    settings: &AgentSettings,
    point: Point2D,
    touch: bool,
) -> Option<f32> {
    let _ = open_preset_menu_card(content, settings, point, touch)?;
    Some(agent_settings_builtin_parts::preset_scroll_max(touch))
}

fn open_preset_menu_card(
    content: Rect,
    settings: &AgentSettings,
    point: Point2D,
    touch: bool,
) -> Option<Rect> {
    let mut card_y =
        agents_body_top(content) + HEADER_HEIGHT + SUBTITLE_HEIGHT + sync_error_height(settings);
    for (index, _) in settings.builtin_agents.iter().enumerate() {
        let card = card_rect(
            content.origin.x,
            card_y,
            content.size.x,
            card_height_for_ui(settings, index, touch),
        );
        if settings.builtin_preset_menu_open == Some(BuiltinAgentPresetMenuTarget::Agent(index))
            && agent_settings_builtin_parts::preset_menu_contains(card, point, touch)
        {
            return Some(card);
        }
        card_y += card.size.y + CARD_GAP;
    }
    if settings.builtin_preset_menu_open == Some(BuiltinAgentPresetMenuTarget::Draft)
        && settings.builtin_agent_draft.is_some()
    {
        let card = card_rect(
            content.origin.x,
            card_y,
            content.size.x,
            draft_card_height_for_ui(settings, touch),
        );
        if agent_settings_builtin_parts::preset_menu_contains(card, point, touch) {
            return Some(card);
        }
    }
    None
}

pub fn paint_builtin_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    content: Rect,
    y: f32,
    now_ms: u64,
) -> f32 {
    let empty = settings.builtin_agents.is_empty() && settings.builtin_agent_draft.is_none();
    let header_action = if ui.touch_chrome() && empty {
        ""
    } else {
        t_settings(ui, "settings.agents.addProvider")
    };
    let mut y = paint_header(
        cx,
        theme,
        t_settings(ui, "settings.agents.builtin"),
        header_action,
        HeaderFrame {
            x: content.origin.x,
            y,
            w: content.size.x,
        },
        settings.hover_add_provider && !(ui.touch_chrome() && empty),
        ui.button_pressed(ButtonPressTarget::AgentSettings(
            AgentSettingsButton::AddProvider,
        )),
    );
    y = settings_form::paint_subtitle(
        cx,
        theme,
        t_settings(ui, "settings.agents.builtinSubtitle"),
        content.origin.x,
        y,
        content.size.x,
    );
    if let Some(error) = settings.web_credential_sync_error.as_deref() {
        let text = format!("{} {error}", t_settings(ui, "settings.agents.syncError"));
        let layout = TextLayout::single_run(
            &text,
            "system-ui",
            12.0,
            (theme.destructive).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&layout, Point2D::new(content.origin.x, y + 14.0));
        y += sync_error_height(settings);
    }
    if empty && ui.touch_chrome() {
        return crate::widgets::agent_settings_builtin_empty::paint(cx, theme, ui, content, y);
    }
    if empty {
        return settings_form::paint_empty(
            cx,
            theme,
            t_settings(ui, "settings.agents.builtinEmpty"),
            content.origin.x,
            y,
            content.size.x,
        );
    }
    for (index, agent) in settings.builtin_agents.iter().enumerate() {
        let card = card_rect(
            content.origin.x,
            y,
            content.size.x,
            card_height_for_ui(settings, index, ui.touch_chrome()),
        );
        paint_builtin_agent_card(cx, theme, settings, ui, agent, index, card, now_ms);
        y += card.size.y + CARD_GAP;
    }
    if let Some(draft) = settings.builtin_agent_draft.as_ref() {
        let card = card_rect(
            content.origin.x,
            y,
            content.size.x,
            draft_card_height_for_ui(settings, ui.touch_chrome()),
        );
        crate::widgets::agent_settings_builtin_form::paint_builtin_agent_form(
            cx, theme, settings, ui, draft, None, card, now_ms,
        );
        let form_h = expanded_card_height_for_ui(settings, None, ui.touch_chrome());
        paint_form_actions_for_ui(
            cx,
            theme,
            ui,
            card,
            form_h,
            ui.builtin_agent_draft_ready(),
            ui.button_pressed(ButtonPressTarget::AgentSettings(
                AgentSettingsButton::BuiltinCancelDraft,
            )),
            ui.button_pressed(ButtonPressTarget::AgentSettings(
                AgentSettingsButton::BuiltinSaveDraft,
            )),
            ui.touch_chrome(),
        );
        y += card.size.y + CARD_GAP;
    }
    y
}

#[derive(Debug, Clone, Copy)]
struct HeaderFrame {
    x: f32,
    y: f32,
    w: f32,
}

fn paint_header(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    title: &str,
    action: &str,
    frame: HeaderFrame,
    action_hover: bool,
    action_pressed: bool,
) -> f32 {
    let copy = fit_header_copy(cx, title, action, frame.w);
    paint_section_title(cx, theme, frame.x, frame.y, &copy.title);
    let action_rect = header_action_rect(
        Rect {
            origin: Point2D::new(frame.x, frame.y),
            size: Point2D::new(frame.w, HEADER_HEIGHT),
        },
        frame.y,
    );
    paint_ghost_button_feedback(cx.backend, theme, action_rect, action_hover, action_pressed);
    let act = TextLayout::single_run(
        &copy.action,
        "system-ui",
        12.0,
        (theme.primary).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &act,
        Point2D::new(
            header_action_text_x(action_rect, copy.action_w),
            header_action_text_baseline_y(action_rect),
        ),
    );
    frame.y + HEADER_HEIGHT
}

#[allow(clippy::too_many_arguments)]
fn paint_builtin_agent_card(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    agent: &BuiltinAgentConfig,
    index: usize,
    card: Rect,
    now_ms: u64,
) {
    if !is_editing(settings, index) {
        paint_compact_builtin_agent_card(cx, theme, settings, ui, agent, index, card);
        return;
    }
    crate::widgets::agent_settings_builtin_form::paint_builtin_agent_form(
        cx,
        theme,
        settings,
        ui,
        agent,
        Some(index),
        card,
        now_ms,
    );
}

/// A saved agent, not being edited: a hairline-separated list row in the
/// modal's shared row language, not a tinted card. Readiness rides the
/// row's own status line — dot plus text, the same shape the MCP server
/// row and the System auto-update row use — instead of the third line it
/// used to add under the detail.
fn paint_compact_builtin_agent_card(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    agent: &BuiltinAgentConfig,
    index: usize,
    card: Rect,
) {
    let hovered = settings.hover_builtin_agent == index;
    if hovered {
        cx.backend.fill_round_rect(card, 8.0, theme.button_hover);
    }
    let last = index + 1 == settings.builtin_agents.len() && settings.builtin_agent_draft.is_none();
    if !last {
        paint_row_hairline(cx, theme, card);
    }
    let avatar = Rect {
        origin: Point2D::new(
            card.origin.x + ROW_PAD_X,
            card.origin.y + (card.size.y - ROW_AVATAR) / 2.0,
        ),
        size: Point2D::new(ROW_AVATAR, ROW_AVATAR),
    };
    cx.backend.fill_round_rect(avatar, 8.0, theme.background);
    agent_settings_builtin_parts::paint_key_glyph(cx, theme, avatar);

    let text_x = card.origin.x + ROW_TEXT_X;
    // Everything to the right of the text column: switch, then the two
    // hover actions, plus their gaps and the row's own inset.
    let switch_rect = if ui.touch_chrome() {
        compact_touch_switch_rect(card)
    } else {
        compact_switch_rect(card)
    };
    let reserved = card.origin.x + card.size.x - switch_rect.origin.x;
    paint_row_label_above_status_at(cx, theme, card, text_x, &agent.display_name, reserved);

    let ready = agent.ready();
    let api_key = if agent.api_key.trim().is_empty() {
        "api key required".to_string()
    } else {
        mask_key(&agent.api_key)
    };
    paint_row_status_line_at_fitted(
        cx,
        card,
        text_x,
        &format!("{}  ·  {api_key}", agent.model),
        if ready {
            theme.status_success
        } else {
            theme.muted_foreground
        },
        reserved,
    );

    paint_settings_switch(cx, theme, switch_rect, agent.enabled);
    if hovered || ui.touch_chrome() {
        let edit_rect = if ui.touch_chrome() {
            compact_touch_edit_rect(card)
        } else {
            compact_edit_rect(card)
        };
        let remove_rect = if ui.touch_chrome() {
            compact_touch_remove_rect(card)
        } else {
            compact_remove_rect(card)
        };
        paint_action(
            cx,
            theme,
            edit_rect,
            Icon::Pencil,
            theme.muted_foreground,
            ui.button_pressed(ButtonPressTarget::AgentSettings(
                AgentSettingsButton::BuiltinEdit(index),
            )),
        );
        paint_action(
            cx,
            theme,
            remove_rect,
            Icon::Trash,
            theme.muted_foreground,
            ui.button_pressed(ButtonPressTarget::AgentSettings(
                AgentSettingsButton::BuiltinRemove(index),
            )),
        );
    }
}

fn mask_key(api_key: &str) -> String {
    if api_key.len() > 12 {
        format!("{}***{}", &api_key[..7], &api_key[api_key.len() - 3..])
    } else {
        "***".to_string()
    }
}
