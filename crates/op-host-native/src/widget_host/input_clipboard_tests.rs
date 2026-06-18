//! Cmd+C / Cmd+X / Cmd+V parity for focused text inputs.
//!
//! Regression guard for the input-clipboard wiring: copy reads the
//! highlighted slice, paste routes through the shared `apply_text`
//! per-input filter, and cut returns + clears the selection. The
//! agent-settings ACP command field stands in as a representative
//! free-text input (the desktop host then writes/reads the OS
//! clipboard around these host-level primitives).

use super::WidgetHostNative;
use op_editor_core::agent_settings::{AcpAgentField, SettingsFocus};

fn host_with_focused_settings_input(seed: &str) -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.agent_settings_open = true;
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .add_acp_agent();
    host.editor_state_mut().editor_ui.agent_settings.focus = Some(SettingsFocus::AcpAgent {
        index: 0,
        field: AcpAgentField::Command,
    });
    host.editor_state_mut()
        .editor_ui
        .settings_input
        .set_text(seed);
    host
}

#[test]
fn copy_reads_the_focused_input_selection() {
    let mut host = host_with_focused_settings_input("claude-code");
    // Caret only, no highlight → nothing to copy.
    assert_eq!(host.input_copy_text(), None);
    host.editor_state_mut()
        .editor_ui
        .settings_input
        .select_all();
    assert_eq!(host.input_copy_text().as_deref(), Some("claude-code"));
}

#[test]
fn paste_inserts_into_the_focused_input() {
    let mut host = host_with_focused_settings_input("claude");
    host.editor_state_mut()
        .editor_ui
        .settings_input
        .select_all();
    // Paste replaces the select-all range; the trailing newline is a
    // control char and is dropped by `apply_input_paste`.
    assert!(host.apply_input_paste("codex\n"));
    assert_eq!(host.editor_state().editor_ui.settings_input.text(), "codex");
}

#[test]
fn cut_returns_and_clears_the_focused_input_selection() {
    let mut host = host_with_focused_settings_input("opencode");
    host.editor_state_mut()
        .editor_ui
        .settings_input
        .select_all();
    assert_eq!(host.input_cut_text().as_deref(), Some("opencode"));
    assert!(host
        .editor_state()
        .editor_ui
        .settings_input
        .text()
        .is_empty());
}

#[test]
fn copy_and_cut_are_noops_without_a_focused_input() {
    // No input focused → both yield None so the desktop dispatch falls
    // through to the document node clipboard instead of swallowing the
    // chord.
    let mut host = WidgetHostNative::new();
    assert_eq!(host.input_copy_text(), None);
    assert_eq!(host.input_cut_text(), None);
}

#[test]
fn copy_reads_a_focused_effect_param_input() {
    use op_editor_core::editor_ui_state::EffectParamFocus;
    use op_editor_core::EffectField;
    // Effect-parameter fields share `property_input` with property
    // fields, so `active_text_input` must resolve them too (else Cmd+C/X
    // in a focused blur/offset field copies nothing).
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.effect_param_focus = Some(EffectParamFocus {
        effect: 0,
        field: EffectField::Blur,
    });
    host.editor_state_mut().ui.property_input.set_text("12");
    host.editor_state_mut().ui.property_input.select_all();
    assert_eq!(host.input_copy_text().as_deref(), Some("12"));
}
