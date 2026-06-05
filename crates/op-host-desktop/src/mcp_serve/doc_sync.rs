//! Whole-document REST sync wire helpers (TS `/api/mcp/document` parity).
//! Shared by the desktop live server (`mcp_live`) and the headless web-canvas
//! daemon (`web_canvas_server`) so both speak the exact same shape as the TS
//! web app's `apps/web/server/api/mcp/document.post.ts`. Re-exported from
//! `mcp_serve` so callers keep using `crate::mcp_serve::*`.

/// True for the TS live-canvas whole-document sync route
/// (`POST /api/mcp/document`). Any other method/path falls through to the
/// JSON-RPC `/mcp` handling.
pub(crate) fn is_document_sync_route(method: &str, path: &str) -> bool {
    method == "POST" && path == "/api/mcp/document"
}

/// Validate a `/api/mcp/document` body and return the inner `document` JSON
/// (ready for `load_canonical`). Mirrors `document.post.ts`: `document` must be
/// present (else "Missing document in request body"), carry a non-empty
/// `version`, and have an array `children` OR `pages` (else "Invalid document
/// format").
pub(crate) fn parse_document_sync_body(body: &str) -> Result<String, String> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|_| "Invalid document format".to_string())?;
    let document = value
        .get("document")
        .ok_or_else(|| "Missing document in request body".to_string())?;
    let obj = document
        .as_object()
        .ok_or_else(|| "Invalid document format".to_string())?;
    let has_version = obj
        .get("version")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|v| !v.is_empty());
    let has_children = obj.get("children").is_some_and(serde_json::Value::is_array);
    let has_pages = obj.get("pages").is_some_and(serde_json::Value::is_array);
    if !has_version || (!has_children && !has_pages) {
        return Err("Invalid document format".to_string());
    }
    serde_json::to_string(document).map_err(|e| e.to_string())
}

/// Success body for a whole-document sync — matches `document.post.ts`'s
/// `{ ok: true, version }`.
pub(crate) fn document_sync_ok(version: u64) -> String {
    format!(r#"{{"ok":true,"version":{version}}}"#)
}

/// Error body for a rejected whole-document sync (HTTP 400).
pub(crate) fn rest_error_body(message: &str) -> String {
    format!(r#"{{"ok":false,"error":"{}"}}"#, json_escape(message))
}

/// Minimal JSON string escaping for embedding a message in a JSON reply body.
pub(crate) fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push(' '),
            c => out.push(c),
        }
    }
    out
}
