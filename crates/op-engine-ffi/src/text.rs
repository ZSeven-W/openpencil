//! Inline canvas text editing + IME for the player shells.
//!
//! The engine drives [`op_editor_core::EditorState`]'s text-edit session
//! (the same model the desktop editor uses): tapping a `Text` node enters
//! edit mode, the draft lives in `ui.text_edit_input`, every mutation
//! syncs back into the document, and the paint pass renders the draft +
//! composition + caret through the shared canvas painter. The shells stay
//! thin: they forward UTF-16 IME events (`UITextInput` / `InputConnection`)
//! and the engine converts offsets to UTF-8 bytes.
//!
//! Scene parity: the layout scene is rebuilt after every text mutation, so
//! `fit_content` boxes re-measure live exactly as the desktop canvas does
//! (the desktop rebuilds its scene cache on document generation changes).

use crate::desc::OpTextState;
use crate::error::{read_utf8, FfiError, STRING_CAP};
use crate::lifecycle::{call_session, Session};
use crate::OpStatus;
use jian_core::render::{byte_to_utf16_offset, utf16_len, utf16_to_byte_offset};
use jian_core::text_input::prev_char_boundary;
use op_editor_core::NodeId;
use op_editor_ui::widgets::canvas_text_edit::text_edit_layout;
use op_editor_ui::widgets::canvas_viewport::EditCaret;
use op_editor_ui::Color;

/// Enter inline text-edit mode on the node with id `node_id`. The shell
/// then shows the system keyboard (the `input_focus_changed` callback
/// fires with `focused = true`).
///
/// # Safety
///
/// `engine` must be live and `node_id` must cover `node_id_len` readable
/// bytes.
#[no_mangle]
pub unsafe extern "C" fn op_text_begin(
    engine: *mut crate::OpEngine,
    node_id_ptr: *const u8,
    node_id_len: usize,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let node_id = read_utf8(node_id_ptr, node_id_len, STRING_CAP, "node id")?;
            let started = session.state.start_text_edit(NodeId::new(node_id.as_str()));
            if !started {
                return Err(FfiError::new(
                    OpStatus::InvalidArg,
                    "node is not an editable Text node",
                ));
            }
            session.rebuild_scene();
            session.focus_changed(true, 0, 0);
            session.request_redraw();
            Ok(())
        })
    }
}

/// Exit text-edit mode, committing the draft into the document. The shell
/// hides the keyboard (`input_focus_changed(false)` fires).
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_text_end(engine: *mut crate::OpEngine) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if session.state.text_edit_commit() {
                session.rebuild_scene();
                session.focus_changed(false, 0, 0);
                session.request_redraw();
            }
            Ok(())
        })
    }
}

/// Insert `text` at the caret, replacing any active selection.
///
/// # Safety
///
/// `engine` must be live and `text` must cover `text_len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn op_text_insert(
    engine: *mut crate::OpEngine,
    text_ptr: *const u8,
    text_len: usize,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if session.state.ui.text_editing.is_none() {
                return Err(FfiError::new(OpStatus::NoFocus, "no text edit in progress"));
            }
            let text = read_utf8(text_ptr, text_len, STRING_CAP, "inserted text")?;
            session.state.text_edit_insert(&text, session.now_ms);
            session.rebuild_scene();
            session.request_redraw();
            Ok(())
        })
    }
}

/// Delete the selection, or the character before the caret.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_text_backspace(engine: *mut crate::OpEngine) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if session.state.ui.text_editing.is_none() {
                return Err(FfiError::new(OpStatus::NoFocus, "no text edit in progress"));
            }
            session.state.text_edit_backspace(session.now_ms);
            session.rebuild_scene();
            session.request_redraw();
            Ok(())
        })
    }
}

/// Delete the selection, or the character after the caret (Delete key).
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_text_delete_forward(engine: *mut crate::OpEngine) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if session.state.ui.text_editing.is_none() {
                return Err(FfiError::new(OpStatus::NoFocus, "no text edit in progress"));
            }
            session.state.text_edit_delete_forward(session.now_ms);
            session.rebuild_scene();
            session.request_redraw();
            Ok(())
        })
    }
}

/// Place the caret at UTF-16 `offset` (clamped to a char boundary);
/// `extend` keeps/establishes the selection anchor (Shift+click).
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_text_set_caret(
    engine: *mut crate::OpEngine,
    offset: u32,
    extend: bool,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let Some(input) = session.editing_input() else {
                return Err(FfiError::new(OpStatus::NoFocus, "no text edit in progress"));
            };
            let byte = utf16_to_byte_offset(input.text(), offset);
            session
                .state
                .text_edit_set_caret(byte, extend, session.now_ms);
            session.request_redraw();
            Ok(())
        })
    }
}

/// Drag-select: anchor at UTF-16 `anchor`, caret follows `focus`.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_text_select_range(
    engine: *mut crate::OpEngine,
    anchor: u32,
    focus: u32,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let Some(input) = session.editing_input() else {
                return Err(FfiError::new(OpStatus::NoFocus, "no text edit in progress"));
            };
            let anchor_byte = utf16_to_byte_offset(input.text(), anchor);
            let focus_byte = utf16_to_byte_offset(input.text(), focus);
            session
                .state
                .text_edit_select_range(anchor_byte, focus_byte, session.now_ms);
            session.request_redraw();
            Ok(())
        })
    }
}

/// Set the in-flight IME composition. `sel_start` / `sel_end` are UTF-16
/// offsets within the NEW composing text (iOS `setMarkedText`
/// `selectedRange`; Android callers pre-convert `newCursorPosition`).
///
/// # Safety
///
/// `engine` must be live and `text` must cover `text_len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn op_ime_set_composing_text(
    engine: *mut crate::OpEngine,
    text_ptr: *const u8,
    text_len: usize,
    sel_start: u32,
    sel_end: u32,
) -> OpStatus {
    let _ = sel_end; // reserved: in-composition selection range
    unsafe {
        call_session(engine, |session| {
            if session.state.ui.text_editing.is_none() {
                return Err(FfiError::new(OpStatus::NoFocus, "no text edit in progress"));
            }
            let text = read_utf8(text_ptr, text_len, STRING_CAP, "composing text")?;
            // The editor wrapper takes a single cursor (byte offset within
            // the composing text); the common IME case has a collapsed
            // selection, so the start offset is the cursor.
            let cursor = utf16_to_byte_offset(&text, sel_start);
            session
                .state
                .text_edit_set_composition(&text, cursor, session.now_ms);
            session.rebuild_scene();
            session.request_redraw();
            Ok(())
        })
    }
}

/// Commit the in-flight IME composition into the draft.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_ime_commit_composition(engine: *mut crate::OpEngine) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if session.state.ui.text_editing.is_none() {
                return Err(FfiError::new(OpStatus::NoFocus, "no text edit in progress"));
            }
            session.state.text_edit_commit_composition(session.now_ms);
            session.rebuild_scene();
            session.request_redraw();
            Ok(())
        })
    }
}

/// Cancel the in-flight IME composition (the preedit region is deleted).
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_ime_cancel_composition(engine: *mut crate::OpEngine) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if session.state.ui.text_editing.is_none() {
                return Err(FfiError::new(OpStatus::NoFocus, "no text edit in progress"));
            }
            session.state.text_edit_clear_composition();
            session.rebuild_scene();
            session.request_redraw();
            Ok(())
        })
    }
}

/// Snapshot the surrounding-text state (borrowed text pointer — the shell
/// copies before the next engine call).
///
/// # Safety
///
/// `engine` must be live and `out` must be writable.
#[no_mangle]
pub unsafe extern "C" fn op_text_get_state(
    engine: *mut crate::OpEngine,
    out: *mut OpTextState,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if out.is_null() {
                return Err(FfiError::invalid("text-state output pointer is null"));
            }
            let Some(input) = session.editing_input() else {
                return Err(FfiError::new(OpStatus::NoFocus, "no text edit in progress"));
            };
            let text = input.text();
            let (sel_start, sel_end) = input.selection().ordered();
            let has_composing = input.composition().is_some();
            // The composing region may extend past the base text (the
            // preedit is not part of it), so the end offset converts the
            // composition's own UTF-16 length instead of clamping into the
            // base string.
            let (comp_start, comp_end) = match input.effective_composing_range() {
                Some((start, _end)) => {
                    let start_u16 = byte_to_utf16_offset(text, start);
                    let len_u16 = input
                        .composition()
                        .map(|c| utf16_len(c.text.as_str()))
                        .unwrap_or(0);
                    (
                        start_u16,
                        start_u16
                            .saturating_add(len_u16)
                            .min(utf16_len(text) + len_u16),
                    )
                }
                None => (0, 0),
            };
            out.write(OpTextState {
                text_ptr: text.as_ptr(),
                text_len: text.len(),
                selection_start: byte_to_utf16_offset(text, sel_start),
                selection_end: byte_to_utf16_offset(text, sel_end),
                has_composing,
                composing_start: comp_start,
                composing_end: comp_end,
            });
            Ok(())
        })
    }
}

/// Caret rect in surface-logical points (`out` receives `x, y, w, h`),
/// used by the shells for cursor anchoring (Android `CursorAnchorInfo`,
/// candidate-window placement). Geometry matches the painted caret: the
/// same line layout the canvas painter uses, with the composition-aware
/// caret offset.
///
/// # Safety
///
/// `engine` must be live and `out` must cover 4 writable floats.
#[no_mangle]
pub unsafe extern "C" fn op_text_caret_rect(
    engine: *mut crate::OpEngine,
    out: *mut f32,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if out.is_null() {
                return Err(FfiError::invalid("caret-rect output pointer is null"));
            }
            let Some(rect) = session.caret_rect() else {
                return Err(FfiError::new(OpStatus::NoFocus, "no text edit in progress"));
            };
            out.write(rect.0);
            out.add(1).write(rect.1);
            out.add(2).write(rect.2);
            out.add(3).write(rect.3);
            Ok(())
        })
    }
}

/// Build the painter's inline-edit descriptor for the active session
/// (draft text + composition + caret), or `None` when not editing.
pub(crate) fn paint_edit_caret(session: &Session) -> Option<EditCaret> {
    let editing = session.state.ui.text_editing.as_ref()?;
    Some(EditCaret {
        editing: editing.as_str().to_owned(),
        input: session.state.ui.text_edit_input.clone(),
        now_ms: session.now_ms,
        selection_color: Color::rgb_u8(79, 70, 229),
    })
}

/// Compute the caret rect in logical viewport coordinates (see
/// [`op_text_caret_rect`]). `None` when no edit session is active or the
/// edited node vanished from the scene.
pub(crate) fn caret_rect(session: &mut Session) -> Option<(f32, f32, f32, f32)> {
    // Split borrows: `state`/`scene`/viewport read-only, `backend` mutable
    // (the measure facade).
    let (safe_left, safe_top) = (session.insets.left, session.insets.top);
    let Session {
        state,
        scene,
        backend,
        viewport_origin,
        zoom,
        ..
    } = session;
    let editing = state.ui.text_editing.as_ref()?;
    let input = &state.ui.text_edit_input;
    let page = scene.active_page()?;
    let node = crate::render::find_node(&page.children, editing.as_str())?;
    let base = node.text.as_deref().unwrap_or("");
    // Composition-aware caret offset — mirrors the canvas painter's
    // `composition_range` math exactly.
    let caret_byte = if let Some(composition) = input.composition() {
        let insert_at = prev_char_boundary(base, input.caret().min(base.len()));
        let cursor = prev_char_boundary(&composition.text, composition.cursor);
        insert_at + cursor
    } else {
        input.caret().min(base.len())
    };
    let mut measure = crate::measure::MeasureOnly { inner: backend };
    let layout = text_edit_layout(&mut measure, node);
    let (doc_x, doc_y) = layout.caret_position(&mut measure, caret_byte);
    let zoom = *zoom;
    // The document camera is safe-area-local, while the public ABI returns
    // surface-logical geometry for platform candidate-window placement.
    let x = safe_left + viewport_origin.x + doc_x * zoom;
    let y = safe_top + viewport_origin.y + doc_y * zoom;
    let h = layout.line_h * zoom;
    let w = (1.0 / zoom).max(1.0);
    Some((x, y, w, h))
}
