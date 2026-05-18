//! OpenPencil desktop runner — winit + skia-safe + WidgetHostNative.
//! Owns the event loop, GL surface, DPI, animation timer + cursor input.

#![cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]

mod app_handler;
mod chat_acp;
mod chat_attachment;
mod chat_claude;
mod chat_copilot;
mod chat_http_server;
mod chat_runtime;
mod chat_session;
mod chat_subprocess;
mod cursor_icon;
mod export;
mod export_pdf;
mod frame;
mod mcp_serve;
mod menu;
mod model_discovery;
mod persistence;
mod settings_io;
mod update_check;
mod window_state;

use op_host_native::{NativeBackend, SharedSkiaContext, SharedSkiaError, WidgetHostNative};
use std::path::PathBuf;
use std::time::Instant;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Fullscreen, Window};

const INITIAL_VIEWPORT_W: f32 = 1440.0;
const INITIAL_VIEWPORT_H: f32 = 900.0;

struct DesktopApp {
    window: Option<Window>,
    ctx: Option<SharedSkiaContext>,
    backend: Option<NativeBackend>,
    host: WidgetHostNative,
    /// Cached LOGICAL viewport size (refreshed on Resumed + Resized).
    viewport_width: f32,
    viewport_height: f32,
    /// Last cursor position (logical, top-left origin).
    cursor_x: f32,
    cursor_y: f32,
    /// Cached scale factor (refreshed on Resumed + ScaleFactorChanged).
    dpi: f32,
    /// Cmd / Ctrl held — promotes scroll to zoom + gates editor shortcuts.
    zoom_modifier: bool,
    alt_modifier: bool,
    /// Shift held — arrow-key nudge 1→10 px.
    shift_modifier: bool,
    /// Cursor moves coalesced between paints; drained on RedrawRequested
    /// and right before apply_press/release so drag-end frames aren't lost.
    pending_cursor_move: Option<(f32, f32)>,
    /// True iff a `request_redraw` is already in flight.
    redraw_pending: bool,
    /// True when the pending redraw needs a paint even if cursor coalescing drained to no-op.
    redraw_dirty: bool,
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
    error: Option<SharedSkiaError>,
    /// In-flight AI chat turn, if any. `chat.begin_send` raises
    /// `chat.pending_send`; the event loop drains that into a
    /// `ChatSession` here and pumps deltas into the transcript.
    current_chat: Option<chat_session::ChatSession>,
    /// Background AI-model discovery — probes the installed CLIs
    /// on a worker thread; its result is drained into
    /// `chat.available_models` on a later frame.
    model_probe: model_discovery::ModelProbe,
    /// Document to open once the window is ready — set from argv by
    /// the file-association launch path (`openpencil-desktop X.op`).
    initial_file: Option<PathBuf>,
    /// Native menu bar — kept alive for the process lifetime;
    /// `None` until `resumed` builds it (and always `None` on Linux,
    /// where there is no native menu).
    app_menu: Option<menu::AppMenu>,
    /// Background auto-update probe — checks the GitHub releases API
    /// on a worker thread; its result is drained into
    /// `editor_ui.update_status` on a later frame.
    update_probe: update_check::UpdateProbe,
    /// Whether the "update available" prompt has already been shown
    /// for the current probe — gates the dialog to once per check.
    update_prompt_shown: bool,
    /// Last *windowed* (non-maximized) outer position, physical px.
    /// Persisted on exit so a restart restores window placement.
    win_pos: Option<(i32, i32)>,
    /// Last *windowed* inner size, physical px.
    win_size: Option<(u32, u32)>,
    /// Whether the window is currently maximized.
    win_maximized: bool,
}

impl DesktopApp {
    fn new(initial_file: Option<PathBuf>) -> Self {
        let mut host = WidgetHostNative::new();
        // Best-effort prefs restore onto the host's `EditorState`.
        settings_io::load(host.editor_state_mut());
        host.mark_editor_state_dirty();
        Self {
            window: None,
            ctx: None,
            backend: None,
            host,
            viewport_width: INITIAL_VIEWPORT_W,
            viewport_height: INITIAL_VIEWPORT_H,
            cursor_x: 0.0,
            cursor_y: 0.0,
            dpi: 1.0,
            zoom_modifier: false,
            alt_modifier: false,
            shift_modifier: false,
            pending_cursor_move: None,
            redraw_pending: false,
            redraw_dirty: false,
            clock_start: Instant::now(),
            rotate_cursor: None,
            current_path: None,
            error: None,
            current_chat: None,
            model_probe: model_discovery::ModelProbe::spawn(),
            initial_file,
            app_menu: None,
            update_probe: update_check::UpdateProbe::spawn(),
            update_prompt_shown: false,
            win_pos: None,
            win_size: None,
            win_maximized: false,
        }
    }

    /// Drain the background auto-update probe into `update_status`.
    /// When the probe reports a newer release, offer to open the
    /// download page — once per check.
    fn poll_update_probe(&mut self) -> bool {
        let Some(status) = self.update_probe.poll() else {
            return false;
        };
        let available = matches!(
            status,
            op_editor_core::UpdateStatus::Available { .. }
        );
        self.host.editor_state_mut().editor_ui.update_status = status.clone();
        self.host.mark_editor_state_dirty();
        if available && !self.update_prompt_shown {
            self.update_prompt_shown = true;
            if let op_editor_core::UpdateStatus::Available { version } = &status {
                prompt_update_available(version);
            }
        }
        true
    }

    /// Dispatch a native-menu selection onto the matching host
    /// action — the same calls the keyboard shortcuts make.
    fn handle_menu_action(&mut self, action: menu::MenuAction, event_loop: &ActiveEventLoop) {
        use menu::MenuAction as A;
        let consumed = match action {
            A::New => {
                persistence::run_action(
                    op_editor_core::editor_ui_state::FileAction::New,
                    &mut self.host,
                    &mut self.current_path,
                    self.window.as_ref(),
                );
                true
            }
            A::Open => {
                persistence::handle_open(&mut self.host, &mut self.current_path, self.window.as_ref())
            }
            A::Save => {
                self.host.commit_variable_row_focus_if_any_pub();
                persistence::handle_save(&mut self.host, &mut self.current_path, self.window.as_ref())
            }
            A::SaveAs => {
                self.host.commit_variable_row_focus_if_any_pub();
                persistence::handle_save_as(
                    &mut self.host,
                    &mut self.current_path,
                    self.window.as_ref(),
                )
            }
            A::Export => {
                self.host.commit_variable_row_focus_if_any_pub();
                persistence::run_action(
                    op_editor_core::editor_ui_state::FileAction::ExportImage,
                    &mut self.host,
                    &mut self.current_path,
                    self.window.as_ref(),
                );
                true
            }
            A::Undo => self.host.apply_undo(),
            A::Redo => self.host.apply_redo(),
            A::Cut => self.host.apply_cut(),
            A::Copy => self.host.apply_copy(),
            A::Paste => self.host.apply_paste(),
            A::SelectAll => self.host.apply_select_all(),
            A::Duplicate => self.host.apply_duplicate(),
            A::Group => self.host.apply_group(),
            A::Ungroup => self.host.apply_ungroup(),
            A::ToggleFullscreen => {
                if let Some(window) = self.window.as_ref() {
                    let next = match window.fullscreen() {
                        Some(_) => None,
                        None => Some(Fullscreen::Borderless(None)),
                    };
                    window.set_fullscreen(next);
                }
                false
            }
            A::Quit => {
                event_loop.exit();
                false
            }
            A::CheckUpdates => {
                // Re-run the probe; the System tab reflects `Checking`
                // immediately and the result lands on a later frame.
                // Skip when a probe is already in flight so repeated
                // menu clicks can't stack untracked worker threads.
                if self.update_probe.is_pending() {
                    false
                } else {
                    self.host.editor_state_mut().editor_ui.update_status =
                        op_editor_core::UpdateStatus::Checking;
                    self.host.mark_editor_state_dirty();
                    self.update_probe = update_check::UpdateProbe::spawn();
                    self.update_prompt_shown = false;
                    true
                }
            }
            A::OpenGithub => {
                update_check::open_url("https://github.com/ZSeven-W/openpencil");
                false
            }
        };
        if consumed {
            self.request_redraw(true);
        }
    }

    fn request_redraw(&mut self, dirty: bool) -> bool {
        if dirty {
            self.redraw_dirty = true;
        }
        if self.redraw_pending {
            return false;
        }
        self.redraw_pending = true;
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
        true
    }

    fn drain_pending_cursor_move(&mut self) -> bool {
        if let Some((cx, cy)) = self.pending_cursor_move.take() {
            let hover_changed = self.host.update_layer_hover(cx, cy, self.viewport_height);
            let cursor_changed = self.host.apply_cursor_move(cx, cy);
            hover_changed || cursor_changed
        } else {
            false
        }
    }

    fn prepare_redraw(&mut self) -> bool {
        let tracked_request = self.redraw_pending;
        self.redraw_pending = false;
        let mut should_paint = !tracked_request || self.redraw_dirty;
        self.redraw_dirty = false;
        should_paint |= self.drain_pending_cursor_move();
        should_paint
    }
}


/// Scan argv for a document to open on launch. This is the
/// file-association entry point: once the `.op` / `.pen` association
/// is registered (see `Cargo.toml`'s `[package.metadata.bundle]`),
/// the OS launches this binary with the document path in argv —
/// double-click on Windows / Linux, or `open file.op` from a shell
/// on any platform. The first existing `.op` / `.pen` argument wins;
/// flags (`--mcp`, …) never match the extension filter.
fn initial_file_from_argv() -> Option<PathBuf> {
    std::env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .find(|p| persistence::is_supported_document(p) && p.is_file())
}

/// Pop a native dialog offering to open the download page when a
/// newer release is found. Yes opens the GitHub releases page.
fn prompt_update_available(version: &str) {
    let body = format!(
        "OpenPencil {version} is available.\n\nYou are running version {}.\n\nOpen the download page?",
        env!("CARGO_PKG_VERSION"),
    );
    let choice = rfd::MessageDialog::new()
        .set_title("Update available")
        .set_description(&body)
        .set_level(rfd::MessageLevel::Info)
        .set_buttons(rfd::MessageButtons::YesNo)
        .show();
    if matches!(choice, rfd::MessageDialogResult::Yes) {
        update_check::open_url(&update_check::releases_url());
    }
}

fn main() {
    // `--mcp` / `--mcp-http` swap the GUI for an MCP server mode;
    // when one of those ran, exit instead of opening a window.
    if mcp_serve::run_cli_if_requested() {
        return;
    }
    let initial_file = initial_file_from_argv();
    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(err) => {
            eprintln!("openpencil-desktop: EventLoop::new failed: {err}");
            std::process::exit(1);
        }
    };
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = DesktopApp::new(initial_file);
    if let Err(err) = event_loop.run_app(&mut app) {
        eprintln!("openpencil-desktop: run_app exited with error: {err}");
        std::process::exit(1);
    }
    if let Some(err) = app.error {
        eprintln!("openpencil-desktop: fatal error during run: {err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod main_tests;
