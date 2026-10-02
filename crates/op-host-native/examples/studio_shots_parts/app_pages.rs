//! The independent device/page choices on desktop and compact Home.
use super::{home_host, shoot, HomeDevice, HomeFamily, Locale};
use op_editor_core::{size_class::EditorSizeClass, AppPages, ThemeMode};

pub(super) fn run(out_dir: &str) {
    for (theme, name) in [(ThemeMode::Light, "light"), (ThemeMode::Dark, "dark")] {
        for (device, device_name) in [
            (HomeDevice::Mobile, "phone"),
            (HomeDevice::Desktop, "desktop-app"),
        ] {
            for pages in [AppPages::Single, AppPages::Multiple] {
                for (w, h, compact, viewport) in [
                    (1440.0, 1000.0, false, "desktop"),
                    (390.0, 844.0, true, "mobile"),
                ] {
                    let mut host = home_host(Locale::ZhCn, HomeFamily::AppUi, device);
                    let ui = &mut host.editor_state_mut().editor_ui;
                    ui.theme_mode = theme;
                    ui.home.set_app_pages(pages);
                    if compact {
                        ui.touch = true;
                        ui.size_class = EditorSizeClass::Compact;
                    }
                    shoot(
                        &mut host,
                        w,
                        h,
                        out_dir,
                        &format!("{viewport}-{device_name}-{}-{name}", pages.id()),
                    );
                }
            }
        }
    }
}
