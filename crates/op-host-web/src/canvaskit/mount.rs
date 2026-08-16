//! Body of the `mount_ck` entry point.
//! Builds the `CkInner` shell and installs the web editor's DOM listeners.
//! The `#[wasm_bindgen]` export stays in the `canvaskit` spine.

use wasm_bindgen::prelude::*;

use super::backend::init_backend;
use super::inner::{
    dispatch_a11y_dom_event, run_late_init_recovery, start_bootstrap_reset, CkInner,
    BOOTSTRAP_RESET_RETRIES,
};

/// See [`super::mount_ck`] — the `#[wasm_bindgen]` export delegates here.
pub(super) async fn mount_ck(canvas_id: String) -> Result<(), JsValue> {
    use crate::listener::{add_listener, now_ms_perf, now_unix_secs, Listener};
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::JsCast;
    use web_sys::{KeyboardEvent, MouseEvent, WheelEvent};

    console_error_panic_hook::set_once();

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("mount_ck: no window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("mount_ck: no document"))?;
    let canvas: web_sys::HtmlCanvasElement = document
        .get_element_by_id(&canvas_id)
        .ok_or_else(|| JsValue::from_str("mount_ck: canvas not found"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("mount_ck: element is not a <canvas>"))?;

    // Device backing store vs CSS size → device-pixel-ratio.
    let dev_w = canvas.width().max(1) as f32;
    let dev_h = canvas.height().max(1) as f32;
    let css_w = (canvas.client_width().max(1)) as f32;
    let dpr = (dev_w / css_w).max(1.0);
    let logical_w = (dev_w / dpr).round().max(1.0) as u32;
    let logical_h = (dev_h / dpr).round().max(1.0) as u32;

    let backend = init_backend(&canvas_id, dpr, logical_w, logical_h).await?;
    let mut host = crate::widget_host::WidgetHost::new();
    // The editor opens on its canvas, not on a chat panel: the AI panel
    // starts as its compact input bar and expands on click. Applied here
    // rather than in `WidgetHost::new` because that constructor is also
    // every widget test's fixture — this is the mount-time policy, not
    // the host's resting state.
    host.editor_state_mut().chat.minimize();
    // Embed-host flag from the page URL (`?embed=vscode` in the VS Code
    // plugin iframe): parsed before the first paint so embedded chrome
    // never flashes in.
    let search = window.location().search().unwrap_or_default();
    host.editor_state_mut().editor_ui.embed = op_editor_core::EmbedHost::from_query(&search);
    let credential_load = crate::web_settings::load_into(host.editor_state_mut());
    // Theme is device-level, so it is resolved from its own unpartitioned key
    // rather than from the partition blob just loaded. On a browser that has
    // never run the split build this adopts the blob's theme and writes the
    // device key, so an existing choice carries over instead of resetting.
    crate::web_settings::theme::apply_after_load(
        host.editor_state_mut(),
        credential_load.payload_theme(),
    );
    // A managed embedding host may impose its current color scheme. Apply it
    // after the user's stored theme but still before the synchronous first
    // paint, so the iframe never flashes the wrong theme. The theme module
    // retains the underlying user preference for every persistence path.
    if let Some(theme) = crate::web_settings::theme::host_theme_from_query(&search) {
        crate::web_settings::theme::set_host_override(host.editor_state_mut(), theme);
    }
    if let Some(locale) = crate::web_settings::host_locale_from_query(&search) {
        host.editor_state_mut()
            .editor_ui
            .set_host_locale_override(Some(locale));
    }
    host.mark_editor_state_dirty();
    let settings_fingerprint = credential_load.initial_settings_fingerprint(host.editor_state());
    let credential_fingerprint = credential_load.initial_fingerprint(host.editor_state());
    let initial_credential_json = credential_load
        .loaded
        .then(|| crate::web_settings::server_credentials_json(host.editor_state()))
        .flatten();
    // Hidden ARIA DOM mirror (#57) — created next to the canvas, refreshed
    // after every paint so screen readers can read the opaque GPU surface.
    let a11y = crate::a11y_dom::A11yDomMirror::create(&canvas);
    // Hidden IME-capture input (#54) — composition is wired to `apply_ime`
    // below; focus is driven from `input_active()` in `repaint`.
    let ime = crate::ime_input::ImeInput::create(&canvas);
    let inner = Rc::new(RefCell::new(CkInner {
        backend,
        host,
        settings_fingerprint,
        credential_fingerprint,
        canvas: canvas.clone(),
        a11y,
        ime,
    }));
    // Reset the credential-sync queue BEFORE the first repaint and before the
    // rAF coalescer is installed below. `repaint` calls
    // `web_credential_sync::credential_changed` whenever a credential edit
    // lands, so the reset must precede any repaint wiring — otherwise an early
    // repaint could queue a change that a later reset silently wipes. This is a
    // pure state reset (no daemon request), so it is safe ahead of the bridge
    // init gate; the daemon-facing policy fetch (`start`) still waits for it.
    crate::web_credential_sync::reset();
    {
        let mut b = inner.borrow_mut();
        let _ = b.resize_to_window(&window)?;
        // The CanvasKit backend accepts runtime font bytes, so the browser
        // shell supports user font import — flip the flag the shared picker
        // reads to paint the Imported group + "Import font…" row (#Phase 4).
        b.host.editor_state_mut().editor_ui.font_import_supported = true;
        // Same for the Styles tab's DESIGN.md import: a hidden `<input
        // type=file>` is a file dialog for this purpose, so the box paints its
        // "choose file" button alongside the paste area rather than offering
        // the paste route alone.
        b.host
            .editor_state_mut()
            .editor_ui
            .style_import_file_picker_supported = true;
        // Online / hub-served mode: a `?tenant=` page URL means the hub is
        // serving this editor at its own origin, so its `/mcp-tokens`
        // portal page is reachable. Reveal the "MCP Tokens" row in the
        // signed-in account dropdown. Self-hosted serve-web (no tenant) and
        // native desktop leave this false so the row never appears there.
        b.host.editor_state_mut().editor_ui.account_mcp_tokens_entry =
            crate::daemon_base::tenant_param().is_some();
        // First frame paints synchronously so the shell is visible immediately
        // (no one-frame blank). Subsequent input-driven repaints coalesce
        // through the rAF installed below.
        b.repaint();
    }
    // Route every input-driven repaint through one rAF (see `repaint_coalescer`):
    // the paint closure borrows the shell and re-arms if it is momentarily
    // borrowed when the frame fires. Installed AFTER the synchronous first frame
    // (so it stays the first paint) and before the DOM listeners below, which
    // are the only callers of `request()`.
    {
        let inner_for_paint = inner.clone();
        crate::repaint_coalescer::install(Rc::new(move || {
            if let Ok(mut b) = inner_for_paint.try_borrow_mut() {
                b.repaint();
                drop(b);
                crate::web_fonts::drain_font_requests(&inner_for_paint);
                crate::web_fonts::drain_missing_fonts_detection(&inner_for_paint);
            } else {
                crate::repaint_coalescer::request();
            }
        }));
    }
    if op_editor_ui::image_runtime::has_pending_decodes() {
        crate::repaint_coalescer::request();
    }
    crate::web_fonts::drain_font_requests(&inner);
    crate::web_fonts::drain_missing_fonts_detection(&inner);
    // Re-register any user-imported fonts persisted in IndexedDB (async; repaints
    // when the read lands so their text re-shapes with the imported typeface).
    crate::web_fonts::load_imported_fonts_at_mount(&inner);

    // ---- daemon bootstrap: startup sequence ----
    //
    // The `SyncController` (gate + wire client + push single-flight) is shared
    // with the postMessage bridge, so both observe/mutate one instance. Build
    // it FIRST so the bridge listener installs before any daemon service.
    let sync_controller: crate::live_sync_glue::SharedSync =
        Rc::new(RefCell::new(crate::live_sync_glue::SyncController::new()));
    // 1. Install the bridge listener + observer BEFORE any daemon request, so an
    //    Init / OpenDocument arriving during bootstrap is never missed.
    crate::vscode_bridge::install(&inner, sync_controller.clone());
    // 2. Inside a webview iframe, await the host's Init (token) with a 2s
    //    fallback (proceed as a direct open on timeout). A standalone browser
    //    tab is a direct open and continues immediately.
    let is_iframe = crate::vscode_bridge::in_iframe(&window);
    if is_iframe {
        crate::vscode_bridge::await_init(&window, 2000).await;
    }
    // 3. Only now start the daemon-dependent services — in managed mode the
    //    token is present so their requests carry the auth header.
    crate::web_credential_sync::start();
    if let Some(json) = initial_credential_json {
        crate::web_credential_sync::credential_changed(json);
    }
    // Populate the chat model picker from the daemon's `/api/ai/models`
    // catalog (best-effort; async, repaints when the response lands).
    crate::web_chat::fetch_models(&inner);
    // Pull the brand-logo catalog (omitted from the wasm bundle) from the daemon
    // in the background so the icon picker / figma can resolve simple-icons.
    crate::iconify_web::fetch_brand_catalog(&inner);
    // Mirror the daemon's agent-indicator registry so design runs paint
    // their agent borders / badges / reveal animations on web too.
    crate::agent_indicator_sync::start(&inner);
    // Device-login relay: seed `account_ui_available` + any session the
    // daemon restored (shared with the desktop GUI), then drive login
    // flows through the daemon's `/api/auth/*` proxy.
    crate::web_auth_sync::start(&inner);
    // Collaboration relay. Starts here rather than inside the sync-reset
    // completion because it neither reads nor writes the document — it drives
    // `editor_ui.collab` only — and the panel should report availability as
    // soon as the daemon can answer, not one bootstrap later.
    crate::collab_sync::start(&inner);
    // 4. Reset the daemon's transient sync document, THEN emit the managed
    //    `ready` reply and start the live-sync ticks. The reset must complete
    //    FIRST for two reasons:
    //      * The 400 ms pull tick must not run before the reset (in BOTH
    //        managed + direct paths) or it pulls the pre-reset state.
    //      * `ready` must be serialized after the reset (managed path): the
    //        host opens a document as soon as it sees `ready`, and an open push
    //        landing before the bootstrap reset would be clobbered when the
    //        reset resets the daemon to `--file` content and the next pull tick
    //        pulls that over the just-opened canvas. `ready` is therefore posted
    //        from the reset-completion callback here, never from `handle_init`.
    //    Managed mode issues the reset with the token (attached automatically by
    //    the `live_sync` helper); direct open issues the same reset — replacing
    //    the fetch removed from index.html — but never emits `ready` (no bridge).
    {
        let base = crate::daemon_base::daemon_base();
        // Captured before the fallback reset is issued: `true` only when the
        // host's `init` (token) had ALREADY landed, so the fallback reset below
        // is itself tokened and authoritative. `false` covers both a standalone
        // tab and a managed webview whose `init` is still in flight.
        let managed = crate::live_sync::bridge_token().is_some();
        let inner_for_sync = inner.clone();
        let inner_for_ready = inner.clone();
        let inner_for_hook = inner.clone();
        let base_for_recovery = base.clone();
        let sync_for_start = sync_controller.clone();
        // Single guarded completion (guarded so `start_bootstrap_reset`'s retry
        // path can never double-emit or double-start): start the live-sync ticks,
        // then settle `ready`. Readiness is decided from the LIVE token, NOT the
        // `managed` flag captured before the reset was issued — a slow host's
        // `init` can land anywhere in the reset's round-trip (up to ~30s with the
        // XHR timeout + one retry). Three cases close the window from both sides:
        //   * token present since capture -> the fallback reset was tokened, so
        //     emit `ready` directly (fast path, no extra reset).
        //   * token arrived DURING the (unmanaged) reset's round-trip -> re-run
        //     the managed recovery inline (tokened reset -> `ready`).
        //   * token still absent -> register the one-shot LATE_INIT_HOOK so a
        //     later `init` runs the same recovery (see `handle_init`).
        // The hook is registered ONLY here, after the fallback reset completed,
        // so the recovery reset can't interleave with it; a standalone tab
        // (`!is_iframe`) never receives an `init`, so it registers nothing.
        let done = std::rc::Rc::new(std::cell::Cell::new(false));
        let complete: std::rc::Rc<dyn Fn()> = {
            let done = done.clone();
            std::rc::Rc::new(move || {
                if done.replace(true) {
                    return;
                }
                crate::live_sync_glue::start(&inner_for_sync, sync_for_start.clone());
                if managed {
                    crate::vscode_bridge::emit_ready(&inner_for_ready);
                } else if crate::live_sync::bridge_token().is_some() {
                    run_late_init_recovery(base_for_recovery.clone(), inner_for_ready.clone());
                } else if is_iframe {
                    let inner_hook = inner_for_hook.clone();
                    let base_hook = base_for_recovery.clone();
                    crate::vscode_bridge::register_late_init_hook(move || {
                        run_late_init_recovery(base_hook.clone(), inner_hook.clone());
                    });
                }
            })
        };
        start_bootstrap_reset(base, complete, BOOTSTRAP_RESET_RETRIES);
    }

    let mut listeners: Vec<Listener> = Vec::new();
    let canvas_target: web_sys::EventTarget = canvas.clone().into();
    let win_target: web_sys::EventTarget = window.clone().into();

    // Accessibility DOM mirror (#57): delegated `focus` / `click` on the
    // hidden mirror container map a focused/activated mirror node back to a
    // host action (focus chat input, blur it on canvas/panel focus, …) then
    // repaint so the canvas reflects the screen-reader-driven change.
    let mirror_target = inner.try_borrow().ok().and_then(|b| {
        b.a11y
            .as_ref()
            .map(|m| -> web_sys::EventTarget { m.container().clone().into() })
    });
    if let Some(mirror_target) = mirror_target {
        // `focusin` bubbles (unlike `focus`), so a single delegated listener
        // on the container catches focus landing on any descendant node.
        {
            let inner = inner.clone();
            add_listener::<web_sys::FocusEvent, _, _>(
                &mirror_target,
                "focusin",
                &mut listeners,
                move |evt| {
                    dispatch_a11y_dom_event(&inner, evt.target(), true);
                },
            )?;
        }
        {
            let inner = inner.clone();
            add_listener::<MouseEvent, _, _>(
                &mirror_target,
                "click",
                &mut listeners,
                move |evt| {
                    dispatch_a11y_dom_event(&inner, evt.target(), false);
                },
            )?;
        }
    }

    // mousedown → press / right-press
    {
        let inner = inner.clone();
        add_listener::<MouseEvent, _, _>(
            &canvas_target,
            "mousedown",
            &mut listeners,
            move |evt| {
                use crate::event::pointer::{classify_mouse_press_button, MousePressAction};

                let action = classify_mouse_press_button(evt.button());
                if matches!(action, MousePressAction::Ignore) {
                    return;
                }
                if matches!(action, MousePressAction::MiddlePan) {
                    evt.prevent_default();
                }
                let Ok(mut b) = inner.try_borrow_mut() else {
                    return;
                };
                b.host.set_modifier_shift(evt.shift_key());
                b.host.set_modifier_alt(evt.alt_key());
                b.host.set_clocks(now_ms_perf(), now_unix_secs());
                let (w, h) = b.backend.logical_size();
                let (x, y) =
                    b.event_offset_to_logical(evt.offset_x() as f32, evt.offset_y() as f32);
                let consumed = match action {
                    MousePressAction::PrimaryPress => b.host.apply_press(x, y, w, h),
                    MousePressAction::MiddlePan => {
                        let started = b.host.apply_pan_press(x, y, w, h);
                        b.host.set_space_pan(started);
                        started
                    }
                    MousePressAction::ContextPress => b.host.apply_right_press(x, y, w, h),
                    MousePressAction::Ignore => false,
                };
                if consumed {
                    crate::repaint_coalescer::request();
                }
                // Release the borrow before draining: a Send / Stop / New Chat
                // button press raised a chat flag; an icon-picker Load-more
                // press queued a remote search. Both drains re-borrow `inner`
                // (mirrors the skia mount's post-press drain points).
                drop(b);
                crate::web_chat::drain_chat_flags(&inner);
                crate::web_image_panel::drain_image_jobs(&inner);
                crate::web_builtin_model_discovery::drain_pending_builtin_model_discovery(&inner);
                crate::iconify_web::drain_iconify_request(&inner);
                crate::codegen_web::drain_codegen_flags(&inner);
                crate::web_design_md::drain_design_md_action(&inner);
                crate::web_style_import::drain_pending_style_import(&inner);
                crate::dom_io::drain_pending_file_action(&inner);
                crate::dom_io::drain_pending_attachment_pick(&inner);
                crate::dom_io::drain_pending_kit_io(&inner);
                crate::theme_preset_io::drain_pending_theme_preset_io(&inner);
                crate::web_fonts::drain_font_requests(&inner);
                crate::web_fonts::drain_missing_fonts_detection(&inner);
            },
        )?;
    }
    // Suppress browser's native context menu over the canvas so right-click
    // stays reserved for editor context menus.
    {
        add_listener::<MouseEvent, _, _>(
            &canvas_target,
            "contextmenu",
            &mut listeners,
            move |evt| {
                evt.prevent_default();
            },
        )?;
    }
    // mousemove → cursor move / drag
    {
        let inner = inner.clone();
        add_listener::<MouseEvent, _, _>(
            &canvas_target,
            "mousemove",
            &mut listeners,
            move |evt| {
                {
                    let Ok(mut b) = inner.try_borrow_mut() else {
                        return;
                    };
                    b.host.set_modifier_shift(evt.shift_key());
                    b.host.set_modifier_alt(evt.alt_key());
                    b.host.set_clocks(now_ms_perf(), now_unix_secs());
                    let (x, y) =
                        b.event_offset_to_logical(evt.offset_x() as f32, evt.offset_y() as f32);
                    if b.host.apply_cursor_move(x, y) {
                        crate::repaint_coalescer::request();
                    }
                }
                // Landing on a top-bar button owes a repaint once its
                // tooltip's dwell expires, and no further DOM event is
                // coming to carry it. Arm the rAF pump after the borrow
                // is released so it can read the host it just updated.
                crate::tooltip_pump::ensure(&inner);
            },
        )?;
    }
    // Window-level mouseup → release. A text selection or canvas drag can
    // leave the canvas before the button is released; listening only on the
    // canvas would strand the drag state until a later in-canvas mouseup.
    {
        let inner = inner.clone();
        add_listener::<MouseEvent, _, _>(&win_target, "mouseup", &mut listeners, move |evt| {
            if evt.button() != 0 && evt.button() != 1 {
                return;
            }
            if evt.button() == 1 {
                evt.prevent_default();
            }
            let Ok(mut b) = inner.try_borrow_mut() else {
                return;
            };
            b.host.set_modifier_shift(evt.shift_key());
            b.host.set_modifier_alt(evt.alt_key());
            b.host.set_clocks(now_ms_perf(), now_unix_secs());
            let (w, h) = b.backend.logical_size();
            let was_middle = evt.button() == 1;
            if b.host.apply_release_with_viewport(w, h) {
                crate::repaint_coalescer::request();
            }
            if was_middle {
                b.host.set_space_pan(false);
            }
        })?;
    }
    // wheel → pan / zoom
    {
        let inner = inner.clone();
        add_listener::<WheelEvent, _, _>(&canvas_target, "wheel", &mut listeners, move |evt| {
            use crate::event::pointer::{classify_wheel_intent, WheelIntent};

            evt.prevent_default();
            let Ok(mut b) = inner.try_borrow_mut() else {
                return;
            };
            let (w, h) = b.backend.logical_size();
            let (x, y) = b.event_offset_to_logical(evt.offset_x() as f32, evt.offset_y() as f32);
            let mut modifiers = op_editor_ui::Modifiers::empty();
            modifiers.set(op_editor_ui::Modifiers::SHIFT, evt.shift_key());
            modifiers.set(op_editor_ui::Modifiers::CTRL, evt.ctrl_key());
            modifiers.set(op_editor_ui::Modifiers::CMD, evt.meta_key());
            modifiers.set(op_editor_ui::Modifiers::ALT, evt.alt_key());
            let consumed = match classify_wheel_intent(
                evt.delta_x() as f32,
                evt.delta_y() as f32,
                evt.delta_mode(),
                w,
                h,
                modifiers,
            ) {
                WheelIntent::Zoom {
                    scroll_delta_y,
                    canvas_delta_y,
                } => {
                    b.host
                        .apply_wheel_with_canvas_delta(x, y, scroll_delta_y, canvas_delta_y, w, h)
                }
                WheelIntent::Pan { dx, dy } => b.host.apply_pan_gesture(x, y, dx, dy, w, h),
            };
            if consumed {
                crate::repaint_coalescer::request();
            }
        })?;
    }
    // compositionstart/end → IME (#54). The hidden `ime` input captures the
    // composition (a `<canvas>` can't); on commit, `apply_ime` routes the
    // string through `apply_text` into whichever field owns the keyboard.
    // `compositionstart` clears the throwaway buffer so it never accumulates;
    // the commit is read from the event's `data`, never the input value.
    if let Some(ime_target) = inner.try_borrow().ok().and_then(|b| {
        b.ime
            .as_ref()
            .map(|i| -> web_sys::EventTarget { i.input().clone().into() })
    }) {
        {
            let inner = inner.clone();
            add_listener::<web_sys::CompositionEvent, _, _>(
                &ime_target,
                "compositionstart",
                &mut listeners,
                move |_evt| {
                    if let Ok(b) = inner.try_borrow() {
                        if let Some(ime) = b.ime.as_ref() {
                            ime.clear();
                        }
                    }
                },
            )?;
        }
        {
            let inner = inner.clone();
            add_listener::<web_sys::CompositionEvent, _, _>(
                &ime_target,
                "compositionend",
                &mut listeners,
                move |evt| {
                    let Ok(mut b) = inner.try_borrow_mut() else {
                        return;
                    };
                    b.host.set_clocks(now_ms_perf(), now_unix_secs());
                    let committed = evt.data().unwrap_or_default();
                    let ime_evt = crate::event::ime::composition_end(committed);
                    let consumed = b.host.apply_ime(&ime_evt);
                    if let Some(ime) = b.ime.as_ref() {
                        ime.clear();
                    }
                    if consumed {
                        crate::repaint_coalescer::request();
                    }
                },
            )?;
        }
        // beforeinput → the text that never opens a composition. CJK
        // punctuation (《 》 【 】 —— ……) is resolved by the IME the instant
        // the key is pressed, so `compositionend` never fires for it and the
        // raw `keydown` carries the untransformed key, not what the IME
        // produced. This is the only path that sees the real character; the
        // `keydown` handler yields its text branch while this input owns
        // focus so the two never both type.
        {
            let inner = inner.clone();
            add_listener::<web_sys::InputEvent, _, _>(
                &ime_target,
                "beforeinput",
                &mut listeners,
                move |evt| {
                    let Some(text) = crate::event::ime::beforeinput_text(
                        &evt.input_type(),
                        evt.data().as_deref(),
                        evt.is_composing(),
                    ) else {
                        return;
                    };
                    let Ok(mut b) = inner.try_borrow_mut() else {
                        return;
                    };
                    b.host.set_clocks(now_ms_perf(), now_unix_secs());
                    let consumed = b.host.apply_paste_text(&text);
                    // The buffer is a throwaway: never read, always cleared,
                    // so it cannot accumulate across keystrokes.
                    if let Some(ime) = b.ime.as_ref() {
                        ime.clear();
                    }
                    if consumed {
                        crate::repaint_coalescer::request();
                    }
                    // Enter is not an `insertText`, but a send can still be
                    // queued by a field reacting to the inserted text.
                    drop(b);
                    crate::web_chat::drain_chat_flags(&inner);
                    crate::web_image_panel::drain_image_jobs(&inner);
                },
            )?;
        }
    }
    // keydown → text input + editor shortcuts. `apply_key` is a stub on this
    // host; real input is dispatched per-key to apply_text / apply_backspace /
    // apply_send / nudge / reorder / clipboard / undo etc. (mirrors the skia
    // mount in lib.rs). Mod = Cmd/Ctrl; named-key shortcuts gate on `!is_mod`.
    {
        let inner = inner.clone();
        add_listener::<KeyboardEvent, _, _>(&win_target, "keydown", &mut listeners, move |evt| {
            use op_editor_core::ReorderDirection;
            // In-flight IME composition owns its keystrokes (handled on commit).
            if evt.is_composing() {
                return;
            }
            let Ok(mut b) = inner.try_borrow_mut() else {
                return;
            };
            b.host.set_clocks(now_ms_perf(), now_unix_secs());
            let key = evt.key();
            let starts_space_pan = evt.code() == "Space"
                && !evt.repeat()
                && !evt.meta_key()
                && !evt.ctrl_key()
                && !evt.alt_key();
            let is_mod = evt.meta_key() || evt.ctrl_key();
            let shift = evt.shift_key();
            let nudge = if shift { 10.0 } else { 1.0 };
            let image_popover_open = {
                let panel = &b.host.editor_state().editor_ui.image_panel;
                panel.search_open || panel.generate_open
            };
            let prompt_center_open = b.host.editor_state().editor_ui.prompt_center.open;
            let mut consumed = false;
            if starts_space_pan && !b.host.input_active() {
                b.host.set_space_pan(true);
                evt.prevent_default();
            }
            match key.as_str() {
                "Backspace" if !is_mod => consumed = b.host.apply_backspace(),
                "Delete" if !is_mod => consumed = b.host.apply_delete(),
                "Enter" if !is_mod => consumed = b.host.apply_send(),
                "Escape" if !is_mod => consumed = b.host.apply_escape(),
                "ArrowUp" | "ArrowDown" if !is_mod && prompt_center_open => consumed = true,
                "ArrowLeft" | "ArrowRight" if is_mod && prompt_center_open => consumed = true,
                "ArrowUp" if !is_mod && image_popover_open => consumed = true,
                "ArrowDown" if !is_mod && image_popover_open => consumed = true,
                "ArrowUp" if !is_mod => {
                    consumed = b.host.apply_text_edit_vertical(false)
                        || b.host.apply_chat_input_vertical_caret(false, shift)
                        || b.host.apply_nudge(0.0, -nudge)
                }
                "ArrowDown" if !is_mod => {
                    consumed = b.host.apply_text_edit_vertical(true)
                        || b.host.apply_chat_input_vertical_caret(true, shift)
                        || b.host.apply_nudge(0.0, nudge)
                }
                "ArrowLeft" if is_mod && image_popover_open => {
                    consumed = b.host.apply_image_panel_edge(false, shift)
                }
                "ArrowRight" if is_mod && image_popover_open => {
                    consumed = b.host.apply_image_panel_edge(true, shift)
                }
                "Home" if image_popover_open => {
                    consumed = b.host.apply_image_panel_edge(false, shift)
                }
                "End" if image_popover_open => {
                    consumed = b.host.apply_image_panel_edge(true, shift)
                }
                "ArrowLeft" if is_mod => consumed = b.host.apply_text_edit_line_edge(false),
                "ArrowRight" if is_mod => consumed = b.host.apply_text_edit_line_edge(true),
                "ArrowLeft" if !is_mod => {
                    consumed = b.host.apply_prompt_center_caret(false, shift)
                        || b.host.apply_image_panel_caret(false, shift)
                        || b.host.apply_settings_caret(false)
                        || b.host.apply_chat_model_picker_caret(false)
                        || b.host.apply_chat_input_caret(false, shift)
                        || b.host.apply_rename_caret(false)
                        || b.host.apply_text_edit_caret(false)
                        || b.host.apply_property_caret(false)
                        || b.host.apply_nudge(-nudge, 0.0);
                }
                "ArrowRight" if !is_mod => {
                    consumed = b.host.apply_prompt_center_caret(true, shift)
                        || b.host.apply_image_panel_caret(true, shift)
                        || b.host.apply_settings_caret(true)
                        || b.host.apply_chat_model_picker_caret(true)
                        || b.host.apply_chat_input_caret(true, shift)
                        || b.host.apply_rename_caret(true)
                        || b.host.apply_text_edit_caret(true)
                        || b.host.apply_property_caret(true)
                        || b.host.apply_nudge(nudge, 0.0);
                }
                "[" if !is_mod && !b.host.input_active() => {
                    consumed = b.host.apply_reorder(ReorderDirection::Down)
                }
                "]" if !is_mod && !b.host.input_active() => {
                    consumed = b.host.apply_reorder(ReorderDirection::Up)
                }
                "F" | "f" | "H" | "h" | "K" | "k"
                    if is_mod
                        && shift
                        && !evt.alt_key()
                        && !image_popover_open
                        && !prompt_center_open =>
                {
                    consumed =
                        b.host
                            .apply_keydown_shortcut(key.as_str(), is_mod, shift, evt.alt_key())
                }
                "d" | "D" if is_mod && !shift && !image_popover_open && !prompt_center_open => {
                    consumed = b.host.apply_duplicate()
                }
                // Cmd/Ctrl+T — open a fresh chat tab (MT.3).
                "t" | "T" if is_mod && !shift && !image_popover_open && !prompt_center_open => {
                    consumed = b.host.apply_new_chat_tab()
                }
                "a" | "A" if is_mod && !shift => consumed = b.host.apply_select_all(),
                "c" | "C" if is_mod && !shift => consumed = b.host.apply_copy(),
                "x" | "X" if is_mod && !shift => consumed = b.host.apply_cut(),
                // Cmd/Ctrl+V is owned by the DOM `paste` listener
                // (`dom_io::register_io_listeners` → `handle_paste_event`): it
                // routes the system clipboard first (Figma HTML / image / text),
                // then falls back to the internal node clipboard. Calling
                // `apply_paste` here would preempt that with the STALE internal
                // clipboard + double-paste, so leave Cmd+V unconsumed → the
                // browser's native paste fires the `paste` event. (Mirrors the
                // skia codegen build, lib.rs.)
                "v" | "V" if is_mod && !shift => {}
                _ if is_mod && prompt_center_open => consumed = true,
                // Case-insensitive: with Shift held, `key` is layout/IME
                // dependent — macOS Chromium can report either "z" or "Z"
                // for Cmd+Shift+Z, so branch on the shift flag alone.
                // In a live session history belongs to the session, not this
                // tab: undo becomes an M1 selective-undo request the daemon
                // sequences, and redo is refused outright. Both short-circuit
                // local history exactly as the desktop host does.
                "z" | "Z" if is_mod && !image_popover_open => {
                    consumed = if shift {
                        crate::collab_sync::reject_redo(b.host.editor_state_mut())
                            || b.host.apply_redo()
                    } else {
                        crate::collab_sync::request_undo(b.host.editor_state_mut())
                            || b.host.apply_undo()
                    };
                }
                "y" | "Y" if is_mod && !shift && !image_popover_open => {
                    consumed = crate::collab_sync::reject_redo(b.host.editor_state_mut())
                        || b.host.apply_redo()
                }
                "s" | "S" if is_mod && !shift => {
                    // VS Code embed: the workbench cannot observe keystrokes
                    // inside this cross-origin iframe, so Cmd/Ctrl+S must be
                    // forwarded for a host-side save (extension runs the
                    // regular workbench save → saveCustomDocument). Outside
                    // the embed the browser default stays suppressed-by-noop.
                    if b.host.editor_state().editor_ui.embed == op_editor_core::EmbedHost::VsCode {
                        crate::web_clipboard::post_save_to_parent();
                    }
                    consumed = true;
                }
                _ if is_mod && image_popover_open => consumed = true,
                _ => {
                    // No Cmd/Ctrl held: a bare letter is first offered to the
                    // single-key tool router (V/R/O/L/T/F/P/Y/H), which self-
                    // gates on no input owning the keyboard; every other letter
                    // (and any keystroke while a field is focused) types via
                    // apply_text.
                    if !is_mod {
                        // Alt-modified keys never switch tools — Alt is a chord
                        // modifier (and on macOS yields special glyphs like ®/π),
                        // so an Alt+letter must not trip the bare-letter router.
                        // A resulting printable char still types via apply_text.
                        if !evt.alt_key() && b.host.apply_tool_shortcut(key.as_str()) {
                            consumed = true;
                        } else if crate::event::ime::keydown_should_insert_text(
                            b.ime.as_ref().is_some_and(|ime| ime.owns_dom_focus()),
                        ) {
                            let mut chars = key.chars();
                            if let (Some(c), None) = (chars.next(), chars.next()) {
                                if !c.is_control() && b.host.apply_text(c) {
                                    consumed = true;
                                }
                            }
                        }
                    }
                }
            }
            if consumed {
                evt.prevent_default();
                crate::repaint_coalescer::request();
            }
            // Release the borrow before draining: Enter may have queued a chat
            // send (apply_send → pending_send) or an image-panel search
            // (apply_image_panel_send → search_epoch); the drains launch them.
            drop(b);
            crate::web_chat::drain_chat_flags(&inner);
            crate::web_image_panel::drain_image_jobs(&inner);
            crate::web_builtin_model_discovery::drain_pending_builtin_model_discovery(&inner);
        })?;
    }

    {
        let inner = inner.clone();
        add_listener::<KeyboardEvent, _, _>(&win_target, "keyup", &mut listeners, move |evt| {
            if evt.code() != "Space" {
                return;
            }
            let Ok(mut b) = inner.try_borrow_mut() else {
                return;
            };
            b.host.set_space_pan(false);
            evt.prevent_default();
        })?;
    }

    {
        let inner = inner.clone();
        let window_for_resize = window.clone();
        add_listener::<web_sys::Event, _, _>(&win_target, "resize", &mut listeners, move |_evt| {
            let Ok(mut b) = inner.try_borrow_mut() else {
                return;
            };
            match b.resize_to_window(&window_for_resize) {
                Ok(true) => crate::repaint_coalescer::request(),
                Ok(false) => {}
                Err(err) => web_sys::console::error_1(&err),
            }
        })?;
    }

    crate::dom_io::register_io_listeners(&inner, &canvas, &win_target, &mut listeners)?;

    // Retain the shell + its listeners for the page lifetime. (A future
    // WebShell-style handle can own these for explicit teardown; leaking keeps
    // the CanvasKit surface + DOM closures alive for now.)
    std::mem::forget(inner);
    std::mem::forget(listeners);
    Ok(())
}
