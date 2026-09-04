use super::WidgetHost;
use op_editor_core::ThemeMode;

#[test]
fn theme_cache_follows_editor_theme_mode() {
    let mut host = WidgetHost::new();

    host.editor_state.editor_ui.theme_mode = ThemeMode::Light;
    host.sync_theme_from_editor();
    assert!(
        host.theme.background.r > 0.9,
        "light mode should refresh the host theme cache"
    );

    host.editor_state.editor_ui.theme_mode = ThemeMode::Dark;
    host.sync_theme_from_editor();
    assert!(
        host.theme.background.r < 0.1,
        "dark mode should refresh the host theme cache"
    );
}

#[test]
fn theme_cache_follows_transient_host_override_without_mutating_preference() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.theme_mode = ThemeMode::Light;
    host.editor_state
        .editor_ui
        .set_host_theme_override(Some(ThemeMode::Dark));

    host.sync_theme_from_editor();

    assert!(host.theme.background.r < 0.1);
    assert_eq!(host.editor_state.editor_ui.theme_mode, ThemeMode::Light);
}
