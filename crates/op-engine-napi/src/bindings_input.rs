//! Mouse, hardware-keyboard, and clipboard entry points.
//!
//! These have no Android counterpart — the Kotlin player is touch-only — but
//! HarmonyOS runs the same editor on 2-in-1 and PC form factors, where ArkUI
//! delivers `onMouse` / `onKeyEvent` instead of touches. Each function is a
//! thin translation onto the editor gestures the engine already exposes, so
//! the OHOS shell never has to reimplement the desktop input rules.

#![cfg(all(target_os = "linux", target_env = "ohos", feature = "editor"))]

use napi_derive_ohos::napi;
use op_engine_ffi::OpStatus;

use crate::action::{map_key, mouse_intent, wheel_intent, MouseIntent, WheelIntent};
use crate::bindings_editor::{
    editor_begin_transform, editor_cancel_gesture, editor_key, editor_move, editor_pan,
    editor_pinch, editor_press, editor_release, editor_right_press, editor_text,
};

/// `mouseMove` — pointer motion (hover or drag); the same channel a finger
/// move uses.
#[napi(js_name = "mouseMove")]
pub fn mouse_move(engine: i64, x: f64, y: f64) -> i32 {
    editor_move(engine, x, y)
}

/// `mouseButton` — a button transition from ArkUI's `MouseButton` ordinal
/// (0 = left, 1 = right, 2 = middle).
///
/// Left maps to the ordinary press/release pair. Right maps to the
/// context-menu press on the DOWN edge only — the engine has no
/// right-release entry point — and its release is accepted as a no-op.
/// Middle and the extra buttons have no editor binding and are swallowed.
/// All three "nothing to do" cases return `OpStatus::Ok` so the shell does
/// not treat a deliberately ignored button as a failure.
#[napi(js_name = "mouseButton")]
pub fn mouse_button(engine: i64, x: f64, y: f64, button: i32, pressed: bool) -> i32 {
    match (mouse_intent(button), pressed) {
        (MouseIntent::Primary, true) => editor_press(engine, x, y),
        (MouseIntent::Primary, false) => editor_release(engine, x, y),
        (MouseIntent::Context, true) => editor_right_press(engine, x, y),
        (MouseIntent::Context, false) | (MouseIntent::Ignored, _) => OpStatus::Ok as i32,
    }
}

/// `mouseWheel` — one wheel / trackpad scroll. `zoom` is the Ctrl/Cmd state:
/// with it, `dy` zooms about the cursor (positive zooms in); without it,
/// `dx`/`dy` pan the canvas.
///
/// The engine's pan/pinch are two-finger gestures that only apply while a
/// transform is captured, so each wheel event is bracketed with
/// begin-transform → gesture → cancel-gesture. That bracket also resets any
/// pointer capture, so a wheel event delivered in the middle of a mouse drag
/// ends that drag — acceptable on the form factors that have a wheel, and
/// preferable to a silently ignored scroll.
#[napi(js_name = "mouseWheel")]
pub fn mouse_wheel(engine: i64, x: f64, y: f64, dx: f64, dy: f64, zoom: bool) -> i32 {
    let intent = wheel_intent(dx as f32, dy as f32, zoom);
    if matches!(intent, WheelIntent::None) {
        return OpStatus::Ok as i32;
    }
    let began = editor_begin_transform(engine, x, y);
    if began != OpStatus::Ok as i32 {
        return began;
    }
    let status = match intent {
        WheelIntent::Pan { dx, dy } => editor_pan(engine, x, y, dx as f64, dy as f64),
        WheelIntent::Zoom { delta } => editor_pinch(engine, x, y, delta as f64),
        WheelIntent::None => OpStatus::Ok as i32,
    };
    let ended = editor_cancel_gesture(engine);
    if status != OpStatus::Ok as i32 {
        status
    } else {
        ended
    }
}

/// `keyEvent` — a hardware key press as a raw HarmonyOS key code plus this
/// crate's modifier bitmask (1 = shift, 2 = ctrl, 4 = alt, 8 = meta).
///
/// Keys with an editor binding (backspace, delete, enter, escape, arrows,
/// and the Cmd/Ctrl duplicate / undo / redo shortcuts) are forwarded as
/// `OpKey_*` codes. Anything else returns `OpStatus::InvalidArg`, which is
/// the shell's signal to route the key through the IME as text instead.
#[napi(js_name = "keyEvent")]
pub fn key_event(engine: i64, key_code: i32, modifiers: i32) -> i32 {
    match map_key(key_code, modifiers) {
        Some(key) => editor_key(engine, key),
        None => OpStatus::InvalidArg as i32,
    }
}

/// `keyModifiers` — the live hardware-modifier bitmask (1 = shift,
/// 2 = ctrl, 4 = alt, 8 = meta), maintained from the native key stream.
/// The shell reads it where ArkUI cannot see keys (wheel-zoom's Ctrl).
#[napi(js_name = "keyModifiers")]
pub fn key_modifiers() -> i32 {
    crate::xcomponent::current_modifiers()
}

/// `setImeConduitAttached` — the shell reports whether its `ImeConduit`
/// currently holds a system `InputMethodController`.
///
/// While it does, the IME owns printable characters and the native key
/// channel must not inject them as well (that types every character twice).
/// While it does not — the desktop-class (2in1) case, where there is no soft
/// keyboard and the SURFACE XComponent eats hardware keys before the IME
/// framework can see them — the native channel is the ONLY route text can
/// take into a focused engine input.
#[napi(js_name = "setImeConduitAttached")]
pub fn set_ime_conduit_attached(attached: bool) {
    crate::xcomponent::set_ime_conduit_attached(attached);
}

/// `clipboardSetText` — deliver text the shell read from
/// `@ohos.pasteboard` into the focused editor input (i.e. paste).
///
/// The pasteboard itself stays ArkTS-owned: OHOS requires the paste to be
/// initiated from the UI thread with the app's own permissions, so the shell
/// reads the system clipboard and hands the text here.
#[napi(js_name = "clipboardSetText")]
pub fn clipboard_set_text(engine: i64, text: String) -> i32 {
    if text.is_empty() {
        return OpStatus::Ok as i32;
    }
    editor_text(engine, text)
}

/// `clipboardGetText` — RESERVED. Always returns `null`.
///
/// Copy is not wired: `op_engine.h` has no channel for the engine to hand a
/// selection back to the shell (the Android player has the same gap), so
/// there is nothing to return. The entry point exists so the ArkTS shell can
/// bind its copy handler once, and start working the day the C ABI grows a
/// copy accessor. Do NOT branch on it — treat `null` as "copy unsupported".
#[napi(js_name = "clipboardGetText")]
pub fn clipboard_get_text(engine: i64) -> Option<String> {
    let _ = engine;
    None
}
