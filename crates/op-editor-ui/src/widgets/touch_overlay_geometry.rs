//! Canonical anchors for overlays opened from touch chrome.
//!
//! Phones and tablets do not paint the desktop TopBar, so reusing its hidden
//! account/collaboration button rect makes paint, press and hover disagree
//! about where a popover lives. Touch overlays instead hang from the persistent
//! app-bar overflow target; desktop/web callers keep their established anchors.

use super::account_menu::AccountMenu;
use super::host_canvas_geometry;
use super::mobile_chrome::MobileAppBar;
use super::top_bar::TopBar;
use super::{CollabPanel, TOP_BAR_HEIGHT};
use crate::{Point2D, Rect};
use op_editor_core::EditorState;

pub fn account_anchor(state: &EditorState, viewport_w: f32) -> Rect {
    if state.editor_ui.touch_chrome() {
        let bar = host_canvas_geometry::touch_app_bar_rect(state, viewport_w);
        return MobileAppBar::overflow_rect(bar);
    }
    let top_bar = Rect::xywh(0.0, 0.0, viewport_w, TOP_BAR_HEIGHT);
    TopBar::for_editor_ui(&state.editor_ui).account_button_rect(top_bar)
}

pub fn collaboration_anchor(state: &EditorState, viewport_w: f32) -> Rect {
    if state.editor_ui.touch_chrome() {
        let bar = host_canvas_geometry::touch_app_bar_rect(state, viewport_w);
        return MobileAppBar::overflow_rect(bar);
    }
    let top_bar = Rect::xywh(0.0, 0.0, viewport_w, TOP_BAR_HEIGHT);
    TopBar::for_editor_ui(&state.editor_ui).collaboration_chip_rect_estimated(top_bar)
}

pub fn account_menu_rect(state: &EditorState, menu: &AccountMenu<'_>, viewport_w: f32) -> Rect {
    let mut rect = menu.rect_at(account_anchor(state, viewport_w));
    let max_x = (viewport_w - rect.size.x - 8.0).max(8.0);
    rect.origin.x = rect.origin.x.clamp(8.0, max_x);
    rect
}

pub fn collaboration_panel_rect(
    state: &EditorState,
    panel: &CollabPanel<'_>,
    viewport_w: f32,
    viewport_h: f32,
) -> Rect {
    panel.rect_at(
        collaboration_anchor(state, viewport_w),
        Rect {
            origin: Point2D::ZERO,
            size: Point2D::new(viewport_w, viewport_h),
        },
    )
}
