//! The timeline header's screen dropdown: which top-level nodes it lists
//! and what picking one does. A pick holds until the selection moves to
//! another screen (see `motion_timeline_scope`).

use crate::motion_timeline_transitions::animates;
use crate::pen_node_ext::PenNodeExt;
use crate::EditorState;
use jian_ops_schema::node::PenNode;

/// One entry of the screen dropdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotionScreen {
    pub id: String,
    pub label: String,
    /// Something under it animates.
    pub animates: bool,
}

impl EditorState {
    /// The screens the dropdown lists, in page order: the page's top-level
    /// frames, plus any other top-level node that animates.
    pub fn motion_timeline_screens(&self) -> Vec<MotionScreen> {
        self.active_children()
            .iter()
            .filter(|root| matches!(root, PenNode::Frame(_)) || animates(root))
            .map(|root| MotionScreen {
                id: root.id_str().to_owned(),
                label: motion_node_label(root),
                animates: animates(root),
            })
            .collect()
    }

    pub fn toggle_motion_timeline_scope_menu(&mut self) {
        let timeline = &mut self.editor_ui.motion_timeline;
        timeline.scope_menu_open = !timeline.scope_menu_open;
    }

    /// Project the screen `id`: the timeline shows its first tab with
    /// motion, rewound, and keeps that screen until the selection moves to
    /// another one.
    pub fn pick_motion_timeline_scope(&mut self, id: &str) {
        let anchor = self.selection.anchor.clone();
        let timeline = &mut self.editor_ui.motion_timeline;
        timeline.scope_menu_open = false;
        timeline.scope_root = Some(id.to_owned());
        timeline.scope_anchor = anchor.is_real().then(|| anchor.as_str().to_owned());
        let (tab, manual_id) = (timeline.tab, timeline.manual_id.clone());
        timeline.show(tab, manual_id);
        if self.motion_timeline_projection().strips.is_empty() {
            if let Some((tab, manual_id)) = self.first_motion_timeline_tab() {
                self.editor_ui.motion_timeline.show(tab, manual_id);
            }
        }
    }
}

/// A node's name, or its id when it has none.
pub fn motion_node_label(node: &PenNode) -> String {
    node.base()
        .name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| node.id_str().to_owned())
}

#[cfg(test)]
#[path = "motion_timeline_scope_tests.rs"]
mod tests;
