//! OpenPencil desktop runner — winit + skia-safe + WidgetHostNative.
//! Owns the event loop, GL surface, DPI, animation timer + cursor input.

#![cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
// Detach from the console subsystem in release builds so launching from
// Explorer / the Start menu doesn't park a console window behind the GUI.
// Debug builds keep the console — tracing writes to stderr (init_tracing).
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod a11y;
mod acp_agent_probe_host;
mod agent_connect_store;
mod app_handler;
mod app_poll;
mod app_state;
mod asset_fetch_error;
mod bundled_fonts;
mod chat_acp;
mod chat_attachment;
mod chat_session;
mod clipboard;
mod codegen_export;
mod codegen_input;
mod codegen_session;
mod collab_avatar_host;
mod collab_jwks;
mod collab_runtime;
mod commit_diff_host;
mod commit_diff_semantic;
mod cursor_icon;
mod design_loop_indicator;
mod design_md_error;
mod design_md_host;
mod design_session;
mod drag_cursor;
mod figma_import_session;
mod font_import_host;
mod fonts;
mod fonts_error;
mod frame;
mod git_host;
mod git_jobs;
mod git_overflow_host;
mod git_session;
mod git_ssh_host;
mod heap_pressure;
mod html_import_error;
mod html_import_session;
mod iconify_host;
mod image_decode_host;
mod image_downscale;
mod image_drop_host;
mod image_generate_host;
mod image_panel_host;
mod image_search_session;
mod ime_window;
mod keyboard_input;
mod kit_io;
mod kit_persistence;
mod kit_persistence_error;
mod legacy_op_upgrade;
mod legacy_op_upgrade_error;
mod macos_app;
mod mcp_config_error;
mod mcp_config_io;
mod mcp_integrations;
mod mcp_port_file;
mod mcp_runtime;
mod mcp_serve;
mod menu;
mod menu_action;
mod persistence;
mod persistence_error;
mod persistence_export_batch;
mod persistence_image;
mod provider_probe_host;
mod remote_image_host;
mod render_cli;
mod render_cli_error;
mod save_session;
mod settings_io;
mod single_instance;
mod sub_agent_session;
mod sub_agent_spawn_error;
mod tcc_selftest;
mod test_config_root;
mod theme_preset_host;
mod ui_prefs;
mod update_check;
mod window_resize;
mod window_state;

use op_host_native::{NativeBackend, SharedSkiaContext, SharedSkiaError, WidgetHostNative};
use std::path::PathBuf;
use std::time::Instant;
use winit::event_loop::{ControlFlow, EventLoop, EventLoopProxy};
use winit::window::Window;

const INITIAL_VIEWPORT_W: f32 = 1440.0;
const INITIAL_VIEWPORT_H: f32 = 900.0;

type HtmlPasteResult = (Vec<jian_ops_schema::node::PenNode>, Vec<String>);
type PendingHtmlPaste = (u64, std::sync::mpsc::Receiver<HtmlPasteResult>);

#[derive(Clone, Copy, Debug)]
enum DesktopEvent {
    McpWake,
    /// A bounded collaboration network worker queued typed input for the
    /// GUI-owned session/editor actor.
    CollabWake,
    /// A background image decode completed and can be installed.
    ImageDecodeReady,
    /// A second launch forwarded a document to this instance (see
    /// `single_instance`). Wakes the loop to drain the forward queue + raise
    /// the window.
    ForwardedFileReady,
    /// The OS accessibility adapter reported activation (a screen reader
    /// attached). `DesktopA11y`'s cached tree may be stale or empty at this
    /// exact instant (the activation callback runs off the render loop, see
    /// `a11y.rs`), so this wakes the loop to force a repaint — the next
    /// painted frame republishes a current full tree via the normal
    /// `RedrawRequested` a11y push.
    A11yActivated,
    /// A background document save finished. The UI thread drains the
    /// completion, applies its generation-scoped revision ack, and surfaces
    /// any native error dialog.
    SaveReady,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PaintedPageIdentity {
    document_epoch: u64,
    page_id: String,
    /// Canonical page ids should be unique. Keep the index only as a fallback
    /// for malformed legacy documents with duplicate ids.
    duplicate_index: Option<usize>,
}

struct DesktopApp {
    window: Option<Window>,
    /// OS accessibility bridge (#67) — publishes the assembled
    /// `accesskit::TreeUpdate` to VoiceOver / Narrator / Orca and queues
    /// incoming action requests. `None` until the window is created.
    a11y: Option<a11y::DesktopA11y>,
    ctx: Option<SharedSkiaContext>,
    backend: Option<NativeBackend>,
    host: WidgetHostNative,
    /// Cached LOGICAL viewport size (refreshed on Resumed + Resized).
    viewport_width: f32,
    viewport_height: f32,
    /// Fresh empty documents are first fit to the default attrs in
    /// `new()`, then once more after winit reports the real window size.
    pending_initial_blank_frame_fit: bool,
    /// Last cursor position (logical, top-left origin).
    cursor_x: f32,
    cursor_y: f32,
    /// Cached scale factor (refreshed on Resumed + ScaleFactorChanged).
    dpi: f32,
    /// Last IME capability + caret area published to the native window.
    /// Keeps Windows from rebuilding its input context on every frame.
    ime_window_sync: ime_window::ImeWindowSync,
    /// Cmd / Ctrl held — promotes scroll to zoom + gates editor shortcuts.
    zoom_modifier: bool,
    alt_modifier: bool,
    /// Shift held — arrow-key nudge 1→10 px.
    shift_modifier: bool,
    /// Cursor moves coalesced between paints; drained on RedrawRequested
    /// and right before apply_press/release so drag-end frames aren't lost.
    pending_cursor_move: Option<(f32, f32)>,
    /// True while a raster image file is being dragged over the window.
    /// Drives the per-frame drop-target probe — the platform gives no
    /// cursor stream during a drag, so the position has to be polled.
    hovered_image_drop: bool,
    /// Last polled drag position (logical, top-left origin). Used as the drop
    /// point when the release itself cannot be re-probed.
    drop_cursor: Option<(f32, f32)>,
    /// True iff a `request_redraw` is already in flight.
    redraw_pending: bool,
    /// True when the pending redraw needs a paint even if cursor coalescing drained to no-op.
    redraw_dirty: bool,
    /// Logical page whose first frame has completed. The document epoch
    /// distinguishes whole-document replacements, while the page id keeps
    /// page deletion/reorder correct even when an index is reused.
    last_painted_page: Option<PaintedPageIdentity>,
    /// Monotonic clock anchor — `Instant.elapsed().as_millis()`
    /// from this is fed into `WidgetHostNative::set_now_ms` so
    /// `jian_core::anim::blink_visible` can drive the caret blink
    /// (and any future time-based UI animation).
    clock_start: Instant,
    /// Cached custom rotate cursor — built once at `Resumed` and
    /// reused for every CursorHint::Rotate to avoid re-decoding
    /// the bitmap every move. None until the event loop is ready.
    rotate_cursor: Option<winit::window::CustomCursor>,
    /// Path of the currently-open .pen/.op document; None when unsaved.
    current_path: Option<PathBuf>,
    /// At most one background document serialization / atomic write. Keeping
    /// saves serialized prevents an older snapshot from committing after a
    /// newer Save request to the same path.
    save_session: save_session::SaveSession,
    /// Generation-scoped Save-As requests started while collaboration was
    /// bound. A successful live acknowledgement detaches into the saved fork.
    collab_fork_saves: Vec<(u64, u64, u64, PathBuf)>,
    error: Option<SharedSkiaError>,
    /// Design-loop canvas indicator — tracks the active agent epoch,
    /// colour/name identity, and initial frame set. `None` when no
    /// design-loop turn is running; populated by `pump_indicator` in
    /// `RedrawRequested` whenever `chat.agents_running.0 > 0`.
    design_loop_indicator: Option<design_loop_indicator::DesignLoopIndicator>,
    /// Design-orchestrator canvas indicator — the same glow/badge/scan
    /// tracking as `design_loop_indicator` above, but driven by
    /// `current_design.is_some()` instead of `chat.agents_running` so the
    /// CLI-orchestrator and builtin-provider design turns (which never set
    /// `agents_running`) also animate their generated frames. `None` when
    /// no design-orchestrator turn is running; populated by
    /// `design_loop_indicator::pump_design_session_indicator` in
    /// `RedrawRequested`, right after `design_session::pump_progress`.
    design_session_indicator: Option<design_loop_indicator::DesignLoopIndicator>,
    /// Sub-agent design loops launched by `spawn_agents` (Task 3.1).
    /// Empty unless the top-level design loop called `spawn_agents`.
    /// Pumped SEQUENTIALLY — `active_sub_agent` indexes the one running
    /// — after the parent `chat_session::pump` each frame.
    sub_agents: Vec<sub_agent_session::SubAgentSession>,
    /// Index of the active sub-agent in `sub_agents` (sequential pump).
    active_sub_agent: usize,
    /// In-flight AI chat turn, if any. `chat.begin_send` raises
    /// `chat.pending_send`; the event loop drains that into a
    /// `ChatSession` here and pumps deltas into the transcript.
    current_chat: Option<chat_session::ChatSession>,
    /// Index of the chat tab a `current_chat` / `current_design` run is bound
    /// to (multi-tab MT.3). Captured from `chat.active_index()` when a turn
    /// launches; the pumps target this tab via `ChatSessions::run_tab_mut`
    /// even after the user switches the active tab. `None` when no run is in
    /// flight. Cleared when the run finishes (pump retired both sessions), on
    /// New Chat / Stop, and when the bound tab is closed.
    chat_running_tab: Option<usize>,
    /// In-flight design-orchestrator turn, if any.
    /// `chat_session::launch_if_pending` classifies the user's message
    /// and routes design intent here, chat intent to `current_chat`.
    /// CLI standard-mode turns (GAP #33) park BOTH sessions while the
    /// async classifier resolves; the route not taken retires via its
    /// pump once the worker drops its channels.
    current_design: Option<design_session::DesignSession>,
    /// In-flight code-generation turn, if any. The Code panel raises
    /// `codegen.pending_generate` / `pending_regenerate`;
    /// `codegen_session::launch_codegen_if_pending` drains that into a
    /// `CodegenSession` here and `pump` streams pipeline progress into
    /// `editor_state.codegen` each frame.
    current_codegen: Option<codegen_session::CodegenSession>,
    /// In-flight Design-MD auto-generation turn, if any. The floating
    /// design-system panel raises `design_md_panel.request`; the host
    /// resolves the selected model and lands the generated markdown
    /// back into `doc.design_md` when the worker completes.
    current_design_md: Option<design_md_host::DesignMdSession>,
    /// Completed generation results keyed by framework and kept host-side
    /// (including raw asset bytes) for Download — not carried in the
    /// wasm-clean `editor_state`.
    codegen_results: codegen_session::CodegenResults,
    #[cfg(test)]
    design_md_test_provider: Option<Box<dyn op_ai::chat_provider::ChatProvider>>,
    /// In-flight `.fig` import — worker thread that parses on a
    /// background thread so the editor UI keeps repainting. The pump
    /// in `RedrawRequested` swaps in the parsed document when the
    /// worker finishes.
    current_figma_import: Option<figma_import_session::FigmaImportSession>,
    /// In-flight `.html` import — same worker/pump lifecycle as the
    /// Figma session above.
    current_html_import: Option<html_import_session::HtmlImportSession>,
    /// In-flight Figma CLIPBOARD paste decode (Cmd+V) — worker sends
    /// the parsed nodes; the redraw path pumps + inserts them.
    pending_figma_paste: Option<(
        u64,
        std::sync::mpsc::Receiver<Vec<jian_ops_schema::node::PenNode>>,
    )>,
    /// In-flight clipboard HTML decode (non-Figma `text/html` paste):
    /// worker thread sends `(nodes, warnings)`.
    pending_html_paste: Option<PendingHtmlPaste>,
    /// Background AI-model discovery — probes the installed CLIs
    /// on a worker thread; its result is drained into
    /// `chat.available_models` on a later frame.
    model_probe: op_host_services::model_discovery::ModelProbe,
    /// Background auto-search jobs that replace generated empty image
    /// nodes with freely licensed remote images.
    image_search: image_search_session::ImageSearchSession,
    /// Property-panel image-section workers: Search / Generate
    /// popover requests + the local-asset existence check.
    image_panel: image_panel_host::ImagePanelJobs,
    /// Background fetches for remote `http(s)` image sources the
    /// canvas painter recorded as cache misses — fetched bytes land in
    /// the painter's shared byte cache so the next frame draws them.
    remote_images: remote_image_host::RemoteImageSession,
    /// Verified collaboration profile images. Encoded bytes remain in an
    /// ephemeral bounded UI cache and never enter the document.
    collab_avatars: collab_avatar_host::CollabAvatarHost,
    /// Two-thread local image raster decode pool.
    image_decodes: image_decode_host::ImageDecodeHost,
    /// Cross-thread wake handle used by live MCP connection threads.
    mcp_wake_proxy: Option<EventLoopProxy<DesktopEvent>>,
    /// Collaboration session/editor actor. Sockets remain on bounded network
    /// workers; this field is touched only by the winit GUI thread.
    collab_runtime: collab_runtime::DesktopCollabRuntime,
    /// Paths forwarded by second-launch processes (`single_instance`),
    /// drained on the UI thread by `drain_forwarded_files`.
    forwarded_files: single_instance::ForwardQueue,
    iconify_job: Option<iconify_host::IconifyJob>,
    /// The `component_browser_open` value last written to
    /// `uikits.json` — `drain_kit_io` rewrites the store when the live
    /// value drifts (TS persists `browserOpen` on every toggle).
    kit_browser_open_persisted: Option<bool>,
    /// In-flight connect-time provider probe (Settings → Agents →
    /// Connect) — spawned from the `pending_provider_connect`
    /// request seam, drained by `drain_provider_connect`.
    provider_connect_job: Option<provider_probe_host::ProviderConnectJob>,
    /// Startup reconnect replay queue (see `agent_connect_store`).
    provider_reconnect_queue: Vec<op_editor_core::AgentProvider>,
    /// Last persisted pencil-cursor style (see `ui_prefs`).
    last_saved_pencil_cursor: Option<op_editor_core::PencilCursorStyle>,
    /// Providers the store remembers as LAST KNOWN GOOD — seeded from
    /// `agents.json` at startup, extended when a probe succeeds, cleared
    /// only when the user explicitly disconnects. Deliberately not a
    /// mirror of the live `connected` flags: a failed probe must not
    /// evict a provider from next launch's reconnect replay.
    remembered_connections: [bool; 6],
    /// Previous frame's per-provider connect phase, so
    /// `persist_connection_changes` can tell an explicit Disconnect (card
    /// returns to Idle) from a probe failure (card shows Error).
    last_seen_provider_phase: [op_editor_core::agent_settings::ProviderConnectPhase; 6],
    /// In-flight ACP-agent connect probe (Settings → Agents → ACP
    /// Connect), drained by `drain_acp_agent_connect`.
    acp_agent_connect_job: Option<acp_agent_probe_host::AcpAgentConnectJob>,
    /// Document to open once the window is ready — set from argv by
    /// the file-association launch path (`openpencil-desktop X.op`).
    initial_file: Option<PathBuf>,
    /// Native menu bar — kept alive for the process lifetime;
    /// `None` until `resumed` builds it (and always `None` on Linux,
    /// where there is no native menu).
    app_menu: Option<menu::AppMenu>,
    /// Labels currently shown in the native File ▸ Open Recent submenu.
    /// Compared against the live recent list each loop iteration so the
    /// submenu is rebuilt only when it actually changed — and stays current
    /// regardless of whether the change came from the native menu, the
    /// in-canvas File menu, or a Finder open.
    recent_menu_labels: Vec<String>,
    /// Raw paths behind `recent_menu_labels` — the allocation-free
    /// change check `refresh_recent_menu` runs every loop iteration.
    recent_menu_paths: Vec<String>,
    /// Background auto-update probe — checks the GitHub releases API
    /// on a worker thread; its result is drained into
    /// `editor_ui.update_status` on a later frame.
    update_probe: update_check::UpdateProbe,
    /// Gates the update-available dialog to once per check.
    update_prompt_shown: bool,
    /// Last *windowed* (non-maximized) outer position, physical px.
    /// Persisted on exit so a restart restores window placement.
    win_pos: Option<(i32, i32)>,
    /// Last *windowed* inner size, physical px.
    win_size: Option<(u32, u32)>,
    /// Whether the window is currently maximized.
    win_maximized: bool,
    /// In-app Git — the repository bound to the open document.
    /// Rebound whenever the document path changes; read by the
    /// window title and the Git panel.
    git_session: git_session::GitSession,
    /// In-flight background `git pull`, if any — keeps the
    /// network-bound pull off the UI thread.
    git_pull_job: Option<git_jobs::GitPullJob>,
    /// In-flight background `git push`, if any.
    git_push_job: Option<git_jobs::GitPushJob>,
    /// Document generation + revision captured when a `git pull` was spawned.
    /// The post-pull reload compares against it to detect edits made
    /// *during* the async pull — which the spawn-time confirm did
    /// not cover — and re-confirm before discarding them.
    git_pull_doc_baseline: Option<(u64, u64, u64)>,
    /// In-flight background Git status query, if any.
    git_status_job: Option<git_jobs::GitStatusJob>,
    /// In-flight background Git diff (`git diff` / `git show`), if any.
    git_diff_job: Option<git_jobs::GitDiffJob>,
    /// In-flight background `git clone`, if any — set while the inline
    /// clone wizard's job runs; drained by `poll_git_clone_job`.
    git_clone_job: Option<git_jobs::GitCloneJob>,
    /// The document path that was current when the in-flight clone was
    /// started. The clone binds its repo onto the live document, so if
    /// the user has since switched / saved-as to a different document by
    /// the time the clone lands, the bind target changed — the result is
    /// discarded rather than bound onto the wrong document. `None` =
    /// started on an untitled document.
    git_clone_origin: Option<std::path::PathBuf>,
    /// When the Git panel was last re-snapshotted — drives the
    /// periodic refresh that keeps an open panel current against
    /// external repository changes.
    last_git_refresh: Instant,
    /// Live in-process MCP HTTP server, started from Settings -> MCP.
    mcp_server: Option<op_host_services::mcp_live::McpLiveServer>,
    /// When set (via the `--live-mcp[=port]` launch flag used by
    /// `op start`), the editor force-enables the live MCP server on
    /// this port during `resumed()`, regardless of the persisted
    /// `agent_settings.mcp_server.running` toggle. This is what lets
    /// `op start` bring up a live-rendering canvas the CLI can drive. When
    /// the live server binds, `reconcile_mcp_server_from_settings` updates
    /// this to the actually-bound port (it never mutates persisted settings
    /// for a forced launch).
    force_live_mcp_port: Option<u16>,
    /// Test-only override for the CLI-integration home dir. When set, MCP
    /// CLI detection + config writes target this dir (env-free), so tests
    /// don't have to mutate process-global `CODEX_HOME`/`HOME`. `None` in
    /// production (real home via `dirs::home_dir`).
    mcp_integrations_home: Option<PathBuf>,
}

/// Scan argv for a document to open on launch. This is the
/// file-association entry point: once the `.op` / `.pen` association
/// is registered (see `Cargo.toml`'s `[package.metadata.bundle]`),
/// the OS launches this binary with the document path in argv —
/// double-click on Windows / Linux, or `open file.op` from a shell
/// on any platform. The first existing `.op` / `.pen` / `.fig`
/// argument wins; flags (`--mcp`, …) never match the extension
/// filter. `.fig` routes through the Figma import worker once the
/// window is up (see `DesktopApp::apply_initial_file`).
fn initial_file_from_argv() -> Option<PathBuf> {
    std::env::args_os().skip(1).map(PathBuf::from).find(|p| {
        (op_host_services::doc_io::is_supported_document(p)
            || op_host_services::doc_io::is_supported_figma_import(p)
            || op_host_services::doc_io::is_supported_html_import(p))
            && p.is_file()
    })
}

/// Default port for the live MCP server when `--live-mcp` is passed
/// without an explicit port. Mirrors the TS `pen-mcp` default (3100)
/// and the `op` CLI default so the CLI finds the editor out of the box.
const DEFAULT_LIVE_MCP_PORT: u16 = 3100;

/// Parse `--live-mcp` / `--live-mcp=<port>` / `--live-mcp <port>` from
/// argv. Returns the requested live MCP port (the GUI then force-enables
/// `McpLiveServer` on it during `resumed()`), or `None` when the flag is
/// absent. `op start` uses this to bring up a live-rendering editor the
/// CLI can drive; double-clicking the app (no flag) keeps the persisted
/// settings-gated behavior.
fn live_mcp_port_from_argv() -> Option<u16> {
    parse_live_mcp_port(std::env::args().skip(1))
}

/// Pure `--live-mcp` parser (extracted for testing). Accepts
/// `--live-mcp`, `--live-mcp=<port>`, and `--live-mcp <port>`.
fn parse_live_mcp_port<I: Iterator<Item = String>>(args: I) -> Option<u16> {
    let mut args = args;
    while let Some(arg) = args.next() {
        if arg == "--live-mcp" {
            // An immediately-following numeric arg is the port; anything
            // else (a file path, another flag, or nothing) falls back to
            // the default — the non-port arg is left for argv scanners
            // that read the real process argv independently.
            if let Some(port) = args.next().and_then(|next| next.parse::<u16>().ok()) {
                return Some(port);
            }
            return Some(DEFAULT_LIVE_MCP_PORT);
        }
        if let Some(value) = arg.strip_prefix("--live-mcp=") {
            return Some(value.parse::<u16>().unwrap_or(DEFAULT_LIVE_MCP_PORT));
        }
    }
    None
}

/// Initialize the device-login runtime (proprietary bridge library) and
/// restore a persisted session, then set the runtime account gate. Stub
/// builds (no prebuilt library for this target) leave the gate closed
/// unless `OPENPENCIL_DEV_FAKE_LOGIN=1` re-opens it for UI work.
fn init_auth_runtime(host: &mut WidgetHostNative) {
    let dev_fake = std::env::var(op_auth_bridge::ENV_DEV_FAKE_LOGIN).as_deref() == Ok("1");
    let mut backend_ready = false;
    if op_auth_bridge::available() {
        if let Ok(dir) = op_config_store::openpencil_dir() {
            let config = op_auth_bridge::desktop_init_config(&dir, env!("CARGO_PKG_VERSION"));
            if op_auth_bridge::init(&config) {
                backend_ready = true;
                if op_auth_bridge::restore() {
                    if let op_auth_bridge::AuthStatus::SignedIn {
                        display_name,
                        primary_email,
                        ..
                    } = op_auth_bridge::poll(op_auth_bridge::SESSION_HANDLE)
                    {
                        host.editor_state_mut().editor_ui.account =
                            op_editor_core::AccountState::SignedIn {
                                handle: primary_email.unwrap_or_else(|| display_name.clone()),
                                display_name,
                            };
                    }
                }
            }
        }
    }
    host.editor_state_mut().editor_ui.account_ui_available = backend_ready || dev_fake;
}

/// Pop a native dialog offering to open the download page when a
/// newer release is found. Yes opens the GitHub releases page.
fn prompt_update_available(locale: op_editor_core::Locale, version: &str) {
    let body = op_i18n::translate(locale, "dialog.updateBody")
        .replace("{{version}}", version)
        .replace("{{current}}", env!("CARGO_PKG_VERSION"));
    let choice = rfd::MessageDialog::new()
        .set_title(op_i18n::translate(locale, "dialog.updateTitle"))
        .set_description(&body)
        .set_level(rfd::MessageLevel::Info)
        .set_buttons(rfd::MessageButtons::YesNo)
        .show();
    if matches!(choice, rfd::MessageDialogResult::Yes) {
        // Download the platform installer in the background and open
        // it when ready; failures fall back to the releases page
        // inside the worker.
        update_check::download_and_open_installer(version);
    }
}

/// Install a stderr tracing subscriber for debug-mode logging — orchestrator
/// LLM calls + parse failures (with the model's raw output). Writes to stderr
/// so it never pollutes the `--mcp` stdout JSON-RPC stream. The default `warn`
/// filter surfaces parse failures with no env var; `RUST_LOG=op_orchestrator=debug`
/// (and/or `op_host_desktop=debug` for per-subtask design progress) opens the
/// full firehose.
fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));
    let _ = fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}

fn main() {
    // FIRST, before any thread exists: graft the login-shell PATH and proxy
    // exports onto this process. A Dock/Finder launch inherits launchd's
    // minimal environment — CLI agents (codex is a node-shebang script) and
    // the Claude agent SDK's env baseline all need the user's real PATH,
    // networked CLI probes need the proxy vars, and the SDK's dangerous-env
    // blocklist forbids passing PATH per-request, so the process env is the
    // only correct carrier.
    op_host_services::chat_spawn::repair_gui_process_env();
    // Register the brand-logo catalog (omitted from the wasm bundle, embedded in
    // this binary) BEFORE any path that can render natively — the GUI app, the
    // headless `--render-shots` rasterizer below, MCP — so they resolve
    // simple-icons instead of the unknown-glyph fallback dot. Set-once /
    // idempotent.
    op_editor_ui::set_brand_catalog(op_host_services::web_static::ICONIFY_BRANDS_JSON);
    // Same rationale for bundled design fonts (Inter / Space Grotesk /
    // …): register before any native render or measure pass so designs
    // referencing them resolve the right glyphs + metrics without a
    // system font install.
    bundled_fonts::register();
    // Re-register user-imported fonts so an imported family survives a restart
    // (applies to the editor canvas AND headless render/export via the shared
    // resolver). Best-effort: a bad file or missing HOME must not block launch.
    match fonts::FontStore::user() {
        Ok(store) => store.rescan_and_register(),
        Err(err) => eprintln!("[fonts] skipping imported-font rescan: {err}"),
    }
    init_tracing();
    // `--mcp` / `--mcp-http` swap the GUI for an MCP server mode;
    // when one of those ran, exit instead of opening a window.
    if mcp_serve::run_cli_if_requested() {
        return;
    }
    // `--tcc-selftest <dir> [outfile]` probes protected-folder access
    // (macOS TCC) and exits — used to verify a signed bundle inherits
    // a granted app's Desktop/Documents access without opening the GUI.
    if tcc_selftest::run_cli_if_requested() {
        return;
    }
    // `--render-shots <file.op> <out_dir> [scale]` renders node-only
    // PNGs headless (model-design benchmark) and exits without a window.
    if render_cli::run_cli_if_requested() {
        return;
    }
    let initial_file = initial_file_from_argv();
    // Single-instance gate: when an editor is already running, a second launch
    // (e.g. a `.op` double-click on Windows / Linux) forwards its document to
    // the running window and exits instead of opening a second editor.
    let primary = match single_instance::acquire(initial_file.as_deref()) {
        single_instance::Acquire::Forwarded => return,
        single_instance::Acquire::Primary(primary) => primary,
    };
    let mut event_loop_builder = EventLoop::<DesktopEvent>::with_user_event();
    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
        event_loop_builder.with_activation_policy(ActivationPolicy::Regular);
    }
    let event_loop = match event_loop_builder.build() {
        Ok(el) => el,
        Err(err) => {
            eprintln!("openpencil-desktop: EventLoop::new failed: {err}");
            std::process::exit(1);
        }
    };
    event_loop.set_control_flow(ControlFlow::Wait);
    let mcp_wake_proxy = event_loop.create_proxy();
    // Give the non-bundled binary a proper Dock name + icon.
    macos_app::apply();
    let mut app = DesktopApp::new(initial_file);
    app.image_decodes.set_wake_proxy(mcp_wake_proxy.clone());
    app.collab_runtime.set_wake_proxy(mcp_wake_proxy.clone());
    app.mcp_wake_proxy = Some(mcp_wake_proxy);
    // Start accepting forwarded opens from second launches, sharing the queue
    // the UI thread drains in `drain_forwarded_files`.
    let forwarded_files = single_instance::ForwardQueue::default();
    primary.spawn_listener(event_loop.create_proxy(), forwarded_files.clone());
    app.forwarded_files = forwarded_files;
    app.force_live_mcp_port = live_mcp_port_from_argv();
    if let Err(err) = event_loop.run_app(&mut app) {
        eprintln!("openpencil-desktop: run_app exited with error: {err}");
        std::process::exit(1);
    }
    if let Some(err) = app.error {
        eprintln!("openpencil-desktop: fatal error during run: {err}");
        std::process::exit(1);
    }
}

// chat_intent moved to op_host_services::chat_intent (its headless tests
// moved alongside it as a `#[path]` sibling). Only the one host-coupled
// test stayed here — it drives the GUI design-session pumps, which need
// `WidgetHostNative` (absent from op-host-services's default-features-off
// op-host-native dependency).
//
// The sibling test stays enabled on macOS + Linux. It is ignored on Windows
// because the host-coupled `WidgetHostNative` path still aborts inside the
// Windows CI Skia/DirectWrite stack before Rust can report a normal assertion.
#[cfg(test)]
#[path = "chat_intent_host_tests.rs"]
mod chat_intent_host_tests;

#[cfg(test)]
mod main_mcp_tests;

#[cfg(test)]
mod main_tests;

#[cfg(test)]
mod page_paint_identity_tests;

#[cfg(test)]
mod keyboard_shortcut_tests;

// Serializes tests that touch the process-global `agent_indicators`
// registry against tests that assert an exact animation deadline. The
// registry is shared across every test in this binary running in
// parallel, so a design-turn test streaming reveals would otherwise
// race a caret-blink deadline assertion (the reveal deadline is smaller
// than the blink). Guard both sides on this lock and clear the registry
// inside the reader's critical section.
#[cfg(test)]
pub(crate) mod agent_indicator_test_lock {
    use std::sync::{LazyLock, Mutex};
    pub(crate) static LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
}
