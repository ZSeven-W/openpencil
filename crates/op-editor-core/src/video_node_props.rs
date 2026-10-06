//! Image-node video metadata getters and undoable editor mutators.
//!
//! Video remains an optional property of `ImageNode`: the image's `src` is
//! the poster and this module owns only the playback metadata. Mutations are
//! expressed as `EditorCommand`s so the same validation path serves the
//! property panel and other editor callers.

use crate::command::{EditorCommand, VideoPlaybackField};
use crate::node_id::NodeId;
use crate::state::EditorState;
use crate::walkers::find_node_mut;
use jian_ops_schema::node::{PenNode, VideoMeta};

/// Return an image node's video metadata, if present.
pub fn video_node(node: &PenNode) -> Option<&VideoMeta> {
    match node {
        PenNode::Image(image) => image.video.as_ref(),
        _ => None,
    }
}

/// Return a default video policy for the image-section "Add video" action.
/// Autoplay is opt-in; muted starts enabled so a later autoplay edit remains
/// browser-compatible without changing the user's explicit choice.
pub fn default_video_meta() -> VideoMeta {
    VideoMeta {
        src: String::new(),
        autoplay: false,
        r#loop: false,
        muted: true,
        hold_last_frame: false,
        click_to_replay: false,
        video_prompt: None,
    }
}

/// Set the source URL on an image node's video metadata.
pub fn set_video_node_src(node: &mut PenNode, src: &str) -> bool {
    let Some(video) = image_video_mut(node) else {
        return false;
    };
    if video.src == src {
        return false;
    }
    video.src = src.to_owned();
    true
}

/// Set one playback-policy boolean on an image node's video metadata.
pub fn set_video_node_playback(node: &mut PenNode, field: VideoPlaybackField, value: bool) -> bool {
    let Some(video) = image_video_mut(node) else {
        return false;
    };
    let target = match field {
        VideoPlaybackField::Autoplay => &mut video.autoplay,
        VideoPlaybackField::Loop => &mut video.r#loop,
        VideoPlaybackField::Muted => &mut video.muted,
        VideoPlaybackField::HoldLastFrame => &mut video.hold_last_frame,
        VideoPlaybackField::ClickToReplay => &mut video.click_to_replay,
    };
    if *target == value {
        return false;
    }
    *target = value;
    true
}

/// Attach default video metadata to an image node.
pub fn add_video_to_image_node(node: &mut PenNode) -> bool {
    let PenNode::Image(image) = node else {
        return false;
    };
    if image.video.is_some() {
        return false;
    }
    image.video = Some(default_video_meta());
    true
}

/// Remove video metadata from an image node.
pub fn remove_video_from_image_node(node: &mut PenNode) -> bool {
    let PenNode::Image(image) = node else {
        return false;
    };
    image.video.take().is_some()
}

fn image_video_mut(node: &mut PenNode) -> Option<&mut VideoMeta> {
    match node {
        PenNode::Image(image) => image.video.as_mut(),
        _ => None,
    }
}

impl EditorState {
    /// Selected image-node video metadata, if the selection points at an
    /// image carrying a video object.
    pub fn selected_video(&self) -> Option<&VideoMeta> {
        self.selected_node().and_then(video_node)
    }

    /// Apply one video command and record exactly one undo snapshot when it
    /// changes the selected image node.
    fn apply_video_command(&mut self, command: EditorCommand) -> bool {
        let before = self.snapshot_for_history();
        let changed = self.apply(command);
        if changed && self.snapshot_for_history() != before {
            self.history_push_past(before);
            true
        } else {
            false
        }
    }

    /// Set the selected image node's video URL through `EditorCommand`.
    pub fn set_selected_video_src(&mut self, src: &str) -> bool {
        let node_id = self.selection.anchor.clone();
        if !node_id.is_real() {
            return false;
        }
        self.apply_video_command(EditorCommand::SetImageVideoSrc {
            node_id,
            src: src.to_owned(),
        })
    }

    /// Set one playback policy field on the selected image node's video.
    pub fn set_selected_video_playback(&mut self, field: VideoPlaybackField, value: bool) -> bool {
        let node_id = self.selection.anchor.clone();
        if !node_id.is_real() {
            return false;
        }
        self.apply_video_command(EditorCommand::SetImageVideoPlayback {
            node_id,
            field,
            value,
        })
    }

    /// Add default video metadata to the selected image node.
    pub fn add_selected_video(&mut self) -> bool {
        let node_id = self.selection.anchor.clone();
        if !node_id.is_real() {
            return false;
        }
        self.apply_video_command(EditorCommand::AddImageVideo { node_id })
    }

    /// Remove video metadata from the selected image node.
    pub fn remove_selected_video(&mut self) -> bool {
        let node_id = self.selection.anchor.clone();
        if !node_id.is_real() {
            return false;
        }
        self.apply_video_command(EditorCommand::RemoveImageVideo { node_id })
    }

    /// Low-level command arm used by the command applier. Kept here beside
    /// the schema-field helpers so the panel cannot bypass command history.
    pub(crate) fn apply_video_src(&mut self, node_id: &NodeId, src: &str) -> bool {
        if !node_id.is_real() || !self.is_editable(node_id) {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), node_id) else {
            return false;
        };
        set_video_node_src(node, src)
    }

    pub(crate) fn video_policy(
        &mut self,
        node_id: &NodeId,
        field: VideoPlaybackField,
        value: bool,
    ) -> bool {
        if !node_id.is_real() || !self.is_editable(node_id) {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), node_id) else {
            return false;
        };
        set_video_node_playback(node, field, value)
    }

    pub(crate) fn apply_add_video(&mut self, node_id: &NodeId) -> bool {
        if !node_id.is_real() || !self.is_editable(node_id) {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), node_id) else {
            return false;
        };
        add_video_to_image_node(node)
    }

    pub(crate) fn apply_remove_video(&mut self, node_id: &NodeId) -> bool {
        if !node_id.is_real() || !self.is_editable(node_id) {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), node_id) else {
            return false;
        };
        remove_video_from_image_node(node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> EditorState {
        let document = jian_ops_schema::load_str(
            r#"{"version":"1.0.0","children":[
                {"type":"image","id":"hero","name":"Hero","x":0,"y":0,
                 "width":320,"height":180,"src":"poster.png"}
            ]}"#,
        )
        .expect("image fixture parses")
        .value;
        let mut state = EditorState::from_document(document);
        state.set_single_selection(NodeId::new("hero"));
        state
    }

    #[test]
    fn selected_video_props_round_trip_through_commands_and_undo() {
        let mut state = state();
        assert!(state.add_selected_video());
        assert_eq!(state.selected_video().map(|video| video.muted), Some(true));
        assert!(state.set_selected_video_src("https://cdn.example/hero.mp4"));
        assert!(state.set_selected_video_playback(VideoPlaybackField::Autoplay, true));
        assert!(state.set_selected_video_playback(VideoPlaybackField::Loop, true));
        assert_eq!(
            state.selected_video().map(|video| video.src.as_str()),
            Some("https://cdn.example/hero.mp4")
        );
        assert_eq!(
            state.selected_video().map(|video| video.autoplay),
            Some(true)
        );
        assert_eq!(state.selected_video().map(|video| video.r#loop), Some(true));

        assert!(state.remove_selected_video());
        assert!(state.selected_video().is_none());
        assert!(state.apply(EditorCommand::Undo));
        assert_eq!(state.selected_video().map(|video| video.r#loop), Some(true));
        assert!(state.apply(EditorCommand::Undo));
        assert_eq!(
            state.selected_video().map(|video| video.r#loop),
            Some(false)
        );
        assert!(state.apply(EditorCommand::Undo));
        assert_eq!(
            state.selected_video().map(|video| video.autoplay),
            Some(false)
        );
        assert!(state.apply(EditorCommand::Undo));
        assert_eq!(
            state.selected_video().map(|video| video.src.as_str()),
            Some("")
        );
        assert!(state.apply(EditorCommand::Undo));
        assert!(state.selected_video().is_none());
    }
}
