//! Live-canvas sync glue — bidirectional document + selection sync between
//! the browser shell and the web-canvas daemon. The Rust counterpart of the
//! TS `apps/web/src/hooks/use-mcp-sync.ts`:
//!
//! * **Pull** (external MCP/CLI writes → browser canvas): a 400 ms tick
//!   probes `GET /api/mcp/version` and fetches + applies the full document
//!   only when the daemon's monotonic version advanced. TS receives
//!   `document:update` pushes over SSE instead — a documented transport
//!   divergence (the daemon's SSE stream carries version bumps only, and the
//!   verified XHR machinery is reused); worst-case latency is one tick.
//! * **Push** (browser edits → daemon, so external MCP/CLI clients see them):
//!   a 2000 ms tick — the TS `PUSH_DEBOUNCE_MS` cadence — serializes the live
//!   document when the host flagged a possible change, skips when the content
//!   hash matches the last applied/pushed baseline (the race-free analog of
//!   TS's `skipPushUntilRef` + `sourceClientId` echo suppression), and POSTs
//!   `{document}` to `/api/mcp/document`. Pushes over 2 MiB are skipped with
//!   a one-shot console warning (TS `SYNC_MAX_BODY_BYTES` parity). Failures
//!   are dropped best-effort (TS catch{} parity) — the next local edit
//!   retriggers.
//! * **Selection push**: the 400 ms tick also samples the selection key and
//!   POSTs `{selectedIds, activePageId}` to `/api/mcp/selection` when it
//!   changed (TS debounces 300 ms; a 400 ms trailing sample is the same
//!   order of latency). One-way browser → daemon, exactly like TS.
//!
//! Architectural divergence (documented): TS's BROWSER is the document
//! authority (it pushes its document on `client:id` and the Nitro server only
//! caches), while the Rust daemon is the authority after mount — so this glue
//! never pushes before the first daemon document has been applied. The static
//! host page calls `/api/mcp/sync-reset` before mounting so a browser refresh
//! starts from the starter document instead of replaying the previous transient
//! web `.op` state; bootstrap pushes remain disabled so stale page state cannot
//! overwrite a deliberately opened daemon document. The TS browser-rendered
//! `screenshot:request` RPC is not implemented on the Rust web shell yet (the
//! daemon reports screenshots honestly unavailable headless).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use op_editor_core::web_sync::{self, WebSyncClient};

use crate::live_sync;
use crate::repaint_ctx::RepaintContext;

/// Pull cadence (version probe). TS gets SSE pushes; one tick of latency.
const POLL_INTERVAL_MS: i32 = 400;
/// Push cadence — TS `PUSH_DEBOUNCE_MS` (use-mcp-sync.ts:6).
const PUSH_INTERVAL_MS: i32 = 2000;
/// TS `SYNC_MAX_BODY_BYTES` (use-mcp-sync.ts:10): documents larger than this
/// are not pushed (warned once), mirroring the renderer's oversize guard.
const SYNC_MAX_BODY_BYTES: usize = 2 * 1024 * 1024;

/// Wire the bidirectional sync loops onto the mounted shell. Called once from
/// `mount()`; both intervals run for the page lifetime.
pub(crate) fn start<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    let base = crate::daemon_base::daemon_base();
    let sync = Rc::new(RefCell::new(WebSyncClient::new()));
    // One document fetch / one push at a time; ticks observing an in-flight
    // request skip (the TS hook queues at most one — same effective shape).
    let fetch_busy = Rc::new(Cell::new(false));
    let push_busy = Rc::new(Cell::new(false));
    let oversize_warned = Rc::new(Cell::new(false));
    // `None` forces a selection re-push on the next tick (used after a doc
    // apply, which resets daemon-side selection). Seeded with the current key
    // so mount does not push an initial no-op selection (TS pushes selection
    // only on change).
    let last_selection_key = Rc::new(RefCell::new(Some(web_sync::selection_sync_key(
        inner.borrow().host().editor_state(),
    ))));

    // ---- pull + selection tick ----
    {
        let inner = inner.clone();
        let sync = sync.clone();
        let base = base.clone();
        let fetch_busy = fetch_busy.clone();
        let last_selection_key = last_selection_key.clone();
        let tick: Rc<dyn Fn()> = Rc::new(move || {
            poll_version(&inner, &base, &sync, &fetch_busy, &last_selection_key);
            push_selection_if_changed(&inner, &base, &last_selection_key);
        });
        let _ = live_sync::start_interval(POLL_INTERVAL_MS, tick);
    }

    // ---- document push tick ----
    {
        let inner = inner.clone();
        let tick: Rc<dyn Fn()> = Rc::new(move || {
            push_document_if_changed(&inner, &base, &sync, &push_busy, &oversize_warned);
        });
        let _ = live_sync::start_interval(PUSH_INTERVAL_MS, tick);
    }
}

/// Probe the daemon version; on a newer version fetch + apply the document.
fn poll_version<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    base: &str,
    sync: &Rc<RefCell<WebSyncClient>>,
    fetch_busy: &Rc<Cell<bool>>,
    last_selection_key: &Rc<RefCell<Option<String>>>,
) {
    if fetch_busy.get() {
        return;
    }
    let inner = inner.clone();
    let sync = sync.clone();
    let base_owned = base.to_string();
    let fetch_busy = fetch_busy.clone();
    let last_selection_key = last_selection_key.clone();
    let on_version: Rc<dyn Fn(String)> = Rc::new(move |body: String| {
        let Some(version) = WebSyncClient::parse_version_probe(&body) else {
            return; // daemon down / non-JSON error body — retry next tick
        };
        let wants_version = sync
            .try_borrow()
            .map(|sync| sync.wants_version(version))
            .unwrap_or(false);
        if !wants_version {
            return;
        }
        // Fetch the full document; the latch is released when the response
        // lands (or never taken if the request can't start).
        fetch_busy.set(true);
        let inner = inner.clone();
        let sync = sync.clone();
        let fetch_busy_done = fetch_busy.clone();
        let last_selection_key = last_selection_key.clone();
        let on_doc: Rc<dyn Fn(String)> = Rc::new(move |doc_body: String| {
            apply_document_response(&inner, &doc_body, &sync, &last_selection_key);
            fetch_busy_done.set(false);
        });
        if !live_sync::get(&format!("{base_owned}/api/mcp/document"), on_doc) {
            fetch_busy.set(false);
        }
    });
    let _ = live_sync::get(&format!("{base}/api/mcp/version"), on_version);
}

/// Apply a `GET /api/mcp/document` response to the live shell.
/// `WebSyncClient::sync` runs the apply closure only for a newer document and
/// commits that exact version only when the closure returns `true` (swap +
/// repaint both succeeded) — so the committed version is never stale and a
/// failed repaint is retried on the next poll. On success the local
/// serialization becomes the push baseline (echo suppression) and the
/// selection key is invalidated (the doc swap reset daemon + local selection,
/// so the daemon must be told the browser's current one again).
fn apply_document_response<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    body: &str,
    sync: &Rc<RefCell<WebSyncClient>>,
    last_selection_key: &Rc<RefCell<Option<String>>>,
) {
    let Ok(mut inner_mut) = inner.try_borrow_mut() else {
        return;
    };
    let inner_ref = &mut *inner_mut;
    let applied = sync
        .try_borrow_mut()
        .ok()
        .and_then(|mut sync| {
            sync.sync(body, |doc, _version| {
                inner_ref.host_mut().replace_document(doc);
                inner_ref.repaint().is_ok()
            })
            .ok()
        })
        .unwrap_or(false);
    if applied {
        // Baseline = OUR serialization of the just-applied document, so the
        // push tick compares apples to apples (serde normalization differs
        // from the daemon's wire bytes).
        if let Ok(json) = serde_json::to_string(&inner_ref.host().editor_state().doc) {
            if let Ok(mut sync) = sync.try_borrow_mut() {
                sync.note_applied_snapshot(&json);
            }
        }
        if let Ok(mut last_selection_key) = last_selection_key.try_borrow_mut() {
            *last_selection_key = None;
        }
    }
}

/// Serialize + push the local document when it changed since the last
/// applied/pushed baseline. Never pushes before the first daemon apply
/// (daemon authority — see the module docs).
fn push_document_if_changed<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    base: &str,
    sync: &Rc<RefCell<WebSyncClient>>,
    push_busy: &Rc<Cell<bool>>,
    oversize_warned: &Rc<Cell<bool>>,
) {
    if push_busy.get() {
        return;
    }
    let initialized = sync
        .try_borrow()
        .map(|sync| sync.initialized())
        .unwrap_or(false);
    let doc_json = {
        let Ok(mut b) = inner.try_borrow_mut() else {
            return;
        };
        // Cheap gate: only serialize when the host flagged a possible change
        // since the last tick (a conservative superset of document edits; the
        // hash check below absorbs the false positives).
        if !b.host_mut().take_doc_sync_dirty() {
            return;
        }
        let Ok(json) = serde_json::to_string(&b.host().editor_state().doc) else {
            return;
        };
        json
    };
    let should_push = sync
        .try_borrow()
        .map(|sync| initialized && sync.should_push(&doc_json))
        .unwrap_or(false);
    if !should_push {
        return;
    }
    if doc_json.len() > SYNC_MAX_BODY_BYTES {
        // TS warns once per oversize streak and skips the push.
        if !oversize_warned.get() {
            oversize_warned.set(true);
            web_sys::console::warn_1(
                &format!(
                    "[mcp-sync] Skip oversized document push: {:.2}MiB > {:.2}MiB",
                    doc_json.len() as f64 / (1024.0 * 1024.0),
                    SYNC_MAX_BODY_BYTES as f64 / (1024.0 * 1024.0)
                )
                .into(),
            );
        }
        return;
    }
    oversize_warned.set(false);
    let body = WebSyncClient::wrap_push_body(&doc_json);
    push_busy.set(true);
    let sync = sync.clone();
    let push_busy_done = push_busy.clone();
    let on_response: Rc<dyn Fn(String)> = Rc::new(move |resp: String| {
        push_busy_done.set(false);
        if let Some(version) = WebSyncClient::parse_push_response(&resp) {
            // Commit baseline + version so our own push is never echoed back.
            if let Ok(mut sync) = sync.try_borrow_mut() {
                sync.mark_pushed(&doc_json, version);
            }
        }
        // A rejected/failed push is dropped best-effort (TS parity) — the
        // next local edit re-flags the host and retries.
    });
    if !live_sync::post_json(
        &format!("{base}/api/mcp/document"),
        &body,
        Some(on_response),
    ) {
        push_busy.set(false);
    }
}

/// POST the selection to the daemon when it changed since the last sample
/// (TS pushes `{selectedIds, activePageId}` debounced 300 ms; this samples on
/// the 400 ms tick). Fire-and-forget like the TS fetch().catch(() => {}).
fn push_selection_if_changed<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    base: &str,
    last_selection_key: &Rc<RefCell<Option<String>>>,
) {
    let (key, body) = {
        let Ok(b) = inner.try_borrow() else {
            return;
        };
        let state = b.host().editor_state();
        (
            web_sync::selection_sync_key(state),
            web_sync::selection_push_body(state),
        )
    };
    if last_selection_key
        .try_borrow()
        .map(|last| last.as_deref() == Some(key.as_str()))
        .unwrap_or(true)
    {
        return;
    }
    let Ok(mut last_selection_key) = last_selection_key.try_borrow_mut() else {
        return;
    };
    *last_selection_key = Some(key);
    let _ = live_sync::post_json(&format!("{base}/api/mcp/selection"), &body, None);
}
