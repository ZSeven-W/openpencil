//! Client-side live-sync protocol for the Rust web shell — the mirror of the
//! web-canvas daemon's `/api/mcp/document` contract (`op-host-desktop`'s
//! `web_canvas_server`, the Rust analog of TS Nitro + `setSyncDocument`).
//!
//! The browser shell (`op-host-web`) polls the daemon's `GET /api/mcp/document`
//! (or, later, subscribes to its SSE stream), feeds each response body to
//! [`WebSyncClient::ingest_document_response`] to decide whether the live
//! document actually changed, and applies the returned document to its canvas;
//! local edits are pushed back with [`WebSyncClient::build_push_body`] over
//! `POST /api/mcp/document`.
//!
//! This module is the host-testable protocol CORE — version tracking, response
//! parsing, push-body shaping. The browser supplies only the thin `web_sys`
//! glue (fetch / setInterval / repaint) that can run solely in a browser, so
//! the decision logic here is exercised by ordinary host unit tests.

use jian_ops_schema::PenDocument;

/// Tracks the last document version this client has applied, so a poll/SSE
/// event only triggers a (re)load when the daemon's monotonic version actually
/// advanced — avoiding redundant document swaps + repaints.
#[derive(Debug, Default)]
pub struct WebSyncClient {
    applied_version: u64,
    initialized: bool,
}

impl WebSyncClient {
    pub fn new() -> Self {
        Self::default()
    }

    /// The last version applied (0 before the first apply).
    pub fn applied_version(&self) -> u64 {
        self.applied_version
    }

    /// Parse a `GET /api/mcp/document` response (`{document, version}`, the
    /// daemon's shape — see `web_canvas_server::handle_web_canvas_request`) and
    /// return the document + its version IFF it is newer than the last APPLIED
    /// version (or it's the first response); otherwise `None`. Errors on a
    /// malformed response.
    ///
    /// This is READ-ONLY: it does NOT advance the applied version. The caller
    /// applies the document, repaints, and only then calls
    /// [`mark_applied`](Self::mark_applied). That decide-then-commit split means
    /// a failed apply/repaint doesn't lose the update — the next poll re-offers
    /// the same (still-newer) version until it is committed.
    pub fn next_document(&self, body: &str) -> Result<Option<(PenDocument, u64)>, String> {
        let value: serde_json::Value =
            serde_json::from_str(body).map_err(|e| format!("sync response parse: {e}"))?;
        let version = value
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| "sync response missing numeric `version`".to_string())?;
        // Already up to date (and past the first sync) → nothing to apply.
        if self.initialized && version <= self.applied_version {
            return Ok(None);
        }
        let document = value
            .get("document")
            .ok_or_else(|| "sync response missing `document`".to_string())?;
        let doc: PenDocument = serde_json::from_value(document.clone())
            .map_err(|e| format!("sync response document parse: {e}"))?;
        Ok(Some((doc, version)))
    }

    /// Record that `version` has been applied AND repainted. Call this only
    /// after a successful apply+repaint so a failed repaint leaves the version
    /// un-committed (and thus retried on the next poll). Prefer [`sync`](Self::sync),
    /// which commits the version atomically and only on success.
    pub fn mark_applied(&mut self, version: u64) {
        self.applied_version = version;
        self.initialized = true;
    }

    /// Decide-then-commit in one call, so a stale version can NEVER be committed
    /// by construction: parse `body`; if it carries a newer document, invoke
    /// `apply` with `(document, version)` to apply + repaint it, and commit that
    /// EXACT version IFF `apply` returns `true` (apply + repaint succeeded).
    /// Returns `Ok(true)` when a newer document was applied+committed,
    /// `Ok(false)` when nothing newer arrived or `apply` reported failure (then
    /// the version stays un-committed and is retried next poll). The committed
    /// version is always the one just applied+painted — never a stale/newer one.
    pub fn sync<F>(&mut self, body: &str, apply: F) -> Result<bool, String>
    where
        F: FnOnce(PenDocument, u64) -> bool,
    {
        match self.next_document(body)? {
            Some((doc, version)) => {
                if apply(doc, version) {
                    self.mark_applied(version);
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            None => Ok(false),
        }
    }

    /// Build the `POST /api/mcp/document` request body for pushing a local edit
    /// to the daemon — the `{document}` wrapper it expects (mirrors the TS web
    /// app's `setSyncDocument` push shape).
    pub fn build_push_body(doc: &PenDocument) -> Result<String, String> {
        let doc_json = serde_json::to_string(doc).map_err(|e| e.to_string())?;
        Ok(format!(r#"{{"document":{doc_json}}}"#))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const V3: &str = r#"{"document":{"version":"1.0","children":[]},"version":3}"#;

    #[test]
    fn next_document_offers_the_first_response() {
        let c = WebSyncClient::new();
        assert!(matches!(c.next_document(V3), Ok(Some((_, 3)))));
        // Read-only: not committed until mark_applied.
        assert_eq!(c.applied_version(), 0);
    }

    #[test]
    fn mark_applied_then_skips_equal_or_older_versions() {
        let mut c = WebSyncClient::new();
        assert!(c.next_document(V3).expect("ok").is_some());
        c.mark_applied(3);
        // Same version → nothing newer.
        assert!(c.next_document(V3).expect("ok").is_none());
        // Older version → nothing newer.
        let older = r#"{"document":{"version":"1.0","children":[]},"version":2}"#;
        assert!(c.next_document(older).expect("ok").is_none());
        assert_eq!(c.applied_version(), 3);
    }

    #[test]
    fn next_document_offers_a_newer_version_after_commit() {
        let mut c = WebSyncClient::new();
        c.mark_applied(3);
        let newer = r#"{"document":{"version":"1.0","children":[]},"version":5}"#;
        assert!(matches!(c.next_document(newer), Ok(Some((_, 5)))));
    }

    #[test]
    fn uncommitted_version_is_re_offered_so_a_failed_repaint_is_not_lost() {
        // Decide-then-commit: if the caller does NOT mark_applied (e.g. repaint
        // failed), the same newer version must still be offered next poll.
        let c = WebSyncClient::new();
        assert!(c.next_document(V3).expect("ok").is_some());
        // No mark_applied → still offered.
        assert!(c.next_document(V3).expect("ok").is_some());
    }

    #[test]
    fn sync_commits_only_on_apply_success_and_never_stale() {
        let mut c = WebSyncClient::new();
        // apply succeeds → commits exactly the applied version (3).
        let mut applied_version = None;
        assert!(c
            .sync(V3, |_doc, v| {
                applied_version = Some(v);
                true
            })
            .expect("ok"));
        assert_eq!(applied_version, Some(3));
        assert_eq!(c.applied_version(), 3);
        // nothing newer → apply callback not invoked.
        let mut called = false;
        assert!(!c
            .sync(V3, |_d, _v| {
                called = true;
                true
            })
            .expect("ok"));
        assert!(!called);
        // newer, but apply (repaint) FAILS → NOT committed (stays 3), retried.
        let v5 = r#"{"document":{"version":"1.0","children":[]},"version":5}"#;
        assert!(!c.sync(v5, |_d, _v| false).expect("ok"));
        assert_eq!(c.applied_version(), 3);
        // retry succeeds → commits 5.
        assert!(c.sync(v5, |_d, _v| true).expect("ok"));
        assert_eq!(c.applied_version(), 5);
    }

    #[test]
    fn next_document_rejects_malformed_responses() {
        let c = WebSyncClient::new();
        assert!(c.next_document("not json").is_err());
        // Missing version.
        assert!(c
            .next_document(r#"{"document":{"version":"1.0","children":[]}}"#)
            .is_err());
        // Missing document on a first (would-apply) response.
        assert!(c.next_document(r#"{"version":1}"#).is_err());
    }

    #[test]
    fn build_push_body_wraps_the_document() {
        let doc: PenDocument =
            serde_json::from_str(r#"{"version":"1.0","children":[]}"#).expect("doc");
        let body = WebSyncClient::build_push_body(&doc).expect("body");
        assert!(body.starts_with(r#"{"document":"#), "{body}");
        assert!(body.contains(r#""version":"1.0""#), "{body}");
        // Round-trips back through the daemon's request parser shape.
        let value: serde_json::Value = serde_json::from_str(&body).expect("valid json");
        assert!(value.get("document").is_some());
    }
}
