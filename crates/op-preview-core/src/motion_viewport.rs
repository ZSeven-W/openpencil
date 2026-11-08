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
use jian_core::motion::InViewWindow;

/// The visible window a device frame shows at `scroll_y`, relative to the
/// framed root so it stays valid when an app-mode screen switch replaces
/// the root.
pub(crate) fn in_view_window(frame: &DeviceFrame, scroll_y: f32) -> InViewWindow {
    let top_inset = frame
        .pinned_top
        .as_ref()
        .map_or(0.0, |strip| strip.node_scene.size.y);
    let pinned = [frame.pinned_top.as_ref(), frame.pinned.as_ref()]
        .into_iter()
        .flatten()
        .map(|strip| strip.node_id.clone())
        .collect();
    InViewWindow::new(scroll_y, top_inset, frame.viewport_h, pinned)
}

impl PreviewSession {
    /// Feed the device frame's visible window into in-view motion triggers.
    /// `None` (no device frame) keeps the whole framed root as the
    /// viewport. Returns whether the window changed, so the host can
    /// repaint and let observation run against the new window.
    pub fn sync_in_view_viewport(&mut self, frame: Option<&DeviceFrame>, scroll_y: f32) -> bool {
        let window = frame.map(|frame| in_view_window(frame, scroll_y));
        self.motion.set_in_view_window(window)
    }
}
