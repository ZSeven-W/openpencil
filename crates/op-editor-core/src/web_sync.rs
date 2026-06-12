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
///
/// The PUSH side (browser edits → daemon, the TS `use-mcp-sync.ts`
/// `pushDocumentToServer` direction) is tracked as a content hash of the
/// locally-serialized document: [`note_applied_snapshot`](Self::note_applied_snapshot)
/// records the baseline after an external apply, [`should_push`](Self::should_push)
/// compares the live doc against it, and [`mark_pushed`](Self::mark_pushed)
/// commits a successful push (baseline + version) so the daemon's echo of our
/// own push is never re-fetched or re-applied.
#[derive(Debug, Default)]
pub struct WebSyncClient {
    applied_version: u64,
    initialized: bool,
    /// FNV-1a hash of the last document serialization this client either
    /// applied from the daemon or successfully pushed to it. `None` until the
    /// first sync — pushes are gated on `initialized` (daemon is the document
    /// authority at startup; TS instead pushes on `client:id` because the TS
    /// BROWSER is the authority — an architectural divergence, documented at
    /// the glue site).
    baseline_hash: Option<u64>,
}

/// FNV-1a 64-bit — tiny, dependency-free content hash for the push baseline.
/// Not cryptographic; only guards against pushing an unchanged document.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
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
        Ok(Self::wrap_push_body(&doc_json))
    }

    /// Wrap an ALREADY-serialized document into the push body shape. The glue
    /// serializes once for the hash check and reuses the same string here.
    pub fn wrap_push_body(doc_json: &str) -> String {
        format!(r#"{{"document":{doc_json}}}"#)
    }

    /// True once the first daemon document has been applied. The push path is
    /// gated on this: the daemon is the document authority at startup, so the
    /// browser must never push its boot-time starter document over it.
    pub fn initialized(&self) -> bool {
        self.initialized
    }

    /// True when a probed daemon `version` warrants fetching the document —
    /// the first sync, or any version newer than the last applied one. Drives
    /// the cheap `GET /api/mcp/version` probe so the (potentially large)
    /// document is only fetched when it actually changed.
    pub fn wants_version(&self, version: u64) -> bool {
        !self.initialized || version > self.applied_version
    }

    /// Parse the `GET /api/mcp/version` probe body (`{"version":N}`).
    pub fn parse_version_probe(body: &str) -> Option<u64> {
        let value: serde_json::Value = serde_json::from_str(body).ok()?;
        value.get("version")?.as_u64()
    }

    /// Record the local serialization of the document JUST applied from the
    /// daemon as the push baseline — the next [`should_push`](Self::should_push)
    /// then reports `false` until a real local edit changes the content.
    /// (The TS hook suppresses the same echo with a 200 ms `skipPushUntil`
    /// window; a content hash is race-free where a timer is heuristic.)
    pub fn note_applied_snapshot(&mut self, doc_json: &str) {
        self.baseline_hash = Some(fnv1a64(doc_json.as_bytes()));
    }

    /// True when the locally-serialized document differs from the last
    /// applied/pushed baseline (and the first daemon sync has happened).
    pub fn should_push(&self, doc_json: &str) -> bool {
        self.initialized && self.baseline_hash != Some(fnv1a64(doc_json.as_bytes()))
    }

    /// Commit a successful push: the daemon accepted `doc_json` and assigned
    /// it `version`. Records the content baseline AND marks the version
    /// applied, so neither the version probe nor the document fetch ever
    /// echoes our own push back into the canvas.
    pub fn mark_pushed(&mut self, doc_json: &str, version: u64) {
        self.note_applied_snapshot(doc_json);
        self.mark_applied(version);
    }

    /// Parse a `POST /api/mcp/document` push response (`{"ok":true,"version":N}`,
    /// the daemon's `document_sync_ok` shape). `None` on a rejected push.
    pub fn parse_push_response(body: &str) -> Option<u64> {
        let value: serde_json::Value = serde_json::from_str(body).ok()?;
        if value.get("ok")?.as_bool() != Some(true) {
            return None;
        }
        value.get("version")?.as_u64()
    }
}

/// Stable change-detection key for the selection-sync push (TS
/// `use-mcp-sync.ts` pushes when `selectedIds` / `activePageId` change). Ids
/// joined in selection order + the active page id, so a reorder or page
/// switch re-pushes. Never empty (the `sel:`/`page:` prefixes), so callers
/// can use an empty sentinel for "force next push".
pub fn selection_sync_key(state: &crate::EditorState) -> String {
    let ids: Vec<&str> = state.selection.set.iter().map(|id| id.as_str()).collect();
    format!(
        "sel:{}|page:{}",
        ids.join(","),
        active_page_id(state).unwrap_or_default()
    )
}

/// Build the `POST /api/mcp/selection` body — the exact TS renderer push
/// shape (`selection.post.ts`): `{selectedIds, activePageId}`. The TS
/// `sourceClientId` field is omitted: the Rust daemon has no client-id
/// concept (it is the single document authority, not a relay cache).
pub fn selection_push_body(state: &crate::EditorState) -> String {
    let ids: Vec<&str> = state.selection.set.iter().map(|id| id.as_str()).collect();
    let body = serde_json::json!({
        "selectedIds": ids,
        "activePageId": active_page_id(state),
    });
    serde_json::to_string(&body).unwrap_or_else(|_| r#"{"selectedIds":[]}"#.to_string())
}

/// The active page's id, when the document carries a pages array.
fn active_page_id(state: &crate::EditorState) -> Option<String> {
    state
        .doc
        .pages
        .as_ref()
        .and_then(|pages| pages.get(state.ui.active_page_index))
        .map(|page| page.id.clone())
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
        // The wrap helper produces the identical body from the same JSON.
        let doc_json = serde_json::to_string(&doc).expect("doc json");
        assert_eq!(WebSyncClient::wrap_push_body(&doc_json), body);
    }

    #[test]
    fn version_probe_gates_the_document_fetch() {
        let mut c = WebSyncClient::new();
        // First sync: any version (even 0) warrants a fetch.
        assert!(c.wants_version(0));
        c.mark_applied(3);
        assert!(!c.wants_version(2));
        assert!(!c.wants_version(3));
        assert!(c.wants_version(4));
        // Probe body parsing.
        assert_eq!(
            WebSyncClient::parse_version_probe(r#"{"version":7}"#),
            Some(7)
        );
        assert_eq!(WebSyncClient::parse_version_probe(r#"{"ok":false}"#), None);
        assert_eq!(WebSyncClient::parse_version_probe("not json"), None);
    }

    #[test]
    fn push_is_gated_on_first_sync_and_content_change() {
        let mut c = WebSyncClient::new();
        let starter = r#"{"version":"1.0","children":[]}"#;
        // Before the first daemon apply the browser must NOT push its
        // boot-time starter document over the daemon's (daemon authority).
        assert!(!c.initialized());
        assert!(!c.should_push(starter));
        // Apply the daemon doc, note its local serialization as baseline.
        assert!(c.sync(V3, |_d, _v| true).expect("ok"));
        c.note_applied_snapshot(starter);
        // Unchanged content → no push (the echo-suppression core).
        assert!(!c.should_push(starter));
        // A real local edit changes the serialization → push.
        let edited = r#"{"version":"1.0","children":[{"id":"n1"}]}"#;
        assert!(c.should_push(edited));
    }

    #[test]
    fn mark_pushed_commits_baseline_and_version_so_echo_is_skipped() {
        let mut c = WebSyncClient::new();
        assert!(c.sync(V3, |_d, _v| true).expect("ok"));
        let edited = r#"{"version":"1.0","children":[{"id":"n1"}]}"#;
        assert!(c.should_push(edited));
        // Daemon accepted the push as version 4.
        c.mark_pushed(edited, 4);
        assert_eq!(c.applied_version(), 4);
        // Neither the content nor the version is re-offered (no echo).
        assert!(!c.should_push(edited));
        assert!(!c.wants_version(4));
        let echo = r#"{"document":{"version":"1.0","children":[]},"version":4}"#;
        assert!(c.next_document(echo).expect("ok").is_none());
        // A later external version still syncs.
        assert!(c.wants_version(5));
    }

    #[test]
    fn push_response_parses_only_the_ok_shape() {
        assert_eq!(
            WebSyncClient::parse_push_response(r#"{"ok":true,"version":9}"#),
            Some(9)
        );
        assert_eq!(
            WebSyncClient::parse_push_response(r#"{"ok":false,"error":"x"}"#),
            None
        );
        assert_eq!(WebSyncClient::parse_push_response(r#"{"version":9}"#), None);
        assert_eq!(WebSyncClient::parse_push_response(""), None);
    }

    #[test]
    fn selection_key_and_body_track_ids_and_active_page() {
        let mut state = crate::EditorState::new();
        let key_empty = selection_sync_key(&state);
        assert_eq!(key_empty, "sel:|page:");
        state.doc.children = vec![];
        state.selection.set = vec![crate::NodeId::new("n1"), crate::NodeId::new("n2")];
        state.selection.anchor = crate::NodeId::new("n2");
        let key = selection_sync_key(&state);
        assert_eq!(key, "sel:n1,n2|page:");
        assert_ne!(key, key_empty);
        // Body matches the TS selection.post.ts renderer shape.
        let body: serde_json::Value =
            serde_json::from_str(&selection_push_body(&state)).expect("json");
        assert_eq!(body["selectedIds"], serde_json::json!(["n1", "n2"]));
        assert_eq!(body["activePageId"], serde_json::Value::Null);
    }

    #[test]
    fn selection_body_carries_the_active_page_id() {
        let doc: PenDocument = serde_json::from_str(
            r#"{"version":"1.0","children":[],"pages":[
                {"id":"p1","name":"One","children":[]},
                {"id":"p2","name":"Two","children":[]}
            ]}"#,
        )
        .expect("doc");
        let mut state = crate::EditorState::from_document(doc);
        assert!(selection_sync_key(&state).ends_with("|page:p1"));
        assert!(state.set_active_page(1));
        assert!(selection_sync_key(&state).ends_with("|page:p2"));
        let body: serde_json::Value =
            serde_json::from_str(&selection_push_body(&state)).expect("json");
        assert_eq!(body["activePageId"], "p2");
    }
}
