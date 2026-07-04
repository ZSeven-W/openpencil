//! Cross-node post-pass — P2 increments **I3** (contrast) + **I4** (layout
//! properties).
//!
//! Port of `resolveTreePostPass` from role-resolver.ts. Runs AFTER role
//! resolution (I1/I2), so it keys off the roles those passes set.
//!
//! - **I3 contrast cluster**: `fixButtonForegroundContrast`,
//!   `fixSectionAlternation`, `fixOrphanContainerContrast`,
//!   `fixInputSiblingConsistency`, plus the color helpers (`hexLuminance`,
//!   `hasFill`, `hasVisibleFill`, `getFirstSolidColor`,
//!   `needsLuminanceContrastOverride`, `resolveColorMaybeRef`).
//! - **I4 layout-PROPERTY cluster** (pure property sets — no layout recompute):
//!   `equalizeCardRow`, `normalizeFormInputWidths`,
//!   `normalizeInputTrailingIconAlignment`, and the `clipContent`-for-image
//!   branch. DEFERRED from I4: `fixHorizontalOverflow` (recomputes pixel
//!   widths / parent width → would fight jian/taffy layout), `fixTextHeights` +
//!   the frame-height expansion (need text measurement), and
//!   `repairPlaceholderIcons` (needs the icon catalog).
//!
//! Operates on a JSON Value tree (serialize the sub-agent forest → fix →
//! deserialize): fills/effects/sizes are far simpler to read and mutate as JSON
//! than across the typed schema, and it mirrors the TS code, which mutates the
//! plain node objects.
//!
//! `resolveColorMaybeRef`: Rust has no document-variable context at sub-agent
//! time, so a `$color-*` ref resolves to `None` and the dependent fix is
//! skipped — matching the TS behavior when a ref doesn't resolve to a hex.

use jian_ops_schema::node::PenNode;
use serde_json::{json, Value};

use crate::role_layout_post_pass::{fix_horizontal_overflow, fix_text_heights, size_number};

// ── color helpers ───────────────────────────────────────────────────────────

/// Perceived luminance 0..1 (port of `hexLuminance` — 0.299/0.587/0.114, NOT
/// the WCAG curve used for theme detection). `None` on a non-hex string.
fn hex_luminance(hex: &str) -> Option<f64> {
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

fn fill_array(node: &Value) -> Option<&Vec<Value>> {
    node.get("fill").and_then(Value::as_array)
}

/// Any declared fill entry (visible or not). Port of `hasFill`.
fn has_fill(node: &Value) -> bool {
    fill_array(node).map(|a| !a.is_empty()).unwrap_or(false)
}

/// True when the parent's fill field (passed down the walk) is non-empty.
fn fill_present(fill: Option<&Value>) -> bool {
    fill.and_then(Value::as_array)
        .map(|a| !a.is_empty())
        .unwrap_or(false)
}

fn is_invisible_color(color: &str) -> bool {
    let c = color.trim().to_lowercase();
    if c == "transparent" || c == "none" {
        return true;
    }
    // 8-digit hex with 00 alpha (#RRGGBB00) — valid but draws nothing.
    c.len() == 9 && c.starts_with('#') && c.ends_with("00")
}

fn is_fill_invisible(fill: &Value) -> bool {
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
fn has_visible_fill(node: &Value) -> bool {
    let Some(arr) = fill_array(node) else {
        return false;
    };
    let Some(first) = arr.first() else {
        return false;
    };
    !is_fill_invisible(first)
}

/// First solid fill's color string. Port of `getFirstSolidColor`.
fn get_first_solid_color(node: &Value) -> Option<String> {
    fill_array(node)?
        .iter()
        .find(|f| f.get("type").and_then(Value::as_str) == Some("solid"))
        .and_then(|f| f.get("color").and_then(Value::as_str))
        .map(str::to_string)
}

/// Resolve a `$color-*` ref to a hex — Rust has no doc-variable context at
/// sub-agent time, so refs resolve to `None` and the dependent fix is skipped
/// (port of `resolveColorMaybeRef`'s unresolved path).
fn resolve_color_maybe_ref(color: &str) -> Option<String> {
    if color.trim_start().starts_with('$') {
        None
    } else {
        Some(color.to_string())
    }
}

/// A design-system token that always binds to a saturated, mid-dark colour
/// needing a WHITE foreground (accent / primary brand colour + the saturated
/// state colours). `$color-warning` is deliberately excluded — it's often a
/// light amber that reads better with a dark foreground. Matches with or
/// without a trailing shade suffix (`$color-accent`, `$color-primary-600`).
fn is_saturated_accent_token(color: &str) -> bool {
    let t = color.trim().trim_start_matches('$').to_ascii_lowercase();
    const ROOTS: &[&str] = &[
        "color-accent",
        "color-primary",
        "color-danger",
        "color-error",
        "color-success",
    ];
    ROOTS.iter().any(|root| {
        t == *root
            || t.strip_prefix(root)
                .is_some_and(|rest| rest.starts_with('-'))
    })
}

/// |Δluminance| < 0.5 → too close, needs a contrast override (port of
/// `needsLuminanceContrastOverride`).
fn needs_luminance_contrast_override(fg: &str, bg: &str) -> bool {
    match (hex_luminance(fg), hex_luminance(bg)) {
        (Some(f), Some(b)) => (f - b).abs() < 0.5,
        _ => false,
    }
}

fn role_of(node: &Value) -> Option<&str> {
    node.get("role").and_then(Value::as_str)
}

fn identity_label(node: &Value) -> String {
    let id = node.get("id").and_then(Value::as_str).unwrap_or("");
    let name = node.get("name").and_then(Value::as_str).unwrap_or("");
    let role = node.get("role").and_then(Value::as_str).unwrap_or("");
    format!("{id} {name} {role}").to_lowercase()
}

fn semantic_label(node: &Value) -> String {
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

fn corner_radius(node: &Value) -> f64 {
    match node.get("cornerRadius") {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::Array(a)) => a.first().and_then(Value::as_f64).unwrap_or(0.0),
        _ => 0.0,
    }
}

fn solid_fill(color: &str) -> Value {
    json!([{ "type": "solid", "color": color }])
}

fn neutral_stroke(color: &str) -> Value {
    json!({ "thickness": 1, "fill": solid_fill(color) })
}

fn clear_visual_chrome(node: &mut Value) {
    if let Some(obj) = node.as_object_mut() {
        obj.remove("fill");
        obj.remove("stroke");
        obj.remove("effects");
        obj.remove("cornerRadius");
    }
}

// ── fixButtonForegroundContrast ──────────────────────────────────────────────

fn fix_button_foreground_contrast(node: &mut Value) {
    if !matches!(role_of(node), Some("button") | Some("icon-button")) {
        return;
    }
    // A transparent button has no bg to compute contrast against.
    if !has_visible_fill(node) {
        return;
    }
    let Some(bg_raw) = get_first_solid_color(node) else {
        return;
    };
    // A brand-accent token (`$color-accent` / `$color-primary`) binds to a
    // concrete hex only at render time, so `resolve_color_maybe_ref` can't
    // read its luminance here and the pass used to bail — leaving the model's
    // default-dark icon on an orange accent button (measured: a `sliders`
    // icon at `#0F172A` on a `$color-accent` filter button). These tokens are
    // always saturated colours that need a WHITE foreground, so treat the bg
    // as dark and let the same override logic below flip the children.
    let (bg, lum) = match resolve_color_maybe_ref(&bg_raw) {
        Some(bg) => match hex_luminance(&bg) {
            Some(lum) => (bg, lum),
            None => return, // unparseable bg → can't pick safely, skip
        },
        None if is_saturated_accent_token(&bg_raw) => ("#EA580C".to_string(), 0.35),
        None => return,
    };

    let fg = if lum < 0.5 { "#FFFFFF" } else { "#0F172A" };
    let fg_fill = solid_fill(fg);

    let Some(children) = node.get("children").and_then(Value::as_array) else {
        return;
    };
    // PASS 1: a sibling text's resolved color is the reference foreground —
    // text + icon in a button read as one unit.
    let mut reference_fg: Option<String> = None;
    for child in children {
        if child.get("type").and_then(Value::as_str) != Some("text") {
            continue;
        }
        if !has_visible_fill(child) {
            continue;
        }
        if let Some(tc) = get_first_solid_color(child) {
            if let Some(resolved) = resolve_color_maybe_ref(&tc) {
                reference_fg = Some(resolved);
                break;
            }
        }
    }
    let final_fg_fill = reference_fg
        .as_ref()
        .map(|c| solid_fill(c))
        .unwrap_or_else(|| fg_fill.clone());

    // PASS 2: apply foreground to children.
    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    for child in children.iter_mut() {
        match child.get("type").and_then(Value::as_str) {
            Some("text") => {
                if !has_visible_fill(child) {
                    child["fill"] = fg_fill.clone();
                }
            }
            Some("icon_font") => {
                if !has_visible_fill(child) {
                    child["fill"] = final_fg_fill.clone();
                    continue;
                }
                if let Some(reference) = &reference_fg {
                    let existing =
                        get_first_solid_color(child).and_then(|c| resolve_color_maybe_ref(&c));
                    if let Some(existing) = existing {
                        if existing.to_lowercase() != reference.to_lowercase() {
                            child["fill"] = final_fg_fill.clone();
                        }
                    }
                } else {
                    // Icon-only button: luminance-delta override.
                    let existing =
                        get_first_solid_color(child).and_then(|c| resolve_color_maybe_ref(&c));
                    if let Some(existing) = existing {
                        if needs_luminance_contrast_override(&existing, &bg) {
                            child["fill"] = fg_fill.clone();
                        }
                    }
                }
            }
            Some("path") => {
                let has_stroke = child.get("stroke").map(|s| !s.is_null()).unwrap_or(false);
                let has_stroke_fill = child
                    .get("stroke")
                    .and_then(|s| s.get("fill"))
                    .and_then(Value::as_array)
                    .map(|a| !a.is_empty())
                    .unwrap_or(false);
                if has_visible_fill(child) {
                    // already styled
                } else if has_stroke && !has_stroke_fill {
                    child["stroke"]["fill"] = final_fg_fill.clone();
                } else if !has_stroke {
                    child["fill"] = final_fg_fill.clone();
                }
            }
            _ => {}
        }
    }
}

// ── fixSectionAlternation ────────────────────────────────────────────────────

const SECTION_ROLES: &[&str] = &["section", "hero", "cta-section", "stats-section", "footer"];
const ALTERNATING_BG: [&str; 2] = ["#FFFFFF", "#F8FAFC"];

fn fix_section_alternation(node: &mut Value) {
    if node.get("layout").and_then(Value::as_str) != Some("vertical") {
        return;
    }
    // Only alternate on light-themed pages (the hardcoded white strips would
    // fight a dark page background).
    if let Some(bg) = get_first_solid_color(node) {
        if hex_luminance(&bg).map(|l| l < 0.5).unwrap_or(false) {
            return;
        }
    }
    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    // Group consecutive section-role children into runs.
    let mut runs: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    for (i, child) in children.iter().enumerate() {
        let is_section = child.get("type").and_then(Value::as_str) == Some("frame")
            && role_of(child)
                .map(|r| SECTION_ROLES.contains(&r))
                .unwrap_or(false);
        if is_section {
            current.push(i);
        } else if !current.is_empty() {
            runs.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        runs.push(current);
    }
    for run in runs {
        let unfilled = run.iter().filter(|&&i| !has_fill(&children[i])).count();
        if unfilled < 3 {
            continue;
        }
        let mut idx = 0;
        for &i in &run {
            if !has_fill(&children[i]) {
                children[i]["fill"] = solid_fill(ALTERNATING_BG[idx % 2]);
                idx += 1;
            }
        }
    }
}

// ── fixOrphanContainerContrast ───────────────────────────────────────────────

const STRUCTURAL_DENYLIST: &[&str] = &[
    "section",
    "row",
    "column",
    "centered-content",
    "form-group",
    "feature-grid",
    "screenshot-frame",
    "phone-mockup",
    "navbar",
    "nav-links",
    "hero",
    "footer",
    "cta-section",
    "stats-section",
    "table",
    "table-row",
    "table-header",
    "spacer",
    "divider",
];
const CARD_LIKE_ALLOWLIST: &[&str] = &[
    "card",
    "stat-card",
    "pricing-card",
    "feature-card",
    "image-card",
    "testimonial",
];

fn is_ring_like_decorative(node: &Value) -> bool {
    let id = node.get("id").and_then(Value::as_str).unwrap_or("");
    let name = node.get("name").and_then(Value::as_str).unwrap_or("");
    let label = format!("{id} {name}").to_lowercase();
    if !(label.contains("ring")
        || label.contains("circle")
        || label.contains("progress")
        || label.contains("activity"))
    {
        return false;
    }
    if node.get("stroke").map(Value::is_null).unwrap_or(true) {
        return false;
    }
    let w = node.get("width").and_then(Value::as_f64).unwrap_or(0.0);
    let h = node.get("height").and_then(Value::as_f64).unwrap_or(0.0);
    if w <= 0.0 || h <= 0.0 {
        return false;
    }
    let roughly_square = (w - h).abs() <= 2.0_f64.max(w.max(h) * 0.08);
    if !roughly_square {
        return false;
    }
    corner_radius(node) >= w.min(h) * 0.35
}

/// layout.md:24 — interior section wrappers (Header / Search Section /
/// Categories Section …) MUST be transparent; an explicit near-white fill
/// makes an unwanted "white card" on the page bg. The model (esp. glm-5.2)
/// ignores that rule, and `role_defaults`' AI-explicit-wins won't strip it —
/// so force it here: a structural wrapper carrying a near-white / page-tone
/// solid fill gets cleared to transparent. CARD_LIKE roles (card, promo
/// banner, …) are intentional surfaces and keep their fill; an intentional
/// dark / saturated section background is also left alone (only near-white
/// fills — luminance ≥ 0.85 — read as the "white card on cream" smell).
/// Neutral page/surface design-variable tokens. A structural wrapper carrying
/// any of these is repainting the page background and must go transparent.
/// Variable refs are NOT resolved at post-pass time (binding runs afterwards),
/// so `hex_luminance` can't read them — match the token names directly. Colored
/// tokens (`$color-accent`, dark bands, etc.) are deliberate → never listed here.
const LIGHT_SURFACE_REFS: &[&str] = &[
    "$color-bg-deep",
    "$color-surface",
    "$color-surface-2",
    "$color-surface-3",
];

/// True when a fill color reads as a light page/surface tone — either a parsed
/// hex with luminance ≥ 0.85, or one of the neutral surface variable refs that
/// glm emits before variable binding resolves them.
fn is_light_surface_fill(color: &str) -> bool {
    LIGHT_SURFACE_REFS.contains(&color) || hex_luminance(color).map(|l| l >= 0.85).unwrap_or(false)
}

/// A child paints a colored (non-page-tone) surface: a gradient, or a solid
/// that is NOT a light page/surface tone. The signal that a child is a real
/// banner/card surface rather than a neutral panel.
fn child_has_colored_fill(node: &Value) -> bool {
    let Some(first) = fill_array(node).and_then(|a| a.first()) else {
        return false;
    };
    match first.get("type").and_then(Value::as_str) {
        Some("linear_gradient") | Some("radial_gradient") => true,
        Some("solid") => first
            .get("color")
            .and_then(Value::as_str)
            .map(|c| !is_light_surface_fill(c))
            .unwrap_or(false),
        _ => false,
    }
}

/// True when the node has EXACTLY ONE container child and that child spans the
/// width (`fill_container`) painting a colored surface. The node is then a
/// redundant frame wrapped around a full-bleed banner/card — its own light
/// surface fill is a box showing around the colored child. (Tiny leaf siblings
/// like an orphan badge are ignored for the count.)
fn is_redundant_colored_wrapper(node: &Value) -> bool {
    let Some(kids) = node.get("children").and_then(Value::as_array) else {
        return false;
    };
    let containers: Vec<&Value> = kids
        .iter()
        .filter(|c| {
            matches!(
                c.get("type").and_then(Value::as_str),
                Some("frame") | Some("group")
            )
        })
        .collect();
    if containers.len() != 1 {
        return false;
    }
    let child = containers[0];
    child.get("width").and_then(Value::as_str) == Some("fill_container")
        && child_has_colored_fill(child)
}

fn fix_structural_wrapper_transparency(node: &mut Value) {
    let Some(role) = role_of(node).map(str::to_string) else {
        return;
    };
    if CARD_LIKE_ALLOWLIST.contains(&role.as_str()) {
        // A card-like role normally keeps its fill — EXCEPT when it is a
        // redundant wrapper around a single full-bleed colored child card (glm
        // wraps an orange promo banner in a `feature-card` whose own
        // `$color-surface` fill then shows as a light box around / under the
        // colored child). The colored child IS the banner surface, so strip the
        // wrapper's light fill + chrome.
        if is_redundant_colored_wrapper(node) {
            if let Some(color) = get_first_solid_color(node) {
                if is_light_surface_fill(&color) {
                    if let Some(obj) = node.as_object_mut() {
                        obj.insert("fill".to_string(), Value::Array(Vec::new()));
                        obj.remove("stroke");
                        obj.remove("effects");
                    }
                }
            }
        }
        return; // intentional surface — keep its fill
    }
    let structural = STRUCTURAL_DENYLIST.contains(&role.as_str())
        || SECTION_ROLES.contains(&role.as_str())
        || role == "header"
        || role == "app-bar"
        || role.contains("section");
    if !structural {
        return;
    }
    let Some(color) = get_first_solid_color(node) else {
        return;
    };
    if is_light_surface_fill(&color) {
        if let Some(obj) = node.as_object_mut() {
            obj.insert("fill".to_string(), Value::Array(Vec::new()));
            // Drop the accompanying hairline border too: a structural wrapper
            // made transparent keeps no "card/bar chrome" (the user flagged the
            // navbar's bottom border as unreasonable alongside its background).
            obj.remove("stroke");
            // …and no elevation shadow — a now-transparent wrapper would cast it
            // as a gray ghost box around its content.
            obj.remove("effects");
        }
    }
}

/// True when an immediate child container (frame/group) already paints a
/// visible surface. Such a node is a redundant wrapper around the real card —
/// it must NOT be whitewashed into its own surface, or the injected drop-shadow
/// ghosts out around the colored child as a gray rounded box.
fn has_filled_container_child(node: &Value) -> bool {
    node.get("children")
        .and_then(Value::as_array)
        .map(|kids| {
            kids.iter().any(|c| {
                matches!(
                    c.get("type").and_then(Value::as_str),
                    Some("frame") | Some("group")
                ) && has_visible_fill(c)
            })
        })
        .unwrap_or(false)
}

fn fix_orphan_container_contrast(node: &mut Value, parent_fill: Option<&Value>) {
    if parent_fill.is_none() {
        return; // root has no parent context
    }
    if has_fill(node) || fill_present(parent_fill) {
        return;
    }
    if is_ring_like_decorative(node) {
        return;
    }
    if corner_radius(node) <= 0.0 {
        return;
    }
    if node
        .get("children")
        .and_then(Value::as_array)
        .map(|a| a.is_empty())
        .unwrap_or(true)
    {
        return;
    }
    // A child container already paints the visible surface (e.g. glm wraps an
    // orange promo banner in a bare `feature-card` frame). Whitewashing this
    // redundant wrapper into a second card leaks the injected drop-shadow out
    // around the colored child as a gray "ghost box" — the exact mysterious
    // background + rounded border the user flagged. Let the child be the card.
    if has_filled_container_child(node) {
        return;
    }
    // Only rescue containers we POSITIVELY identify as card-like; structural
    // wrappers (header / section / banner / search bar) and roleless rounded
    // frames stay transparent — white-washing them is what produced the spurious
    // white panels behind headers / banners / search rows.
    let Some(role) = role_of(node) else {
        return;
    };
    if STRUCTURAL_DENYLIST.contains(&role) || !CARD_LIKE_ALLOWLIST.contains(&role) {
        return;
    }
    node["fill"] = solid_fill("#FFFFFF");
    node["effects"] = json!([
        { "type": "shadow", "offsetX": 0, "offsetY": 1, "blur": 3, "spread": 0, "color": "#0000001A" },
        { "type": "shadow", "offsetX": 0, "offsetY": 1, "blur": 2, "spread": -1, "color": "#0000000F" }
    ]);
}

// ── normalizeNestedSearchShell ──────────────────────────────────────────────

fn child_role(child: &Value) -> Option<&str> {
    child.get("role").and_then(Value::as_str)
}

fn is_search_input_child(child: &Value) -> bool {
    let label = semantic_label(child);
    matches!(
        child_role(child),
        Some("input") | Some("form-input") | Some("search-bar")
    ) || (child.get("type").and_then(Value::as_str) == Some("text_input")
        && (label.contains("search") || label.contains("搜索")))
        || label.contains("search")
        || label.contains("搜索")
}

fn is_filter_button_child(child: &Value) -> bool {
    let label = semantic_label(child);
    if (matches!(child_role(child), Some("icon-button") | Some("button"))
        || child.get("type").and_then(Value::as_str) == Some("icon_font"))
        && mentions_filter_affordance(&label)
    {
        return true;
    }
    child
        .get("children")
        .and_then(Value::as_array)
        .map(|children| {
            children.iter().any(|grandchild| {
                let label = semantic_label(grandchild);
                mentions_filter_affordance(&label)
            })
        })
        .unwrap_or(false)
}

fn mentions_filter_affordance(label: &str) -> bool {
    label.contains("filter")
        || label.contains("slider")
        || label.contains("sliders")
        || label.contains("筛选")
        || label.contains("过滤")
        || label.contains("调节")
}

fn normalize_nested_search_shell(node: &mut Value) {
    if node.get("type").and_then(Value::as_str) != Some("frame")
        || node.get("layout").and_then(Value::as_str) != Some("horizontal")
    {
        return;
    }

    let Some(children) = node.get("children").and_then(Value::as_array) else {
        return;
    };
    let search_child_count = children.iter().filter(|c| is_search_input_child(c)).count();
    let has_filter = children
        .iter()
        .any(|c| !is_search_input_child(c) && is_filter_button_child(c));
    if search_child_count != 1 || !has_filter {
        return;
    }

    clear_visual_chrome(node);
    node["gap"] = json!(12);
    node["padding"] = json!([0, 0]);
    node["alignItems"] = json!("center");

    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    for child in children {
        if is_search_input_child(child) {
            child["fill"] = solid_fill("#FFFFFF");
            child["stroke"] = neutral_stroke("#E5E7EB");
            child["cornerRadius"] = json!(8);
        } else if is_filter_button_child(child) {
            child["fill"] = solid_fill("#FFFFFF");
            child["stroke"] = neutral_stroke("#E5E7EB");
            child["cornerRadius"] = json!(8);
            set_subtree_foreground(child, "$color-accent");
        }
    }
}

fn set_subtree_foreground(node: &mut Value, color: &str) {
    match node.get("type").and_then(Value::as_str) {
        Some("text") | Some("icon_font") => {
            node["fill"] = solid_fill(color);
        }
        Some("path") => {
            if node.get("stroke").map(|s| !s.is_null()).unwrap_or(false) {
                node["stroke"]["fill"] = solid_fill(color);
            } else {
                node["fill"] = solid_fill(color);
            }
        }
        _ => {}
    }
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children {
            set_subtree_foreground(child, color);
        }
    }
}

// ── normalizeFavoriteIconButtons ────────────────────────────────────────────

fn subtree_mentions_heart(node: &Value) -> bool {
    let label = identity_label(node);
    if label.contains("favorite")
        || label.contains("favourite")
        || label.contains("heart")
        || label.contains("like")
        || label.contains("收藏")
        || label.contains("喜欢")
    {
        return true;
    }
    node.get("children")
        .and_then(Value::as_array)
        .map(|children| children.iter().any(subtree_mentions_heart))
        .unwrap_or(false)
}

fn normalize_favorite_icon_buttons(node: &mut Value) {
    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    for child in children {
        if child_role(child) == Some("icon-button") && subtree_mentions_heart(child) {
            clear_visual_chrome(child);
            child["width"] = json!(32);
            child["height"] = json!(32);
        }
    }
}

// ── normalizeSectionHeaderActions ────────────────────────────────────────────

fn text_content(node: &Value) -> Option<&str> {
    node.get("content").and_then(Value::as_str)
}

fn numeric_prop(node: &Value, key: &str) -> Option<f64> {
    node.get(key).and_then(|value| {
        value
            .as_f64()
            .or_else(|| value.as_str().and_then(|s| s.parse::<f64>().ok()))
    })
}

fn is_see_all_action_text(node: &Value) -> bool {
    if node.get("type").and_then(Value::as_str) != Some("text") {
        return false;
    }
    let Some(content) = text_content(node) else {
        return false;
    };
    let compact = content
        .trim()
        .trim_matches(|c: char| matches!(c, '>' | '›' | '→' | '»'))
        .to_lowercase()
        .replace(char::is_whitespace, "");
    matches!(
        compact.as_str(),
        "查看全部" | "查看更多" | "seeall" | "viewall" | "viewmore"
    )
}

fn is_section_heading_text(node: &Value) -> bool {
    if node.get("type").and_then(Value::as_str) != Some("text") || is_see_all_action_text(node) {
        return false;
    }
    let label = identity_label(node);
    if matches!(role_of(node), Some("heading") | Some("subheading"))
        || label.contains("heading")
        || label.contains("title")
        || label.contains("header")
    {
        return true;
    }
    let weight = numeric_prop(node, "fontWeight").unwrap_or(0.0);
    let size = numeric_prop(node, "fontSize").unwrap_or(0.0);
    weight >= 600.0 && size >= 16.0
}

fn rewrite_text_node_as_chevron(child: &mut Value) {
    let Some(obj) = child.as_object_mut() else {
        return;
    };
    obj.insert("type".to_string(), json!("icon_font"));
    obj.insert("iconFontName".to_string(), json!("chevron-right"));
    obj.insert("width".to_string(), json!(20));
    obj.insert("height".to_string(), json!(20));
    obj.insert("fill".to_string(), solid_fill("$color-accent"));
    obj.remove("content");
    obj.remove("fontFamily");
    obj.remove("fontSize");
    obj.remove("fontWeight");
    obj.remove("fontStyle");
    obj.remove("textAlign");
    obj.remove("textGrowth");
    obj.remove("lineHeight");
    obj.remove("letterSpacing");
}

fn normalize_section_header_actions(node: &mut Value) {
    if node.get("type").and_then(Value::as_str) != Some("frame")
        || node.get("layout").and_then(Value::as_str) != Some("horizontal")
    {
        return;
    }
    let Some(children) = node.get("children").and_then(Value::as_array) else {
        return;
    };
    if !children.iter().any(is_see_all_action_text) || !children.iter().any(is_section_heading_text)
    {
        return;
    }
    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    for child in children {
        if is_see_all_action_text(child) {
            rewrite_text_node_as_chevron(child);
        }
    }
}

// ── normalizeMobileCategoryRows ──────────────────────────────────────────────

fn is_category_row_label(label: &str) -> bool {
    label.contains("category")
        || label.contains("categories")
        || label.contains("cuisine")
        || label.contains("分类")
        || label.contains("品类")
}

fn is_chip_like_child(node: &Value) -> bool {
    if node.get("type").and_then(Value::as_str) != Some("frame") {
        return false;
    }
    matches!(
        role_of(node),
        Some("chip") | Some("tag") | Some("pill") | Some("button")
    ) || {
        let label = semantic_label(node);
        label.contains("chip") || label.contains("category") || label.contains("类别")
    }
}

fn has_descendant_type(node: &Value, type_name: &str) -> bool {
    node.get("type").and_then(Value::as_str) == Some(type_name)
        || node
            .get("children")
            .and_then(Value::as_array)
            .map(|children| {
                children
                    .iter()
                    .any(|child| has_descendant_type(child, type_name))
            })
            .unwrap_or(false)
}

fn has_text_descendant(node: &Value) -> bool {
    node.get("type").and_then(Value::as_str) == Some("text")
        && text_content(node)
            .map(|content| !content.trim().is_empty())
            .unwrap_or(false)
        || node
            .get("children")
            .and_then(Value::as_array)
            .map(|children| children.iter().any(has_text_descendant))
            .unwrap_or(false)
}

fn is_icon_label_category_item(node: &Value) -> bool {
    node.get("type").and_then(Value::as_str) == Some("frame")
        && has_descendant_type(node, "icon_font")
        && has_text_descendant(node)
}

fn is_category_item_like_child(node: &Value) -> bool {
    is_chip_like_child(node) || is_icon_label_category_item(node)
}

fn has_loose_category_spacing(node: &Value, canvas_width: f64) -> bool {
    numeric_prop(node, "width")
        .map(|width| width > canvas_width)
        .unwrap_or(false)
        || matches!(
            node.get("justifyContent").and_then(Value::as_str),
            Some("space_between") | Some("space_around")
        )
        || numeric_prop(node, "gap")
            .map(|gap| gap > 48.0)
            .unwrap_or(false)
}

fn category_item_row_count(node: &Value) -> Option<usize> {
    if node.get("type").and_then(Value::as_str) != Some("frame")
        || node.get("layout").and_then(Value::as_str) != Some("horizontal")
    {
        return None;
    }
    let children = node.get("children").and_then(Value::as_array)?;
    let item_count = children
        .iter()
        .filter(|child| is_category_item_like_child(child))
        .count();
    if item_count < 2 {
        return None;
    }
    if children
        .iter()
        .any(|child| !is_category_item_like_child(child))
    {
        return None;
    }
    Some(item_count)
}

fn should_normalize_mobile_category_row(node: &Value, canvas_width: f64) -> bool {
    let Some(item_count) = category_item_row_count(node) else {
        return false;
    };
    let loose_spacing = has_loose_category_spacing(node, canvas_width);
    if !is_category_row_label(&semantic_label(node)) && !loose_spacing {
        return false;
    }
    item_count >= 4 || loose_spacing
}

fn should_normalize_mobile_category_row_in_section(node: &Value) -> bool {
    category_item_row_count(node).is_some()
}

fn category_section_has_direct_item_row(node: &Value, canvas_width: f64) -> bool {
    node.get("children")
        .and_then(Value::as_array)
        .map(|children| {
            children.iter().any(|child| {
                should_normalize_mobile_category_row(child, canvas_width)
                    || should_normalize_mobile_category_row_in_section(child)
            })
        })
        .unwrap_or(false)
}

fn normalize_mobile_category_section(node: &mut Value, canvas_width: f64) {
    if canvas_width > 480.0
        || node.get("type").and_then(Value::as_str) != Some("frame")
        || node.get("layout").and_then(Value::as_str) != Some("vertical")
        || !is_category_row_label(&semantic_label(node))
        || !category_section_has_direct_item_row(node, canvas_width)
    {
        return;
    }

    node["height"] = json!("fit_content");
    node["clipContent"] = json!(false);
    if node.get("gap").is_none()
        || numeric_prop(node, "gap")
            .map(|gap| gap > 12.0)
            .unwrap_or(false)
    {
        node["gap"] = json!(12);
    }

    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children {
            normalize_mobile_category_row_in_section(child);
        }
    }
}

fn normalize_mobile_category_row(node: &mut Value, canvas_width: f64) {
    if !should_normalize_mobile_category_row(node, canvas_width) {
        return;
    }

    normalize_mobile_category_row_unchecked(node);
}

fn normalize_mobile_category_row_in_section(node: &mut Value) {
    if !should_normalize_mobile_category_row_in_section(node) {
        return;
    }

    normalize_mobile_category_row_unchecked(node);
}

fn normalize_mobile_category_row_unchecked(node: &mut Value) {
    node["width"] = json!("fill_container");
    node["height"] = json!("fit_content");
    node["gap"] = json!(12);
    node["clipContent"] = json!(false);
    node["alignItems"] = json!("center");
    // Distribute a small fixed set of chips across the row instead of clustering
    // them on the left with a lopsided empty band on the right. 3+ chips use
    // space_between so the leftover width becomes even gaps (the user's
    // "撑不满就把间距放大一点"); 1-2 chips stay start-aligned (space_between would
    // throw two chips to opposite edges). The chip COUNT follows the model — no
    // truncation — so the screen isn't forced to exactly four categories.
    let child_count = node
        .get("children")
        .and_then(Value::as_array)
        .map(|c| c.len())
        .unwrap_or(0);
    node["justifyContent"] = json!(if child_count >= 3 {
        "space_between"
    } else {
        "start"
    });

    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    // Off-canvas guard: ~5 fit_content icon+label chips is the most that fits one
    // 375px row (overflow.md). The count is NOT pinned to four anymore, but a
    // single non-scrolling row beyond five would push chips off the right edge,
    // so cap at five. (6+ categories should be a horizontal scroll rail or a
    // grid — the model is guided there by overflow.md.)
    const MAX_CHIPS_PER_ROW: usize = 5;
    if children.len() > MAX_CHIPS_PER_ROW {
        children.truncate(MAX_CHIPS_PER_ROW);
    }
    for (idx, child) in children.iter_mut().enumerate() {
        if is_category_item_like_child(child) {
            child["width"] = json!("fit_content");
            child["height"] = json!("fit_content");
            child["cornerRadius"] = json!(8);
            if is_icon_label_category_item(child) {
                child["gap"] = json!(8);
                child["alignItems"] = json!("center");
                normalize_category_icon_tile(child, idx == 0);
            }
        }
    }
}

fn normalize_category_icon_tile(item: &mut Value, active: bool) {
    let Some(children) = item.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    let Some(tile) = children.iter_mut().find(|child| {
        child.get("type").and_then(Value::as_str) == Some("frame")
            && has_descendant_type(child, "icon_font")
    }) else {
        return;
    };

    tile["width"] = json!(56);
    tile["height"] = json!(56);
    tile["cornerRadius"] = json!(8);
    tile["layout"] = json!("horizontal");
    tile["justifyContent"] = json!("center");
    tile["alignItems"] = json!("center");
    if active {
        tile["fill"] = solid_fill("#FFF0E3");
        if let Some(obj) = tile.as_object_mut() {
            obj.remove("stroke");
            obj.remove("effects");
        }
        set_subtree_foreground(tile, "$color-accent");
    } else {
        tile["fill"] = solid_fill("#FFFFFF");
        tile["stroke"] = neutral_stroke("#EAD8C8");
        set_subtree_foreground(tile, "#8A5F49");
    }
}

// ── normalizeMobileProductCardRows ──────────────────────────────────────────

fn is_product_card_role(role: Option<&str>) -> bool {
    matches!(
        role,
        Some("card")
            | Some("image-card")
            | Some("product-card")
            | Some("restaurant-card")
            | Some("menu-card")
            | Some("feature-card")
    )
}

fn is_mobile_product_card_child(node: &Value) -> bool {
    if node.get("type").and_then(Value::as_str) != Some("frame") {
        return false;
    }
    if is_product_card_role(role_of(node)) {
        return true;
    }
    let label = semantic_label(node);
    [
        "card",
        "product",
        "restaurant",
        "popular",
        "dish",
        "menu",
        "nearby",
        "餐厅",
        "美食",
        "热门",
        "菜品",
    ]
    .iter()
    .any(|needle| label.contains(needle))
        || (has_descendant_type(node, "image") && has_text_descendant(node))
}

fn should_normalize_mobile_product_card_row(node: &Value, canvas_width: f64) -> bool {
    if canvas_width > 480.0
        || node.get("type").and_then(Value::as_str) != Some("frame")
        || node.get("layout").and_then(Value::as_str) != Some("horizontal")
    {
        return false;
    }
    let Some(children) = node.get("children").and_then(Value::as_array) else {
        return false;
    };
    if children.len() != 2 || !children.iter().all(is_mobile_product_card_child) {
        return false;
    }

    let fixed_children_width: f64 = children
        .iter()
        .filter_map(|child| numeric_prop(child, "width"))
        .sum();
    let fixed_total = fixed_children_width + numeric_prop(node, "gap").unwrap_or(0.0);
    let content_rail_width = (canvas_width - 40.0).max(0.0);
    fixed_total > content_rail_width
        || numeric_prop(node, "width")
            .map(|width| width > content_rail_width)
            .unwrap_or(false)
        || matches!(
            node.get("justifyContent").and_then(Value::as_str),
            Some("space_between") | Some("space_around")
        )
}

fn normalize_mobile_product_card_row(node: &mut Value, canvas_width: f64) {
    if !should_normalize_mobile_product_card_row(node, canvas_width) {
        return;
    }

    node["width"] = json!("fill_container");
    node["height"] = json!("fit_content");
    node["gap"] = json!(12);
    node["clipContent"] = json!(false);
    node["justifyContent"] = json!("start");
    node["alignItems"] = json!("stretch");

    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    for child in children {
        child["width"] = json!("fill_container");
        child["height"] = json!("fit_content");
        child["cornerRadius"] = json!(8);
    }
}

fn horizontal_padding_sum(node: &Value) -> f64 {
    match node.get("padding") {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0) * 2.0,
        Some(Value::Array(values)) if values.len() == 2 => {
            values.get(1).and_then(Value::as_f64).unwrap_or(0.0) * 2.0
        }
        Some(Value::Array(values)) if values.len() >= 4 => {
            values.get(1).and_then(Value::as_f64).unwrap_or(0.0)
                + values.get(3).and_then(Value::as_f64).unwrap_or(0.0)
        }
        _ => 0.0,
    }
}

fn mobile_card_content_width(node: &Value, canvas_width: f64) -> f64 {
    let fallback = (canvas_width - 40.0).max(0.0);
    let outer = match node.get("width") {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(fallback),
        Some(Value::String(s)) if s == "fill_container" => fallback,
        Some(Value::String(s)) => s.parse::<f64>().unwrap_or(fallback),
        _ => fallback,
    };
    (outer - horizontal_padding_sum(node)).max(0.0)
}

fn direct_image_child_indices_over_height(node: &Value, max_height: f64) -> Vec<usize> {
    let Some(children) = node.get("children").and_then(Value::as_array) else {
        return Vec::new();
    };
    children
        .iter()
        .enumerate()
        .filter_map(|(idx, child)| {
            (child.get("type").and_then(Value::as_str) == Some("image")
                && numeric_prop(child, "height")
                    .map(|height| height > max_height + 24.0)
                    .unwrap_or(false))
            .then_some(idx)
        })
        .collect()
}

fn looks_like_mobile_food_media_card(node: &Value) -> bool {
    if node.get("type").and_then(Value::as_str) != Some("frame")
        || !is_mobile_product_card_child(node)
        || !has_descendant_type(node, "image")
        || !has_text_descendant(node)
    {
        return false;
    }
    let label = semantic_label(node);
    [
        "featured",
        "feature",
        "dish",
        "food",
        "menu",
        "product",
        "restaurant",
        "card",
        "推荐",
        "主题",
        "美食",
        "菜品",
        "餐厅",
    ]
    .iter()
    .any(|needle| label.contains(needle))
}

fn normalize_mobile_featured_card_media(node: &mut Value, canvas_width: f64) {
    if canvas_width > 480.0 || !looks_like_mobile_food_media_card(node) {
        return;
    }
    let content_width = mobile_card_content_width(node, canvas_width);
    if content_width <= 0.0 {
        return;
    }
    let max_height = (content_width * 0.6).round().clamp(148.0, 204.0);
    let image_indices = direct_image_child_indices_over_height(node, max_height);
    if image_indices.is_empty() {
        return;
    }

    node["height"] = json!("fit_content");
    node["clipContent"] = json!(true);
    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    for idx in image_indices {
        if let Some(image) = children.get_mut(idx) {
            image["width"] = json!("fill_container");
            image["height"] = json!(max_height as i64);
        }
    }
}

// ── normalizeCartCountBadges ────────────────────────────────────────────────

fn is_short_count_text(node: &Value) -> bool {
    if node.get("type").and_then(Value::as_str) != Some("text") {
        return false;
    }
    let Some(content) = text_content(node).map(str::trim) else {
        return false;
    };
    !content.is_empty() && content.len() <= 2 && content.chars().all(|c| c.is_ascii_digit())
}

fn has_short_count_text(node: &Value) -> bool {
    is_short_count_text(node)
        || node
            .get("children")
            .and_then(Value::as_array)
            .map(|children| children.iter().any(has_short_count_text))
            .unwrap_or(false)
}

fn is_cart_icon_node(node: &Value) -> bool {
    let label = semantic_label(node);
    label.contains("shopping-cart")
        || label.contains("shopping cart")
        || label.contains("cart")
        || label.contains("basket")
        || label.contains("checkout")
        || label.contains("购物车")
}

fn is_count_badge_candidate(node: &Value) -> bool {
    matches!(role_of(node), Some("badge")) || has_short_count_text(node)
}

fn style_count_badge(node: &mut Value) {
    node["role"] = json!("badge");
    node["width"] = json!(16);
    node["height"] = json!(16);
    node["cornerRadius"] = json!(999);
    node["padding"] = json!([0, 0]);
    node["layout"] = json!("horizontal");
    node["justifyContent"] = json!("center");
    node["alignItems"] = json!("center");
    node["fill"] = solid_fill("$color-accent");
    if let Some(obj) = node.as_object_mut() {
        obj.remove("stroke");
        obj.remove("effects");
    }
    set_count_badge_text_style(node);
}

fn set_count_badge_text_style(node: &mut Value) {
    if is_short_count_text(node) {
        node["fontSize"] = json!(10);
        node["fontWeight"] = json!(700);
        node["fill"] = solid_fill("#FFFFFF");
        node["textAlign"] = json!("center");
    }
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children {
            set_count_badge_text_style(child);
        }
    }
}

fn normalize_cart_count_badges(node: &mut Value) {
    if node.get("type").and_then(Value::as_str) != Some("frame") {
        return;
    }
    let Some(children) = node.get("children").and_then(Value::as_array) else {
        return;
    };
    let has_cart_icon = children.iter().any(is_cart_icon_node);
    let has_count_badge = children.iter().any(is_count_badge_candidate);
    if !has_cart_icon || !has_count_badge {
        return;
    }

    if matches!(role_of(node), Some("icon-button") | Some("button"))
        || semantic_label(node).contains("cart")
    {
        node["fill"] = solid_fill("#FFFFFF");
        node["stroke"] = neutral_stroke("#E5E7EB");
        node["cornerRadius"] = json!(8);
    }

    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    for child in children {
        if is_count_badge_candidate(child) {
            style_count_badge(child);
        }
    }
}

// ── fixInputSiblingConsistency ───────────────────────────────────────────────

fn fix_input_sibling_consistency(node: &mut Value) {
    if node.get("layout").and_then(Value::as_str) != Some("vertical") {
        return;
    }
    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    let input_idxs: Vec<usize> = children
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            c.get("type").and_then(Value::as_str) == Some("frame")
                && matches!(role_of(c), Some("input") | Some("form-input"))
                && has_fill(c)
        })
        .map(|(i, _)| i)
        .collect();
    if input_idxs.len() < 2 {
        return;
    }
    let Some(first_color) = get_first_solid_color(&children[input_idxs[0]]) else {
        return;
    };
    let all_match = input_idxs
        .iter()
        .all(|&i| get_first_solid_color(&children[i]).as_deref() == Some(first_color.as_str()));
    if all_match {
        return;
    }
    let source_fill = children[input_idxs[0]].get("fill").cloned();
    let source_stroke = children[input_idxs[0]].get("stroke").cloned();
    for &i in &input_idxs[1..] {
        if let Some(fill) = &source_fill {
            children[i]["fill"] = fill.clone();
        }
        if let Some(stroke) = &source_stroke {
            children[i]["stroke"] = stroke.clone();
        }
    }
}

// ── I4: layout-property fixes (no layout recompute / no font metrics) ───────

/// Equalize a row of fixed-width card frames to `fill_container` so they stretch
/// evenly (port of `equalizeCardRow` AND the near-identical
/// `equalizeHorizontalSiblings` in design-canvas-ops.ts — the dashboard 等宽
/// pass; the badge/pill/tag exclusions come from the latter). Pure property fix
/// — taffy then lays out.
fn equalize_card_row(node: &mut Value) {
    if node.get("layout").and_then(Value::as_str) != Some("horizontal") {
        return;
    }
    if node.get("width").and_then(Value::as_str) == Some("fit_content") {
        return;
    }
    let Some(children) = node.get("children").and_then(Value::as_array) else {
        return;
    };
    if children.len() < 2 {
        return;
    }
    let candidates: Vec<usize> = children
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            c.get("type").and_then(Value::as_str) == Some("frame")
                && !matches!(
                    role_of(c),
                    Some("divider")
                        | Some("phone-mockup")
                        | Some("badge")
                        | Some("pill")
                        | Some("tag")
                )
                && size_number(c, "height") > 88.0
        })
        .map(|(i, _)| i)
        .collect();
    // Already an explicit fill_container card in the row → widths are equal,
    // nothing to equalize. (A flexbox `minWidth:0` shrink-fix is NOT viable in
    // the node tree: the canonical PenNode schema has no `minWidth` field, so any
    // stamp is silently dropped on the serialize round-trip. Preventing a wide
    // KPI card from overflowing its share of the row is a jian flex-mapping
    // concern — fill_container columns need flex-basis:0 to shrink — not a
    // post-pass.)
    if candidates
        .iter()
        .any(|&i| children[i].get("width").and_then(Value::as_str) == Some("fill_container"))
    {
        return;
    }
    let fixed: Vec<usize> = candidates
        .into_iter()
        .filter(|&i| {
            children[i]
                .get("width")
                .and_then(Value::as_f64)
                .map(|w| w > 0.0)
                .unwrap_or(false)
        })
        .collect();
    if fixed.len() < 2 {
        return;
    }
    let widths: Vec<f64> = fixed
        .iter()
        .map(|&i| size_number(&children[i], "width"))
        .collect();
    let max_w = widths.iter().cloned().fold(0.0_f64, f64::max);
    let min_w = widths.iter().cloned().fold(f64::INFINITY, f64::min);
    if max_w <= 0.0 || min_w / max_w >= 0.6 {
        return; // widths already similar → leave alone
    }
    let heights: Vec<f64> = fixed
        .iter()
        .map(|&i| size_number(&children[i], "height"))
        .collect();
    let max_h = heights.iter().cloned().fold(0.0_f64, f64::max);
    let min_h = heights.iter().cloned().fold(f64::INFINITY, f64::min);
    if max_h <= 0.0 || min_h / max_h <= 0.5 {
        return; // heights too dissimilar → probably not a card row
    }
    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    for &i in &fixed {
        children[i]["width"] = json!("fill_container");
        children[i]["height"] = json!("fill_container");
    }
}

/// When a vertical group has a `fill_container` frame sibling, promote
/// fixed-width inputs to `fill_container` too (port of `normalizeFormInputWidths`).
fn normalize_form_input_widths(node: &mut Value) {
    if node.get("layout").and_then(Value::as_str) != Some("vertical") {
        return;
    }
    if node.get("width").and_then(Value::as_str) == Some("fit_content") {
        return;
    }
    let Some(children) = node.get("children").and_then(Value::as_array) else {
        return;
    };
    if children.len() < 2 {
        return;
    }
    let has_fill_sibling = children.iter().any(|c| {
        c.get("type").and_then(Value::as_str) == Some("frame")
            && c.get("width").and_then(Value::as_str) == Some("fill_container")
            && role_of(c) != Some("divider")
    });
    if !has_fill_sibling {
        return;
    }
    let targets: Vec<usize> = children
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            c.get("type").and_then(Value::as_str) == Some("frame")
                && matches!(role_of(c), Some("input") | Some("form-input"))
                && c.get("width").and_then(Value::as_f64).is_some()
        })
        .map(|(i, _)| i)
        .collect();
    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    for i in targets {
        children[i]["width"] = json!("fill_container");
    }
}

fn is_icon_like(node: &Value) -> bool {
    match node.get("type").and_then(Value::as_str) {
        Some("path") | Some("image") => true,
        Some("frame") => {
            if matches!(role_of(node), Some("icon") | Some("icon-button")) {
                return true;
            }
            let w = size_number(node, "width");
            let h = size_number(node, "height");
            w > 0.0 && h > 0.0 && w.max(h) <= 32.0
        }
        _ => false,
    }
}

/// In an input row with a trailing icon, make the text children `fill_container`
/// so the icon is pushed right while text stays left (port of
/// `normalizeInputTrailingIconAlignment`).
fn normalize_input_trailing_icon_alignment(node: &mut Value) {
    if !matches!(role_of(node), Some("input") | Some("form-input")) {
        return;
    }
    match node.get("justifyContent").and_then(Value::as_str) {
        None | Some("start") => {}
        _ => return,
    }
    let Some(children) = node.get("children").and_then(Value::as_array) else {
        return;
    };
    let visible: Vec<usize> = children
        .iter()
        .enumerate()
        .filter(|(_, c)| c.get("visible").and_then(Value::as_bool) != Some(false))
        .map(|(i, _)| i)
        .collect();
    if visible.len() < 2 {
        return;
    }
    let last = *visible.last().unwrap();
    if !is_icon_like(&children[last]) {
        return;
    }
    let text_idxs: Vec<usize> = visible[..visible.len() - 1]
        .iter()
        .copied()
        .filter(|&i| children[i].get("type").and_then(Value::as_str) == Some("text"))
        .collect();
    if text_idxs.is_empty() {
        return;
    }
    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    for i in text_idxs {
        if children[i].get("width").and_then(Value::as_str) != Some("fill_container") {
            children[i]["width"] = json!("fill_container");
        }
        if children[i].get("textGrowth").is_none() {
            children[i]["textGrowth"] = json!("fixed-width");
        }
    }
}

/// Clip a rounded frame that contains an image so the image respects the
/// corner radius (port of the `clipContent` branch of resolveTreePostPass).
fn apply_clip_content_for_image(node: &mut Value) {
    if node.get("clipContent").and_then(Value::as_bool) == Some(true) {
        return;
    }
    if corner_radius(node) <= 0.0 {
        return;
    }
    let has_image = node
        .get("children")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .any(|c| c.get("type").and_then(Value::as_str) == Some("image"))
        })
        .unwrap_or(false);
    if has_image {
        node["clipContent"] = json!(true);
    }
}

fn has_cta_descendant(node: &Value) -> bool {
    if matches!(
        role_of(node),
        Some("button") | Some("cta") | Some("primary-cta")
    ) {
        return true;
    }
    let label = identity_label(node);
    if label.contains("cta") || label.contains("order now") || label.contains("shop now") {
        return true;
    }
    if node.get("type").and_then(Value::as_str) == Some("text") {
        let content = node
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_lowercase();
        if content.contains("order now")
            || content.contains("buy now")
            || content.contains("shop now")
            || content.contains("立即")
            || content.contains("购买")
            || content.contains("下单")
        {
            return true;
        }
    }
    node.get("children")
        .and_then(Value::as_array)
        .map(|children| children.iter().any(has_cta_descendant))
        .unwrap_or(false)
}

fn is_promo_like_container(node: &Value) -> bool {
    let role = role_of(node).unwrap_or("");
    if matches!(
        role,
        "banner" | "feature-card" | "promo-card" | "offer-card"
    ) {
        return true;
    }
    let label = identity_label(node);
    ["promo", "offer", "deal", "discount", "limited", "banner"]
        .iter()
        .any(|needle| label.contains(needle))
}

fn relax_clipping_promo_height(node: &mut Value) {
    if node.get("type").and_then(Value::as_str) != Some("frame") {
        return;
    }
    if node.get("height").and_then(Value::as_f64).is_none() {
        return;
    }
    let clips = node.get("clipContent").and_then(Value::as_bool) == Some(true);
    if has_cta_descendant(node) && (clips || is_promo_like_container(node)) {
        node["height"] = json!("fit_content");
    }
}

fn normalize_horizontal_promo_copy_pane(node: &mut Value, canvas_width: f64) {
    if canvas_width > 480.0
        || node.get("type").and_then(Value::as_str) != Some("frame")
        || node.get("layout").and_then(Value::as_str) != Some("horizontal")
        || !is_promo_like_container(node)
    {
        return;
    }
    let Some(children) = node.get("children").and_then(Value::as_array) else {
        return;
    };
    if !children
        .iter()
        .any(|child| child.get("type").and_then(Value::as_str) == Some("image"))
    {
        return;
    }
    let copy_idx = children.iter().position(|child| {
        child.get("type").and_then(Value::as_str) == Some("frame")
            && child.get("layout").and_then(Value::as_str) == Some("vertical")
            && child
                .get("children")
                .and_then(Value::as_array)
                .map(|kids| {
                    kids.iter()
                        .any(|kid| kid.get("type").and_then(Value::as_str) == Some("text"))
                })
                .unwrap_or(false)
    });
    let Some(copy_idx) = copy_idx else {
        return;
    };
    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    let copy = &mut children[copy_idx];
    copy["width"] = json!("fill_container");
    copy["minWidth"] = json!(0);
    let Some(copy_children) = copy.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    for child in copy_children {
        if child.get("type").and_then(Value::as_str) != Some("text") {
            continue;
        }
        child["width"] = json!("fill_container");
        child["textGrowth"] = json!("fixed-width");
        if numeric_prop(child, "fontSize")
            .map(|size| size > 28.0)
            .unwrap_or(false)
        {
            child["fontSize"] = json!(28);
            child["lineHeight"] = json!(1.12);
        }
    }
}

// ── walk ──────────────────────────────────────────────────────────────────

fn post_pass_value(node: &mut Value, parent_fill: Option<Value>, canvas_width: f64) {
    if node.get("type").and_then(Value::as_str) != Some("frame") {
        return;
    }
    // I4 layout-property fixes (TS resolveTreePostPass order 1/2/3/4/6/8).
    // Still deferred: frame-height expansion (needs intrinsic measurement) and
    // placeholder icon repair (needs the icon catalog).
    equalize_card_row(node);
    fix_horizontal_overflow(node, canvas_width);
    normalize_form_input_widths(node);
    normalize_input_trailing_icon_alignment(node);
    normalize_nested_search_shell(node);
    normalize_favorite_icon_buttons(node);
    normalize_section_header_actions(node);
    normalize_mobile_category_section(node, canvas_width);
    normalize_mobile_category_row(node, canvas_width);
    normalize_mobile_product_card_row(node, canvas_width);
    normalize_mobile_featured_card_media(node, canvas_width);
    normalize_cart_count_badges(node);
    relax_clipping_promo_height(node);
    normalize_horizontal_promo_copy_pane(node, canvas_width);
    if node
        .get("layout")
        .and_then(Value::as_str)
        .map(|layout| layout != "none")
        .unwrap_or(false)
    {
        fix_text_heights(node);
    }
    apply_clip_content_for_image(node);
    // Strip unwanted white-card fills from structural wrappers (layout.md:24)
    // before the contrast fixes, so they evaluate the corrected transparent
    // section against the page bg instead of a bogus white surface.
    fix_structural_wrapper_transparency(node);
    // I3 contrast fixes (TS order 9-12).
    fix_button_foreground_contrast(node);
    fix_section_alternation(node);
    fix_orphan_container_contrast(node, parent_fill.as_ref());
    fix_input_sibling_consistency(node);

    // Recurse — children see THIS node's (possibly just-set) fill as their
    // parent fill, matching TS passing `currentRoot` down post-mutation.
    // `Some(Null)` distinguishes "parent exists but has no fill" (orphan fix
    // should still fire) from `None` = "no parent / root" (orphan fix skips).
    let this_fill = Some(node.get("fill").cloned().unwrap_or(Value::Null));
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children.iter_mut() {
            post_pass_value(child, this_fill.clone(), canvas_width);
        }
    }
}

/// Run the contrast post-pass over a forest of sub-agent section roots. Each
/// root is round-tripped through JSON; on any (de)serialize failure that root
/// is left untouched (a fix can never drop a node).
/// Semantic state-feedback tokens. Legit only on status/alert elements; glm
/// grabs them as "a light color" for decorative surfaces (a `$color-danger-bg`
/// search input renders pink and clashes with the theme).
const STATE_BG_REFS: &[&str] = &[
    "$color-danger-bg",
    "$color-info-bg",
    "$color-success-bg",
    "$color-warning-bg",
];

/// The page-background token. Only the page root paints it; an inner node using
/// it just repaints a redundant — or theme-clashing (cool `#F8FAFC` over a warm
/// page) — panel.
const PAGE_BG_REF: &str = "$color-bg-deep";

/// A status / feedback element — the ONLY legitimate user of a state-bg token.
fn is_status_element(node: &Value) -> bool {
    if let Some(role) = role_of(node) {
        if matches!(role, "badge" | "alert" | "toast" | "status") {
            return true;
        }
    }
    if let Some(name) = node.get("name").and_then(Value::as_str) {
        let l = name.to_lowercase();
        return [
            "error",
            "success",
            "warning",
            "alert",
            "danger",
            "status",
            "toast",
            "notification",
        ]
        .iter()
        .any(|k| l.contains(k));
    }
    false
}

/// Surface-color discipline — a deterministic floor walking EVERY node type
/// (incl. `text_input`, which the frame-only `post_pass_value` skips). The TS
/// pipeline relies on the prompt for this; weak models (glm-5.2) ignore it, so
/// Rust enforces it after the fact:
///   1. A state-bg token misused as a decorative surface → neutral
///      `$color-surface-2`. (the pink search input / chips)
///   2. `$color-bg-deep` on any inner node → transparent. (the cool grey panel
///      behind the search row / a nav tab repainting the page bg)
///
/// Refs are still UNRESOLVED here (binding runs later), so match token names.
fn node_has_effects(node: &Value) -> bool {
    node.get("effects")
        .and_then(Value::as_array)
        .map(|a| !a.is_empty())
        .unwrap_or(false)
}

fn has_any_stroke(node: &Value) -> bool {
    node.get("stroke").map(|s| !s.is_null()).unwrap_or(false)
}

fn fix_surface_color_discipline(node: &mut Value, is_root: bool) {
    if let Some(color) = get_first_solid_color(node) {
        if STATE_BG_REFS.contains(&color.as_str()) && !is_status_element(node) {
            node["fill"] = solid_fill("$color-surface-2");
        } else if !is_root && color == PAGE_BG_REF {
            node["fill"] = json!([]);
        } else if color.starts_with("$color-text-") && is_container_kind(node) {
            // A CONTAINER filled with a TEXT token is a slot-category error —
            // a search pill painted `$color-text-primary` rendered as a WHITE
            // capsule on the dark luxury theme (measured: ATELIER's search +
            // FILTER pills). Text tokens color glyphs; the container slot for
            // inputs/chips is surface-2. Its dark literal text (styled for
            // the accidental white) flips to the text ladder with it.
            node["fill"] = solid_fill("$color-surface-2");
            rebind_dark_literal_text(node);
        }
    }
    // An elevation shadow needs a surface to sit on. A frame with no visible
    // fill and no stroke that still carries a drop-shadow renders the shadow as
    // a gray "ghost box" floating around its children — strip it. This runs last
    // (after binding + every fill-stripping pass), so it sees the FINAL fill
    // state and catches both our own injected card shadows on wrappers that got
    // emptied and model-authored shadows on bare wrapper frames.
    if node_has_effects(node) && !has_visible_fill(node) && !has_any_stroke(node) {
        if let Some(obj) = node.as_object_mut() {
            obj.remove("effects");
        }
    }
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children.iter_mut() {
            fix_surface_color_discipline(child, false);
        }
    }
}

/// Container node kinds whose `fill` is a SURFACE slot (never a glyph color).
fn is_container_kind(node: &Value) -> bool {
    matches!(
        node.get("type").and_then(Value::as_str),
        Some("frame" | "group" | "rectangle" | "text_input")
    )
}

/// A COUNT BADGE (a painted chip whose only child is a 1-3 digit text — a
/// nav item's "12") reads as a stray square when the model omits its corner
/// radius; the badge convention is a pill. Only fires when `cornerRadius`
/// is ABSENT — an authored radius (0 included, the sharp-luxury look) is a
/// decision and stays.
fn round_count_badges(node: &mut Value) {
    let is_frame = node.get("type").and_then(Value::as_str) == Some("frame");
    if is_frame && node.get("cornerRadius").is_none() {
        let painted = node
            .get("fill")
            .map(|f| match f {
                Value::Array(a) => !a.is_empty(),
                Value::Null => false,
                _ => true,
            })
            .unwrap_or(false);
        let kids = node
            .get("children")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let lone_count_text = kids.len() == 1
            && kids[0].get("type").and_then(Value::as_str) == Some("text")
            && kids[0]
                .get("content")
                .and_then(Value::as_str)
                .map(str::trim)
                .is_some_and(|c| {
                    !c.is_empty()
                        && c.len() <= 3
                        && c.chars().all(|ch| ch.is_ascii_digit() || ch == '+')
                });
        if painted && lone_count_text {
            node["cornerRadius"] = json!(100.0);
        }
    }
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children.iter_mut() {
            round_count_badges(child);
        }
    }
}

/// After a container's accidental text-token fill flips to a surface, its
/// TEXT descendants styled for that light pill (dark literal hex) become
/// unreadable on the dark surface — walk them onto the text ladder.
fn rebind_dark_literal_text(node: &mut Value) {
    if node.get("type").and_then(Value::as_str) == Some("text") {
        if let Some(color) = get_first_solid_color(node) {
            if hex_luminance(&color).is_some_and(|l| l < 0.45) {
                node["fill"] = solid_fill("$color-text-muted");
            }
        }
    }
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children.iter_mut() {
            rebind_dark_literal_text(child);
        }
    }
}

pub fn post_pass_forest(nodes: &mut [PenNode], canvas_width: f64) {
    for node in nodes.iter_mut() {
        let Ok(mut v) = serde_json::to_value(&*node) else {
            continue;
        };
        post_pass_value(&mut v, None, canvas_width);
        if let Ok(new_node) = serde_json::from_value::<PenNode>(v) {
            *node = new_node;
        }
    }
}

/// Surface-color discipline over the whole forest. MUST run AFTER
/// `bind_generated_color_variables` — glm emits raw hex (`#FEE2E2`,
/// `#F8FAFC`), and binding is what normalizes those to the `$color-danger-bg`
/// / `$color-bg-deep` refs this pass matches. Calling it pre-binding (the old
/// site inside `post_pass_forest`) only ever saw hex, so it never matched and
/// the pink search input / cool-grey panels survived.
pub fn enforce_surface_color_discipline(nodes: &mut [PenNode]) {
    for node in nodes.iter_mut() {
        let Ok(mut v) = serde_json::to_value(&*node) else {
            continue;
        };
        fix_surface_color_discipline(&mut v, true);
        round_count_badges(&mut v);
        if let Ok(new_node) = serde_json::from_value::<PenNode>(v) {
            *node = new_node;
        }
    }
}

#[cfg(test)]
#[path = "role_post_pass_tests.rs"]
mod tests;
