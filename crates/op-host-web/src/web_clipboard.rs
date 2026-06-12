// UNVERIFIED: needs EMSDK wasm32 build + browser; run tools/check-wasm-bundle.sh
//! Browser clipboard write + file download (Blob + anchor).
//!
//! The two browser-side actions the codegen panel needs: copy generated source
//! to the clipboard, and download an exported artifact. Pure `web_sys`/`js_sys`
//! IO against the locked web-sys 0.3.94 bindings — verified by inspection (the
//! exact signatures: `Navigator::clipboard() -> Clipboard`,
//! `Clipboard::write_text() -> js_sys::Promise`, `BlobPropertyBag::new() ->
//! Self` + `set_type(&self)`, `Url::create_object_url_with_blob -> Result`),
//! mirroring the Blob/Url download idiom already in `vendor/casement`.
#![allow(dead_code)]

use wasm_bindgen::JsCast;

/// Copy `text` to the system clipboard (fire-and-forget).
///
/// `Clipboard::write_text` returns a `Promise`; we deliberately don't await it
/// (no `wasm-bindgen-futures` in this crate) — the copy either lands or the
/// browser rejects it, and there's no UI surface here to report a rejection.
pub fn copy_text(text: &str) {
    if let Some(win) = web_sys::window() {
        let clipboard = win.navigator().clipboard();
        // Drop the returned Promise; we don't poll it.
        let _ = clipboard.write_text(text);
    }
}

/// Trigger a browser download of `data` as `filename` with MIME type `mime`.
///
/// Builds a `Blob` from the bytes, creates an object URL, clicks a synthetic
/// `<a download>` anchor, then immediately revokes the URL (the click has
/// already kicked off the download by the time `revoke` runs). Mirrors the
/// `vendor/casement` Blob/Url idiom: `BlobPropertyBag::new()` then
/// `set_type(...)`, a single-element parts `Array::of1`.
pub fn download_bytes(
    filename: &str,
    mime: &str,
    data: &[u8],
) -> Result<(), wasm_bindgen::JsValue> {
    // Wrap the bytes in a `Uint8Array` and hand its backing `ArrayBuffer` to
    // the Blob as a one-element parts sequence.
    let arr = js_sys::Uint8Array::from(data);
    let parts = js_sys::Array::of1(&arr.buffer().into());

    let bag = web_sys::BlobPropertyBag::new();
    bag.set_type(mime);
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &bag)?;

    let url = web_sys::Url::create_object_url_with_blob(&blob)?;

    let window = web_sys::window()
        .ok_or_else(|| wasm_bindgen::JsValue::from_str("download: window unavailable"))?;
    let document = window
        .document()
        .ok_or_else(|| wasm_bindgen::JsValue::from_str("download: document unavailable"))?;
    let anchor = document
        .create_element("a")?
        .dyn_into::<web_sys::HtmlAnchorElement>()?;
    anchor.set_href(&url);
    anchor.set_download(filename);
    // `click()` (from HtmlElement) dispatches the download synchronously.
    anchor.click();

    // The browser has captured the Blob for the download; revoke to free it.
    web_sys::Url::revoke_object_url(&url)?;
    Ok(())
}

/// Trigger a browser download of an already-encoded `data:` URL as
/// `filename`. Same synthetic `<a download>` idiom as
/// [`download_bytes`], minus the Blob/object-URL round-trip — used by
/// the Export-image flow, whose payload comes straight from
/// `HtmlCanvasElement::to_data_url`.
pub fn download_data_url(filename: &str, url: &str) -> Result<(), wasm_bindgen::JsValue> {
    let window = web_sys::window()
        .ok_or_else(|| wasm_bindgen::JsValue::from_str("download: window unavailable"))?;
    let document = window
        .document()
        .ok_or_else(|| wasm_bindgen::JsValue::from_str("download: document unavailable"))?;
    let anchor = document
        .create_element("a")?
        .dyn_into::<web_sys::HtmlAnchorElement>()?;
    anchor.set_href(url);
    anchor.set_download(filename);
    anchor.click();
    Ok(())
}
