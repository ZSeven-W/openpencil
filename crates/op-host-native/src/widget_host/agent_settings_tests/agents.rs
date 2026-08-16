//! Built-in (API-key) + ACP agent cards: focus/commit round trips,
//! compact-row actions, preset menus, and the draft save/cancel paths.
//!
//! Split out of `agent_settings_tests.rs` to keep every file under the
//! repo's 800-line cap.

use super::*;

use super::{VIEWPORT_H, VIEWPORT_W};

#[test]
fn close_press_sets_and_release_clears_agent_settings_button() {
    let mut host = WidgetHostNative::new();
    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
    let close = op_editor_ui::widgets::agent_settings_panel::close_button_rect(rect);
    let close_x = close.origin.x + close.size.x / 2.0;
    let close_y = close.origin.y + close.size.y / 2.0;

    assert!(host.dispatch_agent_settings_press(close_x, close_y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state().editor_ui.pressed_button,
        Some(ButtonPressTarget::AgentSettings(AgentSettingsButton::Close))
    );

    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state().editor_ui.pressed_button, None);
}

#[test]
fn builtin_agent_api_key_focus_accepts_text_and_rebuilds_models() {
    let mut host = WidgetHostNative::new();
    let id = host
        .editor_state_mut()
        .editor_ui
        .agent_settings
        .add_builtin_agent();
    host.editor_state_mut().rebuild_chat_models();
    assert!(
        !host
            .editor_state()
            .chat
            .available_models
            .iter()
            .any(|m| m.builtin_provider_id.as_deref() == Some(id.as_str())),
        "empty-key built-in agent must not be selectable yet"
    );

    host.editor_state_mut().editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
        index: 0,
        field: BuiltinAgentField::ApiKey,
    });
    host.editor_state_mut()
        .editor_ui
        .settings_input
        .set_text("");
    for c in "sk-test".chars() {
        assert!(host.apply_text(c));
    }
    assert!(host.apply_send());

    let state = host.editor_state();
    assert_eq!(
        state.editor_ui.agent_settings.builtin_agents[0].api_key,
        "sk-test"
    );
    assert!(state.editor_ui.agent_settings.focus.is_none());
    assert!(
        state
            .chat
            .available_models
            .iter()
            .any(|m| m.builtin_provider_id.as_deref() == Some(id.as_str())),
        "committing a ready built-in agent should add it to the chat model list"
    );
}

#[test]
fn editing_browser_owned_builtin_agent_transfers_it_to_operator_ownership() {
    let mut host = WidgetHostNative::new();
    let settings = &mut host.editor_state_mut().editor_ui.agent_settings;
    settings.add_builtin_agent_with_defaults("Browser", "sk-browser", "browser-model");
    settings.builtin_agents[0].id = "web-credential:builtin:builtin-web-1".into();
    settings.next_builtin_agent_id = 7;
    settings.focus = Some(SettingsFocus::BuiltinAgent {
        index: 0,
        field: BuiltinAgentField::ApiKey,
    });
    host.editor_state_mut()
        .editor_ui
        .settings_input
        .set_text("sk-operator");

    assert!(host.apply_send());

    let settings = &host.editor_state().editor_ui.agent_settings;
    assert_eq!(settings.builtin_agents[0].id, "builtin-7");
    assert_eq!(settings.builtin_agents[0].api_key, "sk-operator");
    assert_eq!(settings.next_builtin_agent_id, 8);
    assert!(host
        .editor_state()
        .chat
        .available_models
        .iter()
        .any(|model| model.builtin_provider_id.as_deref() == Some("builtin-7")));
}

#[test]
fn builtin_agent_compact_switch_toggles_enabled_and_models() {
    let mut host = WidgetHostNative::new();
    let id = host
        .editor_state_mut()
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("MiniMax", "sk-test", "MiniMax-M2.7");
    host.editor_state_mut().rebuild_chat_models();
    assert!(host
        .editor_state()
        .chat
        .available_models
        .iter()
        .any(|m| m.builtin_provider_id.as_deref() == Some(id.as_str())));

    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
    let content_w = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .size
        .x;
    let x = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .origin
        .x
        + content_w
        - 90.0;
    let y = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .origin
        .y
        + op_editor_ui::widgets::agent_settings_panel::AGENTS_HERO_HEIGHT
        + 28.0
        + 28.0
        + 30.0;
    assert!(host.dispatch_agent_settings_press(x, y, VIEWPORT_W, VIEWPORT_H));

    let state = host.editor_state();
    assert!(!state.editor_ui.agent_settings.builtin_agents[0].enabled);
    assert!(!state
        .chat
        .available_models
        .iter()
        .any(|m| m.builtin_provider_id.as_deref() == Some(id.as_str())));
}

#[test]
fn builtin_agent_compact_edit_focuses_display_name_form() {
    let mut host = WidgetHostNative::new();
    host.set_now_ms(1234);
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("MiniMax", "sk-test", "MiniMax-M2.7");
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .hover_builtin_agent = 0;

    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
    let content_w = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .size
        .x;
    let x = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .origin
        .x
        + content_w
        - 52.0;
    let y = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .origin
        .y
        + op_editor_ui::widgets::agent_settings_panel::AGENTS_HERO_HEIGHT
        + 28.0
        + 28.0
        + 30.0;
    assert!(host.dispatch_agent_settings_press(x, y, VIEWPORT_W, VIEWPORT_H));

    assert_eq!(
        host.editor_state().editor_ui.agent_settings.focus,
        Some(SettingsFocus::BuiltinAgent {
            index: 0,
            field: BuiltinAgentField::DisplayName,
        })
    );
    assert_eq!(
        host.editor_state().editor_ui.settings_input.text(),
        "MiniMax"
    );
    assert_eq!(
        host.editor_state()
            .editor_ui
            .settings_input
            .next_blink_flip_ms(1234),
        1734
    );
}

#[test]
fn builtin_agent_kind_toggle_commits_focused_api_key_draft() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .add_builtin_agent_config(
            "MiniMax",
            "",
            "MiniMax-M2.7",
            op_editor_core::agent_settings::BuiltinAgentKind::Anthropic,
            "https://api.minimaxi.com/anthropic",
        );
    host.editor_state_mut().editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
        index: 0,
        field: BuiltinAgentField::ApiKey,
    });
    host.editor_state_mut()
        .editor_ui
        .settings_input
        .set_text("sk-kind-toggle");

    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
    let content_x = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .origin
        .x;
    let content_y = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .origin
        .y;
    let content_w = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .size
        .x;
    let first_card_y =
        content_y + op_editor_ui::widgets::agent_settings_panel::AGENTS_HERO_HEIGHT + 28.0 + 28.0;
    let kind_x = content_x + content_w - 172.0 + 120.0;
    let kind_y = first_card_y + 22.0;

    assert!(host.dispatch_agent_settings_press(kind_x, kind_y, VIEWPORT_W, VIEWPORT_H));

    let agent = &host.editor_state().editor_ui.agent_settings.builtin_agents[0];
    assert_eq!(agent.api_key, "sk-kind-toggle");
    assert!(host.editor_state().editor_ui.agent_settings.focus.is_none());
}

#[test]
fn pure_builtin_agent_base_url_commit_is_ignored_like_ts_read_only_input() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .add_builtin_agent();
    host.editor_state_mut().editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
        index: 0,
        field: BuiltinAgentField::BaseUrl,
    });
    host.editor_state_mut()
        .editor_ui
        .settings_input
        .set_text("https://example.invalid");

    assert!(host.apply_send());

    let settings = &host.editor_state().editor_ui.agent_settings;
    assert_eq!(
        settings.builtin_agents[0].base_url,
        "https://api.anthropic.com"
    );
    assert!(settings.focus.is_none());
    assert!(host
        .editor_state()
        .editor_ui
        .settings_input
        .text()
        .is_empty());
}

#[test]
fn add_provider_opens_unsaved_builtin_agent_draft() {
    let mut host = WidgetHostNative::new();
    host.set_now_ms(1234);

    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
    let content_x = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .origin
        .x;
    let content_y = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .origin
        .y;
    let content_w = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .size
        .x;
    let add_x = content_x + content_w - 48.0;
    let add_y = content_y + op_editor_ui::widgets::agent_settings_panel::AGENTS_HERO_HEIGHT + 12.0;

    assert!(host.dispatch_agent_settings_press(add_x, add_y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state().editor_ui.pressed_button,
        Some(ButtonPressTarget::AgentSettings(
            AgentSettingsButton::AddProvider
        ))
    );

    let settings = &host.editor_state().editor_ui.agent_settings;
    assert!(settings.builtin_agents.is_empty());
    assert!(settings.builtin_agent_draft.is_some());
    assert_eq!(
        settings.focus,
        Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::ApiKey))
    );
    assert_eq!(host.editor_state().editor_ui.settings_input.text(), "");
    assert_eq!(
        host.editor_state()
            .editor_ui
            .settings_input
            .next_blink_flip_ms(1234),
        1734
    );

    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state().editor_ui.pressed_button, None);
}

#[test]
fn builtin_provider_menu_selects_ts_preset_for_draft() {
    let mut host = WidgetHostNative::new();
    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
    let content_x = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .origin
        .x;
    let content_y = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .origin
        .y;
    let content_w = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .size
        .x;
    let add_x = content_x + content_w - 48.0;
    let add_y = content_y + op_editor_ui::widgets::agent_settings_panel::AGENTS_HERO_HEIGHT + 12.0;

    assert!(host.dispatch_agent_settings_press(add_x, add_y, VIEWPORT_W, VIEWPORT_H));
    let card_y =
        content_y + op_editor_ui::widgets::agent_settings_panel::AGENTS_HERO_HEIGHT + 28.0 + 28.0;
    let provider_x = content_x + 68.0 + 24.0;
    let provider_y = card_y + 60.0;
    assert!(host.dispatch_agent_settings_press(provider_x, provider_y, VIEWPORT_W, VIEWPORT_H));
    let minimax_y = card_y + 76.0 + 4.0 + 5.0 * 24.0 + 12.0;
    assert!(host.dispatch_agent_settings_press(provider_x, minimax_y, VIEWPORT_W, VIEWPORT_H));

    let draft = host
        .editor_state()
        .editor_ui
        .agent_settings
        .builtin_agent_draft
        .as_ref()
        .expect("draft remains open");
    assert_eq!(draft.preset, BuiltinAgentPresetKey::MiniMax);
    assert_eq!(draft.display_name, "MiniMax");
    assert_eq!(draft.models, ["MiniMax-M3"]);
    assert_eq!(draft.base_url, "https://api.minimaxi.com/anthropic");
}

#[test]
fn save_builtin_agent_draft_persists_provider() {
    let mut host = WidgetHostNative::new();
    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
    let content_x = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .origin
        .x;
    let content_y = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .origin
        .y;
    let content_w = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .size
        .x;
    let add_x = content_x + content_w - 48.0;
    let add_y = content_y + op_editor_ui::widgets::agent_settings_panel::AGENTS_HERO_HEIGHT + 12.0;

    assert!(host.dispatch_agent_settings_press(add_x, add_y, VIEWPORT_W, VIEWPORT_H));
    for c in "sk-test".chars() {
        assert!(host.apply_text(c));
    }
    let card_y =
        content_y + op_editor_ui::widgets::agent_settings_panel::AGENTS_HERO_HEIGHT + 28.0 + 28.0;
    let save_x = content_x + content_w - 12.0 - 34.0;
    let save_y = card_y + 196.0 + 18.0;
    assert!(host.dispatch_agent_settings_press(save_x, save_y, VIEWPORT_W, VIEWPORT_H));

    let settings = &host.editor_state().editor_ui.agent_settings;
    assert_eq!(settings.builtin_agents.len(), 1);
    assert!(settings.builtin_agent_draft.is_none());
    assert_eq!(settings.builtin_agents[0].api_key, "sk-test");
    assert!(settings.focus.is_none());
}

#[test]
fn add_acp_agent_press_opens_unsaved_draft() {
    let mut host = WidgetHostNative::new();
    host.set_now_ms(1234);
    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
    let content_x = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .origin
        .x;
    let content_y = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .origin
        .y;
    let content_w = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .size
        .x;
    let add_x = content_x + content_w - 12.0 - 48.0;
    let add_y = content_y
        + op_editor_ui::widgets::agent_settings_panel::AGENTS_HERO_HEIGHT
        + 120.0
        + 28.0
        + 12.0;

    assert!(host.dispatch_agent_settings_press(add_x, add_y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state().editor_ui.pressed_button,
        Some(ButtonPressTarget::AgentSettings(
            AgentSettingsButton::AddAcpAgent
        ))
    );

    let settings = &host.editor_state().editor_ui.agent_settings;
    assert!(settings.acp_agents.is_empty());
    assert!(settings.acp_agent_draft.is_some());
    assert_eq!(
        settings.focus,
        Some(SettingsFocus::AcpAgentDraft(AcpAgentField::Command))
    );
    assert_eq!(
        host.editor_state()
            .editor_ui
            .settings_input
            .next_blink_flip_ms(1234),
        1734
    );

    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state().editor_ui.pressed_button, None);
}

#[test]
fn save_acp_agent_draft_persists_agent() {
    let mut host = WidgetHostNative::new();
    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
    let content_x = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .origin
        .x;
    let content_y = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .origin
        .y;
    let content_w = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .size
        .x;
    let add_x = content_x + content_w - 12.0 - 48.0;
    let add_y = content_y
        + op_editor_ui::widgets::agent_settings_panel::AGENTS_HERO_HEIGHT
        + 120.0
        + 28.0
        + 12.0;

    assert!(host.dispatch_agent_settings_press(add_x, add_y, VIEWPORT_W, VIEWPORT_H));
    for c in "op-agent".chars() {
        assert!(host.apply_text(c));
    }
    let card_y = acp_card_y(content_y);
    let save_x = content_x + content_w - 12.0 - 34.0;
    let save_y = card_y + 332.0 + 18.0;
    assert!(host.dispatch_agent_settings_press(save_x, save_y, VIEWPORT_W, VIEWPORT_H));

    let settings = &host.editor_state().editor_ui.agent_settings;
    assert_eq!(settings.acp_agents.len(), 1);
    assert!(settings.acp_agent_draft.is_none());
    assert_eq!(settings.acp_agents[0].command, "op-agent");
    assert!(settings.focus.is_none());
}

#[test]
fn cancel_acp_agent_draft_discards_unsaved_agent() {
    let mut host = WidgetHostNative::new();
    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
    let content_x = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .origin
        .x;
    let content_y = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .origin
        .y;
    let content_w = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect)
        .size
        .x;
    let add_x = content_x + content_w - 12.0 - 48.0;
    let add_y = content_y
        + op_editor_ui::widgets::agent_settings_panel::AGENTS_HERO_HEIGHT
        + 120.0
        + 28.0
        + 12.0;

    assert!(host.dispatch_agent_settings_press(add_x, add_y, VIEWPORT_W, VIEWPORT_H));
    for c in "op-agent".chars() {
        assert!(host.apply_text(c));
    }
    let card_y = acp_card_y(content_y);
    let cancel_x = content_x + content_w - 12.0 - 68.0 - 8.0 - 34.0;
    let cancel_y = card_y + 332.0 + 18.0;
    assert!(host.dispatch_agent_settings_press(cancel_x, cancel_y, VIEWPORT_W, VIEWPORT_H));

    let settings = &host.editor_state().editor_ui.agent_settings;
    assert!(settings.acp_agents.is_empty());
    assert!(settings.acp_agent_draft.is_none());
    assert!(settings.focus.is_none());
}

#[test]
fn acp_agent_compact_edit_focuses_display_name_form() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .add_acp_agent();
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .hover_acp_agent = 0;

    let (content_x, content_y, content_w) = agent_settings_content_metrics(&host);
    let card_y = acp_card_y(content_y);
    assert!(host.dispatch_agent_settings_press(
        content_x + content_w - 156.0,
        card_y + 30.0,
        VIEWPORT_W,
        VIEWPORT_H
    ));

    assert_eq!(
        host.editor_state().editor_ui.agent_settings.focus,
        Some(SettingsFocus::AcpAgent {
            index: 0,
            field: AcpAgentField::DisplayName,
        })
    );
    assert_eq!(
        host.editor_state().editor_ui.settings_input.text(),
        "ACP Agent 1"
    );
}
