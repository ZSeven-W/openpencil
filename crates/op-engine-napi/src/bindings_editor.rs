//! Full-editor entry points — the OHOS twin of
//! `op-engine-jni/src/bindings_editor.rs`.
//!
//! Available only when the crate is built with `--features editor` (the same
//! gate the Android layer uses); the viewer lane never links these.

#![cfg(all(target_os = "linux", target_env = "ohos", feature = "editor"))]

use napi_derive_ohos::napi;
use napi_ohos::bindgen_prelude::Buffer;
use op_engine_ffi::{
    op_editor_account_snapshot, op_editor_auth_sign_out, op_editor_begin_login,
    op_editor_cancel_export, op_editor_cancel_login, op_editor_configure_auth,
    op_editor_copy_export_file_name, op_editor_copy_login_url, op_editor_export_to_path,
    op_editor_locale_code, op_editor_open_document, op_editor_set_locale,
    op_editor_take_shell_action, OpStatus, SHELL_ACTION_NONE,
};

use crate::action::{auth_region_or_default, STATUS_CLOSING};
use crate::bindings::{call_status, copy_out_string, with_engine};

// ---- Canvas gestures -----------------------------------------------------

#[napi(js_name = "editorPress")]
pub fn editor_press(engine: i64, x: f64, y: f64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_press(e, x as f32, y as f32)
    })
}

#[napi(js_name = "editorMove")]
pub fn editor_move(engine: i64, x: f64, y: f64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_move(e, x as f32, y as f32)
    })
}

#[napi(js_name = "editorRelease")]
pub fn editor_release(engine: i64, x: f64, y: f64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_release(e, x as f32, y as f32)
    })
}

#[napi(js_name = "editorCancelGesture")]
pub fn editor_cancel_gesture(engine: i64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_cancel_gesture(e)
    })
}

/// `editorBeginTransform` — begin a two-finger transform at the second
/// finger's Down MIDPOINT (not the raw second-finger position).
#[napi(js_name = "editorBeginTransform")]
pub fn editor_begin_transform(engine: i64, x: f64, y: f64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_begin_transform(e, x as f32, y as f32)
    })
}

#[napi(js_name = "editorRightPress")]
pub fn editor_right_press(engine: i64, x: f64, y: f64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_right_press(e, x as f32, y as f32)
    })
}

#[napi(js_name = "editorPan")]
pub fn editor_pan(engine: i64, x: f64, y: f64, dx: f64, dy: f64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_pan(e, x as f32, y as f32, dx as f32, dy as f32)
    })
}

/// `editorPinch` — positive `deltaY` zooms in.
#[napi(js_name = "editorPinch")]
pub fn editor_pinch(engine: i64, x: f64, y: f64, delta_y: f64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_pinch(e, x as f32, y as f32, delta_y as f32)
    })
}

// ---- Keyboard + IME ------------------------------------------------------

#[napi(js_name = "editorText")]
pub fn editor_text(engine: i64, text: String) -> i32 {
    let bytes = text.into_bytes();
    // SAFETY: `bytes` outlives the call.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_text(e, bytes.as_ptr(), bytes.len())
    })
}

/// `editorKey` — a non-printable key as an `OpKey_*` code. Shells holding raw
/// HarmonyOS key codes should call [`crate::bindings_input::key_event`]
/// instead, which does the translation.
#[napi(js_name = "editorKey")]
pub fn editor_key(engine: i64, key: i32) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_key(e, key)
    })
}

/// `editorImePreedit` — selection offsets are BYTE offsets within the preedit
/// text (unlike the viewer-mode `imeSetComposingText`, which is UTF-16).
#[napi(js_name = "editorImePreedit")]
pub fn editor_ime_preedit(engine: i64, text: String, sel_start: i32, sel_end: i32) -> i32 {
    let bytes = text.into_bytes();
    // SAFETY: `bytes` outlives the call.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_ime_preedit(
            e,
            bytes.as_ptr(),
            bytes.len(),
            sel_start.max(0) as usize,
            sel_end.max(0) as usize,
        )
    })
}

#[napi(js_name = "editorImeCommit")]
pub fn editor_ime_commit(engine: i64, text: String) -> i32 {
    let bytes = text.into_bytes();
    // SAFETY: `bytes` outlives the call.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_ime_commit(e, bytes.as_ptr(), bytes.len())
    })
}

/// `editorImeFocused` — whether the editor holds the IME (show/hide the
/// system keyboard).
#[napi(js_name = "editorImeFocused")]
pub fn editor_ime_focused(engine: i64) -> bool {
    with_engine(engine, move |e| {
        let mut focused = false;
        // SAFETY: dispatched onto the engine's owner thread.
        let status = unsafe { op_engine_ffi::op_editor_ime_focused(e, &mut focused) };
        status == OpStatus::Ok && focused
    })
    .unwrap_or(false)
}

// ---- Authentication ------------------------------------------------------

/// `editorConfigureAuth` — initialize the mobile auth backend with a private
/// shell-owned storage directory and the regional SSO deployment.
///
/// `region` is an `OpAuthRegion` (0 = China, 1 = Global); an unknown value
/// falls back to China rather than being forwarded.
#[napi(js_name = "editorConfigureAuth")]
pub fn editor_configure_auth(
    engine: i64,
    storage_dir: String,
    device_name: String,
    app_version: String,
    region: i32,
) -> i32 {
    let storage_dir = storage_dir.into_bytes();
    let device_name = device_name.into_bytes();
    let app_version = app_version.into_bytes();
    let region = auth_region_or_default(region);
    // SAFETY: every borrowed range outlives the call.
    call_status(engine, move |e| unsafe {
        op_editor_configure_auth(
            e,
            storage_dir.as_ptr(),
            storage_dir.len(),
            device_name.as_ptr(),
            device_name.len(),
            app_version.as_ptr(),
            app_version.len(),
            region,
        )
    })
}

/// `editorTakeLoginUrl` — atomically copy AND CONSUME the pending
/// verification URL, or `null` when none is ready.
#[napi(js_name = "editorTakeLoginUrl")]
pub fn editor_take_login_url(engine: i64) -> Option<String> {
    copy_out_string(engine, op_editor_copy_login_url)
}

/// `editorCancelLogin` — the user dismissed the login UI.
#[napi(js_name = "editorCancelLogin")]
pub fn editor_cancel_login(engine: i64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe { op_editor_cancel_login(e) })
}

/// `editorAccountSnapshot` — the JSON account snapshot for the native account
/// center. Re-read per call and never consumed.
#[napi(js_name = "editorAccountSnapshot")]
pub fn editor_account_snapshot(engine: i64) -> Option<String> {
    copy_out_string(engine, op_editor_account_snapshot)
}

/// `editorSignOut` — revoke the device session and clear the account mirror.
#[napi(js_name = "editorSignOut")]
pub fn editor_sign_out(engine: i64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe { op_editor_auth_sign_out(e) })
}

/// `editorBeginLogin` — start the device flow after `editorConfigureAuth`.
/// `OpStatus::NotReady` (10) means the auth backend is the public stub, which
/// is the state every OHOS build is in today — surface it natively rather
/// than opening a login UI.
#[napi(js_name = "editorBeginLogin")]
pub fn editor_begin_login(engine: i64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe { op_editor_begin_login(e) })
}

// ---- Shell actions, documents, export, locale ----------------------------

/// `editorTakeShellAction` — drain the next `OpShellAction`. Engine failures
/// come back negative so they cannot alias a valid action; an unknown/closing
/// handle returns `STATUS_CLOSING`.
#[napi(js_name = "editorTakeShellAction")]
pub fn editor_take_shell_action(engine: i64) -> i32 {
    with_engine(engine, move |e| {
        let mut action = SHELL_ACTION_NONE;
        // SAFETY: dispatched onto the engine's owner thread.
        let status = unsafe { op_editor_take_shell_action(e, &mut action) };
        if status == OpStatus::Ok {
            action
        } else {
            -(status as i32)
        }
    })
    .unwrap_or(STATUS_CLOSING)
}

/// `editorOpenDocument` — install platform-picked `.op` bytes with an
/// optional display name.
#[napi(js_name = "editorOpenDocument")]
pub fn editor_open_document(engine: i64, bytes: Buffer, name: Option<String>) -> i32 {
    open_document_bytes(engine, bytes.to_vec(), name)
}

/// `editorOpenDocumentPath` — the OHOS convenience twin of
/// `editorOpenDocument`: reads an absolute sandbox path on the caller thread
/// (ArkTS file pickers hand back a URI the app resolves to a path) and
/// installs the bytes. Returns `OpStatus::InvalidArg` when the file cannot be
/// read, so a missing file is never mistaken for a bad document.
#[napi(js_name = "editorOpenDocumentPath")]
pub fn editor_open_document_path(engine: i64, path: String) -> i32 {
    let Ok(bytes) = std::fs::read(&path) else {
        return OpStatus::InvalidArg as i32;
    };
    let name = std::path::Path::new(&path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    open_document_bytes(engine, bytes, name)
}

fn open_document_bytes(engine: i64, document: Vec<u8>, name: Option<String>) -> i32 {
    let name = name.map(String::into_bytes);
    call_status(engine, move |e| {
        let (name_ptr, name_len) = name
            .as_ref()
            .map_or((std::ptr::null(), 0), |bytes| (bytes.as_ptr(), bytes.len()));
        // SAFETY: both ranges outlive the call.
        unsafe { op_editor_open_document(e, document.as_ptr(), document.len(), name_ptr, name_len) }
    })
}

/// `editorExportFileName` — the frozen export's file name, or `null` when
/// none is pending. Reading it does NOT consume the artifact; only a
/// successful `editorExportToPath` or an `editorCancelExport` does.
#[napi(js_name = "editorExportFileName")]
pub fn editor_export_file_name(engine: i64) -> Option<String> {
    copy_out_string(engine, op_editor_copy_export_file_name)
}

/// `editorExportToPath` — atomically write the frozen export into a NEW
/// app-private absolute path (the target must not exist). Success consumes
/// the export; failure leaves it retryable.
#[napi(js_name = "editorExportToPath")]
pub fn editor_export_to_path(engine: i64, path: String) -> i32 {
    let path = path.into_bytes();
    // SAFETY: `path` outlives the call.
    call_status(engine, move |e| unsafe {
        op_editor_export_to_path(e, path.as_ptr(), path.len())
    })
}

/// `editorCancelExport` — discard the frozen export when the save UI cannot
/// run (or the user abandoned it).
#[napi(js_name = "editorCancelExport")]
pub fn editor_cancel_export(engine: i64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe { op_editor_cancel_export(e) })
}

/// `editorSetLocale` — apply a UI locale by BCP-47 tag; unsupported tags are
/// rejected with `OpStatus::InvalidArg`.
#[napi(js_name = "editorSetLocale")]
pub fn editor_set_locale(engine: i64, tag: String) -> i32 {
    let tag = tag.into_bytes();
    // SAFETY: `tag` outlives the call.
    call_status(engine, move |e| unsafe {
        op_editor_set_locale(e, tag.as_ptr(), tag.len())
    })
}

/// `editorLocaleCode` — the current UI locale's BCP-47 tag.
#[napi(js_name = "editorLocaleCode")]
pub fn editor_locale_code(engine: i64) -> Option<String> {
    copy_out_string(engine, op_editor_locale_code)
}
