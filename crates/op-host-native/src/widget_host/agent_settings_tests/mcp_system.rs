//! MCP tab (server start/stop, client-config copy) and System tab
//! (auto-update, experimental gate) presses.
//!
//! Split out of `agent_settings_tests.rs` to keep every file under the
//! repo's 800-line cap.

use super::*;

/// Scroll the settings body so `target` is visible and return the screen
/// point at its centre. The custom-configuration section sits below the
/// twelve CLI rows, so a press at its unscrolled rect lands outside the
/// modal.
fn scroll_to_centre(
    host: &mut WidgetHostNative,
    rect: op_editor_ui::Rect,
    target: op_editor_ui::Rect,
) -> (f32, f32) {
    let content = op_editor_ui::widgets::agent_settings_panel::content_viewport(rect);
    let offset = target.origin.y - content.origin.y;
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .scroll_y
        .offset = offset;
    let effective_offset =
        AgentSettingsPanel::for_editor(host.editor_state()).effective_scroll(rect);
    (
        target.origin.x + target.size.x / 2.0,
        target.origin.y + target.size.y / 2.0 - effective_offset,
    )
}

#[test]
fn starting_mcp_server_commits_port_draft_and_clears_focus() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    host.editor_state_mut().editor_ui.agent_settings.focus = Some(SettingsFocus::McpPort);
    host.editor_state_mut()
        .editor_ui
        .settings_input
        .set_text("3101");

    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
    let button = op_editor_ui::widgets::agent_settings_panel::mcp_server_button(rect);
    assert!(host.dispatch_agent_settings_press(
        button.origin.x + button.size.x / 2.0,
        button.origin.y + button.size.y / 2.0,
        VIEWPORT_W,
        VIEWPORT_H
    ));

    let state = host.editor_state();
    assert!(state.editor_ui.agent_settings.mcp_server.running);
    assert_eq!(state.editor_ui.agent_settings.mcp_server.port, 3101);
    assert!(state.editor_ui.agent_settings.focus.is_none());
    assert!(state.editor_ui.settings_input.text().is_empty());
    assert_eq!(
        state.editor_ui.pressed_button,
        Some(ButtonPressTarget::AgentSettings(
            AgentSettingsButton::McpServer
        ))
    );

    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state().editor_ui.pressed_button, None);
}

#[test]
fn copy_mcp_client_config_queues_clipboard_text() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .mcp_server
        .running = true;
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .mcp_server
        .port = 4123;

    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
    let copy = op_editor_ui::widgets::agent_settings_panel::mcp_copy_config_button(
        rect,
        host.editor_state().editor_ui.external_cli_available,
    );
    let (x, y) = scroll_to_centre(&mut host, rect, copy);
    assert!(host.dispatch_agent_settings_press(x, y, VIEWPORT_W, VIEWPORT_H));

    assert_eq!(
        host.editor_state().chat.pending_copy_text.as_deref(),
        Some("{\n  \"type\": \"http\",\n  \"url\": \"http://127.0.0.1:4123/mcp\"\n}")
    );
}

#[test]
fn system_auto_update_switch_toggles_preference() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::System;
    assert!(
        host.editor_state()
            .editor_ui
            .agent_settings
            .auto_update_enabled
    );

    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
    let switch = op_editor_ui::widgets::agent_settings_panel::system_auto_update_switch(rect);
    assert!(host.dispatch_agent_settings_press(
        switch.origin.x + switch.size.x / 2.0,
        switch.origin.y + switch.size.y / 2.0,
        VIEWPORT_W,
        VIEWPORT_H
    ));

    assert!(
        !host
            .editor_state()
            .editor_ui
            .agent_settings
            .auto_update_enabled
    );
}

#[test]
fn system_experimental_switch_toggles_preference() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::System;
    assert!(
        !host
            .editor_state()
            .editor_ui
            .agent_settings
            .experimental_features_enabled
    );

    let (cx, _, cw) = agent_settings_content_metrics(&host);
    let switch_x = cx + cw - 28.0;
    assert!(host.dispatch_agent_settings_press(
        switch_x,
        experimental_switch_y(&host, switch_x),
        VIEWPORT_W,
        VIEWPORT_H
    ));

    assert!(
        host.editor_state()
            .editor_ui
            .agent_settings
            .experimental_features_enabled
    );
}

/// Preview graduated out of the experimental-features gate (2026-07): the
/// Play button + preview interaction are now a regular always-on feature,
/// so toggling the experimental gate off no longer force-exits a live
/// preview session (contrast `disabling_experimental_clears_widget_
/// property_focus` below — Widget-config stays gated and still clears).
#[test]
fn disabling_experimental_leaves_active_preview_running() {
    let mut host = WidgetHostNative::new();
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0.0","children":[
            {"type":"frame","id":"root","name":"Root","x":0,"y":0,"width":200,"height":200,
             "children":[
               {"type":"rectangle","id":"r","name":"R","x":10,"y":10,"width":50,"height":50}
             ]}
        ]}"#,
    )
    .expect("fixture parses")
    .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .experimental_features_enabled = true;
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::System;

    assert!(host.enter_preview((800.0, 600.0)), "preview should enter");
    assert!(host.preview_active());

    let (cx, _, cw) = agent_settings_content_metrics(&host);
    let switch_x = cx + cw - 28.0;
    assert!(host.dispatch_agent_settings_press(
        switch_x,
        experimental_switch_y(&host, switch_x),
        VIEWPORT_W,
        VIEWPORT_H
    ));

    assert!(
        !host
            .editor_state()
            .editor_ui
            .agent_settings
            .experimental_features_enabled
    );
    assert!(
        host.preview_active(),
        "disabling experimental must NOT exit the live preview session"
    );
    assert!(host.editor_state().editor_ui.preview.mode);
}

#[test]
fn disabling_experimental_clears_widget_property_focus() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .experimental_features_enabled = true;
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::System;
    // A widget field still holds focus when the gate flips off.
    host.editor_state_mut().ui.property_focus =
        Some(op_editor_core::PropertyFocus::WidgetPlaceholder);

    let (cx, _, cw) = agent_settings_content_metrics(&host);
    let switch_x = cx + cw - 28.0;
    assert!(host.dispatch_agent_settings_press(
        switch_x,
        experimental_switch_y(&host, switch_x),
        VIEWPORT_W,
        VIEWPORT_H
    ));

    assert!(
        host.editor_state().ui.property_focus.is_none(),
        "stale Widget property focus must be cleared so it can't commit"
    );
}

#[test]
fn copying_mcp_client_config_records_feedback_time() {
    let mut host = WidgetHostNative::new();
    host.set_now_ms(4_321);
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .mcp_server
        .running = true;

    let rect = AgentSettingsPanel::for_editor(host.editor_state()).rect(VIEWPORT_W, VIEWPORT_H);
    let copy = op_editor_ui::widgets::agent_settings_panel::mcp_copy_config_button(
        rect,
        host.editor_state().editor_ui.external_cli_available,
    );
    let (x, y) = scroll_to_centre(&mut host, rect, copy);

    assert!(host.dispatch_agent_settings_press(x, y, VIEWPORT_W, VIEWPORT_H));

    assert_eq!(
        host.editor_state()
            .editor_ui
            .agent_settings
            .mcp_client_config_copied_at_ms,
        Some(4_321)
    );
    assert_eq!(
        host.editor_state().chat.pending_copy_text.as_deref(),
        Some("{\n  \"type\": \"http\",\n  \"url\": \"http://127.0.0.1:3100/mcp\"\n}")
    );
    assert_eq!(
        host.editor_state().editor_ui.pressed_button,
        Some(ButtonPressTarget::AgentSettings(
            AgentSettingsButton::McpClientConfigCopy
        ))
    );

    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state().editor_ui.pressed_button, None);
}

#[test]
fn external_cli_unavailable_kills_the_mcp_cli_toggle_press_path() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    let rect = AgentSettingsPanel::for_editor(host.editor_state()).rect(VIEWPORT_W, VIEWPORT_H);

    // Row centres of the desktop toggle grid, captured before the flag
    // flips — the exact points that used to write an MCP endpoint into a
    // CLI config file.
    let rows: Vec<(f32, f32)> = {
        let panel = AgentSettingsPanel::for_editor(host.editor_state());
        let content = panel.resolved_content_viewport(rect);
        let mut rows = Vec::new();
        let mut y = content.origin.y;
        while y <= content.origin.y + content.size.y {
            let mut x = content.origin.x;
            while x <= content.origin.x + content.size.x {
                if matches!(
                    panel.hit_test(rect, Point2D::new(x, y)),
                    AgentSettingsHit::ToggleMcpCli(_)
                ) {
                    rows.push((x, y));
                }
                x += 8.0;
            }
            y += 8.0;
        }
        rows
    };
    assert!(
        !rows.is_empty(),
        "the desktop MCP tab must paint hittable CLI toggles"
    );

    host.editor_state_mut().editor_ui.external_cli_available = false;
    for (x, y) in rows {
        assert!(host.dispatch_agent_settings_press(x, y, VIEWPORT_W, VIEWPORT_H));
    }

    assert!(
        !host
            .editor_state()
            .editor_ui
            .agent_settings
            .mcp_cli_enabled
            .iter()
            .any(|enabled| *enabled),
        "pressing where the toggles used to be must not flip any CLI integration"
    );
}
