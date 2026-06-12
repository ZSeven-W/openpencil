//! Hidden IME composition target — extracted verbatim from `lib.rs`
//! when the a11y DOM wiring pushed it past the repo's 800-line file
//! cap; behaviour is unchanged.

use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Document, HtmlElement};

/// Codex Phase C stop-hook fix: create a hidden `<textarea>` that
/// owns the IME composition target. Without this, registering
/// composition listeners on `window` would surface IME activity
/// from the browser's URL bar / devtools / any other editable
/// element on the page as if it were directed at our inspector
/// TextInput. The textarea is positioned off-screen + transparent
/// + aria-hidden so it does not interfere with screen readers or
/// visual layout; programmatic focus on it routes the user's IME
/// composition through our owned listeners.
///
/// Appended to `document.body`; the caller owns removal (both the
/// `mount()` unwind path and `Drop for WebShell` call `.remove()`).
pub(crate) fn create_hidden_ime_textarea(document: &Document) -> Result<HtmlElement, JsValue> {
    let ime_textarea = document
        .create_element("textarea")
        .map_err(|e| {
            JsValue::from_str(&format!(
                "mount: could not create hidden IME textarea: {:?}",
                e
            ))
        })?
        .dyn_into::<HtmlElement>()
        .map_err(|_| JsValue::from_str("mount: textarea is not HtmlElement"))?;
    ime_textarea.set_attribute(
        "style",
        "position:fixed;left:-9999px;top:0;width:1px;height:1px;\
         opacity:0;pointer-events:none;",
    )?;
    ime_textarea.set_attribute("aria-hidden", "true")?;
    ime_textarea.set_attribute("tabindex", "-1")?;
    let body = document
        .body()
        .ok_or_else(|| JsValue::from_str("mount: document.body unavailable"))?;
    body.append_child(&ime_textarea)?;
    // Best-effort focus — focus() returns Err if the document is not
    // visible yet (e.g. tab in background), but the listener
    // registration still works once the user gives the page focus.
    let _ = ime_textarea.focus();
    Ok(ime_textarea)
}
