//! Admission control for the live-GUI MCP endpoint (`127.0.0.1:<port>/mcp`).
//!
//! The live endpoint drives the on-screen document, and during a
//! collaboration session that document is the SHARED one. The gate here is
//! browser screening — it closes the DNS-rebinding path that would let any
//! web page on the machine reach this endpoint.
//!
//! [`check_boundary`] — a `Host` that is not a numeric loopback literal on
//! the bound port, or ANY `Origin` other than this instance's own loopback
//! origin, is refused. Both headers are browser-controlled but not
//! page-forgeable, which is what actually closes DNS rebinding: a rebound
//! `evil.com` page still sends `Host: evil.com:<port>` and
//! `Origin: http://evil.com`. Requests with no `Origin` at all are normal
//! non-browser clients (the `op` CLI, the VS Code MCP proxy, a local agent
//! runner) and pass.
//!
//! No per-instance `X-OpenPencil-Token` is demanded: the local desktop and
//! a self-hosted serve-web daemon trust every local process that clears the
//! boundary — the token it published in `~/.openpencil/.op-mcp-port` and the
//! `ping` reply was readable by any such process anyway, so it only added
//! friction for a caller that had only the URL (a bare MCP client). The
//! online multi-tenant daemon is a SEPARATE request loop that authenticates
//! per account (`web_canvas_server::RequestAuth`, a `Bearer` token the hub
//! introspects), so relaxing this endpoint does not touch online auth.
//! `CollabGatePolicy` still runs on the UI thread for each apply and decides
//! what a session permits. `openpencil/shutdown` keeps its own body-carried
//! token check (`mcp_serve::shutdown_request_id`), unchanged.
//!
//! Deliberately still tokenless too: `OPTIONS` preflight, and the stateless
//! `initialize` / `notifications/initialized` / `ping` probes, which carry
//! no document data and are how a client discovers this instance.
//!
//! # Browser-extension pinning (`OPENPENCIL_EXTENSION_ALLOWED_IDS`)
//!
//! The insert-only snapshot ingress ([`super::snapshot_ingest`]) is the open
//! extension capability. Which ids it accepts has two modes:
//!
//! * **Open (default).** Any well-formed extension origin passes. The
//!   OpenPencil extension is unpublished, so it has no stable Chrome Web
//!   Store id yet — an unpacked load derives a different id per machine and
//!   pinning a literal here would refuse the extension on every developer's
//!   box. What this mode grants is still only the insert-only route.
//! * **Pinned.** Set `OPENPENCIL_EXTENSION_ALLOWED_IDS` to a
//!   comma-separated list of extension ids (the 32-character `a`–`p`
//!   value, without the `chrome-extension://` prefix) and ONLY those ids
//!   pass; every other extension origin is refused as `ForeignOrigin`.
//!   Once the extension ships with a stable id this is how a deployment
//!   locks the route to it.
//!
//! Either way the reply's `Access-Control-Allow-Origin` echoes the ONE
//! origin that was accepted ([`cors_origin_for`]) — never `*`, so an
//! extension that is not the accepted caller cannot read this endpoint's
//! answers even when the browser lets it issue the request.
//!
//! The paid intelligent-design route is stricter. Its boundary and `OPTIONS`
//! response admit any well-formed extension origin only so an unpaired caller
//! can read the handler's `extensionNotPaired` fallback signal. The handler
//! queues model work only when this variable is explicitly non-empty and the
//! caller's id matches it; snapshot ingress's open mode is never reused.

use std::fmt;
use std::sync::OnceLock;

/// Env var pinning which browser-extension ids may reach the snapshot
/// ingress. Unset (or empty) means "any well-formed extension origin" —
/// see the module doc for why that is the default.
pub(super) const EXTENSION_ALLOWLIST_ENV: &str = "OPENPENCIL_EXTENSION_ALLOWED_IDS";

/// JSON-RPC error code for a refused request. Server-defined range
/// (-32000..=-32099), one step away from the -32000 this endpoint already
/// uses for "server busy" / "Invalid or missing session ID".
const DENIED_CODE: i32 = -32001;

/// Per-instance admission material for the live endpoint: the token the
/// server published and the port it actually bound. Shared by every
/// connection thread (`Arc`), immutable for the life of the server.
pub(super) struct LiveAdmission {
    token: String,
    port: u16,
}

impl LiveAdmission {
    pub(super) fn new(token: String, port: u16) -> Self {
        Self { token, port }
    }

    /// The per-instance identity token — also what the `ping` reply and
    /// the `openpencil/shutdown` check use, so the wire contract the CLI
    /// already knows is preserved verbatim.
    pub(super) fn token(&self) -> &str {
        &self.token
    }

    pub(super) fn port(&self) -> u16 {
        self.port
    }
}

/// Why a request was refused. A typed enum rather than a `String` (the
/// workspace rule) — and deliberately NOT an `McpLiveError` variant: every
/// value here is a *client* fault answered on the wire with a 401/403 and
/// a JSON-RPC error body, never a server fault the accept loop logs and
/// turns into a 500.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AdmissionDenial {
    /// `Host` absent, not a numeric loopback literal, or carrying a port
    /// other than the one this server bound.
    ForeignHost,
    /// An `Origin` header that is not this instance's own loopback origin.
    ForeignOrigin,
}

impl AdmissionDenial {
    /// HTTP status line. Boundary failures are 403 (the caller is not
    /// allowed to talk to this endpoint at all); token failures are 401
    /// (the caller may retry with the credential it was given).
    pub(super) fn http_status(self) -> &'static str {
        match self {
            AdmissionDenial::ForeignHost | AdmissionDenial::ForeignOrigin => "403 Forbidden",
        }
    }

    /// Client-facing reason. Intentionally coarse: it names the gate, not
    /// which byte of the token differed.
    pub(super) fn message(self) -> &'static str {
        match self {
            AdmissionDenial::ForeignHost => {
                "live MCP endpoint accepts loopback requests only (bad Host header)"
            }
            AdmissionDenial::ForeignOrigin => {
                "live MCP endpoint refuses cross-origin requests (bad Origin header)"
            }
        }
    }
}

impl fmt::Display for AdmissionDenial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

/// Gate 1 — browser screening, applied to EVERY request (including the
/// stateless probes and the REST document-sync route) before any routing.
///
/// Two narrow paths widen the `Origin` rule because the OpenPencil Chrome
/// extension cannot present this instance's loopback origin. Snapshot ingress
/// accepts a well-formed extension origin for insert-only capture. The paid
/// design route admits the same origin shape only to return a readable pairing
/// result, then requires an explicit id match before it queues model work.
/// Every other capability keeps the strict same-origin rule.
pub(super) fn check_boundary(
    req: &crate::mcp_serve::HttpRequest,
    admission: &LiveAdmission,
) -> Result<(), AdmissionDenial> {
    if !host_allowed(req.host.as_deref(), admission.port()) {
        return Err(AdmissionDenial::ForeignHost);
    }
    let origin = req.origin.as_deref();
    let extension_ingest = super::snapshot_ingest::is_snapshot_ingest_path(&req.path)
        && is_browser_extension_origin(origin);
    // The design route lets any syntactically valid Chrome extension reach
    // its own handler so an unpaired extension receives a readable,
    // origin-scoped `extensionNotPaired` response and can fall back locally.
    // The handler still requires an explicit non-empty allowlist match before
    // it queues any work.
    let extension_design = super::design_md_route::is_design_md_path(&req.path)
        && extension_origin_id(origin).is_some();
    if !origin_allowed(origin, admission.port()) && !extension_ingest && !extension_design {
        return Err(AdmissionDenial::ForeignOrigin);
    }
    Ok(())
}

/// JSON-RPC error body for a refused `/mcp` request, echoing the caller's
/// request id so a client correlates the refusal with its call instead of
/// hanging (same discipline as `op_mcp::parser`'s parse-failure path).
pub(super) fn denial_json_rpc(request_body: &str, denial: AdmissionDenial) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","error":{{"code":{DENIED_CODE},"message":"{}"}},"id":{}}}"#,
        crate::mcp_serve::json_escape(denial.message()),
        request_id_raw(request_body)
    )
}

/// The caller's top-level JSON-RPC `id`, verbatim (so a string id stays a
/// string id), or `null` when the body is not a JSON object / has no id.
fn request_id_raw(request_body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(request_body)
        .ok()
        .and_then(|value| value.get("id").map(|id| id.to_string()))
        .unwrap_or_else(|| "null".to_string())
}

/// `Host` must name a numeric loopback address. A DNS name — including
/// `localhost` — is refused: rebinding attacks work precisely by pointing
/// a name at 127.0.0.1, and a browser writes the name it was given into
/// `Host`, so accepting names would leave the hole open.
///
/// The port is checked when present. It may be absent: `op`'s own
/// transport sends a bare `Host: 127.0.0.1`
/// (`op_rpc_transport::TcpJsonRpc::http_post_request`), and a browser can
/// never produce that against a non-80 port — it always writes the target
/// port it dialled. So "no port" identifies a non-browser client rather
/// than widening the browser surface.
fn host_allowed(host: Option<&str>, expected_port: u16) -> bool {
    // HTTP/1.1 requires `Host`; every browser and every client in this
    // repo sends it. Absent ⇒ refuse rather than guess.
    let Some(raw) = host else {
        return false;
    };
    let Some((host, port)) = split_authority(raw.trim()) else {
        return false;
    };
    is_numeric_loopback(host) && port.is_none_or(|port| port == expected_port)
}

/// Any `Origin` other than this instance's own loopback origin is refused.
/// `None` is the normal non-browser case (CLI / proxy) and passes — a page
/// cannot suppress the header on a cross-origin request, so "no Origin"
/// is not something an attacker page can claim.
fn origin_allowed(origin: Option<&str>, expected_port: u16) -> bool {
    let Some(origin) = origin else {
        return true;
    };
    let origin = origin.trim();
    // The live endpoint is plain HTTP on loopback, so only an `http://`
    // origin can possibly be it; `null` (sandboxed iframe / `file://`)
    // and any `https://` page fall through to a refusal.
    let Some(authority) = origin.strip_prefix("http://") else {
        return false;
    };
    // A real serialized origin is scheme + authority and nothing else.
    if authority.contains(['/', '@', '?', '#']) {
        return false;
    }
    let Some((host, port)) = split_authority(authority) else {
        return false;
    };
    is_numeric_loopback(host) && port.unwrap_or(80) == expected_port
}

/// The extension id inside a well-formed Chrome extension origin
/// (`chrome-extension://<id>`, where `<id>` is the 32-character `a`–`p`
/// identifier Chrome derives from the extension's key), or `None` when the
/// value is not one.
///
/// A web page cannot claim this origin: the browser writes `Origin` itself
/// and a page's origin is always its own scheme+authority, so this widens
/// the surface to installed extensions and nothing else. The shape is
/// validated strictly (exact length, exact alphabet, no path/query) rather
/// than by prefix, so `chrome-extension://x/../..` style values are refused.
pub(super) fn extension_origin_id(origin: Option<&str>) -> Option<&str> {
    let id = origin.map(str::trim)?.strip_prefix("chrome-extension://")?;
    let well_formed = id.len() == 32
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() && byte <= b'p');
    well_formed.then_some(id)
}

/// Whether `origin` is an extension origin this instance accepts on the
/// snapshot ingress. `allowlist` is the pinned id set: `None` = open mode
/// (any well-formed extension origin), `Some(ids)` = only those ids. Taken
/// as a parameter rather than read from the environment so both modes are
/// directly testable.
fn extension_origin_allowed(origin: Option<&str>, allowlist: Option<&[String]>) -> bool {
    let Some(id) = extension_origin_id(origin) else {
        return false;
    };
    match allowlist {
        None => true,
        Some(allowed) => allowed.iter().any(|candidate| candidate == id),
    }
}

/// The pinned extension-id set from [`EXTENSION_ALLOWLIST_ENV`], read once
/// per process (the env cannot change under a running server). `None` is
/// open mode: unset, blank, or nothing but separators.
fn extension_id_allowlist() -> Option<&'static [String]> {
    static ALLOWLIST: OnceLock<Option<Vec<String>>> = OnceLock::new();
    ALLOWLIST
        .get_or_init(|| {
            let raw = std::env::var(EXTENSION_ALLOWLIST_ENV).ok()?;
            let ids: Vec<String> = raw
                .split(',')
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .map(str::to_string)
                .collect();
            (!ids.is_empty()).then_some(ids)
        })
        .as_deref()
}

/// Accepted ONLY on the insert-only snapshot ingress — see
/// `check_boundary` and `snapshot_ingest`.
fn is_browser_extension_origin(origin: Option<&str>) -> bool {
    extension_origin_allowed(origin, extension_id_allowlist())
}

/// Whether a syntactically valid extension origin lacks the explicit pairing
/// the intelligent design route requires. `None`/loopback origins are local
/// non-browser callers and remain governed by the existing boundary.
pub(super) fn is_unpaired_extension_origin(origin: Option<&str>) -> bool {
    is_unpaired_extension_origin_with_allowlist(origin, extension_id_allowlist())
}

fn is_unpaired_extension_origin_with_allowlist(
    origin: Option<&str>,
    allowlist: Option<&[String]>,
) -> bool {
    extension_origin_id(origin).is_some()
        && !allowlist.is_some_and(|allowlist| extension_origin_allowed(origin, Some(allowlist)))
}

/// The `Access-Control-Allow-Origin` value this endpoint may echo back for
/// `req` — the ONE origin the boundary accepts for that exact request, or
/// `None` (emit no header at all).
///
/// Never `*`. A permissive wildcard on a loopback endpoint lets ANY browser
/// context that can reach the socket read the reply, which for the untokened
/// ingress would mean any installed extension, not just the accepted one.
/// `None` covers the non-browser callers (`op`, the MCP proxy), which send no
/// `Origin` and never look at CORS headers.
pub(super) fn cors_origin_for<'a>(
    req: &'a crate::mcp_serve::HttpRequest,
    admission: &LiveAdmission,
) -> Option<&'a str> {
    let origin = req.origin.as_deref()?.trim();
    if origin_allowed(Some(origin), admission.port()) {
        return Some(origin);
    }
    // Same widening as `check_boundary`, and no wider: an extension origin
    // is echoed only on the route it is allowed to reach.
    if super::snapshot_ingest::is_snapshot_ingest_path(&req.path)
        && is_browser_extension_origin(Some(origin))
    {
        return Some(origin);
    }
    // Echo a well-formed extension origin on the exact design route even when
    // it is not paired. That makes the handler's 403 readable to the extension
    // without widening CORS to any other route or origin shape.
    if super::design_md_route::is_design_md_path(&req.path)
        && extension_origin_id(Some(origin)).is_some()
    {
        return Some(origin);
    }
    None
}

/// Split an HTTP authority (`127.0.0.1:3100`, `127.0.0.1`, `[::1]:3100`)
/// into host and optional port. A malformed port refuses the whole value.
fn split_authority(value: &str) -> Option<(&str, Option<u16>)> {
    if let Some(rest) = value.strip_prefix('[') {
        let (inside, tail) = rest.split_once(']')?;
        return match tail {
            "" => Some((inside, None)),
            tail => {
                let port = tail.strip_prefix(':')?.parse::<u16>().ok()?;
                Some((inside, Some(port)))
            }
        };
    }
    match value.rsplit_once(':') {
        // An unbracketed IPv6 literal lands here with a nonsense split;
        // `is_numeric_loopback` then rejects the truncated host, which is
        // correct — unbracketed IPv6 in an authority is malformed anyway.
        Some((host, port)) => Some((host, Some(port.parse::<u16>().ok()?))),
        None => Some((value, None)),
    }
}

/// A numeric IP literal in a loopback range (127.0.0.0/8 or `::1`).
/// Names never qualify.
fn is_numeric_loopback(host: &str) -> bool {
    host.parse::<std::net::IpAddr>()
        .is_ok_and(|address| address.is_loopback())
}

#[cfg(test)]
#[path = "admission_tests.rs"]
mod tests;
