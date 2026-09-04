//! Headless web-canvas daemon — serves the document to the Rust WASM web shell
//! (`op-host-web`, which runs in a browser and can't bind a socket) and to
//! external MCP/CLI clients. It is the Rust analog of the TS web app's
//! `apps/web/server/api/mcp/*` Nitro routes + `setSyncDocument`: it owns the
//! canonical document in memory and answers the same whole-document REST sync
//! shape, so a JSON-RPC/REST client (e.g. the `op` CLI or any MCP client) can
//! drive the Rust *web* canvas the same way it drives the desktop canvas.
//!
//! This module ships the request-handling CORE (`handle_web_canvas_request`,
//! fully unit-testable without a socket) plus a runnable loop
//! (`run_web_canvas`, behind
//! `openpencil-desktop --serve-web <port> [doc] [--host <addr>]`).
//! Layered on top: an SSE endpoint that streams `version` bumps to connected
//! shells, static serving of the host page + WASM bundle (`crate::web_static`
//! — `GET /` and `GET /pkg/*`), and a token-authenticated `openpencil/shutdown`
//! (same contract as `--mcp-http`) so `op stop` works against this daemon.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use base64::Engine as _;
use op_editor_core::agent_settings::{AcpAgentConnectOutcome, ProviderConnectOutcome};
use op_editor_core::{AgentSettings, EditorState};

use crate::web_canvas_server_error::WebCanvasError;
use crate::web_credential_policy::WebCredentialPersistence;

/// Every fallible step of this module fails with [`WebCanvasError`].
type Result<T> = std::result::Result<T, WebCanvasError>;

/// Slow/stalled-peer bound — bodies can be large (whole documents with embedded
/// images), so a connection that opens and dribbles must not pin a thread.
const IO_TIMEOUT: Duration = Duration::from_secs(30);

/// Max concurrent connections (SSE streams are long-lived, so bound them).
const MAX_CONNS: usize = 64;

/// SSE keep-alive cadence — also how quickly a disconnected SSE client is
/// detected (the heartbeat write fails once the socket is gone).
const SSE_HEARTBEAT: Duration = Duration::from_secs(15);

/// One SSE payload: the two sequence numbers a client polls on.
///
/// `version` changes only when document *content* changes, so it is what makes
/// a client refetch the document. `collab_seq` changes whenever the
/// collaboration projection changes — a peer joining, a cursor moving — and
/// must never by itself cause a document fetch.
///
/// Carried as one struct so a bump and its broadcast stay a single atomic
/// event. The serialized form stays a superset of the original `{"version":N}`
/// payload, so a client that only reads `version` is unaffected.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SseTick {
    pub version: u64,
    pub collab_seq: u64,
}

/// RAII decrement for the connection counter — `Drop` runs on normal exit AND
/// panic unwind, so a panicking connection can't leak its `MAX_CONNS` slot.
struct ConnGuard(Arc<AtomicUsize>);

impl Drop for ConnGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

/// In-memory document authority for the web-canvas daemon — the Rust mirror of
/// the TS `mcp-sync-state` (document + monotonic version). The browser shell
/// mirrors this (over the SSE endpoint, layered on later); external MCP clients
/// read/replace it over `/api/mcp/document`.
pub struct WebCanvasState {
    pub(crate) editor: EditorState,
    /// Immutable deployment policy for browser-supplied credentials. Public
    /// demo deployments fail closed to browser-only persistence.
    pub(crate) credential_persistence: WebCredentialPersistence,
    /// Local file backing this daemon document, when `--serve-web` was launched
    /// with a path or the user opened a recent local file through the daemon.
    pub(crate) current_path: Option<PathBuf>,
    /// Monotonic sync version, bumped on every document mutation — the key the
    /// browser shell uses to detect that the live document changed.
    pub(crate) version: u64,
    /// The bound port, reported by `GET /api/mcp/server` (TS `server.get.ts`
    /// parity).
    pub(crate) port: u16,
    /// Managed-mode per-instance lifecycle token (see
    /// `ServeWebOptions::managed` / `run_web_canvas`). `None` outside managed
    /// mode. Ordinary requests are tokenless; `serve_one` consults this only
    /// for the optional body-authenticated graceful-shutdown compatibility
    /// path.
    pub(crate) managed_token: Option<String>,
    /// Managed-mode `--allow-origin` allowlist. Empty outside managed mode.
    /// Read by `serve_one` (via `cors_origin_for`) to decide which `Origin`
    /// is echoed back in `Access-Control-Allow-Origin`.
    pub(crate) allow_origins: Vec<String>,
    /// Idempotence guard for `POST /api/mcp/sync-reset`: set once the FIRST
    /// successful reset in this process's lifetime completes. A failed reset
    /// does not set it (retryable); once set, further sync-reset calls are a
    /// no-op `{"ok":true,"skipped":true,"version":<current>}` reply instead
    /// of touching the document again.
    pub(crate) reset_consumed: bool,
    /// In-flight browser device-login flow driven through the auth proxy
    /// endpoints (`/api/auth/login/*`); `None` when no login is running.
    pub(crate) auth_login_handle: Option<u64>,
    /// Collaboration runtime + its projection sequence. Idle until a browser
    /// posts a start/join action.
    pub(crate) collab: collab_state::WebCollabState,
    /// How this daemon is deployed. `Local` for every existing invocation;
    /// `Online` is the only value that refuses anything, and it is what the
    /// REST tier consults for the routes that would otherwise share one
    /// process's filesystem, settings file and device session between
    /// mutually untrusting accounts. See `online_policy.rs`.
    pub(crate) mode: ServeMode,
}

impl WebCanvasState {
    #[cfg(test)]
    pub(crate) fn new(editor: EditorState, port: u16) -> Self {
        Self::new_with_path(editor, port, None)
    }

    #[cfg(test)]
    pub(crate) fn new_with_policy(
        editor: EditorState,
        port: u16,
        credential_persistence: WebCredentialPersistence,
    ) -> Self {
        Self::new_with_path_and_policy(editor, port, None, credential_persistence)
    }

    #[cfg(test)]
    pub(crate) fn new_with_path(
        editor: EditorState,
        port: u16,
        current_path: Option<PathBuf>,
    ) -> Self {
        Self::new_with_path_and_policy(
            editor,
            port,
            current_path,
            WebCredentialPersistence::BrowserOnly,
        )
    }

    fn new_with_path_and_policy(
        mut editor: EditorState,
        port: u16,
        current_path: Option<PathBuf>,
        credential_persistence: WebCredentialPersistence,
    ) -> Self {
        if !credential_persistence.server_persistence() {
            let _ = crate::web_credentials::remove_browser_owned_credentials(&mut editor);
        }
        Self {
            editor,
            credential_persistence,
            current_path,
            version: 0,
            port,
            managed_token: None,
            allow_origins: Vec::new(),
            reset_consumed: false,
            auth_login_handle: None,
            collab: collab_state::WebCollabState::default(),
            mode: ServeMode::Local,
        }
    }

    /// One online account's document authority.
    ///
    /// Separate from the loader-backed constructors on purpose: a tenant is
    /// never backed by a local path (the online mode serves no local files),
    /// and its browser-supplied credentials never reach the process settings
    /// file, so it always carries the browser-only persistence policy.
    pub(crate) fn new_for_tenant(editor: EditorState, port: u16) -> Self {
        let mut state = Self::new_with_path_and_policy(
            editor,
            port,
            None,
            WebCredentialPersistence::BrowserOnly,
        );
        state.mode = ServeMode::Online;
        state
    }

    /// Replace the whole document (an already-loaded `POST /api/mcp/document`
    /// body), bump and return the new version.
    pub(crate) fn replace_document(&mut self, doc: jian_ops_schema::PenDocument) -> u64 {
        self.editor.replace_document(doc);
        self.version += 1;
        self.version
    }

    /// Clear the transient web-sync document back to the same starter document a
    /// fresh browser shell paints before the daemon applies updates.
    pub(crate) fn reset_document(&mut self) -> Result<u64> {
        if let Some(path) = self.current_path.clone() {
            let mut next = crate::mcp_serve::load_editor_state(&path)?;
            preserve_web_canvas_preferences(&self.editor, &mut next);
            set_file_name_display(&mut next, &path);
            self.editor = next;
        } else {
            self.editor.replace_document(EditorState::starter().doc);
        }
        self.version += 1;
        Ok(self.version)
    }

    /// Idempotent wrapper around [`Self::reset_document`] for
    /// `POST /api/mcp/sync-reset`: the first SUCCESSFUL reset in this
    /// process's lifetime consumes `reset_consumed`; every call after that
    /// is a no-op (`skipped: true`) that neither reloads nor bumps the
    /// version. A failed reset does NOT consume the guard — it stays
    /// retryable, and the document/version are left exactly as
    /// `reset_document`'s own `?`-early-return already guarantees (nothing
    /// touched before the fallible load succeeds).
    pub(crate) fn reset_document_guarded(&mut self) -> Result<ResetOutcome> {
        if self.reset_consumed {
            return Ok(ResetOutcome { skipped: true });
        }
        // A reset swaps the whole document and starts a new generation, which
        // the M1 protocol cannot sequence — the peers would silently diverge.
        // `ReplaceDocument` is refused for every source once a session exists.
        self.gate_daemon_mutation(
            op_editor_core::CollabGateAction::ReplaceDocument,
            op_editor_core::CollabEditSource::ExternalSync,
        )
        .map_err(WebCanvasError::Collab)?;
        self.reset_document()?;
        self.reset_consumed = true;
        Ok(ResetOutcome { skipped: false })
    }

    /// Push a whole-document replacement (`POST /api/mcp/document` body).
    /// Consolidates the route's former inline parse-then-replace logic into
    /// one testable entry point. `base_version_override` is test-only — the
    /// real route always passes `None`, in which case an optional top-level
    /// `baseVersion` field inside `body` itself is consulted (extracted via
    /// serde, not a hand-rolled scan). When a base version is present and
    /// does not match the current version, the write is rejected WITHOUT
    /// mutating anything: that's a version verdict conveyed through
    /// `PushOutcome::applied == false`, not an `Err`. `Err` is reserved for
    /// the existing malformed-JSON / schema-validation failures, which stay
    /// client-fault → HTTP 400 at the route exactly as before.
    pub(crate) fn apply_document_push(
        &mut self,
        body: &str,
        base_version_override: Option<u64>,
    ) -> Result<PushOutcome> {
        let push = PendingDocumentPush::parse(body, self.mode)?;
        self.apply_prepared_document_push(push, base_version_override)
    }

    /// Apply an already-parsed push. Runs entirely under the state lock.
    ///
    /// Everything fallible and expensive — JSON parse, schema load, structural
    /// validation — happened in [`PendingDocumentPush::parse`] before the lock
    /// was taken. What remains is the part that genuinely needs exclusivity:
    /// re-checking `baseVersion` against the live counter, and installing.
    ///
    /// The `baseVersion` check MUST be here rather than at parse time: the
    /// version can move between parse and install, and checking it outside the
    /// lock would be a race that silently clobbers a concurrent write.
    pub(crate) fn apply_prepared_document_push(
        &mut self,
        push: PendingDocumentPush,
        base_version_override: Option<u64>,
    ) -> Result<PushOutcome> {
        let base_version = base_version_override.or(push.base_version);
        if let Some(expected) = base_version {
            if expected != self.version {
                return Ok(PushOutcome {
                    applied: false,
                    current_version: self.version,
                });
            }
        }
        // The gateway runs BEFORE the document is taken out of `push`, so a
        // refusal leaves `push` still owning it — and its `Drop` releases the
        // pending thumbnail seed. Taking first meant a gated push orphaned its
        // seed in the process-global side table.
        //
        // A metadata-only push carries no document and is not a document
        // mutation, so it is not gated; it is handled below.
        let mut push = push;
        if push.prepared.is_some() {
            self.gate_daemon_mutation(
                op_editor_core::CollabGateAction::Document(
                    op_editor_core::CollabDocumentMutation::NodePropertyBatch,
                ),
                op_editor_core::CollabEditSource::User,
            )
            .map_err(WebCanvasError::Collab)?;
        }
        // Taken rather than moved: taking `prepared` is what tells the `Drop`
        // this push reached the install.
        let editor_meta = push.take_editor_meta();
        let Some(prepared) = push.prepared.take() else {
            // Active-page changes do not mutate canonical document content.
            // Apply the scalar pair without replacing the identical document,
            // bumping its generation, or publishing a version that another
            // browser would install as a full undoable remote replacement.
            op_pen_loader::apply_editor_meta(&mut self.editor, editor_meta);
            return Ok(PushOutcome {
                applied: true,
                current_version: self.version,
            });
        };
        let version = if self.collab_session_is_active() {
            // Inside the session's local-edit capture: the session core diffs
            // the document and emits precise node operations, so pushing an
            // unchanged document produces no transaction at all.
            match self.ingest_document_in_session(prepared) {
                // Sequenced: publish it.
                collab_state::IngestOutcome::Committed => {
                    self.version = self.version.saturating_add(1);
                    self.version
                }
                // Accepted, but identical. Answering 200 is right — the
                // client's document IS the daemon's — but bumping the version
                // would make every peer refetch a document that did not
                // change.
                collab_state::IngestOutcome::NoChange => self.version,
                // The session did not take it. Reporting success here (which
                // is what the old `bool` did for `Failed`) left the browser
                // believing a write landed that had been discarded, with no
                // signal to resync.
                // Both mean "your push did not land, refetch". The browser's
                // recovery only fires on `version-conflict` WITH a `version`
                // (see `WebSyncClient::parse_push_conflict`), so both carry
                // the authoritative version — a bare refusal would leave the
                // tab latched with no way to resync.
                collab_state::IngestOutcome::Rejected => {
                    return Err(WebCanvasError::IngestRejected(
                        collab_state::IngestOutcome::Rejected,
                        self.version,
                    ));
                }
                collab_state::IngestOutcome::Failed => {
                    return Err(WebCanvasError::IngestRejected(
                        collab_state::IngestOutcome::Failed,
                        self.version,
                    ));
                }
            }
        } else {
            self.replace_document(prepared.into_document())
        };
        op_pen_loader::apply_editor_meta(&mut self.editor, editor_meta);
        Ok(PushOutcome {
            applied: true,
            current_version: version,
        })
    }

    /// The pair of sequence numbers a client polls on.
    pub(crate) const fn sse_tick(&self) -> SseTick {
        SseTick {
            version: self.version,
            collab_seq: self.collab.seq(),
        }
    }

    #[cfg(test)]
    pub(crate) fn document_version_for_test(&self) -> u64 {
        self.version
    }
}

/// Render an error, adding the machine-readable code when it has one.
///
/// A collaboration refusal is not a malformed request, so a client needs the
/// code to decide between retrying, refetching, and telling the user to leave
/// the session.
/// Outcome of [`WebCanvasState::reset_document_guarded`].
pub(crate) struct ResetOutcome {
    pub skipped: bool,
}

/// Outcome of [`WebCanvasState::apply_document_push`] — expresses only the
/// version verdict (parse/schema failures are `Err`, handled separately).
pub(crate) struct PushOutcome {
    pub applied: bool,
    pub current_version: u64,
}

/// A handled reply: HTTP status line + JSON body, ready for
/// `write_mcp_http_response`.
pub struct WebReply {
    pub(crate) status: &'static str,
    pub(crate) body: String,
}

fn persist_api_settings<F>(
    method: &str,
    path: &str,
    state: &mut WebCanvasState,
    settings_before: crate::settings_io::Fingerprint,
    credential_settings_before: Option<AgentSettings>,
    reply: WebReply,
    save_credentials: F,
) -> WebReply
where
    // `settings_io::save_checked` reports its own typed
    // `SettingsIoError` now; only its success/failure is consulted here, but
    // naming the type keeps the bound honest instead of erasing it to a
    // `String` the caller would have to re-render.
    F: FnOnce(&EditorState) -> std::result::Result<(), crate::settings_io::SettingsIoError>,
{
    // There is ONE settings file per process. In a shared deployment a
    // settings write would overwrite every other account's providers and
    // credentials with this account's, so nothing is persisted: the change
    // still lands in this tenant's in-memory editor and dies with it.
    if !state.mode.allows_settings_persistence() {
        return reply;
    }
    if method == "POST" && path == "/api/settings/credentials" && reply.status == "200 OK" {
        if save_credentials(&state.editor).is_err() {
            if let Some(settings) = credential_settings_before {
                state.editor.editor_ui.agent_settings = settings;
                state.editor.rebuild_chat_models();
            }
            return WebReply {
                status: "500 Internal Server Error",
                body: crate::mcp_serve::rest_error_body("failed to persist server credentials"),
            };
        }
    } else {
        crate::settings_io::save_if_changed(&state.editor, settings_before);
    }
    reply
}

/// Handle one parsed web-canvas REST request against the in-memory state. Pure
/// w.r.t. IO — fully unit-testable without a socket. Mirrors the TS Nitro
/// routes:
/// - `GET  /api/mcp/server`   → health `{ok:true,…}` (like `server.get.ts`)
/// - `GET  /api/mcp/document` → `{document:<doc>,version}` (like `document.get.ts`)
/// - `POST /api/mcp/document` → whole-doc replace → `{ok:true,version}` (like
///   `document.post.ts`); an optional top-level `baseVersion` makes the write
///   conditional — a stale value 409s with `{ok:false,error:"version-conflict",
///   version}` and leaves the document untouched instead of clobbering it
/// - `GET  /api/mcp/version`  → `{version}` — Rust-only cheap change probe; the
///   TS stack pushes documents over SSE instead, so it never needs one. The
///   browser shell polls this and fetches the full document only on a bump.
/// - `GET  /api/mcp/selection` → `{selectedIds,activePageId}` (like `selection.get.ts`)
/// - `POST /api/mcp/selection` → renderer selection push (like `selection.post.ts`)
/// - `POST /api/file/save` → write the browser document to this daemon's
///   backing local path, including embedded active-page metadata
///   (and legacy `.opmeta` fallback compatibility)
/// - `POST /api/file/open-recent` → local-daemon recent-file open, used by the
///   browser shell because only the daemon can read local paths.
/// - anything else → 404 (the JSON-RPC `/mcp` path + SSE are handled by the
///   caller's connection loop, not here).
pub fn handle_web_canvas_request(
    method: &str,
    path: &str,
    body: &str,
    state: &mut WebCanvasState,
) -> WebReply {
    match (method, path) {
        ("GET", "/api/mcp/server") => WebReply {
            status: "200 OK",
            // `{running,port,localIp}` matches TS `server.get.ts`; the daemon
            // binds 127.0.0.1 (localhost-only) so localIp is loopback. Extra
            // `server`/`mode` fields are additive diagnostics.
            // `serveMode` is additive: the browser reads it to learn whether
            // the daemon is the sole sequencer for this document (online) or
            // merely a peer holding the operator's file (local/managed). That
            // decides whether a sync conflict may be auto-resolved — see
            // `op-host-web/src/live_sync_glue.rs::auto_resolve_is_safe`.
            body: format!(
                r#"{{"running":true,"port":{},"localIp":"127.0.0.1","server":"openpencil-mcp","mode":"web-canvas","serveMode":"{}"}}"#,
                state.port,
                state.mode.wire_name()
            ),
        },
        ("POST", "/api/mcp/server") => update_mcp_server_settings(body, state),
        ("GET", "/api/mcp/document") => match serde_json::to_string(&state.editor.doc) {
            Ok(doc_json) => WebReply {
                status: "200 OK",
                body: format!(
                    r#"{{"document":{doc_json},"version":{},"activePageIndex":{},"preserveAuthoredGeometry":{},"scenario":{}}}"#,
                    state.version,
                    state.editor.ui.active_page_index,
                    state.editor.editor_ui.preserve_authored_geometry,
                    // The scene tag rides the same wire as the rest of the
                    // editor meta: a Slides deck must reach the browser as a
                    // deck or web preview cannot enter its presentation.
                    match state.editor.editor_ui.scenario {
                        Some(scene) => format!("\"{}\"", scene.as_str()),
                        None => "null".to_string(),
                    }
                ),
            },
            Err(e) => WebReply {
                status: "500 Internal Server Error",
                body: crate::mcp_serve::rest_error_body(&e.to_string()),
            },
        },
        ("POST", "/api/mcp/document") => match state.apply_document_push(body, None) {
            Ok(outcome) if outcome.applied => WebReply {
                status: "200 OK",
                body: crate::mcp_serve::document_sync_ok(outcome.current_version),
            },
            Ok(outcome) => WebReply {
                // Stale baseVersion: reject without writing, TS-style error
                // envelope plus the current version so the caller can decide
                // whether to refetch and retry.
                status: "409 Conflict",
                body: serde_json::json!({
                    "ok": false,
                    "error": "version-conflict",
                    "version": outcome.current_version,
                })
                .to_string(),
            },
            Err(error) => collab_aware_error_reply(&error),
        },
        ("GET", "/api/mcp/version") => WebReply {
            // `collabSeq` rides along so the existing 400 ms version poll also
            // notices collaboration changes without a second request. Adding a
            // field is backward compatible: a client that reads only `version`
            // is unaffected.
            status: "200 OK",
            body: format!(
                r#"{{"version":{},"collabSeq":{}}}"#,
                state.version,
                state.collab.seq()
            ),
        },
        ("GET", "/api/mcp/indicators") => WebReply {
            // Agent-indicator relay: design runs execute inside this
            // daemon, so the process-global registry the canvas paints
            // from lives HERE — the browser polls this and mirrors it
            // into its own registry (agent_indicators::apply_remote) so
            // agent borders / badges / reveal animations show on web.
            //
            // That registry has no tenant dimension, so a shared deployment
            // relays the empty projection instead of showing one account the
            // shape of another account's design run.
            status: "200 OK",
            body: if state.mode.allows_agent_indicator_relay() {
                op_editor_core::agent_indicators::relay_json()
            } else {
                online_policy::EMPTY_INDICATOR_RELAY.to_string()
            },
        },
        // The wasm shell posts a sync-reset on every mount. Locally that
        // means "the browser just booted, drop the transient document";
        // online it would wipe the document the returning account left
        // behind, so the route answers with the already-reset shape — the
        // same body a second local reset produces — and touches nothing.
        ("POST", "/api/mcp/sync-reset") if state.mode.sync_reset_is_noop() => WebReply {
            status: "200 OK",
            body: format!(
                r#"{{"ok":true,"skipped":true,"version":{}}}"#,
                state.version
            ),
        },
        ("POST", "/api/mcp/sync-reset") => match state.reset_document_guarded() {
            Ok(outcome) if outcome.skipped => WebReply {
                status: "200 OK",
                body: format!(
                    r#"{{"ok":true,"skipped":true,"version":{}}}"#,
                    state.version
                ),
            },
            Ok(_) => WebReply {
                status: "200 OK",
                body: crate::mcp_serve::document_sync_ok(state.version),
            },
            Err(e) if e.error_code().is_some() => collab_aware_error_reply(&e),
            Err(e) => WebReply {
                status: e.http_status(),
                body: crate::mcp_serve::rest_error_body(&format!("sync reset failed: {e}")),
            },
        },
        ("GET", "/api/mcp/selection") => {
            // TS `selection.get.ts` → `getSyncSelection()` shape:
            // `{selectedIds, activePageId}`. Read straight off the live
            // editor selection so MCP clients and the REST route agree.
            let ids: Vec<&str> = state
                .editor
                .selection
                .set
                .iter()
                .map(|id| id.as_str())
                .collect();
            let active_page_id = state
                .editor
                .doc
                .pages
                .as_ref()
                .and_then(|pages| pages.get(state.editor.ui.active_page_index))
                .map(|page| page.id.clone());
            let body = serde_json::json!({
                "selectedIds": ids,
                "activePageId": active_page_id,
            });
            WebReply {
                status: "200 OK",
                body: serde_json::to_string(&body)
                    .unwrap_or_else(|_| r#"{"selectedIds":[],"activePageId":null}"#.to_string()),
            }
        }
        ("POST", "/api/mcp/selection") => apply_selection_sync(body, state),
        // Both file routes touch the daemon host's filesystem, which in a
        // shared process means every account reading and writing through the
        // service account's paths. Refused before the handler, not inside it.
        ("POST", "/api/file/save" | "/api/file/open-recent")
            if !state.mode.allows_local_file_routes() =>
        {
            online_policy::refusal_reply(online_policy::OnlineRouteRefusal::LocalFileAccess)
        }
        // A save reply REPLACES `state.editor` (it installs the document it
        // just wrote, plus its metadata), so it is a whole-document swap and
        // has to clear the same gate `open-recent` does. Swapping the document
        // out from under a live session would leave the peers editing a
        // document this daemon no longer has.
        ("POST", "/api/file/save") => match state.gate_daemon_mutation(
            op_editor_core::CollabGateAction::ReplaceDocument,
            op_editor_core::CollabEditSource::ExternalSync,
        ) {
            Ok(()) => save_current_file(body, state),
            Err(refusal) => WebReply {
                status: refusal.http_status(),
                body: serde_json::json!({
                    "ok": false,
                    "error": refusal.code(),
                    "message": refusal.to_string(),
                })
                .to_string(),
            },
        },
        ("POST", "/api/file/open-recent") => open_recent_file(body, state),
        ("POST", "/api/export/pdf") => export_pdf_download(body, state),
        ("POST", "/api/export/raster") => export_raster_download(body, state),
        ("GET", "/api/ai/models") => WebReply {
            // JSON array of model ids the AI proxy can serve (the
            // configured built-in agents). The web bundle queries this
            // to populate its model picker without bundling a static
            // list or holding API keys. `POST /api/ai/stream` is a
            // streaming route handled in the connection loop, not here.
            status: "200 OK",
            body: crate::ai_proxy::models_json(&state.editor),
        },
        ("GET", "/api/settings/credential-policy") => WebReply {
            status: "200 OK",
            body: serde_json::json!({
                "serverPersistence": state.credential_persistence.server_persistence(),
            })
            .to_string(),
        },
        ("POST", "/api/settings/credentials") => {
            if !state.credential_persistence.server_persistence() {
                WebReply {
                    status: "403 Forbidden",
                    body: crate::mcp_serve::rest_error_body(
                        "server credential persistence is disabled",
                    ),
                }
            } else {
                match crate::web_credentials::apply_json(&mut state.editor, body) {
                    Ok(()) => WebReply {
                        status: "200 OK",
                        body: r#"{"ok":true}"#.into(),
                    },
                    // The oversize verdict now rides on the error itself
                    // (`is_payload_too_large`) instead of this route
                    // re-measuring the body against the cap — same 413/400
                    // split, one owner for the threshold.
                    Err(error) => WebReply {
                        status: if error.is_payload_too_large() {
                            "413 Payload Too Large"
                        } else {
                            "400 Bad Request"
                        },
                        body: crate::mcp_serve::rest_error_body(&error.to_string()),
                    },
                }
            }
        }
        // Device-login proxy: the wasm bundle ships no auth code and
        // drives the flow through the daemon's op-auth-bridge runtime.
        // (`POST /api/auth/login/begin` is a streaming-tier route — it
        // waits for the verification URI off the state lock.)
        //
        // The bridge holds ONE device session per process, so online the
        // routes fall through to the 404 arm: an account signs in to the
        // hub, not to this daemon, and proxying the service account's
        // session would sign every visitor in as it.
        (_, path) if !state.mode.allows_device_login_proxy() && is_device_login_route(path) => {
            not_found_reply()
        }
        ("GET", op_editor_core::auth_routes::STATUS) => crate::web_auth::status(state),
        ("GET", op_editor_core::auth_routes::LOGIN_STATUS) => crate::web_auth::login_status(state),
        ("POST", op_editor_core::auth_routes::LOGIN_CANCEL) => crate::web_auth::login_cancel(state),
        ("POST", op_editor_core::auth_routes::LOGOUT) => crate::web_auth::logout(state),
        // Collaboration: the runtime lives in this daemon, the panel in the
        // browser. See `web_canvas_server/collab_routes.rs`.
        ("GET", op_editor_core::collab_routes::STATE) => collab_routes::state(state),
        ("POST", op_editor_core::collab_routes::ACTION) => collab_routes::action(body, state),
        ("POST", op_editor_core::collab_routes::PRESENCE) => collab_routes::presence(body, state),
        _ => not_found_reply(),
    }
}

/// The daemon's canonical unknown-route reply.
fn not_found_reply() -> WebReply {
    WebReply {
        status: "404 Not Found",
        body: r#"{"ok":false,"error":"Not found. Use /api/mcp/document, /api/mcp/sync-reset, /api/mcp/server, /api/file/save, /api/export/raster, /api/export/pdf, or /mcp."}"#
            .to_string(),
    }
}

/// Whether `path` belongs to the daemon-hosted device-login proxy.
fn is_device_login_route(path: &str) -> bool {
    path.starts_with(op_editor_core::auth_routes::API_PREFIX)
}

mod collab_driver;
mod document_push;
#[cfg(test)]
pub(crate) use document_push::collab_aware_error_reply_for_test;
pub(crate) use document_push::{document_push_reply, PendingDocumentPush};
// Spine-local: the shared coded-error renderer for the document routes.
use document_push::collab_aware_error_reply;
mod sse_hub;
pub use sse_hub::SseHub;
pub(crate) use sse_hub::SseSlot;
mod collab_routes;
pub(crate) mod collab_state;
mod connect_routes;
mod connection;
mod connection_ai_routes;
mod doc_routes;
mod export_routes;
mod hub_verifier;
pub mod online_policy;
mod online_run_loop;
mod origin_guard;
mod run_loop;
mod serve_options;
mod share_routes;
pub mod tenant;
pub mod tenant_auth;
pub mod tenant_store;
mod tool_scopes;

pub use collab_state::{DaemonMutationRefusal, IngestOutcome};
pub use connect_routes::*;
use connection::*;
use doc_routes::*;
use export_routes::*;
pub use hub_verifier::HubVerifier;
pub use online_policy::{OnlineRouteRefusal, ServeMode};
pub use online_run_loop::*;
use origin_guard::*;
pub use run_loop::*;
pub use serve_options::*;
pub use share_routes::ShareError;
pub use tenant::{TenantError, TenantLease, TenantLimits, TenantRegistry, WriteBarrier, WritePass};
pub use tenant_auth::{
    IdentityVerifier, OnlineAuthError, PresentedCredentials, ResolvedIdentity, StaticVerifier,
};
pub use tenant_store::{TenantStore, TenantStoreError};

/// Serialises the tests that mutate `jian_ops_schema::image_thumbs`.
///
/// The registry is process-global and both `restore_snapshot` and
/// `clear_registry` replace the whole table, so two such tests running in
/// parallel wipe each other's entries — a failure with nothing to do with what
/// either test asserts. Unique ids are not enough; the replacement is
/// wholesale.
#[cfg(test)]
pub(crate) fn lock_image_thumb_registry() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|poison| poison.into_inner())
}

#[cfg(test)]
#[path = "web_canvas_server_tests.rs"]
mod tests;
