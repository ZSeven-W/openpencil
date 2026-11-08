//! The part of the framed root that the device frame actually shows, for
//! in-view motion triggers.
//!
//! A page-scrolling root grows to its content height in the preview scene,
//! and the host scrolls it by painting at an offset rather than by moving
//! scene nodes. The root's own bounds therefore always contain every child,
//! so testing a node against them treats the whole page as visible and fires
//! every in-view animation at lifecycle start. In-view observation instead
//! needs the host's visible window: the scroll offset, the strip a pinned
//! status bar covers, and the visible height between the pinned strips.
//! Pinned nodes (status bar, bottom nav) never scroll, so they always count
//! as in view.

use crate::device_frame::DeviceFrame;
use crate::session::PreviewSession;
use op_editor_ui::layout_scene::ScenePage;
use op_editor_ui::{Point2D, Rect};

/// The visible window of a framed root, relative to the root, so it stays
/// valid when an app-mode screen switch replaces the root.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InViewViewport {
    scroll_y: f32,
    top_inset: f32,
    height: f32,
    pinned: Vec<String>,
}

impl InViewViewport {
    pub(crate) fn from_device_frame(frame: &DeviceFrame, scroll_y: f32) -> Self {
        let top_inset = frame
            .pinned_top
            .as_ref()
            .map_or(0.0, |strip| strip.node_scene.size.y);
        let pinned = [frame.pinned_top.as_ref(), frame.pinned.as_ref()]
            .into_iter()
            .flatten()
            .map(|strip| strip.node_id.clone())
            .filter(|id| !id.is_empty())
            .collect();
        Self {
            scroll_y: scroll_y.max(0.0),
            top_inset,
            height: frame.viewport_h.max(0.0),
            pinned,
        }
    }

    /// The visible window in scene space for a framed root at `root`.
    pub(crate) fn rect_for_root(&self, root: Rect) -> Rect {
        Rect {
            origin: Point2D::new(
                root.origin.x,
                root.origin.y + self.scroll_y + self.top_inset,
            ),
            size: Point2D::new(root.size.x, self.height),
        }
    }

    /// Whether `target` is a pinned node or sits inside one.
    pub(crate) fn is_pinned(&self, page: &ScenePage, target: &str) -> bool {
        self.pinned.iter().any(|id| {
            id == target
                || page
                    .find(id)
                    .is_some_and(|node| node.find(target).is_some())
        })
    }
}

impl PreviewSession {
    /// Feed the device frame's visible window into in-view motion triggers.
    /// `None` (no device frame) keeps the whole framed root as the
    /// viewport. Returns whether the window changed, so the host can
    /// repaint and let observation run against the new window.
    pub fn sync_in_view_viewport(&mut self, frame: Option<&DeviceFrame>, scroll_y: f32) -> bool {
        let viewport = frame.map(|frame| InViewViewport::from_device_frame(frame, scroll_y));
        self.motion.set_in_view_viewport(viewport)
    }
}
