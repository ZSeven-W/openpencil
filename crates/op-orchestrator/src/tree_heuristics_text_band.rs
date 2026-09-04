//! `fix_invisible_text_band` (the glm banner-fill floor) and the
//! dominant-design-accent tally it shares with the nav rounding pass.
//!
//! This pass operates at **section scope** (see caller in tree_heuristics.rs).
//! The forest fed to apply_tree_heuristics contains only section-level roots
//! (page-root children or top-level nodes in flat layout), so the blast radius
//! of any section-repainting heuristic is that entire section. A missing bound
//! here becomes a full-screen fill, not a cosmetic mistake.

use super::*;

// ── fix_invisible_text_band (glm banner-fill floor, not a TS pass) ───────────
//
// glm intermittently designs a promo banner with WHITE text but omits the
// colored/gradient fill it intended (gen2/3/5: "Get 30% Off" white-on-cream =
// invisible). Deterministic floor: on a LIGHT page, a fill-less container whose
// text descendants are ALL white/light → it was meant to sit on a colored
// surface → stamp `$--primary` so the copy becomes readable. Conservative:
// fires only when every text is light (no dark text to contradict) and the
// container truly has no renderable fill.

// Light text tokens/hexes that vanish on a light page — white + the neutral
// surface tints (a banner headline written in any of these needs a colored
// surface beneath it).
pub(super) const LIGHT_TEXT_REFS: &[&str] = &[
    "$--card",
    "$--muted",
    "$--accent",
    "$--secondary",
    "$--popover",
    "$--sidebar",
    "$--background",
];
pub(super) const LIGHT_TEXT_HEXES: &[&str] = &["#ffffff", "#fff", "#fefefe", "#fdfdfd", "white"];

pub(super) fn is_light_text(color: &str) -> bool {
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
pub(super) fn tally_surface_text_colors(node: &Value, light: &mut usize, dark: &mut usize) {
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
/// token as the brand accent because the palette's `$--primary` defaults to
/// blue — wrong for a warm app). Returns the first chart/accent/primary token
/// found in the subtree, so the band matches the rest of the screen instead of
/// stamping a clashing default blue.
pub(super) fn find_design_accent(node: &Value) -> Option<String> {
    if let Some(c) = first_solid_color(node) {
        if is_accent_ref(&c) {
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
pub(super) fn is_light_surface_color(color: &str) -> bool {
    matches!(
        color,
        "$--card"
            | "$--muted"
            | "$--accent"
            | "$--secondary"
            | "$--popover"
            | "$--sidebar"
            | "$--background"
    ) || SAFE_LIGHT_HEXES.contains(&normalize_hex(color).as_str())
}

/// The brand/emphasis colour tokens: shadcn `--primary` (+ its sidebar
/// variant) and the chart ramp.
fn is_accent_ref(c: &str) -> bool {
    matches!(c, "$--primary" | "$--sidebar-primary") || c.starts_with("$--chart-")
}

pub(super) fn fix_invisible_text_band(node: &mut Value, theme: super::Theme, design_accent: &str) {
    // Guard B: unknown theme → abstain. When we can't tell if the page is light
    // or dark (unresolved tokens or unparseable hex), repainting on a guess
    // produced the full-bleed blue wall. Better to leave the node untouched.
    if theme == super::Theme::Unknown {
        return;
    }
    if theme != super::Theme::Light {
        return; // white text on a dark page is fine
    }
    if node.get("type").and_then(Value::as_str) != Some("frame") {
        return;
    }
    // Exempt status bars (role or name/id aliases, including Chinese): they are
    // fill-less by contract and their foreground is already derived from the root's
    // luminance, so "light text on no fill" is a false positive here. Stamping the
    // accent would turn OS chrome into a coloured band.
    if crate::cleanup::is_status_bar_from_json(node) {
        return;
    }
    // Skip only when the node ALREADY paints a non-light surface (a colored or
    // dark solid, or a gradient / image) — light text reads fine there. A node
    // with NO fill, OR a LIGHT-SURFACE solid fill (`$--card`, white), is a
    // band where light text is invisible. The latter is the broken-promo-banner
    // case: glm gives the card white text + a dark CTA + a translucent-white
    // badge (all implying a colored background) yet fills the card with
    // `$--card`, so the headline vanishes. Repaint with the design accent.
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
/// `$--primary` often defaults to a clashing blue). Counting across the
/// assembled-so-far page (passed by the caller from the doc sink) picks e.g.
/// `$--chart-6` when it's used 9× vs `$--primary` 1×, so an injected
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

pub(super) fn tally_accent(node: &Value, counts: &mut Vec<(String, usize)>) {
    if let Some(c) = first_solid_color(node) {
        if is_accent_ref(&c) {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Status bar with role="status-bar" and white 9:41 text under a light theme
    /// should NOT get the design_accent stamp.
    #[test]
    fn test_status_bar_role_exempt_from_text_band_stamp() {
        let mut bar = json!({
            "type": "frame",
            "id": "status-bar-001",
            "role": "status-bar",
            "children": [
                {
                    "type": "text",
                    "content": "9:41",
                    "fill": [{"type": "solid", "color": "#ffffffff"}]
                }
            ]
        });

        // Apply the pass: theme = Light, design_accent = "$--primary" (blue).
        fix_invisible_text_band(&mut bar, super::Theme::Light, "$--primary");

        // Status bar should have NO fill (exempt by role).
        assert!(
            bar.get("fill").is_none(),
            "Status bar with role should not receive fill stamp"
        );
    }

    /// Status bar identified by name only (no role) should also be exempt.
    #[test]
    fn test_status_bar_name_exempt_from_text_band_stamp() {
        let mut bar = json!({
            "type": "frame",
            "id": "bar-001",
            "name": "Status Bar",
            "children": [
                {
                    "type": "text",
                    "content": "9:41",
                    "fill": [{"type": "solid", "color": "#ffffffff"}]
                }
            ]
        });

        fix_invisible_text_band(&mut bar, super::Theme::Light, "$--primary");

        assert!(
            bar.get("fill").is_none(),
            "Status bar identified by name should not receive fill stamp"
        );
    }

    /// Non-status-bar frame with the same light-text shape SHOULD get the stamp.
    #[test]
    fn test_non_status_bar_with_light_text_gets_stamp() {
        let mut banner = json!({
            "type": "frame",
            "id": "banner-001",
            "name": "Promo Banner",
            "children": [
                {
                    "type": "text",
                    "content": "Get 30% Off",
                    "fill": [{"type": "solid", "color": "#ffffff"}]
                }
            ]
        });

        fix_invisible_text_band(&mut banner, super::Theme::Light, "$--primary");

        // Non-status-bar frame SHOULD get the accent fill.
        assert!(
            banner.get("fill").is_some(),
            "Non-status-bar frame with light text should receive fill stamp"
        );
        assert_eq!(
            banner["fill"][0]["color"], "$--primary",
            "Fill should be the design accent"
        );
    }

    /// Status bar with Chinese alias (e.g., 顶部状态栏) should be exempt.
    #[test]
    fn test_status_bar_chinese_name_exempt() {
        let mut bar = json!({
            "type": "frame",
            "id": "bar-001",
            "name": "顶部状态栏",
            "children": [
                {
                    "type": "text",
                    "content": "9:41",
                    "fill": [{"type": "solid", "color": "#ffffffff"}]
                }
            ]
        });

        fix_invisible_text_band(&mut bar, super::Theme::Light, "$--primary");

        assert!(
            bar.get("fill").is_none(),
            "Status bar with Chinese name should not receive fill stamp"
        );
    }

    /// Guard B: unknown theme → abstain. When the page background is an
    /// unresolved token (variables: null), we can't tell if the page is light or
    /// dark, so we must not repaint on a guess. The screen root should remain
    /// unchanged, preserving the model's original design intent.
    #[test]
    fn guard_b_unknown_theme_abstains_from_repaint() {
        let mut screen_root = json!({
            "type": "frame",
            "id": "root",
            "height": 844.0,
            "children": [
                {
                    "type": "text",
                    "content": "Hello",
                    "fill": [{"type": "solid", "color": "#ffffff"}]
                },
                {
                    "type": "text",
                    "content": "World",
                    "fill": [{"type": "solid", "color": "#fff"}]
                }
            ]
        });

        // Theme is Unknown (unresolved token or missing variables).
        fix_invisible_text_band(&mut screen_root, super::Theme::Unknown, "$--primary");

        // Screen root should NOT be repainted, even though all text is light.
        assert!(
            screen_root.get("fill").is_none(),
            "Unknown theme must abstain from repainting"
        );
    }

    /// Guard A: the measured case — a body section containing a card.
    /// The section should NOT be repainted because it has frame children
    /// (container structure), not text descendants on its own surface.
    #[test]
    fn guard_a_section_with_card_child_not_repainted() {
        // Measured failure structure: section holding a card.
        let mut section = json!({
            "type": "frame",
            "id": "body-section",
            "name": "Body",
            "height": "fit_content",
            "children": [
                {
                    "type": "frame",
                    "id": "card",
                    "name": "Card",
                    "fill": [{"type": "solid", "color": "$--card"}],
                    "children": [
                        {
                            "type": "text",
                            "content": "Card content",
                            "fill": [{"type": "solid", "color": "#ffffff"}]
                        }
                    ]
                }
            ]
        });

        fix_invisible_text_band(&mut section, super::Theme::Light, "$--primary");

        // Section with frame children is a container, not a band.
        assert!(
            section.get("fill").is_none(),
            "Section with frame child should not be repainted"
        );
    }

    /// Genuine band: text sits directly in the node, no nested frames with text.
    /// Should still be repainted when all text is light and surface fill is light.
    #[test]
    fn guard_a_genuine_band_with_direct_text_gets_stamp() {
        let mut banner = json!({
            "type": "frame",
            "id": "banner-001",
            "name": "Promo Banner",
            "fill": [{"type": "solid", "color": "$--card"}],
            "children": [
                {
                    "type": "text",
                    "content": "Get 30% Off",
                    "fill": [{"type": "solid", "color": "#ffffff"}]
                },
                {
                    "type": "text",
                    "content": "Limited time",
                    "fill": [{"type": "solid", "color": "#ffffff"}]
                }
            ]
        });

        fix_invisible_text_band(&mut banner, super::Theme::Light, "$--primary");

        // True band with only text children (no nested frames with text) should
        // be repainted when all text is light and surface fill is light.
        assert!(
            banner.get("fill").is_some(),
            "Genuine band with direct text should receive fill stamp"
        );
        assert_eq!(
            banner["fill"][0]["color"], "$--primary",
            "Fill should be the design accent"
        );
    }

    /// Row/grid with multiple cards: has frame children, so it's a structure,
    /// not a band. Should not be repainted.
    #[test]
    fn guard_a_grid_with_multiple_cards_not_repainted() {
        let mut grid = json!({
            "type": "frame",
            "id": "grid-container",
            "children": [
                {
                    "type": "frame",
                    "id": "card-1",
                    "fill": [{"type": "solid", "color": "$--card"}],
                    "children": [
                        {
                            "type": "text",
                            "content": "Card 1",
                            "fill": [{"type": "solid", "color": "#ffffff"}]
                        }
                    ]
                },
                {
                    "type": "frame",
                    "id": "card-2",
                    "fill": [{"type": "solid", "color": "$--card"}],
                    "children": [
                        {
                            "type": "text",
                            "content": "Card 2",
                            "fill": [{"type": "solid", "color": "#ffffff"}]
                        }
                    ]
                }
            ]
        });

        fix_invisible_text_band(&mut grid, super::Theme::Light, "$--primary");

        // Grid with frame children is a container structure, not a band.
        assert!(
            grid.get("fill").is_none(),
            "Grid with frame children should not be repainted"
        );
    }
}
