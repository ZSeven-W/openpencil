use crate::chat::ChatState;

impl ChatState {
    /// Mark an assistant design JSON block as already applied, matching
    /// the TS chat marker that hides the apply action on re-render.
    pub fn mark_message_design_applied(&mut self, message_index: usize) -> bool {
        let Some(message) = self.messages.get_mut(message_index) else {
            return false;
        };
        if message.content.contains("<!-- APPLIED -->") || message.content.contains('\u{2705}') {
            return false;
        }
        message.content.push_str("\n\n<!-- APPLIED -->");
        true
    }
}
