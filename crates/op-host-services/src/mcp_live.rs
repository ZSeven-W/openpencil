//! In-process MCP HTTP server for the live desktop editor.
//!
//! The CLI-only `mcp_serve` path owns a file-backed `EditorState`.
//! This module keeps the GUI as the source of truth: the server thread
//! requests a fresh snapshot from the UI thread for each HTTP request,
//! then sends write commands back for the UI thread to apply.

use std::collections::VecDeque;
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use op_editor_core::{
    CollabEditSource, CollabGateAction, CollabGatePolicy, EditorCommand, EditorState,
};

pub mod error;
#[cfg(feature = "mcp-debug-tools")]
pub mod screenshot;

pub use design_md_route::{DesignMdResponder, DesignMdResponseError, PendingDesignMdRequest};
pub use error::McpLiveError;

/// Per-request budget for a UI-thread snapshot/apply ack. Sized to cover a
/// large single editor operation without the connection giving up; the CLI
/// allows even longer per tool call (`POST_TIMEOUT`).
const UI_ACK_TIMEOUT: Duration = Duration::from_secs(15);
const ACCEPT_IDLE_SLEEP: Duration = Duration::from_millis(25);
/// Cap on concurrently-served connections, so a flood of localhost connects
/// can't exhaust threads/memory. Excess connections are shed with a 503.
const MAX_LIVE_CONNS: usize = 64;
/// Maximum UI-thread MCP requests to drain in one redraw. A client that
/// enqueues a large burst should not be able to monopolize the frame.
const UI_PUMP_REQUEST_BUDGET: usize = 8;
/// Per-request HTTP connection stack. Deep JSON deserialization is handled by
/// `serde_stacker`; reserving hundreds of MB per short-lived localhost request
/// makes large live-canvas syncs look like runaway desktop memory growth.
const LIVE_CONN_STACK_SIZE: usize = 16 * 1024 * 1024;
const _: () = assert!(LIVE_CONN_STACK_SIZE <= 16 * 1024 * 1024);
type UiWake = Arc<dyn Fn() + Send + Sync + 'static>;

pub struct McpLiveServer {
    port: u16,
    /// Per-instance identity token, reported in the live `ping` reply and
    /// written into `~/.openpencil/.op-mcp-port`. Lets the `op` CLI confirm
    /// the server it pings is the exact instance that wrote the discovery
    /// file (defeats stale-file / port-reuse / pid-recycle confusion).
    token: String,
    /// Set by a connection thread when a token-authed `openpencil/shutdown`
    /// arrives; the UI thread polls it (`shutdown_requested`) and exits the
    /// event loop, so `op stop` quits the editor cleanly without a signal.
    quit_flag: Arc<AtomicBool>,
    req_rx: Receiver<UiRequest>,
    stop_tx: Sender<()>,
    /// The MCP write → canvas-indicator session this loop shares with its
    /// connection threads — the client-declared badge identity (sniffed
    /// off `initialize` on whichever connection thread handles it) plus
    /// the epoch `pump` last registered, reused while its reveal queue
    /// drains so a burst of writes reads as one continuous run. See
    /// `client_write.rs` — the serve-web daemon keeps the same session
    /// so the two request loops cannot drift.
    write_indicators: Arc<Mutex<ClientWriteIndicators>>,
    /// Validated extension evidence waiting for the desktop's asynchronous
    /// provider worker. Kept separate from `EditorState`: this route neither
    /// reads nor mutates the live document.
    pending_design_md: VecDeque<PendingDesignMdRequest>,
}

struct ApplyAck {
    applied: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct McpPumpOutcome {
    pub repaint: bool,
    pub layout_dirty: bool,
    /// Set when this pump swapped in a whole new document
    /// (`UiRequest::ReplaceDocument` → `EditorState::replace_document`).
    /// `replace_document` resets `document_revision()` back to 0, which can
    /// alias a previously-scanned revision on an unrelated document — the
    /// caller MUST invalidate any revision-keyed cache (e.g. the desktop
    /// host's image-search scan gate) whenever this is `true`, the same way
    /// it already does after a Figma import installs a fresh `EditorState`.
    pub document_replaced: bool,
}

enum UiRequest {
    Snapshot {
        ack: SyncSender<EditorState>,
    },
    /// Lightweight read snapshot for `list_pages`. Copying only page metadata
    /// avoids cloning every node in a large live document just to report page
    /// ids, names, and the active index.
    ListPages {
        ack: SyncSender<op_mcp::ListPages>,
    },
    Apply {
        /// The MCP tool that produced `cmd` — carried for the boundary
        /// log the pump emits when the write lands canvas indicators.
        tool_name: String,
        cmd: EditorCommand,
        ack: SyncSender<ApplyAck>,
    },
    /// Whole-document sync (TS REST `/api/mcp/document` parity). The document
    /// is parsed/loaded OFF the UI thread (so a large document never pins the
    /// UI thread, and the stateful lock is held only for the swap); the UI pump
    /// just swaps the already-loaded document into the live `EditorState`. The
    /// The bool ack is false when the current collaboration session rejects
    /// whole-document replacement.
    ReplaceDocument {
        doc: Box<jian_ops_schema::PenDocument>,
        editor_meta: op_pen_loader::EditorMeta,
        ack: SyncSender<bool>,
    },
    /// Editor-only live-sync update. Page switches must not install an
    /// identical whole document as an undoable replacement.
    UpdateEditorMeta {
        editor_meta: op_pen_loader::EditorMeta,
        ack: SyncSender<()>,
    },
    GenerateDesignMd {
        request: PendingDesignMdRequest,
    },
    /// `debug_screenshot` against the LIVE canvas — the connection
    /// thread blocks on `ack` while the UI pump renders the active
    /// page / node through the raster export pipeline
    /// (`export::screenshot::capture`). Same pending-intent +
    /// bounded-wait discipline as the chat canvas tools.
    #[cfg(feature = "mcp-debug-tools")]
    Screenshot {
        spec: crate::export::screenshot::CaptureSpec,
        ack: SyncSender<
            Result<crate::export::screenshot::ScreenshotPng, crate::export::ExportError>,
        >,
    },
}

impl McpLiveServer {
    #[doc(hidden)]
    pub fn start(port: u16) -> Result<Self, McpLiveError> {
        Self::start_with_wake(port, || {})
    }

    pub fn start_with_wake<F>(port: u16, wake_ui: F) -> Result<Self, McpLiveError>
    where
        F: Fn() + Send + Sync + 'static,
    {
        let listener = TcpListener::bind(("127.0.0.1", port))
            .map_err(|e| McpLiveError::Startup(format!("bind 127.0.0.1:{port}: {e}")))?;
        let bound_port = listener
            .local_addr()
            .map_err(|e| McpLiveError::Startup(format!("read bound MCP port: {e}")))?
            .port();
        listener
            .set_nonblocking(true)
            .map_err(|e| McpLiveError::Startup(format!("set nonblocking: {e}")))?;
        let (req_tx, req_rx) = mpsc::channel();
        let (stop_tx, stop_rx) = mpsc::channel();
        let token = make_live_token();
        // The connection threads need both halves of the admission material:
        // the token they must see on every state-touching request, and the
        // port they actually bound (so `Host` can be pinned to it — the
        // DNS-rebinding check). Bundled so `server_loop`'s argument list
        // stays the same width.
        let admission = Arc::new(LiveAdmission::new(token.clone(), bound_port));
        let quit_flag = Arc::new(AtomicBool::new(false));
        let server_quit = Arc::clone(&quit_flag);
        let wake_ui: UiWake = Arc::new(wake_ui);
        let write_indicators = Arc::new(Mutex::new(ClientWriteIndicators::default()));
        let server_indicators = Arc::clone(&write_indicators);
        thread::Builder::new()
            .name("op-mcp-live-http".into())
            .spawn(move || {
                server_loop(
                    listener,
                    req_tx,
                    stop_rx,
                    admission,
                    server_quit,
                    wake_ui,
                    server_indicators,
                )
            })
            .map_err(|e| McpLiveError::Startup(format!("spawn MCP live server: {e}")))?;
        eprintln!("openpencil-desktop mcp: listening on 127.0.0.1:{bound_port}/mcp");
        Ok(Self {
            port: bound_port,
            token,
            quit_flag,
            req_rx,
            stop_tx,
            write_indicators,
            pending_design_md: VecDeque::new(),
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Identity token to publish in the discovery file.
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Whether a token-authed `openpencil/shutdown` was received — the UI
    /// thread should exit the event loop.
    pub fn shutdown_requested(&self) -> bool {
        self.quit_flag.load(Ordering::Acquire)
    }

    pub fn pump(&mut self, state: &mut EditorState) -> McpPumpOutcome {
        let mut outcome = McpPumpOutcome::default();
        for _ in 0..UI_PUMP_REQUEST_BUDGET {
            match self.req_rx.try_recv() {
                Ok(UiRequest::Snapshot { ack }) => {
                    // Narrowed clone: the MCP registry + applier never read
                    // `chat` / `codegen` / `theme_presets`, and those are the
                    // sub-states that grow with SESSION length (transcripts,
                    // generated source, saved preset tables) rather than with
                    // the document — so copying them into every tool call's
                    // snapshot is pure UI-thread stall. See
                    // `op_editor_core::request_snapshot` for the field audit
                    // and for why an `Arc` can't replace this clone (the
                    // connection thread re-applies the ack'd command to its
                    // own copy, and the live state keeps mutating here).
                    let _ = ack.send(op_editor_core::request_snapshot::narrowed_snapshot(state));
                }
                Ok(UiRequest::ListPages { ack }) => {
                    let _ = ack.send(op_mcp::list_pages_snapshot(state));
                }
                Ok(UiRequest::Apply {
                    tool_name,
                    cmd,
                    ack,
                }) => {
                    if let Err(reason) = CollabGatePolicy::from(&state.editor_ui.collab)
                        .check_command(&cmd, CollabEditSource::Mcp)
                    {
                        state.editor_ui.collab.set_notice(
                            reason.notice_kind(),
                            crate::design_agent_tools::reveal_now_millis(),
                        );
                        let _ = ack.send(ApplyAck { applied: false });
                        outcome.repaint = true;
                        continue;
                    }
                    let layout_dirty = command_mutates_document(&cmd);
                    // The id-diff snapshot that powers the indicators'
                    // new-node reveal sweep — taken before apply, only
                    // for commands that can change the active tree's id
                    // set (a `SetNodeFillHex` never justifies the walk).
                    let can_retree = command_can_change_id_set(&cmd);
                    let ids_before = can_retree
                        .then(|| crate::design_agent_tools::collect_active_node_ids(state));
                    // The page the diff was taken on — an apply that moves
                    // the active page must not read its pre-existing tree
                    // as fresh content.
                    let page_before = can_retree.then(|| active_page_identity(state)).flatten();
                    let touched_ids = op_editor_core::affected_node_ids(&cmd);
                    let applied = state.apply(cmd);
                    if applied {
                        let write_repairs = crate::mcp_serve::run_write_repairs_after_apply(state);
                        let ids_before = ids_before.filter(|_| {
                            page_before.is_none() || active_page_identity(state) == page_before
                        });
                        // Canvas presence for the nodes this write touched /
                        // created — the same `register_applied_writes` the
                        // serve-web daemon runs, so every node-mutating MCP
                        // write shows where the client just worked (a
                        // targeted update focuses the cursor without a
                        // pop; fresh content gets the full reveal sweep).
                        let epoch = self
                            .write_indicators
                            .lock()
                            .unwrap_or_else(|poison| poison.into_inner())
                            .register_applied_writes(
                                state,
                                ids_before.as_ref(),
                                &touched_ids,
                                EpochReuse::WhileDraining,
                                crate::design_agent_tools::reveal_now_millis(),
                            );
                        if let Some(epoch) = epoch {
                            eprintln!(
                                "openpencil-desktop mcp: {tool_name}: canvas indicators epoch {epoch}"
                            );
                            // Each call is its own self-contained turn —
                            // the desktop's paint drains THIS process's
                            // registry directly, so finishing now is safe:
                            // `run_active` drops immediately but the
                            // already-queued reveals keep animating at
                            // their own pace (paint-side maintenance
                            // retires the epoch once the queue empties).
                            op_editor_core::agent_indicators::finish_if_epoch(epoch);
                        }
                        outcome.layout_dirty |= write_repairs;
                    }
                    let _ = ack.send(ApplyAck { applied });
                    if applied {
                        outcome.repaint = true;
                        outcome.layout_dirty |= layout_dirty;
                    }
                }
                Ok(UiRequest::ReplaceDocument {
                    doc,
                    editor_meta,
                    ack,
                }) => {
                    if let Err(reason) = CollabGatePolicy::from(&state.editor_ui.collab)
                        .check(CollabGateAction::ReplaceDocument, CollabEditSource::Mcp)
                    {
                        state.editor_ui.collab.set_notice(
                            reason.notice_kind(),
                            crate::design_agent_tools::reveal_now_millis(),
                        );
                        let _ = ack.send(false);
                        outcome.repaint = true;
                        continue;
                    }
                    // The document was already loaded off the UI thread; just
                    // swap it in (preserving editor chrome). Layout is computed
                    // by the renderer at paint, so the swap + dirty flag
                    // repaints the new document with no extra layout pass here.
                    state.replace_document(*doc);
                    op_pen_loader::apply_editor_meta(state, editor_meta);
                    let _ = ack.send(true);
                    outcome.repaint = true;
                    outcome.layout_dirty = true;
                    outcome.document_replaced = true;
                }
                Ok(UiRequest::UpdateEditorMeta { editor_meta, ack }) => {
                    op_pen_loader::apply_editor_meta(state, editor_meta);
                    let _ = ack.send(());
                    outcome.repaint = true;
                    outcome.layout_dirty = true;
                }
                Ok(UiRequest::GenerateDesignMd { request }) => {
                    self.pending_design_md.push_back(request);
                }
                #[cfg(feature = "mcp-debug-tools")]
                Ok(UiRequest::Screenshot { spec, ack }) => {
                    // Read-only render of the live state — no repaint needed.
                    let _ = ack.send(crate::export::screenshot::capture(state, &spec));
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        outcome
    }

    /// Take the next extension design-analysis request for asynchronous
    /// provider execution by the desktop host. At most one can exist globally,
    /// but this queue-shaped API keeps the UI pump non-blocking.
    pub fn take_pending_design_md_request(&mut self) -> Option<PendingDesignMdRequest> {
        self.pending_design_md.pop_front()
    }

    pub fn stop(&mut self) {
        let _ = self.stop_tx.send(());
    }
}

impl Drop for McpLiveServer {
    fn drop(&mut self) {
        self.stop();
    }
}

mod admission;
pub(crate) mod client_write;
mod connection;
mod design_md_output;
pub(crate) mod design_md_route;
mod doc_sync;
/// `pub(crate)` for two consts only: the shared HTTP parser
/// (`mcp_serve::read_http_request`) has to recognise this route's path to
/// apply its smaller body cap before reading a body.
pub(crate) mod snapshot_ingest;
mod ui_requests;

use admission::*;
// `pub(crate)` so the serve-web daemon reuses the same MCP write →
// canvas-indicator session (`crate::mcp_live::ClientWriteIndicators` &
// friends) instead of growing a second copy of the semantics.
pub(crate) use client_write::*;
use connection::*;
use doc_sync::*;
use ui_requests::*;

#[cfg(test)]
#[path = "mcp_live_cursor_tests.rs"]
mod cursor_tests;
#[cfg(test)]
#[path = "mcp_live_tests.rs"]
mod tests;
