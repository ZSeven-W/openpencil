//! Repeatable visual matrix for normal-mode control spacing and translations.
//! Uses the same host painting as desktop and mobile, with no model requests.

use super::{home_host, shoot, workspace_host, HomeDevice, HomeFamily, Locale};
use op_editor_core::{size_class::EditorSizeClass, ThemeMode, WorkspaceView};

pub(super) fn run(out_dir: &str) {
    for (theme, theme_name) in [(ThemeMode::Dark, "dark"), (ThemeMode::Light, "light")] {
        for (locale, lang) in [(Locale::ZhCn, "zh"), (Locale::EnUs, "en")] {
            for width in [320.0, 390.0, 430.0] {
                let mut host = home_host(locale, HomeFamily::AppUi, HomeDevice::Mobile);
                let ui = &mut host.editor_state_mut().editor_ui;
                ui.touch = true;
                ui.size_class = EditorSizeClass::Compact;
                ui.theme_mode = theme;
                shoot(
                    &mut host,
                    width,
                    844.0,
                    out_dir,
                    &format!("home-{width}-{lang}-{theme_name}"),
                );
            }
            for (view, view_name) in [
                (WorkspaceView::AllBoards, "all"),
                (WorkspaceView::Single { index: 0 }, "single"),
            ] {
                let mut host =
                    workspace_host(locale, "sample.op", HomeFamily::AppUi, 1180.0, 820.0);
                host.editor_state_mut().editor_ui.theme_mode = theme;
                host.editor_state_mut().editor_ui.workspace.view = view;
                shoot(
                    &mut host,
                    1180.0,
                    820.0,
                    out_dir,
                    &format!("workspace-{view_name}-{lang}-{theme_name}"),
                );
            }
        }
    }
}
