//! The JavaScript boundary. Type conversion only — no decisions.
//!
//! Kept deliberately thin so the logic underneath stays host-agnostic and
//! natively testable. Every export is either a plain scalar/string or a small
//! JSON document the popup's glue parses with `JSON.parse`; nothing here
//! hands JavaScript a structure it has to know the layout of by hand.
//!
//! Numbers cross as `f64` because that is what a JavaScript number is. The
//! conversions to the internal `u32` counters clamp rather than wrap, so a
//! nonsensical value from the JS side degrades into a refused transfer
//! instead of undefined behaviour.

use wasm_bindgen::prelude::*;

use crate::account::{self, Region};
use crate::delivery;
use crate::design_md;
use crate::design_md_job::{self, DesignMdReply, PollReply, StartReply};
use crate::endpoint;
use crate::filename;
use crate::hub;
use crate::hub_reply::{self, CreateReply};
use crate::ingress::{self, Reply};
use crate::op_export::{self, OpExport};
use crate::transfer;

/// Endpoint the popup pre-fills when the user has none stored.
#[wasm_bindgen(js_name = defaultEndpoint)]
pub fn default_endpoint() -> String {
    endpoint::DEFAULT_ENDPOINT.to_owned()
}

/// Normalize a user-typed endpoint to `host:port`, or `undefined` when it is
/// unparseable or names anything but loopback.
#[wasm_bindgen(js_name = normalizeEndpoint)]
pub fn normalize_endpoint(raw: &str) -> Option<String> {
    endpoint::normalize_endpoint(raw)
}

/// Absolute URL of the desktop app's snapshot ingest route.
#[wasm_bindgen(js_name = ingestUrl)]
pub fn ingest_url(endpoint: &str) -> String {
    endpoint::ingest_url(endpoint)
}

/// Absolute URL of the paired desktop host's intelligent `design.md` route.
#[wasm_bindgen(js_name = designMdUrl)]
pub fn design_md_url(endpoint: &str) -> String {
    endpoint::design_md_url(endpoint)
}

/// URL that starts a short-lived intelligent generation job.
#[wasm_bindgen(js_name = designMdStartUrl)]
pub fn design_md_start_url(endpoint: &str) -> String {
    endpoint::design_md_start_url(endpoint)
}

/// URL that polls or cancels a validated opaque job id.
#[wasm_bindgen(js_name = designMdPollUrl)]
pub fn design_md_poll_url(endpoint: &str, job_id: &str) -> Option<String> {
    endpoint::design_md_poll_url(endpoint, job_id)
}

/// Absolute URL of the `/mcp` fallback.
#[wasm_bindgen(js_name = mcpUrl)]
pub fn mcp_url(endpoint: &str) -> String {
    endpoint::mcp_url(endpoint)
}

/// Request body for the `/mcp` fallback, minus the snapshot.
///
/// The caller splits this on [`snapshot_placeholder`] and joins the two
/// halves around `JSON.stringify(snapshot)`. The snapshot deliberately never
/// crosses this boundary — see [`ingress::mcp_envelope_template`].
#[wasm_bindgen(js_name = mcpEnvelopeTemplate)]
pub fn mcp_envelope_template() -> String {
    ingress::mcp_envelope_template()
}

/// The marker inside [`mcp_envelope_template`] to split on.
#[wasm_bindgen(js_name = snapshotPlaceholder)]
pub fn snapshot_placeholder() -> String {
    ingress::SNAPSHOT_PLACEHOLDER.to_owned()
}

/// Download file name for the `.op` document of a page titled `title`.
#[wasm_bindgen(js_name = opFilename)]
pub fn op_filename(title: &str) -> String {
    filename::op_filename(title)
}

/// Conventional fixed download name for an extracted design system.
#[wasm_bindgen(js_name = designMdFilename)]
pub fn design_md_filename() -> String {
    filename::design_md_filename().to_owned()
}

/// Convert bounded design evidence into a deterministic local `design.md`.
///
/// The returned JSON always carries all three fields:
/// `{"ok":true,"markdown":"…","error":null}` or
/// `{"ok":false,"markdown":null,"error":"…"}`.
#[wasm_bindgen(js_name = evidenceToDesignMd)]
pub fn evidence_to_design_md(evidence_json: &str) -> String {
    let value = match design_md::evidence_to_design_md(evidence_json) {
        Ok(markdown) => serde_json::json!({
            "ok": true,
            "markdown": markdown,
            "error": null,
        }),
        Err(error) => serde_json::json!({
            "ok": false,
            "markdown": null,
            "error": error.to_string(),
        }),
    };
    value.to_string()
}

/// Legacy alias for one short job request, not the whole generation budget.
#[wasm_bindgen(js_name = designMdRequestTimeoutMs)]
pub fn design_md_request_timeout_ms() -> u32 {
    design_md_job::START_TIMEOUT_MS
}

#[wasm_bindgen(js_name = designMdStartTimeoutMs)]
pub fn design_md_start_timeout_ms() -> u32 {
    design_md_job::START_TIMEOUT_MS
}

#[wasm_bindgen(js_name = designMdPollTimeoutMs)]
pub fn design_md_poll_timeout_ms() -> u32 {
    design_md_job::POLL_TIMEOUT_MS
}

#[wasm_bindgen(js_name = designMdPollIntervalMs)]
pub fn design_md_poll_interval_ms() -> u32 {
    design_md_job::POLL_INTERVAL_MS
}

#[wasm_bindgen(js_name = designMdTotalBudgetMs)]
pub fn design_md_total_budget_ms() -> u32 {
    design_md_job::TOTAL_BUDGET_MS
}

/// Maximum UTF-8 evidence body accepted by the local and intelligent paths.
#[wasm_bindgen(js_name = maxDesignEvidenceBytes)]
pub fn max_design_evidence_bytes() -> u32 {
    design_md::MAX_EVIDENCE_BYTES as u32
}

/// Maximum UTF-8 response envelope the client may read from the model route.
#[wasm_bindgen(js_name = maxDesignMdReplyBytes)]
pub fn max_design_md_reply_bytes() -> u32 {
    design_md_job::MAX_POLL_REPLY_BYTES as u32
}

#[wasm_bindgen(js_name = maxDesignMdStartReplyBytes)]
pub fn max_design_md_start_reply_bytes() -> u32 {
    design_md_job::MAX_START_REPLY_BYTES as u32
}

#[wasm_bindgen(js_name = maxDesignMdPollReplyBytes)]
pub fn max_design_md_poll_reply_bytes() -> u32 {
    design_md_job::MAX_POLL_REPLY_BYTES as u32
}

#[wasm_bindgen(js_name = maxDesignMdPendingReplyBytes)]
pub fn max_design_md_pending_reply_bytes() -> u32 {
    design_md_job::MAX_PENDING_REPLY_BYTES as u32
}

#[wasm_bindgen(js_name = maxDesignMdPollAttempts)]
pub fn max_design_md_poll_attempts() -> u32 {
    design_md_job::MAX_POLL_ATTEMPTS
}

#[wasm_bindgen(js_name = maxDesignMdTotalReplyBytes)]
pub fn max_design_md_total_reply_bytes() -> u32 {
    design_md_job::MAX_TOTAL_REPLY_BYTES as u32
}

/// Whether an evidence body is over the shared 256 KiB cap.
#[wasm_bindgen(js_name = designEvidenceTooLarge)]
pub fn design_evidence_too_large(bytes: f64) -> bool {
    !bytes.is_finite() || bytes < 0.0 || bytes > design_md::MAX_EVIDENCE_BYTES as f64
}

/// Classify a reply from `POST /api/generate/design-md`.
///
/// Success is `{"outcome":"ok","markdown":"…","intelligent":true}`;
/// failure is `{"outcome":"error","code":"…","detail":"…"}`.
#[wasm_bindgen(js_name = classifyDesignMdReply)]
pub fn classify_design_md_reply(status: u16, text: &str) -> String {
    let value = match design_md_job::classify_reply(status, text) {
        DesignMdReply::Ready { markdown } => serde_json::json!({
            "outcome": "ok",
            "markdown": markdown,
            "intelligent": true,
        }),
        DesignMdReply::Failed { code, detail } => serde_json::json!({
            "outcome": "error",
            "code": code.as_str(),
            "detail": detail,
        }),
    };
    value.to_string()
}

/// Classify the short POST acknowledgement.
#[wasm_bindgen(js_name = classifyDesignMdStartReply)]
pub fn classify_design_md_start_reply(status: u16, text: &str) -> String {
    let value = match design_md_job::classify_start_reply(status, text) {
        StartReply::Pending {
            job_id,
            retry_after_ms,
        } => serde_json::json!({
            "outcome": "pending",
            "jobId": job_id,
            "retryAfterMs": retry_after_ms,
        }),
        StartReply::Failed { code, detail } => serde_json::json!({
            "outcome": "error",
            "code": code.as_str(),
            "detail": detail,
        }),
    };
    value.to_string()
}

/// Classify one short GET poll.
#[wasm_bindgen(js_name = classifyDesignMdPollReply)]
pub fn classify_design_md_poll_reply(status: u16, text: &str) -> String {
    let value = match design_md_job::classify_poll_reply(status, text) {
        PollReply::Pending { retry_after_ms } => serde_json::json!({
            "outcome": "pending",
            "retryAfterMs": retry_after_ms,
        }),
        PollReply::Ready { markdown } => serde_json::json!({
            "outcome": "ok",
            "markdown": markdown,
            "intelligent": true,
        }),
        PollReply::Failed { code, detail } => serde_json::json!({
            "outcome": "error",
            "code": code.as_str(),
            "detail": detail,
        }),
    };
    value.to_string()
}

/// Convert an extractor snapshot into a ready-to-open `.op` document.
///
/// Returns a small JSON document the popup glue parses with `JSON.parse`:
///
/// * `{"ok":true,"op":"<PenDocument JSON>","nodeCount":N,"warnings":[…]}` —
///   `op` is the exact text to write to a `.op` file.
/// * `{"ok":false,"error":"…"}` — the capture produced no importable content
///   (empty page, unsupported snapshot, malformed JSON); the caller must show
///   an actionable error instead of downloading a broken file.
///
/// `title`, when present and non-blank, names the produced document.
#[wasm_bindgen(js_name = snapshotToOpDocument)]
pub fn snapshot_to_op_document(snapshot_json: &str, title: Option<String>) -> String {
    let value = match op_export::snapshot_to_op(snapshot_json, title.as_deref()) {
        OpExport::Ready {
            op,
            node_count,
            warnings,
        } => serde_json::json!({
            "ok": true,
            "op": op,
            "nodeCount": node_count,
            "warnings": warnings,
        }),
        OpExport::Failed { error } => serde_json::json!({
            "ok": false,
            "error": error,
        }),
    };
    value.to_string()
}

/// Milliseconds before an in-flight request is aborted.
#[wasm_bindgen(js_name = requestTimeoutMs)]
pub fn request_timeout_ms() -> u32 {
    transfer::REQUEST_TIMEOUT_MS
}

/// Ingest-route body cap, in megabytes (used in the over-size message).
#[wasm_bindgen(js_name = maxSnapshotMb)]
pub fn max_snapshot_mb() -> u32 {
    transfer::MAX_SNAPSHOT_MB
}

/// Whether a snapshot of `chars` UTF-16 code units exceeds that cap.
#[wasm_bindgen(js_name = snapshotTooLarge)]
pub fn snapshot_too_large(chars: f64) -> bool {
    transfer::snapshot_too_large(chars)
}

/// Whether a built `/mcp` fallback envelope exceeds the endpoint's 64 MiB
/// whole-body cap (JSON re-escaping can inflate past the snapshot cap).
#[wasm_bindgen(js_name = mcpEnvelopeTooLarge)]
pub fn mcp_envelope_too_large(chars: f64) -> bool {
    transfer::mcp_envelope_too_large(chars)
}

/// Classify a reply from `POST /api/import/web-snapshot`. See
/// [`reply_to_json`] for the shape returned.
#[wasm_bindgen(js_name = classifyIngestReply)]
pub fn classify_ingest_reply(status: u16, text: &str) -> String {
    reply_to_json(&ingress::classify_ingest_reply(status, text))
}

/// Classify a reply from the `POST /mcp` fallback.
#[wasm_bindgen(js_name = classifyMcpReply)]
pub fn classify_mcp_reply(status: u16, text: &str) -> String {
    reply_to_json(&ingress::classify_mcp_reply(status, text))
}

/* ------------------------------------------------------- the account row */

/// Normalize a stored region value to `"cn"` or `"global"`.
#[wasm_bindgen(js_name = accountRegion)]
pub fn account_region(stored: &str) -> String {
    Region::from_stored(stored).as_str().to_owned()
}

/// The hub origin for `region`, unless `override_raw` names a loopback
/// development hub. See [`account::hub_origin`] for why that is the only
/// override shape accepted.
#[wasm_bindgen(js_name = hubOrigin)]
pub fn hub_origin(region: &str, override_raw: &str) -> String {
    account::hub_origin(Region::from_stored(region), override_raw)
}

/// The sign-in URL to OPEN IN A TAB. Never fetched — the SSO handshake is
/// the browser's, not the extension's.
#[wasm_bindgen(js_name = hubLoginUrl)]
pub fn hub_login_url(origin: &str) -> String {
    account::login_url(origin)
}

/// `GET` this with `credentials: 'include'` to learn who is signed in.
#[wasm_bindgen(js_name = hubSessionUrl)]
pub fn hub_session_url(origin: &str) -> String {
    account::session_url(origin)
}

/// `POST` this with the session's CSRF header to sign out.
#[wasm_bindgen(js_name = hubLogoutUrl)]
pub fn hub_logout_url(origin: &str) -> String {
    account::logout_url(origin)
}

/// The account page, opened in a tab when the extension cannot sign out
/// itself.
#[wasm_bindgen(js_name = hubAccountUrl)]
pub fn hub_account_url(origin: &str) -> String {
    account::account_url(origin)
}

/// The `host_permissions` match pattern `origin` needs.
///
/// The popup checks this with `chrome.permissions.contains` before its first
/// probe, so a manifest that does not grant the origin fails with a sentence
/// naming the origin rather than with an unexplained network error.
#[wasm_bindgen(js_name = hubHostPermission)]
pub fn hub_host_permission(origin: &str) -> String {
    account::host_permission(origin)
}

/// Classify a reply from `GET /api/v1/session`.
///
/// * `{"state":"signedIn","userId":…,"displayName":…,"avatarUrl":…|null,"csrfToken":…}`
/// * `{"state":"signedOut"}`
/// * `{"state":"error","detail":…}`
#[wasm_bindgen(js_name = parseSession)]
pub fn parse_session(status: u16, text: &str) -> String {
    account::session_to_json(&account::parse_session(status, text))
}

/// Re-validate an avatar URL read back out of `chrome.storage`, returning
/// `undefined` for anything that must not reach an `<img src>`.
#[wasm_bindgen(js_name = sanitizeAvatarUrl)]
pub fn sanitize_avatar_url(raw: &str) -> Option<String> {
    account::avatar_url(Some(raw))
}

/// Milliseconds before a hub request is abandoned.
#[wasm_bindgen(js_name = accountRequestTimeoutMs)]
pub fn account_request_timeout_ms() -> u32 {
    account::REQUEST_TIMEOUT_MS
}

/// Whether delivery to the signed-in account is implemented end to end.
#[wasm_bindgen(js_name = accountDeliveryAvailable)]
pub fn account_delivery_available() -> bool {
    delivery::ACCOUNT_AVAILABLE
}

/// The delivery target actually used for the next capture, given what the
/// user last chose and whether a session was observed.
#[wasm_bindgen(js_name = deliveryTarget)]
pub fn delivery_target(stored: &str, signed_in: bool) -> String {
    delivery::resolve(stored, signed_in).as_str().to_owned()
}

/// Whether the popup should render the delivery row at all.
#[wasm_bindgen(js_name = deliveryRowVisible)]
pub fn delivery_row_visible(signed_in: bool) -> bool {
    delivery::row_visible(signed_in)
}

/* ----------------------------------------------- the account snapshot inbox */

/// Absolute URL of the Hub's inbox create route on `origin`.
#[wasm_bindgen(js_name = hubSnapshotsUrl)]
pub fn hub_snapshots_url(origin: &str) -> String {
    hub::snapshots_url(origin)
}

/// Request body for `POST /api/v1/snapshots`, minus the snapshot document.
///
/// The caller splits this on [`hub_snapshot_placeholder`] and joins the two
/// halves around the extractor's raw JSON text. Unlike the `/mcp` envelope the
/// document goes in as a JSON **object**, so there is no `JSON.stringify`
/// around it; see [`hub::create_envelope_template`] for why splicing it
/// verbatim cannot forge a field.
///
/// `captured_at_ms` is `Date.now()` and `tz_offset_minutes` is
/// `-new Date().getTimezoneOffset()` — the latter only affects the local-time
/// stamp inside the display name, never the wire instant, which is always UTC.
#[wasm_bindgen(js_name = hubEnvelopeTemplate)]
pub fn hub_envelope_template(
    title: &str,
    source_url: &str,
    captured_at_ms: f64,
    tz_offset_minutes: f64,
) -> String {
    hub::create_envelope_template(title, source_url, captured_at_ms, tz_offset_minutes)
}

/// The marker inside [`hub_envelope_template`] to split on.
#[wasm_bindgen(js_name = hubSnapshotPlaceholder)]
pub fn hub_snapshot_placeholder() -> String {
    hub::SNAPSHOT_PLACEHOLDER.to_owned()
}

/// Whether a snapshot of `chars` UTF-16 code units cannot fit in the Hub's
/// 32 MiB request body once the envelope is wrapped around it.
#[wasm_bindgen(js_name = hubSnapshotTooLarge)]
pub fn hub_snapshot_too_large(chars: f64) -> bool {
    hub::snapshot_too_large(chars)
}

/// The Hub's own body cap, in megabytes (used in the account over-size
/// message; smaller than the local ingest cap).
#[wasm_bindgen(js_name = hubMaxSnapshotMb)]
pub fn hub_max_snapshot_mb() -> u32 {
    hub::HUB_MAX_SNAPSHOT_MB
}

/// Milliseconds before an upload to the Hub is abandoned.
#[wasm_bindgen(js_name = hubUploadTimeoutMs)]
pub fn hub_upload_timeout_ms() -> u32 {
    hub::UPLOAD_TIMEOUT_MS
}

/// Per-user ceiling on stored snapshots, for the "inbox is full" message.
#[wasm_bindgen(js_name = hubQuotaItems)]
pub fn hub_quota_items() -> u32 {
    hub::QUOTA_ITEMS
}

/// Per-user ceiling on stored bytes, in MiB, for the same message.
#[wasm_bindgen(js_name = hubQuotaTotalMb)]
pub fn hub_quota_total_mb() -> u32 {
    hub::QUOTA_TOTAL_MB
}

/// Classify a reply from `POST /api/v1/snapshots`.
///
/// * `{"outcome":"ok","id":…,"name":…,"bytes":N,"expiresAt":…}`
/// * `{"outcome":"error","code":"signedOut"|"forbidden"|"quota"|"tooLarge"|
///   "rateLimited"|"rejected"|"unavailable","detail":…,"retryAfterSeconds":N|null}`
#[wasm_bindgen(js_name = classifyHubReply)]
pub fn classify_hub_reply(status: u16, text: &str, retry_after: &str) -> String {
    let value = match hub_reply::classify_create_reply(status, text, retry_after) {
        CreateReply::Created {
            id,
            name,
            bytes,
            expires_at,
        } => serde_json::json!({
            "outcome": "ok",
            "id": id,
            "name": name,
            "bytes": bytes,
            "expiresAt": expires_at,
        }),
        CreateReply::Failed {
            code,
            detail,
            retry_after_seconds,
        } => serde_json::json!({
            "outcome": "error",
            "code": code.as_str(),
            "detail": detail,
            "retryAfterSeconds": retry_after_seconds,
        }),
    };
    value.to_string()
}

/// Plan and verify one chunked snapshot readback.
///
/// JavaScript drives this: while `!done`, it asks the tab for `nextLength`
/// code units at `nextOffset` and passes the answer to `accept`; when `done`,
/// it calls `verifyTotal` with the length of the joined string. A `false`
/// from either means `chunkLost`.
#[wasm_bindgen(js_name = ChunkPlan)]
pub struct JsChunkPlan {
    inner: transfer::ChunkPlan,
}

#[wasm_bindgen(js_class = ChunkPlan)]
impl JsChunkPlan {
    /// Plan a readback of `expected_chars` UTF-16 code units.
    #[wasm_bindgen(constructor)]
    pub fn new(expected_chars: f64) -> JsChunkPlan {
        JsChunkPlan {
            inner: transfer::ChunkPlan::new(clamp_u32(expected_chars)),
        }
    }

    /// Offset of the next slice to request.
    #[wasm_bindgen(getter, js_name = nextOffset)]
    pub fn next_offset(&self) -> u32 {
        self.inner.next_offset()
    }

    /// Maximum length of the next slice to request.
    #[wasm_bindgen(getter, js_name = nextLength)]
    pub fn next_length(&self) -> u32 {
        self.inner.next_length()
    }

    /// Code units accepted so far.
    #[wasm_bindgen(getter)]
    pub fn transferred(&self) -> u32 {
        self.inner.transferred()
    }

    /// Whether every expected code unit has arrived.
    #[wasm_bindgen(getter)]
    pub fn done(&self) -> bool {
        self.inner.done()
    }

    /// Whether the payload carries the extractor's `truncated` flag.
    #[wasm_bindgen(getter)]
    pub fn truncated(&self) -> bool {
        self.inner.truncated()
    }

    /// Record one slice. `chunk_chars` is the `String.prototype.length` the
    /// caller measured on the intact slice, and `-1` when the tab answered
    /// with something that was not a string.
    #[wasm_bindgen(js_name = accept)]
    pub fn accept(&mut self, chunk: &str, chunk_chars: f64) -> bool {
        self.inner.accept(chunk, chunk_chars)
    }

    /// Final integrity check against the length of the joined text.
    #[wasm_bindgen(js_name = verifyTotal)]
    pub fn verify_total(&self, total_chars: f64) -> bool {
        total_chars.is_finite()
            && total_chars >= 0.0
            && total_chars <= f64::from(u32::MAX)
            && self.inner.verify_total(total_chars as u32)
    }
}

/// Serialize a [`Reply`] for the popup.
///
/// * `{"outcome":"fallback"}` — try the next ingress.
/// * `{"outcome":"ok","nodeCount":N,"warnings":[…]}`
/// * `{"outcome":"error","code":"forbidden"|"import","detail":"…"}`
fn reply_to_json(reply: &Reply) -> String {
    let value = match reply {
        Reply::Fallback => serde_json::json!({ "outcome": "fallback" }),
        Reply::Imported {
            node_count,
            warnings,
        } => serde_json::json!({
            "outcome": "ok",
            "nodeCount": node_count,
            "warnings": warnings,
        }),
        Reply::Failed { code, detail } => serde_json::json!({
            "outcome": "error",
            "code": code.as_str(),
            "detail": detail,
        }),
    };
    value.to_string()
}

/// Clamp a JavaScript number into the `u32` counter space. NaN and negatives
/// become 0, which makes a nonsensical plan finish immediately and fail its
/// final length check rather than looping.
fn clamp_u32(value: f64) -> u32 {
    if !value.is_finite() || value <= 0.0 {
        0
    } else if value >= f64::from(u32::MAX) {
        u32::MAX
    } else {
        value as u32
    }
}
