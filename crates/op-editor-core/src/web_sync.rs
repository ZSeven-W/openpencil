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

#[path = "web_sync_error.rs"]
mod web_sync_error;
pub use web_sync_error::WebSyncError;

/// A whole-document live-sync payload plus editor-only view state carried by
/// the response wrapper. These fields intentionally stay outside
/// [`PenDocument`] so older schema bindings remain valid.
#[derive(Debug)]
pub struct WebSyncDocument {
    pub document: PenDocument,
    pub version: u64,
    pub active_page_index: usize,
    pub preserve_authored_geometry: bool,
    /// The document's scene tag (`"slides"` marks a deck), when the daemon
    /// is new enough to send it. Older daemons omit the field — `None` then
    /// means "unknown", not "no scenario", so appliers keep their current
    /// value rather than clearing it.
    pub scenario: Option<crate::scene_template_catalog::TemplateScene>,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireEditorMeta {
    #[serde(default, alias = "active_page_index")]
    active_page_index: Option<usize>,
    #[serde(default, alias = "preserve_authored_geometry")]
    preserve_authored_geometry: Option<bool>,
}

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
    /// Editor-only metadata paired with `baseline_hash`. Kept separate so the
    /// existing document-only hash APIs retain their historical behavior.
    baseline_active_page_index: usize,
    baseline_preserve_authored_geometry: bool,
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

    /// Alias for [`applied_version`](Self::applied_version) in the bridge's
    /// vocabulary — the sync-gate baseline's server-side half — for callers
    /// that reason about "the last version this client knows about" rather
    /// than the pull-decision path.
    pub fn last_version(&self) -> u64 {
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
    pub fn next_document(&self, body: &str) -> Result<Option<(PenDocument, u64)>, WebSyncError> {
        self.next_document_with_metadata(body)
            .map(|next| next.map(|next| (next.document, next.version)))
    }

    /// Metadata-aware companion to [`next_document`](Self::next_document).
    /// `activePageIndex` and `preserveAuthoredGeometry` are additive top-level
    /// wrapper fields. Each top-level field independently overrides its nested
    /// `document.editorMeta` counterpart. Older daemons may omit both; nested
    /// metadata is then honored, otherwise the legacy `0` / `false` defaults
    /// apply.
    pub fn next_document_with_metadata(
        &self,
        body: &str,
    ) -> Result<Option<WebSyncDocument>, WebSyncError> {
        let value: serde_json::Value =
            serde_json::from_str(body).map_err(|e| WebSyncError::ResponseParse(e.to_string()))?;
        let version = value
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .ok_or(WebSyncError::MissingVersion)?;
        // Already up to date (and past the first sync) → nothing to apply.
        if self.initialized && version <= self.applied_version {
            return Ok(None);
        }
        let document = value.get("document").ok_or(WebSyncError::MissingDocument)?;
        let nested_meta = document
            .get("editorMeta")
            .and_then(|meta| serde_json::from_value::<WireEditorMeta>(meta.clone()).ok())
            .unwrap_or_default();
        let active_page_index = value
            .get("activePageIndex")
            .and_then(serde_json::Value::as_u64)
            .and_then(|index| usize::try_from(index).ok())
            .or(nested_meta.active_page_index)
            .unwrap_or(0);
        let preserve_authored_geometry = value
            .get("preserveAuthoredGeometry")
            .and_then(serde_json::Value::as_bool)
            .or(nested_meta.preserve_authored_geometry)
            .unwrap_or(false);
        let doc: PenDocument = serde_json::from_value(document.clone())
            .map_err(|e| WebSyncError::DocumentParse(e.to_string()))?;
        let scenario = value
            .get("scenario")
            .and_then(serde_json::Value::as_str)
            .and_then(|name| std::str::FromStr::from_str(name).ok());
        Ok(Some(WebSyncDocument {
            document: doc,
            version,
            active_page_index,
            preserve_authored_geometry,
            scenario,
        }))
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
    pub fn sync<F>(&mut self, body: &str, apply: F) -> Result<bool, WebSyncError>
    where
        F: FnOnce(PenDocument, u64) -> bool,
    {
        self.sync_with_metadata(body, |doc, version, _preserve_authored_geometry| {
            apply(doc, version)
        })
    }

    /// Metadata-aware companion to [`sync`](Self::sync). The geometry mode is
    /// committed together with the exact document/version by the caller's
    /// apply closure, avoiding a typed `PenDocument` replacement that silently
    /// resets Preserve-mode Figma imports.
    pub fn sync_with_metadata<F>(&mut self, body: &str, apply: F) -> Result<bool, WebSyncError>
    where
        F: FnOnce(PenDocument, u64, bool) -> bool,
    {
        self.sync_with_editor_meta(
            body,
            |doc, version, _active_page_index, preserve, _scenario| apply(doc, version, preserve),
        )
    }

    /// Full editor-metadata companion to [`sync`](Self::sync). Active page and
    /// authored-geometry mode are applied as one versioned snapshot while the
    /// preserve-only helper above remains source compatible.
    pub fn sync_with_editor_meta<F>(&mut self, body: &str, apply: F) -> Result<bool, WebSyncError>
    where
        F: FnOnce(
            PenDocument,
            u64,
            usize,
            bool,
            Option<crate::scene_template_catalog::TemplateScene>,
        ) -> bool,
    {
        match self.next_document_with_metadata(body)? {
            Some(next) => {
                let version = next.version;
                if apply(
                    next.document,
                    version,
                    next.active_page_index,
                    next.preserve_authored_geometry,
                    next.scenario,
                ) {
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
    pub fn build_push_body(doc: &PenDocument) -> Result<String, WebSyncError> {
        let doc_json = serde_json::to_string(doc)
            .map_err(|e| WebSyncError::SerializeDocument(e.to_string()))?;
        Ok(Self::wrap_push_body(&doc_json))
    }

    /// Wrap an ALREADY-serialized document into the push body shape. The glue
    /// serializes once for the hash check and reuses the same string here.
    pub fn wrap_push_body(doc_json: &str) -> String {
        format!(r#"{{"document":{doc_json}}}"#)
    }

    /// Same shape as [`wrap_push_body`](Self::wrap_push_body) plus a
    /// `baseVersion` field carrying the sync-gate baseline's server version —
    /// the daemon's optimistic-concurrency check for a conditional push (see
    /// [`parse_push_conflict`](Self::parse_push_conflict) for the rejection shape).
    pub fn wrap_push_body_with_base(doc_json: &str, base_version: u64) -> String {
        format!(r#"{{"document":{doc_json},"baseVersion":{base_version}}}"#)
    }

    /// Metadata-aware conditional push. The additive wrapper field is ignored
    /// by older daemons and lets newer daemons preserve Figma-authored absolute
    /// geometry without changing the canonical document schema.
    pub fn wrap_push_body_with_base_and_preserve(
        doc_json: &str,
        base_version: u64,
        preserve_authored_geometry: bool,
    ) -> String {
        format!(
            r#"{{"document":{doc_json},"baseVersion":{base_version},"preserveAuthoredGeometry":{preserve_authored_geometry}}}"#
        )
    }

    /// Full editor-metadata conditional push. The fields remain additive and
    /// can be ignored by older daemons.
    pub fn wrap_push_body_with_base_and_editor_meta(
        doc_json: &str,
        base_version: u64,
        active_page_index: usize,
        preserve_authored_geometry: bool,
    ) -> String {
        Self::wrap_push_body_with_base_editor_meta_and_mode(
            doc_json,
            base_version,
            active_page_index,
            preserve_authored_geometry,
            false,
        )
    }

    /// Full editor metadata with an optional metadata-only hint. New daemons
    /// use the hint to avoid replacing an identical document (and creating a
    /// remote undo step) for a page-only change; older daemons ignore it.
    pub fn wrap_push_body_with_base_editor_meta_and_mode(
        doc_json: &str,
        base_version: u64,
        active_page_index: usize,
        preserve_authored_geometry: bool,
        metadata_only: bool,
    ) -> String {
        let metadata_only_field = if metadata_only {
            r#","metadataOnly":true"#
        } else {
            ""
        };
        format!(
            r#"{{"document":{doc_json},"baseVersion":{base_version},"activePageIndex":{active_page_index},"preserveAuthoredGeometry":{preserve_authored_geometry}{metadata_only_field}}}"#
        )
    }

    /// True once the first daemon document has been applied. The push path is
    /// gated on this: the host page may reset the daemon document on refresh,
    /// but the browser must still pull that authoritative post-reset state
    /// before it can push local edits.
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
        self.baseline_active_page_index = 0;
        self.baseline_preserve_authored_geometry = false;
    }

    /// Record the document plus its live-sync wrapper metadata as the applied
    /// baseline. Unlike [`note_applied_snapshot`](Self::note_applied_snapshot),
    /// this detects a Preserve-mode change even when the typed document bytes
    /// are identical.
    pub fn note_applied_snapshot_with_metadata(
        &mut self,
        doc_json: &str,
        preserve_authored_geometry: bool,
    ) {
        self.note_applied_snapshot_with_editor_meta(doc_json, 0, preserve_authored_geometry);
    }

    /// Record the document and complete editor metadata as the applied
    /// baseline.
    pub fn note_applied_snapshot_with_editor_meta(
        &mut self,
        doc_json: &str,
        active_page_index: usize,
        preserve_authored_geometry: bool,
    ) {
        self.baseline_hash = Some(fnv1a64(doc_json.as_bytes()));
        self.note_applied_editor_meta(active_page_index, preserve_authored_geometry);
    }

    /// Update only the editor-metadata half of the baseline. Pulling an
    /// oversized document may intentionally skip serializing its bytes, but
    /// must still baseline these small fields or every tick would treat them
    /// as a new local edit.
    pub fn note_applied_editor_meta(
        &mut self,
        active_page_index: usize,
        preserve_authored_geometry: bool,
    ) {
        self.baseline_active_page_index = active_page_index;
        self.baseline_preserve_authored_geometry = preserve_authored_geometry;
    }

    /// Record a daemon-installed snapshot when its typed-document byte hash is
    /// not available without a second large serialization. The sync gate owns
    /// the exact generation/revision baseline; leaving the content hash
    /// unknown guarantees the next real content edit pushes, while the scalar
    /// metadata baseline prevents an immediate no-op echo.
    pub fn note_applied_snapshot_without_hash(
        &mut self,
        active_page_index: usize,
        preserve_authored_geometry: bool,
    ) {
        self.baseline_hash = None;
        self.note_applied_editor_meta(active_page_index, preserve_authored_geometry);
    }

    /// Cheap metadata-only change check used before deciding whether a
    /// same-revision page switch needs a push. This avoids serializing the
    /// document merely to compare two scalar editor fields.
    pub fn editor_meta_needs_push(
        &self,
        active_page_index: usize,
        preserve_authored_geometry: bool,
    ) -> bool {
        self.initialized
            && (self.baseline_active_page_index != active_page_index
                || self.baseline_preserve_authored_geometry != preserve_authored_geometry)
    }

    /// True when the locally-serialized document differs from the last
    /// applied/pushed baseline (and the first daemon sync has happened).
    pub fn should_push(&self, doc_json: &str) -> bool {
        self.initialized && self.baseline_hash != Some(fnv1a64(doc_json.as_bytes()))
    }

    /// Metadata-aware push check for the current live-sync wrapper.
    pub fn should_push_with_metadata(
        &self,
        doc_json: &str,
        preserve_authored_geometry: bool,
    ) -> bool {
        self.should_push_with_editor_meta(doc_json, 0, preserve_authored_geometry)
    }

    /// Full editor-metadata push check for the current live-sync wrapper.
    pub fn should_push_with_editor_meta(
        &self,
        doc_json: &str,
        active_page_index: usize,
        preserve_authored_geometry: bool,
    ) -> bool {
        self.initialized
            && (self.baseline_hash != Some(fnv1a64(doc_json.as_bytes()))
                || self.editor_meta_needs_push(active_page_index, preserve_authored_geometry))
    }

    /// Bootstrap pushes are intentionally disabled. A refreshed web page first
    /// asks the daemon to reset its transient sync document, then pulls the
    /// daemon's authoritative starter document; local edits may push only after
    /// that first pull has succeeded.
    pub fn should_bootstrap_push(&self, _doc_json: &str, _starter_doc_json: &str) -> bool {
        false
    }

    /// Commit a successful push: the daemon accepted `doc_json` and assigned
    /// it `version`. Records the content baseline AND marks the version
    /// applied, so neither the version probe nor the document fetch ever
    /// echoes our own push back into the canvas.
    pub fn mark_pushed(&mut self, doc_json: &str, version: u64) {
        self.note_applied_snapshot(doc_json);
        self.mark_applied(version);
    }

    /// Metadata-aware companion to [`mark_pushed`](Self::mark_pushed).
    pub fn mark_pushed_with_metadata(
        &mut self,
        doc_json: &str,
        preserve_authored_geometry: bool,
        version: u64,
    ) {
        self.note_applied_snapshot_with_metadata(doc_json, preserve_authored_geometry);
        self.mark_applied(version);
    }

    /// Full editor-metadata companion to [`mark_pushed`](Self::mark_pushed).
    pub fn mark_pushed_with_editor_meta(
        &mut self,
        doc_json: &str,
        active_page_index: usize,
        preserve_authored_geometry: bool,
        version: u64,
    ) {
        self.note_applied_snapshot_with_editor_meta(
            doc_json,
            active_page_index,
            preserve_authored_geometry,
        );
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

    /// Parse a rejected `POST /api/mcp/document` push response's
    /// `version-conflict` shape (`{"ok":false,"error":"version-conflict","version":N}`)
    /// and return the daemon's current server version. `None` for an
    /// accepted push or any other rejection reason.
    pub fn parse_push_conflict(resp: &str) -> Option<u64> {
        let value: serde_json::Value = serde_json::from_str(resp).ok()?;
        if value.get("ok")?.as_bool() != Some(false) {
            return None;
        }
        if value.get("error")?.as_str()? != "version-conflict" {
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
#[path = "web_sync_editor_meta_tests.rs"]
mod editor_meta_tests;

#[cfg(test)]
#[path = "web_sync_tests.rs"]
mod tests;
