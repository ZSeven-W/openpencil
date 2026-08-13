//! Android JNI marshalling layer for the OpenPencil engine player.
//!
//! `engine_thread` is the host-testable queue core and `registry` is the
//! host-testable handle table; the JNI bindings, callback trampolines, and
//! window ownership are Android-only modules.

pub mod engine_thread;
pub mod registry;

#[cfg(target_os = "android")]
pub mod alog;
#[cfg(target_os = "android")]
pub mod bindings;
#[cfg(all(target_os = "android", feature = "editor"))]
mod bindings_editor;
#[cfg(target_os = "android")]
mod bindings_media;
#[cfg(target_os = "android")]
mod bindings_text;
#[cfg(target_os = "android")]
pub mod callbacks;
#[cfg(target_os = "android")]
pub mod window;

pub use engine_thread::{Dispatch, EngineThread, STATUS_CLOSING};
pub use registry::Registry;

#[cfg(any(target_os = "android", test))]
fn system_icon_preference_or_false(status: op_engine_ffi::OpStatus, prefers_light: bool) -> bool {
    status == op_engine_ffi::OpStatus::Ok && prefers_light
}

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

#[cfg(test)]
mod binding_contract_tests {
    #[test]
    fn atomic_resize_native_forwards_the_complete_tuple_once() {
        let source = include_str!("bindings.rs");
        let start = source
            .find("Java_dev_openpencil_player_OpNative_nativeResizeWithSafeArea")
            .expect("atomic resize JNI export");
        let tail = &source[start..];
        let end = tail[1..]
            .find("#[no_mangle]")
            .map_or(tail.len(), |offset| offset + 1);
        let function = &tail[..end];

        assert!(function.contains("op_resize_with_safe_area(e, w, h, dpr, t, r, b, l)"));
        assert_eq!(function.matches("op_resize_with_safe_area(").count(), 1);
        assert!(!function.contains("op_set_safe_area("));
    }

    #[test]
    fn editor_transform_begin_native_forwards_the_down_midpoint() {
        let source = include_str!("bindings_editor.rs");
        let start = source
            .find("Java_dev_openpencil_player_OpNative_nativeEditorBeginTransform")
            .expect("editor transform begin JNI export");
        let function = &source[start..];
        assert!(function.contains("op_engine_ffi::op_editor_begin_transform(e, x, y)"));
    }
}
