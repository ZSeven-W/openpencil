use super::WidgetHost;

impl WidgetHost {
    pub(crate) fn apply_text_edit_caret(&mut self, forward: bool) -> bool {
        if self.editor_state.ui.text_editing.is_none() {
            return false;
        }
        if self
            .editor_state
            .text_edit_caret_horizontal(forward, self.shift_held, self.now_ms)
        {
            self.mark_dirty();
        }
        true
    }

    pub(crate) fn apply_text_edit_vertical(&mut self, down: bool) -> bool {
        if self.editor_state.ui.text_editing.is_none() {
            return false;
        }
        let ranges = hard_line_ranges(self.editor_state.ui.text_edit_input.text());
        if self
            .editor_state
            .text_edit_caret_vertical(down, self.shift_held, &ranges, self.now_ms)
        {
            self.mark_dirty();
        }
        true
    }

    pub(crate) fn apply_text_edit_line_edge(&mut self, forward: bool) -> bool {
        if self.editor_state.ui.text_editing.is_none() {
            return false;
        }
        let ranges = hard_line_ranges(self.editor_state.ui.text_edit_input.text());
        if self
            .editor_state
            .text_edit_line_edge(forward, self.shift_held, &ranges, self.now_ms)
        {
            self.mark_dirty();
        }
        true
    }
}

fn hard_line_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = 0;
    for (idx, ch) in text.char_indices() {
        if ch == '\n' {
            ranges.push((start, idx));
            start = idx + ch.len_utf8();
        }
    }
    ranges.push((start, text.len()));
    ranges
}
