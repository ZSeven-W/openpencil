//! Live CanvasKit shell state (`CkInner`) and the daemon bootstrap sequence.
//!
//! Split out of `canvaskit.rs`: the widget host + backend pair driven by
//! `mount_ck`, its `RepaintContext` impl, the accessibility DOM event router,
//! and the bootstrap sync-reset / late-init recovery helpers.

use op_editor_ui::RenderBackend;
use wasm_bindgen::prelude::*;

use super::backend::CanvasKitBackend;
use super::convert::display_dpr;

/// Live shell state for the CanvasKit host: the widget host + its backend.
pub(super) struct CkInner {
    pub(super) backend: CanvasKitBackend,
    pub(super) host: crate::widget_host::WidgetHost,
    pub(super) settings_fingerprint: Option<crate::web_settings::Fingerprint>,
    pub(super) credential_fingerprint: crate::web_settings::CredentialFingerprint,
    pub(super) canvas: web_sys::HtmlCanvasElement,
    /// Hidden ARIA DOM mirror (#57) — kept in sync after every paint so a
    /// screen reader can read the opaque CanvasKit surface. `None` only if
    /// the DOM container couldn't be created (non-browser host).
    pub(super) a11y: Option<crate::a11y_dom::A11yDomMirror>,
    /// Hidden IME-capture input (#54) — focused while a text field is active
    /// so the browser IME can compose CJK into it; its `compositionend` is
    /// routed to `apply_ime`. `None` only if the DOM is unreachable.
    pub(super) ime: Option<crate::ime_input::ImeInput>,
    /// DOM-side video playback layer used only while CanvasKit preview is
    /// active. The painted canvas remains the poster source of truth.
    pub(super) video_overlay: crate::video_overlay::VideoOverlayLayer,
}

impl CkInner {
    pub(super) fn repaint(&mut self) {
        if crate::web_asset_fetch::reconcile_pending_locale(
            &mut self.host.editor_state_mut().editor_ui,
        ) {
            self.host.mark_editor_state_dirty();
        }
        self.backend.drain_pending_decodes(2);
        // Assets the last paint asked for but the bundle does not carry
        // (preview JPEGs, template documents, the icon catalog). Bounded per
        // call; the installs wake a later frame through `repaint_coalescer`.
        // Ask for the icon catalog the first time the picker is up. Idempotent
        // and single-flighted, so a per-frame check is the cheapest place to
        // notice — the `open` flag is set by shared flows with no single
        // host-side call site to hang this off.
        if self.host.editor_state().editor_ui.icon_picker.open {
            crate::iconify_web::ensure_core_catalog();
        }
        crate::web_asset_fetch::drain_pending();
        // Collaboration-peer avatars: drain what the roster paint enqueued and
        // fetch it through the daemon proxy (a wasm page cannot reach the CDN
        // directly). Bounded per frame; empty and cheap outside a session. The
        // self-account avatar is NOT handled here — `web_auth_sync` owns it.
        crate::collab_avatar_fetch::drain_pending();
        // A template clicked before its document arrived is instantiated here,
        // the frame after the fetch lands. No-op on every other frame and on
        // native, where the document was already in the binary.
        self.host.retry_pending_scene_template();
        crate::web_chat::reconcile_models(self.host.editor_state_mut());
        // Detect a credential edit and enqueue the daemon sync BEFORE mirroring
        // the sync status below: a corrective edit clears the stale error in
        // the sync state machine here, so the mirror reflects it in the SAME
        // frame instead of leaving the banner up until the next repaint.
        if crate::web_settings::save_credentials_if_changed(
            self.host.editor_state(),
            &mut self.credential_fingerprint,
        )
        .is_some()
        {
            if let Some(json) =
                crate::web_settings::server_credentials_json(self.host.editor_state())
            {
                crate::web_credential_sync::credential_changed(json);
            }
        }
        // Mirror the (now up-to-date) credential-sync status into the settings
        // modal so a rejected server save is visible instead of console-only.
        let sync_error = crate::web_credential_sync::current_sync_error();
        {
            let settings = &mut self.host.editor_state_mut().editor_ui.agent_settings;
            if settings.web_credential_sync_error != sync_error {
                settings.web_credential_sync_error = sync_error;
            }
        }
        let (w, h) = self.backend.logical_size();
        self.backend.begin_frame();
        self.host.paint_dyn(&mut self.backend, w, h);
        self.backend.end_frame();
        let placements = self.host.preview_video_overlay_placements(w, h);
        self.video_overlay.sync(&self.canvas, w, h, &placements);
        self.sync_a11y();
        // #54: focus the hidden IME input only while a text field owns the
        // keyboard, so CJK composition works when editing and no soft keyboard
        // appears otherwise. Cheap — toggles only on a focus transition.
        let ime_focus = self.host.text_input_focus_active();
        let ime_anchor = self.host.ime_anchor_rect().map(|anchor| {
            let bounds = self.canvas.get_bounding_client_rect();
            let scale_x = if w == 0.0 {
                1.0
            } else {
                bounds.width() / f64::from(w)
            };
            let scale_y = if h == 0.0 {
                1.0
            } else {
                bounds.height() / f64::from(h)
            };
            (
                bounds.left() + f64::from(anchor.origin.x) * scale_x,
                bounds.top() + f64::from(anchor.origin.y + anchor.size.y) * scale_y,
            )
        });
        if let Some(ime) = self.ime.as_mut() {
            if let Some((left, top)) = ime_anchor {
                ime.sync_position(left, top);
            }
            ime.sync_focus(ime_focus);
        }
        // Device theme first, and OUTSIDE both gates below. The settings
        // fingerprint is `None` whenever the partition blob is unwritable, and
        // a device preference must not be collateral damage of an account blob
        // this tab refuses to touch. Cheap: a comparison, not a write.
        let _ = crate::web_settings::theme::save_if_changed(self.host.editor_state());
        if !crate::web_settings::credential_migration_pending(&self.credential_fingerprint) {
            if let Some(settings_fingerprint) = self.settings_fingerprint.as_mut() {
                let _ = crate::web_settings::save_if_changed(
                    self.host.editor_state(),
                    settings_fingerprint,
                );
            }
        }
        if self.host.layout_transition_active() {
            crate::repaint_coalescer::request();
        }
        if op_editor_ui::image_runtime::has_pending_decodes() {
            crate::repaint_coalescer::request();
        }
    }

    /// Rebuild the hidden ARIA DOM mirror from a freshly assembled tree.
    /// Called after each paint so the mirror tracks the painted frame
    /// (cheap: ~8 always-present region nodes). A diff-or-rebuild refinement
    /// can replace the full rebuild later; v1 rebuilds.
    fn sync_a11y(&mut self) {
        if let Some(mirror) = self.a11y.as_mut() {
            let (w, h) = self.backend.logical_size();
            let tree = self.host.accessibility_tree_update(w, h);
            mirror.update(&tree);
        }
    }

    pub(super) fn resize_to_window(&mut self, window: &web_sys::Window) -> Result<bool, JsValue> {
        let css_w = window
            .inner_width()?
            .as_f64()
            .unwrap_or_else(|| self.canvas.client_width().max(1) as f64)
            .round()
            .max(1.0) as u32;
        let css_h = window
            .inner_height()?
            .as_f64()
            .unwrap_or_else(|| self.canvas.client_height().max(1) as f64)
            .round()
            .max(1.0) as u32;
        let dpr = display_dpr(window.device_pixel_ratio() as f32);
        let dev_w = ((css_w as f32) * dpr).round().max(1.0) as u32;
        let dev_h = ((css_h as f32) * dpr).round().max(1.0) as u32;

        let style = format!("width: {css_w}px; height: {css_h}px;");
        let mut changed = self.canvas.get_attribute("style").as_deref() != Some(style.as_str());
        if changed {
            self.canvas.set_attribute("style", &style)?;
        }
        if self.canvas.width() != dev_w {
            self.canvas.set_width(dev_w);
            changed = true;
        }
        if self.canvas.height() != dev_h {
            self.canvas.set_height(dev_h);
            changed = true;
        }
        let (logical_w, logical_h) = self.backend.logical_size();
        let backend_changed = logical_w.round() as u32 != css_w
            || logical_h.round() as u32 != css_h
            || (self.backend.dpr - dpr).abs() > f32::EPSILON;
        if changed || backend_changed {
            self.backend.resize_for_display(css_w, css_h, dpr);
            return Ok(true);
        }
        Ok(false)
    }

    pub(super) fn event_offset_to_logical(&self, offset_x: f32, offset_y: f32) -> (f32, f32) {
        let (logical_w, logical_h) = self.backend.logical_size();
        crate::event::pointer::map_offset_to_logical(
            offset_x,
            offset_y,
            self.canvas.client_width().max(1) as f32,
            self.canvas.client_height().max(1) as f32,
            logical_w,
            logical_h,
        )
    }
}

impl crate::repaint_ctx::RepaintContext for CkInner {
    fn host(&self) -> &crate::widget_host::WidgetHost {
        &self.host
    }
    fn host_mut(&mut self) -> &mut crate::widget_host::WidgetHost {
        &mut self.host
    }

    fn reset_persistence_baselines(&mut self, load: &crate::web_settings::CredentialLoad) {
        // Through the SAME constructors mount uses. A bare recompute set
        // `settings_fingerprint` to `None` — which the save gate reads as
        // "never save" — and dropped the credential write-pending retry and
        // the fail-closed `write_disabled` an unsupported snapshot sets.
        self.settings_fingerprint = load.initial_settings_fingerprint(self.host.editor_state());
        self.credential_fingerprint = load.initial_fingerprint(self.host.editor_state());
    }
    fn viewport_size(&self) -> (f32, f32) {
        self.backend.logical_size()
    }
    fn register_system_font(&mut self, family: &str, bytes: &[u8]) -> bool {
        self.backend.ck.register_system_font(family, bytes)
    }
    fn register_imported_font(&mut self, family: &str, bytes: &[u8]) -> bool {
        self.backend.register_imported_font(family, bytes)
    }
    fn register_imported_font_from_bytes(&mut self, bytes: &[u8]) -> Option<String> {
        self.backend.register_imported_font_from_bytes(bytes)
    }
    fn register_bundled_font(&mut self, family: &str, bytes: &[u8]) -> bool {
        self.backend.register_bundled_font(family, bytes)
    }
    fn imported_family_list(&self) -> Vec<String> {
        self.backend.imported_family_list()
    }
    fn remove_imported_font(&mut self, family: &str) {
        self.backend.remove_imported_font(family);
    }
    fn repaint(&mut self) -> Result<(), JsValue> {
        // CanvasKit present is infallible (GPU flush, no pixel round-trip).
        CkInner::repaint(self);
        Ok(())
    }
}

/// Resolve an accessibility DOM event's target to its `accesskit::NodeId`
/// and route it into the host (#57). `is_focus` distinguishes `focusin`
/// from `click`. Repaints on a state change so the canvas + the mirror
/// re-sync with the screen-reader-driven focus / activation.
pub(super) fn dispatch_a11y_dom_event(
    inner: &std::rc::Rc<std::cell::RefCell<CkInner>>,
    target: Option<web_sys::EventTarget>,
    is_focus: bool,
) {
    use wasm_bindgen::JsCast;
    let Some(element) = target.and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else {
        return;
    };
    let Some(node_id) = crate::a11y_dom::A11yDomMirror::node_id_for_target(&element) else {
        return;
    };
    let Ok(mut b) = inner.try_borrow_mut() else {
        return;
    };
    b.host.set_clocks(
        crate::listener::now_ms_perf(),
        crate::listener::now_unix_secs(),
    );
    if b.host.apply_a11y_action(node_id.0, is_focus) {
        crate::repaint_coalescer::request();
    }
}

/// Retries for the bootstrap sync-reset after a transport/server error before
/// giving up and proceeding anyway (exactly one — see [`start_bootstrap_reset`]).
pub(super) const BOOTSTRAP_RESET_RETRIES: u8 = 1;

/// POST the bootstrap `POST /api/mcp/sync-reset`, invoking `complete` EXACTLY
/// once the daemon has been reset — a fresh reset OR a peer view that already
/// reset it (`"skipped":true`, which the daemon still answers `"ok":true`) both
/// count as completion. A transport/server error retries once, then proceeds
/// anyway with a console warning.
///
/// The request goes through [`crate::live_sync::post_json_with_status`], which
/// arms an XHR timeout: a STALLED connection therefore fires `onloadend` with
/// status 0 (empty body) instead of hanging silently, so it lands on the same
/// retry-then-complete path as any other transport error. Without the timeout a
/// hung reset would fire neither success nor error and wedge `ready` forever.
///
/// `complete` is ALWAYS eventually called: a webview that never emits `ready`
/// (wedged forever) is worse than one running on a best-effort-reset daemon
/// (degraded), so the retry is bounded and completion is unconditional past it.
pub(super) fn start_bootstrap_reset(
    base: String,
    complete: std::rc::Rc<dyn Fn()>,
    retries_left: u8,
) {
    let url = crate::daemon_base::daemon_url("/api/mcp/sync-reset");
    let on_reset: std::rc::Rc<dyn Fn(u16, String)> = {
        let complete = complete.clone();
        let base = base.clone();
        std::rc::Rc::new(move |status: u16, body: String| {
            // The daemon answers `{"ok":true,...}` for both a fresh reset and a
            // peer-skipped one (`"skipped":true`) — either is completion.
            if body.contains("\"ok\":true") {
                complete();
                return;
            }
            // A live collaboration session owns this document, so the daemon
            // refuses to reset it (409 `collab-active`). That is a deliberate
            // answer, not a failure: resetting is exactly the wrong thing to do
            // to a document peers are editing. Retrying would only ask again
            // and warn about a daemon that is working correctly.
            if status == 409 {
                complete();
                return;
            }
            // Error body / empty (transport failure, or an XHR timeout ->
            // status 0 + empty body): retry once, then proceed.
            if retries_left > 0 {
                start_bootstrap_reset(base.clone(), complete.clone(), retries_left - 1);
                return;
            }
            web_sys::console::warn_1(&JsValue::from_str(
                "[op-bridge] sync-reset failed after retry; proceeding on a best-effort daemon",
            ));
            complete();
        })
    };
    if !crate::live_sync::post_json_with_status(&url, "", on_reset) {
        // Request could not even start — treat as a transport error.
        if retries_left > 0 {
            start_bootstrap_reset(base, complete, retries_left - 1);
        } else {
            web_sys::console::warn_1(&JsValue::from_str(
                "[op-bridge] sync-reset could not be issued after retry; proceeding",
            ));
            complete();
        }
    }
}

/// Run the managed late-init recovery: a tokened bootstrap sync-reset whose
/// completion emits `ready`. Shared by the two paths that recover a `ready` the
/// fallback (unmanaged) bootstrap could not emit — the completion-time inline
/// path (the host's `init` arrived DURING the fallback reset) and the
/// `LATE_INIT_HOOK` path in `vscode_bridge` (it arrived AFTER completion). The
/// reset carries the now-stored token; the daemon's Task-5 guard makes a repeat
/// reset a no-op skip (`"ok":true` / `"skipped":true`) that still counts as
/// completion. The inner one-shot guard means `ready` cannot double-fire across
/// the reset's own retry.
pub(super) fn run_late_init_recovery(
    base: String,
    inner_ready: std::rc::Rc<std::cell::RefCell<CkInner>>,
) {
    let done = std::rc::Rc::new(std::cell::Cell::new(false));
    let complete: std::rc::Rc<dyn Fn()> = std::rc::Rc::new(move || {
        if done.replace(true) {
            return;
        }
        crate::vscode_bridge::emit_ready(&inner_ready);
    });
    start_bootstrap_reset(base, complete, BOOTSTRAP_RESET_RETRIES);
}
