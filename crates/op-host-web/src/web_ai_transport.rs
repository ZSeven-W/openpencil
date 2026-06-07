// UNVERIFIED: needs EMSDK wasm32 build + browser; run tools/check-wasm-bundle.sh
//! XHR-based SSE transport to the desktop daemon's `/api/ai/stream` proxy.
//!
//! Mirrors `live_sync.rs` (no `wasm-bindgen-futures`): an `XmlHttpRequest`
//! with an `onprogress` closure that parses newly-arrived `data: {...}`
//! events. The browser populates `responseText` incrementally as the SSE body
//! streams in; the `onprogress` closure tracks a consumed-length cursor so each
//! `data:` block is delivered exactly once.
//!
//! Pure `web_sys`/`js_sys` IO (no skia, no `op_editor_core`) so the wire layer
//! is correct-by-construction against the locked web-sys 0.3.94 bindings; the
//! browser round-trip itself still needs a running daemon + browser to verify.
#![allow(dead_code)]

use std::cell::Cell;
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

/// One streamed event from the AI proxy.
pub enum AiEvent {
    /// A token / text fragment to append to the in-flight assistant turn.
    Delta(String),
    /// The stream finished successfully.
    Done,
    /// The proxy reported an error for this turn.
    Error(String),
}

/// POST `body_json` to `{base}/api/ai/stream` and invoke `on_event` for each
/// streamed SSE event.
///
/// `base` is the daemon origin, e.g. `http://127.0.0.1:3100`. `on_event` is
/// `Rc<dyn Fn(AiEvent)>` so it can be shared across each `onprogress` tick (the
/// same shared-callback pattern `live_sync::start` uses for `on_response`).
///
/// The `onprogress` closure tracks a consumed-length cursor: each tick parses
/// only fully-terminated `data:` blocks (those before the last blank-line
/// separator) and advances the cursor past that separator, so a partial tail is
/// re-parsed on the next tick and no block is delivered twice. The closure is
/// `forget()`-leaked (page-scoped like `live_sync::start`); the XHR owns it for
/// the duration of the request.
pub fn post_ai_stream(
    base: &str,
    body_json: String,
    on_event: Rc<dyn Fn(AiEvent)>,
) -> Result<(), wasm_bindgen::JsValue> {
    let xhr = web_sys::XmlHttpRequest::new()?;
    xhr.open_with_async("POST", &format!("{base}/api/ai/stream"), true)?;
    xhr.set_request_header("Content-Type", "application/json")?;

    // `cursor` is the byte offset in `responseText` up to which we've already
    // delivered complete SSE blocks. Shared (Rc<Cell>) between the closure body
    // and re-entrant `onprogress` firings.
    let cursor = Rc::new(Cell::new(0usize));
    let xhr_for_cb = xhr.clone();
    let on_event_cb = on_event.clone();
    let cursor_cb = cursor.clone();

    let onprogress = Closure::<dyn FnMut()>::new(move || {
        let Ok(Some(text)) = xhr_for_cb.response_text() else {
            return;
        };
        let start = cursor_cb.get();
        // Guard against a shrinking/replaced body (shouldn't happen mid-stream,
        // but `start` past the end would panic the slice below).
        if start > text.len() {
            cursor_cb.set(text.len());
            return;
        }
        if text.len() <= start {
            return; // nothing new since the last tick
        }

        // Only the prefix up to the LAST blank-line separator holds complete
        // SSE blocks; everything after it is a partial tail to retry next tick.
        // `rfind` over the whole `text` (not just the fresh slice) so a `\n\n`
        // that straddles the previous cursor boundary is still found.
        let Some(last_sep) = text.rfind("\n\n") else {
            return; // no complete block yet
        };
        let complete_end = last_sep + 2; // include the separator
        if complete_end <= start {
            return; // already consumed past the last complete block
        }

        // Parse each complete block in the freshly-completed region.
        let fresh = &text[start..complete_end];
        for block in fresh.split("\n\n") {
            // SSE field lines: we only care about `data:` lines. A block may in
            // principle carry multiple lines (event:, id:, data:); forward each
            // `data:` payload we find.
            for line in block.lines() {
                let line = line.trim_start();
                if let Some(rest) = line.strip_prefix("data:") {
                    if let Some(evt) = parse_event(rest.trim()) {
                        on_event_cb(evt);
                    }
                }
            }
        }

        cursor_cb.set(complete_end);
    });

    xhr.set_onprogress(Some(onprogress.as_ref().unchecked_ref()));
    onprogress.forget(); // lives until the XHR completes (page-scoped like live_sync)
    xhr.send_with_opt_str(Some(&body_json))?;
    Ok(())
}

/// Parse one SSE `data:` payload (already trimmed of the `data:` prefix) into an
/// [`AiEvent`].
///
/// Uses `js_sys::JSON::parse` + `Reflect::get` rather than a hand-rolled string
/// parser: `js-sys` is already a dependency, and the browser's JSON parser
/// handles all escaping (`\"`, `\\`, `\n`, `\uXXXX`, …) correctly. A `"done"`
/// signal may arrive either as a bare SSE sentinel (`[DONE]`) or as a JSON
/// object with a `done` field; both map to [`AiEvent::Done`]. An `error` field
/// takes precedence over `delta` so a turn that fails mid-stream surfaces the
/// error rather than a partial delta.
fn parse_event(payload: &str) -> Option<AiEvent> {
    // OpenAI-style sentinel terminator.
    if payload == "[DONE]" {
        return Some(AiEvent::Done);
    }

    let value = js_sys::JSON::parse(payload).ok()?;

    // `error` first — a failed turn must not be reported as a delta.
    if let Some(msg) = string_field(&value, "error") {
        return Some(AiEvent::Error(msg));
    }
    // Any truthy/present `done` field ends the stream.
    if field_present(&value, "done") {
        return Some(AiEvent::Done);
    }
    if let Some(delta) = string_field(&value, "delta") {
        return Some(AiEvent::Delta(delta));
    }
    None
}

/// Read `value[key]` as a `String`, or `None` if the field is absent / not a
/// string. The empty string is treated as "absent" so `{"delta":""}` heartbeats
/// don't spam empty deltas.
fn string_field(value: &wasm_bindgen::JsValue, key: &str) -> Option<String> {
    let field = js_sys::Reflect::get(value, &wasm_bindgen::JsValue::from_str(key)).ok()?;
    let s = field.as_string()?;
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Whether `value[key]` exists and is neither `undefined`, `null`, nor `false`.
fn field_present(value: &wasm_bindgen::JsValue, key: &str) -> bool {
    match js_sys::Reflect::get(value, &wasm_bindgen::JsValue::from_str(key)) {
        Ok(field) => {
            if field.is_undefined() || field.is_null() {
                false
            } else {
                // `{"done": false}` should NOT terminate; only a truthy value.
                field.as_bool() != Some(false)
            }
        }
        Err(_) => false,
    }
}
