//! Colour / fill / label primitives shared by every post-pass cluster.
//!
//! Ports of `hexLuminance`, `hasFill`, `hasVisibleFill`, `getFirstSolidColor`,
//! `needsLuminanceContrastOverride` and `resolveColorMaybeRef`, plus the tiny
//! node accessors (role / identity / semantic label / corner radius) and the
//! fill+stroke JSON constructors.

use super::*;

/// Perceived luminance 0..1 (port of `hexLuminance` — 0.299/0.587/0.114, NOT
/// the WCAG curve used for theme detection). `None` on a non-hex string.
pub(super) fn hex_luminance(hex: &str) -> Option<f64> {
    let h = hex.strip_prefix('#').unwrap_or(hex);
    // Reject non-ASCII BEFORE byte-slicing: a multi-byte char (e.g. "#héllo")
    // can pass the length check yet make `h[0..2]` land mid-codepoint → panic.
    if !h.is_ascii() || h.len() < 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()? as f64 / 255.0;
    let g = u8::from_str_radix(&h[2..4], 16).ok()? as f64 / 255.0;
    let b = u8::from_str_radix(&h[4..6], 16).ok()? as f64 / 255.0;
    Some(0.299 * r + 0.587 * g + 0.114 * b)
}

pub(super) fn hex_saturation(hex: &str) -> Option<f64> {
    let h = hex.strip_prefix('#').unwrap_or(hex);
    // Keep this guard in lockstep with `hex_luminance`: byte slicing below is
    // only valid once malformed / short / non-ASCII strings are rejected.
    if !h.is_ascii() || h.len() < 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()? as f64 / 255.0;
    let g = u8::from_str_radix(&h[2..4], 16).ok()? as f64 / 255.0;
    let b = u8::from_str_radix(&h[4..6], 16).ok()? as f64 / 255.0;
    let max = r.max(g).max(b);
    if max == 0.0 {
        return Some(0.0);
    }
    let min = r.min(g).min(b);
    Some((max - min) / max)
}

pub(super) fn preferred_foreground_for_bg(bg_hex: &str) -> &'static str {
    let Some(lum) = hex_luminance(bg_hex) else {
        return "#0F172A";
    };
    if lum < 0.5 {
        return "#FFFFFF";
    }
    if lum <= 0.72 && hex_saturation(bg_hex).is_some_and(|sat| sat >= 0.5) {
        return "#FFFFFF";
    }
    "#0F172A"
}

pub(super) fn fill_array(node: &Value) -> Option<&Vec<Value>> {
    node.get("fill").and_then(Value::as_array)
}

/// Any declared fill entry (visible or not). Port of `hasFill`.
pub(super) fn has_fill(node: &Value) -> bool {
    fill_array(node).map(|a| !a.is_empty()).unwrap_or(false)
}

/// True when the parent's fill field (passed down the walk) is non-empty.
pub(super) fn fill_present(fill: Option<&Value>) -> bool {
    fill.and_then(Value::as_array)
        .map(|a| !a.is_empty())
        .unwrap_or(false)
}

pub(super) fn is_invisible_color(color: &str) -> bool {
    let c = color.trim().to_lowercase();
    if c == "transparent" || c == "none" {
        return true;
    }
    // 8-digit hex with 00 alpha (#RRGGBB00) — valid but draws nothing.
    c.len() == 9 && c.starts_with('#') && c.ends_with("00")
}

pub(super) fn is_fill_invisible(fill: &Value) -> bool {
    if let Some(op) = fill.get("opacity").and_then(Value::as_f64) {
        if op <= 0.0 {
            return true;
        }
    }
    if fill.get("type").and_then(Value::as_str) == Some("solid") {
        if let Some(color) = fill.get("color").and_then(Value::as_str) {
            return is_invisible_color(color);
        }
    }
    false
}

/// Will the node paint a visible color? Port of `hasVisibleFill`.
pub(super) fn has_visible_fill(node: &Value) -> bool {
    let Some(arr) = fill_array(node) else {
        return false;
    };
    let Some(first) = arr.first() else {
        return false;
    };
    !is_fill_invisible(first)
}

/// First solid fill's color string. Port of `getFirstSolidColor`.
pub(super) fn get_first_solid_color(node: &Value) -> Option<String> {
    fill_array(node)?
        .iter()
        .find(|f| f.get("type").and_then(Value::as_str) == Some("solid"))
        .and_then(|f| f.get("color").and_then(Value::as_str))
        .map(str::to_string)
}

/// Resolve a `$color-*` ref to a hex — Rust has no doc-variable context at
/// sub-agent time, so refs resolve to `None` and the dependent fix is skipped
/// (port of `resolveColorMaybeRef`'s unresolved path).
pub(super) fn resolve_color_maybe_ref(color: &str) -> Option<String> {
    if color.trim_start().starts_with('$') {
        None
    } else {
        Some(color.to_string())
    }
}

/// A design-system token that always binds to a saturated, mid-dark colour
/// needing a WHITE foreground (accent / primary brand colour + the saturated
/// state colours). `$--color-warning` is deliberately excluded — it's often a
/// light amber that reads better with a dark foreground. Matches with or
/// without a trailing shade suffix (`$--primary`, `$--primary-600`).
pub(super) fn is_saturated_accent_token(color: &str) -> bool {
    let t = color.trim().trim_start_matches('$').to_ascii_lowercase();
    const ROOTS: &[&str] = &[
        "--primary",
        "--destructive",
        "--color-error",
        "--color-success",
    ];
    ROOTS.iter().any(|root| {
        t == *root
            || t.strip_prefix(root)
                .is_some_and(|rest| rest.starts_with('-'))
    })
}

/// |Δluminance| < 0.5 → too close, needs a contrast override (port of
/// `needsLuminanceContrastOverride`).
pub(super) fn needs_luminance_contrast_override(fg: &str, bg: &str) -> bool {
    match (hex_luminance(fg), hex_luminance(bg)) {
        (Some(f), Some(b)) => (f - b).abs() < 0.5,
        _ => false,
    }
}

pub(super) fn role_of(node: &Value) -> Option<&str> {
    node.get("role").and_then(Value::as_str)
}

pub(super) fn identity_label(node: &Value) -> String {
    let id = node.get("id").and_then(Value::as_str).unwrap_or("");
    let name = node.get("name").and_then(Value::as_str).unwrap_or("");
    let role = node.get("role").and_then(Value::as_str).unwrap_or("");
    format!("{id} {name} {role}").to_lowercase()
}

pub(super) fn semantic_label(node: &Value) -> String {
    let icon_font_name = node
        .get("iconFontName")
        .and_then(Value::as_str)
        .unwrap_or("");
    let placeholder = node
        .get("placeholder")
        .and_then(Value::as_str)
        .unwrap_or("");
    let value = node.get("value").and_then(Value::as_str).unwrap_or("");
    let content = node.get("content").and_then(Value::as_str).unwrap_or("");
    format!(
        "{} {icon_font_name} {placeholder} {value} {content}",
        identity_label(node)
    )
    .to_lowercase()
}

pub(super) fn corner_radius(node: &Value) -> f64 {
    match node.get("cornerRadius") {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::Array(a)) => a.first().and_then(Value::as_f64).unwrap_or(0.0),
        _ => 0.0,
    }
}

pub(super) fn solid_fill(color: &str) -> Value {
    json!([{ "type": "solid", "color": color }])
}

pub(super) fn neutral_stroke(color: &str) -> Value {
    json!({ "thickness": 1, "fill": solid_fill(color) })
}

pub(super) fn clear_visual_chrome(node: &mut Value) {
    if let Some(obj) = node.as_object_mut() {
        obj.remove("fill");
        obj.remove("stroke");
        obj.remove("effects");
        obj.remove("cornerRadius");
    }
}

// ── fixButtonForegroundContrast ──────────────────────────────────────────────
