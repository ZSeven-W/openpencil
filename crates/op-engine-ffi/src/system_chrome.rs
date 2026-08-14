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
