// Browser/daemon boundary: XHR SSE needs a running desktop daemon; pure SSE
// parsing is unit-tested and the CanvasKit bundle gate covers wasm linkability.
//! XHR-based SSE transport to the desktop daemon's AI SSE endpoints.
//!
//! Mirrors `live_sync.rs` (no `wasm-bindgen-futures`): an `XmlHttpRequest`
//! with an `onprogress` closure that parses newly-arrived `data: {...}`
//! events. The browser populates `responseText` incrementally as the SSE body
//! streams in; the `onprogress` closure tracks a consumed-length cursor so each
//! `data:` block is delivered exactly once.
//!
//! The SSE buffer/payload parsing is pure Rust (`drain_sse_buffer` /
//! `parse_event`, serde_json-backed) so the wire-format logic is unit-testable
//! on the native host; only the XHR plumbing needs a browser. The browser
//! round-trip itself still needs a running daemon + browser to verify.
#![allow(dead_code)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

/// One streamed event from the AI proxy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiEvent {
    /// A token / text fragment to append to the in-flight assistant turn.
    Delta(String),
    /// A reasoning fragment (`{"thinking":"…"}`) — the chat transcript
    /// renders these in the message's collapsible thinking block.
    Thinking(String),
    /// The stream finished successfully.
    Done,
    /// The proxy reported an error for this turn.
    Error(String),
}

impl AiEvent {
    /// True for the events that end a turn (`Done` / `Error`).
    pub fn is_terminal(&self) -> bool {
        matches!(self, AiEvent::Done | AiEvent::Error(_))
    }
}

/// Handle to an in-flight AI SSE request. `abort()` cancels the
/// underlying XHR — the chat Stop button and a replacing send both use it so
/// a cancelled turn stops consuming the socket (and its late events are
/// dropped by the caller's generation check).
pub struct AiStreamHandle {
    xhr: web_sys::XmlHttpRequest,
}

impl AiStreamHandle {
    pub fn abort(&self) {
        let _ = self.xhr.abort();
    }
}

/// POST `body_json` to `{base}/api/ai/stream` and invoke `on_event` for each
/// streamed SSE event. Returns a handle that can abort the in-flight request.
///
/// `base` is the daemon origin (see `crate::daemon_base`). `on_event` is
/// `Rc<dyn Fn(AiEvent)>` so it can be shared across each `onprogress` tick (the
/// same shared-callback pattern `live_sync::get` uses for `on_response`).
///
/// The `onprogress` closure tracks a consumed-length cursor: each tick parses
/// only fully-terminated `data:` blocks (those before the last blank-line
/// separator) and advances the cursor past that separator, so a partial tail is
/// re-parsed on the next tick and no block is delivered twice. A one-shot
/// `onloadend` detaches and drops the progress closure, then runs a final drain and —
/// when the stream ended without a terminal `done` / `error` event (daemon
/// down, non-200, connection dropped mid-stream) — synthesizes an
/// [`AiEvent::Error`] so the caller's turn never hangs in the streaming state.
pub fn post_ai_stream(
    base: &str,
    body_json: String,
    on_event: Rc<dyn Fn(AiEvent)>,
) -> Result<AiStreamHandle, wasm_bindgen::JsValue> {
    post_ai_stream_to(base, "/api/ai/stream", body_json, on_event)
}

/// POST `body_json` to an AI SSE endpoint under `base`. `endpoint` must be an
/// absolute daemon path such as `/api/ai/stream` or `/api/ai/standard`.
pub fn post_ai_stream_to(
    base: &str,
    endpoint: &str,
    body_json: String,
    on_event: Rc<dyn Fn(AiEvent)>,
) -> Result<AiStreamHandle, wasm_bindgen::JsValue> {
    let xhr = web_sys::XmlHttpRequest::new()?;
    xhr.open_with_async("POST", &format!("{base}{endpoint}"), true)?;
    xhr.set_request_header("Content-Type", "application/json")?;

    // `cursor` is the byte offset in `responseText` up to which we've already
    // delivered complete SSE blocks. Shared (Rc<Cell>) between the onprogress
    // and onloadend closures. `saw_terminal` records whether a Done / Error
    // event was delivered, so onloadend can detect an abnormal end.
    let cursor = Rc::new(Cell::new(0usize));
    let saw_terminal = Rc::new(Cell::new(false));
    let terminal_for_send_error = saw_terminal.clone();

    let xhr_for_cb = xhr.clone();
    let on_event_cb = on_event.clone();
    let cursor_cb = cursor.clone();
    let terminal_cb = saw_terminal.clone();
    let onprogress = Closure::<dyn FnMut()>::new(move || {
        let Ok(Some(text)) = xhr_for_cb.response_text() else {
            return;
        };
        let (events, next) = drain_sse_buffer(&text, cursor_cb.get());
        cursor_cb.set(next);
        for evt in events {
            if evt.is_terminal() {
                terminal_cb.set(true);
            }
            on_event_cb(evt);
        }
    });
    let progress_holder = Rc::new(RefCell::new(Some(onprogress)));
    xhr.set_onprogress(
        progress_holder
            .borrow()
            .as_ref()
            .map(|callback| callback.as_ref().unchecked_ref()),
    );

    // One-shot end-of-request hook (fires for load, error, AND abort): drain
    // any tail the last onprogress missed, then synthesize an Error if the
    // stream ended without a terminal event. After an abort the caller has
    // already dropped its turn, so the synthesized event is simply ignored.
    let xhr_for_end = xhr.clone();
    let progress_holder_for_end = progress_holder.clone();
    let onloadend = Closure::<dyn FnMut()>::once_into_js(move || {
        xhr_for_end.set_onprogress(None);
        progress_holder_for_end.borrow_mut().take();
        if let Ok(Some(text)) = xhr_for_end.response_text() {
            let (events, next) = drain_sse_buffer(&text, cursor.get());
            cursor.set(next);
            for evt in events {
                if evt.is_terminal() {
                    saw_terminal.set(true);
                }
                on_event(evt);
            }
        }
        if !saw_terminal.get() {
            let status = xhr_for_end.status().unwrap_or(0);
            on_event(AiEvent::Error(format!(
                "AI stream ended unexpectedly (status {status})"
            )));
        }
    });
    xhr.set_onloadend(Some(onloadend.unchecked_ref()));

    if let Err(error) = xhr.send_with_opt_str(Some(&body_json)) {
        xhr.set_onprogress(None);
        progress_holder.borrow_mut().take();
        xhr.set_onloadend(None);
        // `once_into_js` releases its Rust capture when invoked. Mark this as
        // terminal and invoke the now-detached callback so a synchronous send
        // error cannot leave its XHR/on_event capture waiting for JS GC.
        terminal_for_send_error.set(true);
        let _ = onloadend
            .unchecked_ref::<js_sys::Function>()
            .call0(&wasm_bindgen::JsValue::NULL);
        return Err(error);
    }
    Ok(AiStreamHandle { xhr })
}

/// Parse every complete SSE block in `text[start..]`, returning the parsed
/// events plus the new cursor. Pure — the onprogress/onloadend closures are
/// thin wrappers over this.
///
/// Only the prefix up to the LAST blank-line separator holds complete SSE
/// blocks; everything after it is a partial tail retried on the next tick.
/// `rfind` runs over the whole `text` (not just the fresh slice) so a `\n\n`
/// straddling the previous cursor boundary is still found. A `start` past the
/// end of a shrunk/replaced body clamps to the end instead of panicking.
pub(crate) fn drain_sse_buffer(text: &str, start: usize) -> (Vec<AiEvent>, usize) {
    if start > text.len() {
        return (Vec::new(), text.len());
    }
    if text.len() <= start {
        return (Vec::new(), start); // nothing new since the last tick
    }
    let Some(last_sep) = text.rfind("\n\n") else {
        return (Vec::new(), start); // no complete block yet
    };
    let complete_end = last_sep + 2; // include the separator
    if complete_end <= start {
        return (Vec::new(), start); // already consumed past the last complete block
    }

    let mut events = Vec::new();
    for block in text[start..complete_end].split("\n\n") {
        // SSE field lines: we only care about `data:` lines. A block may in
        // principle carry multiple lines (event:, id:, data:); forward each
        // `data:` payload we find.
        for line in block.lines() {
            let line = line.trim_start();
            if let Some(rest) = line.strip_prefix("data:") {
                if let Some(evt) = parse_event(rest.trim()) {
                    events.push(evt);
                }
            }
        }
    }
    (events, complete_end)
}

/// Parse one SSE `data:` payload (already trimmed of the `data:` prefix) into
/// an [`AiEvent`]. Wire shape is `ai_proxy::delta_to_sse` on the desktop side:
/// `{"delta":…}` / `{"thinking":…}` / `{"done":true}` / `{"error":…}` (plus
/// the OpenAI-style `[DONE]` sentinel for compatibility).
///
/// An `error` field takes precedence over `delta` so a turn that fails
/// mid-stream surfaces the error rather than a partial delta; `{"done":false}`
/// is NOT terminal (only a present, non-false `done` ends the stream); empty
/// `delta` / `thinking` strings are treated as heartbeats and skipped.
pub(crate) fn parse_event(payload: &str) -> Option<AiEvent> {
    // OpenAI-style sentinel terminator.
    if payload == "[DONE]" {
        return Some(AiEvent::Done);
    }
    let value: serde_json::Value = serde_json::from_str(payload).ok()?;
    let obj = value.as_object()?;

    // `error` first — a failed turn must not be reported as a delta.
    if let Some(msg) = nonempty_str(obj, "error") {
        return Some(AiEvent::Error(msg));
    }
    // Any present, non-null, non-false `done` field ends the stream.
    if obj
        .get("done")
        .is_some_and(|v| !v.is_null() && v.as_bool() != Some(false))
    {
        return Some(AiEvent::Done);
    }
    if let Some(delta) = nonempty_str(obj, "delta") {
        return Some(AiEvent::Delta(delta));
    }
    if let Some(thinking) = nonempty_str(obj, "thinking") {
        return Some(AiEvent::Thinking(thinking));
    }
    None
}

/// Read `obj[key]` as a non-empty `String`. The empty string is treated as
/// "absent" so `{"delta":""}` heartbeats don't spam empty deltas.
fn nonempty_str(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<String> {
    let s = obj.get(key)?.as_str()?;
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xhr_progress_callback_has_terminal_cleanup_instead_of_a_page_lifetime_leak() {
        let source = include_str!("web_ai_transport.rs");
        let implementation = source
            .split("#[cfg(test)]")
            .next()
            .expect("transport implementation");
        assert!(!implementation.contains(&["onprogress", ".forget()"].concat()));
        assert!(implementation.contains("xhr_for_end.set_onprogress(None)"));
        assert!(implementation.contains("progress_holder_for_end.borrow_mut().take()"));
        assert!(implementation.contains("xhr.set_onloadend(None)"));
    }

    #[test]
    fn parse_event_maps_each_wire_shape() {
        assert_eq!(
            parse_event(r#"{"delta":"Hi"}"#),
            Some(AiEvent::Delta("Hi".into()))
        );
        assert_eq!(
            parse_event(r#"{"thinking":"hmm"}"#),
            Some(AiEvent::Thinking("hmm".into()))
        );
        assert_eq!(parse_event(r#"{"done":true}"#), Some(AiEvent::Done));
        assert_eq!(parse_event("[DONE]"), Some(AiEvent::Done));
        assert_eq!(
            parse_event(r#"{"error":"boom"}"#),
            Some(AiEvent::Error("boom".into()))
        );
    }

    #[test]
    fn parse_event_error_takes_precedence_over_delta_and_done() {
        assert_eq!(
            parse_event(r#"{"delta":"x","error":"boom","done":true}"#),
            Some(AiEvent::Error("boom".into()))
        );
    }

    #[test]
    fn parse_event_skips_heartbeats_and_unknown_payloads() {
        assert_eq!(parse_event(r#"{"delta":""}"#), None);
        assert_eq!(parse_event(r#"{"tool":"insert_node","args":"{}"}"#), None);
        assert_eq!(parse_event("not json"), None);
        // `done:false` must NOT terminate the stream.
        assert_eq!(parse_event(r#"{"done":false}"#), None);
        // Non-object payloads (arrays / scalars) are dropped.
        assert_eq!(parse_event("[]"), None);
    }

    #[test]
    fn parse_event_unescapes_json_strings() {
        assert_eq!(
            parse_event(r#"{"delta":"a\"b\nc"}"#),
            Some(AiEvent::Delta("a\"b\nc".into()))
        );
    }

    #[test]
    fn drain_delivers_only_complete_blocks_and_advances_cursor() {
        // First chunk: one complete block + a partial tail.
        let chunk1 = "data: {\"delta\":\"He\"}\n\ndata: {\"del";
        let (events, cursor) = drain_sse_buffer(chunk1, 0);
        assert_eq!(events, vec![AiEvent::Delta("He".into())]);
        assert_eq!(cursor, "data: {\"delta\":\"He\"}\n\n".len());

        // No new separator → nothing delivered, cursor unchanged.
        let (events, cursor2) = drain_sse_buffer(chunk1, cursor);
        assert!(events.is_empty());
        assert_eq!(cursor2, cursor);

        // The tail completes (plus the terminal event) → exactly the new
        // blocks are delivered, never the first one again.
        let full =
            "data: {\"delta\":\"He\"}\n\ndata: {\"delta\":\"llo\"}\n\ndata: {\"done\":true}\n\n";
        let (events, cursor3) = drain_sse_buffer(full, cursor);
        assert_eq!(events, vec![AiEvent::Delta("llo".into()), AiEvent::Done]);
        assert_eq!(cursor3, full.len());
    }

    #[test]
    fn drain_handles_shrunk_body_and_consumed_prefix() {
        // Cursor past the end (replaced body) clamps without panicking.
        let (events, cursor) = drain_sse_buffer("short", 100);
        assert!(events.is_empty());
        assert_eq!(cursor, 5);
        // Cursor already past the last separator → idle.
        let text = "data: {\"delta\":\"x\"}\n\n";
        let (events, cursor) = drain_sse_buffer(text, text.len());
        assert!(events.is_empty());
        assert_eq!(cursor, text.len());
    }

    #[test]
    fn drain_forwards_multiple_data_lines_in_one_block() {
        let text = "data: {\"delta\":\"a\"}\ndata: {\"delta\":\"b\"}\n\n";
        let (events, _) = drain_sse_buffer(text, 0);
        assert_eq!(
            events,
            vec![AiEvent::Delta("a".into()), AiEvent::Delta("b".into())]
        );
    }
}
