//! The two local ingresses: request bodies out, reply classification back.
//!
//! 1. `POST /api/import/web-snapshot` — the insert-only snapshot route the
//!    running desktop editor exposes on its live MCP endpoint
//!    (`crates/op-host-services/src/mcp_live/snapshot_ingest.rs`). It is the
//!    only route on that endpoint reachable from a `chrome-extension://`
//!    origin, and it needs no token. The request body IS the snapshot JSON,
//!    so there is nothing to build.
//! 2. `POST /mcp` — a plain MCP `tools/call` for `import_web_snapshot`. This
//!    covers exactly one host: the UNMANAGED headless daemon
//!    (`openpencil-desktop --serve-web <port>`), whose `/mcp` needs no token.
//!    It is NOT a compatibility path for older desktop builds — a desktop app
//!    that predates the ingest route answers 404 there and 403 on `/mcp` (its
//!    admission gate refuses extension origins outright), and the fallback
//!    only runs on a 404.
//!
//! Neither request is subject to CORS. The popup is an extension page, and
//! `manifest.json` grants loopback `host_permissions`, which is what lets a
//! `fetch` from that page reach the editor as a privileged request: no
//! preflight is sent and no `Access-Control-Allow-Origin` is required on the
//! way back. The editor's own origin checks still apply — they are its
//! admission gate, not the browser's — so both hosts still get to refuse an
//! extension origin, and both replies still name the requesting origin.

use op_util::json_escape::escape_json_quoted;
use serde_json::Value;

use crate::js_text::truncate_utf16;

/// Tool the `/mcp` fallback calls.
const IMPORT_TOOL: &str = "import_web_snapshot";

/// Longest raw reply body echoed back into a status message.
const MAX_DETAIL_UNITS: usize = 400;

/// What the popup should do with a reply.
#[derive(Debug, Clone, PartialEq)]
pub enum Reply {
    /// This host has no such route — try the next ingress.
    Fallback,
    /// The snapshot landed.
    Imported {
        node_count: f64,
        warnings: Vec<String>,
    },
    /// Refused. `code` is the popup's message selector.
    Failed { code: FailureCode, detail: String },
}

/// Failure kinds the popup renders differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureCode {
    /// The host is listening but will not accept an extension origin.
    Forbidden,
    /// The host accepted the request and refused the snapshot.
    Import,
}

impl FailureCode {
    /// Wire name shared with the popup's error switch.
    pub fn as_str(self) -> &'static str {
        match self {
            FailureCode::Forbidden => "forbidden",
            FailureCode::Import => "import",
        }
    }
}

/// Marker standing in for the snapshot's JSON string literal inside
/// [`mcp_envelope_template`].
///
/// It is never adjacent to anything a splice could confuse it with: the
/// template is generated entirely here, and the only run of `_` characters in
/// it is this one.
pub const SNAPSHOT_PLACEHOLDER: &str = "__OPENPENCIL_SNAPSHOT__";

/// The JSON-RPC `tools/call` envelope, with [`SNAPSHOT_PLACEHOLDER`] where
/// the snapshot's JSON string literal (quotes included) goes.
///
/// # Why the snapshot itself is spliced in by JavaScript
///
/// The envelope's *shape* is a wire contract — method, tool name, key order,
/// and the fact that the snapshot travels as a JSON **string** the tool
/// parses itself — so it is pinned here and covered by tests. The snapshot's
/// *escaping* is JavaScript's job for two reasons, both of which the previous
/// `build_mcp_envelope(snapshot: &str)` got wrong by construction:
///
/// * **Lone surrogates.** A page can put an unpaired surrogate into a title
///   or an `alt` attribute. `JSON.stringify` (well-formed since ES2019)
///   escapes it as `\udXXX` and round-trips it. A `&str` parameter cannot
///   hold one at all, so wasm-bindgen rewrote it to `U+FFFD` on the way in —
///   silent corruption of an otherwise intact capture.
/// * **Peak memory.** Handing a maximum-size (48 MB) snapshot to wasm copied it into the
///   wasm heap, then built the escaped envelope beside it, then copied the
///   result back out: roughly four live copies at the peak. Splicing in JS
///   leaves the snapshot where it already is.
///
/// The template is a few dozen bytes, so JS pays nothing to hold it, and one
/// `split` on the placeholder yields the prefix and suffix to join around
/// `JSON.stringify(snapshot)`. Injection safety is unchanged: everything
/// attacker-influenced is inside that one `JSON.stringify` call, and the
/// bytes around it come from here.
pub fn mcp_envelope_template() -> String {
    let mut out = String::with_capacity(128);
    out.push_str(r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"#);
    out.push_str(&escape_json_quoted(IMPORT_TOOL));
    out.push_str(r#","arguments":{"snapshot":"#);
    out.push_str(SNAPSHOT_PLACEHOLDER);
    out.push_str("}}}");
    out
}

/// Classify a reply from `POST /api/import/web-snapshot`.
pub fn classify_ingest_reply(status: u16, text: &str) -> Reply {
    // No such route on this host — fall back rather than reporting an error.
    if status == 404 {
        return Reply::Fallback;
    }
    let json = parse_reply(text);
    if (200..300).contains(&status) && json.as_ref().is_some_and(|v| v["ok"] == Value::Bool(true)) {
        let result = json.as_ref().map(|v| &v["result"]);
        return imported(result);
    }
    Reply::Failed {
        code: if status == 403 {
            FailureCode::Forbidden
        } else {
            FailureCode::Import
        },
        detail: error_text(status, text, json.as_ref()),
    }
}

/// Classify a reply from the `POST /mcp` fallback.
pub fn classify_mcp_reply(status: u16, text: &str) -> Reply {
    let json = parse_reply(text);
    if status == 401 || status == 403 {
        // Only reachable when the 404 above sent us here and this host then
        // refused the general tool surface — i.e. not the unmanaged daemon
        // this fallback exists for. Report it as "no ingress" rather than
        // retrying.
        return Reply::Failed {
            code: FailureCode::Forbidden,
            detail: error_text(status, text, json.as_ref()),
        };
    }
    if !(200..300).contains(&status) {
        return Reply::Failed {
            code: FailureCode::Import,
            detail: error_text(status, text, json.as_ref()),
        };
    }
    let result = json.as_ref().map(|v| &v["result"]);
    let tool_failed = result.is_none_or(|r| is_falsy(r) || r["isError"] == Value::Bool(true));
    if tool_failed {
        return Reply::Failed {
            code: FailureCode::Import,
            detail: error_text(status, text, json.as_ref()),
        };
    }
    // The tool answers with its payload as text inside the MCP content
    // envelope; a payload that will not parse is not an error here — it just
    // yields the zero outcome, exactly as the JS predecessor's
    // `try { JSON.parse } catch { null }` did.
    let payload = result
        .and_then(|r| r["content"][0]["text"].as_str())
        .and_then(|t| serde_json::from_str::<Value>(t).ok());
    imported(payload.as_ref())
}

/// Read `{nodeCount, warnings}` out of the import tool's result object.
fn imported(result: Option<&Value>) -> Reply {
    let node_count = result.map_or(0.0, |r| js_number(&r["nodeCount"]));
    let warnings = result
        .and_then(|r| r["warnings"].as_str())
        .filter(|w| !w.is_empty())
        .map(|w| w.split('\n').map(str::to_owned).collect())
        .unwrap_or_default();
    Reply::Imported {
        node_count,
        warnings,
    }
}

/// Pull a human-readable failure out of either reply shape.
///
/// Every branch goes through [`MAX_DETAIL_UNITS`]. The cap is not only about
/// the raw-body case: a structured `error.message` is just as much
/// server-chosen text, an `isError` tool result routinely carries the
/// importer's whole warning list, and this string lands verbatim in a 340 px
/// popup. A reply that is one long line must not push the buttons off-screen.
fn error_text(status: u16, text: &str, json: Option<&Value>) -> String {
    if let Some(value) = json {
        if let Some(message) = value["error"].as_str() {
            return capped(message);
        }
        if let Some(message) = value["error"]["message"].as_str() {
            return capped(message);
        }
        if let Some(message) = value["result"]["content"][0]["text"].as_str() {
            return capped(message);
        }
    }
    let head = truncate_utf16(text, MAX_DETAIL_UNITS);
    if head.is_empty() {
        format!("HTTP {status}")
    } else {
        head.to_owned()
    }
}

/// `text` cut to [`MAX_DETAIL_UNITS`] UTF-16 code units.
fn capped(text: &str) -> String {
    truncate_utf16(text, MAX_DETAIL_UNITS).to_owned()
}

/// Parse a reply body, tolerating an empty or non-JSON one.
fn parse_reply(text: &str) -> Option<Value> {
    if text.is_empty() {
        return None;
    }
    serde_json::from_str(text).ok()
}

/// JS `Number(x) || 0` over a JSON value, matching
/// `Number(result?.nodeCount ?? 0)` plus the `Number.isFinite` guard.
fn js_number(value: &Value) -> f64 {
    match value {
        Value::Number(n) => n.as_f64().filter(|n| n.is_finite()).unwrap_or(0.0),
        Value::String(s) => {
            // `Number(" 12 ")` is 12 and `Number("")` is 0; anything else is
            // NaN, which the caller's finiteness guard turned into 0.
            let trimmed = crate::js_text::js_trim(s);
            if trimmed.is_empty() {
                0.0
            } else {
                trimmed
                    .parse::<f64>()
                    .ok()
                    .filter(|n| n.is_finite())
                    .unwrap_or(0.0)
            }
        }
        Value::Bool(b) => f64::from(u8::from(*b)),
        // `Number(null)` is 0; `undefined`, objects and arrays give NaN,
        // which the guard folds to 0 as well.
        _ => 0.0,
    }
}

/// JS truthiness for the values a JSON reply can hold.
fn is_falsy(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Bool(b) => !*b,
        Value::Number(n) => n.as_f64().is_none_or(|n| n == 0.0 || n.is_nan()),
        Value::String(s) => s.is_empty(),
        _ => false,
    }
}
