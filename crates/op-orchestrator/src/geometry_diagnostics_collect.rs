//! Diagnostic collectors — bottom-nav order / containment, the generic
//! tree walk and the rail-width-collapse diagnostics.

use super::*;

/// Structure echo: on a mobile-width root, any sibling that lands AFTER the
/// bottom tab bar is a misplaced "catch-up" section (measured: MiniMax-M3
/// appended the greeting+search header after the nav). The nav-last CONTRACT
/// is deterministically repaired at finalize (`anchor_bottom_nav_last`);
/// where the late section belongs is INTENT, so this echo tells the in-loop
/// model to relocate it with `M()` instead of us guessing.
pub(super) fn bottom_nav_order_diagnostic(root: &Value, out: &mut Vec<String>) {
    let width = root
        .get("width")
        .and_then(Value::as_f64)
        .unwrap_or(f64::MAX);
    if width > 480.0 {
        return;
    }
    let Some(children) = root.get("children").and_then(Value::as_array) else {
        return;
    };
    let is_nav = |c: &Value| {
        c.get("role").and_then(Value::as_str) == Some("bottom-tab-bar")
            || c.get("name").and_then(Value::as_str).is_some_and(|n| {
                let n = n.to_ascii_lowercase();
                n.contains("tab bar") || n.contains("bottom nav")
            })
    };
    let Some(nav_index) = children.iter().position(is_nav) else {
        return;
    };
    for late in children.iter().skip(nav_index + 1).filter(|c| !is_nav(c)) {
        if out.len() >= MAX_DIAGNOSTICS {
            return;
        }
        out.push(format!(
            "{}: sits AFTER the bottom tab bar — bottom navigation must be the LAST child. \
             If this is top context (greeting / search / page header), move it to the top of \
             the content with M(); do not leave it below the nav.",
            diag_label(late)
        ));
    }
}

/// A direct, in-flow bottom tab bar must resolve inside its mobile artboard.
///
/// `clipContent` normally suppresses overflow diagnostics because clipping is
/// often deliberate (avatars, cover images, carousels). A bottom navigation
/// that the root's own flow places below the root is different: clipping hides
/// required app chrome. Keep this exception deliberately narrow — mobile root,
/// direct child, bottom-specific semantics, and no authored absolute position.
pub(super) fn bottom_nav_root_containment_diagnostic(
    root: &Value,
    rects: &HashMap<String, Rect>,
    out: &mut Vec<String>,
) {
    const MOBILE_ROOT_MAX_WIDTH: f64 = 480.0;
    const BOUNDS_EPS: f64 = 1.0;

    if out.len() >= MAX_DIAGNOSTICS {
        return;
    }
    let Some(root_rect) = root
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| rects.get(id))
    else {
        return;
    };
    if root_rect.w > MOBILE_ROOT_MAX_WIDTH
        || root_rect.h < 500.0
        || root.get("layout").and_then(Value::as_str) != Some("vertical")
    {
        return;
    }

    let Some(nav) = children(root).last().filter(|child| {
        let is_bottom_nav = child.get("role").and_then(Value::as_str) == Some("bottom-tab-bar")
            || child
                .get("name")
                .and_then(Value::as_str)
                .is_some_and(|name| {
                    let name = name.to_ascii_lowercase();
                    name.contains("bottom tab")
                        || name.contains("bottom nav")
                        || name.contains("bottom navigation")
                });
        is_bottom_nav && !has_authored_position(child)
    }) else {
        return;
    };
    let Some(nav_rect) = nav
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| rects.get(id))
    else {
        return;
    };

    let root_bottom = root_rect.y + root_rect.h;
    let nav_bottom = nav_rect.y + nav_rect.h;
    if nav_bottom <= root_bottom + BOUNDS_EPS {
        return;
    }

    out.push(format!(
        "mobile-root-bottom-nav-overflow: {} is a direct flow bottom tab bar whose resolved bottom is {:.0}px past {}'s bottom. Keep the content wrapper Hug Height or grow the mobile root so the bottom navigation remains inside the artboard.",
        diag_label(nav),
        nav_bottom - root_bottom,
        diag_label(root)
    ));
}

/// `Name (id)` label for a diagnostic line.
pub(super) fn diag_label(v: &Value) -> String {
    let name = v.get("name").and_then(Value::as_str).unwrap_or("frame");
    let id = v.get("id").and_then(Value::as_str).unwrap_or("?");
    format!("{name} ({id})")
}

pub(super) fn collect_diagnostics(v: &Value, rects: &HashMap<String, Rect>, out: &mut Vec<String>) {
    collect_diagnostics_with_context(v, rects, out, false);
}

pub(super) fn collect_diagnostics_with_context(
    v: &Value,
    rects: &HashMap<String, Rect>,
    out: &mut Vec<String>,
    in_table: bool,
) {
    if out.len() >= MAX_DIAGNOSTICS {
        return;
    }
    if is_collapsed_fill_container(v, rects) {
        out.push(format!(
            "{}: declared height fill_container but resolved to ~0px (its ancestor hugs) — give the ancestor chain a definite height or use fit_content here",
            diag_label(v)
        ));
    }
    if out.len() < MAX_DIAGNOSTICS {
        let resolved_size = v
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| rects.get(id))
            .map(|r| (r.w, r.h));
        if let Some(line) = crate::stub_repair::empty_decorated_stub_diagnostic(v, resolved_size) {
            out.push(line);
        }
    }
    if out.len() < MAX_DIAGNOSTICS {
        for line in crate::sidebar_archetype::horizontal_navbar_archetype_diagnostics(v, |id| {
            rects.get(id).map(|r| r.w)
        }) {
            out.push(line);
            if out.len() >= MAX_DIAGNOSTICS {
                return;
            }
        }
    }
    if table_overflow_scale(v, rects).is_some() {
        out.push(format!(
            "{}: fixed column widths sum wider than the resolved row — shrink the column widths (or make columns fill_container) so they fit",
            diag_label(v)
        ));
    }
    if let Some((cols, inner)) = table_columns_exceed_width(v, rects) {
        out.push(format!(
            "{}: {cols} columns cannot fit a {inner}px row — no rescale can save this; drop columns (stack name+email in one cell) or widen the table",
            diag_label(v)
        ));
    }
    // Children overflowing a FLEX parent's resolved width. `clipContent`
    // parents are intentional croppers (scrollers, image masks) — skip them.
    let clips = v.get("clipContent").and_then(Value::as_bool) == Some(true);
    if matches!(layout_str(v), Some("vertical" | "horizontal")) && !clips {
        if let Some((parent_x, pw)) = v
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| rects.get(id))
            .map(|r| (r.x, r.w))
        {
            let compact_tail_id = compact_trailing_status_pair(v, rects)
                .and_then(|(_, tail)| tail.get("id").and_then(Value::as_str));
            for c in children(v) {
                if out.len() >= MAX_DIAGNOSTICS {
                    return;
                }
                let Some(cr) = c
                    .get("id")
                    .and_then(Value::as_str)
                    .and_then(|id| rects.get(id))
                else {
                    continue;
                };
                // Same trigger the fixers use: wider than the parent OR (for
                // containers) a right edge pushed past the parent's.
                let is_container = matches!(
                    c.get("type").and_then(Value::as_str),
                    Some("frame" | "group")
                );
                let past_right = is_container
                    && compact_tail_id != c.get("id").and_then(Value::as_str)
                    && cr.x + cr.w > parent_x + pw + TEXT_OVERFLOW_EPS;
                if cr.w <= pw + TEXT_OVERFLOW_EPS && !past_right {
                    continue;
                }
                if c.get("type").and_then(Value::as_str) == Some("text") {
                    let is_pill_with_numeric_width = crate::chip_repair::is_pill_chip(v)
                        && v.get("width")
                            .and_then(|w| match w {
                                Value::Number(n) => n.as_f64(),
                                Value::String(s) => s.parse::<f64>().ok(),
                                _ => None,
                            })
                            .is_some_and(|w| w > 0.0);
                    let is_hug_pill =
                        crate::chip_repair::is_pill_chip(v) && !is_pill_with_numeric_width;
                    if is_hug_pill || is_compact_status_badge(v, rects) {
                        continue;
                    }
                }
                if c.get("type").and_then(Value::as_str) == Some("text") {
                    out.push(format!(
                        "{}: text resolved {}px wide inside a {}px block — it overflows into siblings; set width fill_container + textGrowth fixed-width to wrap it",
                        diag_label(c),
                        cr.w.round(),
                        pw.round()
                    ));
                } else {
                    out.push(format!(
                        "{}: resolved {}px wide inside its {}px parent — it spills out; shrink its width (or use fill_container) so it fits",
                        diag_label(c),
                        cr.w.round(),
                        pw.round()
                    ));
                }
            }
        }
    }
    collect_vertical_spill_diagnostics(v, rects, out);
    collect_sibling_jam_diagnostics(v, rects, out);
    collect_starved_fill_diagnostics(v, rects, out);
    collect_rail_width_collapse_diagnostics_with_context(v, rects, out, in_table);
    let child_in_table = in_table || is_table_shape(v);
    for c in children(v) {
        collect_diagnostics_with_context(c, rects, out, child_in_table);
    }
}

/// Detect-only twin of [`collect_rail_width_collapse_fixes`] — same
/// reference-card / collapse-ratio condition, but reports instead of
/// fixing, so the `geometry_echo` in-loop step (which only ever runs the
/// DETECT half of this detector family) can catch the exact failure mode
/// that fixer exists for: a card rail whose `fill_container` siblings
/// collapsed beside a fixed-width reference card (root-cause fixture:
/// `geometry_rail_collapse_tests.rs`'s de-identified "Savings Goals" rail —
/// a 200px fixed card 1 starving cards 2/3 down to ~51px, truncating their
/// titles and ballooning their height from forced wrapping).
#[cfg(test)]
pub(super) fn collect_rail_width_collapse_diagnostics(
    v: &Value,
    rects: &HashMap<String, Rect>,
    out: &mut Vec<String>,
) {
    collect_rail_width_collapse_diagnostics_with_context(v, rects, out, false);
}

pub(super) fn collect_rail_width_collapse_diagnostics_with_context(
    v: &Value,
    rects: &HashMap<String, Rect>,
    out: &mut Vec<String>,
    in_table: bool,
) {
    if in_table || layout_str(v) != Some("horizontal") {
        return;
    }
    let cards = children(v);
    let Some(ref_w) = rail_reference_width(cards) else {
        return;
    };
    for c in cards {
        if out.len() >= MAX_DIAGNOSTICS {
            return;
        }
        if !is_rail_card_sibling(c)
            || is_compact_status_badge_structure(c)
            || c.get("width").and_then(Value::as_str) != Some("fill_container")
        {
            continue;
        }
        let Some(cid) = c.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(cr) = rects.get(cid) else {
            continue;
        };
        if cr.w <= 0.0 {
            continue;
        }
        if cr.w < RAIL_COLLAPSE_FLOOR && ref_w / cr.w > RAIL_COLLAPSE_RATIO {
            out.push(format!(
                "{}: fill_container sibling collapsed to {}px beside a {}px fixed-width \
                 reference card in the same row — widen it to match the reference (or make \
                 the reference fill_container too) so it isn't starved",
                diag_label(c),
                cr.w.round(),
                ref_w.round()
            ));
        }
    }
}
