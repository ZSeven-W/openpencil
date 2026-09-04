//! Endpoint parsing, normalization and validation.
//!
//! This is the extension's outbound-destination guard. It is deliberately a
//! whitelist of loopback literals rather than a URL parser, for two
//! independent reasons:
//!
//! * The extension must never be talked into POSTing a capture of the page
//!   you are on (cookies, form values, and rasterized images included) to an
//!   arbitrary host. Restricting the parse is the guard; the manifest's
//!   loopback-only `host_permissions` is the backstop.
//! * The live editor's admission gate refuses any `Host` header that is not a
//!   numeric loopback literal (a DNS name is exactly what a rebinding attack
//!   supplies), so `localhost:3100` would be answered with 403 even though it
//!   resolves to the same socket — hence the rewrite to `127.0.0.1`.

use crate::js_text::js_trim;

/// Endpoint the popup offers before the user has stored one.
pub const DEFAULT_ENDPOINT: &str = "127.0.0.1:3100";

/// The insert-only snapshot route on the desktop app's live MCP endpoint.
const INGEST_PATH: &str = "/api/import/web-snapshot";
/// The extension-only design-system generation route.
const DESIGN_MD_PATH: &str = "/api/generate/design-md";
/// The general MCP surface, used only as the unmanaged-daemon fallback.
const MCP_PATH: &str = "/mcp";

/// Hosts an endpoint may name. `localhost` is accepted and rewritten to the
/// IPv4 literal the live MCP listener and extension manifest both support.
const LOOPBACK_HOSTS: [&str; 2] = ["127.0.0.1", "localhost"];

/// Normalize a user-typed endpoint to `host:port`.
///
/// Accepts an optional `http://` / `https://` prefix and trailing slashes.
/// This intentionally narrows the JS predecessor's loopback regex to
/// `127.0.0.1` plus `localhost` rewritten to IPv4: the live MCP listener is
/// IPv4-only and Chrome cannot grant an IPv6 host match pattern.
///
/// Returns `None` when the input is unparseable or names anything but
/// loopback — the caller must refuse to send.
pub fn normalize_endpoint(raw: &str) -> Option<String> {
    let trimmed = js_trim(raw);
    if trimmed.is_empty() {
        return None;
    }
    let without_scheme = strip_trailing_slashes(strip_scheme(trimmed));

    // Split on the last colon. Any leftover colon in the host part fails the
    // IPv4-only whitelist below.
    let (host, port_text) = without_scheme.rsplit_once(':')?;

    // `[0-9]{1,5}` — no sign, no whitespace, no leading `+`, 1..=5 digits.
    if port_text.is_empty() || port_text.len() > 5 || !port_text.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let port: u32 = port_text.parse().ok()?;
    if !(1..=65535).contains(&port) {
        return None;
    }

    let matched = LOOPBACK_HOSTS
        .iter()
        .find(|candidate| candidate.eq_ignore_ascii_case(host))?;
    // The JS regex carried the `i` flag, so `LOCALHOST` was accepted; only
    // `localhost` has letters, so case folding matters for it alone.
    let host = if matched.eq_ignore_ascii_case("localhost") {
        "127.0.0.1"
    } else {
        *matched
    };
    Some(format!("{host}:{port}"))
}

/// Absolute URL of the desktop app's insert-only snapshot ingest route.
///
/// `endpoint` must already have passed [`normalize_endpoint`]; the caller
/// never has a reason to hold an un-normalized one.
pub fn ingest_url(endpoint: &str) -> String {
    format!("http://{endpoint}{INGEST_PATH}")
}

/// Absolute URL of the extension-only intelligent `design.md` route.
///
/// Like [`ingest_url`], `endpoint` must already have passed
/// [`normalize_endpoint`]. The route is deliberately narrower than `/mcp`:
/// generating a guide may spend the user's configured model quota, so the
/// desktop host applies its extension-pairing and concurrency policy here.
pub fn design_md_url(endpoint: &str) -> String {
    design_md_start_url(endpoint)
}

/// Absolute URL that starts an intelligent `design.md` job.
pub fn design_md_start_url(endpoint: &str) -> String {
    format!("http://{endpoint}{DESIGN_MD_PATH}")
}

/// Absolute URL that polls or cancels `job_id`.
///
/// The host emits exactly 32 lowercase hexadecimal characters. Refusing any
/// other spelling keeps the opaque id inside one URL path segment without
/// relying on JavaScript's URL encoder or permitting traversal delimiters.
pub fn design_md_poll_url(endpoint: &str, job_id: &str) -> Option<String> {
    crate::design_md_job::is_valid_job_id(job_id)
        .then(|| format!("http://{endpoint}{DESIGN_MD_PATH}/{job_id}"))
}

/// Absolute URL of the general MCP surface (the unmanaged-daemon fallback).
pub fn mcp_url(endpoint: &str) -> String {
    format!("http://{endpoint}{MCP_PATH}")
}

/// Drop a single leading `http://` or `https://`, case-insensitively.
fn strip_scheme(s: &str) -> &str {
    for scheme in ["http://", "https://"] {
        // `get` rather than a slice index: the prefix length can land inside
        // a multi-byte character, which would panic on `&s[..n]`.
        if s.get(..scheme.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(scheme))
        {
            return &s[scheme.len()..];
        }
    }
    s
}

fn strip_trailing_slashes(s: &str) -> &str {
    s.trim_end_matches('/')
}
