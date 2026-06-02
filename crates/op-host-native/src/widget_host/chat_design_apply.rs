use super::WidgetHostNative;
use op_editor_core::{EditorCommand, NodeId};

impl WidgetHostNative {
    pub(in crate::widget_host) fn apply_chat_design_block(
        &mut self,
        message_index: usize,
        code: &str,
    ) -> bool {
        let Ok(nodes) = op_editor_ui::widgets::parse_design_json_nodes(code) else {
            return true;
        };
        if self.editor_state.apply(EditorCommand::InsertSubtree {
            nodes,
            parent_id: NodeId::NONE,
            page_id: None,
        }) {
            self.editor_state
                .chat
                .mark_message_design_applied(message_index);
            self.mark_dirty();
        }
        true
    }
}
