//! The canvas area a Preview device frame may occupy.
//!
//! The device switcher (`PreviewDeviceSwitcher`) floats at the canvas top
//! and, for multi-screen documents, the screen pills
//! (`ScreenSwitcherPills`) sit just below it. The device frame used to be
//! fitted to the WHOLE canvas, so both rows covered the phone's status bar
//! and title, and the floating chat composer covered its bottom edge and
//! swallowed taps there. Both hosts now fit the frame into this area
//! instead; everything that derives from the frame (paint, hit-testing,
//! pointer mapping, scrolling, pinned bars, the mode-switch animation)
//! follows automatically.

use crate::widgets::{PreviewDeviceSwitcher, ScreenSwitcherPills};
use crate::{Point2D, Rect};

/// Breathing room between the switcher rows / an obstacle and the frame.
const FRAME_GAP: f32 = 12.0;
/// Below this height an obstacle is ignored rather than squeezing the
/// frame to a sliver (tiny windows still keep the switcher band).
const MIN_FRAME_AREA_H: f32 = 160.0;

/// Area of `canvas` left for the device frame: below the switcher rows
/// (`screen_pills` = the screen-pill row is shown) and clear of
/// `obstacle` (the floating chat) when it is anchored to the top or
/// bottom edge of the canvas.
pub fn preview_frame_area(canvas: Rect, screen_pills: bool, obstacle: Option<Rect>) -> Rect {
    let band_bottom = if screen_pills {
        let row = ScreenSwitcherPills::row_rect(canvas, 1);
        row.origin.y + row.size.y
    } else {
        let pill = PreviewDeviceSwitcher::pill_rect(canvas);
        pill.origin.y + pill.size.y
    };
    let canvas_bottom = canvas.origin.y + canvas.size.y;
    let mut top = (band_bottom + FRAME_GAP).min(canvas_bottom);
    let mut bottom = canvas_bottom - FRAME_GAP;
    if let Some(ob) = obstacle.filter(|ob| overlaps(*ob, canvas)) {
        let mid = canvas.origin.y + canvas.size.y / 2.0;
        let (new_top, new_bottom) = if ob.origin.y > mid {
            (top, bottom.min(ob.origin.y - FRAME_GAP))
        } else if ob.origin.y + ob.size.y < mid {
            (top.max(ob.origin.y + ob.size.y + FRAME_GAP), bottom)
        } else {
            (top, bottom)
        };
        if new_bottom - new_top >= MIN_FRAME_AREA_H {
            top = new_top;
            bottom = new_bottom;
        }
    }
    Rect {
        origin: Point2D::new(canvas.origin.x, top),
        size: Point2D::new(canvas.size.x, (bottom - top).max(0.0)),
    }
}

/// Canvas-mode Preview pan (`viewport.pan_x/pan_y`, canvas-relative) that
/// frames the scene-space `screen` inside `area` at `zoom`: centred when it
/// fits, top-aligned just below the switcher rows when it is taller, so the
/// status bar and title are never parked under the switchers.
pub fn canvas_preview_pan(canvas: Rect, area: Rect, screen: Rect, zoom: f32) -> (f32, f32) {
    let area_x = area.origin.x - canvas.origin.x;
    let area_y = area.origin.y - canvas.origin.y;
    let pan_x = area_x + area.size.x / 2.0 - (screen.origin.x + screen.size.x / 2.0) * zoom;
    let height = screen.size.y * zoom;
    let pan_y = if height <= area.size.y {
        area_y + area.size.y / 2.0 - (screen.origin.y + screen.size.y / 2.0) * zoom
    } else {
        area_y - screen.origin.y * zoom
    };
    (pan_x, pan_y)
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.origin.x < b.origin.x + b.size.x
        && b.origin.x < a.origin.x + a.size.x
        && a.origin.y < b.origin.y + b.size.y
        && b.origin.y < a.origin.y + a.size.y
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas() -> Rect {
        Rect::xywh(100.0, 100.0, 1000.0, 900.0)
    }

    fn bottom(r: Rect) -> f32 {
        r.origin.y + r.size.y
    }

    #[test]
    fn area_starts_below_both_switcher_rows() {
        let area = preview_frame_area(canvas(), true, None);
        let pills = ScreenSwitcherPills::row_rect(canvas(), 2);
        assert!(area.origin.y >= bottom(pills));
        let single = preview_frame_area(canvas(), false, None);
        assert!(single.origin.y >= bottom(PreviewDeviceSwitcher::pill_rect(canvas())));
        assert!(single.origin.y < area.origin.y);
    }

    #[test]
    fn bottom_chat_trims_the_bottom_and_top_chat_trims_the_top() {
        let chat = Rect::xywh(120.0, 900.0, 300.0, 80.0);
        let area = preview_frame_area(canvas(), true, Some(chat));
        assert!(bottom(area) <= chat.origin.y);
        let top_chat = Rect::xywh(120.0, 160.0, 300.0, 80.0);
        let area = preview_frame_area(canvas(), false, Some(top_chat));
        assert!(area.origin.y >= bottom(top_chat));
    }

    #[test]
    fn canvas_pan_keeps_a_tall_screen_top_below_the_switchers() {
        let area = preview_frame_area(canvas(), true, None);
        let tall = Rect::xywh(0.0, 0.0, 390.0, 2000.0);
        let (_, pan_y) = canvas_preview_pan(canvas(), area, tall, 1.0);
        let screen_top = canvas().origin.y + pan_y;
        assert!((screen_top - area.origin.y).abs() < 0.01);
        let short = Rect::xywh(0.0, 0.0, 390.0, 300.0);
        let (_, pan_y) = canvas_preview_pan(canvas(), area, short, 1.0);
        let top = canvas().origin.y + pan_y;
        assert!(top >= area.origin.y && top + 300.0 <= area.origin.y + area.size.y);
    }

    #[test]
    fn a_short_canvas_ignores_the_obstacle_but_keeps_the_band() {
        let short = Rect::xywh(0.0, 0.0, 800.0, 260.0);
        let chat = Rect::xywh(0.0, 180.0, 300.0, 70.0);
        let area = preview_frame_area(short, true, Some(chat));
        assert!(area.size.y > 0.0);
        assert!(area.origin.y >= bottom(ScreenSwitcherPills::row_rect(short, 1)));
        assert!(
            bottom(area) > chat.origin.y,
            "obstacle ignored when it would crush the frame"
        );
    }
}
