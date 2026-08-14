//! Remote image fetching + imported font registration.
//!
//! Remote images: the shared painter records `http(s)` misses into a
//! bounded queue; the engine drains it after every frame and hands each
//! `(request_id, url)` to the shell via the `remote_image_request`
//! upcall. The shell fetches on its own network path and pushes the bytes
//! back through [`op_remote_image_result`] — the next paint finds them in
//! the cache and draws the bitmap instead of the placeholder. Empty bytes
//! mark the fetch failed (the placeholder stays; the in-flight mark is
//! cleared so a later cache eviction may retry).
//!
//! Imported fonts: [`op_register_font`] feeds raw TTF/OTF bytes into
//! jian-skia's imported-font registry and rebuilds the layout scene, so
//! documents referencing the family re-measure and re-render with the
//! real glyphs (the same path the desktop's `FontStore` uses).

use crate::error::{FfiError, FfiResult};
use crate::lifecycle::{call_session, Session};
use crate::OpStatus;

/// Hard cap on remote image payloads (64 MiB) and font files (64 MiB).
const MEDIA_BYTE_CAP: usize = 64 * 1024 * 1024;

/// Drain pending remote-image misses recorded by the last paint and fire
/// the `remote_image_request` upcall for each. Called after every frame.
pub(crate) fn drain_remote_image_requests(session: &Session) {
    let requests =
        op_editor_ui::widgets::canvas_viewport_image::take_remote_image_requests(usize::MAX);
    if requests.is_empty() {
        return;
    }
    let Some(callback) = session.callbacks.remote_image_request else {
        return;
    };
    for (request_id, url) in requests {
        // SAFETY: `url` is live for the call; the shell must copy it
        // before returning (callbacks never re-enter the engine).
        unsafe {
            callback(
                session.callbacks.user_data,
                request_id,
                url.as_ptr(),
                url.len(),
            )
        };
    }
}

/// Push fetched remote-image bytes back into the engine (empty bytes =
/// fetch failed).
///
/// # Safety
///
/// `engine` must be live and `bytes` must cover `bytes_len` readable
/// bytes.
#[no_mangle]
pub unsafe extern "C" fn op_remote_image_result(
    engine: *mut crate::OpEngine,
    request_id: u64,
    bytes_ptr: *const u8,
    bytes_len: usize,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let bytes = read_media_bytes(bytes_ptr, bytes_len, "remote image bytes")?;
            op_editor_ui::widgets::canvas_viewport_image::store_remote_image_bytes(
                request_id, bytes,
            );
            session.request_redraw();
            Ok(())
        })
    }
}

/// Register an imported TTF/OTF font (from the app bundle/assets) with
/// the engine's font registry and re-layout the document, so text set in
/// that family measures and renders with the real face.
///
/// # Safety
///
/// `engine` must be live and `bytes` must cover `bytes_len` readable
/// bytes.
#[no_mangle]
pub unsafe extern "C" fn op_register_font(
    engine: *mut crate::OpEngine,
    bytes_ptr: *const u8,
    bytes_len: usize,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let bytes = read_media_bytes(bytes_ptr, bytes_len, "font bytes")?;
            jian_skia::register_imported_font(bytes)
                .map_err(|e| FfiError::new(OpStatus::InvalidArg, e))?;
            // Font metrics changed: the layout scene must re-measure.
            session.rebuild_scene();
            session.request_redraw();
            Ok(())
        })
    }
}

/// Copy caller-owned media bytes with the payload cap applied.
///
/// # Safety
///
/// `pointer` must cover `length` readable bytes when non-null.
unsafe fn read_media_bytes(pointer: *const u8, length: usize, label: &str) -> FfiResult<Vec<u8>> {
    if length > MEDIA_BYTE_CAP {
        return Err(FfiError::invalid(format!(
            "{label} length exceeds {MEDIA_BYTE_CAP} bytes"
        )));
    }
    if length == 0 {
        return Ok(Vec::new());
    }
    if pointer.is_null() {
        return Err(FfiError::invalid(format!(
            "{label} pointer is null with nonzero length"
        )));
    }
    Ok(unsafe { std::slice::from_raw_parts(pointer, length) }.to_vec())
}
