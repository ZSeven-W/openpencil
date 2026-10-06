//! I3 contrast cluster — `fixButtonForegroundContrast`,
//! the container/descendant text-contrast fixes and `fixSectionAlternation`.

use super::*;

pub(super) fn fix_button_foreground_contrast(node: &mut Value) {
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
    // A brand-accent token (`$--primary` / `$--primary`) binds to a
    // concrete hex only at render time, so `resolve_color_maybe_ref` can't
    // read its luminance here and the pass used to bail — leaving the model's
    // default-dark icon on an orange accent button (measured: a `sliders`
    // icon at `#0F172A` on a `$--primary` filter button). These tokens are
    // always saturated colours that need a WHITE foreground, so treat the bg
    // as dark and let the same override logic below flip the children.
    let bg = match resolve_color_maybe_ref(&bg_raw) {
        Some(bg) if hex_luminance(&bg).is_some() => bg,
        Some(_) => return, // unparseable bg → can't pick safely, skip
        None if is_saturated_accent_token(&bg_raw) => "#EA580C".to_string(),
        None => return,
    };

    let fg = preferred_foreground_for_bg(&bg);
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

// ── fixContainerTextContrast ────────────────────────────────────────────────

pub(super) fn fix_container_text_contrast(node: &mut Value) {
    if matches!(role_of(node), Some("button") | Some("icon-button")) {
        return;
    }
    fix_container_text_contrast_for_current(node);
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children {
            fix_container_text_contrast(child);
        }
    }
}

pub(super) fn fix_container_text_contrast_for_current(node: &mut Value) {
    if node.get("type").and_then(Value::as_str) != Some("frame") {
        return;
    }
    if !has_visible_fill(node) {
        return;
    }
    let Some(bg_raw) = get_first_solid_color(node) else {
        return;
    };
    let Some(bg) = resolve_color_maybe_ref(&bg_raw) else {
        return;
    };
    if hex_luminance(&bg).is_none() {
        return;
    }

    let fg_fill = solid_fill(preferred_foreground_for_bg(&bg));
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children {
            fix_descendant_text_contrast(child, &bg, &fg_fill);
        }
    }
}

pub(super) fn fix_descendant_text_contrast(node: &mut Value, bg: &str, fg_fill: &Value) {
    if node.get("type").and_then(Value::as_str) == Some("text") {
        let existing =
            get_first_solid_color(node).and_then(|color| resolve_color_maybe_ref(&color));
        if existing
            .as_deref()
            .is_some_and(|fg| needs_luminance_contrast_override(fg, bg))
        {
            node["fill"] = fg_fill.clone();
        }
        return;
    }
    if matches!(role_of(node), Some("button") | Some("icon-button")) {
        return;
    }
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children {
            fix_descendant_text_contrast(child, bg, fg_fill);
        }
    }
}

// ── fixSectionAlternation ────────────────────────────────────────────────────

pub(super) const SECTION_ROLES: &[&str] =
    &["section", "hero", "cta-section", "stats-section", "footer"];
pub(super) const ALTERNATING_BG: [&str; 2] = ["#FFFFFF", "#F8FAFC"];

pub(super) fn fix_section_alternation(node: &mut Value) {
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
