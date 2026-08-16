//! Native application menu bar (macOS only).
//!
//! On macOS the native menu bar is built with `muda` and attached to
//! the running NSApp. Menu selections arrive on `muda`'s global event
//! channel; [`poll`] drains them into a [`MenuAction`] the runner maps
//! onto the same `WidgetHostNative` calls the keyboard shortcuts use.
//!
//! Windows and Linux have no native menu — the in-canvas File menu is
//! the primary menu surface. On Windows the window is borderless (custom
//! chrome), and a native `muda` menu drawn inside the client area would
//! flash during `drag_resize_window()` when the OS repaints the window.
//! `muda` is gated to macOS in `Cargo.toml`; other platforms get stubs.

/// A menu selection, decoupled from `muda` so the runner matches on
/// a plain enum. Each variant maps onto an existing host action.
///
/// `muda` only runs on macOS / Windows; the Linux `backend` is a
/// stub whose `poll()` always returns `None`, so on Linux every
/// variant is unconstructed by design. Silence `-D dead_code`
/// there without weakening the lint on the platforms that actually
/// build the menu.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    New,
    Open,
    /// Open the recent-file at this index into `editor_ui.recent_files`
    /// (the File ▸ Open Recent submenu is rebuilt in sync with that list,
    /// so the index is valid at click time).
    OpenRecent(usize),
    Save,
    SaveAs,
    /// Save the open document as a reusable `user:` scene template.
    SaveAsTemplate,
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
    ToggleGitPanel,
    ToggleDesignMdPanel,
    ToggleComponentBrowserPanel,
    Quit,
    CheckUpdates,
    OpenGithub,
}

// --------------------------------------------------------------------
// macOS — the real `muda` backend.
// --------------------------------------------------------------------
#[cfg(target_os = "macos")]
mod backend {
    use super::MenuAction;
    use muda::accelerator::{Accelerator, Code, Modifiers};
    use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
    use op_editor_core::Locale;

    /// Resolve the UI locale the same way app startup does (persisted
    /// settings override the OS locale). The menu is built before the
    /// live editor state is reachable from here, and locale changes are
    /// persisted on change, so this matches what the chrome shows.
    fn startup_locale() -> Locale {
        op_host_services::provider_probe::resolved_ui_locale()
    }

    fn tr(locale: Locale, key: &'static str) -> &'static str {
        op_i18n::translate(locale, key)
    }

    // Stable menu-item ids — the wire between the `muda` menu and
    // `action_for_id`. Kept as `&str` consts so the build + the
    // dispatch can't drift.
    const ID_NEW: &str = "new";
    const ID_OPEN: &str = "open";
    const ID_SAVE: &str = "save";
    const ID_SAVE_AS: &str = "save-as";
    const ID_SAVE_AS_TEMPLATE: &str = "save-as-template";
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
    const ID_GIT_PANEL: &str = "git-panel";
    const ID_DESIGN_MD: &str = "design-md";
    const ID_COMPONENT_BROWSER: &str = "component-browser";
    const ID_QUIT: &str = "quit";
    const ID_CHECK_UPDATES: &str = "check-updates";
    const ID_GITHUB: &str = "github";
    /// Prefix for File ▸ Open Recent items — the suffix is the index into
    /// `editor_ui.recent_files` (e.g. `recent:0`). The disabled empty-state
    /// item uses [`ID_RECENT_NONE`] and maps to no action.
    const ID_OPEN_RECENT_PREFIX: &str = "recent:";
    const ID_RECENT_NONE: &str = "recent-none";

    /// Map a `muda` menu-item id string onto a [`MenuAction`].
    fn action_for_id(id: &str) -> Option<MenuAction> {
        if let Some(index) = id.strip_prefix(ID_OPEN_RECENT_PREFIX) {
            return index.parse::<usize>().ok().map(MenuAction::OpenRecent);
        }
        Some(match id {
            ID_NEW => MenuAction::New,
            ID_OPEN => MenuAction::Open,
            ID_SAVE => MenuAction::Save,
            ID_SAVE_AS => MenuAction::SaveAs,
            ID_SAVE_AS_TEMPLATE => MenuAction::SaveAsTemplate,
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
            ID_GIT_PANEL => MenuAction::ToggleGitPanel,
            ID_DESIGN_MD => MenuAction::ToggleDesignMdPanel,
            ID_COMPONENT_BROWSER => MenuAction::ToggleComponentBrowserPanel,
            ID_QUIT => MenuAction::Quit,
            ID_CHECK_UPDATES => MenuAction::CheckUpdates,
            ID_GITHUB => MenuAction::OpenGithub,
            _ => return None,
        })
    }

    /// Owns the `muda` `Menu`. Kept alive for the process lifetime —
    /// dropping it would tear the native menu down. `recent_submenu` is
    /// held so File ▸ Open Recent can be rebuilt as the recent-file list
    /// changes ([`AppMenu::set_recent_files`]).
    pub struct AppMenu {
        _menu: Menu,
        recent_submenu: Submenu,
        /// Locale resolved at build time — reused when the recent-file
        /// submenu is rebuilt so its empty-state row stays translated.
        locale: Locale,
    }

    fn primary() -> Modifiers {
        Modifiers::META
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
        /// Must be called from `resumed`, after window creation — macOS
        /// needs the NSApp to exist.
        pub fn install(window: &winit::window::Window) -> Self {
            Self::install_with_locale(window, startup_locale())
        }

        /// Build the menu for an explicit locale. The runner calls this
        /// again when `EditorUiState.locale` changes so the native menu
        /// re-labels live instead of waiting for the next launch
        /// (`init_for_nsapp` replaces the previous NSApp main menu).
        pub fn install_with_locale(window: &winit::window::Window, locale: Locale) -> Self {
            let menu = Menu::new();

            // macOS app menu — About / Services / Hide / Quit, the
            // conventional first submenu macOS labels with the app
            // name. Quit is custom-id'd so the runner drives the same
            // clean-shutdown path as the window-close button.
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
                    &item(ID_QUIT, tr(locale, "menu.quit"), Some(accel(Code::KeyQ))),
                ]);
                let _ = menu.append(&app_menu);
            }

            // File menu. The Open Recent submenu is populated later via
            // `set_recent_files` (empty at build time → a disabled
            // placeholder until the runner seeds it from the recent list).
            let recent_submenu = Submenu::new(tr(locale, "menu.openRecent"), true);
            let file = Submenu::new(tr(locale, "menu.file"), true);
            let _ = file.append_items(&[
                &item(ID_NEW, tr(locale, "menu.new"), Some(accel(Code::KeyN))),
                &item(ID_OPEN, tr(locale, "menu.open"), Some(accel(Code::KeyO))),
                &recent_submenu,
                &PredefinedMenuItem::separator(),
                &item(ID_SAVE, tr(locale, "common.save"), Some(accel(Code::KeyS))),
                &item(
                    ID_SAVE_AS,
                    tr(locale, "menu.saveAs"),
                    Some(accel_shift(Code::KeyS)),
                ),
                // No accelerator: saving a template is a deliberate, occasional
                // action, not something a keystroke should risk by accident.
                &item(ID_SAVE_AS_TEMPLATE, tr(locale, "menu.saveAsTemplate"), None),
                &PredefinedMenuItem::separator(),
                &item(
                    ID_EXPORT,
                    tr(locale, "menu.exportImage"),
                    Some(accel_shift(Code::KeyP)),
                ),
            ]);
            let _ = menu.append(&file);

            // Edit menu — custom items routed to the host's own
            // selection / clipboard ops (the canvas is not a native
            // text field, so `PredefinedMenuItem` copy/paste would be
            // inert here).
            let edit = Submenu::new(tr(locale, "menu.edit"), true);
            let _ = edit.append_items(&[
                &item(ID_UNDO, tr(locale, "toolbar.undo"), Some(accel(Code::KeyZ))),
                &item(
                    ID_REDO,
                    tr(locale, "toolbar.redo"),
                    Some(accel_shift(Code::KeyZ)),
                ),
                &PredefinedMenuItem::separator(),
                &item(ID_CUT, tr(locale, "menu.cut"), Some(accel(Code::KeyX))),
                &item(ID_COPY, tr(locale, "menu.copy"), Some(accel(Code::KeyC))),
                &item(ID_PASTE, tr(locale, "menu.paste"), Some(accel(Code::KeyV))),
                &item(
                    ID_SELECT_ALL,
                    tr(locale, "menu.selectAll"),
                    Some(accel(Code::KeyA)),
                ),
                &PredefinedMenuItem::separator(),
                &item(
                    ID_DUPLICATE,
                    tr(locale, "common.duplicate"),
                    Some(accel(Code::KeyD)),
                ),
                &item(ID_GROUP, tr(locale, "menu.group"), Some(accel(Code::KeyG))),
                &item(
                    ID_UNGROUP,
                    tr(locale, "menu.ungroup"),
                    Some(accel_shift(Code::KeyG)),
                ),
            ]);
            let _ = menu.append(&edit);

            // View menu.
            let view = Submenu::new(tr(locale, "menu.view"), true);
            let fullscreen_accel =
                Accelerator::new(Some(Modifiers::META | Modifiers::CONTROL), Code::KeyF);
            let _ = view.append(&item(
                ID_FULLSCREEN,
                tr(locale, "menu.toggleFullScreen"),
                Some(fullscreen_accel),
            ));
            let _ = view.append(&PredefinedMenuItem::separator());
            let _ = view.append(&item(ID_GIT_PANEL, tr(locale, "menu.gitPanel"), None));
            let _ = view.append(&item(ID_DESIGN_MD, tr(locale, "menu.designMdPanel"), None));
            let _ = view.append(&item(
                ID_COMPONENT_BROWSER,
                tr(locale, "componentBrowser.title"),
                None,
            ));
            let _ = menu.append(&view);

            // Help menu.
            let help = Submenu::new(tr(locale, "menu.help"), true);
            let _ = help.append_items(&[
                &item(ID_CHECK_UPDATES, tr(locale, "menu.checkForUpdates"), None),
                &PredefinedMenuItem::separator(),
                &item(ID_GITHUB, tr(locale, "menu.githubLink"), None),
            ]);
            let _ = menu.append(&help);

            // Attach to the platform.
            let _ = window; // not needed — macOS attaches to the NSApp
            menu.init_for_nsapp();

            let this = Self {
                _menu: menu,
                recent_submenu,
                locale,
            };
            this.set_recent_files(&[]); // seed the disabled placeholder
            this
        }

        /// Locale the menu labels were built with.
        pub fn locale(&self) -> Locale {
            self.locale
        }

        /// Rebuild the File ▸ Open Recent submenu from `labels` (newest
        /// first). Item `i` gets id `recent:i`, matching the runner's
        /// `FileAction::OpenRecent(i)`; an empty list shows one disabled
        /// "No Recent Documents" row. Call whenever `recent_files` changes.
        pub fn set_recent_files(&self, labels: &[String]) {
            while self.recent_submenu.remove_at(0).is_some() {}
            if labels.is_empty() {
                let none = MenuItem::with_id(
                    ID_RECENT_NONE,
                    tr(self.locale, "menu.noRecentDocuments"),
                    false,
                    None,
                );
                let _ = self.recent_submenu.append(&none);
                return;
            }
            for (i, label) in labels.iter().enumerate() {
                let entry =
                    MenuItem::with_id(format!("{ID_OPEN_RECENT_PREFIX}{i}"), label, true, None);
                let _ = self.recent_submenu.append(&entry);
            }
        }
    }

    fn about_metadata() -> muda::AboutMetadata {
        muda::AboutMetadata {
            name: Some("OpenPencil".to_string()),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
            ..Default::default()
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
                ID_SAVE_AS_TEMPLATE,
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
                ID_GIT_PANEL,
                ID_DESIGN_MD,
                ID_COMPONENT_BROWSER,
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
            assert!(action_for_id("export-all-frames").is_none());
            assert!(action_for_id("").is_none());
        }

        #[test]
        fn recent_ids_map_to_open_recent() {
            assert_eq!(action_for_id("recent:0"), Some(MenuAction::OpenRecent(0)));
            assert_eq!(action_for_id("recent:9"), Some(MenuAction::OpenRecent(9)));
            // The disabled placeholder + malformed suffixes map to nothing.
            assert!(action_for_id(ID_RECENT_NONE).is_none());
            assert!(action_for_id("recent:").is_none());
            assert!(action_for_id("recent:x").is_none());
        }
    }
}

// --------------------------------------------------------------------
// Other targets (Windows / Linux) — no native menu; the in-canvas
// File menu is the menu surface there.
// --------------------------------------------------------------------
#[cfg(not(target_os = "macos"))]
mod backend {
    use super::MenuAction;
    use op_editor_core::Locale;

    pub struct AppMenu {
        locale: Locale,
    }

    impl AppMenu {
        pub fn install(window: &winit::window::Window) -> Self {
            Self::install_with_locale(window, Locale::EnUs)
        }

        /// No native menu off macOS — stores the locale so the runner's
        /// change detection settles after one no-op rebuild.
        pub fn install_with_locale(_window: &winit::window::Window, locale: Locale) -> Self {
            Self { locale }
        }

        pub fn locale(&self) -> Locale {
            self.locale
        }

        /// No native menu off macOS — the in-canvas File menu shows recents.
        pub fn set_recent_files(&self, _labels: &[String]) {}
    }

    pub fn poll() -> Option<MenuAction> {
        None
    }
}

pub use backend::{poll, AppMenu};
