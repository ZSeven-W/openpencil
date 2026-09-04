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
    op_editor_cancel_export, op_editor_cancel_login, op_editor_cancel_save, op_editor_commit_save,
    op_editor_configure_auth, op_editor_configure_save_picker, op_editor_copy_export_file_name,
    op_editor_copy_login_url, op_editor_copy_save_file_name, op_editor_copy_save_target,
    op_editor_export_to_path, op_editor_import_image_or_svg, op_editor_locale_code,
    op_editor_open_document, op_editor_set_locale, op_editor_stage_save_to_path,
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

/// `editorPressAt` — press with the event's factual monotonic timestamp
/// (ArkUI `TouchEvent.timestamp / 1e6`, boot-uptime ms). The JS `number`
/// clamps at zero before the u64 ABI — a negative or NaN clock is never
/// a valid timestamp.
#[napi(js_name = "editorPressAt")]
pub fn editor_press_at(engine: i64, x: f64, y: f64, t_ms: f64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_press_at(e, x as f32, y as f32, clamp_t_ms(t_ms))
    })
}

#[napi(js_name = "editorMove")]
pub fn editor_move(engine: i64, x: f64, y: f64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_move(e, x as f32, y as f32)
    })
}

/// `editorMoveAt` — move with `TouchEvent.timestamp / 1e6`; clamped at
/// zero before u64.
#[napi(js_name = "editorMoveAt")]
pub fn editor_move_at(engine: i64, x: f64, y: f64, t_ms: f64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_move_at(e, x as f32, y as f32, clamp_t_ms(t_ms))
    })
}

#[napi(js_name = "editorRelease")]
pub fn editor_release(engine: i64, x: f64, y: f64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_release(e, x as f32, y as f32)
    })
}

/// `editorReleaseAt` — release with the gesture endpoint's factual
/// timestamp; clamped at zero before u64.
#[napi(js_name = "editorReleaseAt")]
pub fn editor_release_at(engine: i64, x: f64, y: f64, t_ms: f64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_release_at(e, x as f32, y as f32, clamp_t_ms(t_ms))
    })
}

#[napi(js_name = "editorCancelGesture")]
pub fn editor_cancel_gesture(engine: i64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_cancel_gesture(e)
    })
}

/// `editorCancelGestureAt` — cancel with the synthetic-clock timestamp
/// (MonotonicClock.nowMs / TouchEvent.timestamp / 1e6); clamped at zero
/// before u64.
#[napi(js_name = "editorCancelGestureAt")]
pub fn editor_cancel_gesture_at(engine: i64, t_ms: f64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_cancel_gesture_at(e, clamp_t_ms(t_ms))
    })
}

/// JS number → u64 clock with a floor at zero (`f64 as u64` saturates past
/// u64::MAX; negative / NaN shrink to 0, which is never a valid timestamp).
fn clamp_t_ms(t_ms: f64) -> u64 {
    t_ms.max(0.0) as u64
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

/// `editorPasteText` — paste clipboard text into whichever text input owns
/// the keyboard (long-press paste menu). No-op without a focused input.
#[napi(js_name = "editorPasteText")]
pub fn editor_paste_text(engine: i64, text: String) -> i32 {
    let bytes = text.into_bytes();
    // SAFETY: `bytes` outlives the call.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_paste_text(e, bytes.as_ptr(), bytes.len())
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

/// `editorTakeCopyText` — atomically copy AND CONSUME the engine's pending
/// copy-to-clipboard text (collab invite / share address, MCP config, chat
/// copy buttons), or `null` when none is pending. The shell polls this in
/// its pump and writes the system pasteboard.
#[napi(js_name = "editorTakeCopyText")]
pub fn editor_take_copy_text(engine: i64) -> Option<String> {
    copy_out_string(engine, op_engine_ffi::op_editor_take_copy_text)
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

/// `editorImportImageOrSvg` — return one shell-picked PNG, JPEG, GIF, WebP,
/// or SVG plus its display name. ArkTS bounds the buffer to 32 MiB before it
/// crosses NAPI; the FFI repeats that check before touching the document.
#[napi(js_name = "editorImportImageOrSvg")]
pub fn editor_import_image_or_svg(engine: i64, bytes: Buffer, file_name: String) -> i32 {
    let image = bytes.to_vec();
    let file_name = file_name.into_bytes();
    // SAFETY: both owned byte ranges outlive the synchronous FFI call.
    call_status(engine, move |e| unsafe {
        op_editor_import_image_or_svg(
            e,
            image.as_ptr(),
            image.len(),
            file_name.as_ptr(),
            file_name.len(),
        )
    })
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

// ---- Picker-backed Save / Save As ----------------------------------------
//
// The ArkTS side is `DocumentShell.saveDocument`. See
// `op-engine-ffi/src/editor_document_shell.rs` for the round trip.

/// `editorConfigureSavePicker` — declare that this shell routes Save /
/// Save As through `DocumentViewPicker.save`.
#[napi(js_name = "editorConfigureSavePicker")]
pub fn editor_configure_save_picker(engine: i64, enabled: bool) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_editor_configure_save_picker(e, enabled)
    })
}

/// `editorSaveFileName` — the pending save's suggested file name, or `null`
/// when no save is pending. Reading never consumes it.
#[napi(js_name = "editorSaveFileName")]
pub fn editor_save_file_name(engine: i64) -> Option<String> {
    copy_out_string(engine, op_editor_copy_save_file_name)
}

/// `editorSaveTarget` — the durable destination handle a plain Save should
/// rewrite, or `null` when the shell must present the picker.
#[napi(js_name = "editorSaveTarget")]
pub fn editor_save_target(engine: i64) -> Option<String> {
    copy_out_string(engine, op_editor_copy_save_target)
}

/// `editorStageSaveToPath` — write the canonical `.op` bytes into a NEW
/// app-private staging file. Does not mark the document saved.
#[napi(js_name = "editorStageSaveToPath")]
pub fn editor_stage_save_to_path(engine: i64, path: String) -> i32 {
    let path = path.into_bytes();
    // SAFETY: `path` outlives the call.
    call_status(engine, move |e| unsafe {
        op_editor_stage_save_to_path(e, path.as_ptr(), path.len())
    })
}

/// `editorCommitSave` — the staged bytes reached the picked file URI; bind
/// it and mark the document saved.
#[napi(js_name = "editorCommitSave")]
pub fn editor_commit_save(engine: i64, handle: String, display_name: String) -> i32 {
    let handle = handle.into_bytes();
    let display_name = display_name.into_bytes();
    // SAFETY: both ranges outlive the call.
    call_status(engine, move |e| unsafe {
        op_editor_commit_save(
            e,
            handle.as_ptr(),
            handle.len(),
            display_name.as_ptr(),
            display_name.len(),
        )
    })
}

/// `editorCancelSave` — the picker was dismissed, or the copy into the
/// picked file failed. The document stays dirty either way.
#[napi(js_name = "editorCancelSave")]
pub fn editor_cancel_save(engine: i64, failed: bool) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe { op_editor_cancel_save(e, failed) })
}

/// `editorHover` — desktop-chrome cursor motion without a pressed button;
/// drives hover highlighting (mouse/trackpad on 2in1 devices).
#[napi(js_name = "editorHover")]
pub fn editor_hover(engine: i64, x: f64, y: f64) -> i32 {
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_hover(e, x as f32, y as f32)
    })
}

/// `editorWheel` — desktop wheel/trackpad scroll at the cursor: panel-aware
/// (scrolls the panel under the cursor, pans the canvas otherwise); `zoom`
/// promotes the delta to Ctrl+wheel zoom.
#[napi(js_name = "editorWheel")]
pub fn editor_wheel(engine: i64, x: f64, y: f64, dx: f64, dy: f64, zoom: bool) -> i32 {
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_wheel(e, x as f32, y as f32, dx as f32, dy as f32, i32::from(zoom))
    })
}

/// `editorSetTouchChrome` — chrome family: `true` keeps the mobile touch
/// chrome (phone/tablet default), `false` switches the engine to the full
/// desktop layout for desktop-class devices (HarmonyOS PC / 2in1).
#[napi(js_name = "editorSetTouchChrome")]
pub fn editor_set_touch_chrome(engine: i64, enabled: bool) -> i32 {
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_set_touch_chrome(e, i32::from(enabled))
    })
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
