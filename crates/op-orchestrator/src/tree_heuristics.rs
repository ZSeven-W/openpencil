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

use crate::role_defaults::Theme;
use jian_ops_schema::node::PenNode;
use serde_json::{json, Value};

// Pass submodules: this file keeps the shared tiny helpers and the forest
// entry point (`apply_tree_heuristics`); each heuristic pass lives in its own
// file and is re-imported here so the entry point (and the test module mounted
// below) see the same flat namespace as before.
#[path = "tree_heuristics_card_decoration.rs"]
mod tree_heuristics_card_decoration;
#[path = "tree_heuristics_fills.rs"]
mod tree_heuristics_fills;
#[path = "tree_heuristics_image_overlay.rs"]
mod tree_heuristics_image_overlay;
#[path = "tree_heuristics_nav_rounding.rs"]
mod tree_heuristics_nav_rounding;
#[path = "tree_heuristics_text_band.rs"]
mod tree_heuristics_text_band;

use tree_heuristics_card_decoration::*;
use tree_heuristics_fills::*;
use tree_heuristics_image_overlay::*;
use tree_heuristics_nav_rounding::*;
pub use tree_heuristics_text_band::*;

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

pub fn apply_tree_heuristics(
    nodes: &mut [PenNode],
    page_bg: Option<&str>,
    theme: Theme,
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
        .unwrap_or_else(|| "$--primary".to_string());
    for node in nodes.iter_mut() {
        let Ok(mut v) = serde_json::to_value(&*node) else {
            continue;
        };
        // Section-level (operate on the forest root = a page-root child).
        strip_redundant_section_fill(&mut v, page_bg);
        inject_nav_surface_for_section(&mut v);
        fix_invisible_text_band(&mut v, theme, &design_accent);
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
#[path = "tree_heuristics_media_tests.rs"]
mod media_tests;
#[cfg(test)]
#[path = "tree_heuristics_tests.rs"]
mod tests;
