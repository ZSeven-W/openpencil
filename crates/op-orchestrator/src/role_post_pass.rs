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

use crate::deck_echo::DeckEcho;
use crate::design_type::DesignForm;
use crate::role_layout_post_pass::{fix_horizontal_overflow, fix_text_heights, size_number};

// Sibling modules: this file keeps the public surface (`post_pass_forest` /
// `enforce_surface_color_discipline`) plus the tree walk; each cluster of
// fixes lives in its own module and is re-imported here so the walk (and the
// test modules mounted below) see the same flat namespace as before.
mod color_utils;
mod contrast;
mod layout_props;
mod micro_surface_radius;
mod mobile_cards;
mod mobile_category_row;
mod mobile_search_header;
mod surface_discipline;
mod surface_structure;

use color_utils::*;
use contrast::*;
use layout_props::*;
use micro_surface_radius::*;
use mobile_cards::*;
use mobile_category_row::*;
use mobile_search_header::*;
use surface_discipline::*;
use surface_structure::*;

// ── walk ──────────────────────────────────────────────────────────────────

fn post_pass_value(
    node: &mut Value,
    parent_fill: Option<Value>,
    canvas_width: f64,
    run_intent: bool,
    form: DesignForm,
    echoes: &mut Vec<DeckEcho>,
) {
    if node.get("type").and_then(Value::as_str) != Some("frame") {
        return;
    }
    // I4 layout-property fixes (TS resolveTreePostPass order 1/2/3/4/6/8).
    // Still deferred: frame-height expansion (needs intrinsic measurement) and
    // placeholder icon repair (needs the icon catalog).
    equalize_card_row(node);
    fix_horizontal_overflow(node, canvas_width, form, echoes);
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
    // …unless the document is authored template input, whose near-white card
    // surfaces are the design rather than a stray white-card fill
    // (`crate::repair_tier`, `TieredPass::StructuralWrapperTransparency`).
    if run_intent {
        fix_structural_wrapper_transparency(node);
    }
    // I3 contrast fixes (TS order 9-12).
    fix_button_foreground_contrast(node);
    fix_section_alternation(node);
    fix_orphan_container_contrast(node, parent_fill.as_ref());
    fix_deck_front_card_transparency(node);
    fix_container_text_contrast(node);
    fix_input_sibling_consistency(node);

    // Recurse — children see THIS node's (possibly just-set) fill as their
    // parent fill, matching TS passing `currentRoot` down post-mutation.
    // `Some(Null)` distinguishes "parent exists but has no fill" (orphan fix
    // should still fire) from `None` = "no parent / root" (orphan fix skips).
    let this_fill = Some(node.get("fill").cloned().unwrap_or(Value::Null));
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children.iter_mut() {
            post_pass_value(
                child,
                this_fill.clone(),
                canvas_width,
                run_intent,
                form,
                echoes,
            );
        }
    }
}

pub fn post_pass_forest(nodes: &mut [PenNode], canvas_width: f64) {
    post_pass_forest_with_tier(
        nodes,
        canvas_width,
        &crate::repair_tier::RepairTierPolicy::all(),
        DesignForm::Unknown,
    );
}

/// [`post_pass_forest`] under a repair-tier policy, for a known surface.
///
/// The walk is overwhelmingly contract-tier (overflow, text heights, contrast),
/// so it is not gated as a whole — only the one intent-tier fix inside it
/// (`fix_structural_wrapper_transparency`) answers to `policy`. Production
/// callers use this form; `post_pass_forest` is the full-tier shorthand for
/// tests and any caller with no document in hand.
///
/// `form` is the ROOT's form, not each section's: the forest handed here is a
/// list of sections that sit on a board / page / screen, and only the surface
/// they sit on decides whether clipping an overflowing row is acceptable. The
/// returned echoes are violations the walk deliberately did not repair (see
/// [`crate::deck_echo`]); a caller with nowhere to put them may drop them,
/// which is why they are returned rather than written anywhere here.
pub fn post_pass_forest_with_tier(
    nodes: &mut [PenNode],
    canvas_width: f64,
    policy: &crate::repair_tier::RepairTierPolicy,
    form: DesignForm,
) -> Vec<DeckEcho> {
    let run_intent =
        policy.runs_pass(crate::repair_tier::TieredPass::StructuralWrapperTransparency);
    let mut echoes = Vec::new();
    for node in nodes.iter_mut() {
        let Ok(mut v) = serde_json::to_value(&*node) else {
            continue;
        };
        post_pass_value(&mut v, None, canvas_width, run_intent, form, &mut echoes);
        if let Ok(new_node) = serde_json::from_value::<PenNode>(v) {
            *node = new_node;
        }
    }
    echoes
}

/// Surface-color discipline over the whole forest. MUST run AFTER
/// `bind_generated_color_variables` — glm emits raw hex (`#FEE2E2`,
/// `#F8FAFC`), and binding is what normalizes those to the `$--color-error`
/// / `$--background` refs this pass matches. Calling it pre-binding (the old
/// site inside `post_pass_forest`) only ever saw hex, so it never matched and
/// the pink search input / cool-grey panels survived.
pub fn enforce_surface_color_discipline(nodes: &mut [PenNode]) {
    enforce_surface_color_discipline_with_tier(nodes, &crate::repair_tier::RepairTierPolicy::all());
}

/// [`enforce_surface_color_discipline`] under a repair-tier policy.
///
/// `fix_surface_color_discipline` is intent-tier: a deliberate tonal band is
/// indistinguishable from a surface that redundantly repeats its parent's
/// token. The radius normalizers alongside it are contract-tier and keep
/// running either way — a count badge that is not round is a defect at any
/// provenance.
pub fn enforce_surface_color_discipline_with_tier(
    nodes: &mut [PenNode],
    policy: &crate::repair_tier::RepairTierPolicy,
) {
    let run_discipline = policy.runs_pass(crate::repair_tier::TieredPass::SurfaceColorDiscipline);
    for node in nodes.iter_mut() {
        let Ok(mut v) = serde_json::to_value(&*node) else {
            continue;
        };
        if run_discipline {
            fix_surface_color_discipline(&mut v, true);
        }
        round_missing_semantic_micro_surfaces(&mut v, false);
        round_count_badges(&mut v);
        round_missing_compact_pill_radius(&mut v);
        if let Ok(new_node) = serde_json::from_value::<PenNode>(v) {
            *node = new_node;
        }
    }
}

#[cfg(test)]
mod saturated_fill_contrast_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn preferred_foreground_uses_white_for_saturated_mid_tone_fills() {
        for color in ["#F97316", "#F59E0B", "#22C55E", "#2563EB", "#EA580C"] {
            assert_eq!(
                preferred_foreground_for_bg(color),
                "#FFFFFF",
                "{color} should use white foreground"
            );
        }
    }

    #[test]
    fn preferred_foreground_keeps_dark_for_pale_and_bright_fills() {
        for color in ["#FEF3C7", "#FDE047", "#F1F5F9", "#FFFFFF"] {
            assert_eq!(
                preferred_foreground_for_bg(color),
                "#0F172A",
                "{color} should use dark foreground"
            );
        }
    }

    #[test]
    fn raw_hex_saturated_orange_button_flips_dark_child_foreground_to_white() {
        let mut button = json!({
            "type": "frame",
            "role": "icon-button",
            "fill": [{"type": "solid", "color": "#F97316"}],
            "children": [{
                "type": "icon_font",
                "iconFontName": "sliders",
                "fill": [{"type": "solid", "color": "#0F172A"}]
            }]
        });

        fix_button_foreground_contrast(&mut button);

        assert_eq!(
            button["children"][0]["fill"],
            json!([{"type": "solid", "color": "#FFFFFF"}])
        );
    }

    #[test]
    fn pale_amber_button_keeps_dark_child_foreground() {
        let mut button = json!({
            "type": "frame",
            "role": "button",
            "fill": [{"type": "solid", "color": "#FEF3C7"}],
            "children": [{
                "type": "text",
                "content": "Details",
                "fill": [{"type": "solid", "color": "#0F172A"}]
            }]
        });

        fix_button_foreground_contrast(&mut button);

        assert_eq!(
            button["children"][0]["fill"],
            json!([{"type": "solid", "color": "#0F172A"}])
        );
    }

    #[test]
    fn light_text_on_light_chip_bg_gets_dark_foreground() {
        let bg = "#F5F1EA";
        let mut chip = json!({
            "type": "frame",
            "role": "badge",
            "fill": [{"type": "solid", "color": bg}],
            "children": [{
                "type": "text",
                "content": "May 15",
                "fill": [{"type": "solid", "color": "#EDE6DC"}]
            }]
        });

        fix_container_text_contrast(&mut chip);

        let preferred = preferred_foreground_for_bg(bg);
        assert_eq!(
            chip["children"][0]["fill"],
            json!([{"type": "solid", "color": preferred}])
        );
        assert!(!needs_luminance_contrast_override(preferred, bg));
    }

    #[test]
    fn well_contrasting_text_untouched() {
        let mut card = json!({
            "type": "frame",
            "role": "card",
            "fill": [{"type": "solid", "color": "#F5F1EA"}],
            "children": [{
                "type": "text",
                "content": "May 15",
                "fill": [{"type": "solid", "color": "#21140F"}]
            }]
        });

        fix_container_text_contrast(&mut card);

        assert_eq!(
            card["children"][0]["fill"],
            json!([{"type": "solid", "color": "#21140F"}])
        );
    }

    #[test]
    fn button_still_handled_by_existing_pass_not_double() {
        let mut button = json!({
            "type": "frame",
            "role": "button",
            "fill": [{"type": "solid", "color": "#F5F1EA"}],
            "children": [{
                "type": "text",
                "content": "May 15",
                "fill": [{"type": "solid", "color": "#EDE6DC"}]
            }]
        });

        fix_container_text_contrast(&mut button);

        assert_eq!(
            button["children"][0]["fill"],
            json!([{"type": "solid", "color": "#EDE6DC"}])
        );
    }

    #[test]
    fn no_solid_bg_container_skipped() {
        let mut chip = json!({
            "type": "frame",
            "role": "badge",
            "fill": [{"type": "solid", "color": "transparent"}],
            "children": [{
                "type": "text",
                "content": "May 15",
                "fill": [{"type": "solid", "color": "#EDE6DC"}]
            }]
        });

        fix_container_text_contrast(&mut chip);

        assert_eq!(
            chip["children"][0]["fill"],
            json!([{"type": "solid", "color": "#EDE6DC"}])
        );
    }
}

#[cfg(test)]
#[path = "role_post_pass_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "role_post_pass_contrast_tests.rs"]
mod contrast_tests;

#[cfg(test)]
#[path = "role_post_pass_mobile_tests.rs"]
mod mobile_tests;

#[cfg(test)]
#[path = "role_post_pass_deck_tests.rs"]
mod deck_tests;

#[cfg(test)]
#[path = "role_post_pass_pill_radius_tests.rs"]
mod pill_radius_tests;
