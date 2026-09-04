//! Live-canvas sync glue — bidirectional document + selection sync between
//! the browser shell and the web-canvas daemon. The Rust counterpart of the
//! TS `apps/web/src/hooks/use-mcp-sync.ts`:
//!
//! * **Pull** (external MCP/CLI writes → browser canvas): a 400 ms tick
//!   probes `GET /api/mcp/version` and fetches + applies the full document
//!   only when the daemon's monotonic version advanced. TS receives
//!   `document:update` pushes over SSE instead — a documented transport
//!   divergence (the daemon's SSE stream carries version bumps only, and the
//!   verified XHR machinery is reused); worst-case latency is one tick. Every
//!   pull decision — the tick gate AND the apply-time re-check — is keyed on
//!   the CURRENT `(document_generation, document_revision)` pair via
//!   [`op_editor_core::sync_gate::SyncGate`], not on any cheap dirty hint.
//! * **Push** (browser edits → daemon, so external MCP/CLI clients see them):
//!   a 2000 ms tick — the TS `PUSH_DEBOUNCE_MS` cadence — serializes the live
//!   document when `SyncGate::needs_push` says the current pair moved past
//!   the last-synced baseline (the sole authoritative gate; a stored conflict
//!   holds it closed until explicit resolution), skips the actual POST when
//!   the content hash matches the daemon baseline (re-baselining directly so
//!   the gate can't deadlock), and POSTs
//!   `{document, baseVersion, activePageIndex, preserveAuthoredGeometry}` to
//!   `/api/mcp/document`. Pushes over 2 MiB are skipped with a one-shot
//!   console warning (TS `SYNC_MAX_BODY_BYTES` parity), baseline untouched. A
//!   `version-conflict` response suspends the gate until the host resolves it
//!   (no silent retry); other failures are dropped best-effort (TS catch{}
//!   parity) — the next local edit retriggers.
//! * **Selection push**: the 400 ms tick also samples the selection key and
//!   POSTs `{selectedIds, activePageId}` to `/api/mcp/selection` when it
//!   changed (TS debounces 300 ms; a 400 ms trailing sample is the same
//!   order of latency). One-way browser → daemon, exactly like TS.
//!
//! Architectural divergence (documented): TS's BROWSER is the document
//! authority (it pushes its document on `client:id` and the Nitro server only
//! caches), while the Rust daemon is the authority after mount — so this glue
//! never pushes before the first daemon document has been applied (subsumed
//! by `SyncGate::needs_push`'s None-baseline rule). The static host page calls
//! `/api/mcp/sync-reset` before mounting so a browser refresh starts from the
//! starter document instead of replaying the previous transient web `.op`
//! state; bootstrap pushes remain disabled so stale page state cannot
//! overwrite a deliberately opened daemon document. Live screenshot requests
//! are served by the `--serve-web` daemon's MCP route from this same synced
//! document authority, rather than by browser-side capture.

use std::cell::{Cell, RefCell};
use std::io::{self, Write};
use std::rc::{Rc, Weak};

use op_editor_core::sync_gate::SyncGate;
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

enum SyncDocumentJson {
    Ready(String),
    Oversize,
    Failed,
}

struct CappedJsonWriter {
    bytes: Vec<u8>,
    exceeded: bool,
}

impl CappedJsonWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            exceeded: false,
        }
    }
}

impl Write for CappedJsonWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.bytes.len().saturating_add(bytes.len()) > SYNC_MAX_BODY_BYTES {
            self.exceeded = true;
            return Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                "sync document exceeds limit",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Serialize directly into a capped buffer. A giant Figma document therefore
/// never creates a giant temporary `String` merely to discover that live sync
/// cannot send it.
fn serialize_sync_document(doc: &op_editor_core::PenDocument) -> SyncDocumentJson {
    let mut writer = CappedJsonWriter::new();
    if serde_json::to_writer(&mut writer, doc).is_err() {
        return if writer.exceeded {
            SyncDocumentJson::Oversize
        } else {
            SyncDocumentJson::Failed
        };
    }
    match String::from_utf8(writer.bytes) {
        Ok(json) => SyncDocumentJson::Ready(json),
        Err(_) => SyncDocumentJson::Failed,
    }
}

/// Shared sync state: the gate deciding pull/push eligibility, the wire-level
/// client (version/hash bookkeeping), and the push single-flight latch. Built
/// once in `mount_ck` and handed to both this module's ticks and the
/// postMessage bridge, so both sides observe and mutate the exact same
/// `SyncGate` instance (a v1 defect — the bridge couldn't reach a local
/// Content changes advance the gate's `(generation, revision)` pair, while
/// active-page switches intentionally do not. Treat an editor-metadata delta
/// as an additional push reason without weakening the conflict latch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PushReasons {
    content: bool,
    editor_meta: bool,
}

impl PushReasons {
    fn any(self) -> bool {
        self.content || self.editor_meta
    }
}

fn push_reasons(
    sync: &SyncController,
    pair: (u64, u64),
    active_page_index: usize,
    preserve_authored_geometry: bool,
) -> PushReasons {
    PushReasons {
        content: sync.gate.needs_push(pair),
        editor_meta: sync.gate.conflict().is_none()
            && sync
                .client
                .editor_meta_needs_push(active_page_index, preserve_authored_geometry),
    }
}

/// Wire the bidirectional sync loops onto the mounted shell. Called once from
/// `mount_ck`; both intervals run for the page lifetime. `sync` is shared with
/// the postMessage bridge — this module only ever borrows it for the duration
/// of a single decision, never across an await/callback boundary.
pub(crate) fn start<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>, sync: SharedSync) {
    ACTIVE_SYNC.with(|slot| *slot.borrow_mut() = Some(Rc::downgrade(&sync)));
    let base = crate::daemon_base::daemon_base();
    // One-shot: learn whether this daemon is the document's sole sequencer,
    // which decides whether a sync conflict may auto-resolve.
    probe_serve_mode(&base);
    // One document fetch / one push at a time; ticks observing an in-flight
    // request skip (the TS hook queues at most one — same effective shape).
    // `fetch_busy` stays a plain Cell (pull-side, local to this module);
    // `push_busy` moved onto the shared controller so the bridge can also
    // observe it.
    let fetch_busy = Rc::new(Cell::new(false));
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
        let sync = sync.clone();
        let tick: Rc<dyn Fn()> = Rc::new(move || {
            push_document_if_changed(&inner, &base, &sync, &oversize_warned);
        });
        let _ = live_sync::start_interval(PUSH_INTERVAL_MS, tick);
    }
}

/// Probe the daemon version; on a newer version fetch + apply the document.
/// Gated on the sync-gate FIRST (before any network round-trip): an accept
/// window broken by an intervening local edit re-enters the conflict flow,
/// and otherwise `pull_allowed` must hold for the current pair.
fn poll_version<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    base: &str,
    sync: &SharedSync,
    fetch_busy: &Rc<Cell<bool>>,
    last_selection_key: &Rc<RefCell<Option<String>>>,
) {
    if fetch_busy.get() {
        return;
    }
    let pair = current_pair(&*inner.borrow());
    // Borrow discipline: Rust 2021 extends an `if let` scrutinee's temporary
    // borrow through the whole arm body, so a `borrow_mut()` inside the body
    // below would panic at runtime. Binding the `Option` first lets the
    // `borrow()` temporary drop at the end of THIS statement instead.
    let broken = sync.borrow().gate.accept_window_broken(pair); // borrow ends HERE
    if let Some(v) = broken {
        sync.borrow_mut().gate.note_conflict(v); // observer latch reports; user decides again
        return;
    }
    maybe_auto_resolve_conflict_in_session(inner, sync, pair);
    if !sync.borrow().gate.pull_allowed(pair) {
        return;
    }

    let inner = inner.clone();
    let sync = sync.clone();
    let base_owned = base.to_string();
    let fetch_busy = fetch_busy.clone();
    let last_selection_key = last_selection_key.clone();
    let on_version: Rc<dyn Fn(u16, String)> = Rc::new(move |status: u16, body: String| {
        // A refused credential is the signal that this tab no longer speaks
        // for the account whose document it is showing. Treating it as "no
        // version" — which is what an untyped body read does — is what let the
        // previous account's document sit on screen indefinitely.
        if matches!(status, 401 | 403) {
            note_auth_invalid();
            return;
        }
        // The daemon answers both counters here. Hand the collaboration one to
        // its own loop rather than opening a second probe; a `collabSeq` bump
        // must never reach the document fetch below.
        crate::collab_sync::note_version_probe(&body);
        let Some(version) = WebSyncClient::parse_version_probe(&body) else {
            return; // daemon down / non-JSON error body — retry next tick
        };
        let wants_version = sync
            .try_borrow()
            .map(|s| s.client.wants_version(version))
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
    let _ = live_sync::get_with_status(&format!("{base}/api/mcp/version"), on_version);
}

/// Auto-resolve a latched push conflict, but only inside a live session.
///
/// A conflict closes both the pull and the push gate and is cleared by exactly
/// two things, neither reachable from this tick: the VS Code host's explicit
/// `resolve-conflict`, and a successful re-push. A plain browser tab has
/// neither, so a single 409 wedges live sync for the rest of the page's life.
///
/// Auto-accepting the remote is only safe when the *server* is the authority
/// and the user's dropped work is recoverable — which is exactly what an
/// `Active` collaboration session provides:
///
/// * the daemon's document is the sequenced truth every peer already sees, so
///   there is nothing for this tab to "win" by holding its copy back;
/// * the runtime projects a rejected local edit into
///   `collab.discarded_edit`, and the panel offers to replay it, so accepting
///   the remote does not silently destroy the edit that lost.
///
/// Outside a session neither holds — the daemon is a peer, not an authority,
/// and nothing preserves the losing edit — so the latch stays and the existing
/// explicit-resolution semantics are untouched. That is the difference between
/// recovering automatically and quietly overwriting a user's unpushed work.
fn maybe_auto_resolve_conflict_in_session<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    sync: &SharedSync,
    pair: (u64, u64),
) {
    let has_conflict = sync.borrow().gate.conflict().is_some();
    let phase = inner
        .try_borrow()
        .map(|context| context.host().editor_state().editor_ui.collab.phase)
        .unwrap_or(op_editor_core::CollabConnectionPhase::Idle);
    if !auto_resolve_is_safe(has_conflict, phase, server_is_authoritative()) {
        return;
    }
    // Accepting the remote overwrites whatever this tab had not yet pushed.
    // Undo is the primary way back (the apply is undoable); this stash is the
    // belt-and-braces copy plus the notice that tells the user any of it
    // happened. It must run BEFORE the resolve, while the local document is
    // still the one on screen.
    if !live_sync_conflict::preserve_local_document(inner, crate::collab_sync::now_ms()) {
        // No backup taken, so nothing is accepted. Better to converge one tick
        // later than to overwrite unpushed work with no way back.
        return;
    }
    // Re-opens the pull for THIS pair only; the resolving pull's apply calls
    // `note_synced`, which is what finally clears the baseline and reopens the
    // push side too.
    if let Ok(mut sync) = sync.try_borrow_mut() {
        sync.gate.resolve_accept_remote(pair);
    }
}

/// Apply a `GET /api/mcp/document` response to the live shell.
/// `WebSyncClient::sync_with_editor_meta` runs the apply closure only for a
/// newer document and commits that exact version only when the closure returns
/// `true` (swap + repaint both succeeded) — so the committed version is never
/// stale and a failed repaint is retried on the next poll. On success the local
/// serialization becomes the push baseline (echo suppression), the selection
/// key is invalidated (the doc swap reset daemon + local selection, so the
/// daemon must be told the browser's current one again), and the sync-gate
/// baseline is committed to the PAIR AFTER the apply (a `replace_document`
/// bumps generation, so the pre-apply pair would be stale).
fn apply_document_response<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    body: &str,
    sync: &SharedSync,
    last_selection_key: &Rc<RefCell<Option<String>>>,
) {
    let Ok(mut inner_mut) = inner.try_borrow_mut() else {
        return;
    };
    let inner_ref = &mut *inner_mut;
    // Apply-time re-check: a local edit may have landed while the fetch was
    // in flight; re-read the CURRENT pair and re-run `pull_allowed` before
    // taking any mutable borrow of the controller. Borrow ends at the end of
    // this statement (plain `if`, not `if let` — the condition's temporary
    // does not extend into the body).
    if !sync.borrow().gate.pull_allowed(current_pair(inner_ref)) {
        return; // a local edit landed while the fetch was in flight — abort apply
    }
    // Captured BEFORE sync() flips it on the first apply: the
    // mount-time pull (starter doc → daemon doc) must NOT become an
    // undo step; every later external apply (AI turn, MCP client) must.
    let undoable = sync
        .try_borrow()
        .map(|s| s.client.initialized())
        .unwrap_or(false);
    // The `WebSyncClient::sync_with_editor_meta` closure runs while `sync` is held
    // mutably borrowed (via `s` below) — it must never itself touch `sync`;
    // it only touches `inner_ref`, which is a separate RefCell.
    let applied = sync
        .try_borrow_mut()
        .ok()
        .and_then(|mut s| {
            s.client
                .sync_with_editor_meta(
                    body,
                    |doc,
                     _version,
                     active_page_index,
                     preserve_authored_geometry,
                     wire_scenario| {
                        let host = inner_ref.host_mut();
                        host.replace_document_from_sync(doc, undoable);
                        // A daemon new enough to send `scenario` is the
                        // authority (the browser boots with `None`, and a deck
                        // must present as a deck). An older daemon omits the
                        // field — keep whatever the open document already had
                        // rather than letting every sync erase its tag.
                        let scenario = wire_scenario.or(host.editor_state().editor_ui.scenario);
                        let pinned_style_guide =
                            host.editor_state().editor_ui.pinned_style_guide.clone();
                        op_pen_loader::apply_editor_meta(
                            host.editor_state_mut(),
                            op_pen_loader::EditorMeta {
                                active_page_index,
                                preserve_authored_geometry,
                                scenario,
                                pinned_style_guide,
                            },
                        );
                        inner_ref.repaint().is_ok()
                    },
                )
                .ok()
        })
        .unwrap_or(false);
    if applied {
        // Every id in this document came from the daemon, so it is the set an
        // active session will accept a push against. Recording it here is what
        // lets the push gate tell an edited node from an invented one.
        crate::collab_sync::note_daemon_document(inner_ref.host().editor_state());
        // Baseline = OUR serialization of the just-applied document, so the
        // push tick compares apples to apples (serde normalization differs
        // from the daemon's wire bytes).
        let state = inner_ref.host().editor_state();
        let active_page_index = state.ui.active_page_index;
        let preserve_authored_geometry = state.editor_ui.preserve_authored_geometry;
        let serialized = serialize_sync_document(&state.doc);
        if let Ok(mut s) = sync.try_borrow_mut() {
            // Always baseline the two scalar fields. Oversized documents skip
            // their byte hash, but must not look metadata-dirty forever.
            s.client
                .note_applied_editor_meta(active_page_index, preserve_authored_geometry);
            if let SyncDocumentJson::Ready(json) = serialized {
                s.client.note_applied_snapshot_with_editor_meta(
                    &json,
                    active_page_index,
                    preserve_authored_geometry,
                );
            }
        }
        if let Ok(mut last_selection_key) = last_selection_key.try_borrow_mut() {
            *last_selection_key = None;
        }
        // Commit the sync-gate baseline AFTER the apply closure has returned
        // (its mutable borrow above is released) and using the POST-apply pair.
        //
        // An external post-bootstrap apply (`undoable` — an AI turn or an MCP
        // client write) is an UNSAVED daemon-side edit: the tab must read dirty
        // so closing it prompts to save (spec: MCP edits mark the tab dirty).
        // That dirtiness is already established by construction:
        // `replace_document_with_undo` runs `history_push_past`, which bumps the
        // content revision via `mark_document_changed` — so no manual bump is
        // needed here (it would only be a redundant second revision tick).
        // `note_synced` records this post-bump pair as the gate baseline, so the
        // very next push tick sees current == baseline (no spurious echo-push of
        // the just-applied doc back to the daemon) while `is_dirty()` stays true
        // (revision != saved 0). The bootstrap apply (`undoable == false`, the
        // mount-time starter→daemon pull) goes through the plain
        // `replace_document`, which resets revision to 0 and stays clean.
        let post_apply = current_pair(inner_ref);
        sync.borrow_mut()
            .gate
            .note_synced(post_apply.0, post_apply.1);
    }
}

/// Serialize + conditionally push the local document. Content changes are
/// gated by `SyncGate::needs_push`; the two editor-metadata fields have their
/// own baseline because page switching intentionally leaves the content
/// revision unchanged. Both paths honor the conflict latch and bootstrap
/// authority. `take_doc_sync_dirty` remains only a cheap hint.
fn push_document_if_changed<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    base: &str,
    sync: &SharedSync,
    oversize_warned: &Rc<Cell<bool>>,
) {
    let busy = sync.borrow().push_busy;
    if busy {
        return;
    }
    let (pair, doc_json, active_page_index, preserve_authored_geometry, reasons) = {
        let Ok(mut b) = inner.try_borrow_mut() else {
            return;
        };
        // Consume the hint — NEVER a gate. A false positive here just costs
        // one wasted `needs_push` check below; a false negative would be
        // caught by the gate anyway since it reads the live pair directly.
        let _ = b.host_mut().take_doc_sync_dirty();
        let pair = current_pair(&*b);
        let state = b.host().editor_state();
        let active_page_index = state.ui.active_page_index;
        let preserve_authored_geometry = state.editor_ui.preserve_authored_geometry;
        let reasons = push_reasons(
            &sync.borrow(),
            pair,
            active_page_index,
            preserve_authored_geometry,
        );
        if !reasons.any() {
            return;
        }
        let oversize_identity = current_oversize_identity(&*b);
        if sync.borrow().oversize_identity == Some(oversize_identity) {
            return;
        }
        let json = match serialize_sync_document(&b.host().editor_state().doc) {
            SyncDocumentJson::Ready(json) => json,
            SyncDocumentJson::Oversize => {
                sync.borrow_mut().oversize_identity = Some(oversize_identity);
                if !oversize_warned.replace(true) {
                    web_sys::console::warn_1(
                        &format!(
                            "[mcp-sync] Skip oversized document push: > {:.2}MiB",
                            SYNC_MAX_BODY_BYTES as f64 / (1024.0 * 1024.0)
                        )
                        .into(),
                    );
                }
                return;
            }
            SyncDocumentJson::Failed => return,
        };
        (
            pair,
            json,
            active_page_index,
            preserve_authored_geometry,
            reasons,
        )
    };

    let should_push = if reasons.content {
        sync.borrow().client.should_push_with_editor_meta(
            &doc_json,
            active_page_index,
            preserve_authored_geometry,
        )
    } else {
        // A daemon Save acknowledgement intentionally leaves the large typed
        // document hash unknown. A metadata-only page switch must be decided
        // from the scalar baseline, not mistaken for a content change.
        reasons.editor_meta
    };
    // The daemon refused this tab's credential. Whatever is on screen belongs
    // to whoever was signed in before, so pushing it now would write one
    // account's work into whichever tenant the new credential resolves to.
    // Held until the identity reset rebuilds the tab.
    if auth_is_invalid() {
        return;
    }
    // An active collaboration session cannot sequence a node id this browser
    // minted from its local counter. Hold the push rather than fork the shared
    // document; the next pull replaces the local node with the daemon's copy.
    if crate::collab_sync::push_blocked_by_session(inner.borrow().host().editor_state()) {
        return;
    }
    if !should_push {
        // Bytes already match the daemon baseline (e.g. a generation-only
        // replace — same content, new generation): no push to send, but the
        // baseline MUST advance directly or `needs_push` stays true forever,
        // the hash check keeps skipping the push, and the pull gate never
        // reopens (deadlock).
        let mut sync = sync.borrow_mut();
        sync.oversize_identity = None;
        sync.gate.note_synced(pair.0, pair.1);
        return;
    }
    if !SyncGate::periodic_push_allowed(doc_json.len()) {
        // TS warns once per oversize streak and skips the push; baseline
        // stays put (the uncapped snapshot channel is the fallback path).
        let warned = oversize_warned.get();
        if !warned {
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
        sync.borrow_mut().oversize_identity = Some(current_oversize_identity(&*inner.borrow()));
        return;
    }
    sync.borrow_mut().oversize_identity = None;
    oversize_warned.set(false);
    let base_version = sync.borrow().client.last_version();
    let body = WebSyncClient::wrap_push_body_with_base_editor_meta_and_mode(
        &doc_json,
        base_version,
        active_page_index,
        preserve_authored_geometry,
        !reasons.content,
    );
    sync.borrow_mut().push_busy = true;
    let sync_done = sync.clone();
    // The pair captured AT SERIALIZATION TIME (above), not whatever the
    // current pair is when the response lands — an edit landing while this
    // push is in flight must keep reporting `needs_push`.
    let pushed_pair = pair;
    let on_response: Rc<dyn Fn(String)> = Rc::new(move |resp: String| {
        sync_done.borrow_mut().push_busy = false;
        // Conflict check first: a rejected push must never fall through to
        // the success branch's `note_synced` (which would wrongly reopen
        // the pull gate on rejected bytes).
        let conflict_version = WebSyncClient::parse_push_conflict(&resp);
        if let Some(server_v) = conflict_version {
            sync_done.borrow_mut().gate.note_conflict(server_v);
            return;
        }
        let accepted_version = WebSyncClient::parse_push_response(&resp);
        if let Some(version) = accepted_version {
            let mut s = sync_done.borrow_mut();
            s.client.mark_pushed_with_editor_meta(
                &doc_json,
                active_page_index,
                preserve_authored_geometry,
                version,
            );
            s.gate.note_synced(pushed_pair.0, pushed_pair.1);
        }
        // Neither shape recognized (network/parse failure): dropped
        // best-effort (TS catch{} parity) — the next local edit re-flags
        // needs_push and retries.
    });
    if !live_sync::post_json(
        &format!("{base}/api/mcp/document"),
        &body,
        Some(on_response),
    ) {
        sync.borrow_mut().push_busy = false;
    }
}

/// POST the selection to the daemon when it changed since the last sample
/// (TS pushes `{selectedIds, activePageId}` debounced 300 ms; this samples on
/// the 400 ms tick). Fire-and-forget like the TS fetch().catch(() => {}).
/// Unaffected by the sync gate (one-way browser → daemon presentation state,
/// not document content).
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

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_document() -> op_editor_core::PenDocument {
        serde_json::from_str(r#"{"version":"1.0","children":[]}"#)
            .expect("minimal document should deserialize")
    }

    #[test]
    fn capped_serializer_matches_regular_json_for_small_documents() {
        let doc = minimal_document();
        let expected = serde_json::to_string(&doc).expect("regular serialization should succeed");

        match serialize_sync_document(&doc) {
            SyncDocumentJson::Ready(actual) => assert_eq!(actual, expected),
            SyncDocumentJson::Oversize => panic!("small document should fit under the cap"),
            SyncDocumentJson::Failed => panic!("small document should serialize"),
        }
    }

    #[test]
    fn capped_serializer_stops_oversized_documents_at_the_limit() {
        let mut doc = minimal_document();
        doc.name = Some("x".repeat(SYNC_MAX_BODY_BYTES));

        assert!(matches!(
            serialize_sync_document(&doc),
            SyncDocumentJson::Oversize
        ));
    }

    #[test]
    fn capped_writer_accepts_exact_limit_then_rejects_another_byte() {
        let mut writer = CappedJsonWriter::new();
        let exact_limit = vec![b'x'; SYNC_MAX_BODY_BYTES];
        writer
            .write_all(&exact_limit)
            .expect("the exact cap should be accepted");
        assert_eq!(writer.bytes.len(), SYNC_MAX_BODY_BYTES);

        assert!(writer.write_all(b"x").is_err());
        assert!(writer.exceeded);
        assert_eq!(writer.bytes.len(), SYNC_MAX_BODY_BYTES);
    }

    #[test]
    fn active_page_only_change_is_push_due_without_a_revision_change() {
        let mut sync = SyncController::new();
        let pair = (7, 11);
        sync.gate.note_synced(pair.0, pair.1);
        sync.client.mark_applied(3);
        sync.client.note_applied_snapshot_without_hash(0, true);

        assert!(!push_reasons(&sync, pair, 0, true).any());
        assert!(
            push_reasons(&sync, pair, 1, true).editor_meta,
            "page metadata must bypass the unchanged revision pair"
        );

        sync.gate.note_conflict(4);
        assert!(
            !push_reasons(&sync, pair, 1, true).any(),
            "metadata pushes must still honor the conflict latch"
        );
    }
}

thread_local! {
    /// Raised when the daemon answered a sync request 401/403.
    ///
    /// A tab whose credential stopped being accepted must stop pushing: the
    /// document on screen belongs to whoever WAS signed in, and pushing it
    /// after a switch would write one account's work into another's tenant.
    /// Cleared by [`clear_auth_invalid`], which the identity reset calls once
    /// the tab has been rebuilt for the new account.
    pub(super) static AUTH_INVALID: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };

    /// The mounted editor's sync controller, exposed weakly so File → Save can
    /// consume the daemon's returned version without creating an ownership
    /// cycle or threading sync state through every DOM file-action callback.
    pub(super) static ACTIVE_SYNC: RefCell<Option<Weak<RefCell<SyncController>>>> = const { RefCell::new(None) };
}

#[path = "live_sync_controller.rs"]
mod live_sync_controller;
pub(crate) use live_sync_controller::{acknowledge_daemon_save, SharedSync, SyncController};
// Spine-local: the two identity pairs every gating decision here is keyed on.
use live_sync_controller::{current_oversize_identity, current_pair};

#[path = "live_sync_conflict.rs"]
mod live_sync_conflict;
#[path = "live_sync_identity.rs"]
mod live_sync_identity;
pub(crate) use live_sync_identity::{auth_is_invalid, note_auth_invalid, reset_for_new_identity};
// Only the latch's own test lifts it; the reset clears it internally.
use live_sync_conflict::{auto_resolve_is_safe, probe_serve_mode, server_is_authoritative};
#[cfg(test)]
pub(crate) use live_sync_identity::clear_auth_invalid;
// Lives here rather than beside `lib.rs`'s modules because the two paths it
// drives are private submodules of this spine.
#[cfg(test)]
#[path = "live_sync_toast_tests.rs"]
mod live_sync_toast_tests;
