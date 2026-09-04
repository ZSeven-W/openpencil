//! Typed failures for the `--serve-web` web-canvas daemon
//! (`web_canvas_server.rs`).
//!
//! Style follows `op_orchestrator::OrchestratorError`: a plain enum plus a
//! hand-written `Display`, no `thiserror` and no new dependency. `Display` is
//! transparent — each variant carries the exact sentence the route already
//! embedded in its JSON error body, so the wire bytes are unchanged.
//!
//! What the enum adds is a route-independent classification plus
//! [`WebCanvasError::http_status`], which turns "which status does this
//! failure answer with" from a per-call-site literal into one table.
//!
//! `mcp_serve` now reports [`crate::mcp_serve::McpServeError`], so its
//! failures reach this enum through the [`From`] impl at the bottom of this
//! file and the routes just use `?` — no per-call-site re-labelling. The
//! file-writing `export` / `export_pdf` entry points are typed too now (the
//! `op-host-desktop::persistence` caller that pinned them to `String`
//! converted), so they arrive through the second [`From`] impl rather than a
//! `.map_err`. The remaining dependencies (`op_pen_loader` and the shared
//! settings/document IO helpers) are adapted into a variant at the call
//! site. The daemon's three public entry points
//! (`parse_serve_web_args`, `startup_editor_for_web_canvas`,
//! `run_web_canvas`) report this enum directly now: `cli_modes.rs` is their
//! only consumer in the workspace — the host binaries reach them through
//! `cli_modes::run_cli_mode`, not directly — and it merely `Display`s the
//! error, so the `*_typed` twins and their `String` wrappers are gone. The
//! shared settings loader is typed too (`settings_io::SettingsIoError`) and is
//! adapted into `Config` at the two start-up call sites, since a settings file
//! the daemon cannot round-trip aborts start-up rather than answering a
//! request.
//!
//! The enum is `pub` — matching its two sibling reference enums
//! [`crate::mcp_serve::McpServeError`] and [`crate::export::ExportError`] —
//! because those three public entry points name it in their signatures.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebCanvasError {
    /// The request body is malformed, has the wrong shape, or names an
    /// unsupported parameter — a client fault.
    BadRequest(String),
    /// A document failed to load or validate through the canonical loader.
    Document(String),
    /// The raster / PDF export pipeline failed.
    Export(String),
    /// A filesystem operation on a request-scoped file (the save target, an
    /// export temp file, the backing document) failed.
    Io(String),
    /// Start-up configuration failed: bad `--serve-web` argv, an unusable
    /// settings file, or a socket that would not bind. Never becomes an HTTP
    /// response — it aborts daemon start-up.
    Config(String),
    /// A connection-level read/write failed (request parse, response write,
    /// SSE stream). Never becomes an HTTP response — the socket is already
    /// unusable; the accept loop just logs it.
    Transport(String),
    /// A live collaboration session refuses this write. Not a client fault and
    /// not a daemon fault — the document is healthy and the request was
    /// well-formed; it simply cannot be sequenced right now.
    Collab(crate::web_canvas_server::DaemonMutationRefusal),
    /// A live session opened a capture for a whole-document push and then
    /// declined to commit it. Distinct from `Collab`: the write was not
    /// refused up front, it was discarded, so the browser's copy is now
    /// definitively behind and must be refetched rather than retried.
    IngestRejected(crate::web_canvas_server::IngestOutcome, u64),
}

impl WebCanvasError {
    /// The HTTP status a route answers with when it surfaces this failure.
    ///
    /// The four request-scoped kinds are all client faults — the daemon's
    /// in-memory document authority is healthy; what failed is the request's
    /// payload or the file it named — so they answer `400`, matching the
    /// pre-conversion behaviour of every route in this module exactly.
    /// `Config` / `Transport` never reach a response; they report the generic
    /// `500` so a future caller that does surface one is not silently wrong.
    pub fn http_status(&self) -> &'static str {
        match self {
            WebCanvasError::BadRequest(_)
            | WebCanvasError::Document(_)
            | WebCanvasError::Export(_)
            | WebCanvasError::Io(_) => "400 Bad Request",
            WebCanvasError::Config(_) | WebCanvasError::Transport(_) => "500 Internal Server Error",
            WebCanvasError::Collab(refusal) => refusal.http_status(),
            WebCanvasError::IngestRejected(..) => "409 Conflict",
        }
    }

    /// Stable machine-readable code, when the failure has one.
    ///
    /// Only collaboration refusals carry a code today: a client has to tell
    /// "the session is read-only for you" from "the session is busy" to decide
    /// whether retrying is worth it.
    pub fn error_code(&self) -> Option<&'static str> {
        match self {
            WebCanvasError::Collab(refusal) => Some(refusal.code()),
            WebCanvasError::IngestRejected(outcome, _) => outcome.error_code(),
            _ => None,
        }
    }
}

impl fmt::Display for WebCanvasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WebCanvasError::BadRequest(m)
            | WebCanvasError::Document(m)
            | WebCanvasError::Export(m)
            | WebCanvasError::Io(m)
            | WebCanvasError::Config(m)
            | WebCanvasError::Transport(m) => f.write_str(m),
            WebCanvasError::Collab(refusal) => write!(f, "{refusal}"),
            WebCanvasError::IngestRejected(..) => {
                f.write_str("the live session discarded this document push; refetch and reapply")
            }
        }
    }
}

impl std::error::Error for WebCanvasError {}

/// Single-table mapping from the shared MCP transport's failures onto this
/// daemon's classification, replacing the per-call-site `.map_err(...)`
/// re-labelling every route used to carry. Both `Display` impls are
/// transparent, so the resulting HTTP body / log line is byte-identical to
/// the pre-conversion text.
impl From<crate::mcp_serve::McpServeError> for WebCanvasError {
    fn from(error: crate::mcp_serve::McpServeError) -> WebCanvasError {
        use crate::mcp_serve::McpServeError as E;
        match error {
            // The daemon's document authority failed to load the file it was
            // pointed at — the same 400 the route reported before.
            E::Document(m) => WebCanvasError::Document(m),
            // A rejected JSON-RPC message is a client fault: the parser or
            // registry refused it and nothing was applied. A refused REST
            // body (`Validation`) is the same shape of fault on the
            // document-sync route, and answered the same 400 before this
            // conversion via an explicit `.map_err(WebCanvasError::BadRequest)`.
            E::Dispatch(m) | E::Validation(m) => WebCanvasError::BadRequest(m),
            // Malformed framing and socket failures alike leave the
            // connection unusable, so both land on `Transport` — which the
            // accept loop logs instead of answering with. Matches the
            // pre-conversion `.map_err(WebCanvasError::Transport)` exactly.
            E::Protocol(m) | E::Io(m) => WebCanvasError::Transport(m),
            // A route-specific framing refusal (over-cap / missing
            // `Content-Length`) never fires on this daemon's routes — only
            // the live endpoint's extension-scoped routes declare one — but
            // it is the same class of client fault as `Dispatch`, so it
            // answers 400 here rather than being logged as a transport failure.
            E::Framing { message, .. } => WebCanvasError::BadRequest(message),
            E::Config(m) => WebCanvasError::Config(m),
        }
    }
}

/// Same idea for the raster/PDF export core. Every export failure answered
/// `400` before this conversion, and `Export` / `Io` both still do (see
/// [`WebCanvasError::http_status`]), so routing the write failure to `Io`
/// sharpens the classification without moving a single status code or byte
/// of the response body.
impl From<crate::export::ExportError> for WebCanvasError {
    fn from(error: crate::export::ExportError) -> WebCanvasError {
        use crate::export::ExportError as E;
        match error {
            E::Write(m) => WebCanvasError::Io(m),
            other => WebCanvasError::Export(other.to_string()),
        }
    }
}

/// Same idea for the shared document IO core. The save/open routes used to
/// funnel every `doc_io` failure through one hand-written
/// `.map_err(WebCanvasError::Io)`; this table keeps that `400` (see
/// [`WebCanvasError::http_status`], where `Document` and `Io` share a status)
/// while separating "the bytes we were handed are not a document" from "the
/// filesystem underneath refused" — the split `doc_io::DocIoError`'s variant
/// set already encodes. No status code and no byte of any response body moves:
/// every `DocIoError` `Display` is either structured-then-reformatted or
/// transparent, so the sentence is the same one the `.to_string()` produced.
impl From<crate::doc_io::DocIoError> for WebCanvasError {
    fn from(error: crate::doc_io::DocIoError) -> WebCanvasError {
        use crate::doc_io::DocIoError as E;
        match error {
            // Content faults: the document itself is empty, not UTF-8, off
            // schema, or from a build whose format this one cannot read.
            E::SourceEmpty { .. }
            | E::SourceNotUtf8 { .. }
            | E::InvalidUtf8Document(_)
            | E::Schema(_)
            | E::LegacyFormat(_) => WebCanvasError::Document(error.to_string()),
            // Everything else is the filesystem or the serializer around it.
            other => WebCanvasError::Io(other.to_string()),
        }
    }
}
