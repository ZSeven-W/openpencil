//! Keyboard-driven editor-state transitions shared by the native and
//! web widget hosts (`widget_host/keyboard*.rs` / `shortcuts.rs`).
//! Both hosts used to carry these as copy-pasted method bodies.
//!
//! Hosts stay thin: they own the genuinely platform-specific glue —
//! modifier sourcing (winit `ModifiersChanged` vs the browser
//! `KeyboardEvent`), clipboard routing, and the per-host
//! "which surface owns the keyboard" predicates — then call these and
//! `mark_dirty()` when a transition reports a change.
//!
//! Return conventions:
//! - `bool` — `true` means "this transition acted" (host marks dirty
//!   and reports the key consumed).
//! - `Option<bool>` — `None` means "this surface is not focused, fall
//!   through to the next router arm"; `Some(false)` means "focused but
//!   the keystroke was rejected" (the host stops routing and reports
//!   not-consumed, matching both hosts' original `return false`).

use jian_core::text_input::TextInputState;

use crate::editor_ui_state::{EditorUiState, VariableRowFocus};
use crate::figma_import_state::ImportSource;
use crate::state::EditorState;
use crate::ui_draft::PropertyFocus;
use crate::walkers::ReorderDirection;
use crate::Tool;

pub use crate::prompt_center_keyboard::{
    backspace as prompt_center_backspace, delete_forward as prompt_center_delete_forward,
    move_caret as prompt_center_caret, select_all as prompt_center_select_all,
    text as prompt_center_text,
};

pub use crate::scene_template_keyboard::{
    backspace as scene_template_backspace, delete_forward as scene_template_delete_forward,
    move_caret as scene_template_caret, paste as scene_template_paste,
    select_all as scene_template_select_all, text as scene_template_text,
};

// ─── Legacy draft mirrors ──────────────────────────────────────────────
//
// The widget layer still paints the caret from the flat
// `ui.property_input_draft` / `property_caret_pos` mirror, so every
// `TextInputState` edit re-publishes itself there.

/// Mirror `ui.property_input` into the flat legacy draft fields.
pub fn mirror_property_input_legacy(state: &mut EditorState, select_all: bool, now_ms: u64) {
    let ui = &mut state.ui;
    ui.property_input_draft = ui.property_input.text().to_owned();
    ui.property_caret_pos = ui.property_input.caret();
    ui.property_draft_select_all = select_all;
    ui.property_caret_anchor_ms = now_ms;
}

/// Mirror `editor_ui.variables_header_input` into the legacy draft.
pub fn mirror_variables_header_input_legacy(
    state: &mut EditorState,
    select_all: bool,
    now_ms: u64,
) {
    let text = state.editor_ui.variables_header_input.text().to_owned();
    let caret = state.editor_ui.variables_header_input.caret();
    let ui = &mut state.ui;
    ui.property_input_draft = text;
    ui.property_caret_pos = caret;
    ui.property_draft_select_all = select_all;
    ui.property_caret_anchor_ms = now_ms;
}

/// Mirror `editor_ui.variable_row_input` into the legacy draft.
pub fn mirror_variable_row_input_legacy(state: &mut EditorState, select_all: bool, now_ms: u64) {
    let text = state.editor_ui.variable_row_input.text().to_owned();
    let caret = state.editor_ui.variable_row_input.caret();
    let ui = &mut state.ui;
    ui.property_input_draft = text;
    ui.property_caret_pos = caret;
    ui.property_draft_select_all = select_all;
    ui.property_caret_anchor_ms = now_ms;
}

// ─── Focus predicates ──────────────────────────────────────────────────

impl EditorUiState {
    /// Whether the visible collaboration Join field owns the keyboard.
    ///
    /// The focus bit alone is insufficient: an async phase transition or an
    /// externally dismissed panel can leave it stale. Hosts use this
    /// visibility-aware predicate for shortcuts, clipboard, and IME focus.
    pub fn collab_join_input_active(&self) -> bool {
        self.collab.availability == crate::CollabAvailability::Ready
            && matches!(
                self.collab.phase,
                crate::CollabConnectionPhase::Idle | crate::CollabConnectionPhase::Discovering
            )
            && self.collab.panel.open
            && self.collab.panel.view == crate::CollabPanelView::Join
            && self.collab.panel.join_address_focused
    }

    /// Drop even a stale Join-field focus bit when another surface takes
    /// over. The input's selection collapses with it so a later refocus
    /// never resurrects a destructive replace-on-type state.
    pub fn blur_collab_join_input(&mut self) -> bool {
        let input = &mut self.collab.panel.join_input;
        let end = input.text().len();
        input.set_caret(end, 0);
        std::mem::take(&mut self.collab.panel.join_address_focused)
    }

    /// Whether a variables-panel theme-axis / variant header rename
    /// draft owns the keyboard.
    pub fn variables_header_rename_active(&self) -> bool {
        self.variables_theme_rename_axis.is_some() || self.variables_variant_rename_value.is_some()
    }

    /// Whether the variables-panel search input owns the keyboard.
    /// Gated on the panel being open so a stale focus flag can't eat
    /// keystrokes after the panel closes.
    pub fn variables_search_input_active(&self) -> bool {
        self.variables_panel_open && self.variables_search_focus
    }

    /// Blur the variables-panel search box, KEEPING the typed filter —
    /// clearing it would surprise mid-search. Both the Escape rung and
    /// Enter run this same transition.
    pub fn blur_variables_search(&mut self) -> bool {
        if !self.variables_search_input_active() {
            return false;
        }
        self.variables_search_focus = false;
        true
    }
}

// ─── Inline layer rename ───────────────────────────────────────────────

/// Append `c` to the inline layer-rename draft. `None` when no rename
/// is active or `c` is a control char, so the caller falls through to
/// the next router arm.
pub fn rename_text(state: &mut EditorState, c: char, now_ms: u64) -> Option<bool> {
    if state.ui.layer_rename.is_none() || c.is_control() {
        return None;
    }
    let mut buf = [0u8; 4];
    let ok = state.rename_append(c.encode_utf8(&mut buf));
    if ok {
        if let Some(rename) = state.ui.layer_rename.as_mut() {
            rename.input.touch(now_ms);
        }
    }
    Some(ok)
}

/// Backspace (and forward-Delete — the rename draft has no separate
/// forward deletion) in the inline layer-rename draft.
pub fn rename_backspace(state: &mut EditorState, now_ms: u64) -> Option<bool> {
    state.ui.layer_rename.as_ref()?;
    let ok = state.rename_backspace();
    if ok {
        if let Some(rename) = state.ui.layer_rename.as_mut() {
            rename.input.touch(now_ms);
        }
    }
    Some(ok)
}

// ─── Canvas text editing ───────────────────────────────────────────────

/// Insert `c` into the canvas text-edit session.
pub fn text_edit_text(state: &mut EditorState, c: char, now_ms: u64) -> Option<bool> {
    if state.ui.text_editing.is_none() || c.is_control() {
        return None;
    }
    let mut buf = [0u8; 4];
    Some(state.text_edit_insert(c.encode_utf8(&mut buf), now_ms))
}

/// Backspace in the canvas text-edit session.
pub fn text_edit_backspace(state: &mut EditorState, now_ms: u64) -> Option<bool> {
    state.ui.text_editing.as_ref()?;
    Some(state.text_edit_backspace(now_ms))
}

/// Forward-delete in the canvas text-edit session — Delete deletes AT
/// the caret (or removes the active selection), textarea parity.
pub fn text_edit_delete_forward(state: &mut EditorState, now_ms: u64) -> Option<bool> {
    state.ui.text_editing.as_ref()?;
    Some(state.text_edit_delete_forward(now_ms))
}

// ─── Variables-panel search filter ─────────────────────────────────────
//
// A live-filter box with no draft / commit machinery (the TS side is a
// controlled `<input>`), so it appends and pops straight onto
// `editor_ui.variables_search`.

/// Append `c` to the variables-panel search filter.
pub fn variables_search_text(state: &mut EditorState, c: char, now_ms: u64) -> bool {
    if !state.editor_ui.variables_search_input_active() || c.is_control() {
        return false;
    }
    state.editor_ui.variables_search.push(c);
    state.ui.property_caret_anchor_ms = now_ms;
    // A narrower list invalidates the scroll offset — the widget
    // clamps, but reset for a stable reveal-from-top.
    state.editor_ui.variables_scroll.offset = 0.0;
    true
}

/// Pop the last char off the variables-panel search filter.
/// `Some(false)` on an empty filter: the box still owns the key, so the
/// host must not fall through to deleting the selected node.
pub fn variables_search_backspace(state: &mut EditorState, now_ms: u64) -> Option<bool> {
    if !state.editor_ui.variables_search_input_active() {
        return None;
    }
    if state.editor_ui.variables_search.pop().is_none() {
        return Some(false);
    }
    state.ui.property_caret_anchor_ms = now_ms;
    state.editor_ui.variables_scroll.offset = 0.0;
    Some(true)
}

// ─── Chat composer ─────────────────────────────────────────────────────

/// Insert `c` into the focused chat input. `false` when the composer
/// is not focused or `c` is a control character.
pub fn chat_input_text(state: &mut EditorState, c: char, now_ms: u64) -> bool {
    if !state.chat.focused || c.is_control() {
        return false;
    }
    let mut buf = [0u8; 4];
    state
        .chat
        .insert_input_text(c.encode_utf8(&mut buf), now_ms);
    true
}

/// Backspace in the focused chat input.
pub fn chat_input_backspace(state: &mut EditorState, now_ms: u64) -> Option<bool> {
    if !state.chat.focused {
        return None;
    }
    Some(state.chat.backspace_input(now_ms))
}

// ─── Per-focus character gates ─────────────────────────────────────────

/// Numeric-draft gate (effect-parameter value boxes and any other
/// plain numeric field): digits, a leading `-`, and a single `.`.
pub fn numeric_draft_accepts(input: &TextInputState, c: char) -> bool {
    let replacing_all = input.is_select_all();
    let draft = input.text();
    let pos = if replacing_all {
        0
    } else {
        input.caret().min(draft.len())
    };
    c.is_ascii_digit()
        || (c == '-' && pos == 0 && (replacing_all || !draft.starts_with('-')))
        || (c == '.' && (replacing_all || !draft.contains('.')))
}

/// Property-panel input gate. Free-text rows take any printable char;
/// hex rows cap at the focus's `#RRGGBB(AA)` width with a sticky `#`
/// prefix; everything else is numeric.
///
/// Caret byte-index is also the char index (drafts are ASCII). `-` /
/// `#` are gated on the caret being at the start, NOT on the draft
/// being empty: typing `-` at the head of an existing `40` is a valid
/// edit (`-40`).
pub fn property_focus_accepts(focus: PropertyFocus, input: &TextInputState, c: char) -> bool {
    let replacing_all = input.is_select_all();
    let draft = input.text();
    let pos = if replacing_all {
        0
    } else {
        input.caret().min(draft.len())
    };
    if focus.is_free_text() {
        // Widget text rows (placeholder / value / label / icon names /
        // bind key) take any non-control character.
        !c.is_control()
    } else if focus.is_hex() {
        // Most colour rows cap at `#RRGGBB`; fill and page background
        // rows additionally author `#RRGGBBAA`. Keep that exception on
        // the focus type so adding it cannot silently widen fill /
        // stroke inputs.
        (replacing_all || draft.len() < focus.hex_max_len().unwrap_or(7))
            && (c.is_ascii_hexdigit() || (c == '#' && pos == 0 && !draft.starts_with('#')))
    } else {
        c.is_ascii_digit()
            || (c == '-' && pos == 0 && (replacing_all || !draft.starts_with('-')))
            || (c == '.' && focus.accepts_decimal() && (replacing_all || !draft.contains('.')))
    }
}

/// Variables-panel row / cell draft gate — per-kind (free text,
/// numeric, inline colour hex).
pub fn variable_row_accepts(focus: VariableRowFocus, input: &TextInputState, c: char) -> bool {
    let replacing_all = input.is_select_all();
    let draft = input.text();
    let pos = if replacing_all {
        0
    } else {
        input.caret().min(draft.len())
    };
    match focus {
        VariableRowFocus::Name(_) => !c.is_control(),
        VariableRowFocus::Number(_) | VariableRowFocus::NumberCell { .. } => {
            c.is_ascii_digit()
                || (c == '-' && (replacing_all || (pos == 0 && !draft.starts_with('-'))))
                || (c == '.' && (replacing_all || !draft.contains('.')))
        }
        VariableRowFocus::String(_) | VariableRowFocus::StringCell { .. } => !c.is_control(),
        // Inline colour hex — `#` only at the front, hex digits after,
        // capped at `#rrggbb` (same gating as the property panel's
        // FillHex draft).
        VariableRowFocus::ColorCell { .. } => {
            let len_after_clear = if replacing_all { 0 } else { draft.len() };
            if c == '#' {
                len_after_clear == 0
            } else {
                c.is_ascii_hexdigit() && len_after_clear < 7
            }
        }
    }
}

// ─── Typed-character routing ───────────────────────────────────────────

/// Insert `c` into the focused property-panel / effect-parameter input.
///
/// `None` when neither focus is set. When both are set the property
/// focus wins — an unreachable combination today (the two focuses are
/// set exclusively), kept explicit so the routing order is not
/// accidentally load-bearing.
pub fn property_input_text(state: &mut EditorState, c: char, now_ms: u64) -> Option<bool> {
    let focus = state.ui.property_focus;
    if focus.is_none() && state.editor_ui.effect_param_focus.is_none() {
        return None;
    }
    if c.is_control() {
        return Some(false);
    }
    let allowed = match focus {
        Some(focus) => property_focus_accepts(focus, &state.ui.property_input, c),
        None => numeric_draft_accepts(&state.ui.property_input, c),
    };
    if !allowed {
        return Some(false);
    }
    let mut buf = [0u8; 4];
    state
        .ui
        .property_input
        .insert_str(c.encode_utf8(&mut buf), now_ms);
    mirror_property_input_legacy(state, false, now_ms);
    Some(true)
}

/// Insert `c` into the focused variables-panel row / cell draft.
/// `None` when no row is focused.
pub fn variable_row_text(state: &mut EditorState, c: char, now_ms: u64) -> Option<bool> {
    let focus = state.editor_ui.variable_row_focus?;
    if !variable_row_accepts(focus, &state.editor_ui.variable_row_input, c) {
        return Some(false);
    }
    let mut buf = [0u8; 4];
    state
        .editor_ui
        .variable_row_input
        .insert_str(c.encode_utf8(&mut buf), now_ms);
    mirror_variable_row_input_legacy(state, false, now_ms);
    Some(true)
}

/// Insert `c` into the variables-panel theme/variant header rename
/// draft. `false` (fall through) when no rename is active or `c` is a
/// control character.
pub fn variables_header_text(state: &mut EditorState, c: char, now_ms: u64) -> bool {
    if !state.editor_ui.variables_header_rename_active() || c.is_control() {
        return false;
    }
    let mut buf = [0u8; 4];
    state
        .editor_ui
        .variables_header_input
        .insert_str(c.encode_utf8(&mut buf), now_ms);
    mirror_variables_header_input_legacy(state, false, now_ms);
    true
}

// ─── Backspace / forward-delete ────────────────────────────────────────
//
// Each helper compares (text, caret) before and after so a no-op at a
// text boundary reports `false` and the caller can fall through.

macro_rules! edited {
    ($input:expr, $op:expr) => {{
        let before = ($input.text().to_owned(), $input.caret());
        $op;
        (before.0.as_str(), before.1) != ($input.text(), $input.caret())
    }};
}

/// Backspace in the focused property-panel / effect-parameter input.
pub fn property_input_backspace(state: &mut EditorState, now_ms: u64) -> bool {
    let changed = edited!(
        state.ui.property_input,
        state.ui.property_input.backspace(now_ms)
    );
    if changed {
        mirror_property_input_legacy(state, false, now_ms);
    }
    changed
}

/// Forward-delete in the focused property-panel / effect-parameter input.
pub fn property_input_delete_forward(state: &mut EditorState, now_ms: u64) -> bool {
    let changed = edited!(
        state.ui.property_input,
        state.ui.property_input.delete_forward(now_ms)
    );
    if changed {
        mirror_property_input_legacy(state, false, now_ms);
    }
    changed
}

/// Backspace in the variables header rename draft.
pub fn variables_header_backspace(state: &mut EditorState, now_ms: u64) -> bool {
    let changed = edited!(
        state.editor_ui.variables_header_input,
        state.editor_ui.variables_header_input.backspace(now_ms)
    );
    if changed {
        mirror_variables_header_input_legacy(state, false, now_ms);
    }
    changed
}

/// Forward-delete in the variables header rename draft.
pub fn variables_header_delete_forward(state: &mut EditorState, now_ms: u64) -> bool {
    let changed = edited!(
        state.editor_ui.variables_header_input,
        state
            .editor_ui
            .variables_header_input
            .delete_forward(now_ms)
    );
    if changed {
        mirror_variables_header_input_legacy(state, false, now_ms);
    }
    changed
}

/// Backspace in the variables row / cell draft.
pub fn variable_row_backspace(state: &mut EditorState, now_ms: u64) -> bool {
    let changed = edited!(
        state.editor_ui.variable_row_input,
        state.editor_ui.variable_row_input.backspace(now_ms)
    );
    if changed {
        mirror_variable_row_input_legacy(state, false, now_ms);
    }
    changed
}

/// Forward-delete in the variables row / cell draft.
pub fn variable_row_delete_forward(state: &mut EditorState, now_ms: u64) -> bool {
    let changed = edited!(
        state.editor_ui.variable_row_input,
        state.editor_ui.variable_row_input.delete_forward(now_ms)
    );
    if changed {
        mirror_variable_row_input_legacy(state, false, now_ms);
    }
    changed
}

// ─── Caret movement ────────────────────────────────────────────────────

/// Left / Right arrow across the three `TextInputState`-backed chrome
/// drafts, in host priority order: property / effect-param → variables
/// header rename → variables row. `false` when none of them is focused,
/// so the caller can fall through to its own tail (the native host's
/// preset-name draft) or to node-nudge.
pub fn property_caret_move(state: &mut EditorState, forward: bool, now_ms: u64) -> bool {
    if state.ui.property_focus.is_some() || state.editor_ui.effect_param_focus.is_some() {
        if forward {
            state.ui.property_input.move_right(false, now_ms);
        } else {
            state.ui.property_input.move_left(false, now_ms);
        }
        mirror_property_input_legacy(state, false, now_ms);
        return true;
    }
    if state.editor_ui.variables_header_rename_active() {
        if forward {
            state
                .editor_ui
                .variables_header_input
                .move_right(false, now_ms);
        } else {
            state
                .editor_ui
                .variables_header_input
                .move_left(false, now_ms);
        }
        mirror_variables_header_input_legacy(state, false, now_ms);
        return true;
    }
    if state.editor_ui.variable_row_focus.is_some() {
        if forward {
            state.editor_ui.variable_row_input.move_right(false, now_ms);
        } else {
            state.editor_ui.variable_row_input.move_left(false, now_ms);
        }
        mirror_variable_row_input_legacy(state, false, now_ms);
        return true;
    }
    false
}

/// Left / Right arrow during an inline layer rename.
pub fn rename_caret(state: &mut EditorState, forward: bool, now_ms: u64) -> bool {
    let moved = if forward {
        state.rename_caret_right()
    } else {
        state.rename_caret_left()
    };
    if moved {
        if let Some(rename) = state.ui.layer_rename.as_mut() {
            rename.input.touch(now_ms);
        }
    }
    moved
}

/// Left / Right arrow on the focused chat input. Consumes the key even
/// at text boundaries so it never falls through to canvas nudge.
///
/// `extend` grows the selection from its existing anchor (Shift+arrow)
/// instead of collapsing it.
///
/// While an IME composition is live the key is swallowed WITHOUT moving
/// the caret: the platform owns caret motion inside a preedit, and
/// splicing our own move under it would desync the composing region from
/// what the input method believes it is editing.
pub fn chat_input_caret(state: &mut EditorState, forward: bool, extend: bool, now_ms: u64) -> bool {
    if !state.chat.focused {
        return false;
    }
    if state.chat.input.composition().is_some() {
        return true;
    }
    if forward {
        state.chat.input.move_right(extend, now_ms);
    } else {
        state.chat.input.move_left(extend, now_ms);
    }
    true
}

// ─── Select-all across focused inputs ──────────────────────────────────

/// Cmd/Ctrl+A over the focused chrome text input, in host priority
/// order. Callers resolve the surfaces whose predicates are host-side
/// (image popovers, settings modal, Git panel) BEFORE calling this.
pub fn select_all_focused_input(state: &mut EditorState, now_ms: u64) -> bool {
    if prompt_center_select_all(state, now_ms) {
        return true;
    }
    if let Some(rename) = state.ui.layer_rename.as_mut() {
        rename.input.select_all();
        rename.input.touch(now_ms);
        return true;
    }
    if state.ui.text_editing.is_some() {
        let _ = state.text_edit_select_all_now(now_ms);
        return true;
    }
    if state.ui.property_focus.is_some() || state.editor_ui.effect_param_focus.is_some() {
        state.ui.property_input.select_all();
        state.ui.property_input.touch(now_ms);
        mirror_property_input_legacy(state, true, now_ms);
        return true;
    }
    if state.editor_ui.variables_header_rename_active() {
        state.editor_ui.variables_header_input.select_all();
        state.editor_ui.variables_header_input.touch(now_ms);
        mirror_variables_header_input_legacy(state, true, now_ms);
        return true;
    }
    if state.editor_ui.variable_row_focus.is_some() {
        state.editor_ui.variable_row_input.select_all();
        state.editor_ui.variable_row_input.touch(now_ms);
        mirror_variable_row_input_legacy(state, true, now_ms);
        return true;
    }
    if state.editor_ui.icon_picker.open {
        state.editor_ui.icon_picker_select_all = true;
        return true;
    }
    if state.editor_ui.chat_model_picker.open {
        state.editor_ui.chat_model_picker_input.select_all();
        state.editor_ui.chat_model_picker_input.touch(now_ms);
        return true;
    }
    if state.editor_ui.component_browser_open {
        state.editor_ui.component_browser_select_all = true;
        return true;
    }
    if state.chat.focused {
        state.chat.select_all_input(now_ms);
        return true;
    }
    false
}

// ─── Selection edit ops ────────────────────────────────────────────────
//
// The "does a text input own the keyboard?" guard stays host-side —
// each host resolves a different set of surfaces.

/// Whether a chrome text field / search overlay owns forward-Delete.
///
/// Each of these surfaces either handles Delete itself in an earlier
/// router arm or deliberately swallows it; falling through to
/// [`delete_selection_with_history`] behind one of them would silently
/// drop the node under the focused field.
///
/// The two hosts had drifted to *different* subsets of this list —
/// native was missing the variables search box, web was missing the
/// icon picker and the component browser — so each could destroy a
/// node while the other's overlay was up. This is the union.
///
/// Host-resolved surfaces (font picker, image popovers, settings, Git)
/// are checked host-side before this predicate.
pub fn delete_owned_by_chrome_input(state: &EditorState) -> bool {
    state.editor_ui.collab_join_input_active()
        || state.ui.property_focus.is_some()
        || state.editor_ui.effect_param_focus.is_some()
        || state.editor_ui.variable_row_focus.is_some()
        || state.editor_ui.variables_header_rename_active()
        // Both hosts route the preset draft in an earlier arm; listed
        // here so reordering the arms cannot reopen the node-destroying
        // fall-through this predicate exists to stop.
        || state.editor_ui.preset_name_input_active()
        || state.editor_ui.variables_search_input_active()
        || state.editor_ui.icon_picker.open
        || state.editor_ui.prompt_center.open
        || state.editor_ui.chat_model_picker.open
        || state.editor_ui.component_browser_open
        || state.chat.focused
}

/// Delete the selection, pushing the pre-delete snapshot onto history.
pub fn delete_selection_with_history(state: &mut EditorState) -> bool {
    if state.selection.is_empty() {
        return false;
    }
    let snap = state.snapshot_for_history();
    if state.delete_selected() {
        state.history_push_past(snap);
        return true;
    }
    false
}

/// Cmd/Ctrl+D — duplicate the selection as a sibling at +10 doc px.
pub fn duplicate_selection(state: &mut EditorState, next_node_id: &mut u64) -> bool {
    if state.selection.is_empty() {
        return false;
    }
    state.commit_history();
    state.duplicate_selected(next_node_id, 10.0).is_some()
}

/// Arrow-key nudge — layout-aware move first, plain translate second.
pub fn nudge_selection(state: &mut EditorState, dx: f32, dy: f32) -> bool {
    if state.selection.is_empty() {
        return false;
    }
    let snap = state.snapshot_for_history();
    if state.move_selected_in_layout_direction(dx as f64, dy as f64) {
        state.history_push_past(snap);
        return true;
    }
    if state.translate_selected(dx as f64, dy as f64) {
        state.history_push_past(snap);
        return true;
    }
    false
}

/// `[` / `]` — bump the selection within its parent's children.
pub fn reorder_selection(state: &mut EditorState, direction: ReorderDirection) -> bool {
    if state.selection.is_empty() {
        return false;
    }
    state.commit_history();
    state.reorder_selected(direction)
}

/// Cmd/Ctrl+V — paste the clipboard at +10 doc px; selection follows
/// the clones.
pub fn paste_clipboard_at_default_offset(state: &mut EditorState, next_node_id: &mut u64) -> bool {
    if state.clipboard.is_empty() {
        return false;
    }
    state.commit_history();
    !state.paste_clipboard(next_node_id, 10.0).is_empty()
}

/// Single-key tool switch tail: drop the canvas hover outline, install
/// the tool, and sync the toolbar shape slot for shape variants.
///
/// Hosts run their own prologue first — image-crop exit, variable-row
/// commit, and the pen-path discard (TS `onToolChange` resets the pen
/// preview without committing, `skia-pen-tool.ts:38-50`); the native
/// host routes that through its own `cancel_pen_on_tool_switch`
/// helper, which the pen module owns.
pub fn set_active_tool(state: &mut EditorState, tool: Tool) {
    // Leaving Select must drop the hover outline immediately — cursor
    // moves stop updating it for other tools.
    state.editor_ui.canvas_hover_node = None;
    state.tool = tool;
    if matches!(
        tool,
        Tool::Rect | Tool::Ellipse | Tool::Polygon | Tool::Line | Tool::Pen
    ) {
        state.editor_ui.shape_tool = tool;
    }
}

// ─── Import-modal chord (Cmd+Shift+F / Cmd+Shift+H) ────────────────────

/// Overlay/modal state that must keep ownership of the import chord.
/// The Git-panel and image-popover halves of the guard stay host-side
/// (host-resolved visibility predicates).
pub fn import_modal_blocked_by_overlay(ui: &EditorUiState) -> bool {
    ui.figma_import_in_progress
        || !ui.figma_import_pages.is_empty()
        || ui.export_dialog_open
        || ((ui.account_ui_available || ui.touch_chrome()) && ui.login_modal_open)
        || ui.agent_settings_open
        // A stale settings focus still routes keys to the port / API-key
        // field; the chord must not steal them.
        || ui.agent_settings.focus.is_some()
        || (ui.missing_fonts_modal_open
            && ui
                .missing_fonts_prompt
                .as_ref()
                .is_some_and(|prompt| !prompt.entries.is_empty()))
}

/// Commit canvas / layer editing before a full-screen modal covers it.
pub fn commit_editing_for_modal(state: &mut EditorState) {
    let _ = state.rename_commit();
    let _ = state.text_edit_commit();
    state.color_picker_blur_hex();
    state.color_picker_blur_rgb();
    let _ = state.close_color_picker();
}

/// Close every chrome overlay the import modal covers, then open it.
pub fn open_import_modal(ui: &mut EditorUiState, source: ImportSource) {
    ui.close_font_picker();
    ui.close_icon_picker();
    ui.component_browser_open = false;
    ui.component_browser_kit_picker_open = false;
    ui.component_browser_confirm_delete_kit = None;
    ui.component_browser_hover = None;
    ui.ime_preedit = None;
    ui.import_source = source;
    ui.figma_import_open = true;
    ui.figma_import_hover = None;
}
