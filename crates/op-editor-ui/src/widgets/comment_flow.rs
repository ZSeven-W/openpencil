//! Comment press / hover flow shared by both widget hosts.
//!
//! Two press entry points, one per ladder position:
//!
//! - [`press_popover`] runs in the top-most overlay tier, where the
//!   popover paints. It consumes only presses inside the card.
//! - [`press_canvas`] runs first in the canvas tier: pins sit above node
//!   content, so a pin wins over selection; in comment mode a press
//!   anywhere else opens a draft there. Outside comment mode, a press
//!   that misses every pin closes an open popover and falls through.
//!
//! Hosts keep only the platform tail (`mark_dirty()`); a collaboration
//! refusal becomes the session notice inside the shared flow.

use op_editor_core::comment_ui::{self, comment_anchor_at, CommentPopoverHit};
use op_editor_core::EditorState;

use crate::layout_scene::LayoutScene;
use crate::widgets::comment_pins::{comment_pins, pin_at};
use crate::widgets::host_canvas_geometry::canvas_doc_point_unclamped;
use crate::Point2D;

/// What a comment press did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CommentPress {
    /// The press belongs to comments; stop the ladder.
    pub consumed: bool,
    /// Something visible changed; repaint.
    pub changed: bool,
}

impl CommentPress {
    fn consumed(changed: bool) -> Self {
        Self {
            consumed: true,
            changed,
        }
    }
}

/// Top-most overlay tier: a press on the open popover.
pub fn press_popover(state: &mut EditorState, point: Point2D, now_ms: u64) -> CommentPress {
    let Some(hit) = state.editor_ui.comments.popover_hit(point.x, point.y) else {
        return CommentPress::default();
    };
    match hit {
        CommentPopoverHit::Close => {
            CommentPress::consumed(comment_ui::close_comment_popover(state))
        }
        CommentPopoverHit::Delete => {
            let result = comment_ui::delete_open_comment(state);
            CommentPress::consumed(comment_ui::settle(state, result, now_ms))
        }
        CommentPopoverHit::Submit => {
            CommentPress::consumed(comment_ui::comment_enter(state, now_ms).unwrap_or(false))
        }
        CommentPopoverHit::SetStatus(status) => {
            let result = comment_ui::set_open_comment_status(state, status, now_ms);
            CommentPress::consumed(comment_ui::settle(state, result, now_ms))
        }
        CommentPopoverHit::Input | CommentPopoverHit::Body => {
            state.editor_ui.comments.input.touch(now_ms);
            CommentPress::consumed(true)
        }
    }
}

/// Canvas tier, ahead of tool dispatch: pins, comment-mode placement,
/// and closing the popover on a press elsewhere.
pub fn press_canvas(
    state: &mut EditorState,
    scene: &LayoutScene,
    point: Point2D,
    viewport_width: f32,
    viewport_height: f32,
) -> CommentPress {
    let pins = comment_pins(state, scene, viewport_width, viewport_height);
    if let Some(pin) = pin_at(&pins, point) {
        let id = pin.id.clone();
        comment_ui::open_comment_thread(state, &id);
        return CommentPress::consumed(true);
    }
    if state.editor_ui.comments.mode {
        let doc = canvas_doc_point_unclamped(state, point.x, point.y);
        let node = scene
            .node_path_at_doc_point(doc, state.viewport.zoom)
            .and_then(|path| path.last().cloned())
            .and_then(|id| {
                let b = scene.active_page()?.find(&id)?.aggregate_bounds();
                Some((id, [b.origin.x, b.origin.y]))
            });
        let anchor = comment_anchor_at(
            state,
            node.as_ref()
                .map(|(id, origin): &(String, [f32; 2])| (id.as_str(), *origin)),
            [doc.x, doc.y],
        );
        comment_ui::begin_comment_draft(state, anchor);
        return CommentPress::consumed(true);
    }
    CommentPress {
        consumed: false,
        changed: comment_ui::close_comment_popover(state),
    }
}

/// Hover over the popover's controls. `true` when the wash changed.
pub fn hover_popover(state: &mut EditorState, point: Point2D) -> bool {
    let hit = state
        .editor_ui
        .comments
        .popover_hit(point.x, point.y)
        .filter(|hit| !matches!(hit, CommentPopoverHit::Body | CommentPopoverHit::Input));
    if state.editor_ui.comments.hover == hit {
        return false;
    }
    state.editor_ui.comments.hover = hit;
    true
}

#[cfg(test)]
#[path = "comment_flow_tests.rs"]
mod tests;
