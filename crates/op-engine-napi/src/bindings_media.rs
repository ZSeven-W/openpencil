//! Remote image / font / page entry points — the OHOS twin of
//! `op-engine-jni/src/bindings_media.rs`.

#![cfg(all(target_os = "linux", target_env = "ohos"))]

use napi_derive_ohos::napi;
use napi_ohos::bindgen_prelude::Buffer;
use op_engine_ffi::OpStatus;

use crate::action::STATUS_CLOSING;
use crate::bindings::{call_status, with_engine};

/// `remoteImageResult` — push fetched bytes back into the engine. Pass
/// `null`/empty to report a failed fetch, which lets the engine stop waiting.
#[napi(js_name = "remoteImageResult")]
pub fn remote_image_result(engine: i64, request_id: i64, bytes: Option<Buffer>) -> i32 {
    let bytes: Vec<u8> = bytes.map(|buffer| buffer.to_vec()).unwrap_or_default();
    // SAFETY: `bytes` outlives the call; dispatched onto the owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_remote_image_result(e, request_id as u64, bytes.as_ptr(), bytes.len())
    })
}

/// `registerFont` — register an imported TTF/OTF and re-layout the document.
#[napi(js_name = "registerFont")]
pub fn register_font(engine: i64, bytes: Buffer) -> i32 {
    let bytes = bytes.to_vec();
    // SAFETY: `bytes` outlives the call.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_register_font(e, bytes.as_ptr(), bytes.len())
    })
}

/// `getPageCount` — the document's page count, an `OpStatus` on engine
/// failure, or `STATUS_CLOSING` for a dead handle.
#[napi(js_name = "getPageCount")]
pub fn get_page_count(engine: i64) -> i32 {
    with_engine(engine, move |e| {
        let mut count = 0_u32;
        // SAFETY: dispatched onto the engine's owner thread.
        let status = unsafe { op_engine_ffi::op_get_page_count(e, &mut count) };
        if status == OpStatus::Ok {
            count as i32
        } else {
            status as i32
        }
    })
    .unwrap_or(STATUS_CLOSING)
}

/// `setActivePage` — switch to a 0-based page index; the viewport re-fits.
#[napi(js_name = "setActivePage")]
pub fn set_active_page(engine: i64, index: i32) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_set_active_page(e, index as u32)
    })
}
