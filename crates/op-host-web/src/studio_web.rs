// Browser boundary: `window.confirm` and the rAF-driven phase pump need a
// live page; the decisions they drive are host-unit-tested in
// `widget_host/studio_*_tests.rs`.
//! Studio (Home + generation workspace) glue for the browser host.
//!
//! The widget host owns every Studio decision; this module supplies the
//! things only the page can:
//!
//! * the entry-surface choice at mount ([`apply_entry_surface`]);
//! * unbinding the daemon's file when the document is swapped for one it
//!   does not hold ([`unbind_daemon_file`]);
//! * the discard confirm a Home send parks when it would replace a document
//!   with unsaved changes ([`drain_home_replace_confirm`]) — see
//!   `widget_host/studio_home_send.rs` for why the browser asks rather than
//!   keeping a rescue copy;
//! * the per-frame workspace phase pump desktop runs from its winit loop
//!   ([`ensure_workspace_pump`] / [`on_turn_launched`]).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use op_editor_core::{EditorState, EmbedHost, EntrySurface};

use crate::repaint_ctx::RepaintContext;

thread_local! {
    /// One phase pump per page; the rAF closure owns itself and this is only
    /// a duplicate-start guard.
    static PUMP_RUNNING: Cell<bool> = const { Cell::new(false) };
}

/// Show Studio Home on first paint when the persisted preference asks for
/// it. Embedded hosts open straight onto the document they were handed.
pub(crate) fn apply_entry_surface(state: &mut EditorState) {
    let show = state.editor_ui.embed != EmbedHost::VsCode
        && state.editor_ui.entry_surface == EntrySurface::Home;
    state.editor_ui.home.visible = show;
    // The daemon runs one design per turn; several directions side by
    // side are desktop-only for now.
    state.editor_ui.home.variants_unavailable = true;
}

/// Ask before a Home send discards unsaved work. Runs from the DOM listeners
/// after their press / key borrow is released (`window.confirm` blocks, so
/// no borrow may be held across it).
pub(crate) fn drain_home_replace_confirm<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    // A swap that needed no confirm already happened during the press / key.
    drain_daemon_file_unbind(inner);
    let (intent, message) = {
        let Ok(mut b) = inner.try_borrow_mut() else {
            return;
        };
        let Some(intent) = b.host_mut().take_home_replace_confirm() else {
            return;
        };
        let locale = b.host().editor_state().editor_ui.effective_locale();
        (
            intent,
            op_i18n::translate(locale, "home.replaceUnsavedConfirm").to_string(),
        )
    };
    let confirmed = web_sys::window()
        .and_then(|window| window.confirm_with_message(&message).ok())
        .unwrap_or(false);
    if !confirmed {
        return;
    }
    let Ok(mut b) = inner.try_borrow_mut() else {
        return;
    };
    b.host_mut().set_clocks(
        crate::listener::now_ms_perf(),
        crate::listener::now_unix_secs(),
    );
    if b.host_mut().confirm_home_replace(intent) {
        b.host_mut().mark_editor_state_dirty();
        crate::repaint_coalescer::request();
    }
    drop(b);
    drain_daemon_file_unbind(inner);
}

/// A Home swap replaced the document with a fresh page: tell the daemon to
/// stop treating its bound file as this document's Save target.
fn drain_daemon_file_unbind<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    let embed = {
        let Ok(mut b) = inner.try_borrow_mut() else {
            return;
        };
        if !b.host_mut().take_daemon_file_unbind() {
            return;
        }
        b.host().editor_state().editor_ui.embed
    };
    unbind_daemon_file(embed);
}

/// `POST /api/file/unbind`: the browser's document is no longer the file the
/// daemon was started with (or last opened), so File > Save must fall back
/// to a download instead of overwriting it. Called for Home swaps, File >
/// New and a document opened from the browser's own file picker.
///
/// Fire-and-forget: the route is idempotent and has no failure the user
/// could act on. The VS Code embed is skipped — its extension owns the
/// document's file and saves through its own bridge.
pub(crate) fn unbind_daemon_file(embed: EmbedHost) {
    if embed == EmbedHost::VsCode {
        return;
    }
    let url = crate::daemon_base::daemon_url("/api/file/unbind");
    if !crate::live_sync::post_json(&url, "{}", None) {
        web_sys::console::warn_1(&"[file] daemon unbind could not start".into());
    }
}

/// A chat turn just launched with `generation`: stamp it onto the Studio
/// workspace it belongs to and make sure the phase pump is running.
pub(crate) fn on_turn_launched<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    generation: u64,
) {
    if let Ok(mut b) = inner.try_borrow_mut() {
        b.host_mut().stamp_workspace_run(generation);
    }
    ensure_workspace_pump(inner);
}

/// Start the self-terminating phase pump while a workspace run is
/// Generating. Cheap and idempotent; called from every drain point that can
/// start a run.
pub(crate) fn ensure_workspace_pump<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    if PUMP_RUNNING.with(Cell::get) {
        return;
    }
    let generating = inner
        .try_borrow()
        .is_ok_and(|b| b.host().workspace_run_generating());
    if !generating {
        return;
    }
    PUMP_RUNNING.with(|running| running.set(true));
    let inner = inner.clone();
    crate::raf_pump::start(Rc::new(move || {
        let Ok(mut b) = inner.try_borrow_mut() else {
            // A DOM event owns the shell; retry on the next frame.
            return true;
        };
        let now = crate::listener::now_ms_perf();
        b.host_mut()
            .set_clocks(now, crate::listener::now_unix_secs());
        let (w, h) = b.viewport_size();
        let tick = b
            .host_mut()
            .drive_workspace_run(w, h, crate::web_chat::turn_in_flight(), now);
        if tick.changed {
            b.host_mut().mark_editor_state_dirty();
            crate::repaint_coalescer::request();
        }
        if !tick.keep_pumping {
            PUMP_RUNNING.with(|running| running.set(false));
        }
        tick.keep_pumping
    }));
}
