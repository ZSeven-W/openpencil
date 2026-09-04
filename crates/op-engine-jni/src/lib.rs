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

#[cfg(any(target_os = "android", test))]
fn background_work_or_false(status: op_engine_ffi::OpStatus, active: bool) -> bool {
    status == op_engine_ffi::OpStatus::Ok && active
}

/// Converts an Android `String` offset into the editor ABI's UTF-8 byte unit.
/// Negative and oversized offsets clamp to the text bounds; a UTF-16 offset
/// inside a surrogate pair snaps to that scalar's start.
#[cfg(any(target_os = "android", test))]
fn utf16_offset_to_utf8_byte(text: &str, offset: i32) -> usize {
    let target = usize::try_from(offset).unwrap_or(0);
    let mut utf16_units = 0usize;
    for (byte, ch) in text.char_indices() {
        if target <= utf16_units {
            return byte;
        }
        let next = utf16_units.saturating_add(ch.len_utf16());
        if target < next {
            return byte;
        }
        utf16_units = next;
    }
    text.len()
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
mod background_work_tests {
    use super::*;

    #[test]
    fn only_a_successful_active_result_keeps_the_platform_service_alive() {
        assert!(background_work_or_false(op_engine_ffi::OpStatus::Ok, true));
        assert!(!background_work_or_false(
            op_engine_ffi::OpStatus::Ok,
            false
        ));
        assert!(!background_work_or_false(
            op_engine_ffi::OpStatus::InvalidArg,
            true
        ));
        assert!(!background_work_or_false(
            op_engine_ffi::OpStatus::Poisoned,
            true
        ));
    }
}

#[cfg(test)]
mod editor_ime_offset_tests {
    use super::utf16_offset_to_utf8_byte;

    #[test]
    fn converts_bmp_utf16_offsets_to_utf8_bytes() {
        let text = "中a文";
        assert_eq!(utf16_offset_to_utf8_byte(text, 0), 0);
        assert_eq!(utf16_offset_to_utf8_byte(text, 1), 3);
        assert_eq!(utf16_offset_to_utf8_byte(text, 2), 4);
        assert_eq!(utf16_offset_to_utf8_byte(text, 3), 7);
    }

    #[test]
    fn clamps_invalid_offsets_and_snaps_inside_surrogate_pairs() {
        let text = "a😀中";
        assert_eq!(utf16_offset_to_utf8_byte(text, -7), 0);
        assert_eq!(utf16_offset_to_utf8_byte(text, 1), 1);
        assert_eq!(utf16_offset_to_utf8_byte(text, 2), 1);
        assert_eq!(utf16_offset_to_utf8_byte(text, 3), 5);
        assert_eq!(utf16_offset_to_utf8_byte(text, 4), text.len());
        assert_eq!(utf16_offset_to_utf8_byte(text, i32::MAX), text.len());
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
    fn java_strings_are_decoded_to_canonical_utf8_before_ffi() {
        let source = include_str!("bindings.rs");
        let start = source.find("fn jstring_bytes").expect("JNI string helper");
        let function = &source[start..];
        assert!(function.contains("String::from(value).into_bytes()"));
        assert!(!function.contains("s.to_bytes().to_vec()"));
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
    fn background_natives_forward_only_through_the_owner_thread_dispatch() {
        let source = include_str!("bindings.rs");
        for (export, ffi) in [
            (
                "nativeHasBackgroundWork",
                "op_has_background_work(e, &mut active)",
            ),
            (
                "nativeBackgroundTick",
                "op_background_tick(e, now_ms.max(0) as u64, &mut active)",
            ),
        ] {
            let signature = format!("fn {CANONICAL_JNI_PREFIX}{export}");
            let start = source.find(&signature).expect("background JNI export");
            let tail = &source[start..];
            let end = tail[1..]
                .find("#[no_mangle]")
                .map_or(tail.len(), |offset| offset + 1);
            let function = &tail[..end];
            assert!(function.contains("with_engine(engine"));
            assert!(function.contains(ffi));
            assert!(function.contains("background_work_or_false"));
            assert!(!function.contains("op_frame("));
        }
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
    fn editor_pointer_at_natives_forward_factual_timestamps_clamped_at_zero() {
        let source = include_str!("bindings_editor.rs");
        for (export, ffi) in [
            (
                "nativeEditorPressAt",
                "op_engine_ffi::op_editor_press_at(e, x, y, t_ms.max(0) as u64)",
            ),
            (
                "nativeEditorMoveAt",
                "op_engine_ffi::op_editor_move_at(e, x, y, t_ms.max(0) as u64)",
            ),
            (
                "nativeEditorReleaseAt",
                "op_engine_ffi::op_editor_release_at(e, x, y, t_ms.max(0) as u64)",
            ),
            (
                "nativeEditorCancelGestureAt",
                "op_engine_ffi::op_editor_cancel_gesture_at(e, t_ms.max(0) as u64)",
            ),
        ] {
            let signature = format!("fn {CANONICAL_JNI_PREFIX}{export}");
            let start = source
                .find(&signature)
                .unwrap_or_else(|| panic!("{export} JNI export missing"));
            let tail = &source[start..];
            let end = tail[1..]
                .find("#[no_mangle]")
                .map_or(tail.len(), |offset| offset + 1);
            let function = &tail[..end];
            assert!(function.contains("call_status(engine"));
            assert!(function.contains(ffi));
            // The signed jlong clock must clamp at zero before u64 — a
            // negative Android clock is never a valid timestamp.
            assert!(!function.contains("t_ms as u64"));
        }
    }

    #[test]
    fn editor_image_import_native_owns_java_arguments_before_dispatch() {
        let source = include_str!("bindings_editor.rs");
        let start = source
            .find("Java_tech_zseven_openpencil_OpNative_nativeEditorImportImageOrSvg")
            .expect("image import JNI export");
        let tail = &source[start..];
        let end = tail[1..]
            .find("#[no_mangle]")
            .map_or(tail.len(), |offset| offset + 1);
        let function = &tail[..end];

        assert!(function.contains("env.convert_byte_array(&data)"));
        assert!(function.contains("jstring_bytes(&mut env, &file_name)"));
        assert!(function.contains("call_status(engine"));
        assert!(function.contains("op_engine_ffi::op_editor_import_image_or_svg("));
    }

    #[test]
    fn editor_preedit_converts_android_utf16_offsets_before_ffi() {
        let source = include_str!("bindings_editor.rs");
        let start = source
            .find("Java_tech_zseven_openpencil_OpNative_nativeEditorImePreedit")
            .expect("editor preedit JNI export");
        let tail = &source[start..];
        let end = tail[1..]
            .find("#[no_mangle]")
            .map_or(tail.len(), |offset| offset + 1);
        let function = &tail[..end];

        assert_eq!(function.matches("utf16_offset_to_utf8_byte").count(), 2);
        assert!(function.contains("op_engine_ffi::op_editor_ime_preedit("));
        assert!(!function.contains("sel_start as usize"));
        assert!(!function.contains("sel_end as usize"));
    }

    #[test]
    fn credential_callbacks_attach_workers_bound_lengths_and_wipe_arrays() {
        let source = include_str!("callbacks.rs");
        let redraw = &source[source.find("extern \"C\" fn needs_redraw").unwrap()
            ..source.find("extern \"C\" fn runtime_error").unwrap()];
        let secure = &source[source.find("extern \"C\" fn credential_load").unwrap()
            ..source.find("fn clear_pending_exception").unwrap()];
        assert_eq!(redraw.matches("attached_upcall(ctx").count(), 1);
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
