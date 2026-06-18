//! Hidden IME-capture input for the CanvasKit web shell (#54).
//!
//! The editor renders to a `<canvas>`, which is not an editable element, so a
//! browser IME (CJK / etc.) has nowhere to compose — composition events never
//! fire and the keydown handler's `is_composing()` guard drops the keystrokes,
//! leaving CJK input unusable in the browser.
//!
//! This module adds the missing piece: a hidden, focusable `<input>` that the
//! IME composes into. Its `compositionend` is wired (in `canvaskit.rs`) to
//! `WidgetHost::apply_ime`, which routes the committed string through
//! `apply_text` into whichever editor field owns the keyboard (canvas text
//! editor, chat input, property inputs …) — exactly the native host's
//! `Ime::Commit` behaviour. Preedit is intentionally not painted (matches
//! native), so only the final commit is consumed.
//!
//! Focus is gated on [`WidgetHost::input_active`](crate::widget_host): the
//! input is focused only while a text field is active, so it never pops a
//! mobile soft keyboard (or steals focus) when the user isn't editing text.
//!
//! The candidate-window position (anchoring it to the caret) is the separate
//! `#49` polish; v1 fixes the input at the top-left corner — the committed text
//! is correct regardless of where the candidate window paints.
//!
//! Compiled under both the `web` stub and the production `canvaskit` build;
//! only the latter wires it, so its surface reads as dead code under `web`.
#![cfg_attr(not(feature = "canvaskit"), allow(dead_code))]

use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

/// The id of the hidden IME input appended next to the canvas.
const INPUT_ID: &str = "op-ime-input";

/// Off-screen-but-focusable style. NOT `display:none` / `visibility:hidden`
/// (both block focus + IME composition); uses `opacity:0` + `pointer-events:
/// none` so it can't be seen or clicked but still accepts programmatic focus
/// and IME.
const HIDDEN_STYLE: &str = "position:fixed;left:0;top:0;width:1px;height:1px;\
opacity:0;border:0;padding:0;margin:0;pointer-events:none;z-index:-1;";

/// Owns the hidden IME `<input>` and tracks its focus state so we only call
/// `focus()` / `blur()` on a transition (no per-frame focus churn).
pub(crate) struct ImeInput {
    input: HtmlInputElement,
    focused: bool,
}

impl ImeInput {
    /// Create the hidden input and append it as a sibling of `canvas` (or, if
    /// that fails, to `<body>`). Returns `None` only if the DOM is unreachable.
    pub(crate) fn create(canvas: &web_sys::HtmlCanvasElement) -> Option<Self> {
        let document = canvas
            .owner_document()
            .or_else(|| web_sys::window().and_then(|w| w.document()))?;

        // Reuse an existing input across re-mounts so we don't leak a second.
        let input: HtmlInputElement = match document.get_element_by_id(INPUT_ID) {
            Some(el) => el.dyn_into::<HtmlInputElement>().ok()?,
            None => {
                let el = document.create_element("input").ok()?;
                let el: HtmlInputElement = el.dyn_into::<HtmlInputElement>().ok()?;
                el.set_id(INPUT_ID);
                el.set_type("text");
                let _ = el.set_attribute("style", HIDDEN_STYLE);
                let _ = el.set_attribute("aria-hidden", "true");
                let _ = el.set_attribute("tabindex", "-1");
                // Keep the browser from autofilling / autocorrecting the
                // throwaway composition buffer.
                let _ = el.set_attribute("autocomplete", "off");
                let _ = el.set_attribute("autocorrect", "off");
                let _ = el.set_attribute("autocapitalize", "off");
                let _ = el.set_attribute("spellcheck", "false");
                if let Some(parent) = canvas.parent_node() {
                    let _ = parent.append_child(&el);
                } else if let Some(body) = document.body() {
                    let _ = body.append_child(&el);
                }
                el
            }
        };

        Some(Self {
            input,
            focused: false,
        })
    }

    /// The input element — `canvaskit.rs` attaches the composition listeners
    /// to it.
    pub(crate) fn input(&self) -> &HtmlInputElement {
        &self.input
    }

    /// Drive focus from the host's text-input-active state. Focuses the input
    /// (so the IME composes into it) only while a field is active; blurs it
    /// otherwise so no soft keyboard appears when not editing. Only toggles on
    /// a transition.
    pub(crate) fn sync_focus(&mut self, want: bool) {
        if want == self.focused {
            return;
        }
        self.focused = want;
        if want {
            let _ = self.input.focus();
        } else {
            let _ = self.input.blur();
        }
    }

    /// Empty the throwaway composition buffer (called on composition
    /// start/end) so it never accumulates committed text — the commit is read
    /// from the event's `data`, never the input value.
    pub(crate) fn clear(&self) {
        self.input.set_value("");
    }
}
