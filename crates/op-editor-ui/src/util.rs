//! Pure, platform-free helpers shared by the native + web hosts and the
//! property-panel widgets: forgiving hex-color parse/format, panel-number
//! formatting, and selection-handle resize math.
//!
//! These were previously maintained as byte-identical copies inside
//! `op-host-native` (`widget_host/helpers.rs`), `op-host-web`
//! (`widget_host/property_dispatch.rs` + `resize_drag.rs`), and several
//! `property_panel_*` widget modules. Hoisting them into this wasm-clean UI
//! crate — which both hosts already depend on — removes the copy-paste and
//! the silent-drift hazard between platforms.

use crate::widgets::SelectionHandle;
use crate::{Color, Rect};
use jian_core::scroll::ScrollState;

/// Parse a `#RRGGBB` / `#RRGGBBAA` / `#RGB` / bare-hex string into a `Color`.
///
/// Forgiving on purpose: 1–8 hex digits parse. CSS 3-char shorthand expands
/// each nibble (`#F00` → `#FF0000`); lengths 4–5 and 7 are zero-padded into
/// the next supported width (6 or 8) so a mid-edit commit like `#0000` /
/// `#0000000` doesn't visibly reset the colour.
pub fn parse_hex_color(s: &str) -> Option<Color> {
    let trimmed = s.trim().trim_start_matches('#');
    if trimmed.is_empty() || trimmed.len() > 8 {
        return None;
    }
    if !trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    // Canonicalise to either 6 (`RRGGBB`) or 8 (`RRGGBBAA`) chars.
    let canonical = match trimmed.len() {
        3 => {
            let mut out = String::with_capacity(6);
            for c in trimmed.chars() {
                out.push(c);
                out.push(c);
            }
            out
        }
        8 => trimmed.to_string(),
        7 => format!("{:0>8}", trimmed),
        _ => format!("{:0>6}", trimmed),
    };
    let r = u8::from_str_radix(&canonical[0..2], 16).ok()?;
    let g = u8::from_str_radix(&canonical[2..4], 16).ok()?;
    let b = u8::from_str_radix(&canonical[4..6], 16).ok()?;
    let a = if canonical.len() == 8 {
        u8::from_str_radix(&canonical[6..8], 16).ok()? as f32 / 255.0
    } else {
        1.0
    };
    Some(Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a,
    })
}

/// Format a `Color` as `#RRGGBB`. Alpha is dropped — the solid fill / stroke
/// pills carry opacity in a separate input, so they want the 6-char form.
pub fn color_to_hex(c: Color) -> String {
    let r = (c.r.clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (c.g.clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (c.b.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

/// Format a `Color` as `#RRGGBB` when fully opaque, otherwise `#RRGGBBAA`.
/// Gradient stops need this — the schema allows per-stop alpha (the default
/// LinearGradient end stop is `#00000000`), and dropping it on commit would
/// silently turn a transparent stop into a fully opaque one.
pub fn color_to_hex_with_alpha(c: Color) -> String {
    let r = (c.r.clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (c.g.clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (c.b.clamp(0.0, 1.0) * 255.0).round() as u8;
    let a = (c.a.clamp(0.0, 1.0) * 255.0).round() as u8;
    if a == 255 {
        format!("#{:02X}{:02X}{:02X}", r, g, b)
    } else {
        format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
    }
}

/// Format a numeric property value for a panel input: drop a trailing `.0`
/// for whole numbers, otherwise show two decimals.
pub fn format_panel_number(value: f32) -> String {
    if value.fract().abs() < f32::EPSILON {
        format!("{}", value.round() as i32)
    } else {
        format!("{value:.2}")
    }
}

/// Resize `start` by `(dx, dy)` document px in the direction `handle`
/// controls. Negative widths/heights are clamped to 1 px so the bounds never
/// collapse to zero or invert — the user sees a thin sliver instead. Mirrors
/// the TS skia-interaction handle math.
pub fn resize_bounds(start: Rect, handle: SelectionHandle, dx: f32, dy: f32) -> Rect {
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

/// Scroll `scroll` by `delta` clamped to `[0, max]`, returning whether the
/// offset actually moved (false at a clamp edge). Both hosts route every
/// wheel/scroll handler through this so a no-op scroll doesn't request a
/// redraw.
pub fn scroll_by_max(scroll: &mut ScrollState, delta: f32, max: f32) -> bool {
    let before = scroll.offset;
    scroll.scroll_by(delta, max, 0.0);
    scroll.offset != before
}

/// Truncate `s` to at most `max` characters, appending a `…` ellipsis when it
/// is shortened (the ellipsis counts toward `max`). Character-aware, so
/// multibyte glyphs are never split.
pub fn truncate_ellipsis(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let kept: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{kept}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_ellipsis_appends_only_when_shortened() {
        assert_eq!(truncate_ellipsis("hello", 10), "hello");
        assert_eq!(truncate_ellipsis("hello", 5), "hello");
        assert_eq!(truncate_ellipsis("hello", 3), "he…");
        // Saturating: max == 0 must not panic.
        assert_eq!(truncate_ellipsis("hello", 0), "…");
    }

    #[test]
    fn parse_hex_expands_shorthand_and_zero_pads() {
        assert_eq!(
            parse_hex_color("#F00").unwrap(),
            parse_hex_color("#FF0000").unwrap()
        );
        // 5-char mid-edit value zero-pads to 6 instead of rejecting.
        assert!(parse_hex_color("#00000").is_some());
        assert!(parse_hex_color("nothex").is_none());
        assert!(parse_hex_color("").is_none());
    }

    #[test]
    fn hex_round_trip_drops_or_keeps_alpha() {
        let c = parse_hex_color("#112233").unwrap();
        assert_eq!(color_to_hex(c), "#112233");
        let translucent = parse_hex_color("#11223380").unwrap();
        assert_eq!(color_to_hex_with_alpha(translucent), "#11223380");
        assert_eq!(
            color_to_hex_with_alpha(parse_hex_color("#112233").unwrap()),
            "#112233"
        );
    }

    #[test]
    fn panel_number_drops_trailing_zero() {
        assert_eq!(format_panel_number(12.0), "12");
        assert_eq!(format_panel_number(12.50), "12.50");
    }

    #[test]
    fn resize_clamps_instead_of_inverting() {
        let start = Rect::xywh(0.0, 0.0, 10.0, 10.0);
        // Drag the left handle far past the right edge: width clamps to 1px
        // and x pins to (right - 1) rather than inverting.
        let out = resize_bounds(start, SelectionHandle::Left, 100.0, 0.0);
        assert_eq!(out.size.x, 1.0);
        assert_eq!(out.origin.x, 9.0);
    }
}
