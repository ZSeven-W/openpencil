//! Timeline → Inspector. Pressing a lane selects its node, so the
//! Inspector shows what the timeline holds.

use crate::{EditorState, NodeId};

impl EditorState {
    /// Select the node a timeline lane belongs to; the Inspector follows.
    pub fn select_motion_lane(&mut self, node_id: &str) {
        if self.selection.anchor.as_str() != node_id {
            self.set_single_selection(NodeId::new(node_id));
        }
    }
}
