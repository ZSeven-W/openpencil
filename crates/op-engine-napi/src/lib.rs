//! OpenHarmony (OHOS) NAPI marshalling layer for the OpenPencil engine
//! player — the `libopenpencil.so` an ArkTS shell loads with
//! `import native from 'libopenpencil.so'`.
//!
//! This is the OHOS twin of `op-engine-jni`: the same `op-engine-ffi` C ABI,
//! the same one-engine-thread marshalling discipline, the same tombstoning
//! handle table — reached through Node-API instead of JNI. The exported
//! function names are the Kotlin `OpNative` contract translated to camelCase
//! without the `native` prefix (`nativeEditorPress` → `editorPress`), so the
//! two mobile shells stay reviewable side by side. See `README.md` for the
//! full API table.
//!
//! Module layout:
//! * [`action`] — platform-free constants and event mappings. Compiled on
//!   every host, and the only part of this crate `cargo test` can exercise
//!   without a device.
//! * `bindings*` — the NAPI entry points, split the same way the JNI layer
//!   splits its natives.
//! * [`callbacks`] — engine → ArkTS upcalls (threadsafe functions).
//! * `module` — the Node-API module surface: the callback object, the
//!   XComponent lifecycle subscription, and the load-time registration hook.
//! * [`window`] / [`xcomponent`] — surface ownership and the hand-rolled
//!   `OH_NativeXComponent` bindings.
//! * [`hilog`] — the HiLog writer.
//!
//! The engine-thread queue and the handle registry are NOT re-implemented
//! here: they are pure `std` and are imported from `op_engine_jni` so the two
//! mobile bindings can never drift on teardown ordering.
//!
//! Everything OHOS-specific is gated on
//! `all(target_os = "linux", target_env = "ohos")`, so on every other host
//! this crate is an inert stub that still compiles, lints, and runs its
//! contract tests.

pub mod action;

#[cfg(all(target_os = "linux", target_env = "ohos"))]
pub mod bindings;
#[cfg(all(target_os = "linux", target_env = "ohos", feature = "editor"))]
pub mod bindings_editor;
#[cfg(all(target_os = "linux", target_env = "ohos", feature = "editor"))]
pub mod bindings_input;
#[cfg(all(target_os = "linux", target_env = "ohos"))]
pub mod bindings_media;
#[cfg(all(target_os = "linux", target_env = "ohos"))]
pub mod bindings_text;
#[cfg(all(target_os = "linux", target_env = "ohos"))]
pub mod callbacks;
#[cfg(all(target_os = "linux", target_env = "ohos"))]
pub mod hilog;
#[cfg(all(target_os = "linux", target_env = "ohos"))]
pub mod module;
#[cfg(all(target_os = "linux", target_env = "ohos"))]
pub mod window;
#[cfg(all(target_os = "linux", target_env = "ohos"))]
pub mod xcomponent;

#[cfg(all(target_os = "linux", target_env = "ohos"))]
pub use module::EngineCallbacks;

pub use action::STATUS_CLOSING;
pub use op_engine_jni::{Dispatch, EngineThread, Registry};

/// Whether the platform bars should use light-colored icons. Any failed read
/// falls back to dark icons, so a broken engine can never leave the status
/// bar unreadable. Shared with the Android layer's identical rule.
#[cfg(any(all(target_os = "linux", target_env = "ohos"), test))]
fn system_icon_preference_or_false(status: op_engine_ffi::OpStatus, prefers_light: bool) -> bool {
    status == op_engine_ffi::OpStatus::Ok && prefers_light
}

// ---- Contract tests ------------------------------------------------------

#[cfg(test)]
mod system_chrome_tests {
    use super::*;

    #[test]
    fn failures_and_false_preferences_fall_back_to_dark_icons() {
        assert!(!system_icon_preference_or_false(
            op_engine_ffi::OpStatus::InvalidArg,
            true
        ));
        assert!(!system_icon_preference_or_false(
            op_engine_ffi::OpStatus::Ok,
            false
        ));
        assert!(system_icon_preference_or_false(
            op_engine_ffi::OpStatus::Ok,
            true
        ));
    }
}

/// Source-level contract tests.
///
/// The NAPI surface cannot be compiled on this host (it needs the OHOS
/// target and the NDK sysroot), and `packaging/harmony` is owned by the app
/// side — so the ArkTS-visible names and the shell action codes are pinned
/// HERE, by scanning the binding sources and `op_engine.h`. A rename that the
/// README table does not also carry fails `cargo test -p op-engine-napi`.
#[cfg(test)]
mod contract_tests {
    /// Every function this module exports to ArkTS, in the canonical order:
    /// the `OpNative.kt` order first (camelCase, `native` prefix dropped),
    /// then the OHOS-only additions. `README.md` documents this exact list.
    pub const EXPORTED_NAMES: &[&str] = &[
        // --- Lifecycle (OpNative.kt order) ---
        "create",
        "lastError",
        "attachSurface",
        "suspend",
        "resume",
        "resize",
        "resizeWithSafeArea",
        "setSafeArea",
        "setKeyboard",
        "prefersLightSystemIcons",
        "frame",
        "pointer",
        "destroy",
        // --- Text editing + IME ---
        "textBegin",
        "textEnd",
        "textInsert",
        "textBackspace",
        "textDeleteForward",
        "textSetCaret",
        "textSelectRange",
        "imeSetComposingText",
        "imeCommitComposition",
        "imeCancelComposition",
        "textGetState",
        "textCaretRect",
        // --- Remote images / fonts / pages ---
        "remoteImageResult",
        "registerFont",
        "getPageCount",
        "setActivePage",
        // --- Full-editor mode ---
        "editorPress",
        "editorMove",
        "editorRelease",
        "editorCancelGesture",
        "editorBeginTransform",
        "editorRightPress",
        "editorPan",
        "editorPinch",
        "editorText",
        "editorKey",
        "editorImePreedit",
        "editorImeCommit",
        "editorImeFocused",
        "editorConfigureAuth",
        "editorTakeLoginUrl",
        "editorCancelLogin",
        "editorTakeShellAction",
        "editorOpenDocument",
        "editorExportFileName",
        "editorExportToPath",
        "editorCancelExport",
        "editorAccountSnapshot",
        "editorSignOut",
        "editorBeginLogin",
        "editorSetLocale",
        "editorLocaleCode",
        // --- OHOS-only additions (no Android counterpart) ---
        "setXcomponentListener",
        "pixelSize",
        "touchEvent",
        "editorOpenDocumentPath",
        "mouseMove",
        "mouseButton",
        "mouseWheel",
        "keyEvent",
        "clipboardSetText",
        "clipboardGetText",
        "editorSetTouchChrome",
        "editorHover",
        "editorWheel",
        "keyModifiers",
        "setImeConduitAttached",
    ];

    /// Every source file that may carry a `#[napi]` export.
    const SOURCES: &[&str] = &[
        include_str!("bindings.rs"),
        include_str!("bindings_editor.rs"),
        include_str!("bindings_input.rs"),
        include_str!("bindings_media.rs"),
        include_str!("bindings_text.rs"),
        include_str!("module.rs"),
    ];

    /// Every explicit `js_name` literal declared across the binding sources.
    fn declared_names() -> Vec<String> {
        let mut names = Vec::new();
        for source in SOURCES {
            for line in source.lines() {
                let Some(rest) = line.split_once("js_name = \"") else {
                    continue;
                };
                let Some((name, _)) = rest.1.split_once('"') else {
                    continue;
                };
                names.push(name.to_owned());
            }
        }
        names
    }

    #[test]
    fn every_exported_name_is_declared_exactly_once() {
        let declared = declared_names();
        for name in EXPORTED_NAMES {
            let count = declared.iter().filter(|found| *found == name).count();
            assert_eq!(count, 1, "`{name}` must be exported exactly once");
        }
    }

    #[test]
    fn no_undocumented_exports_leak_into_the_module() {
        for name in declared_names() {
            assert!(
                EXPORTED_NAMES.contains(&name.as_str()),
                "`{name}` is exported but missing from EXPORTED_NAMES (and the README table)"
            );
        }
    }

    #[test]
    fn every_napi_export_pins_an_explicit_js_name() {
        // `#[napi]` would auto-camelCase the Rust name; the ArkTS contract is
        // too important to leave to a derive's casing rules.
        for source in SOURCES {
            for line in source.lines() {
                let trimmed = line.trim();
                if !trimmed.starts_with("#[napi") || trimmed.starts_with("#[napi(object") {
                    continue;
                }
                assert!(
                    trimmed.contains("js_name = \""),
                    "bare `{trimmed}` — every exported function needs an explicit js_name"
                );
            }
        }
    }

    #[test]
    fn the_kotlin_derived_prefix_matches_opnative_order() {
        // The first 55 names are `OpNative.kt`'s own order; the additions
        // follow. A new OHOS-only export must be appended, never spliced in.
        const KOTLIN_DERIVED: usize = 55;
        assert_eq!(EXPORTED_NAMES[KOTLIN_DERIVED], "setXcomponentListener");
        assert_eq!(EXPORTED_NAMES[0], "create");
        assert_eq!(EXPORTED_NAMES[KOTLIN_DERIVED - 1], "editorLocaleCode");
        assert_eq!(EXPORTED_NAMES.len(), 70);
    }

    /// The engine's own C header — the single source of truth for the codes
    /// the ArkTS shell switches on.
    const HEADER: &str = include_str!("../../op-engine-ffi/include/op_engine.h");

    #[test]
    fn shell_action_codes_match_the_c_header() {
        use crate::action::*;
        for (name, code) in [
            ("OpShellAction_None", SHELL_ACTION_NONE),
            ("OpShellAction_OpenDocument", SHELL_ACTION_OPEN_DOCUMENT),
            (
                "OpShellAction_OpenLoginWebView",
                SHELL_ACTION_OPEN_LOGIN_WEBVIEW,
            ),
            (
                "OpShellAction_CloseLoginWebView",
                SHELL_ACTION_CLOSE_LOGIN_WEBVIEW,
            ),
            ("OpShellAction_ExportDocument", SHELL_ACTION_EXPORT_DOCUMENT),
            (
                "OpShellAction_OpenAccountCenter",
                SHELL_ACTION_OPEN_ACCOUNT_CENTER,
            ),
            ("OpShellAction_RequestLogin", SHELL_ACTION_REQUEST_LOGIN),
            (
                "OpShellAction_OpenLanguagePicker",
                SHELL_ACTION_OPEN_LANGUAGE_PICKER,
            ),
            ("OpShellAction_WindowClose", SHELL_ACTION_WINDOW_CLOSE),
            ("OpShellAction_WindowMinimize", SHELL_ACTION_WINDOW_MINIMIZE),
            ("OpShellAction_WindowZoom", SHELL_ACTION_WINDOW_ZOOM),
        ] {
            assert!(
                HEADER.contains(&format!("{name} = {code},")),
                "`{name}` is not `{code}` in op_engine.h"
            );
        }
    }

    #[test]
    fn auth_region_and_pointer_phase_codes_match_the_c_header() {
        use crate::action::*;
        for (name, code) in [
            ("OpAuthRegion_China", AUTH_REGION_CHINA),
            ("OpAuthRegion_Global", AUTH_REGION_GLOBAL),
            ("OpPointerPhase_Down", POINTER_PHASE_DOWN),
            ("OpPointerPhase_Move", POINTER_PHASE_MOVE),
            ("OpPointerPhase_Up", POINTER_PHASE_UP),
            ("OpPointerPhase_Cancel", POINTER_PHASE_CANCEL),
        ] {
            assert!(
                HEADER.contains(&format!("{name} = {code},")),
                "`{name}` is not `{code}` in op_engine.h"
            );
        }
    }

    #[test]
    fn editor_key_codes_match_the_c_header() {
        use crate::action::*;
        for (name, code) in [
            ("OpKey_Backspace", OP_KEY_BACKSPACE),
            ("OpKey_Delete", OP_KEY_DELETE),
            ("OpKey_Enter", OP_KEY_ENTER),
            ("OpKey_Escape", OP_KEY_ESCAPE),
            ("OpKey_Duplicate", OP_KEY_DUPLICATE),
            ("OpKey_Undo", OP_KEY_UNDO),
            ("OpKey_Redo", OP_KEY_REDO),
            ("OpKey_ArrowUp", OP_KEY_ARROW_UP),
            ("OpKey_ArrowDown", OP_KEY_ARROW_DOWN),
            ("OpKey_ArrowLeft", OP_KEY_ARROW_LEFT),
            ("OpKey_ArrowRight", OP_KEY_ARROW_RIGHT),
        ] {
            assert!(
                HEADER.contains(&format!("{name} = {code},")),
                "`{name}` is not `{code}` in op_engine.h"
            );
        }
    }

    #[test]
    fn every_ohos_module_is_target_gated() {
        // A missing gate would drag napi/NDK symbols into the host build.
        for source in [
            include_str!("bindings.rs"),
            include_str!("bindings_editor.rs"),
            include_str!("bindings_input.rs"),
            include_str!("bindings_media.rs"),
            include_str!("bindings_text.rs"),
            include_str!("callbacks.rs"),
            include_str!("hilog.rs"),
            include_str!("module.rs"),
            include_str!("window.rs"),
            include_str!("xcomponent.rs"),
        ] {
            assert!(
                source.contains("#![cfg(all(target_os = \"linux\", target_env = \"ohos\""),
                "every OHOS module needs its own inner cfg gate"
            );
        }
    }

    #[test]
    fn the_readme_documents_every_exported_name() {
        const README: &str = include_str!("../README.md");
        for name in EXPORTED_NAMES {
            assert!(
                README.contains(&format!("`{name}`")),
                "`{name}` is exported but absent from the README API table"
            );
        }
    }
}
