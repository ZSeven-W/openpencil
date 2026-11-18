//! Capture a desktop audio action before a picker or worker changes selection.

use crate::pen_node_ext::PenNodeExt;
use crate::walkers::find_node;
use crate::{EditorState, NodeId};
use jian_core::motion::ClipTrigger;
use jian_ops_schema::node::{PenNode, TextContent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceoverProvider {
    System,
    Neural,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NativeAudioRequest {
    pub document_generation: u64,
    pub node_id: String,
    pub text: Option<String>,
    pub provider: Option<VoiceoverProvider>,
    pub start_ms: u64,
    pub trigger: ClipTrigger,
}

impl EditorState {
    pub fn request_native_audio(&mut self, provider: Option<VoiceoverProvider>) {
        if self.editor_ui.motion_timeline.audio_busy {
            return;
        }
        let Some(root) = self.motion_timeline_scope() else {
            return;
        };
        let node_id = root.base().id.clone();
        let Some(trigger) = self.editor_ui.motion_timeline.clip_trigger() else {
            return;
        };
        let text = provider.map(|_| {
            self.selection
                .set
                .iter()
                .filter_map(|id| match find_node(self.active_children(), id) {
                    Some(PenNode::Text(node)) => Some(match &node.content {
                        TextContent::Plain(s) => s.clone(),
                        TextContent::Styled(parts) => {
                            parts.iter().map(|p| p.text.as_str()).collect::<String>()
                        }
                    }),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n")
        });
        let request = NativeAudioRequest {
            document_generation: self.document_generation(),
            node_id,
            text,
            provider,
            start_ms: self.editor_ui.motion_timeline.playhead_ms.max(0.0) as u64,
            trigger,
        };
        let timeline = &mut self.editor_ui.motion_timeline;
        timeline.playing = false;
        timeline.play_clock = None;
        timeline.audio_request = Some(request);
    }

    pub fn audio_request_target_exists(&self, request: &NativeAudioRequest) -> bool {
        find_node(self.active_children(), &NodeId::new(&request.node_id)).is_some()
    }
}
