//! Text + IME entry points — the OHOS twin of
//! `op-engine-jni/src/bindings_text.rs`.
//!
//! Offsets are UTF-16 code units, matching both the C ABI and the ArkTS
//! `inputMethod` surrounding-text contract.

#![cfg(all(target_os = "linux", target_env = "ohos"))]

use napi_derive_ohos::napi;
use op_engine_ffi::OpStatus;

use crate::action::STATUS_CLOSING;
use crate::bindings::{call_status, with_engine};

/// Owned surrounding-text snapshot. `status` is the `OpStatus` integer (0 =
/// Ok); every other field is meaningful only when it is 0.
#[napi(object)]
pub struct TextState {
    pub status: i32,
    pub text: String,
    pub selection_start: i32,
    pub selection_end: i32,
    pub has_composing: bool,
    pub composing_start: i32,
    pub composing_end: i32,
}

impl TextState {
    fn failed(status: i32) -> Self {
        Self {
            status,
            text: String::new(),
            selection_start: 0,
            selection_end: 0,
            has_composing: false,
            composing_start: 0,
            composing_end: 0,
        }
    }
}

#[napi(js_name = "textBegin")]
pub fn text_begin(engine: i64, node_id: String) -> i32 {
    let bytes = node_id.into_bytes();
    // SAFETY: `bytes` outlives the call; dispatched onto the owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_text_begin(e, bytes.as_ptr(), bytes.len())
    })
}

#[napi(js_name = "textEnd")]
pub fn text_end(engine: i64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe { op_engine_ffi::op_text_end(e) })
}

#[napi(js_name = "textInsert")]
pub fn text_insert(engine: i64, text: String) -> i32 {
    let bytes = text.into_bytes();
    // SAFETY: `bytes` outlives the call.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_text_insert(e, bytes.as_ptr(), bytes.len())
    })
}

#[napi(js_name = "textBackspace")]
pub fn text_backspace(engine: i64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_text_backspace(e)
    })
}

#[napi(js_name = "textDeleteForward")]
pub fn text_delete_forward(engine: i64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_text_delete_forward(e)
    })
}

#[napi(js_name = "textSetCaret")]
pub fn text_set_caret(engine: i64, offset: i32, extend: bool) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_text_set_caret(e, offset as u32, extend)
    })
}

#[napi(js_name = "textSelectRange")]
pub fn text_select_range(engine: i64, anchor: i32, focus: i32) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_text_select_range(e, anchor as u32, focus as u32)
    })
}

#[napi(js_name = "imeSetComposingText")]
pub fn ime_set_composing_text(engine: i64, text: String, sel_start: i32, sel_end: i32) -> i32 {
    let bytes = text.into_bytes();
    // SAFETY: `bytes` outlives the call.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_ime_set_composing_text(
            e,
            bytes.as_ptr(),
            bytes.len(),
            sel_start as u32,
            sel_end as u32,
        )
    })
}

#[napi(js_name = "imeCommitComposition")]
pub fn ime_commit_composition(engine: i64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_ime_commit_composition(e)
    })
}

#[napi(js_name = "imeCancelComposition")]
pub fn ime_cancel_composition(engine: i64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_ime_cancel_composition(e)
    })
}

/// `textGetState` — surrounding-text snapshot for the shell's input method.
/// The C `text_ptr` is only valid until the next engine call, so it is copied
/// out ON the engine thread.
#[napi(js_name = "textGetState")]
pub fn text_get_state(engine: i64) -> TextState {
    with_engine(engine, move |e| {
        let mut state = op_engine_ffi::OpTextState {
            text_ptr: std::ptr::null(),
            text_len: 0,
            selection_start: 0,
            selection_end: 0,
            has_composing: false,
            composing_start: 0,
            composing_end: 0,
        };
        // SAFETY: dispatched onto the engine's owner thread.
        let status = unsafe { op_engine_ffi::op_text_get_state(e, &mut state) };
        if status != OpStatus::Ok {
            return TextState::failed(status as i32);
        }
        let text = if state.text_ptr.is_null() || state.text_len == 0 {
            String::new()
        } else {
            // SAFETY: the engine guarantees the range until the next call,
            // and this copy happens before any other engine call.
            unsafe {
                String::from_utf8_lossy(std::slice::from_raw_parts(state.text_ptr, state.text_len))
                    .into_owned()
            }
        };
        TextState {
            status: 0,
            text,
            selection_start: state.selection_start as i32,
            selection_end: state.selection_end as i32,
            has_composing: state.has_composing,
            composing_start: state.composing_start as i32,
            composing_end: state.composing_end as i32,
        }
    })
    .unwrap_or_else(|| TextState::failed(STATUS_CLOSING))
}

/// `textCaretRect` — the caret rect in surface-logical points as
/// `[x, y, w, h]`, or an empty array when it cannot be read.
#[napi(js_name = "textCaretRect")]
pub fn text_caret_rect(engine: i64) -> Vec<f64> {
    with_engine(engine, move |e| {
        let mut rect = [0.0_f32; 4];
        // SAFETY: `rect` has the four floats the ABI writes.
        let status = unsafe { op_engine_ffi::op_text_caret_rect(e, rect.as_mut_ptr()) };
        if status == OpStatus::Ok {
            rect.iter().map(|value| *value as f64).collect()
        } else {
            Vec::new()
        }
    })
    .unwrap_or_default()
}
