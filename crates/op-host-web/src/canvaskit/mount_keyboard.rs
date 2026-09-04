//! Keyboard listener registration for the CanvasKit web mount.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use web_sys::KeyboardEvent;

use super::inner::CkInner;
use crate::listener::{add_listener, now_ms_perf, now_unix_secs, Listener};

pub(super) fn register_keyboard_listeners(
    inner: &Rc<RefCell<CkInner>>,
    win_target: &web_sys::EventTarget,
    listeners: &mut Vec<Listener>,
) -> Result<(), JsValue> {
    // keydown → text input + editor shortcuts. `apply_key` is a stub on this
    // host; real input is dispatched per-key to apply_text / apply_backspace /
    // apply_send / nudge / reorder / clipboard / undo etc. (mirrors the skia
    // mount in lib.rs). Mod = Cmd/Ctrl; named-key shortcuts gate on `!is_mod`.
    {
        let inner = Rc::clone(inner);
        add_listener::<KeyboardEvent, _, _>(win_target, "keydown", listeners, move |evt| {
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
            let preview_active = b.host.is_preview_active();
            let mut consumed = false;
            if starts_space_pan && !b.host.input_active() {
                b.host.set_space_pan(true);
                evt.prevent_default();
            }
            match key.as_str() {
                "Backspace" if !is_mod => consumed = b.host.apply_backspace(),
                "Delete" if !is_mod => consumed = b.host.apply_delete(),
                "Enter" if !is_mod => consumed = b.host.apply_send(),
                "Escape" if !is_mod => {
                    if preview_active {
                        // Exit preview mode (same as native behavior).
                        // Note: Esc is called from keydown before paint has the
                        // current viewport dimensions; exit logic will short-circuit
                        // if there's no device frame, so pass zero dimensions here.
                        b.host.exit_preview(0.0, 0.0);
                        consumed = true;
                    } else {
                        consumed = b.host.apply_escape();
                    }
                }
                "Tab" if !is_mod && preview_active => {
                    // Tab / Shift+Tab for preview focus traversal
                    consumed = b.host.apply_preview_focus(shift);
                    if consumed {
                        evt.prevent_default();
                    }
                }
                // Preview owns the navigation keys BEFORE the editor's own
                // arrow arms below (canvas nudge / panel scroll): during a
                // deck presentation they step the slides, and in an app
                // preview they belong to the runtime, not the editor. The
                // late fallthrough arm can't do this — the nudge arms match
                // first — so this mirror of the native key order is load-
                // bearing, not a duplicate.
                "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight" | "Home" | "End"
                    if !is_mod && preview_active =>
                {
                    consumed = b.host.apply_preview_key(key.as_str(), shift);
                    if consumed {
                        evt.prevent_default();
                    }
                }
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
                    // Preview mode takes precedence over canvas text editing and
                    // other shortcuts. Dispatch keys first to preview if active.
                    if preview_active && !is_mod {
                        // Try dispatching to preview runtime first
                        let mut chars = key.chars();
                        if let (Some(c), None) = (chars.next(), chars.next()) {
                            // Printable characters — dispatch as text
                            if !c.is_control() && b.host.apply_preview_text(&c.to_string()) {
                                consumed = true;
                            }
                        }
                        // Dispatch non-printable named keys to preview
                        if !consumed {
                            consumed = b.host.apply_preview_key(key.as_str(), shift);
                        }
                    }
                    // No Cmd/Ctrl held: a bare letter is first offered to the
                    // single-key tool router (V/R/O/L/T/F/P/Y/H), which self-
                    // gates on no input owning the keyboard; every other letter
                    // (and any keystroke while a field is focused) types via
                    // apply_text.
                    if !consumed && !is_mod {
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
        let inner = Rc::clone(inner);
        add_listener::<KeyboardEvent, _, _>(win_target, "keyup", listeners, move |evt| {
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

    Ok(())
}
