//! Caret movement + numeric stepping on `WidgetHostNative`: arrow-key
//! nudge, property step, and the per-surface caret handlers.
//!
//! Split out of `keyboard.rs` to keep every file under the repo's
//! 800-line cap.

use super::WidgetHostNative;
use op_editor_core::host_keyboard_transitions as shared;
use op_editor_core::host_preset_name_draft as preset_name;

impl WidgetHostNative {
    /// Up / Down arrow on a focused numeric property input — steps
    /// the value by `delta` and commits it (like a `−` / `+`
    /// stepper). Returns `false` when no numeric property input is
    /// focused, so the caller falls back to nudging the selection.
    pub fn apply_property_step(&mut self, delta: f32) -> bool {
        // Effect-parameter focus: step the value, commit via
        // `SetEffectParam`, and reflect it back into the draft.
        if let Some(ef) = self.editor_state.editor_ui.effect_param_focus {
            if !self.collab_allows_document_mutation(
                op_editor_core::CollabDocumentMutation::Unsupported(
                    op_editor_core::CollabUnsupportedFeature::Effects,
                ),
            ) {
                let _ = op_editor_ui::widgets::property_panel_commit::discard_effect_param_focus(
                    &mut self.editor_state,
                );
                self.mark_dirty();
                return true;
            }
            let current: f32 = self
                .editor_state
                .ui
                .property_input
                .text()
                .trim()
                .parse()
                .unwrap_or(0.0);
            let next = current + delta;
            let id = self.editor_state.selection.anchor.clone();
            if id.is_real() {
                self.editor_state.commit_history();
                let _ = self
                    .editor_state
                    .apply(op_editor_core::EditorCommand::SetEffectParam {
                        node_id: id,
                        index: ef.effect as u32,
                        field: ef.field,
                        value: next,
                    });
            }
            let next_text = if next.fract() == 0.0 {
                format!("{}", next as i64)
            } else {
                format!("{next}")
            };
            self.editor_state
                .ui
                .property_input
                .set_text(next_text.clone());
            self.editor_state.ui.property_input.touch(self.now_ms);
            self.editor_state.ui.property_input_draft = next_text;
            self.editor_state.ui.property_caret_pos = self.editor_state.ui.property_input.caret();
            self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
            self.mark_dirty();
            return true;
        }
        let Some(focus) = self.editor_state.ui.property_focus else {
            return false;
        };
        // Hex colour fields aren't numerically steppable.
        if focus.is_hex() {
            return false;
        }
        if !self.collab_allows_document_mutation(focus.collab_document_mutation()) {
            let _ = op_editor_ui::widgets::property_panel_commit::discard_property_focus(
                &mut self.editor_state,
            );
            self.mark_dirty();
            return true;
        }
        let current: f32 = self
            .editor_state
            .ui
            .property_input
            .text()
            .trim()
            .parse()
            .unwrap_or(0.0);
        let next = current + delta;
        // Instance-write redirect (GAP #10) — see property_dispatch
        // for the choke-point note.
        let instance_scope = self.editor_state.begin_instance_write_for_anchor();
        let _ = self.editor_state.commit_property_edit(focus, next);
        if let Some(scope) = instance_scope {
            self.editor_state.finish_instance_write(scope);
        }
        // Reflect the committed value back into the draft so the
        // field shows it and a further step builds on the new value.
        let next_text = if next.fract() == 0.0 {
            format!("{}", next as i64)
        } else {
            format!("{next}")
        };
        self.editor_state
            .ui
            .property_input
            .set_text(next_text.clone());
        self.editor_state.ui.property_input.touch(self.now_ms);
        self.editor_state.ui.property_input_draft = next_text;
        self.editor_state.ui.property_caret_pos = self.editor_state.ui.property_input.caret();
        self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
        self.mark_dirty();
        true
    }

    /// Left / Right arrow during an inline rename — moves the rename
    /// caret one character. Returns `false` when no rename is active,
    /// so the caller falls back to the property caret / node-nudge.
    pub fn apply_rename_caret(&mut self, forward: bool) -> bool {
        let moved = shared::rename_caret(&mut self.editor_state, forward, self.now_ms);
        if moved {
            self.mark_dirty();
        }
        moved
    }

    /// Left / Right arrow on the focused chat input. Consumes the key
    /// even at text boundaries so it never falls through to canvas nudge.
    pub fn apply_chat_input_caret(&mut self, forward: bool) -> bool {
        if shared::chat_input_caret(&mut self.editor_state, forward, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Left / Right arrow on a focused property input — moves the
    /// text caret one character. Returns `false` when no property
    /// input is focused, so the caller falls back to node-nudge.
    pub fn apply_property_caret(&mut self, forward: bool) -> bool {
        if shared::property_caret_move(&mut self.editor_state, forward, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        // #20: the preset-name input rides the flat legacy draft, not a
        // `TextInputState`, so it has its own caret module. Consumed
        // even when the caret can't move — an arrow over a focused
        // input must never fall through to nudging the selected node.
        if let Some(moved) =
            preset_name::preset_name_caret_move(&mut self.editor_state, forward, self.now_ms)
        {
            if moved {
                self.mark_dirty();
            }
            return true;
        }
        false
    }

    /// Arrow-key nudge — translate selection by (dx, dy) doc px.
    pub fn apply_nudge(&mut self, dx: f32, dy: f32) -> bool {
        if self.input_active() {
            return false;
        }
        if !self.collab_allows_document_mutation(op_editor_core::CollabDocumentMutation::NodeMove) {
            return true;
        }
        if shared::nudge_selection(&mut self.editor_state, dx, dy) {
            self.mark_dirty();
            return true;
        }
        false
    }
}
