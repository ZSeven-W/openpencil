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
    const CANONICAL_JNI_PREFIX: &str = "Java_tech_zseven_openpencil_OpNative_";

    #[test]
    fn every_native_export_uses_the_canonical_android_package() {
        for source in [
            include_str!("bindings.rs"),
            include_str!("bindings_editor.rs"),
            include_str!("bindings_media.rs"),
            include_str!("bindings_text.rs"),
        ] {
            assert!(!source.contains("Java_dev_openpencil_player_"));
            for line in source.lines().filter(|line| line.contains("fn Java_")) {
                assert!(
                    line.contains(CANONICAL_JNI_PREFIX),
                    "stale JNI export: {line}"
                );
            }
        }
    }

    #[test]
    fn atomic_resize_native_forwards_the_complete_tuple_once() {
        let source = include_str!("bindings.rs");
        let start = source
            .find("Java_tech_zseven_openpencil_OpNative_nativeResizeWithSafeArea")
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
            .find("Java_tech_zseven_openpencil_OpNative_nativeEditorBeginTransform")
            .expect("editor transform begin JNI export");
        let function = &source[start..];
        assert!(function.contains("op_engine_ffi::op_editor_begin_transform(e, x, y)"));
    }

    #[test]
    fn credential_callbacks_attach_workers_bound_lengths_and_wipe_arrays() {
        let source = include_str!("callbacks.rs");
        let secure = &source[source.find("extern \"C\" fn credential_load").unwrap()
            ..source.find("fn clear_pending_exception").unwrap()];
        assert!(source.contains("ctx.vm.attach_current_thread()"));
        assert!(source.contains("length == COLLAB_CREDENTIAL_BYTES"));
        assert!(source.contains("value_len != COLLAB_CREDENTIAL_BYTES"));
        assert!(source.contains("struct WipedCredential"));
        assert!(source.contains("self.0.zeroize()"));
        assert!(source.contains("wipe_java_byte_array"));
        assert!(source.matches("wipe_java_byte_array(").count() >= 3);
        assert!(source.contains(".set_byte_array_region(array"));
        assert!(source.contains("clear_pending_exception"));
        assert!(secure.matches("with_local_frame(").count() >= 2);
        assert!(!secure.contains("Vec<"));
        assert!(!secure.contains("value_len as i32"));
    }

    #[test]
    fn native_create_forwards_the_precreate_storage_root() {
        let source = include_str!("bindings.rs");
        assert!(source.contains("storage_root: JString<'local>"));
        assert!(source.contains("storage_root_ptr: storage_root.as_ptr()"));
        assert!(source.contains("storage_root_len: storage_root.len()"));
    }

    #[test]
    fn callback_context_is_freed_only_after_clean_worker_shutdown() {
        let source = include_str!("bindings.rs");
        let start = source.find("let status = unsafe { op_destroy").unwrap();
        let teardown = &source[start..source[start..].find("};\n    if thread").unwrap() + start];
        assert!(teardown.contains("if matches!(status, OpStatus::Ok)"));
        let ok_block = &teardown[teardown.find("if matches!").unwrap()..];
        assert!(ok_block.contains("release_all_windows"));
        assert!(ok_block.contains("drop_ctx(ctx.get())"));
    }
}
