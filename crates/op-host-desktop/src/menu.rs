//! Native application menu bar.
//!
//! winit owns the window but has no menu primitive, so the native
//! menu bar is built with `muda` (the tao/tauri menu crate) and
//! attached to the running NSApp (macOS) / window (Windows). Menu
//! selections arrive on `muda`'s global event channel; [`poll`]
//! drains them into a [`MenuAction`] the runner maps onto the same
//! `WidgetHostNative` calls the keyboard shortcuts use.
//!
//! Linux: `muda` needs a GTK window, but this build's winit is
//! configured for the x11 / wayland backends (no GTK), so the menu
//! is a no-op there — the in-canvas File menu covers Linux. The
//! `muda` dependency is itself gated to macOS / Windows in
//! `Cargo.toml`, so this module compiles to stubs elsewhere.

/// A menu selection, decoupled from `muda` so the runner matches on
/// a plain enum. Each variant maps onto an existing host action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    New,
    Open,
    Save,
    SaveAs,
    Export,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    Duplicate,
    Group,
    Ungroup,
    ToggleFullscreen,
    Quit,
    CheckUpdates,
    OpenGithub,
}

// --------------------------------------------------------------------
// macOS / Windows — the real `muda` backend.
// --------------------------------------------------------------------
#[cfg(any(target_os = "macos", target_os = "windows"))]
mod backend {
    use super::MenuAction;
    use muda::accelerator::{Accelerator, Code, Modifiers};
    use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};

    // Stable menu-item ids — the wire between the `muda` menu and
    // `action_for_id`. Kept as `&str` consts so the build + the
    // dispatch can't drift.
    const ID_NEW: &str = "new";
    const ID_OPEN: &str = "open";
    const ID_SAVE: &str = "save";
    const ID_SAVE_AS: &str = "save-as";
    const ID_EXPORT: &str = "export";
    const ID_UNDO: &str = "undo";
    const ID_REDO: &str = "redo";
    const ID_CUT: &str = "cut";
    const ID_COPY: &str = "copy";
    const ID_PASTE: &str = "paste";
    const ID_SELECT_ALL: &str = "select-all";
    const ID_DUPLICATE: &str = "duplicate";
    const ID_GROUP: &str = "group";
    const ID_UNGROUP: &str = "ungroup";
    const ID_FULLSCREEN: &str = "fullscreen";
    const ID_QUIT: &str = "quit";
    const ID_CHECK_UPDATES: &str = "check-updates";
    const ID_GITHUB: &str = "github";

    /// Map a `muda` menu-item id string onto a [`MenuAction`].
    fn action_for_id(id: &str) -> Option<MenuAction> {
        Some(match id {
            ID_NEW => MenuAction::New,
            ID_OPEN => MenuAction::Open,
            ID_SAVE => MenuAction::Save,
            ID_SAVE_AS => MenuAction::SaveAs,
            ID_EXPORT => MenuAction::Export,
            ID_UNDO => MenuAction::Undo,
            ID_REDO => MenuAction::Redo,
            ID_CUT => MenuAction::Cut,
            ID_COPY => MenuAction::Copy,
            ID_PASTE => MenuAction::Paste,
            ID_SELECT_ALL => MenuAction::SelectAll,
            ID_DUPLICATE => MenuAction::Duplicate,
            ID_GROUP => MenuAction::Group,
            ID_UNGROUP => MenuAction::Ungroup,
            ID_FULLSCREEN => MenuAction::ToggleFullscreen,
            ID_QUIT => MenuAction::Quit,
            ID_CHECK_UPDATES => MenuAction::CheckUpdates,
            ID_GITHUB => MenuAction::OpenGithub,
            _ => return None,
        })
    }

    /// Owns the `muda` `Menu`. Kept alive for the process lifetime —
    /// dropping it would tear the native menu down.
    pub struct AppMenu {
        _menu: Menu,
    }

    /// Primary command modifier — Cmd on macOS, Ctrl on Windows.
    fn primary() -> Modifiers {
        #[cfg(target_os = "macos")]
        {
            Modifiers::META
        }
        #[cfg(not(target_os = "macos"))]
        {
            Modifiers::CONTROL
        }
    }

    fn accel(code: Code) -> Accelerator {
        Accelerator::new(Some(primary()), code)
    }

    fn accel_shift(code: Code) -> Accelerator {
        Accelerator::new(Some(primary() | Modifiers::SHIFT), code)
    }

    /// A custom, id-tagged menu item with an accelerator.
    fn item(id: &str, text: &str, accel: Option<Accelerator>) -> MenuItem {
        MenuItem::with_id(id, text, true, accel)
    }

    impl AppMenu {
        /// Build the menu and attach it to the running app / window.
        /// macOS needs the NSApp to exist, Windows needs the window —
        /// so this is called from `resumed`, after window creation.
        pub fn install(window: &winit::window::Window) -> Self {
            let menu = Menu::new();

            // macOS app menu — About / Services / Hide / Quit, the
            // conventional first submenu macOS labels with the app
            // name. Quit is custom-id'd so the runner drives the same
            // clean-shutdown path as the window-close button.
            #[cfg(target_os = "macos")]
            {
                let app_menu = Submenu::new("OpenPencil", true);
                let _ = app_menu.append_items(&[
                    &PredefinedMenuItem::about(None, Some(about_metadata())),
                    &PredefinedMenuItem::separator(),
                    &PredefinedMenuItem::services(None),
                    &PredefinedMenuItem::separator(),
                    &PredefinedMenuItem::hide(None),
                    &PredefinedMenuItem::hide_others(None),
                    &PredefinedMenuItem::show_all(None),
                    &PredefinedMenuItem::separator(),
                    &item(ID_QUIT, "Quit OpenPencil", Some(accel(Code::KeyQ))),
                ]);
                let _ = menu.append(&app_menu);
            }

            // File menu.
            let file = Submenu::new("File", true);
            let _ = file.append_items(&[
                &item(ID_NEW, "New", Some(accel(Code::KeyN))),
                &item(ID_OPEN, "Open\u{2026}", Some(accel(Code::KeyO))),
                &PredefinedMenuItem::separator(),
                &item(ID_SAVE, "Save", Some(accel(Code::KeyS))),
                &item(ID_SAVE_AS, "Save As\u{2026}", Some(accel_shift(Code::KeyS))),
                &PredefinedMenuItem::separator(),
                &item(ID_EXPORT, "Export Image\u{2026}", Some(accel_shift(Code::KeyP))),
            ]);
            // Windows has no app menu — Quit lives at the File-menu foot.
            #[cfg(not(target_os = "macos"))]
            {
                let _ = file.append(&PredefinedMenuItem::separator());
                let _ = file.append(&item(ID_QUIT, "Quit", Some(accel(Code::KeyQ))));
            }
            let _ = menu.append(&file);

            // Edit menu — custom items routed to the host's own
            // selection / clipboard ops (the canvas is not a native
            // text field, so `PredefinedMenuItem` copy/paste would be
            // inert here).
            let edit = Submenu::new("Edit", true);
            let _ = edit.append_items(&[
                &item(ID_UNDO, "Undo", Some(accel(Code::KeyZ))),
                &item(ID_REDO, "Redo", Some(accel_shift(Code::KeyZ))),
                &PredefinedMenuItem::separator(),
                &item(ID_CUT, "Cut", Some(accel(Code::KeyX))),
                &item(ID_COPY, "Copy", Some(accel(Code::KeyC))),
                &item(ID_PASTE, "Paste", Some(accel(Code::KeyV))),
                &item(ID_SELECT_ALL, "Select All", Some(accel(Code::KeyA))),
                &PredefinedMenuItem::separator(),
                &item(ID_DUPLICATE, "Duplicate", Some(accel(Code::KeyD))),
                &item(ID_GROUP, "Group", Some(accel(Code::KeyG))),
                &item(ID_UNGROUP, "Ungroup", Some(accel_shift(Code::KeyG))),
            ]);
            let _ = menu.append(&edit);

            // View menu.
            let view = Submenu::new("View", true);
            let fullscreen_accel = {
                #[cfg(target_os = "macos")]
                {
                    Accelerator::new(Some(Modifiers::META | Modifiers::CONTROL), Code::KeyF)
                }
                #[cfg(not(target_os = "macos"))]
                {
                    Accelerator::new(None, Code::F11)
                }
            };
            let _ = view.append(&item(
                ID_FULLSCREEN,
                "Toggle Full Screen",
                Some(fullscreen_accel),
            ));
            let _ = menu.append(&view);

            // Help menu.
            let help = Submenu::new("Help", true);
            let _ = help.append_items(&[
                &item(ID_CHECK_UPDATES, "Check for Updates\u{2026}", None),
                &PredefinedMenuItem::separator(),
                &item(ID_GITHUB, "OpenPencil on GitHub", None),
            ]);
            let _ = menu.append(&help);

            // Attach to the platform.
            #[cfg(target_os = "macos")]
            {
                let _ = window; // not needed — macOS attaches to the NSApp
                menu.init_for_nsapp();
            }
            #[cfg(target_os = "windows")]
            {
                if let Some(hwnd) = win32_hwnd(window) {
                    // SAFETY: `hwnd` is the live handle of the window
                    // winit just created; `muda` subclasses it to
                    // route `WM_COMMAND` to the menu event channel.
                    let _ = unsafe { menu.init_for_hwnd(hwnd) };
                }
            }

            Self { _menu: menu }
        }
    }

    #[cfg(target_os = "macos")]
    fn about_metadata() -> muda::AboutMetadata {
        muda::AboutMetadata {
            name: Some("OpenPencil".to_string()),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
            ..Default::default()
        }
    }

    /// Extract the Win32 `HWND` from a winit window (rwh_06 handle).
    #[cfg(target_os = "windows")]
    fn win32_hwnd(window: &winit::window::Window) -> Option<isize> {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        match window.window_handle().ok()?.as_raw() {
            RawWindowHandle::Win32(h) => Some(h.hwnd.get()),
            _ => None,
        }
    }

    /// Drain one pending menu selection, if any.
    pub fn poll() -> Option<MenuAction> {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if let Some(action) = action_for_id(event.id.as_ref()) {
                return Some(action);
            }
            // An unrecognized id (e.g. a predefined item handled
            // natively) — skip it and keep draining.
        }
        None
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn every_known_id_maps_to_an_action() {
            // The build appends exactly these ids; each must resolve.
            for id in [
                ID_NEW,
                ID_OPEN,
                ID_SAVE,
                ID_SAVE_AS,
                ID_EXPORT,
                ID_UNDO,
                ID_REDO,
                ID_CUT,
                ID_COPY,
                ID_PASTE,
                ID_SELECT_ALL,
                ID_DUPLICATE,
                ID_GROUP,
                ID_UNGROUP,
                ID_FULLSCREEN,
                ID_QUIT,
                ID_CHECK_UPDATES,
                ID_GITHUB,
            ] {
                assert!(action_for_id(id).is_some(), "id {id} should map");
            }
        }

        #[test]
        fn an_unknown_id_maps_to_nothing() {
            assert!(action_for_id("predefined-separator").is_none());
            assert!(action_for_id("").is_none());
        }
    }
}

// --------------------------------------------------------------------
// Other targets (Linux) — no native menu; the in-canvas File menu
// is the menu surface there.
// --------------------------------------------------------------------
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod backend {
    use super::MenuAction;

    pub struct AppMenu;

    impl AppMenu {
        pub fn install(_window: &winit::window::Window) -> Self {
            Self
        }
    }

    pub fn poll() -> Option<MenuAction> {
        None
    }
}

pub use backend::{poll, AppMenu};
