//! Host wiring for the TopBar's painted traffic-light dots.
//!
//! The dots have always been PAINTED by `top_bar_paint`, and the desktop
//! binary reads them itself before dispatching a press. The embedded hosts
//! (iOS / Android / OHOS) never reach that code, so a dot click used to do
//! nothing at all there — on HarmonyOS 2in1, where the platform title bar is
//! hidden, that left the window with no controls whatsoever. These tests pin
//! the tier that turns a dot press into a one-shot request the shell drains
//! over the C ABI's shell-action channel.
//!
//! macOS is excluded on purpose, not for convenience: there the platform
//! paints the real traffic lights and `TopBar::window_control_at` returns
//! `None` by design, so the tier can never fire. The FFI half of the wiring
//! is pinned platform-independently in
//! `op-engine-ffi/src/editor_auth_window_tests.rs`.
#![cfg(not(target_os = "macos"))]

use super::*;
use op_editor_core::WindowControlRequest;
use op_editor_ui::widgets::{TopBar, WindowControl, TOP_BAR_HEIGHT};
use op_editor_ui::{Point2D, Rect};

const VIEWPORT_W: f32 = 1400.0;
const VIEWPORT_H: f32 = 900.0;

fn hosted() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    host.last_viewport_w = VIEWPORT_W;
    host.last_viewport_h = VIEWPORT_H;
    host
}

fn top_bar_rect() -> Rect {
    Rect {
        origin: Point2D::ZERO,
        size: Point2D::new(VIEWPORT_W, TOP_BAR_HEIGHT),
    }
}

/// A point inside the dot that resolves to `control`, found through the same
/// hit-test the paint pass is aligned to — so the test cannot drift away from
/// the painted geometry.
fn dot_point(host: &WidgetHostNative, control: WindowControl) -> Point2D {
    let bar = TopBar::for_editor_ui(&host.editor_state().editor_ui);
    let rect = top_bar_rect();
    let y = rect.size.y / 2.0;
    let mut x = rect.origin.x;
    while x < rect.origin.x + rect.size.x {
        let point = Point2D::new(x, y);
        if bar.window_control_at(rect, point) == Some(control) {
            return point;
        }
        x += 1.0;
    }
    panic!("no painted dot resolves to {control:?}");
}

#[test]
fn each_dot_raises_its_own_window_request() {
    for (control, expected) in [
        (WindowControl::Close, WindowControlRequest::Close),
        (WindowControl::Minimize, WindowControlRequest::Minimize),
        (WindowControl::Maximize, WindowControlRequest::Zoom),
    ] {
        let mut host = hosted();
        let point = dot_point(&host, control);
        assert!(
            host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H),
            "{control:?} must consume the press"
        );
        assert_eq!(
            host.editor_state().editor_ui.pending_window_control,
            Some(expected),
            "{control:?} must raise {expected:?}"
        );
    }
}

#[test]
fn touch_chrome_leaves_the_window_to_the_platform() {
    // Phones and tablets paint no dots and their shells own the window, so
    // the tier must not fire there — otherwise a tap near the bar's left
    // edge would minimise or close the app.
    let mut host = hosted();
    let point = dot_point(&host, WindowControl::Close);
    host.editor_state_mut().editor_ui.touch = true;
    host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H);
    assert_eq!(host.editor_state().editor_ui.pending_window_control, None);
}

#[test]
fn the_apps_own_top_bar_buttons_are_untouched() {
    // The dots reserve a fixed left inset and the app's chrome starts after
    // it; a press on the sidebar toggle must still reach the app.
    let mut host = hosted();
    let bar = TopBar::for_editor_ui(&host.editor_state().editor_ui);
    let rect = bar
        .button_rect(top_bar_rect(), op_editor_core::TopBarButton::ToggleSidebar)
        .expect("the sidebar toggle paints in this build");
    let point = Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    );
    let before = host.editor_state().editor_ui.sidebar_open;
    host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H);
    assert_eq!(host.editor_state().editor_ui.pending_window_control, None);
    assert_ne!(
        host.editor_state().editor_ui.sidebar_open,
        before,
        "the sidebar toggle must still own its own press"
    );
}
