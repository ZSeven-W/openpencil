//! Pure HSV / RGB / hex colour-conversion helpers — extracted from
//! `color_picker.rs` to keep that file under the 800-line cap. No
//! `EditorState` or schema dependency; just float / string math
//! ported verbatim from shell-core. Re-exported through
//! `color_picker.rs` (and the crate root) so existing
//! `crate::color_picker::parse_hex_rgb` callers keep working.

/// HSV → RGB, h 0..360, s/v 0..1. Each channel 0..1.
/// Ported verbatim from shell-core's `hsv_to_rgb`.
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let h = h.rem_euclid(360.0);
    let c = v * s;
    let hh = h / 60.0;
    let x = c * (1.0 - (hh.rem_euclid(2.0) - 1.0).abs());
    let (r1, g1, b1) = match hh as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    (r1 + m, g1 + m, b1 + m)
}

/// RGB (0..1) → HSV (h 0..360, s 0..1, v 0..1).
/// Ported verbatim from shell-core's `rgb_to_hsv`.
pub fn rgb_to_hsv(rgb: (f32, f32, f32)) -> (f32, f32, f32) {
    let (r, g, b) = rgb;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let v = max;
    let delta = max - min;
    let s = if max <= 0.0 { 0.0 } else { delta / max };
    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };
    (h, s, v)
}

/// Parse `#rgb` / `#rrggbb` / `#rrggbbaa` into RGB floats (0..1).
/// Lenient on case; requires the leading `#`.
pub fn parse_hex_rgb(s: &str) -> Option<(f32, f32, f32)> {
    let s = s.trim().strip_prefix('#')?;
    let (r, g, b) = match s.len() {
        3 => (
            u8::from_str_radix(&s[0..1].repeat(2), 16).ok()?,
            u8::from_str_radix(&s[1..2].repeat(2), 16).ok()?,
            u8::from_str_radix(&s[2..3].repeat(2), 16).ok()?,
        ),
        6 | 8 => (
            u8::from_str_radix(&s[0..2], 16).ok()?,
            u8::from_str_radix(&s[2..4], 16).ok()?,
            u8::from_str_radix(&s[4..6], 16).ok()?,
        ),
        _ => return None,
    };
    Some((r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0))
}

/// Parse the alpha channel out of `#rrggbbaa` — defaults to `1.0`
/// when the hex is 6-char (no alpha authored) or unparseable. Used
/// by the gradient-stop colour picker so dragging SV / hue doesn't
/// drop the stop's authored transparency.
pub fn parse_hex_alpha(s: &str) -> f32 {
    let Some(stripped) = s.trim().strip_prefix('#') else {
        return 1.0;
    };
    if stripped.len() != 8 {
        return 1.0;
    }
    u8::from_str_radix(&stripped[6..8], 16)
        .map(|a| a as f32 / 255.0)
        .unwrap_or(1.0)
}

/// Format RGB floats (0..1) as a `#rrggbb` hex string.
pub fn rgb_to_hex(r: f32, g: f32, b: f32) -> String {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    format!("#{:02x}{:02x}{:02x}", ch(r), ch(g), ch(b))
}
