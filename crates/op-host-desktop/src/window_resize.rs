//! Window-edge resize hit-testing for borderless (Windows / Linux) windows.
//!
//! macOS keeps its native `NSWindow` decorations (a transparent titlebar over a
//! real window), so the OS already paints edge-resize cursors and owns the drag.
//! Windows / Linux create the window with `with_decorations(false)`
//! (`app_handler.rs`) — a truly borderless window with no OS-provided resize
//! band — so we synthesize one: an invisible ring around the window edge that
//! maps to a `ResizeDirection`, driving both the hover cursor and a
//! `Window::drag_resize_window` grab on press.
//!
//! The pure hit-test below is compiled on every platform so its unit tests run
//! on the macOS dev machine; only the `app_handler` call sites are cfg-gated to
//! non-macOS (macOS must keep the native edges untouched).
#![cfg_attr(target_os = "macos", allow(dead_code))]

use winit::window::ResizeDirection;

/// Logical-pixel thickness of the invisible edge-resize band.
pub const RESIZE_BORDER: f32 = 6.0;

/// Map a window-relative cursor position (logical px) to the edge-resize
/// direction it falls in, or `None` for interior points. `w` / `h` are the
/// window's logical size. Corner squares take priority over straight edges so a
/// corner grab resizes diagonally.
pub fn window_resize_direction(x: f32, y: f32, w: f32, h: f32) -> Option<ResizeDirection> {
    window_resize_direction_with_border(x, y, w, h, RESIZE_BORDER)
}

fn window_resize_direction_with_border(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    border: f32,
) -> Option<ResizeDirection> {
    // Guard degenerate sizes (both edge bands would overlap) and out-of-window
    // coordinates so a stray negative / overshoot event can't report a phantom
    // edge. The 640×400 min inner size means the size guard never fires in
    // practice — it just keeps the function total.
    if w <= border * 2.0 || h <= border * 2.0 {
        return None;
    }
    if x < 0.0 || y < 0.0 || x > w || y > h {
        return None;
    }
    let west = x <= border;
    let east = x >= w - border;
    let north = y <= border;
    let south = y >= h - border;
    Some(match (north, south, west, east) {
        (true, _, true, _) => ResizeDirection::NorthWest,
        (true, _, _, true) => ResizeDirection::NorthEast,
        (_, true, true, _) => ResizeDirection::SouthWest,
        (_, true, _, true) => ResizeDirection::SouthEast,
        (true, ..) => ResizeDirection::North,
        (_, true, ..) => ResizeDirection::South,
        (_, _, true, _) => ResizeDirection::West,
        (.., true) => ResizeDirection::East,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::window::ResizeDirection::*;

    const W: f32 = 1200.0;
    const H: f32 = 800.0;

    #[test]
    fn interior_point_is_not_a_resize() {
        assert_eq!(window_resize_direction(W / 2.0, H / 2.0, W, H), None);
        // Just inside the band on every side.
        assert_eq!(
            window_resize_direction(RESIZE_BORDER + 1.0, H / 2.0, W, H),
            None
        );
        assert_eq!(
            window_resize_direction(W - RESIZE_BORDER - 1.0, H / 2.0, W, H),
            None
        );
        assert_eq!(
            window_resize_direction(W / 2.0, RESIZE_BORDER + 1.0, W, H),
            None
        );
        assert_eq!(
            window_resize_direction(W / 2.0, H - RESIZE_BORDER - 1.0, W, H),
            None
        );
    }

    #[test]
    fn straight_edges_map_to_axis_directions() {
        assert_eq!(window_resize_direction(0.0, H / 2.0, W, H), Some(West));
        assert_eq!(window_resize_direction(W, H / 2.0, W, H), Some(East));
        assert_eq!(window_resize_direction(W / 2.0, 0.0, W, H), Some(North));
        assert_eq!(window_resize_direction(W / 2.0, H, W, H), Some(South));
    }

    #[test]
    fn corners_take_priority_and_resize_diagonally() {
        assert_eq!(window_resize_direction(0.0, 0.0, W, H), Some(NorthWest));
        assert_eq!(window_resize_direction(W, 0.0, W, H), Some(NorthEast));
        assert_eq!(window_resize_direction(0.0, H, W, H), Some(SouthWest));
        assert_eq!(window_resize_direction(W, H, W, H), Some(SouthEast));
    }

    #[test]
    fn band_boundary_is_inclusive() {
        // Exactly `RESIZE_BORDER` from the edge still counts as the edge.
        assert_eq!(
            window_resize_direction(RESIZE_BORDER, H / 2.0, W, H),
            Some(West)
        );
        assert_eq!(
            window_resize_direction(W - RESIZE_BORDER, H / 2.0, W, H),
            Some(East)
        );
    }

    #[test]
    fn out_of_window_or_degenerate_is_none() {
        assert_eq!(window_resize_direction(-1.0, H / 2.0, W, H), None);
        assert_eq!(window_resize_direction(W + 1.0, H / 2.0, W, H), None);
        assert_eq!(window_resize_direction(W / 2.0, -1.0, W, H), None);
        // A window narrower than both bands combined never reports an edge.
        assert_eq!(
            window_resize_direction_with_border(1.0, 1.0, 8.0, 8.0, 6.0),
            None
        );
    }
}
