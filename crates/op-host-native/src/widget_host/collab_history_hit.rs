//! Read-only hit-test seam for collaboration-aware undo/redo dispatch.
//!
//! `WidgetHostNative` intentionally does not own a collaboration runtime.
//! Mobile embedders ask this seam before the ordinary press ladder so active
//! sessions can route history to the runtime without teaching widgets about
//! transport actors.

use op_editor_ui::widgets::{MobileAppBar, MobileAppBarHit, Toolbar, ToolbarAction, ToolbarHit};
use op_editor_ui::Point2D;

use super::WidgetHostNative;

/// Collaboration-aware history action hit by mobile/embedded chrome.
///
/// The concrete collaboration runtime stays outside this crate. Embedders use
/// this result to route undo/redo to selective collaboration history before
/// allowing the ordinary host press ladder to invoke standalone history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollabHistoryAction {
    Undo,
    Redo,
}

impl WidgetHostNative {
    /// Resolve an undo/redo button at `point`, using the exact app-bar/toolbar
    /// geometry and hit-test routines used by the press ladder.
    pub fn collab_history_action_at(
        &self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Option<CollabHistoryAction> {
        if self.preview_slideshow_active() {
            return None;
        }
        let point = Point2D::new(x, y);
        if self.editor_state.editor_ui.touch_chrome() {
            let bar = MobileAppBar::for_editor(&self.editor_state);
            let rect = op_editor_ui::widgets::host_canvas_geometry::touch_app_bar_rect(
                &self.editor_state,
                viewport_width,
            );
            return match bar.hit_test(rect, point) {
                Some(MobileAppBarHit::Undo) => Some(CollabHistoryAction::Undo),
                Some(MobileAppBarHit::Redo) => Some(CollabHistoryAction::Redo),
                _ => None,
            };
        }

        let rect = self.toolbar_rect(viewport_width, viewport_height);
        match Toolbar::for_editor(&self.editor_state).hit_test(rect, point) {
            Some(ToolbarHit::Action(ToolbarAction::Undo)) => Some(CollabHistoryAction::Undo),
            Some(ToolbarHit::Action(ToolbarAction::Redo)) => Some(CollabHistoryAction::Redo),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find_action(
        host: &WidgetHostNative,
        viewport_width: f32,
        viewport_height: f32,
        expected: CollabHistoryAction,
    ) -> Option<(f32, f32)> {
        for y in 0..viewport_height as usize {
            for x in 0..viewport_width as usize {
                if host.collab_history_action_at(
                    x as f32 + 0.5,
                    y as f32 + 0.5,
                    viewport_width,
                    viewport_height,
                ) == Some(expected)
                {
                    return Some((x as f32 + 0.5, y as f32 + 0.5));
                }
            }
        }
        None
    }

    #[test]
    fn compact_app_bar_exposes_both_history_actions() {
        let mut host = WidgetHostNative::new();
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.touch = true;
        ui.size_class = op_editor_core::size_class::EditorSizeClass::Compact;
        assert!(find_action(&host, 390.0, 844.0, CollabHistoryAction::Undo).is_some());
        assert!(find_action(&host, 390.0, 844.0, CollabHistoryAction::Redo).is_some());
    }

    #[test]
    fn desktop_toolbar_exposes_both_history_actions() {
        let host = WidgetHostNative::new();
        assert!(find_action(&host, 1200.0, 800.0, CollabHistoryAction::Undo).is_some());
        assert!(find_action(&host, 1200.0, 800.0, CollabHistoryAction::Redo).is_some());
    }
}
