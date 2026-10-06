//! Shared availability for explicit undo/redo in the normal work view.

use crate::{EditorState, PenNodeExt, WorkspacePhase};

#[derive(Debug, PartialEq)]
pub struct WorkspaceGeometry {
    page: usize,
    boards: Vec<BoardGeometry>,
}

#[derive(Debug, PartialEq)]
struct BoardGeometry {
    id: String,
    x: Option<f64>,
    y: Option<f64>,
    width: Option<f64>,
    height: Option<f64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WorkspaceHistory {
    pub undo: bool,
    pub redo: bool,
}

impl WorkspaceHistory {
    /// History restores document geometry but intentionally leaves the camera
    /// alone. Hosts refit only when board identities/order or sizes changed.
    pub fn geometry(state: &EditorState) -> WorkspaceGeometry {
        WorkspaceGeometry {
            page: state.ui.active_page_index,
            boards: state
                .active_children()
                .iter()
                .filter(|node| matches!(node, jian_ops_schema::node::PenNode::Frame(_)))
                .map(|node| BoardGeometry {
                    id: node.id_str().to_string(),
                    x: node.base().x,
                    y: node.base().y,
                    width: node.width_px(),
                    height: node.height_px(),
                })
                .collect(),
        }
    }

    /// A queued or streaming turn can still write into the document. History
    /// controls wait for it to settle, including the gap before host launch.
    pub fn for_editor(state: &EditorState) -> Self {
        if state.editor_ui.workspace.phase == WorkspacePhase::Generating
            || crate::workspace_run::awaiting_launch(state)
            || crate::workspace_run::assistant_streaming(state)
            || state.chat.agents_running.0 > 0
        {
            return Self::default();
        }
        Self {
            undo: state.history.can_undo(),
            redo: state.history.can_redo(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChatMessage, EditorCommand, HomeFamily, NodeId};

    fn work() -> EditorState {
        let doc = jian_ops_schema::load_str(r#"{"version":"1.0.0","children":[{"type":"frame","id":"work","width":400,"height":600,"children":[] }]}"#).unwrap().value;
        let mut state = EditorState::from_document(doc);
        state
            .editor_ui
            .workspace
            .open_for_reading(HomeFamily::EventPoster, 1);
        state.with_history_group(|state| {
            assert!(state.apply(EditorCommand::SetNodeName {
                node_id: NodeId::new("work"),
                name: "Edited".into()
            }));
        });
        state
    }

    #[test]
    fn recovery_waits_for_generation_launch_stream_and_agents() {
        let mut state = work();
        assert!(WorkspaceHistory::for_editor(&state).undo);
        state.editor_ui.workspace.phase = WorkspacePhase::Generating;
        assert_eq!(
            WorkspaceHistory::for_editor(&state),
            WorkspaceHistory::default()
        );
        state.editor_ui.workspace.phase = WorkspacePhase::Stopped;
        state.chat.pending_send = Some("edit".into());
        assert!(!WorkspaceHistory::for_editor(&state).undo);
        state.chat.pending_send = None;
        let mut message = ChatMessage::assistant("");
        message.streaming = true;
        state.chat.messages.push(message);
        assert!(!WorkspaceHistory::for_editor(&state).undo);
        state.chat.messages[0].streaming = false;
        state.chat.agents_running = (1, 1);
        assert!(!WorkspaceHistory::for_editor(&state).undo);
        state.chat.agents_running = (0, 1);
        assert!(WorkspaceHistory::for_editor(&state).undo);
        assert!(state.undo());
        assert!(WorkspaceHistory::for_editor(&state).redo);
    }

    #[test]
    fn grouping_many_writes_at_the_cap_preserves_earlier_history_and_redo() {
        let mut state = work();
        for i in 0..crate::history::HISTORY_CAP {
            state.commit_history();
            assert!(state.apply(EditorCommand::SetNodeName {
                node_id: NodeId::new("work"),
                name: format!("Earlier {i}")
            }));
        }
        let before = state.doc.clone();
        let earliest_retained = state.history.past[1].clone();
        state.with_history_group(|state| {
            for i in 0..110 {
                state.commit_history();
                assert!(state.apply(EditorCommand::SetNodeName {
                    node_id: NodeId::new("work"),
                    name: format!("Grouped {i}")
                }));
            }
        });
        let after = state.doc.clone();
        assert_eq!(state.history.past.len(), crate::history::HISTORY_CAP);
        assert_eq!(state.history.past[0], earliest_retained);
        assert!(state.undo());
        assert_eq!(state.doc, before);
        let past = state.history.past.clone();
        let future = state.history.future.clone();
        state.with_history_group(|_| {});
        assert_eq!(state.history.past, past);
        assert_eq!(state.history.future, future);
        assert!(state.redo());
        assert_eq!(state.doc, after);
    }

    #[test]
    fn a_rolled_back_batch_inside_a_group_preserves_redo() {
        let mut state = work();
        assert!(state.undo());
        let before = state.doc.clone();
        let future = state.history.future.clone();
        assert!(
            !state.with_history_group(|state| state.apply(EditorCommand::Batch {
                commands: vec![
                    EditorCommand::SetNodeName {
                        node_id: NodeId::new("work"),
                        name: "Will roll back".into()
                    },
                    EditorCommand::SetNodeName {
                        node_id: NodeId::new("missing"),
                        name: "Rejected".into()
                    },
                ],
            }))
        );
        assert_eq!(state.doc, before);
        assert!(state.history.past.is_empty());
        assert_eq!(state.history.future, future);
    }
}
