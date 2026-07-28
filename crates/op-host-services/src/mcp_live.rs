//! In-process MCP HTTP server for the live desktop editor.
//!
//! The CLI-only `mcp_serve` path owns a file-backed `EditorState`.
//! This module keeps the GUI as the source of truth: the server thread
//! requests a fresh snapshot from the UI thread for each HTTP request,
//! then sends write commands back for the UI thread to apply.

use std::collections::HashSet;
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use jian_ops_schema::node::PenNode;
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::{
    CollabEditSource, CollabGateAction, CollabGatePolicy, EditorCommand, EditorState,
};

pub mod error;
#[cfg(feature = "mcp-debug-tools")]
pub mod screenshot;

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

/// Fallback client-facing label when `initialize` carried no
/// `clientInfo.name` (some minimal/older MCP clients omit it) — honest
/// about what it is ("some external MCP tool"), not a fabricated persona.
const MCP_CLIENT_FALLBACK_NAME: &str = "MCP Client";
/// Fixed neutral badge colour for every MCP-driven generation — a
/// deliberate contrast with the in-app agent persona pool
/// (`assign_agent_identities_seeded`'s vivid per-agent colours): an
/// external MCP client isn't one of the app's own named agents.
const MCP_CLIENT_COLOR: &str = "#8A8F98";

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
    /// The connected MCP client's declared identity — `(name, color)`,
    /// captured from `initialize`'s `params.clientInfo.name` the first
    /// time a connection thread sees it (`Arc<Mutex<_>>`: written from
    /// whichever connection thread handles that request, read from the
    /// UI thread in `pump` when tagging a `batch_design` root). A fixed
    /// neutral colour, not the in-app persona pool — an external MCP
    /// client is not one of the app's own named agents.
    client_identity: Arc<Mutex<Option<(String, String)>>>,
    /// The last epoch this server minted for a `batch_design` write, or
    /// `0` before the first one. `pump`'s indicator hook reuses it
    /// (rather than starting a fresh epoch) while its reveal queue is
    /// still draining, so a burst of back-to-back `batch_design` calls
    /// reads as one continuous generation instead of each call clipping
    /// the previous one's tail animation. UI-thread-only (`pump` takes
    /// `&mut self`), so a plain field — no interior mutability needed.
    last_mcp_epoch: u64,
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
        /// The MCP tool that produced `cmd` — `McpLiveServer::pump` reads
        /// this to gate canvas-generation indicators (frame glow + reveal
        /// sweep) on `"batch_design"` specifically, mirroring how the
        /// in-app design-agent loop only animates that one tool
        /// (`design_agent_tools.rs::should_register_batch_reveals`).
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
        let server_token = token.clone();
        let quit_flag = Arc::new(AtomicBool::new(false));
        let server_quit = Arc::clone(&quit_flag);
        let wake_ui: UiWake = Arc::new(wake_ui);
        let client_identity = Arc::new(Mutex::new(None));
        let server_identity = Arc::clone(&client_identity);
        thread::Builder::new()
            .name("op-mcp-live-http".into())
            .spawn(move || {
                server_loop(
                    listener,
                    req_tx,
                    stop_rx,
                    server_token,
                    server_quit,
                    wake_ui,
                    server_identity,
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
            client_identity,
            last_mcp_epoch: 0,
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
                    let layout_dirty = command_invalidates_layout(&cmd);
                    // Snapshot BEFORE apply, only for the one tool the canvas
                    // radar-scan cares about — `design_agent_tools.rs`'s
                    // `should_register_batch_reveals` gates the in-app
                    // design-agent loop's reveal registration the exact same
                    // way, for the exact same reason (every other write tool
                    // is a targeted property tweak, not a "generation").
                    let ids_before = (tool_name == "batch_design")
                        .then(|| crate::design_agent_tools::collect_active_node_ids(state));
                    let applied = state.apply(cmd);
                    if applied {
                        if let Some(ids_before) = ids_before {
                            self.register_mcp_generation(&ids_before, state);
                        }
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

    pub fn stop(&mut self) {
        let _ = self.stop_tx.send(());
    }

    /// Tag a JUST-APPLIED `batch_design` write's genuinely new content
    /// with canvas-generation indicators, so the radar-scan (root frame
    /// glow — `op_editor_ui`'s `canvas_generation_scan` gates purely on
    /// `AgentIndicators.frames` being non-empty) and the per-node reveal
    /// entrance animation activate for MCP-driven generation the same
    /// way they already do for the in-app design-agent loop. A no-op
    /// when nothing new landed (a `batch_design` Direct-op call that
    /// only tweaked an existing node's properties, say).
    fn register_mcp_generation(&mut self, ids_before: &HashSet<String>, state: &EditorState) {
        let now_ms = crate::design_agent_tools::reveal_now_millis();
        let mut saw_new_content = false;
        for node in state.active_children() {
            if !ids_before.contains(node.id_str()) {
                saw_new_content = true;
                break;
            }
        }
        if !saw_new_content {
            // Reveals can still land nested under an EXISTING root (an
            // "add a section to this screen" follow-up) even when no
            // top-level id is new — `register_new_node_reveals` below
            // walks the whole tree, so check that shape too before
            // giving up entirely.
            saw_new_content = state
                .active_children()
                .iter()
                .any(|node| subtree_has_new_id(node, ids_before));
        }
        if !saw_new_content {
            return;
        }
        let epoch = self.resolve_mcp_epoch(now_ms);
        let (name, color) = self
            .client_identity
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
            .unwrap_or_else(|| {
                (
                    MCP_CLIENT_FALLBACK_NAME.to_string(),
                    MCP_CLIENT_COLOR.to_string(),
                )
            });
        // Only a genuinely NEW top-level Frame is a fresh generation
        // root — mirrors `design_loop_indicator.rs::register_new_frames`
        // (Frame-only) and `run_screen_groups.rs::insert_screen_group_roots`
        // (new-vs-existing top-level diff), the two established places
        // this codebase already tags a root frame.
        for node in state.active_children() {
            if matches!(node, PenNode::Frame(_)) && !ids_before.contains(node.id_str()) {
                op_editor_core::agent_indicators::add_frame(epoch, node.id_str(), &color, &name);
            }
        }
        crate::design_agent_tools::register_new_node_reveals(
            ids_before,
            state,
            Some(epoch),
            now_ms,
        );
        self.last_mcp_epoch = epoch;
        // Each `batch_design` call is its own self-contained turn — no
        // idle-timeout bookkeeping needed. `finish_if_epoch` is the
        // established graceful-drain: `run_active` drops immediately but
        // the already-queued reveals keep animating at their own pace
        // (`agent_indicators.rs`'s paint-side maintenance retires the
        // epoch once the queue empties), so this is safe to call even
        // though the call that just landed is still visually settling.
        op_editor_core::agent_indicators::finish_if_epoch(epoch);
    }

    /// Reuse the last MCP epoch while its reveal queue is still
    /// draining (`latest_reveal_end_ms` is the same "is anything still
    /// animating" check the orchestrator's own tree-restructure gate
    /// polls), so a burst of back-to-back `batch_design` calls (a
    /// multi-screen design, one call per screen) reads as one
    /// continuous generation instead of each call clipping the
    /// previous one's tail animation via `begin()`'s "bump epoch +
    /// clear every map" semantics. Starts fresh once the queue empties.
    fn resolve_mcp_epoch(&self, now_ms: u64) -> u64 {
        if self.last_mcp_epoch != 0 {
            if let Some(end_ms) =
                op_editor_core::agent_indicators::latest_reveal_end_ms(self.last_mcp_epoch)
            {
                if now_ms < end_ms {
                    return self.last_mcp_epoch;
                }
            }
        }
        op_editor_core::agent_indicators::begin()
    }
}

/// Whether `node` or any descendant carries an id not in `ids_before` —
/// the "did ANY new content land, even nested under an existing root"
/// check `register_mcp_generation` needs before deciding whether a
/// `batch_design` call is worth animating at all.
fn subtree_has_new_id(node: &PenNode, ids_before: &HashSet<String>) -> bool {
    if !ids_before.contains(node.id_str()) {
        return true;
    }
    if let Some(children) = node.children() {
        return children
            .iter()
            .any(|child| subtree_has_new_id(child, ids_before));
    }
    false
}

impl Drop for McpLiveServer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn command_invalidates_layout(cmd: &EditorCommand) -> bool {
    match cmd {
        EditorCommand::ClearSelection
        | EditorCommand::SetSelection { .. }
        | EditorCommand::SetSelectionSet { .. }
        | EditorCommand::ToggleNodeSelection { .. }
        | EditorCommand::SetViewport { .. }
        | EditorCommand::SetActiveTool { .. }
        | EditorCommand::CopySelected => false,
        EditorCommand::SetNodeFlag { flag, .. } => {
            !matches!(flag, op_editor_core::NodeFlag::Collapsed)
        }
        _ => true,
    }
}

mod connection;
mod doc_sync;
mod ui_requests;

use connection::*;
use doc_sync::*;
use ui_requests::*;

#[cfg(test)]
#[path = "mcp_live_tests.rs"]
mod tests;
