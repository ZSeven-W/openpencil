//! `postMessage` bridge between the VS Code extension host and the wasm web
//! editor — the thin DOM wiring layer. The wire codec (parse + event builders)
//! lives in `op_editor_core::bridge_protocol`; the concurrency semantics live in
//! `op_editor_core::sync_gate::SyncGate`. This file only translates DOM
//! `message` events into gate/client mutations + daemon requests, and posts the
//! codec's outbound strings back to `window.parent`.
//!
//! Emission discipline (enforced by construction): the three EDGE/STATE events
//! `dirty-changed`, `sync-conflict`, and `opened` are emitted ONLY from the tick
//! observer ([`observe_tick`]) which drains the gate's consumable latches
//! (`take_conflict_edge` / `take_opened_edge`) and compares the
//! `(generation, revision, is_dirty)` triple. Message handlers NEVER post those
//! three directly — they mutate the gate (`note_conflict` / `note_synced`) and
//! let the observer report. Handlers DO post the direct replies `ready`,
//! `snapshot-result`, `snapshot-conflict`, and `conflict-resolved`.
//!
//! Borrow discipline: a `SharedSync`/`inner` borrow is held only for the span
//! of one synchronous decision, never across an XHR callback.
//!
//! Startup handoff (dsh-openpencil #2): [`early_listener`] installs a
//! minimal `message` listener at the very start of the mount — before the
//! ~24 MB wasm download — buffers inbound messages and announces
//! `op-bridge/listening`; [`install`] then replays the buffer through this
//! pipeline once the shell state exists.

use std::cell::RefCell;
use std::rc::Rc;

use js_sys::{Function, Object};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::MessageEvent;

#[path = "vscode_bridge_snapshot.rs"]
mod document_snapshot;
mod early_listener;
mod external_relay;
mod helpers;
mod startup;

pub(crate) use early_listener::install_early;
pub(crate) use startup::{await_init, in_iframe};

/// Hold an external-navigation request until the locked bridge proves it is
/// attached to the token-authenticated managed daemon.
pub(crate) fn request_external_navigation(url: &str) {
    external_relay::request(url);
}

use crate::document_json::{parse_document_json, with_borrowed_parsed_document};
use crate::live_sync;
use crate::live_sync_glue::SharedSync;
use crate::repaint_ctx::RepaintContext;
use op_editor_core::bridge_protocol::{
    event_conflict_resolved, event_ready, event_snapshot_conflict, event_snapshot_result,
    BridgeInbound, ConflictMode,
};
use op_editor_core::web_sync::WebSyncClient;

use document_snapshot::BridgeDocumentSnapshot;
use helpers::{
    acquire_push_busy, post_to_parent, read_triple, release_push_busy, schedule_once,
    snapshot_state,
};

/// Tick cadence for the outbound-event observer. Latency only: the gate's edge
/// latches never lose an event between ticks (a fast rise+fall is still drained
/// once), so this can stay a coarse poll.
const BRIDGE_TICK_INTERVAL_MS: i32 = 250;
/// Re-check interval while waiting for an in-flight push to release `push_busy`
/// so the bridge's own (open / snapshot / resolve) push can serialize behind it.
const PUSH_BUSY_RETRY_MS: i32 = 40;
/// A 409 between probe and push means a concurrent MCP write landed; re-probe
/// and retry exactly once before surfacing the conflict.
const RETRY_ONCE: u8 = 1;

/// The `(generation, revision, is_dirty)` triple the observer compares to emit
/// `dirty-changed`.
type DirtyTriple = (u64, u64, bool);
/// The observer's remembered-last-triple cell.
type LastTripleCell = Rc<RefCell<Option<DirtyTriple>>>;

thread_local! {
    /// Locked host origin — recorded from the FIRST valid `init`'s
    /// `event.origin`, then enforced on every later message (mismatch dropped).
    static BRIDGE_ORIGIN: RefCell<Option<String>> = const { RefCell::new(None) };
    /// The pending `await_init` promise resolver — the `init` handler calls it
    /// so `mount_ck` stops waiting the moment the token lands. `pub(super)`
    /// because `startup::await_init` installs it from the sibling module.
    pub(super) static INIT_RESOLVER: RefCell<Option<Function>> = const { RefCell::new(None) };
    /// One-shot late-init recovery hook. Registered by `canvaskit`'s FALLBACK
    /// (unmanaged) bootstrap completion when `await_init` timed out; invoked by
    /// [`handle_init`] if a slow host's `init` lands afterwards, so the managed
    /// bootstrap re-runs and `ready` is still emitted. `None` on the normal path
    /// (init before the timeout), where it is never registered.
    static LATE_INIT_HOOK: RefCell<Option<Box<dyn Fn()>>> = const { RefCell::new(None) };
}

// ---------------------------------------------------------------------------
// Install + startup coordination
// ---------------------------------------------------------------------------

/// Install the window `message` listener + the outbound-event tick observer,
/// then replay whatever the early listener buffered during the backend
/// download (typically the host's `init`). The listener `Closure` is
/// deliberately leaked (`forget`) — the bridge lives for the whole page,
/// exactly like the other page-level listeners; returning an owning handle
/// would tear the bridge down when `mount_ck` returns.
pub(crate) fn install<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>, sync: SharedSync) {
    // Take over from the early listener FIRST: it no-ops from here on, and
    // everything it buffered is drained for the replay below. This function
    // runs as one synchronous task (no message event can interleave), so any
    // message either sits in the drained buffer or is handled by the full
    // listener registered below — nothing is dropped in between.
    let buffered = early_listener::take_over();

    let Some(window) = web_sys::window() else {
        return;
    };

    // Message listener.
    {
        let inner = inner.clone();
        let sync = sync.clone();
        let closure = Closure::<dyn FnMut(MessageEvent)>::new(move |evt: MessageEvent| {
            handle_message(&inner, &sync, &evt);
        });
        let _ =
            window.add_event_listener_with_callback("message", closure.as_ref().unchecked_ref());
        closure.forget(); // page-lifetime listener, deliberately leaked
    }

    // Outbound-event tick observer — the SOLE emitter of the three edge/state
    // events. Seed the dirty triple with the current state so only real changes
    // (not the starter-document baseline) produce a `dirty-changed`.
    let seed = read_triple(inner);
    let last_triple = Rc::new(RefCell::new(seed));
    {
        let inner = inner.clone();
        let sync = sync.clone();
        let last_triple = last_triple.clone();
        let tick: Rc<dyn Fn()> = Rc::new(move || observe_tick(&inner, &sync, &last_triple));
        let _ = live_sync::start_interval(BRIDGE_TICK_INTERVAL_MS, tick);
    }

    // Replay what the early listener buffered while the wasm was still
    // downloading — most importantly the host's `init`, which would
    // otherwise die with its finite retry burst — through the exact same
    // pipeline the live listener uses, with the receive-time origin each
    // message carried.
    for (origin, msg) in buffered {
        route_message(inner, &sync, &origin, msg);
    }
}

// ---------------------------------------------------------------------------
// Inbound message routing
// ---------------------------------------------------------------------------

fn handle_message<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    sync: &SharedSync,
    evt: &MessageEvent,
) {
    // Source lock: only the parent frame (the VS Code webview host) may drive
    // the bridge. `event.source == window.parent`.
    let parent = web_sys::window().and_then(|w| w.parent().ok().flatten());
    let source_ok = match (evt.source(), parent) {
        (Some(src), Some(par)) => Object::is(src.as_ref(), par.as_ref()),
        _ => false,
    };
    if !source_ok {
        return;
    }

    // Bridge messages are JSON strings; anything else (react-devtools objects,
    // etc.) is foreign traffic — silently ignored, never an error.
    let Some(raw) = evt.data().as_string() else {
        return;
    };
    let Some(msg) = BridgeInbound::parse(&raw) else {
        return; // non-bridge / malformed
    };

    route_message(inner, sync, &evt.origin(), msg);
}

/// Origin lock + dispatch — shared by the live listener and the early-inbox
/// replay: [`install`] feeds [`early_listener`]'s buffered messages through
/// here with their receive-time origin, so both paths enforce the same rules.
fn route_message<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    sync: &SharedSync,
    origin: &str,
    msg: BridgeInbound,
) {
    // Origin lock: the first valid `init` records the origin; every later
    // message (including a re-`init`) must match it. A non-init arriving before
    // any lock is dropped.
    let is_init = matches!(msg, BridgeInbound::Init { .. });
    let locked = BRIDGE_ORIGIN.with(|o| o.borrow().clone());
    match &locked {
        Some(l) if *l != origin => return,
        None if !is_init => return,
        _ => {}
    }
    if is_init && locked.is_none() {
        BRIDGE_ORIGIN.with(|o| *o.borrow_mut() = Some(origin.to_string()));
    }

    match msg {
        BridgeInbound::Init { token, mcp_url } => handle_init(inner, token, mcp_url),
        BridgeInbound::Theme { color_scheme } => handle_theme(inner, color_scheme),
        BridgeInbound::Locale { locale } => handle_locale(inner, locale),
        BridgeInbound::OpenDocument { json } => handle_open_document(inner, sync, json),
        BridgeInbound::Snapshot { request_id, .. } => handle_snapshot(inner, sync, request_id),
        BridgeInbound::SaveCommitted {
            generation,
            revision,
        } => handle_save_committed(inner, generation, revision),
        BridgeInbound::ResolveConflict { mode, request_id } => {
            handle_resolve_conflict(inner, sync, mode, request_id)
        }
    }
}

/// `locale`: apply the embedding host's locale as a page-lifetime override
/// without replacing the user's OpenPencil locale preference.
fn handle_locale<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    locale: op_editor_core::Locale,
) {
    let Ok(mut context) = inner.try_borrow_mut() else {
        return;
    };
    context
        .host_mut()
        .editor_state_mut()
        .editor_ui
        .set_host_locale_override(Some(locale));
    context.host_mut().mark_editor_state_dirty();
    let _ = context.repaint();
}

/// `theme`: apply the embedding host's color scheme as a page-lifetime
/// override and repaint immediately. `web_settings::theme` retains the user's
/// prior OpenPencil preference underneath this override, so neither the device
/// key nor the compatibility settings payload adopts the host scheme.
fn handle_theme<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    color_scheme: op_editor_core::ThemeMode,
) {
    let Ok(mut context) = inner.try_borrow_mut() else {
        return;
    };
    crate::web_settings::theme::set_host_override(
        context.host_mut().editor_state_mut(),
        color_scheme,
    );
    context.host_mut().mark_editor_state_dirty();
    let _ = context.repaint();
}

/// `init`: store the managed token and unblock `mount_ck`'s `await_init`.
/// Idempotent by construction: the token write below is unconditional, so the
/// host's repeated `init` messages (its own retries plus the resend the
/// `op-bridge/listening` announcement triggers) all just overwrite the same
/// value.
///
/// `ready` is deliberately NOT emitted here. It must be serialized strictly
/// AFTER the managed bootstrap sync-reset: the host sends `open-document` the
/// moment it sees `ready`, and an open push that lands before the reset would
/// be silently clobbered when the still-in-flight reset resets the daemon to
/// its `--file` content and the next pull tick pulls that over the just-opened
/// canvas. The reset-completion callback in `canvaskit`'s managed path calls
/// [`emit_ready`] once the reset has landed.
fn handle_init<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    token: String,
    mcp_url: Option<String>,
) {
    let token_changed = live_sync::bridge_token().as_deref() != Some(token.as_str());
    live_sync::set_bridge_token(token);
    if token_changed {
        crate::web_auth_sync::refresh_status(inner);
    }
    if let Some(url) = mcp_url {
        // Surface the host's real MCP endpoint on the settings card (the
        // daemon-internal port would point clients at a dead endpoint).
        if let Ok(mut b) = inner.try_borrow_mut() {
            b.host_mut()
                .editor_state_mut()
                .editor_ui
                .agent_settings
                .embed_mcp_url = Some(url);
            b.host_mut().mark_editor_state_dirty();
        }
    }
    INIT_RESOLVER.with(|r| {
        if let Some(resolve) = r.borrow_mut().take() {
            let _ = resolve.call0(&JsValue::NULL);
        }
    });
    // Late-init recovery: when the host's `init` lands AFTER `await_init`'s
    // timeout, the fallback (unmanaged) bootstrap already ran and never emitted
    // `ready`. The one-shot hook — registered by that fallback bootstrap's
    // completion — re-runs the managed bootstrap (a tokened sync-reset, now that
    // the token above is stored) so `ready` is finally emitted. Take it so it
    // fires at most once; on the normal path (init before the timeout) it is
    // never registered, so this is a no-op.
    let hook = LATE_INIT_HOOK.with(|h| h.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

/// Register the one-shot late-init recovery hook (see [`LATE_INIT_HOOK`]).
/// `canvaskit`'s fallback bootstrap calls this ONLY after its unmanaged
/// sync-reset has completed, so the recovery reset the hook later triggers
/// cannot interleave with the fallback reset. Overwrites any prior hook (only
/// one bootstrap ever registers).
pub(crate) fn register_late_init_hook<F: Fn() + 'static>(hook: F) {
    LATE_INIT_HOOK.with(|h| *h.borrow_mut() = Some(Box::new(hook)));
}

/// Post the managed `ready` reply with the current `(generation, revision)` so
/// the host learns the starter document's identity before it sends
/// `open-document`. Called from `canvaskit`'s managed bootstrap ONLY after the
/// sync-reset has completed (see [`handle_init`]). `ready` is a
/// request/response-style reply — not one of the three edge/state events the
/// tick observer owns — so a direct post here keeps single-point edge
/// discipline intact while ordering `ready` strictly after the reset.
pub(crate) fn emit_ready<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    let (gen, rev, _) = read_triple(inner).unwrap_or((0, 0, false));
    post_to_parent(&event_ready(gen, rev));
}

/// `open-document`: SYNCHRONOUS PROLOGUE (fixed order, before the first await) —
/// `replace_document` mints the target generation `G`, `note_open_pending(G)`
/// scopes the open + blocks pulls, then a probe-conditional push carries the
/// opened bytes to the daemon. `opened` is NOT posted here: `note_synced` on
/// push confirmation sets the opened latch, which the observer drains.
fn handle_open_document<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    sync: &SharedSync,
    json: String,
) {
    // The extension sends the raw on-disk bytes — files saved by the
    // desktop / CLI carry a deduplicated `images` table that must be
    // resolved back to inline data URLs before the typed parse.
    let parsed = match parse_document_json(&json) {
        Ok(parsed) => parsed,
        Err(err) => {
            web_sys::console::warn_1(&JsValue::from_str(&format!(
                "[op-bridge] open-document: bad JSON: {err}"
            )));
            return;
        }
    };

    // Prologue borrow: replace + repaint + capture the opened pair/bytes.
    let Some(snapshot) = with_borrowed_parsed_document(inner, parsed, |b, doc, meta| {
        b.host_mut().editor_state_mut().replace_document(doc);
        op_pen_loader::apply_editor_meta_or_legacy_fallback(b.host_mut().editor_state_mut(), meta);
        b.host_mut().force_rotate_layer_panel_owner();
        b.host_mut().mark_editor_state_dirty();
        b.host_mut().arm_missing_fonts_detection();
        let (w, h) = b.viewport_size();
        b.host_mut().fit_content_to_viewport(w, h);
        let _ = b.repaint();
        BridgeDocumentSnapshot::capture(b.host().editor_state())
    })
    .flatten() else {
        return;
    };

    // Scope the open to generation G and block pulls, all before any await.
    if let Ok(mut s) = sync.try_borrow_mut() {
        s.gate.note_open_pending(snapshot.pair().0);
    } else {
        return;
    }

    let base = crate::daemon_base::daemon_base();
    drive_open_push(sync.clone(), base, snapshot);
}

/// `snapshot`: independent of the tick. Conflict pending → reply
/// `snapshot-conflict` at once (host must resolve first). Otherwise capture the
/// pair + bytes atomically; if a push is due, send it over the uncapped
/// snapshot channel (respecting `push_busy` serialization) and reply on
/// confirmation; if nothing is due, reply immediately with the current bytes.
fn handle_snapshot<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    sync: &SharedSync,
    request_id: String,
) {
    if let Some(server_v) = sync.try_borrow().ok().and_then(|s| s.gate.conflict()) {
        post_to_parent(&event_snapshot_conflict(&request_id, server_v));
        return;
    }
    let Some(snapshot) = snapshot_state(inner) else {
        return;
    };
    let pair = snapshot.pair();
    let needs_push = sync
        .try_borrow()
        .map(|s| s.gate.needs_push(pair))
        .unwrap_or(false);
    if !needs_push {
        post_to_parent(&event_snapshot_result(
            &request_id,
            &snapshot.externalized_json(),
            pair.0,
            pair.1,
        ));
        return;
    }
    let base = crate::daemon_base::daemon_base();
    drive_snapshot_push(sync.clone(), base, request_id, snapshot);
}

/// `save-committed`: mark the reported revision saved. A stale generation
/// (the host acked a save for a document already replaced) returns `false` and
/// is silently dropped. The resulting dirty-flag flip is reported by the
/// observer as `dirty-changed`, not here.
fn handle_save_committed<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    generation: u64,
    revision: u64,
) {
    let Ok(mut b) = inner.try_borrow_mut() else {
        return;
    };
    let _ = b
        .host_mut()
        .editor_state_mut()
        .mark_saved_revision_at(generation, revision);
    let _ = b.repaint();
}

fn handle_resolve_conflict<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    sync: &SharedSync,
    mode: ConflictMode,
    request_id: String,
) {
    match mode {
        ConflictMode::UseLocal => resolve_use_local(inner, sync, request_id),
        ConflictMode::AcceptRemote => resolve_accept_remote(inner, sync, request_id),
    }
}

/// `resolve-conflict: use-local`: re-push the local document over the snapshot
/// channel using the conflict's server version as `baseVersion`. Success →
/// `mark_pushed` + `note_synced` (clears the conflict, and any pending open,
/// whose `opened` the observer then reports) + `conflict-resolved`. A second
/// conflict retries once with the fresh server version; still failing →
/// `snapshot-conflict`.
fn resolve_use_local<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    sync: &SharedSync,
    request_id: String,
) {
    let Some(server_v) = sync.try_borrow().ok().and_then(|s| s.gate.conflict()) else {
        // No conflict pending — nothing to re-push. Acknowledge idempotently.
        post_to_parent(&event_conflict_resolved(&request_id));
        return;
    };
    let Some(snapshot) = snapshot_state(inner) else {
        return;
    };
    let base = crate::daemon_base::daemon_base();
    drive_use_local_push(sync.clone(), base, request_id, snapshot, server_v);
}

/// `resolve-conflict: accept-remote`: first reply `snapshot-result` with the
/// LOCAL bytes (the host keeps a backup — spec "neither version is lost"), then
/// hand the gate the accept window over the current pair (pull re-opens for
/// THIS pair only; open_pending is retained), then reply `conflict-resolved`.
/// `opened` is NOT posted now — the remote is not applied yet; the resolving
/// pull's `note_synced` sets the opened latch, which the observer drains then.
fn resolve_accept_remote<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    sync: &SharedSync,
    request_id: String,
) {
    let Some(snapshot) = snapshot_state(inner) else {
        return;
    };
    let pair = snapshot.pair();
    post_to_parent(&event_snapshot_result(
        &request_id,
        &snapshot.externalized_json(),
        pair.0,
        pair.1,
    ));
    if let Ok(mut s) = sync.try_borrow_mut() {
        s.gate.resolve_accept_remote(pair);
    } else {
        return;
    }
    post_to_parent(&event_conflict_resolved(&request_id));
}

// ---------------------------------------------------------------------------
// Push drivers (SharedSync-only; no `inner`, no `C`)
// ---------------------------------------------------------------------------

/// Acquire `push_busy` (waiting via a short self-reschedule while an in-flight
/// push holds it — the open's state is already latched, and `open_pull_block`
/// keeps pulls out meanwhile), then run the probe-conditional open push.
fn drive_open_push(sync: SharedSync, base: String, snapshot: BridgeDocumentSnapshot) {
    if !acquire_push_busy(&sync) {
        schedule_once(PUSH_BUSY_RETRY_MS, move || {
            drive_open_push(sync, base, snapshot)
        });
        return;
    }
    open_push_attempt(sync, base, snapshot, RETRY_ONCE);
}

/// Probe `GET /api/mcp/version` for the daemon's live version `V` (NOT
/// `last_version()` — during bootstrap that is 0 while sync-reset already
/// bumped it, forcing a spurious 409), then conditionally push with
/// `baseVersion=V`. 409 → re-probe + retry once; still 409 → `note_conflict`
/// (its latch drives the observer's `sync-conflict`), release `push_busy`, no
/// `opened`. `push_busy` is HELD across the retry (single-flight preserved).
fn open_push_attempt(
    sync: SharedSync,
    base: String,
    snapshot: BridgeDocumentSnapshot,
    retries_left: u8,
) {
    let version_url = format!("{base}/api/mcp/version");
    let on_version: Rc<dyn Fn(String)> = {
        let sync = sync.clone();
        Rc::new(move |body: String| {
            let Some(v) = WebSyncClient::parse_version_probe(&body) else {
                // Daemon down / non-JSON — abort without wedging: release the
                // latch, leave open_pending (a later retry / reload recovers).
                release_push_busy(&sync);
                return;
            };
            let push_body = snapshot.push_body(v);
            let doc_url = format!("{base}/api/mcp/document");
            let on_resp: Rc<dyn Fn(String)> = {
                let sync = sync.clone();
                let base = base.clone();
                let snapshot = snapshot.clone();
                Rc::new(move |resp: String| {
                    if let Some(server_v) = WebSyncClient::parse_push_conflict(&resp) {
                        if retries_left > 0 {
                            // Re-probe + retry once, still holding push_busy.
                            open_push_attempt(
                                sync.clone(),
                                base.clone(),
                                snapshot.clone(),
                                retries_left - 1,
                            );
                            return;
                        }
                        if let Ok(mut s) = sync.try_borrow_mut() {
                            s.gate.note_conflict(server_v);
                            s.push_busy = false;
                        }
                        return;
                    }
                    if let Some(version) = WebSyncClient::parse_push_response(&resp) {
                        if let Ok(mut s) = sync.try_borrow_mut() {
                            snapshot.mark_pushed(&mut s.client, version);
                            let pair = snapshot.pair();
                            s.gate.note_synced(pair.0, pair.1);
                            s.push_busy = false;
                        }
                        return;
                    }
                    // Unrecognized (network/parse failure): release the latch.
                    release_push_busy(&sync);
                })
            };
            if !live_sync::post_json(&doc_url, &push_body, Some(on_resp)) {
                release_push_busy(&sync);
            }
        })
    };
    if !live_sync::get(&version_url, on_version) {
        release_push_busy(&sync);
    }
}

/// Acquire `push_busy` (waiting via self-reschedule), then push the snapshot.
fn drive_snapshot_push(
    sync: SharedSync,
    base: String,
    request_id: String,
    snapshot: BridgeDocumentSnapshot,
) {
    if !acquire_push_busy(&sync) {
        schedule_once(PUSH_BUSY_RETRY_MS, move || {
            drive_snapshot_push(sync, base, request_id, snapshot)
        });
        return;
    }
    snapshot_push_attempt(sync, base, request_id, snapshot);
}

/// Snapshot-channel push with `baseVersion = last_version()` (the established
/// baseline). Confirm → `mark_pushed` + `note_synced` + `snapshot-result`.
/// Conflict → `note_conflict` (observer will report `sync-conflict`) +
/// `snapshot-conflict` reply.
fn snapshot_push_attempt(
    sync: SharedSync,
    base: String,
    request_id: String,
    snapshot: BridgeDocumentSnapshot,
) {
    let base_version = sync
        .try_borrow()
        .map(|s| s.client.last_version())
        .unwrap_or(0);
    let push_body = snapshot.push_body(base_version);
    let doc_url = format!("{base}/api/mcp/document");
    let on_resp: Rc<dyn Fn(String)> = {
        let sync = sync.clone();
        Rc::new(move |resp: String| {
            if let Some(server_v) = WebSyncClient::parse_push_conflict(&resp) {
                if let Ok(mut s) = sync.try_borrow_mut() {
                    s.gate.note_conflict(server_v);
                    s.push_busy = false;
                }
                post_to_parent(&event_snapshot_conflict(&request_id, server_v));
                return;
            }
            if let Some(version) = WebSyncClient::parse_push_response(&resp) {
                let pair = snapshot.pair();
                if let Ok(mut s) = sync.try_borrow_mut() {
                    snapshot.mark_pushed(&mut s.client, version);
                    s.gate.note_synced(pair.0, pair.1);
                    s.push_busy = false;
                }
                post_to_parent(&event_snapshot_result(
                    &request_id,
                    &snapshot.externalized_json(),
                    pair.0,
                    pair.1,
                ));
                return;
            }
            release_push_busy(&sync);
        })
    };
    if !live_sync::post_json(&doc_url, &push_body, Some(on_resp)) {
        release_push_busy(&sync);
    }
}

/// Acquire `push_busy` (waiting via self-reschedule), then run the use-local
/// re-push.
fn drive_use_local_push(
    sync: SharedSync,
    base: String,
    request_id: String,
    snapshot: BridgeDocumentSnapshot,
    base_version: u64,
) {
    if !acquire_push_busy(&sync) {
        schedule_once(PUSH_BUSY_RETRY_MS, move || {
            drive_use_local_push(sync, base, request_id, snapshot, base_version)
        });
        return;
    }
    use_local_push_attempt(sync, base, request_id, snapshot, base_version, RETRY_ONCE);
}

/// Re-push the local document with `baseVersion = base_version` (the conflict's
/// server version). Confirm → `mark_pushed` + `note_synced` (clears conflict +
/// any pending open) + `conflict-resolved`. Second conflict → retry once with
/// the fresh server version (push_busy held); still failing → `note_conflict`
/// (keeps the gate's conflict version current for a later retry) +
/// `snapshot-conflict`.
fn use_local_push_attempt(
    sync: SharedSync,
    base: String,
    request_id: String,
    snapshot: BridgeDocumentSnapshot,
    base_version: u64,
    retries_left: u8,
) {
    let push_body = snapshot.push_body(base_version);
    let doc_url = format!("{base}/api/mcp/document");
    let on_resp: Rc<dyn Fn(String)> = {
        let sync = sync.clone();
        Rc::new(move |resp: String| {
            if let Some(server_v) = WebSyncClient::parse_push_conflict(&resp) {
                if retries_left > 0 {
                    use_local_push_attempt(
                        sync.clone(),
                        base.clone(),
                        request_id.clone(),
                        snapshot.clone(),
                        server_v,
                        retries_left - 1,
                    );
                    return;
                }
                if let Ok(mut s) = sync.try_borrow_mut() {
                    s.gate.note_conflict(server_v);
                    s.push_busy = false;
                }
                post_to_parent(&event_snapshot_conflict(&request_id, server_v));
                return;
            }
            if let Some(version) = WebSyncClient::parse_push_response(&resp) {
                if let Ok(mut s) = sync.try_borrow_mut() {
                    snapshot.mark_pushed(&mut s.client, version);
                    let pair = snapshot.pair();
                    s.gate.note_synced(pair.0, pair.1);
                    s.push_busy = false;
                }
                post_to_parent(&event_conflict_resolved(&request_id));
                return;
            }
            release_push_busy(&sync);
        })
    };
    if !live_sync::post_json(&doc_url, &push_body, Some(on_resp)) {
        release_push_busy(&sync);
    }
}

// ---------------------------------------------------------------------------
// Outbound-event observer
// ---------------------------------------------------------------------------

/// The SOLE emitter of `sync-conflict` / `opened` (drained from the gate's
/// consumable latches) and `dirty-changed` (on a `(generation, revision,
/// is_dirty)` triple change). If either RefCell is momentarily borrowed the
/// tick is skipped BEFORE the latches are drained, so no edge is lost.
fn observe_tick<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    sync: &SharedSync,
    last_triple: &LastTripleCell,
) {
    // Read the triple first: if `inner` is busy, skip WITHOUT draining latches.
    let Some(triple) = read_triple(inner) else {
        return;
    };
    // Drain the latches only once we can take the sync borrow; a failed borrow
    // here also leaves the latches intact for the next tick.
    let (opened, conflict) = {
        let Ok(mut s) = sync.try_borrow_mut() else {
            return;
        };
        (s.gate.take_opened_edge(), s.gate.take_conflict_edge())
    };

    if let Some(server_v) = conflict {
        post_to_parent(&op_editor_core::bridge_protocol::event_sync_conflict(
            triple.0, triple.1, server_v,
        ));
    }
    if let Some(gen) = opened {
        post_to_parent(&op_editor_core::bridge_protocol::event_opened(gen));
    }
    let changed = *last_triple.borrow() != Some(triple);
    if changed {
        *last_triple.borrow_mut() = Some(triple);
        post_to_parent(&op_editor_core::bridge_protocol::event_dirty_changed(
            triple.0, triple.1, triple.2,
        ));
    }
}
