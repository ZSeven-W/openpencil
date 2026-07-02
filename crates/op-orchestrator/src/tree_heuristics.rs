//! Post-streaming tree heuristics — a faithful port of the cleanup passes
//! TS runs in `applyPostStreamingTreeHeuristics` (design-canvas-ops.ts) via
//! `packages/pen-core/src/layout/*.ts`. These run on the ASSEMBLED page in TS;
//! the Rust orchestrator runs the role pipeline PER-SUBTASK, and each subtask
//! forest root is a direct child of the page root — i.e. a section. So the
//! per-section passes here are applied to each forest root, which reproduces
//! the TS effect (iterating the page root's children) without a page-assembly
//! hook.
//!
//! Ported passes (HIGH/MED visual impact per the 2026-06-19 TS↔Rust parity
//! audit — the Rust port had NONE of these, degrading every model incl. Opus):
//!   - `inject_missing_nav_surface_fill` — anchor a transparent nav bar with a
//!     surface fill + lift shadow (port of inject-nav-surface-fill.ts).
//!   - `strip_redundant_section_fill` — drop a section wrapper's hedge fill
//!     (safe-dark / safe-light hex) + the wrapper chrome that travels with it
//!     (port of strip-redundant-section-fills.ts). Matches HEX, so it must run
//!     BEFORE variable binding.
//!   - `strip_nested_card_decoration` — strip a nested card's redundant
//!     stroke / cornerRadius / shadow when an ancestor already carries it
//!     (port of strip-nested-card-decoration.ts) — kills box-in-box borders.

use jian_ops_schema::node::PenNode;
use serde_json::{json, Value};

// ── shared tiny helpers (self-contained; mirror role_post_pass) ──────────────

fn role_of(node: &Value) -> Option<&str> {
    node.get("role").and_then(Value::as_str)
}

fn first_solid_color(node: &Value) -> Option<String> {
    let fill = node.get("fill")?;
    if let Some(s) = fill.as_str() {
        return Some(s.to_string());
    }
    let arr = fill.as_array()?;
    let first = arr.first()?;
    if first.get("type").and_then(Value::as_str) == Some("solid") {
        return first
            .get("color")
            .and_then(Value::as_str)
            .map(str::to_string);
    }
    None
}

/// Will the first fill paint a visible color? (port of `hasAnyFill`). A truthy
/// `type` alone isn't enough — `{type:solid}` with no/empty color, or an
/// unknown variant, render as transparent and count as "no fill".
fn has_renderable_fill(node: &Value) -> bool {
    let Some(fill) = node.get("fill") else {
        return false;
    };
    if let Some(s) = fill.as_str() {
        return !s.is_empty();
    }
    let Some(arr) = fill.as_array() else {
        return false;
    };
    let Some(first) = arr.first() else {
        return false;
    };
    match first.get("type").and_then(Value::as_str) {
        Some("solid") => first
            .get("color")
            .and_then(Value::as_str)
            .map(|c| !c.is_empty())
            .unwrap_or(false),
        Some("linear_gradient") | Some("radial_gradient") => first
            .get("stops")
            .and_then(Value::as_array)
            .map(|s| !s.is_empty())
            .unwrap_or(false),
        Some("image") => first
            .get("src")
            .and_then(Value::as_str)
            .map(|s| !s.is_empty())
            .unwrap_or(false),
        _ => false,
    }
}

fn children_of(node: &Value) -> &[Value] {
    node.get("children")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn child_count(node: &Value) -> usize {
    children_of(node).len()
}

// ── inject_missing_nav_surface_fill (port of inject-nav-surface-fill.ts) ──────

// Surface-fill injection is for a BOTTOM nav / tab bar that floats over content
// and needs a surface to read against the page. A TOP header (`navbar` /
// `top-nav-bar` / `top-app-bar`) is intentionally transparent on mobile (the TS
// references), and `fix_structural_wrapper_transparency` already strips its
// "white-card-on-cream" fill — so they must NOT be re-filled here (that fight is
// exactly what re-boxed the mobile header). Desktop top-nav fills still come
// from `role_defaults::navbar`.
const NAV_ROLES: &[&str] = &["nav", "tab-bar", "bottom-tab-bar", "tab-row"];
const BOTTOM_NAV_ROLES: &[&str] = &["bottom-tab-bar"];

fn apply_nav_surface_fill(nav: &mut Value, role: &str) -> bool {
    if has_renderable_fill(nav) {
        return false;
    }
    let Some(obj) = nav.as_object_mut() else {
        return false;
    };
    obj.insert(
        "fill".to_string(),
        json!([{ "type": "solid", "color": "$color-surface" }]),
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
fn inject_nav_surface_for_section(node: &mut Value) {
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

const PROTECTED_ROLES: &[&str] = &[
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
const ATOMIC_PROTECTED_ROLES: &[&str] = &[
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
const PRIMARY_ATOMIC_ROLES: &[&str] = &["input", "form-input", "search-bar"];
const STRUCTURAL_ROLES: &[&str] = &[
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
const CONTAINER_PROTECTED_ROLES: &[&str] = &[
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
const SAFE_DARK_HEXES: &[&str] = &[
    "#000000", "#000", "#0a0a0a", "#0f0f0f", "#111", "#111111", "#121212", "#141414", "#1a1a1a",
    "#181818", "#1c1c1c", "#1e1e1e", "#202020",
];
const SAFE_LIGHT_HEXES: &[&str] = &[
    "#ffffff", "#fff", "#fefefe", "#fdfdfd", "#fcfcfc", "#fafafa", "#f9fafb", "#f8f8f8", "#f8fafc",
    "#f5f5f5", "#f4f4f5", "#f3f4f6",
];

fn normalize_hex(color: &str) -> String {
    let mut c = color.trim().to_lowercase();
    if c.len() == 9 && c.starts_with('#') {
        c.truncate(7);
    }
    c
}

fn has_multiple_same_role_children(node: &Value, parent_role: &str) -> bool {
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

fn has_nested_filled_component(node: &Value, parent_role: &str) -> bool {
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

fn is_section_level_frame(node: &Value) -> bool {
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

fn should_strip_fill(child_fill: &str, root_fill: Option<&str>) -> bool {
    let child_key = normalize_hex(child_fill);
    if let Some(rf) = root_fill {
        if child_key == normalize_hex(rf) {
            return true;
        }
    }
    SAFE_DARK_HEXES.contains(&child_key.as_str()) || SAFE_LIGHT_HEXES.contains(&child_key.as_str())
}

/// Per-section decision (the body of the TS loop, applied to one page-root
/// child). `page_bg` is the page root's fill hex when known.
fn strip_redundant_section_fill(node: &mut Value, page_bg: Option<&str>) {
    if node.get("type").and_then(Value::as_str) != Some("frame") {
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

// ── strip_nested_card_decoration (port of strip-nested-card-decoration.ts) ────

const KEEP_DECORATION_ROLES: &[&str] = &[
    "button",
    "icon-button",
    "fab",
    "tag",
    "chip",
    "badge",
    "status-badge",
    "pill",
    "input",
    "search-bar",
    "form-field",
    "textarea",
    "select",
    "combobox",
    "avatar",
    "avatar-stack",
    "switch",
    "checkbox",
    "radio",
    "toolbar",
    "segmented-control",
];
const MEDIA_CLIP_ROLES: &[&str] = &[
    "image",
    "image-card",
    "image-placeholder",
    "video",
    "video-placeholder",
    "media",
    "media-thumbnail",
    "thumbnail",
    "cover",
    "cover-image",
    "gallery-item",
];

#[derive(Clone, Copy, Default)]
struct DecoFlags {
    stroke: bool,
    corner: bool,
    shadow: bool,
}

fn read_decoration(node: &Value) -> DecoFlags {
    let stroke = node
        .get("stroke")
        .and_then(|s| s.get("thickness"))
        .and_then(Value::as_f64)
        .map(|t| t > 0.0)
        .unwrap_or(false);
    let corner = match node.get("cornerRadius") {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0) > 0.0,
        Some(Value::Array(a)) => a.first().and_then(Value::as_f64).unwrap_or(0.0) > 0.0,
        _ => false,
    };
    let shadow = node
        .get("effects")
        .and_then(Value::as_array)
        .map(|e| {
            e.iter()
                .any(|x| x.get("type").and_then(Value::as_str) == Some("shadow"))
        })
        .unwrap_or(false);
    DecoFlags {
        stroke,
        corner,
        shadow,
    }
}

fn role_lower(node: &Value) -> String {
    role_of(node).unwrap_or("").to_lowercase()
}

fn is_role_protected(node: &Value) -> bool {
    KEEP_DECORATION_ROLES.contains(&role_lower(node).as_str())
}

fn is_media_clipper(node: &Value) -> bool {
    let role = role_lower(node);
    if MEDIA_CLIP_ROLES.contains(&role.as_str()) {
        return true;
    }
    if node.get("clipContent").and_then(Value::as_bool) != Some(true) {
        return false;
    }
    children_of(node).iter().any(|c| {
        c.get("type").and_then(Value::as_str) == Some("image")
            || MEDIA_CLIP_ROLES.contains(&role_lower(c).as_str())
    })
}

fn strip_nested_card_decoration(node: &mut Value, ancestor: DecoFlags) {
    let is_frame = node.get("type").and_then(Value::as_str) == Some("frame");
    let mut next_ancestor = ancestor;
    if is_frame {
        let own = read_decoration(node);
        if !is_role_protected(node) {
            let media = is_media_clipper(node);
            if let Some(obj) = node.as_object_mut() {
                if own.stroke && ancestor.stroke {
                    obj.remove("stroke");
                }
                if own.corner && ancestor.corner && !media {
                    obj.remove("cornerRadius");
                }
                if own.shadow && ancestor.shadow {
                    obj.remove("effects");
                }
            }
        }
        // Accumulate THIS node's (original) decoration for descendants.
        next_ancestor = DecoFlags {
            stroke: ancestor.stroke || own.stroke,
            corner: ancestor.corner || own.corner,
            shadow: ancestor.shadow || own.shadow,
        };
    }
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children.iter_mut() {
            strip_nested_card_decoration(child, next_ancestor);
        }
    }
}

// ── fix_invisible_text_band (glm banner-fill floor, not a TS pass) ───────────
//
// glm intermittently designs a promo banner with WHITE text but omits the
// colored/gradient fill it intended (gen2/3/5: "Get 30% Off" white-on-cream =
// invisible). Deterministic floor: on a LIGHT page, a fill-less container whose
// text descendants are ALL white/light → it was meant to sit on a colored
// surface → stamp `$color-accent` so the copy becomes readable. Conservative:
// fires only when every text is light (no dark text to contradict) and the
// container truly has no renderable fill.

// Light text tokens/hexes that vanish on a light page — white + the neutral
// surface tints (a banner headline written in any of these needs a colored
// surface beneath it).
const LIGHT_TEXT_REFS: &[&str] = &[
    "$color-surface",
    "$color-surface-2",
    "$color-surface-3",
    "$color-bg-deep",
];
const LIGHT_TEXT_HEXES: &[&str] = &["#ffffff", "#fff", "#fefefe", "#fdfdfd", "white"];

fn is_light_text(color: &str) -> bool {
    if LIGHT_TEXT_REFS.contains(&color) {
        return true;
    }
    LIGHT_TEXT_HEXES.contains(&normalize_hex(color).as_str())
}

/// Tally text colors that sit DIRECTLY on this container's (unfilled) surface.
/// Do NOT descend into a child that carries its own renderable fill — a button
/// / avatar / chip has its own surface, so its text colour says nothing about
/// whether THIS container needs a fill (e.g. a promo banner's white headline +
/// an orange-text "Order Now" button on a white pill: only the headline counts).
fn tally_surface_text_colors(node: &Value, light: &mut usize, dark: &mut usize) {
    for child in children_of(node) {
        if child.get("type").and_then(Value::as_str) == Some("text") {
            if let Some(c) = first_solid_color(child) {
                if is_light_text(&c) {
                    *light += 1;
                } else {
                    *dark += 1;
                }
            }
        } else if !has_renderable_fill(child) {
            tally_surface_text_colors(child, light, dark);
        }
    }
}

/// The emphasis color the design ACTUALLY uses (glm often repurposes a chart
/// token as the brand accent because the palette's `$color-accent` defaults to
/// blue — wrong for a warm app). Returns the first chart/accent/primary token
/// found in the subtree, so the band matches the rest of the screen instead of
/// stamping a clashing default blue.
fn find_design_accent(node: &Value) -> Option<String> {
    if let Some(c) = first_solid_color(node) {
        if c == "$color-accent"
            || c == "$color-primary"
            || c == "$color-brand"
            || c.starts_with("$color-chart-")
        {
            return Some(c);
        }
    }
    for child in children_of(node) {
        if let Some(c) = find_design_accent(child) {
            return Some(c);
        }
    }
    None
}

/// True when a solid fill color reads as a light page/surface tone — light/white
/// text on it is invisible. Covers the neutral surface variable refs (binding
/// hasn't resolved them at this pass) plus white-ish hexes.
fn is_light_surface_color(color: &str) -> bool {
    matches!(
        color,
        "$color-surface" | "$color-surface-2" | "$color-surface-3" | "$color-bg-deep"
    ) || SAFE_LIGHT_HEXES.contains(&normalize_hex(color).as_str())
}

fn fix_invisible_text_band(node: &mut Value, light_theme: bool, design_accent: &str) {
    if !light_theme {
        return; // white text on a dark page is fine
    }
    if node.get("type").and_then(Value::as_str) != Some("frame") {
        return;
    }
    // Skip only when the node ALREADY paints a non-light surface (a colored or
    // dark solid, or a gradient / image) — light text reads fine there. A node
    // with NO fill, OR a LIGHT-SURFACE solid fill (`$color-surface`, white), is a
    // band where light text is invisible. The latter is the broken-promo-banner
    // case: glm gives the card white text + a dark CTA + a translucent-white
    // badge (all implying a colored background) yet fills the card with
    // `$color-surface`, so the headline vanishes. Repaint with the design accent.
    if has_renderable_fill(node) {
        match first_solid_color(node) {
            Some(c) if is_light_surface_color(&c) => {} // light surface → still invisible
            _ => return, // colored/dark solid or gradient → real surface, text fine
        }
    }
    let (mut light, mut dark) = (0usize, 0usize);
    tally_surface_text_colors(node, &mut light, &mut dark);
    if light >= 1 && dark == 0 {
        if let Some(obj) = node.as_object_mut() {
            obj.insert(
                "fill".to_string(),
                json!([{ "type": "solid", "color": design_accent }]),
            );
        }
    }
}

/// The design's DOMINANT accent token across already-generated siblings — glm
/// uses a chart token as the de-facto brand accent (the palette's
/// `$color-accent` often defaults to a clashing blue). Counting across the
/// assembled-so-far page (passed by the caller from the doc sink) picks e.g.
/// `$color-chart-6` when it's used 9× vs `$color-accent` 1×, so an injected
/// banner band matches the rest of the screen.
pub fn dominant_design_accent(nodes: &[PenNode]) -> Option<String> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for n in nodes {
        if let Ok(v) = serde_json::to_value(n) {
            tally_accent(&v, &mut counts);
        }
    }
    counts.into_iter().max_by_key(|(_, n)| *n).map(|(c, _)| c)
}

fn tally_accent(node: &Value, counts: &mut Vec<(String, usize)>) {
    if let Some(c) = first_solid_color(node) {
        if c == "$color-accent"
            || c == "$color-primary"
            || c == "$color-brand"
            || c.starts_with("$color-chart-")
        {
            if let Some(e) = counts.iter_mut().find(|(k, _)| *k == c) {
                e.1 += 1;
            } else {
                counts.push((c, 1));
            }
        }
    }
    for child in children_of(node) {
        tally_accent(child, counts);
    }
}

// ── clip_card_image_corners (port of the TS pass) ────────────────────────────
//
// A card-shape frame (scalar cornerRadius > 0) whose FIRST child is a
// full-width image carrying its OWN scalar cornerRadius renders the image's
// bottom corners rounded inside the card while the title below sits flush —
// the ragged look. Enforce `clipContent:true` on the card (its outer radius
// clips the image) + drop the image's own radius. Port of
// clip-card-image-corners.ts.
fn is_image_node(node: &Value) -> bool {
    match node.get("type").and_then(Value::as_str) {
        Some("image") => true,
        Some("frame") => role_of(node) == Some("image-placeholder"),
        _ => false,
    }
}

/// A card's leading header image authored at a FIXED width narrower than the
/// card leaves a white gap beside it — a 160 px image inside a 252 px dish
/// card fills only ~63 % of the width. A vertical card's header image should
/// span the full card width, so set the leading image to
/// `width: fill_container`. The renderer's object-fit cover (parity with TS)
/// keeps the photo from distorting once it widens.
///
/// Scoped tightly to avoid touching avatars / logos / inline thumbnails: only
/// a LARGE (>= 80 px) leading image, in a `vertical` container that is NOT
/// center-aligned (centered leading images are avatars/logos, not full-bleed
/// headers), with at least one sibling below it.
fn fill_card_leading_image_width(node: &mut Value) {
    let is_container = matches!(
        node.get("type").and_then(Value::as_str),
        Some("frame") | Some("group")
    );
    let is_vertical = node.get("layout").and_then(Value::as_str) == Some("vertical");
    let is_centered = node.get("alignItems").and_then(Value::as_str) == Some("center");
    if is_container && is_vertical && !is_centered && child_count(node) >= 2 {
        let lead_is_wide_fixed_image = children_of(node)
            .first()
            .map(|c| {
                is_image_node(c)
                    && c.get("width")
                        .and_then(Value::as_f64)
                        .map(|w| w >= 80.0)
                        .unwrap_or(false)
            })
            .unwrap_or(false);
        if lead_is_wide_fixed_image {
            if let Some(first) = node
                .get_mut("children")
                .and_then(Value::as_array_mut)
                .and_then(|a| a.first_mut())
                .and_then(Value::as_object_mut)
            {
                first.insert("width".to_string(), json!("fill_container"));
            }
        }
    }
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children.iter_mut() {
            fill_card_leading_image_width(child);
        }
    }
}

// ── fix_notification_badge_overlay ───────────────────────────────────────

/// A small, roughly-square icon button whose two children are exactly one icon
/// glyph and one tiny filled dot — the notification-badge pattern.
fn is_small_icon_button(node: &Value) -> bool {
    if node.get("type").and_then(Value::as_str) != Some("frame") {
        return false;
    }
    let (Some(w), Some(h)) = (
        node.get("width").and_then(Value::as_f64),
        node.get("height").and_then(Value::as_f64),
    ) else {
        return false;
    };
    w <= 56.0 && h <= 56.0 && (w - h).abs() <= 12.0 && child_count(node) == 2
}

fn is_icon_glyph(node: &Value) -> bool {
    node.get("type").and_then(Value::as_str) == Some("icon_font") || is_image_node(node)
}

/// A tiny (<= 14 px), roughly-square, filled, childless shape — a badge dot.
fn is_badge_dot(node: &Value) -> bool {
    if !matches!(
        node.get("type").and_then(Value::as_str),
        Some("frame") | Some("rectangle") | Some("ellipse")
    ) {
        return false;
    }
    let (Some(w), Some(h)) = (
        node.get("width").and_then(Value::as_f64),
        node.get("height").and_then(Value::as_f64),
    ) else {
        return false;
    };
    let has_fill = node
        .get("fill")
        .and_then(Value::as_array)
        .map(|a| !a.is_empty())
        .unwrap_or(false);
    w <= 14.0 && h <= 14.0 && (w - h).abs() <= 4.0 && has_fill && children_of(node).is_empty()
}

/// A notification badge authored as a flex SIBLING of the icon inside a small
/// icon button renders BESIDE the icon (a square dot to its left, per the live
/// `Notification Button` = horizontal frame > [8×8 dot, bell]) instead of
/// overlaying the icon's top-right corner. Detect the pattern and convert it to
/// a corner-overlay badge: the button becomes `layout: none`, the icon is
/// centered, and the dot is rounded to a circle (square dots only — ellipses
/// are already round) and pinned to the icon's top-right corner.
fn fix_notification_badge_overlay(node: &mut Value) {
    if is_small_icon_button(node) {
        let kids = children_of(node);
        let mut icon_i: Option<usize> = None;
        let mut dot_i: Option<usize> = None;
        for (i, c) in kids.iter().enumerate() {
            if is_icon_glyph(c) {
                icon_i = Some(i);
            } else if is_badge_dot(c) {
                dot_i = Some(i);
            }
        }
        if let (Some(ii), Some(di)) = (icon_i, dot_i) {
            let bw = node.get("width").and_then(Value::as_f64).unwrap_or(0.0);
            let bh = node.get("height").and_then(Value::as_f64).unwrap_or(0.0);
            let iw = kids[ii].get("width").and_then(Value::as_f64).unwrap_or(0.0);
            let ih = kids[ii]
                .get("height")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let dw = kids[di].get("width").and_then(Value::as_f64).unwrap_or(0.0);
            let icon_x = ((bw - iw) * 0.5).max(0.0);
            let icon_y = ((bh - ih) * 0.5).max(0.0);
            let dot_x = (icon_x + iw - dw * 0.5).min((bw - dw).max(0.0));
            let dot_y = (icon_y - dw * 0.5).max(0.0);
            if let Some(obj) = node.as_object_mut() {
                obj.insert("layout".to_string(), json!("none"));
                obj.remove("gap");
            }
            if let Some(arr) = node.get_mut("children").and_then(Value::as_array_mut) {
                if let Some(icon) = arr.get_mut(ii).and_then(Value::as_object_mut) {
                    icon.insert("x".to_string(), json!(icon_x));
                    icon.insert("y".to_string(), json!(icon_y));
                }
                if let Some(dot) = arr.get_mut(di).and_then(Value::as_object_mut) {
                    dot.insert("x".to_string(), json!(dot_x));
                    dot.insert("y".to_string(), json!(dot_y));
                    if dot.get("type").and_then(Value::as_str) != Some("ellipse") {
                        dot.insert("cornerRadius".to_string(), json!(dw * 0.5));
                    }
                }
            }
        }
    }
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children.iter_mut() {
            fix_notification_badge_overlay(child);
        }
    }
}

fn clip_card_image_corners(node: &mut Value) {
    let is_container = matches!(
        node.get("type").and_then(Value::as_str),
        Some("frame") | Some("group")
    );
    if is_container {
        let radius_ok = node
            .get("cornerRadius")
            .and_then(Value::as_f64)
            .map(|r| r > 0.0)
            .unwrap_or(false);
        let first_image_with_radius = children_of(node)
            .first()
            .map(|c| {
                is_image_node(c)
                    && c.get("cornerRadius")
                        .and_then(Value::as_f64)
                        .map(|r| r > 0.0)
                        .unwrap_or(false)
            })
            .unwrap_or(false);
        if radius_ok && child_count(node) >= 2 && first_image_with_radius {
            if let Some(obj) = node.as_object_mut() {
                obj.insert("clipContent".to_string(), json!(true));
            }
            if let Some(first) = node
                .get_mut("children")
                .and_then(Value::as_array_mut)
                .and_then(|a| a.first_mut())
                .and_then(Value::as_object_mut)
            {
                first.remove("cornerRadius");
            }
        }
    }
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children.iter_mut() {
            clip_card_image_corners(child);
        }
    }
}

// ── convert_stacked_overlay_to_absolute (port of the TS pass) ────────────────
//
// Hero/banner overlay: a `layout:vertical` frame with a NUMERIC height whose
// children include ≥2 full-height bg-like nodes (a full-bleed image + a
// gradient-overlay rect/frame) that the model intends to LAYER, not stack.
// Vertical layout sequences them instead → the image + overlay collapse to
// h=0 and the content overflows the section (the broken "Featured Restaurants"
// promo card: Restaurant Image h=0, Card Details spilling below the section).
// Switch to `layout:none` so children stack at their own x/y (0,0 default),
// which is the layered intent. Conservative: requires EXPLICIT vertical +
// numeric height + ≥2 same-height/fill bg children (false positives are worse
// than misses). Port of convert-stacked-overlay-to-absolute.ts.
fn convert_stacked_overlay_to_absolute(node: &mut Value) {
    if node.get("type").and_then(Value::as_str) == Some("frame")
        && node.get("layout").and_then(Value::as_str) == Some("vertical")
    {
        if let Some(h) = node.get("height").and_then(Value::as_f64) {
            let mut bg_like = 0;
            for child in children_of(node) {
                let t = child.get("type").and_then(Value::as_str);
                if !matches!(t, Some("image") | Some("rectangle") | Some("frame")) {
                    continue;
                }
                let ch = child.get("height");
                let matches_h = ch.and_then(Value::as_f64).map(|x| x == h).unwrap_or(false)
                    || ch.and_then(Value::as_str) == Some("fill_container");
                if matches_h {
                    bg_like += 1;
                }
            }
            if bg_like >= 2 {
                if let Some(obj) = node.as_object_mut() {
                    obj.insert("layout".to_string(), json!("none"));
                }
            }
        }
    }
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children.iter_mut() {
            convert_stacked_overlay_to_absolute(child);
        }
    }
}

// ── forest entry ─────────────────────────────────────────────────────────────

/// Run the post-streaming heuristics over a subtask forest. Each root is a
/// page-root child (a section). MUST run BEFORE variable binding —
/// `strip_redundant_section_fill` matches literal hedge HEX that binding would
/// otherwise convert to `$color-*` refs. `page_bg` is the plan root frame's
/// fill hex when known (enables the "section fill == page bg" strip case).
/// A bottom-tab-bar's ACTIVE tab is frequently authored as a solid-filled frame
/// with NO `cornerRadius` — a sharp rectangle that pokes out of the rounded nav
/// pill (the "active block is an unreasonable square overflowing the navbar" the
/// user flagged). The TS references render the active item as a rounded pill.
/// Round any filled tab-item frame inside a bottom-tab-bar so it reads as a
/// contained highlight; `cornerRadius: 999` lets the renderer clamp to height/2
/// → a pill. The bar / pill container itself (already carrying a radius) and the
/// inactive tabs (no fill) are untouched.
/// Roles / structures that identify a bottom navigation bar. The role set is
/// broad (raw-JSON models tag the bar `bottom-tab-bar`; others use `tab-bar` /
/// `nav` / `tab-row`), and as a fallback ANY horizontal container whose children
/// are nav tab items (`nav-item` / `nav-item-active`, the manifest
/// element-builder roles) counts — so the active-tab rounding fires regardless
/// of which generation path produced the bar.
const NAV_BAR_ROLES: &[&str] = &[
    "bottom-tab-bar",
    "tab-bar",
    "nav",
    "tab-row",
    "navbar",
    "bottom-nav",
];

fn is_nav_bar_container(node: &Value) -> bool {
    if NAV_BAR_ROLES.contains(&role_of(node).unwrap_or_default()) {
        return true;
    }
    let horizontal = node.get("layout").and_then(Value::as_str) == Some("horizontal");
    horizontal
        && node
            .get("children")
            .and_then(Value::as_array)
            .map(|kids| {
                kids.iter()
                    .any(|c| matches!(role_of(c), Some("nav-item") | Some("nav-item-active")))
            })
            .unwrap_or(false)
}

fn round_active_nav_tab(node: &mut Value, accent: &str) {
    if is_nav_bar_container(node) {
        // A rounded-pill bar MUST clip its children — a full-height active block
        // that pokes out past the pill's rounded corners gets confined.
        let bar_radius = node
            .get("cornerRadius")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        if bar_radius >= 8.0 {
            if let Some(obj) = node.as_object_mut() {
                obj.insert("clipContent".to_string(), Value::Bool(true));
            }
        }
        round_nav_tab_items(node, true, accent);
    } else if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children {
            round_active_nav_tab(child, accent);
        }
    }
}

fn round_nav_tab_items(node: &mut Value, is_bar_root: bool, accent: &str) {
    // Cover both `frame` tab items AND a bare `rectangle` highlight behind the
    // icon/label — either is a sharp overflowing square if left un-rounded.
    let is_block = matches!(
        node.get("type").and_then(Value::as_str),
        Some("frame") | Some("rectangle")
    );
    let role = role_of(node).map(str::to_string).unwrap_or_default();
    let is_active_tab = role == "nav-item-active";
    let has_fill = node
        .get("fill")
        .and_then(Value::as_array)
        .map(|a| !a.is_empty())
        .unwrap_or(false);
    let is_full_row = node.get("width").and_then(Value::as_str) == Some("fill_container")
        && node.get("layout").and_then(Value::as_str) == Some("horizontal");
    let radius = node
        .get("cornerRadius")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    if !is_bar_root && is_block && !is_full_row {
        if is_active_tab {
            // The active tab MUST read as a CONTAINED rounded pill. Round it, and
            // if it carries no visible highlight fill, inject the design accent so
            // the active state is a proper orange pill (the manifest element
            // builder leaves `nav-item-active` unfilled; glm sometimes gives it a
            // sharp filled square). Either path → rounded accent pill.
            if radius < 8.0 {
                node["cornerRadius"] = Value::from(999.0);
            }
            if !has_renderable_fill(node) {
                node["fill"] = json!([{ "type": "solid", "color": accent }]);
            }
        } else if has_fill && radius < 8.0 {
            node["cornerRadius"] = Value::from(999.0);
        }
    }
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children {
            round_nav_tab_items(child, false, accent);
        }
    }
}

/// A model authors an "active background" as a FLEX SIBLING: a fill-less
/// container whose first child is an EMPTY `fill_container` × `fill_container`
/// rectangle/frame carrying the fill. In flex flow that backdrop is a real
/// item — it eats half the row and shoves the actual content sideways
/// (measured: a sidebar nav item's icon+label pushed outside the 260px rail by
/// its own "Active Bg" rectangle). The fill/cornerRadius belong on the
/// CONTAINER: move them there and delete the backdrop child. The double-fill
/// empty box is unambiguous — a divider is thin (one fixed axis), a spacer
/// carries no fill, an avatar placeholder has a fixed size.
fn merge_backdrop_child_fill(v: &mut Value) {
    let is_backdrop = |c: &Value| -> bool {
        matches!(
            c.get("type").and_then(Value::as_str),
            Some("rectangle" | "frame")
        ) && c
            .get("children")
            .and_then(Value::as_array)
            .map(|k| k.is_empty())
            .unwrap_or(true)
            && c.get("width").and_then(Value::as_str) == Some("fill_container")
            && c.get("height").and_then(Value::as_str) == Some("fill_container")
            && c.get("fill")
                .and_then(Value::as_array)
                .is_some_and(|f| !f.is_empty())
    };
    let container_has_fill = v
        .get("fill")
        .and_then(Value::as_array)
        .is_some_and(|f| !f.is_empty())
        || v.get("fill").and_then(Value::as_str).is_some();
    let eligible = v.get("type").and_then(Value::as_str) == Some("frame")
        && !container_has_fill
        && v.get("children")
            .and_then(Value::as_array)
            .is_some_and(|kids| kids.len() >= 2 && is_backdrop(&kids[0]));
    if eligible {
        if let Some(obj) = v.as_object_mut() {
            if let Some(Value::Array(mut kids)) = obj.remove("children") {
                let backdrop = kids.remove(0);
                if let Some(fill) = backdrop.get("fill").cloned() {
                    obj.insert("fill".into(), fill);
                }
                if obj.get("cornerRadius").is_none() {
                    if let Some(radius) = backdrop.get("cornerRadius").cloned() {
                        obj.insert("cornerRadius".into(), radius);
                    }
                }
                obj.insert("children".into(), Value::Array(kids));
            }
        }
    }
    if let Some(kids) = v.get_mut("children").and_then(Value::as_array_mut) {
        for c in kids.iter_mut() {
            merge_backdrop_child_fill(c);
        }
    }
}

pub fn apply_tree_heuristics(
    nodes: &mut [PenNode],
    page_bg: Option<&str>,
    light_theme: bool,
    prior_accent: Option<&str>,
) {
    // The emphasis color to use when filling an invisible band. Prefer the
    // DESIGN-WIDE dominant accent (counted across already-generated siblings,
    // passed by the caller) — this subtask's own forest may lack any accent
    // token. Fall back to a per-subtask scan, then the palette default.
    let design_accent = prior_accent
        .map(str::to_string)
        .or_else(|| {
            nodes
                .iter()
                .filter_map(|n| serde_json::to_value(n).ok())
                .find_map(|v| find_design_accent(&v))
        })
        .unwrap_or_else(|| "$color-accent".to_string());
    for node in nodes.iter_mut() {
        let Ok(mut v) = serde_json::to_value(&*node) else {
            continue;
        };
        // Section-level (operate on the forest root = a page-root child).
        strip_redundant_section_fill(&mut v, page_bg);
        inject_nav_surface_for_section(&mut v);
        fix_invisible_text_band(&mut v, light_theme, &design_accent);
        // Subtree-recursive.
        merge_backdrop_child_fill(&mut v);
        convert_stacked_overlay_to_absolute(&mut v);
        clip_card_image_corners(&mut v);
        fill_card_leading_image_width(&mut v);
        fix_notification_badge_overlay(&mut v);
        strip_nested_card_decoration(&mut v, DecoFlags::default());
        // Round the active nav tab into a pill LAST — it must run AFTER
        // `strip_nested_card_decoration`, which would otherwise strip the pill's
        // `cornerRadius` back off (the active tab is a rounded frame nested
        // inside an already-rounded nav pill, so the dedup pass reads it as
        // redundant nested-card decoration). Running last makes the pill the
        // final word, fixing the sharp active square overflowing the nav pill.
        round_active_nav_tab(&mut v, &design_accent);
        if let Ok(new_node) = serde_json::from_value::<PenNode>(v) {
            *node = new_node;
        }
    }
}

#[cfg(test)]
#[path = "tree_heuristics_tests.rs"]
mod tests;
