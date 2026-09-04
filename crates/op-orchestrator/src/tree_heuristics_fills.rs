//! Fill passes: `inject_missing_nav_surface_fill` and
//! `strip_redundant_section_fill` (plus the role/hex tables they key off).

use super::*;

// ── inject_missing_nav_surface_fill (port of inject-nav-surface-fill.ts) ──────

// Surface-fill injection is for a BOTTOM nav / tab bar that floats over content
// and needs a surface to read against the page. A TOP header (`navbar` /
// `top-nav-bar` / `top-app-bar`) is intentionally transparent on mobile (the TS
// references), and `fix_structural_wrapper_transparency` already strips its
// "white-card-on-cream" fill — so they must NOT be re-filled here (that fight is
// exactly what re-boxed the mobile header). Desktop top-nav fills still come
// from `role_defaults::navbar`.
pub(super) const NAV_ROLES: &[&str] = &["nav", "tab-bar", "bottom-tab-bar", "tab-row"];
pub(super) const BOTTOM_NAV_ROLES: &[&str] = &["bottom-tab-bar"];

pub(super) fn apply_nav_surface_fill(nav: &mut Value, role: &str) -> bool {
    if has_renderable_fill(nav) {
        return false;
    }
    let Some(obj) = nav.as_object_mut() else {
        return false;
    };
    obj.insert(
        "fill".to_string(),
        json!([{ "type": "solid", "color": "$--card" }]),
    );
    // Lift shadow only when no effects were authored. Bottom nav → shadow
    // points up (offsetY < 0); top/other nav → down.
    let has_effects = obj
        .get("effects")
        .and_then(Value::as_array)
        .map(|e| !e.is_empty())
        .unwrap_or(false);
    if !has_effects {
        let is_bottom = BOTTOM_NAV_ROLES.contains(&role);
        obj.insert(
            "effects".to_string(),
            json!([{
                "type": "shadow", "offsetX": 0, "offsetY": if is_bottom { -4 } else { 4 },
                "blur": 12, "spread": 0, "color": "#0000000F"
            }]),
        );
    }
    true
}

/// Apply to one page-root child (a section / nav). Handles the nav-as-root
/// case AND the section-wrapping-a-single-nav case (one hop).
pub(super) fn inject_nav_surface_for_section(node: &mut Value) {
    let role = role_of(node).map(str::to_string);
    if let Some(r) = role.as_deref() {
        if NAV_ROLES.contains(&r) {
            apply_nav_surface_fill(node, r);
            return;
        }
        if r == "section" && child_count(node) == 1 {
            // wrapper section around a single nav child — one hop in.
            let inner_role = children_of(node)
                .first()
                .and_then(role_of)
                .map(str::to_string);
            if let Some(ir) = inner_role {
                if NAV_ROLES.contains(&ir.as_str()) {
                    if let Some(inner) = node
                        .get_mut("children")
                        .and_then(Value::as_array_mut)
                        .and_then(|a| a.first_mut())
                    {
                        apply_nav_surface_fill(inner, &ir);
                    }
                }
            }
        }
    }
}

// ── strip_redundant_section_fill (port of strip-redundant-section-fills.ts) ───

pub(super) const PROTECTED_ROLES: &[&str] = &[
    "card",
    "stat-card",
    "pricing-card",
    "feature-card",
    "image-card",
    "testimonial",
    "button",
    "icon-button",
    "badge",
    "chip",
    "tag",
    "pill",
    "input",
    "form-input",
    "search-bar",
    "phone-mockup",
    "banner",
    "metric-card",
    "gallery-item",
    "status-bar",
    "navbar",
    "nav",
    "tab-bar",
    "bottom-tab-bar",
    "top-nav-bar",
];
pub(super) const ATOMIC_PROTECTED_ROLES: &[&str] = &[
    "button",
    "icon-button",
    "badge",
    "chip",
    "tag",
    "pill",
    "input",
    "form-input",
    "search-bar",
];
pub(super) const PRIMARY_ATOMIC_ROLES: &[&str] = &["input", "form-input", "search-bar"];
pub(super) const STRUCTURAL_ROLES: &[&str] = &[
    "section",
    "row",
    "column",
    "stack",
    "container",
    "content-area",
    "section-header",
    "wrapper",
    "group",
    "hero",
    "footer",
    "cta-section",
    "stats-section",
];
pub(super) const CONTAINER_PROTECTED_ROLES: &[&str] = &[
    "card",
    "stat-card",
    "pricing-card",
    "feature-card",
    "image-card",
    "testimonial",
    "banner",
    "metric-card",
    "gallery-item",
    "phone-mockup",
];
pub(super) const SAFE_DARK_HEXES: &[&str] = &[
    "#000000", "#000", "#0a0a0a", "#0f0f0f", "#111", "#111111", "#121212", "#141414", "#1a1a1a",
    "#181818", "#1c1c1c", "#1e1e1e", "#202020",
];
pub(super) const SAFE_LIGHT_HEXES: &[&str] = &[
    "#ffffff", "#fff", "#fefefe", "#fdfdfd", "#fcfcfc", "#fafafa", "#f9fafb", "#f8f8f8", "#f8fafc",
    "#f5f5f5", "#f4f4f5", "#f3f4f6",
];

pub(super) fn normalize_hex(color: &str) -> String {
    let mut c = color.trim().to_lowercase();
    if c.len() == 9 && c.starts_with('#') {
        c.truncate(7);
    }
    c
}

pub(super) fn has_multiple_same_role_children(node: &Value, parent_role: &str) -> bool {
    if !CONTAINER_PROTECTED_ROLES.contains(&parent_role) {
        return false;
    }
    let mut n = 0;
    for c in children_of(node) {
        if role_of(c) == Some(parent_role) {
            n += 1;
            if n >= 2 {
                return true;
            }
        }
    }
    false
}

pub(super) fn has_nested_filled_component(node: &Value, parent_role: &str) -> bool {
    if !ATOMIC_PROTECTED_ROLES.contains(&parent_role) {
        return false;
    }
    for c in children_of(node) {
        let Some(cr) = role_of(c) else { continue };
        if cr == parent_role {
            return true;
        }
        if PRIMARY_ATOMIC_ROLES.contains(&cr) && first_solid_color(c).is_some() {
            return true;
        }
    }
    false
}

pub(super) fn is_section_level_frame(node: &Value) -> bool {
    let Some(role) = role_of(node) else {
        return true; // unrolled section root
    };
    if PROTECTED_ROLES.contains(&role) {
        if has_nested_filled_component(node, role) {
            return true;
        }
        if has_multiple_same_role_children(node, role) {
            return true;
        }
        return false;
    }
    if STRUCTURAL_ROLES.contains(&role) {
        return true;
    }
    false
}

pub(super) fn should_strip_fill(child_fill: &str, root_fill: Option<&str>) -> bool {
    let child_key = normalize_hex(child_fill);
    if let Some(rf) = root_fill {
        if child_key == normalize_hex(rf) {
            return true;
        }
    }
    SAFE_DARK_HEXES.contains(&child_key.as_str()) || SAFE_LIGHT_HEXES.contains(&child_key.as_str())
}

/// Check if a node looks like a screen root (mobile artboard).
/// A screen root is the ground level for content, so its fill is never "redundant" —
/// nothing is behind it to paint. Screen roots are identified by:
/// - Width 300-480 (mobile range)
/// - Height >= 480 (numeric) OR "fit_content" (potential generated mobile root)
fn looks_like_screen_root(node: &Value) -> bool {
    let width = node.get("width").and_then(Value::as_f64).unwrap_or(0.0);
    let height_value = node.get("height");

    // Mobile width range: 300-480.
    if !(300.0..=480.0).contains(&width) {
        return false;
    }

    // Check height: numeric >= 480 OR the string "fit_content".
    if let Some(h) = height_value.and_then(Value::as_f64) {
        return h >= 480.0;
    }
    if let Some(h) = height_value.and_then(Value::as_str) {
        return h == "fit_content";
    }
    false
}

/// Per-section decision (the body of the TS loop, applied to one page-root
/// child). `page_bg` is the page root's fill hex when known.
///
/// **Invariant:** A screen root's fill is never redundant — it is the ground level
/// for content, and nothing is behind it to paint. Only strip fills from true
/// sections (content wrappers), not from screen roots (mobile artboards).
pub(super) fn strip_redundant_section_fill(node: &mut Value, page_bg: Option<&str>) {
    if node.get("type").and_then(Value::as_str) != Some("frame") {
        return;
    }
    // Do not strip a screen root's fill — it is the ground level, so the fill is never
    // redundant. A section wrapper's fill might be redundant with the page background,
    // but a mobile artboard (screen root) must retain its background.
    if looks_like_screen_root(node) {
        return;
    }
    if !is_section_level_frame(node) {
        return;
    }
    let Some(child_fill) = first_solid_color(node) else {
        return;
    };
    if should_strip_fill(&child_fill, page_bg) {
        if let Some(obj) = node.as_object_mut() {
            obj.remove("fill");
            obj.remove("stroke");
            obj.remove("cornerRadius");
        }
    }
}
