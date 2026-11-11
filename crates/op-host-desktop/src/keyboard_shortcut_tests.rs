//! Track C-6 (`Cmd+P` toggles Preview) + its non-conflict with the
//! existing `Cmd+Shift+P` (Export Image) chord. Split out of
//! `main_tests.rs` to keep that file under the repo's 800-line cap.

use super::*;
use op_editor_core::{agent_settings::SettingsFocus, figma_import_state::ImportSource};
use winit::keyboard::Key;

#[test]
fn cmd_p_enters_and_exits_preview() {
    let mut app = DesktopApp::new(None);
    assert!(!app.host.preview_active());

    app.zoom_modifier = true;
    app.handle_key_pressed(&Key::Character("p".into()), Some("p"));
    assert!(app.host.preview_active(), "Cmd+P must enter preview");

    app.handle_key_pressed(&Key::Character("p".into()), Some("p"));
    assert!(!app.host.preview_active(), "Cmd+P again must exit preview");
}

#[test]
fn cmd_p_works_even_while_preview_owns_the_keyboard() {
    // Preview's own takeover (Tab / arrow keys, gated on
    // `preview_active()`) only claims UNMODIFIED named keys — it must
    // never shadow the Cmd+P character chord that exits it.
    let mut app = DesktopApp::new(None);
    app.zoom_modifier = true;
    app.handle_key_pressed(&Key::Character("p".into()), Some("p"));
    assert!(app.host.preview_active());

    app.handle_key_pressed(&Key::Character("p".into()), Some("p"));
    assert!(
        !app.host.preview_active(),
        "Cmd+P must exit preview from within preview mode, not be swallowed by its keyboard takeover"
    );
}

#[test]
fn cmd_shift_p_stays_export_image_not_preview() {
    // Cmd+Shift+P is the pre-existing Export Image chord
    // (`keyboard_input.rs`'s `zoom_modifier && shift_modifier` arm) —
    // Track C-6 must not collide with it. Plain Cmd+P (no Shift) is the
    // only chord that toggles preview.
    let mut app = DesktopApp::new(None);
    app.zoom_modifier = true;
    app.shift_modifier = true;
    app.handle_key_pressed(&Key::Character("p".into()), Some("p"));
    assert!(
        !app.host.preview_active(),
        "Cmd+Shift+P must not toggle preview — it's Export Image"
    );
}

#[test]
fn cmd_shift_import_shortcuts_select_their_source() {
    let mut app = DesktopApp::new(None);
    app.zoom_modifier = true;
    app.shift_modifier = true;

    app.host.editor_state_mut().editor_ui.import_source = ImportSource::Html;
    app.handle_key_pressed(&Key::Character("f".into()), Some("f"));
    assert!(app.host.editor_state().editor_ui.figma_import_open);
    assert_eq!(
        app.host.editor_state().editor_ui.import_source,
        ImportSource::Figma
    );

    app.host.editor_state_mut().editor_ui.figma_import_open = false;
    app.handle_key_pressed(&Key::Character("H".into()), Some("H"));
    assert!(app.host.editor_state().editor_ui.figma_import_open);
    assert_eq!(
        app.host.editor_state().editor_ui.import_source,
        ImportSource::Html
    );
}

#[test]
fn cmd_shift_alt_does_not_open_an_import_modal() {
    let mut app = DesktopApp::new(None);
    app.zoom_modifier = true;
    app.shift_modifier = true;
    app.alt_modifier = true;

    app.handle_key_pressed(&Key::Character("f".into()), Some("f"));
    app.handle_key_pressed(&Key::Character("h".into()), Some("h"));
    assert!(!app.host.editor_state().editor_ui.figma_import_open);
}

#[test]
fn closing_settings_with_cmd_comma_does_not_leave_import_shortcuts_blocked() {
    let mut app = DesktopApp::new(None);
    app.zoom_modifier = true;
    {
        let ui = &mut app.host.editor_state_mut().editor_ui;
        ui.agent_settings_open = true;
        ui.agent_settings.focus = Some(SettingsFocus::McpPort);
    }

    app.handle_key_pressed(&Key::Character(",".into()), Some(","));
    assert!(!app.host.editor_state().editor_ui.agent_settings_open);
    assert!(app
        .host
        .editor_state()
        .editor_ui
        .agent_settings
        .focus
        .is_none());

    app.shift_modifier = true;
    app.handle_key_pressed(&Key::Character("f".into()), Some("f"));
    assert!(app.host.editor_state().editor_ui.figma_import_open);
}

#[test]
fn home_vertical_arrow_keys_reach_the_prompt_caret_and_shift_selection() {
    use winit::keyboard::NamedKey;
    let mut app = DesktopApp::new(None);
    app.host.apply_press(0.0, 0.0, 1440.0, 900.0);
    let home = &mut app.host.editor_state_mut().editor_ui.home;
    home.visible = true;
    home.composer_focused = true;
    home.set_draft("abcd\nefgh\nijkl");
    home.set_caret(12, 0);
    app.handle_key_pressed(&Key::Named(NamedKey::ArrowUp), None);
    assert_eq!(app.host.editor_state().editor_ui.home.input.caret(), 7);
    app.shift_modifier = true;
    app.handle_key_pressed(&Key::Named(NamedKey::ArrowUp), None);
    assert_eq!(
        app.host
            .editor_state()
            .editor_ui
            .home
            .input
            .highlight_range(),
        Some((2, 7))
    );
    app.shift_modifier = false;
    app.handle_key_pressed(&Key::Named(NamedKey::ArrowDown), None);
    assert_eq!(app.host.editor_state().editor_ui.home.input.caret(), 7);
    assert!(app
        .host
        .editor_state()
        .editor_ui
        .home
        .input
        .highlight_range()
        .is_none());
}

#[test]
fn bare_m_toggles_the_motion_timeline_and_h_stays_the_hand() {
    let mut app = DesktopApp::new(None);
    app.handle_key_pressed(&Key::Character("m".into()), Some("m"));
    assert!(app.host.editor_state().editor_ui.motion_timeline.open);
    app.handle_key_pressed(&Key::Character("h".into()), Some("h"));
    assert_eq!(app.host.editor_state().tool, op_editor_core::Tool::Hand);
    assert!(app.host.editor_state().editor_ui.motion_timeline.open);
    app.handle_key_pressed(&Key::Character("M".into()), Some("M"));
    assert!(!app.host.editor_state().editor_ui.motion_timeline.open);
}
