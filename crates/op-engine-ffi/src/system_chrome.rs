//! Platform system-chrome appearance derived from the rendered surface.

use crate::error::FfiError;
use crate::lifecycle::call_session;
use crate::OpStatus;

#[cfg(feature = "editor")]
fn prefers_light_system_icons(session: &crate::lifecycle::Session) -> bool {
    session
        .editor()
        .map(|host| host.editor_state().editor_ui.theme_mode == op_editor_core::ThemeMode::Dark)
        .unwrap_or(false)
}

#[cfg(not(feature = "editor"))]
fn prefers_light_system_icons(_session: &crate::lifecycle::Session) -> bool {
    false
}

/// Whether platform status/navigation bars should use light-colored icons.
///
/// Full-editor mode follows the editor chrome theme. The viewer paints a
/// light canvas backdrop, so it always asks the shell for dark icons.
///
/// # Safety
///
/// `engine` must be live and `out` must be writable.
#[no_mangle]
pub unsafe extern "C" fn op_prefers_light_system_icons(
    engine: *mut crate::OpEngine,
    out: *mut bool,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if out.is_null() {
                return Err(FfiError::invalid(
                    "system-icon preference output pointer is null",
                ));
            }
            out.write(prefers_light_system_icons(session));
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desc::{Callbacks, CreateOptions};
    use crate::lifecycle::{OpEngine, Session};

    const SAMPLE_DOC: &str =
        include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");

    fn engine(editor_mode: bool) -> OpEngine {
        #[cfg(not(feature = "editor"))]
        let _ = editor_mode;
        OpEngine::new(
            Session::new(CreateOptions {
                document: SAMPLE_DOC.to_owned(),
                width: 800.0,
                height: 600.0,
                dpr: 1.0,
                callbacks: Callbacks::default(),
                asset_base: None,
                #[cfg(feature = "editor")]
                editor_mode,
            })
            .expect("engine session"),
        )
    }

    #[test]
    fn null_output_is_invalid() {
        let mut engine = engine(false);
        assert_eq!(
            unsafe { op_prefers_light_system_icons(&mut engine, std::ptr::null_mut()) },
            OpStatus::InvalidArg
        );
    }

    #[test]
    fn viewer_prefers_dark_system_icons() {
        let mut engine = engine(false);
        let mut prefers_light = true;
        assert_eq!(
            unsafe { op_prefers_light_system_icons(&mut engine, &mut prefers_light) },
            OpStatus::Ok
        );
        assert!(!prefers_light);
    }

    #[cfg(feature = "editor")]
    #[test]
    fn editor_system_icons_follow_dark_and_light_themes() {
        let mut engine = engine(true);
        let pointer = &mut engine as *mut OpEngine;

        engine
            .session_mut_for_test()
            .editor
            .as_mut()
            .expect("editor host")
            .editor_state_mut()
            .editor_ui
            .theme_mode = op_editor_core::ThemeMode::Dark;
        let mut prefers_light = false;
        assert_eq!(
            unsafe { op_prefers_light_system_icons(pointer, &mut prefers_light) },
            OpStatus::Ok
        );
        assert!(prefers_light);

        engine
            .session_mut_for_test()
            .editor
            .as_mut()
            .expect("editor host")
            .editor_state_mut()
            .editor_ui
            .theme_mode = op_editor_core::ThemeMode::Light;
        assert_eq!(
            unsafe { op_prefers_light_system_icons(pointer, &mut prefers_light) },
            OpStatus::Ok
        );
        assert!(!prefers_light);
    }
}

/// Apply a UI locale by BCP-47 tag ("zh-CN", "en-US", …). Unsupported tags
/// are rejected so the shell's picker cannot silently fall back.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread; a non-empty byte
/// range must cover readable UTF-8 for its declared length.
/// Select the chrome family for this engine. Mobile shells default to the
/// touch chrome; a desktop-class device (HarmonyOS 2in1 / PC) passes
/// `enabled = 0` to get the full desktop layout — top bar, side panels, and
/// pointer-first interactions — from the same engine build.
///
/// # Safety
/// `engine` must be a live engine on its owner thread.
#[cfg(feature = "editor")]
#[no_mangle]
pub unsafe extern "C" fn op_editor_set_touch_chrome(
    engine: *mut crate::OpEngine,
    enabled: i32,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let host = session.editor_mut()?;
            let state = host.editor_state_mut();
            let ui = &mut state.editor_ui;
            ui.touch = enabled != 0;
            // Desktop chrome opens the rail layout the way the desktop
            // binary does; touch chrome re-applies the mobile default.
            ui.sidebar_open = if ui.touch {
                ui.size_class.is_rail_layout()
            } else {
                true
            };
            host.mark_editor_state_dirty();
            session.request_redraw();
            Ok(())
        })
    }
}
