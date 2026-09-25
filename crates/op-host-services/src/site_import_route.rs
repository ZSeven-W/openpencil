//! `POST /api/ai/site-import`: Studio Home's "import this website" for the
//! browser shell.
//!
//! The browser cannot fetch a third-party page (CORS, and it must not dial
//! arbitrary addresses on the user's network either), so the daemon runs the
//! SAME pipeline the desktop host runs — [`crate::site_import`]: the
//! SSRF-screened, size-capped fetcher, `op-html`, the brand kit, the
//! contract-tier-only finalize, colour binding, component recognition and the
//! closing audit — and returns the finished document for the browser to
//! install. Nothing is written into the daemon's own document here: the
//! browser swaps the result in and its normal live-sync push publishes it,
//! exactly as it does for every other Home deliverable.
//!
//! Request: `{"url": "https://acme.example", "locale": "zh-CN"}`. The URL goes
//! through [`op_editor_core::site_import_url`], the same link-only rule the
//! Home button uses, so the route accepts exactly what the button offers.
//!
//! Reply (200): `{"ok":true,"document":{…},"summary":{…},"report":{…}}`; any
//! failure is `{"ok":false,"error":"…"}` with a status that says whose fault
//! it was (400 request, 422 page, 429 busy, 502 fetch).

use op_editor_core::{Locale, SiteImportResult};

use crate::site_import_error::SiteImportError;

/// The route path, shared by the dispatcher and the browser shell.
pub const SITE_IMPORT_ROUTE: &str = "/api/ai/site-import";

/// A request is one URL and a locale tag; anything larger is not a request.
const MAX_REQUEST_BYTES: usize = 4 * 1024;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRequest {
    url: String,
    #[serde(default)]
    locale: Option<String>,
}

/// The parsed request: the normalized page URL and the report's locale.
fn parse_request(body: &str) -> Result<(String, Locale), &'static str> {
    if body.len() > MAX_REQUEST_BYTES {
        return Err("request too large");
    }
    let request: WireRequest = serde_json::from_str(body).map_err(|_| "invalid request")?;
    let url = op_editor_core::site_import_url(&request.url).ok_or("not a website link")?;
    let locale = request
        .locale
        .as_deref()
        .and_then(Locale::from_tag)
        .unwrap_or(Locale::EnUs);
    Ok((url, locale))
}

/// Serve one request through `run` (production: [`serve_screened`]; tests
/// inject an in-memory site). Returns `(status, body)`.
pub fn serve_with(
    body: &str,
    run: impl FnOnce(&str, Locale) -> Result<SiteImportResult, SiteImportError>,
) -> (&'static str, String) {
    let (url, locale) = match parse_request(body) {
        Ok(parsed) => parsed,
        Err(error) => return ("400 Bad Request", error_body(error)),
    };
    match run(&url, locale) {
        Ok(result) => match result_body(&result) {
            Ok(body) => ("200 OK", body),
            Err(error) => ("500 Internal Server Error", error_body(&error.to_string())),
        },
        Err(error) => (status_for(&error), error_body(&error.to_string())),
    }
}

/// Serve one request with the production, policy-screened fetcher. Blocking
/// (network + the whole pipeline): the dispatcher runs it on the
/// connection's own thread, never under the state lock.
pub fn serve_screened(body: &str) -> (&'static str, String) {
    serve_with(body, crate::site_import::import_site_screened)
}

fn status_for(error: &SiteImportError) -> &'static str {
    match error {
        SiteImportError::Busy => "429 Too Many Requests",
        SiteImportError::Fetch(_) => "502 Bad Gateway",
        SiteImportError::NotHtml { .. } | SiteImportError::NoContent { .. } => {
            "422 Unprocessable Entity"
        }
    }
}

fn result_body(result: &SiteImportResult) -> serde_json::Result<String> {
    serde_json::to_string(&serde_json::json!({
        "ok": true,
        "document": result.document,
        "summary": result.summary,
        "report": result.report,
    }))
}

fn error_body(error: &str) -> String {
    serde_json::json!({ "ok": false, "error": error }).to_string()
}

#[cfg(test)]
#[path = "site_import_route_tests.rs"]
mod tests;
