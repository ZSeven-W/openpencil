// Browser boundary: clipboard writes, external navigation, and download clicks
// require host/browser integration; the CanvasKit bundle gate covers wasm
// linkability.
//! Browser clipboard, VS Code host relay, and file download (Blob + anchor).
//!
//! The browser-side actions the editor needs: copy generated source, relay a
//! narrow host action from an embed, and download an artifact. Pure `web_sys`/`js_sys`
//! IO against the locked web-sys 0.3.94 bindings — verified by inspection (the
//! exact signatures: `Navigator::clipboard() -> Clipboard`,
//! `Clipboard::write_text() -> js_sys::Promise`, `BlobPropertyBag::new() ->
//! Self` + `set_type(&self)`, `Url::create_object_url_with_blob -> Result`),
//! mirroring the Blob/Url download idiom already in `vendor/casement`.
#![allow(dead_code)]

use wasm_bindgen::JsCast;

fn is_vscode_embed_state(search: &str, in_iframe: bool) -> bool {
    in_iframe && op_editor_core::EmbedHost::from_query(search) == op_editor_core::EmbedHost::VsCode
}

/// Whether this canvas is running inside the VS Code/Cursor relay shell.
pub fn is_vscode_embed() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let Ok(search) = window.location().search() else {
        return false;
    };
    is_vscode_embed_state(&search, crate::vscode_bridge::in_iframe(&window))
}

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

/// Relay `text` to the embedding shell for a host-side clipboard write.
///
/// Inside the VS Code webview the nested-iframe permissions chain rejects
/// `navigator.clipboard` writes, so the embed posts an `op-shell/copy`
/// control message to the parent (the extension's relay shell), which
/// forwards it to the extension host for `vscode.env.clipboard.writeText`.
/// Target origin is `"*"`: the parent is the relay shell by construction,
/// and the payload is content the user explicitly asked to copy.
/// Relay a Cmd/Ctrl+S inside the embed to the extension host: the
/// workbench can't observe keystrokes inside the cross-origin editor
/// iframe, so the page forwards the intent and the extension runs the
/// regular VS Code save (which lands in `saveCustomDocument`).
pub fn post_save_to_parent() {
    let Some(win) = web_sys::window() else { return };
    let Ok(Some(parent)) = win.parent() else {
        return;
    };
    let msg = r#"{"type":"op-shell/save"}"#;
    let _ = parent.post_message(&wasm_bindgen::JsValue::from_str(msg), "*");
}

pub fn post_copy_to_parent(text: &str) {
    let Some(win) = web_sys::window() else { return };
    let Ok(Some(parent)) = win.parent() else {
        return;
    };
    let msg = serde_json::json!({ "type": "op-shell/copy", "text": text }).to_string();
    let _ = parent.post_message(&wasm_bindgen::JsValue::from_str(&msg), "*");
}

/// Ask a VS Code embedding shell to open an external URL in the user's browser.
///
/// `true` means the VS Code embed claimed the navigation, not that a message
/// was already posted. The URL is held while the bridge proves that the
/// token-authenticated daemon is running in `managed` mode; failure is a
/// conservative drop so the auth callback cannot fall back to a nested
/// `window.open` and bypass the proof.
pub fn post_open_external_to_parent(url: &str) -> bool {
    if !is_vscode_embed() {
        return false;
    }
    crate::vscode_bridge::request_external_navigation(url);
    true
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

#[cfg(test)]
mod tests {
    use super::is_vscode_embed_state;

    #[test]
    fn external_navigation_relay_is_scoped_to_the_vscode_embed() {
        assert!(is_vscode_embed_state("?embed=vscode", true));
        assert!(is_vscode_embed_state("?foo=1&embed=vscode", true));
        assert!(!is_vscode_embed_state("", true));
        assert!(!is_vscode_embed_state("?embed=web", true));
        assert!(!is_vscode_embed_state("?embedded=vscode", true));
    }

    #[test]
    fn top_level_query_cannot_spoof_a_vscode_embed() {
        assert!(!is_vscode_embed_state("?embed=vscode", false));
    }
}
