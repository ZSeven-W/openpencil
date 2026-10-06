//! Structural-surface repairs: transparent interior wrappers
//! (`fix_structural_wrapper_transparency`), orphan-container contrast rescue and
//! the `layout:none` deck front-card transparency fix.

use super::*;

pub(super) const STRUCTURAL_DENYLIST: &[&str] = &[
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
pub(super) const CARD_LIKE_ALLOWLIST: &[&str] = &[
    "card",
    "stat-card",
    "pricing-card",
    "feature-card",
    "image-card",
    "testimonial",
];

pub(super) fn is_ring_like_decorative(node: &Value) -> bool {
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
/// tokens (`$--primary`, dark bands, etc.) are deliberate → never listed here.
pub(super) const LIGHT_SURFACE_REFS: &[&str] =
    &["$--background", "$--card", "$--muted", "$--accent"];

/// True when a fill color reads as a light page/surface tone — either a parsed
/// hex with luminance ≥ 0.85, or one of the neutral surface variable refs that
/// glm emits before variable binding resolves them.
pub(super) fn is_light_surface_fill(color: &str) -> bool {
    LIGHT_SURFACE_REFS.contains(&color) || hex_luminance(color).map(|l| l >= 0.85).unwrap_or(false)
}

/// A child paints a colored (non-page-tone) surface: a gradient, or a solid
/// that is NOT a light page/surface tone. The signal that a child is a real
/// banner/card surface rather than a neutral panel.
pub(super) fn child_has_colored_fill(node: &Value) -> bool {
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
pub(super) fn is_redundant_colored_wrapper(node: &Value) -> bool {
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

pub(super) fn fix_structural_wrapper_transparency(node: &mut Value) {
    let Some(role) = role_of(node).map(str::to_string) else {
        return;
    };
    if CARD_LIKE_ALLOWLIST.contains(&role.as_str()) {
        // A card-like role normally keeps its fill — EXCEPT when it is a
        // redundant wrapper around a single full-bleed colored child card (glm
        // wraps an orange promo banner in a `feature-card` whose own
        // `$--card` fill then shows as a light box around / under the
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

pub(super) fn is_compact_control_semantics(node: &Value) -> bool {
    if role_of(node).is_some_and(|role| {
        matches!(
            role,
            "button"
                | "icon-button"
                | "badge"
                | "pill"
                | "tag"
                | "status"
                | "input"
                | "form-input"
                | "search-bar"
        )
    }) {
        return true;
    }
    let words = name_words(node);
    ["button", "badge", "pill", "tag", "input", "icon", "control"]
        .iter()
        .any(|word| has_name_word(&words, word))
}

pub(super) fn has_no_positive_padding(node: &Value) -> bool {
    match node.get("padding") {
        None | Some(Value::Null) => true,
        Some(Value::Number(value)) => value.as_f64().is_some_and(|padding| padding <= 0.0),
        Some(Value::Array(values)) => values
            .iter()
            .all(|value| value.as_f64().is_some_and(|padding| padding <= 0.0)),
        Some(_) => false,
    }
}

pub(super) fn absolute_child_covers_parent(parent: &Value, child: &Value) -> bool {
    if parent.get("layout").and_then(Value::as_str) != Some("none") {
        return false;
    }
    let (Some(parent_width), Some(parent_height)) = (
        numeric_prop(parent, "width"),
        numeric_prop(parent, "height"),
    ) else {
        return false;
    };
    let (Some(x), Some(y), Some(width), Some(height)) = (
        numeric_prop(child, "x"),
        numeric_prop(child, "y"),
        numeric_prop(child, "width"),
        numeric_prop(child, "height"),
    ) else {
        return false;
    };
    if parent_width <= 0.0 || parent_height <= 0.0 || width <= 0.0 || height <= 0.0 {
        return false;
    }
    let tolerance = 2.0;
    x >= -tolerance
        && y >= -tolerance
        && x <= tolerance
        && y <= tolerance
        && x + width >= parent_width - tolerance
        && y + height >= parent_height - tolerance
        && x + width <= parent_width + tolerance
        && y + height <= parent_height + tolerance
}

pub(super) fn flex_child_fills_parent(parent: &Value, child: &Value, child_count: usize) -> bool {
    parent.get("layout").and_then(Value::as_str) != Some("none")
        && child_count == 1
        && has_no_positive_padding(parent)
        && child.get("width").and_then(Value::as_str) == Some("fill_container")
        && child.get("height").and_then(Value::as_str) == Some("fill_container")
}

/// A painted child suppresses orphan-card rescue only when it can reasonably
/// be the wrapper's actual surface. In flow, this requires the sole child to
/// fill both axes inside a zero-padding wrapper. In `layout:none`, explicit
/// x/y/width/height must cover the parent bounds, so other children are true
/// overlays. Small controls, direct or nested, never satisfy either proof.
pub(super) fn has_near_full_bleed_painted_container_child(node: &Value) -> bool {
    let Some(children) = node.get("children").and_then(Value::as_array) else {
        return false;
    };

    children.iter().any(|child| {
        if !matches!(
            child.get("type").and_then(Value::as_str),
            Some("frame") | Some("group")
        ) || !has_visible_fill(child)
            || is_compact_control_semantics(child)
        {
            return false;
        }

        absolute_child_covers_parent(node, child)
            || flex_child_fills_parent(node, child, children.len())
    })
}

pub(super) fn effects_missing_or_null(node: &Value) -> bool {
    match node.get("effects") {
        None | Some(Value::Null) => true,
        Some(_) => false,
    }
}

pub(super) fn fix_orphan_container_contrast(node: &mut Value, parent_fill: Option<&Value>) {
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
    // A near-full-bleed child already paints the visible surface (e.g. glm
    // wraps an orange promo card plus an overlay badge in a bare
    // `feature-card`). A compact tag/button is content, not a replacement
    // surface, and must not suppress rescue of the outer card.
    if has_near_full_bleed_painted_container_child(node) {
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
    // Use the semantic surface token directly. A literal #FFFFFF can bind to
    // `$--background` when both variables share the light-mode value; the
    // later surface-discipline pass correctly strips page-bg tokens from inner
    // nodes, making this rescued card transparent again.
    node["fill"] = solid_fill("$--card");
    if effects_missing_or_null(node) {
        node["effects"] = json!([
            { "type": "shadow", "offsetX": 0, "offsetY": 1, "blur": 3, "spread": 0, "color": "#0000001A" },
            { "type": "shadow", "offsetX": 0, "offsetY": 1, "blur": 2, "spread": -1, "color": "#0000000F" }
        ]);
    }
}

// ── fixDeckFrontCardTransparency ─────────────────────────────────────────────

/// Explicit `x`/`y`/numeric `width`/`height` rect — only meaningful under
/// `layout: none`, where children are absolutely positioned instead of flowed.
pub(super) fn deck_layer_rect(node: &Value) -> Option<(f64, f64, f64, f64)> {
    let x = node.get("x").and_then(Value::as_f64)?;
    let y = node.get("y").and_then(Value::as_f64)?;
    let w = size_number(node, "width");
    let h = size_number(node, "height");
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    Some((x, y, w, h))
}

/// Intersection area is at least half of EACH rect's own area — mirrors the
/// empty-shell decorative-stack overlap threshold (`design_agent_tools.rs`)
/// so the two structural checks agree on what "substantially overlapping"
/// means for a `layout:none` card/deck stack.
pub(super) fn deck_rects_substantially_overlap(
    a: (f64, f64, f64, f64),
    b: (f64, f64, f64, f64),
) -> bool {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;
    let iw = (ax + aw).min(bx + bw) - ax.max(bx);
    let ih = (ay + ah).min(by + bh) - ay.max(by);
    if iw <= 0.0 || ih <= 0.0 {
        return false;
    }
    let overlap_area = iw * ih;
    overlap_area >= 0.5 * (aw * ah) && overlap_area >= 0.5 * (bw * bh)
}

/// A `layout:none` deck/stack's topmost card (`children[0]` — the canvas
/// scene paints siblings topmost-first, see `canvas_viewport_paint.rs`)
/// carrying text but no fill lets a painted sibling beneath it show straight
/// through the text whenever the two substantially overlap: a structural
/// fact (an unfilled text-bearing card sitting over a solid-fill sibling of
/// near-identical footprint always leaks), not a design-intent call, so it's
/// safe to auto-repair. Measured: 0724-1-gm-3.op's Flashcard Deck Stack,
/// where the front card's empty fill let "Back Layer 1"'s
/// `$--accent` bleed across the whole card. `frame`/`rectangle` only
/// (never `ellipse`) — a ring/donut sibling behind a centered label is a
/// deliberate see-through composition, not a leak.
pub(super) fn fix_deck_front_card_transparency(node: &mut Value) {
    if node.get("layout").and_then(Value::as_str) != Some("none") {
        return;
    }
    let Some(children) = node.get("children").and_then(Value::as_array) else {
        return;
    };
    if children.len() < 2 {
        return;
    }
    let front = &children[0];
    if front.get("type").and_then(Value::as_str) != Some("frame") {
        return;
    }
    if has_fill(front) || !has_text_descendant(front) {
        return;
    }
    let Some(front_rect) = deck_layer_rect(front) else {
        return;
    };
    // An image sibling anywhere in the stack means this is an image-backed
    // card (photo underneath, scrim in the middle, transparent content on
    // top) — the see-through front is the composition, not a leak. Filling
    // it would blot out the photo and its scrim entirely (measured:
    // 0728-gm.op's three theme-list cards, where the model's deliberate
    // photo + 3-stop alpha scrim + transparent content stack came back with
    // an opaque $--card lid and no visible image). Same rationale as
    // the ellipse exclusion above.
    if children
        .iter()
        .any(|sibling| sibling.get("type").and_then(Value::as_str) == Some("image"))
    {
        return;
    }
    let leaks_through = children.iter().skip(1).any(|sibling| {
        matches!(
            sibling.get("type").and_then(Value::as_str),
            Some("frame") | Some("rectangle")
        ) && has_visible_fill(sibling)
            && deck_layer_rect(sibling)
                .is_some_and(|rect| deck_rects_substantially_overlap(front_rect, rect))
    });
    if !leaks_through {
        return;
    }
    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    // Semantic token, not a literal hex — same convention as
    // `fix_orphan_container_contrast` (a later surface-discipline pass
    // resolves `$--card` against the active theme).
    children[0]["fill"] = solid_fill("$--card");
}

// ── normalizeNestedSearchShell ──────────────────────────────────────────────
