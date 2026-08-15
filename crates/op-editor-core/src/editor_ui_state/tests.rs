//! Tests for the editor-UI overlay state.
//!
//! Split out of the `editor_ui_state` spine (800-line file ceiling).

use super::*;

#[test]
fn default_editor_ui_is_quiescent() {
    let c = EditorUiState::new();
    assert!(c.sidebar_open);
    assert_eq!(c.theme_mode, ThemeMode::Dark);
    assert_eq!(c.locale, Locale::ZhCn);
    assert!(!c.file_menu_open);
    assert!(!c.export_dialog_open);
    assert!(!c.agent_settings_open);
    assert_eq!(c.shape_tool, Tool::Rect);
    assert_eq!(c.property_tab, PropertyTab::Design);
    assert_eq!(c.flex_layout, FlexLayout::Free);
    assert!(c.recent_files.is_empty());
    assert!(c.collapsed_layers.is_empty());
}

#[test]
fn editor_pixel_scroll_fields_use_scroll_state() {
    let mut s = EditorUiState::default();

    s.property_panel_scroll.offset = 12.0;
    s.layer_pages_scroll.offset = 24.0;
    s.layer_layers_scroll.offset = 36.0;
    s.layer_pages_h_scroll.offset = 48.0;
    s.layer_layers_h_scroll.offset = 60.0;
    s.variables_scroll.offset = 72.0;
    s.design_md_panel.scroll.offset = 84.0;

    assert_eq!(s.property_panel_scroll.offset, 12.0);
    assert_eq!(s.layer_pages_scroll.offset, 24.0);
    assert_eq!(s.layer_layers_scroll.offset, 36.0);
    assert_eq!(s.layer_pages_h_scroll.offset, 48.0);
    assert_eq!(s.layer_layers_h_scroll.offset, 60.0);
    assert_eq!(s.variables_scroll.offset, 72.0);
    assert_eq!(s.design_md_panel.scroll.offset, 84.0);
}

#[test]
fn button_press_target_clears_chrome_button_families() {
    let mut ui = EditorUiState {
        pressed_button: Some(crate::button_press_state::ButtonPressTarget::FigmaImport(
            crate::FigmaImportButton::DropZone,
        )),
        ..Default::default()
    };

    assert!(
        ui.button_pressed(crate::button_press_state::ButtonPressTarget::FigmaImport(
            crate::FigmaImportButton::DropZone,
        ))
    );

    ui.clear_button_press_target();

    assert_eq!(ui.pressed_button, None);
}

#[test]
fn theme_mode_flips() {
    assert_eq!(ThemeMode::Dark.flipped(), ThemeMode::Light);
    assert_eq!(ThemeMode::Light.flipped(), ThemeMode::Dark);
}

#[test]
fn preview_mode_defaults_off() {
    let ui = EditorUiState::new();
    assert!(!ui.preview.mode);
    assert!(ui.preview.warnings.is_empty());
}

#[test]
fn preview_toggle_round_trips_and_clears_warnings() {
    let mut ui = EditorUiState::new();
    // Enter — flips on + clears stale warnings.
    ui.preview.warnings.push("stale".to_string());
    assert!(ui.toggle_preview());
    assert!(ui.preview.mode);
    assert!(ui.preview.warnings.is_empty());

    // A warning recorded during preview is dropped on exit.
    ui.preview
        .warnings
        .push("LegacyRolePromoted: button → text_input".to_string());
    assert!(!ui.toggle_preview());
    assert!(!ui.preview.mode);
    assert!(ui.preview.warnings.is_empty());
}

#[test]
fn preview_enter_exit_are_idempotent() {
    let mut ui = EditorUiState::new();
    ui.enter_preview();
    ui.enter_preview();
    assert!(ui.preview.mode);
    ui.exit_preview();
    ui.exit_preview();
    assert!(!ui.preview.mode);
}

#[test]
fn exit_preview_clears_device_state() {
    let mut c = EditorUiState::new();
    c.enter_preview();
    c.preview.device = Some(PreviewDeviceKind::Phone);
    c.preview.switcher_hover = Some(PreviewDeviceKind::Desktop);
    c.preview.switcher_pressed = Some(PreviewDeviceKind::Canvas);
    c.preview.screen_switcher_hover = Some(1);
    c.preview.screen_switcher_pressed = Some(2);
    c.exit_preview();
    assert_eq!(c.preview.device, None);
    assert_eq!(c.preview.switcher_hover, None);
    assert_eq!(c.preview.switcher_pressed, None);
    assert_eq!(c.preview.screen_switcher_hover, None);
    assert_eq!(c.preview.screen_switcher_pressed, None);
}

#[test]
fn export_format_metadata() {
    assert_eq!(ExportFormat::ALL.len(), 5);
    assert_eq!(ExportFormat::Png.extension(), "png");
    assert_eq!(ExportFormat::Jpeg.extension(), "jpg");
}

#[test]
fn builtin_agent_draft_ready_reads_focused_settings_input() {
    use crate::agent_settings::{BuiltinAgentField, SettingsFocus};

    let mut ui = EditorUiState::new();
    ui.agent_settings.begin_builtin_agent_draft();
    assert!(!ui.builtin_agent_draft_ready());

    ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::ApiKey));
    ui.settings_input.set_text("sk-test");

    assert!(ui.builtin_agent_draft_ready());

    ui.agent_settings.focus = None;
    let draft = ui
        .agent_settings
        .builtin_agent_draft
        .as_mut()
        .expect("draft exists");
    draft.api_key = "sk-test".into();
    draft.models.clear();
    assert!(ui.builtin_agent_draft_ready());
}

#[test]
fn acp_agent_draft_ready_reads_focused_settings_input() {
    use crate::agent_settings::{AcpAgentField, SettingsFocus};

    let mut ui = EditorUiState::new();
    ui.agent_settings.begin_acp_agent_draft();
    assert!(!ui.acp_agent_draft_ready());

    ui.agent_settings.focus = Some(SettingsFocus::AcpAgentDraft(AcpAgentField::Command));
    ui.settings_input.set_text("op-agent");

    assert!(ui.acp_agent_draft_ready());
}

#[test]
fn dirty_ready_repo_keeps_header_popovers_allowed() {
    // TS parity (the ready view now shows for dirty trees too): a
    // bound, non-merging repo shows the branch-picker / overflow
    // popovers whether the working tree is clean OR dirty. A periodic
    // status refresh must NOT force-close them just because files
    // changed — that was the pre-parity behaviour.
    let mut s = GitPanelState {
        in_repo: true,
        merging: false,
        ..Default::default()
    };
    assert!(s.header_popovers_allowed(), "clean bound repo");
    s.changed_files = vec![GitFileEntry {
        path: "a.op".into(),
        staged: false,
        status: 'M',
    }];
    assert!(
        s.header_popovers_allowed(),
        "dirty bound repo still shows the ready view → popovers stay"
    );
}

#[test]
fn non_ready_states_disallow_header_popovers() {
    // No repo, or a merge in progress → not the ready view → the
    // header popovers can't exist, so a refresh clears them.
    let mut s = GitPanelState {
        in_repo: false,
        ..Default::default()
    };
    assert!(!s.header_popovers_allowed(), "unbound repo");
    s.in_repo = true;
    s.merging = true;
    assert!(!s.header_popovers_allowed(), "merge in progress");
}

#[test]
fn tracked_picker_helpers_reset_select_interaction_state() {
    let mut s = GitPanelState {
        tracked_picker_selected: Some(2),
        ..Default::default()
    };
    s.tracked_picker.hover = Some(1);
    s.tracked_picker.pressed = Some(1);
    s.tracked_picker.scroll.offset = 44.0;

    s.open_tracked_picker();
    assert!(s.tracked_picker.open);
    assert_eq!(s.tracked_picker_selected, None);
    assert_eq!(s.tracked_picker.hover, None);
    assert_eq!(s.tracked_picker.pressed, None);
    assert_eq!(s.tracked_picker.scroll.offset, 0.0);

    s.tracked_picker_selected = Some(0);
    s.tracked_picker.hover = Some(0);
    s.tracked_picker.pressed = Some(0);
    s.tracked_picker.scroll.offset = 44.0;
    assert!(s.close_tracked_picker());
    assert!(!s.tracked_picker.open);
    assert_eq!(s.tracked_picker_selected, None);
    assert_eq!(s.tracked_picker.hover, None);
    assert_eq!(s.tracked_picker.pressed, None);
    assert_eq!(s.tracked_picker.scroll.offset, 0.0);
}

#[test]
fn font_picker_helpers_reset_select_interaction_state_and_search() {
    let mut ui = EditorUiState::new();
    ui.font_picker_search = "inter".to_string();
    ui.font_picker.hover = Some(1);
    ui.font_picker.pressed = Some(1);
    ui.font_picker.scroll.offset = 24.0;

    ui.toggle_font_picker();
    assert!(ui.font_picker.open);
    assert!(ui.font_picker_search.is_empty());
    assert_eq!(ui.font_picker.hover, None);
    assert_eq!(ui.font_picker.pressed, None);
    assert_eq!(ui.font_picker.scroll.offset, 0.0);

    ui.font_picker_search = "roboto".to_string();
    ui.font_picker.hover = Some(0);
    ui.font_picker.pressed = Some(0);
    ui.font_picker.scroll.offset = 24.0;
    assert!(ui.close_font_picker());
    assert!(!ui.font_picker.open);
    assert!(ui.font_picker_search.is_empty());
    assert_eq!(ui.font_picker.hover, None);
    assert_eq!(ui.font_picker.pressed, None);
    assert_eq!(ui.font_picker.scroll.offset, 0.0);
}

#[test]
fn chat_model_picker_helpers_reset_select_interaction_state_and_search() {
    let mut ui = EditorUiState::new();
    ui.chat_model_picker_input.set_text("gpt");
    ui.chat_model_picker.hover = Some(1);
    ui.chat_model_picker.pressed = Some(1);
    ui.chat_model_picker.scroll.offset = 28.0;

    assert!(ui.toggle_chat_model_picker());
    assert!(ui.chat_model_picker.open);
    assert!(ui.chat_model_picker_input.text().is_empty());
    assert_eq!(ui.chat_model_picker.hover, None);
    assert_eq!(ui.chat_model_picker.pressed, None);
    assert_eq!(ui.chat_model_picker.scroll.offset, 0.0);

    ui.chat_model_picker_input.set_text("claude");
    ui.chat_model_picker.hover = Some(0);
    ui.chat_model_picker.pressed = Some(0);
    ui.chat_model_picker.scroll.offset = 28.0;
    assert!(!ui.toggle_chat_model_picker());
    assert!(!ui.chat_model_picker.open);
    assert!(ui.chat_model_picker_input.text().is_empty());
    assert_eq!(ui.chat_model_picker.hover, None);
    assert_eq!(ui.chat_model_picker.pressed, None);
    assert_eq!(ui.chat_model_picker.scroll.offset, 0.0);
}

#[test]
fn icon_picker_helpers_reset_select_interaction_state_and_search() {
    let mut ui = EditorUiState::new();
    ui.icon_picker_replace_selection = true;
    ui.icon_picker_search = "home".to_string();
    ui.icon_picker_select_all = true;
    ui.icon_picker.hover = Some(1);
    ui.icon_picker.pressed = Some(1);

    ui.open_icon_picker(false);
    assert!(ui.icon_picker.open);
    assert!(!ui.icon_picker_replace_selection);
    assert!(ui.icon_picker_search.is_empty());
    assert!(!ui.icon_picker_select_all);
    assert_eq!(ui.icon_picker.hover, None);
    assert_eq!(ui.icon_picker.pressed, None);

    ui.icon_picker_replace_selection = true;
    ui.icon_picker_search = "settings".to_string();
    ui.icon_picker_select_all = true;
    ui.icon_picker.hover = Some(0);
    ui.icon_picker.pressed = Some(0);
    assert!(ui.close_icon_picker());
    assert!(!ui.icon_picker.open);
    assert!(!ui.icon_picker_replace_selection);
    assert!(ui.icon_picker_search.is_empty());
    assert!(!ui.icon_picker_select_all);
    assert_eq!(ui.icon_picker.hover, None);
    assert_eq!(ui.icon_picker.pressed, None);
}

#[test]
fn embed_host_parses_vscode_query() {
    assert_eq!(EmbedHost::from_query("?embed=vscode"), EmbedHost::VsCode);
    assert_eq!(EmbedHost::from_query("embed=vscode"), EmbedHost::VsCode);
    assert_eq!(
        EmbedHost::from_query("?foo=1&embed=vscode"),
        EmbedHost::VsCode
    );
}

#[test]
fn embed_host_defaults_to_none_for_unknown_or_absent() {
    assert_eq!(EmbedHost::from_query(""), EmbedHost::None);
    assert_eq!(EmbedHost::from_query("?embed=web"), EmbedHost::None);
    assert_eq!(EmbedHost::from_query("?embedded=vscode"), EmbedHost::None);
    assert_eq!(EditorUiState::default().embed, EmbedHost::None);
}

#[test]
fn host_theme_override_changes_only_the_effective_theme() {
    let mut ui = EditorUiState {
        theme_mode: ThemeMode::Dark,
        ..EditorUiState::default()
    };
    assert_eq!(ui.effective_theme_mode(), ThemeMode::Dark);

    ui.set_host_theme_override(Some(ThemeMode::Light));
    assert_eq!(ui.effective_theme_mode(), ThemeMode::Light);
    assert_eq!(ui.theme_mode, ThemeMode::Dark);

    ui.set_host_theme_override(None);
    assert_eq!(ui.effective_theme_mode(), ThemeMode::Dark);
}

#[test]
fn host_locale_override_changes_only_the_effective_locale() {
    let mut ui = EditorUiState {
        locale: Locale::ZhCn,
        ..EditorUiState::default()
    };
    assert_eq!(ui.effective_locale(), Locale::ZhCn);

    ui.set_host_locale_override(Some(Locale::EnUs));
    assert_eq!(ui.effective_locale(), Locale::EnUs);
    assert_eq!(ui.locale, Locale::ZhCn);

    ui.set_host_locale_override(None);
    assert_eq!(ui.effective_locale(), Locale::ZhCn);
}
