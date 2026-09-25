// Browser boundary: `window.confirm` and the rAF-driven phase pump need a
// live page; the decisions they drive are host-unit-tested in
// `widget_host/studio_*_tests.rs`.
//! Studio (Home + generation workspace) glue for the browser host.
//!
//! The widget host owns every Studio decision; this module supplies the
//! things only the page can:
//!
//! * the entry-surface choice at mount ([`apply_entry_surface`]), which asks
//!   the daemon whether a file is open first ([`probe_bound_file`]);
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

use op_editor_core::{EditorState, EmbedHost};

use crate::repaint_ctx::RepaintContext;

/// Applies a late `fileBound` answer to the mounted shell.
type FileBoundHook = Box<dyn Fn(bool)>;

thread_local! {
    /// One phase pump per page; the rAF closure owns itself and this is only
    /// a duplicate-start guard.
    static PUMP_RUNNING: Cell<bool> = const { Cell::new(false) };
    /// The daemon's `fileBound` answer, once it arrived.
    static FILE_BOUND: Cell<Option<bool>> = const { Cell::new(None) };
    /// The daemon's `siteImport` answer: it serves `/api/ai/site-import`.
    static SITE_IMPORT: Cell<bool> = const { Cell::new(false) };
    /// The daemon's `variants` answer: it runs side-by-side directions.
    static VARIANTS: Cell<bool> = const { Cell::new(false) };
    /// Applies a `fileBound` answer that lands after the first paint.
    static FILE_BOUND_HOOK: RefCell<Option<FileBoundHook>> = const { RefCell::new(None) };
}

/// Ask the daemon whether a file backs its document (`--file`, or a file
/// it opened). Started before the CanvasKit download so the answer is
/// normally in hand by the first paint; a later answer goes through
/// [`adopt_bound_file_answer`]'s hook. The VS Code embed never shows Home,
/// so it does not ask.
pub(crate) fn probe_bound_file(embed: EmbedHost) {
    if embed == EmbedHost::VsCode {
        return;
    }
    let url = crate::daemon_base::daemon_url("/api/mcp/server");
    crate::live_sync::get(
        &url,
        Rc::new(|body: String| {
            SITE_IMPORT.with(|slot| slot.set(parse_site_import(&body)));
            VARIANTS.with(|slot| slot.set(parse_variants(&body)));
            let Some(bound) = parse_file_bound(&body) else {
                return;
            };
            FILE_BOUND.with(|slot| slot.set(Some(bound)));
            FILE_BOUND_HOOK.with(|hook| {
                if let Some(apply) = hook.borrow_mut().take() {
                    apply(bound);
                }
            });
        }),
    );
}

/// `fileBound` out of `GET /api/mcp/server`. `None` for an older daemon
/// that does not report it (the preference then decides alone).
fn parse_file_bound(body: &str) -> Option<bool> {
    let parsed: serde_json::Value = serde_json::from_str(body).ok()?;
    parsed.get("fileBound")?.as_bool()
}

/// `siteImport` out of `GET /api/mcp/server`. An older daemon (no field)
/// has no import route, so Home must not offer the import.
fn parse_site_import(body: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|parsed| parsed.get("siteImport")?.as_bool())
        .unwrap_or(false)
}

/// `variants` out of `GET /api/mcp/server`. An older daemon (no field)
/// runs one design per turn, so Home must not offer the directions toggle.
fn parse_variants(body: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|parsed| parsed.get("variants")?.as_bool())
        .unwrap_or(false)
}

/// Show Studio Home on first paint when the persisted preference asks for
/// it. Embedded hosts and a daemon holding an opened file open straight
/// onto the canvas (desktop `should_show_home`). An answer still in flight
/// counts as "no file" here; [`adopt_bound_file_answer`] corrects it.
pub(crate) fn apply_entry_surface(state: &mut EditorState) {
    let file_bound = FILE_BOUND.with(Cell::get).unwrap_or(false);
    let show = crate::widget_host::entry_shows_home(
        state.editor_ui.embed,
        state.editor_ui.entry_surface,
        file_bound,
    );
    state.editor_ui.home.visible = show;
    // Side-by-side directions run on the daemon; only one that says it
    // runs them gets the toggle.
    state.editor_ui.home.variants_unavailable = !VARIANTS.with(Cell::get);
    state.editor_ui.home.site_import.available = SITE_IMPORT.with(Cell::get);
}

/// The mounted shell takes over a `fileBound` answer that had not arrived
/// by the first paint: when it says a file is open, Home steps aside for
/// the canvas (unless the user already started typing a brief on it).
pub(crate) fn adopt_bound_file_answer<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    if FILE_BOUND.with(Cell::get).is_some() {
        return;
    }
    let inner = inner.clone();
    FILE_BOUND_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |bound| {
            let Ok(mut b) = inner.try_borrow_mut() else {
                return;
            };
            // The same late answer says whether the daemon can import sites.
            b.host_mut()
                .editor_state_mut()
                .editor_ui
                .home
                .site_import
                .available = SITE_IMPORT.with(Cell::get);
            b.host_mut()
                .editor_state_mut()
                .editor_ui
                .home
                .variants_unavailable = !VARIANTS.with(Cell::get);
            if !bound {
                return;
            }
            if b.host_mut().open_bound_file_on_canvas() {
                b.host_mut().mark_editor_state_dirty();
                crate::repaint_coalescer::request();
            }
        }));
    });
}

/// Ask before a Home send discards unsaved work. Runs from the DOM listeners
/// after their press / key borrow is released (`window.confirm` blocks, so
/// no borrow may be held across it).
pub(crate) fn drain_home_replace_confirm<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    // A swap that needed no confirm already happened during the press / key.
    drain_daemon_file_unbind(inner);
    // So did a website import that needed none.
    crate::studio_web_site_import::drain_home_site_import(inner);
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
    crate::studio_web_site_import::drain_home_site_import(inner);
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

#[cfg(test)]
mod tests {
    use super::parse_file_bound;

    #[test]
    fn file_bound_is_read_from_the_server_probe() {
        assert_eq!(
            parse_file_bound(r#"{"running":true,"serveMode":"local","fileBound":true}"#),
            Some(true)
        );
        assert_eq!(parse_file_bound(r#"{"fileBound":false}"#), Some(false));
        // An older daemon says nothing; the preference decides alone.
        assert_eq!(parse_file_bound(r#"{"running":true}"#), None);
        assert_eq!(parse_file_bound("not json"), None);
    }

    #[test]
    fn the_directions_toggle_follows_the_daemon_variants_capability() {
        use super::{apply_entry_surface, parse_variants, VARIANTS};
        assert!(parse_variants(r#"{"siteImport":true,"variants":true}"#));
        assert!(!parse_variants(r#"{"variants":false}"#));
        // An older daemon says nothing: no toggle.
        assert!(!parse_variants(r#"{"running":true}"#));

        let mut state = op_editor_core::EditorState::new();
        VARIANTS.with(|slot| slot.set(false));
        apply_entry_surface(&mut state);
        assert!(state.editor_ui.home.variants_unavailable);
        VARIANTS.with(|slot| slot.set(true));
        apply_entry_surface(&mut state);
        assert!(!state.editor_ui.home.variants_unavailable);
    }
}
