use super::WidgetHostNative;

impl WidgetHostNative {
    /// Cmd-D — duplicate selection as a sibling at +10 doc px.
    pub fn apply_duplicate(&mut self) -> bool {
        if self.input_active() {
            return false;
        }
        if self.editor_state.selection.is_empty() {
            return false;
        }
        self.editor_state.commit_history();
        let dup = self
            .editor_state
            .duplicate_selected(&mut self.next_node_id, 10.0)
            .is_some();
        if dup {
            self.mark_dirty();
        }
        dup
    }

    /// Up / Down arrow on a focused numeric property input — steps
    /// the value by `delta` and commits it.
    pub fn apply_property_step(&mut self, delta: f32) -> bool {
        if let Some(ef) = self.editor_state.editor_ui.effect_param_focus {
            let current: f32 = self
                .editor_state
                .ui
                .property_input_draft
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
            self.seed_property_step_draft(next);
            return true;
        }
        let Some(focus) = self.editor_state.ui.property_focus else {
            return false;
        };
        if focus.is_hex() {
            return false;
        }
        let current: f32 = self
            .editor_state
            .ui
            .property_input_draft
            .trim()
            .parse()
            .unwrap_or(0.0);
        let next = current + delta;
        let _ = self.editor_state.commit_property_edit(focus, next);
        self.seed_property_step_draft(next);
        true
    }

    /// Left / Right arrow on a focused property input.
    pub fn apply_property_caret(&mut self, forward: bool) -> bool {
        if self.editor_state.ui.property_focus.is_none()
            && self.editor_state.editor_ui.effect_param_focus.is_none()
            && self
                .editor_state
                .editor_ui
                .variables_theme_rename_axis
                .is_none()
            && self
                .editor_state
                .editor_ui
                .variables_variant_rename_value
                .is_none()
            && self.editor_state.editor_ui.variable_row_focus.is_none()
        {
            return false;
        }
        let draft = &self.editor_state.ui.property_input_draft;
        let pos = text_boundary_at_or_before(draft, self.editor_state.ui.property_caret_pos);
        let next = if forward {
            next_text_boundary(draft, pos)
        } else {
            previous_text_boundary(draft, pos)
        };
        if next != self.editor_state.ui.property_caret_pos {
            self.editor_state.ui.property_caret_pos = next;
            self.editor_state.ui.property_draft_select_all = false;
            self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
            self.mark_dirty();
        }
        true
    }

    /// Arrow-key nudge — translate selection by (dx, dy) doc px.
    pub fn apply_nudge(&mut self, dx: f32, dy: f32) -> bool {
        if self.input_active() {
            return false;
        }
        if self.editor_state.selection.is_empty() {
            return false;
        }
        self.editor_state.commit_history();
        self.editor_state.translate_selected(dx as f64, dy as f64);
        self.mark_dirty();
        true
    }

    fn seed_property_step_draft(&mut self, value: f32) {
        self.editor_state.ui.property_input_draft = if value.fract() == 0.0 {
            format!("{}", value as i64)
        } else {
            format!("{value}")
        };
        self.editor_state.ui.property_caret_pos = self.editor_state.ui.property_input_draft.len();
        self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
        self.mark_dirty();
    }
}

fn text_boundary_at_or_before(value: &str, pos: usize) -> usize {
    let mut clipped = pos.min(value.len());
    while clipped > 0 && !value.is_char_boundary(clipped) {
        clipped -= 1;
    }
    clipped
}

fn previous_text_boundary(value: &str, pos: usize) -> usize {
    let pos = text_boundary_at_or_before(value, pos);
    value[..pos]
        .char_indices()
        .last()
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}

fn next_text_boundary(value: &str, pos: usize) -> usize {
    let pos = text_boundary_at_or_before(value, pos);
    if pos >= value.len() {
        return value.len();
    }
    pos + value[pos..].chars().next().map(char::len_utf8).unwrap_or(0)
}
