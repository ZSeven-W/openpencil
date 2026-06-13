//! Blank-press blur coverage — `#[cfg(test)]` companion to
//! `blur_inputs.rs`. Every chrome text input must defocus (and commit
//! its draft) when a press lands on blank chrome: a top-bar gap, a
//! panel-rail gap, modal chrome, or the bare canvas.

use std::collections::BTreeMap;

use super::WidgetHostNative;
use op_editor_core::agent_settings::SettingsFocus;
use op_editor_core::editor_ui_state::{CloneField, CloneFormState};
use op_editor_core::ui_draft::PropertyFocus;
use op_editor_core::{own_bounds, NodeId};

const VW: f32 = 1200.0;
const VH: f32 = 800.0;

const ONE_RECT: &str = r#"{"version":"0.8.0","children":[{"type":"rectangle","id":"n1","name":"n1","x":0,"y":0,"width":100,"height":50}]}"#;

/// Seed a host's `editor_state` from a canonical `.op` JSON snippet.
fn seed(host: &mut WidgetHostNative, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.mark_paint_dirty_for_test();
}

#[test]
fn top_bar_gap_press_blurs_chat_and_model_picker() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, ONE_RECT);
    host.editor_state_mut().chat.focused = true;
    host.editor_state_mut().editor_ui.chat_model_picker_open = true;
    host.editor_state_mut()
        .editor_ui
        .chat_model_picker_input
        .set_text("gpt");

    // Dead centre of the top bar — between the left file controls and
    // the right chrome chips.
    assert!(host.apply_press(VW / 2.0, 20.0, VW, VH));

    let state = host.editor_state();
    assert!(
        !state.chat.focused,
        "top-bar gap press must blur the chat input"
    );
    assert!(!state.editor_ui.chat_model_picker_open);
    assert!(state.editor_ui.chat_model_picker_input.text().is_empty());
}

#[test]
fn canvas_press_defocuses_every_git_input() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, ONE_RECT);
    {
        let git = &mut host.editor_state_mut().editor_ui.git_panel;
        git.open = true;
        git.author_prompt = true;
        git.commit_focused = true;
        git.author_name_focused = true;
        git.author_email_focused = true;
        git.branch_create_focused = true;
        git.clone_form = Some(CloneFormState {
            url: "https://example.com/repo.git".into(),
            dest: "/tmp/repo".into(),
            focus: Some(CloneField::Url),
            ..Default::default()
        });
    }

    // Empty canvas — right of the chat panel (which floats bottom-left,
    // x ≤ 612) and below the git panel popover (y ≤ ~350).
    assert!(host.apply_press(700.0, 600.0, VW, VH));

    let git = &host.editor_state().editor_ui.git_panel;
    assert!(!git.open, "canvas press dismisses the floating git panel");
    assert!(!git.commit_focused);
    assert!(!git.author_name_focused);
    assert!(!git.author_email_focused);
    assert!(!git.branch_create_focused);
    assert!(git.clone_form.is_none());
}

#[test]
fn settings_modal_blank_press_commits_mcp_port_draft() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, ONE_RECT);
    {
        let eui = &mut host.editor_state_mut().editor_ui;
        eui.agent_settings_open = true;
        eui.agent_settings.focus = Some(SettingsFocus::McpPort);
        eui.settings_input.set_text("4321");
    }

    // Bottom of the modal's sidebar column — below the nav tabs, no
    // control under it. The 720×720 modal spans x ∈ [240, 960] at
    // this viewport, so x=260 sits just inside its left edge.
    assert!(host.apply_press(260.0, 700.0, VW, VH));

    let eui = &host.editor_state().editor_ui;
    assert!(
        eui.agent_settings.focus.is_none(),
        "blank press inside the modal must commit + blur the port input"
    );
    assert_eq!(eui.agent_settings.mcp_server.port, 4321);
    assert!(
        eui.agent_settings_open,
        "blank press inside the modal must not close it"
    );
}

#[test]
fn property_panel_gap_press_commits_size_draft() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, ONE_RECT);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n1"));
    // Collapse the chat so its floating panel can't shadow the
    // property-rail gap this test presses.
    host.editor_state_mut().chat.collapsed = true;
    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::SizeW);
    host.editor_state_mut().ui.property_input.set_text("321");

    // Inside the right rail's edge padding — the input rows are inset
    // from the panel edges, so 2 px from the viewport edge hits no
    // control at any section y.
    assert!(host.apply_press(VW - 2.0, 400.0, VW, VH));

    let state = host.editor_state();
    assert!(
        state.ui.property_focus.is_none(),
        "panel-gap press must commit + blur the property input"
    );
    let node = state.selected_node().expect("selection survives");
    assert_eq!(own_bounds(node).w, 321.0);
}

#[test]
fn canvas_press_commits_variables_header_rename() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, ONE_RECT);
    host.editor_state_mut().doc.themes = Some(BTreeMap::from([(
        "Theme-1".to_string(),
        vec!["Default".to_string()],
    )]));
    host.editor_state_mut()
        .editor_ui
        .variables_theme_rename_axis = Some("Theme-1".into());
    host.editor_state_mut()
        .editor_ui
        .variables_header_input
        .set_text("Brand");

    // Empty canvas right of the chat panel (floats bottom-left, x ≤ 612).
    assert!(host.apply_press(700.0, 400.0, VW, VH));

    let state = host.editor_state();
    assert!(state.editor_ui.variables_theme_rename_axis.is_none());
    let themes = state.doc.themes.as_ref().expect("themes survive");
    assert!(
        themes.contains_key("Brand"),
        "canvas press must commit the pending theme-axis rename"
    );
    assert!(!themes.contains_key("Theme-1"));
}

#[test]
fn canvas_press_commits_variables_variant_header_rename() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, ONE_RECT);
    host.editor_state_mut().doc.themes = Some(BTreeMap::from([(
        "Theme-1".to_string(),
        vec!["Default".to_string()],
    )]));
    host.editor_state_mut().editor_ui.variables_current_axis = Some("Theme-1".into());
    host.editor_state_mut()
        .editor_ui
        .variables_variant_rename_value = Some("Default".into());
    host.editor_state_mut()
        .editor_ui
        .variables_header_input
        .set_text("Default123213");

    // Empty canvas right of the chat panel (floats bottom-left, x <= 612).
    assert!(host.apply_press(700.0, 400.0, VW, VH));

    let state = host.editor_state();
    assert!(state.editor_ui.variables_variant_rename_value.is_none());
    assert_eq!(
        state
            .doc
            .themes
            .as_ref()
            .expect("themes survive")
            .get("Theme-1")
            .expect("axis survives"),
        &vec!["Default123213".to_string()],
        "canvas press must commit the pending variant rename"
    );
}

#[test]
fn canvas_press_commits_implicit_default_variant_rename() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, ONE_RECT);
    host.editor_state_mut().doc.themes = None;
    host.editor_state_mut()
        .editor_ui
        .variables_variant_rename_value = Some("Default".into());
    host.editor_state_mut()
        .editor_ui
        .variables_header_input
        .set_text("Default123213");

    // Empty canvas right of the chat panel (floats bottom-left, x <= 612).
    assert!(host.apply_press(700.0, 400.0, VW, VH));

    let state = host.editor_state();
    assert!(state.editor_ui.variables_variant_rename_value.is_none());
    assert_eq!(
        state
            .doc
            .themes
            .as_ref()
            .expect("implicit theme should be materialized")
            .get("Theme-1")
            .expect("implicit axis should be materialized"),
        &vec!["Default123213".to_string()],
        "blur commit must persist edits to the implicit Default variant"
    );
}
