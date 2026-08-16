//! Browser-extension snapshot ingress (`POST /api/import/web-snapshot`).
//!
//! The OpenPencil Chrome extension (`packages/op-chrome-extension`) runs
//! the canonical DOM extractor (`op-html/assets/snapshot-extractor.js`) in
//! the active tab and has to hand the resulting JSON to the running
//! editor. It cannot use the general `/mcp` surface:
//!
//! * `admission::check_boundary` refuses every `Origin` other than this
//!   instance's own loopback origin, and a browser ALWAYS attaches
//!   `Origin: chrome-extension://<id>` to an extension-initiated `POST`
//!   (Fetch spec: `Origin` is appended for any non-`GET`/`HEAD` request).
//! * `admission::check_token` requires the per-instance token from
//!   `~/.openpencil/.op-mcp-port`, and an extension has no filesystem
//!   access to read it.
//!
//! So this module adds ONE narrowly-scoped route instead of widening
//! either gate:
//!
//! * It is a fixed alias for exactly one MCP tool, `import_web_snapshot`
//!   — insert-only. It cannot read the document, cannot delete, cannot
//!   move or export anything, so an extension reaching it can add visible
//!   (and undoable) nodes and nothing else. Reads and every other write
//!   tool stay behind the full token gate.
//! * `admission::check_boundary` still screens it: `Host` must be a
//!   numeric loopback literal on the bound port, and the `Origin` must be
//!   either this instance's own loopback origin or an ACCEPTED
//!   `chrome-extension://<id>` origin. A drive-by web page sends its own
//!   `http://evil.example` origin (which a page cannot forge or suppress
//!   on a cross-origin request), so DNS rebinding stays closed.
//! * `Content-Type: application/json` is required, so the route is not
//!   reachable as a CORS "simple request".
//! * The reply's `Access-Control-Allow-Origin` echoes exactly the origin
//!   that was accepted — never `*` — so an extension the boundary did not
//!   accept cannot read the answer even if the browser lets it ask.
//! * A declared body over [`MAX_SNAPSHOT_BODY`] is refused with `413`
//!   BEFORE the body is read, so an untokened caller cannot make this
//!   process buffer the endpoint-wide 64 MiB cap. A `POST` here without a
//!   `Content-Length` is refused with `411`.
//!
//! Which extension ids count as accepted has two modes — open by default,
//! pinned via `OPENPENCIL_EXTENSION_ALLOWED_IDS`. See the
//! "Browser-extension pinning" section of [`super::admission`].
//!
//! A local non-browser process (no `Origin` at all) also reaches this
//! route untokened — but it can already read the discovery file and use
//! the full tool surface with the real token, so this adds no capability
//! there.

use super::*;

/// The single route a browser extension may reach on the live endpoint.
/// `pub(crate)` because [`crate::mcp_serve::read_http_request`] has to
/// recognise it to apply this route's smaller body cap before it reads a
/// body (see [`MAX_SNAPSHOT_BODY`]).
pub(crate) const SNAPSHOT_INGEST_PATH: &str = "/api/import/web-snapshot";

/// Largest `Content-Length` this route accepts, well under the endpoint-wide
/// 64 MiB `MAX_BODY`.
///
/// This is the ONE untokened body-carrying route, so its cap is what bounds
/// how much memory an unauthenticated local caller can make the editor
/// buffer PER CONNECTION — the loopback listener admits up to
/// `MAX_LIVE_CONNS` (64) concurrent connections, so the aggregate untokened
/// ceiling is 64 × this (plus the UTF-8→String copy each body takes on the
/// way in). 48 MiB fits any real snapshot: the extractor stops at 40,000
/// nodes (~20 MiB of styled nodes at the pathological densest) and
/// rasterizes at most ~24 MiB of images
/// (`crates/op-html/assets/snapshot-extractor.js`) — ~44 MiB at both maxima
/// simultaneously, which no real page reaches — and a body that large is
/// already a partial capture the user is told about.
pub(crate) const MAX_SNAPSHOT_BODY: usize = 48 * 1024 * 1024;

/// The one MCP tool this route is an alias for.
const SNAPSHOT_TOOL: &str = "import_web_snapshot";

/// The `tools/call` envelope around the snapshot argument, hand-written
/// rather than built through `serde_json::json!` so the body — which can be
/// tens of MiB — is copied once (see [`snapshot_tool_call`]).
///
/// Key order is deliberate: `method`, then `params.name`, then the snapshot.
/// `op_mcp`'s hand-rolled parser takes the FIRST `"method"` / `"name"` it
/// finds, so putting both ahead of the payload keeps them unreachable
/// independently of serde's escaping (which is what actually prevents a
/// payload from producing a bare `"name"` key at all — pinned by
/// `snapshot_ingest_tests::hostile_snapshot_bodies_cannot_reshape_the_call`).
const CALL_HEAD: &str = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":""#;
const CALL_MID: &str = r#"","arguments":{"snapshot":"#;
const CALL_TAIL: &str = "}}}";

/// Whether `path` is the ingest route, regardless of method. Used by the
/// admission boundary, which must also let the `OPTIONS` preflight for
/// this path through from an extension origin.
pub(super) fn is_snapshot_ingest_path(path: &str) -> bool {
    path == SNAPSHOT_INGEST_PATH
}

/// Whether this request is the ingest call itself.
pub(super) fn is_snapshot_ingest_route(method: &str, path: &str) -> bool {
    method == "POST" && is_snapshot_ingest_path(path)
}

/// `application/json` (optionally with parameters, e.g. `; charset=utf-8`).
/// Mirrors the web daemon's `origin_guard::content_type_is_json`, which is
/// private to that module.
fn content_type_is_json(value: Option<&str>) -> bool {
    value.is_some_and(|value| {
        value
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .eq_ignore_ascii_case("application/json")
    })
}

/// Build the `tools/call` message this route is an alias for. Pure, so the
/// escaping of an arbitrary snapshot body into the `snapshot` string arg is
/// unit-testable without a socket.
///
/// The snapshot is streamed straight into the output buffer through serde's
/// string serializer — same escaping a `serde_json::json!` value would
/// produce, but without materializing an intermediate `Value` and then a
/// second `String` of it. On a 48 MiB body that is one copy instead of
/// three, on a route any local caller can reach untokened.
pub(super) fn snapshot_tool_call(snapshot: &str) -> String {
    let mut out: Vec<u8> = Vec::with_capacity(snapshot.len() + CALL_HEAD.len() + 96);
    out.extend_from_slice(CALL_HEAD.as_bytes());
    out.extend_from_slice(SNAPSHOT_TOOL.as_bytes());
    out.extend_from_slice(CALL_MID.as_bytes());
    // Serializing a `&str` cannot fail, and writing into a `Vec` cannot
    // either; the escaping is serde's, so no hand-rolled quoting is trusted.
    serde_json::to_writer(&mut out, snapshot).expect("serializing a string into a Vec cannot fail");
    out.extend_from_slice(CALL_TAIL.as_bytes());
    String::from_utf8(out).expect("serde_json emits UTF-8")
}

/// Translate the tool's JSON-RPC reply into this route's REST shape:
/// `{ok:true,result:{…}}` on success, `{ok:false,error:"…"}` otherwise
/// (the same `{ok,error}` contract `POST /api/mcp/document` answers with).
pub(super) fn rest_reply(response: &str) -> (&'static str, String) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(response) else {
        return (
            "500 Internal Server Error",
            crate::mcp_serve::rest_error_body("import produced no MCP reply"),
        );
    };
    // A transport/parse fault is a JSON-RPC `error`; a tool-level failure is
    // an `isError` result. Both are client faults here → 400.
    if let Some(message) = value
        .pointer("/error/message")
        .and_then(serde_json::Value::as_str)
    {
        return (
            "400 Bad Request",
            crate::mcp_serve::rest_error_body(message),
        );
    }
    let text = value
        .pointer("/result/content/0/text")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if value
        .pointer("/result/isError")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return ("400 Bad Request", crate::mcp_serve::rest_error_body(text));
    }
    // The tool's own result is itself a JSON object (`{"wrote":"true",
    // "nodeCount":"12"}`); embed it verbatim so the caller reads fields
    // instead of a doubly-encoded string. A non-JSON text (no tool ever
    // emits one today) degrades to a JSON string rather than corrupting
    // the reply.
    match serde_json::from_str::<serde_json::Value>(text) {
        Ok(result) => ("200 OK", format!(r#"{{"ok":true,"result":{result}}}"#)),
        Err(_) => (
            "200 OK",
            format!(
                r#"{{"ok":true,"result":{}}}"#,
                serde_json::Value::String(text.to_string())
            ),
        ),
    }
}

/// Handle `POST /api/import/web-snapshot`: the request body IS the
/// extractor's snapshot JSON. Runs the `import_web_snapshot` tool through
/// the same snapshot/apply round-trip the JSON-RPC path uses, so the
/// insert lands on the UI thread under the usual `CollabGatePolicy`.
///
/// `cors_origin` is the one origin the boundary accepted for this request
/// (`admission::cors_origin_for`); every reply below echoes exactly that
/// and nothing else.
pub(super) fn serve_snapshot_ingest<S: std::io::Read + std::io::Write>(
    stream: &mut S,
    req_tx: &Sender<UiRequest>,
    wake_ui: &UiWake,
    stateful_lock: &Mutex<()>,
    req: &crate::mcp_serve::HttpRequest,
    cors_origin: Option<&str>,
) -> Result<(), McpLiveError> {
    if !content_type_is_json(req.content_type.as_deref()) {
        return write_http_with_origin(
            stream,
            "415 Unsupported Media Type",
            &crate::mcp_serve::rest_error_body(
                "this route requires Content-Type: application/json",
            ),
            cors_origin,
        );
    }
    if req.body.trim().is_empty() {
        return write_http_with_origin(
            stream,
            "400 Bad Request",
            &crate::mcp_serve::rest_error_body("snapshot body is empty"),
            cors_origin,
        );
    }
    let call = snapshot_tool_call(&req.body);
    // Same serialization as the JSON-RPC write path: one live apply at a
    // time, and a coherent snapshot to validate against.
    let _guard = stateful_lock
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let mut state = request_snapshot(req_tx, wake_ui)?;
    let response = crate::mcp_serve::process_message_with_applier(
        &mut state,
        &call,
        |tool_name, local_state, cmd| match request_apply(
            req_tx,
            wake_ui,
            tool_name.to_string(),
            cmd.clone(),
        ) {
            Ok(ack) => {
                if ack.applied {
                    let _ = local_state.apply(cmd.clone());
                }
                ack.applied
            }
            Err(e) => {
                eprintln!("openpencil-desktop mcp: snapshot ingest apply failed: {e}");
                false
            }
        },
    )?
    .unwrap_or_default();
    let (status, body) = rest_reply(&response);
    write_http_with_origin(stream, status, &body, cors_origin)
}

#[cfg(test)]
#[path = "snapshot_ingest_tests.rs"]
mod tests;
