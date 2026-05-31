use super::WidgetHostNative;
use op_editor_core::ThemeMode;

#[test]
fn theme_cache_follows_editor_theme_mode() {
    let mut host = WidgetHostNative::new();

    host.editor_state_mut().editor_ui.theme_mode = ThemeMode::Light;
    host.sync_theme_from_editor();
    assert!(
        host.theme.background.r > 0.9,
        "light mode should refresh the host theme cache"
    );

    host.editor_state_mut().editor_ui.theme_mode = ThemeMode::Dark;
    host.sync_theme_from_editor();
    assert!(
        host.theme.background.r < 0.1,
        "dark mode should refresh the host theme cache"
    );
}
