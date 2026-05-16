//! Free-standing helpers used across the `widget_host` submodules:
//! hex-color parsing, color formatting, rect containment, and the
//! resize-bounds math. Pulled out of `widget_host.rs` to keep the
//! spine file under the 800-line ceiling.

use op_editor_ui::widgets::SelectionHandle;
use op_editor_ui::{Color, Point2D, Rect};

/// Small breathing room from the canvas corner so the chat pill
/// doesn't visually touch the canvas edge (per 2026-05-10 user
/// note "稍微加一点上下偶有的间距，一点点").
pub(in crate::widget_host) const AICHAT_INSET_BOTTOM: f32 = 12.0;
pub(in crate::widget_host) const AICHAT_INSET_LEFT: f32 = 12.0;

pub(in crate::widget_host) const TOOLBAR_INSET_X: f32 = 12.0;
pub(in crate::widget_host) const TOOLBAR_INSET_Y: f32 = 12.0;
pub(in crate::widget_host) const STATUS_INSET: f32 = 16.0;

/// Pixel half-thickness of the resize gutter on each panel edge —
/// click within this distance of the edge to begin a resize drag.
pub(in crate::widget_host) const PANEL_RESIZE_GUTTER: f32 = 4.0;
/// Hard floor / ceiling for resizable panels (TS app uses similar
/// limits — left/right rails can't shrink below ~180 or grow past
/// half the viewport).
pub(in crate::widget_host) const PANEL_MIN_WIDTH: f32 = 180.0;
pub(in crate::widget_host) const PANEL_MAX_WIDTH: f32 = 480.0;

/// Parse a `#RRGGBB` / `#RGB` / bare-hex string into an OP
/// `Color`. Forgiving: anything between 1 and 6 hex digits parses
/// — 3 chars expands CSS-style (`#F00` → `#FF0000`), other
/// shorter values are zero-padded to 6 so a slightly-too-short
/// typed value like `#00000` lands on `#000000` instead of being
/// rejected silently.
pub(in crate::widget_host) fn parse_hex_color(s: &str) -> Option<Color> {
    let trimmed = s.trim().trim_start_matches('#');
    if trimmed.is_empty() || trimmed.len() > 6 {
        return None;
    }
    if !trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let six = match trimmed.len() {
        3 => {
            // CSS shorthand — duplicate each nibble.
            let mut out = String::with_capacity(6);
            for c in trimmed.chars() {
                out.push(c);
                out.push(c);
            }
            out
        }
        len if len < 6 => {
            // Pad with leading zeros so the user's typed digits
            // populate the lowest bits (`#00000` → `#000000`,
            // `#FF` → `#0000FF`).
            format!("{:0>6}", trimmed)
        }
        _ => trimmed.to_string(),
    };
    let r = u8::from_str_radix(&six[0..2], 16).ok()?;
    let g = u8::from_str_radix(&six[2..4], 16).ok()?;
    let b = u8::from_str_radix(&six[4..6], 16).ok()?;
    Some(Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    })
}

/// Format an OP `Color` as `#RRGGBB`. Alpha is dropped (the hex
/// pill ignores it; opacity has its own input).
#[allow(dead_code)]
pub(in crate::widget_host) fn color_to_hex(c: Color) -> String {
    let r = (c.r.clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (c.g.clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (c.b.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

pub(in crate::widget_host) fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.x <= r.origin.x + r.size.x
        && p.y >= r.origin.y
        && p.y <= r.origin.y + r.size.y
}

/// Resize `start_bounds` by `(dx, dy)` document px in the
/// direction the handle controls. Negative widths/heights are
/// clamped to ~1 px so the bounds never collapse to zero or
/// invert. Mirrors the TS skia-interaction handle math.
pub(in crate::widget_host) fn resize_bounds(
    start: Rect,
    handle: SelectionHandle,
    dx: f32,
    dy: f32,
) -> Rect {
    let mut x = start.origin.x;
    let mut y = start.origin.y;
    let mut w = start.size.x;
    let mut h = start.size.y;
    match handle {
        SelectionHandle::TopLeft => {
            x += dx;
            y += dy;
            w -= dx;
            h -= dy;
        }
        SelectionHandle::Top => {
            y += dy;
            h -= dy;
        }
        SelectionHandle::TopRight => {
            y += dy;
            w += dx;
            h -= dy;
        }
        SelectionHandle::Right => {
            w += dx;
        }
        SelectionHandle::BottomRight => {
            w += dx;
            h += dy;
        }
        SelectionHandle::Bottom => {
            h += dy;
        }
        SelectionHandle::BottomLeft => {
            x += dx;
            w -= dx;
            h += dy;
        }
        SelectionHandle::Left => {
            x += dx;
            w -= dx;
        }
    }
    if w < 1.0 {
        // Don't flip; clamp at 1 px so the user sees a thin
        // sliver instead of an inverted rect.
        if matches!(
            handle,
            SelectionHandle::Left | SelectionHandle::TopLeft | SelectionHandle::BottomLeft
        ) {
            x = start.origin.x + start.size.x - 1.0;
        }
        w = 1.0;
    }
    if h < 1.0 {
        if matches!(
            handle,
            SelectionHandle::Top | SelectionHandle::TopLeft | SelectionHandle::TopRight
        ) {
            y = start.origin.y + start.size.y - 1.0;
        }
        h = 1.0;
    }
    Rect::xywh(x, y, w, h)
}
