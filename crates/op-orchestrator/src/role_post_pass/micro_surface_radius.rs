//! Semantic micro-surface rounding: pills, capsules, count badges and the
//! dark-literal text rebind.

use super::*;

/// Container node kinds whose `fill` is a SURFACE slot (never a glyph color).
pub(super) fn is_container_kind(node: &Value) -> bool {
    matches!(
        node.get("type").and_then(Value::as_str),
        Some("frame" | "group" | "rectangle" | "text_input")
    )
}

pub(super) const FULL_MICRO_SURFACE_RADIUS: f64 = 999.0;
pub(super) const MAX_MICRO_SURFACE_SHORT_AXIS: f64 = 64.0;

pub(super) fn name_words(node: &Value) -> Vec<String> {
    node.get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

pub(super) fn has_name_word(words: &[String], needle: &str) -> bool {
    words.iter().any(|word| word == needle)
}

pub(super) fn has_large_container_semantics(words: &[String]) -> bool {
    [
        "card", "cloud", "preview", "panel", "screen", "grid", "list",
    ]
    .iter()
    .any(|word| has_name_word(words, word))
}

pub(super) fn has_compact_padding(node: &Value) -> bool {
    let Some(padding) = node.get("padding") else {
        return true;
    };
    let values: Vec<f64> = match padding {
        Value::Number(value) => value.as_f64().into_iter().collect(),
        Value::Array(values) => values.iter().filter_map(Value::as_f64).collect(),
        _ => return false,
    };
    if values.is_empty() || values.iter().any(|value| *value < 0.0 || *value > 24.0) {
        return false;
    }
    let vertical = match values.as_slice() {
        [all] => all * 2.0,
        [vertical, _horizontal] => vertical * 2.0,
        [top, _right, bottom, _left] => top + bottom,
        _ => return false,
    };
    vertical <= 32.0
}

pub(super) fn has_compact_hug_anatomy(node: &Value) -> bool {
    if !matches!(
        node.get("layout").and_then(Value::as_str),
        None | Some("horizontal")
    ) || !has_compact_padding(node)
    {
        return false;
    }
    let Some(children) = node.get("children").and_then(Value::as_array) else {
        return false;
    };
    if children.is_empty() || children.len() > 3 {
        return false;
    }
    children.iter().all(|child| {
        let child_type = child.get("type").and_then(Value::as_str);
        if !matches!(
            child_type,
            Some("text" | "icon_font" | "path" | "image" | "ellipse")
        ) {
            return false;
        }
        if child_type == Some("text")
            && numeric_prop(child, "fontSize").is_some_and(|size| size > 24.0)
        {
            return false;
        }
        ["width", "height"].iter().all(|key| {
            numeric_prop(child, key)
                .map(|size| size > 0.0 && size <= MAX_MICRO_SURFACE_SHORT_AXIS)
                .unwrap_or(true)
        })
    })
}

pub(super) fn is_compact_capsule_surface(node: &Value, words: &[String], is_avatar: bool) -> bool {
    if has_large_container_semantics(words) {
        return false;
    }

    let width = numeric_prop(node, "width");
    let height = numeric_prop(node, "height");
    if width.is_some_and(|size| size <= 0.0) || height.is_some_and(|size| size <= 0.0) {
        return false;
    }

    if is_avatar {
        let (Some(width), Some(height)) = (width, height) else {
            return false;
        };
        let tolerance = 2.0_f64.max(width.max(height) * 0.15);
        return width <= MAX_MICRO_SURFACE_SHORT_AXIS
            && height <= MAX_MICRO_SURFACE_SHORT_AXIS
            && (width - height).abs() <= tolerance;
    }

    let width_hugs_or_fills = matches!(
        node.get("width").and_then(Value::as_str),
        Some("fit_content" | "fill_container")
    );
    let height_hugs = node.get("height").and_then(Value::as_str) == Some("fit_content");
    let width_supported = width.is_some() || width_hugs_or_fills || node.get("width").is_none();
    let height_supported = height.is_some() || height_hugs || node.get("height").is_none();
    if !width_supported || !height_supported {
        return false;
    }

    if let (Some(width), Some(height)) = (width, height) {
        return width.min(height) <= MAX_MICRO_SURFACE_SHORT_AXIS;
    }
    if width.is_some_and(|size| size > MAX_MICRO_SURFACE_SHORT_AXIS) && height_hugs
        || height.is_some_and(|size| size > MAX_MICRO_SURFACE_SHORT_AXIS) && width_hugs_or_fills
    {
        return false;
    }

    let needs_hug_proof = width.is_none() || height.is_none();
    !needs_hug_proof || has_compact_hug_anatomy(node)
}

pub(super) fn excludes_semantic_rounding(node: &Value, words: &[String]) -> bool {
    let role = role_of(node).unwrap_or("").to_ascii_lowercase();
    role.contains("nav")
        || matches!(
            role.as_str(),
            "status-bar" | "tab-bar" | "bottom-tab-bar" | "navbar"
        )
        || [
            "bar",
            "row",
            "container",
            "group",
            "wrapper",
            "section",
            "nav",
            "navigation",
        ]
        .iter()
        .any(|word| has_name_word(words, word))
}

pub(super) fn fixed_near_square_side(node: &Value) -> Option<f64> {
    let width = node.get("width").and_then(Value::as_f64)?;
    let height = node.get("height").and_then(Value::as_f64)?;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    let side = width.min(height);
    let tolerance = 2.0_f64.max(side * 0.15);
    ((width - height).abs() <= tolerance).then_some(side)
}

pub(super) fn is_explicit_rounded_card(node: &Value, words: &[String]) -> bool {
    if corner_radius(node) <= 0.0 {
        return false;
    }
    let role = role_of(node).unwrap_or("");
    let role_is_card = matches!(
        role,
        "card"
            | "stat-card"
            | "pricing-card"
            | "feature-card"
            | "image-card"
            | "product-card"
            | "restaurant-card"
            | "menu-card"
            | "testimonial"
    );
    let name = node
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    role_is_card || name == "card" || (name.ends_with(" card") && has_name_word(words, "card"))
}

/// Add only omitted micro-component radii when the node's semantics make a
/// square result unambiguously accidental. Explicit `cornerRadius: 0` remains
/// an authored sharp-style decision; structural bars/rows/containers/nav stay
/// untouched. Icon wells are more style-dependent, so they are rounded only
/// inside an already-rounded, explicitly card-like ancestor.
pub(super) fn round_missing_semantic_micro_surfaces(node: &mut Value, rounded_card_ancestor: bool) {
    let is_frame = node.get("type").and_then(Value::as_str) == Some("frame");
    let words = if is_frame {
        name_words(node)
    } else {
        Vec::new()
    };
    let this_is_rounded_card = is_frame && is_explicit_rounded_card(node, &words);
    let structural = is_frame && excludes_semantic_rounding(node, &words);
    let radius_missing = is_frame && node.get("cornerRadius").is_none();
    let painted = is_frame && has_visible_fill(node);

    if radius_missing && painted && !structural {
        let role = role_of(node).unwrap_or("");
        let capsule_semantics = matches!(role, "badge" | "pill" | "tag" | "avatar")
            || ["badge", "pill", "tag", "avatar"]
                .iter()
                .any(|word| has_name_word(&words, word));
        let avatar_semantics = role == "avatar" || has_name_word(&words, "avatar");
        if capsule_semantics && is_compact_capsule_surface(node, &words, avatar_semantics) {
            node["cornerRadius"] = json!(FULL_MICRO_SURFACE_RADIUS);
        } else {
            let status_indicator = role == "status"
                || (has_name_word(&words, "status")
                    && (has_name_word(&words, "dot") || has_name_word(&words, "indicator")))
                || (has_name_word(&words, "active") && has_name_word(&words, "indicator"));
            if status_indicator {
                if let Some(side) = fixed_near_square_side(node) {
                    node["cornerRadius"] = json!(side / 2.0);
                }
            } else {
                let exact_icon_box = node
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|name| name.trim().eq_ignore_ascii_case("Icon Box"));
                if rounded_card_ancestor && exact_icon_box {
                    if let Some(side) = fixed_near_square_side(node) {
                        node["cornerRadius"] = json!((side / 4.0).min(12.0));
                    }
                }
            }
        }
    }

    let child_has_rounded_card_ancestor = rounded_card_ancestor || this_is_rounded_card;
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children.iter_mut() {
            round_missing_semantic_micro_surfaces(child, child_has_rounded_card_ancestor);
        }
    }
}

/// A COUNT BADGE (a painted chip whose only child is a 1-3 digit text — a
/// nav item's "12") reads as a stray square when the model omits its corner
/// radius; the badge convention is a pill. Only fires when `cornerRadius`
/// is ABSENT — an authored radius (0 included, the sharp-luxury look) is a
/// decision and stays.
pub(super) fn round_count_badges(node: &mut Value) {
    let is_frame = node.get("type").and_then(Value::as_str) == Some("frame");
    let words = if is_frame {
        name_words(node)
    } else {
        Vec::new()
    };
    if is_frame
        && node.get("cornerRadius").is_none()
        && is_compact_capsule_surface(node, &words, false)
    {
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

/// True when `node` has at least one DIRECT child of type `text` — the
/// structural line between a tappable LABEL surface (button/badge: icon +
/// text, or text alone) and an icon-only tap target (avatar / icon-box),
/// which must never be swept into pill-rounding by this pass.
pub(super) fn has_direct_text_child(node: &Value) -> bool {
    node.get("children")
        .and_then(Value::as_array)
        .is_some_and(|children| {
            children
                .iter()
                .any(|child| child.get("type").and_then(Value::as_str) == Some("text"))
        })
}

/// Structural "compact painted capsule with text" shape shared by the
/// corner-rounding consistency gate and the missing-radius candidate
/// detector below. Reuses `is_compact_capsule_surface` — the same hug/size
/// anatomy `round_count_badges` keys off, which already handles BOTH
/// literal-pixel small frames AND `fit_content`-sized ones (real CTA
/// buttons are almost always the latter: padding + content, no authored
/// width/height) — plus an explicit text-child requirement so a text-less
/// icon-box/avatar is never mistaken for a label surface. Radius state is
/// checked separately by each caller — this only describes the anatomy.
pub(super) fn is_compact_painted_capsule_with_text(node: &Value) -> bool {
    if node.get("type").and_then(Value::as_str) != Some("frame")
        || !has_visible_fill(node)
        || !has_direct_text_child(node)
    {
        return false;
    }
    let words = name_words(node);
    is_compact_capsule_surface(node, &words, false)
}

/// Count `is_compact_painted_capsule_with_text` nodes that already carry an
/// authored `cornerRadius >= 6` anywhere in `node`'s subtree — the evidence
/// that THIS design's own convention is rounded compact surfaces.
pub(super) fn count_rounded_compact_capsules(node: &Value, out: &mut u32) {
    if is_compact_painted_capsule_with_text(node) && corner_radius(node) >= 6.0 {
        *out += 1;
    }
    if let Some(children) = node.get("children").and_then(Value::as_array) {
        for child in children {
            count_rounded_compact_capsules(child, out);
        }
    }
}

/// Structural fallback for CTA/pill corner rounding — a PAINTED, compact,
/// hug-anatomy frame carrying a text child (button/badge/pill) reads as a
/// tappable label surface, not a card or an icon-only tap target. No name
/// matching: a text-less icon-box fails `has_direct_text_child`, and a
/// large/loose container fails `is_compact_capsule_surface`'s own anatomy
/// bounds.
///
/// Gated on document consistency: fires only when this screen root already
/// has >= 2 OTHER compact painted capsules-with-text carrying an authored
/// `cornerRadius >= 6` — proof the design's own convention is rounded
/// compact surfaces, so an intentionally all-sharp-corners design system is
/// never touched by this pass.
pub(super) fn round_missing_compact_pill_radius(root: &mut Value) {
    let mut existing = 0u32;
    count_rounded_compact_capsules(root, &mut existing);
    if existing < 2 {
        return;
    }
    fn walk(node: &mut Value) {
        if node.get("cornerRadius").is_none() && is_compact_painted_capsule_with_text(node) {
            // Height is usually `fit_content` (no literal number) for a hug
            // button, matching the prompt guidance's "buttons 8-12" default;
            // when a literal height IS authored, stay under half of it so a
            // tall capsule doesn't get an accidental full-pill look.
            let radius = numeric_prop(node, "height")
                .map(|h| (h / 2.0).min(10.0))
                .unwrap_or(10.0);
            node["cornerRadius"] = json!(radius);
        }
        if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
            for child in children.iter_mut() {
                walk(child);
            }
        }
    }
    walk(root);
}

/// After a container's accidental text-token fill flips to a surface, its
/// TEXT descendants styled for that light pill (dark literal hex) become
/// unreadable on the dark surface — walk them onto the text ladder.
pub(super) fn rebind_dark_literal_text(node: &mut Value) {
    if node.get("type").and_then(Value::as_str) == Some("text") {
        if let Some(color) = get_first_solid_color(node) {
            if hex_luminance(&color).is_some_and(|l| l < 0.45) {
                node["fill"] = solid_fill("$--muted-foreground");
            }
        }
    }
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children.iter_mut() {
            rebind_dark_literal_text(child);
        }
    }
}
