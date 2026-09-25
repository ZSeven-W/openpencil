//! Facts the tablet reader shows beside the board that the phone reader
//! does not need: the boards' aspect (strip tile width), the name of the
//! board on show, and the latest finished AI reply.

use op_editor_core::{ChatRole, EditorState, NodeId, PenNodeExt};

/// Longest reply excerpt the side panel keeps (chars); the card clips
/// to its own height anyway, this just bounds the per-frame copy.
const REPLY_EXCERPT_CHARS: usize = 600;

/// `(first board's width / height, the current board's name)`.
pub(super) fn board_facts(
    state: &EditorState,
    boards: &[String],
    selected: usize,
) -> (f32, Option<String>) {
    let node = |index: usize| {
        boards.get(index).and_then(|id| {
            op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(id.as_str()))
        })
    };
    let aspect = node(0)
        .and_then(|first| Some((first.width_px()?, first.height_px()?)))
        .filter(|&(w, h)| w > 0.0 && h > 0.0)
        .map_or(16.0 / 9.0, |(w, h)| (w / h) as f32);
    let name = node(selected)
        .and_then(|board| board.base().name.clone())
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty());
    (aspect, name)
}

/// The latest assistant message that finished with real content — not
/// a streaming bubble, not an `error: …` line.
pub(super) fn last_reply(state: &EditorState) -> Option<String> {
    let message = state
        .chat
        .messages
        .iter()
        .rev()
        .find(|message| message.role == ChatRole::Assistant && !message.streaming)?;
    let content = message.content.trim();
    if content.is_empty() || content.starts_with("error:") {
        return None;
    }
    Some(content.chars().take(REPLY_EXCERPT_CHARS).collect())
}
