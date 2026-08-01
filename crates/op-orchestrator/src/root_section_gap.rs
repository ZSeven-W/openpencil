//! Give a page root the inter-section gap the planner would have given it.
//!
//! The orchestrator path sets this twice before a document is ever assembled:
//! `plan_normalize` writes `MOBILE_DEFAULT_ROOT_GAP` onto a mobile plan whose
//! root gap is missing or zero, and `scaffold::resolve_section_gap` falls back
//! to `SECTION_STACK_GAP` when building the root frame. The agentic tool-loop
//! goes through neither — the model builds its own root — so whether sections
//! breathe depends entirely on the model remembering to write `gap`.
//!
//! Measured across 117 shipped roots (2026-08-01): 67 carried gap 16, which is
//! `plan_normalize`'s value rather than anything a model chose, while every
//! root produced through the loop by a model that omits `gap` had none. It
//! reads as a per-model defect — one model was missing it 5 times out of 5 —
//! but it is a per-PATH one: the same model wrote a gap on the orchestrator
//! path, and a different model omitted it on the loop path. Any model that
//! does not write `gap` lands here.
//!
//! The repaired value matches the path that already works, split by device
//! exactly as the planner splits it, so this introduces no third default.

use serde_json::Value;

use crate::plan_normalize::{MOBILE_DEFAULT_ROOT_GAP, MOBILE_MAX_WIDTH};
use crate::scaffold::SECTION_STACK_GAP;

/// A root needs at least this many children before a missing gap is a defect.
///
/// One or two sections read as a deliberate composition (a hero over a form);
/// three or more stacked flush is the "no breathing room" look the planner's
/// own default exists to prevent.
const MIN_SECTIONS: usize = 3;

/// Repair a page root's missing inter-section gap. Returns whether it changed.
pub(crate) fn fix_root_section_gap(root: &mut Value) -> bool {
    if root.get("layout").and_then(Value::as_str) != Some("vertical") {
        return false;
    }
    // Only a genuinely absent or zero gap is repaired. An explicit positive
    // value is the author's, including one tighter than the default.
    let gap = root.get("gap").and_then(Value::as_f64);
    if gap.is_some_and(|value| value > 0.0) {
        return false;
    }
    let sections = root
        .get("children")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    if sections < MIN_SECTIONS {
        return false;
    }
    let width = root.get("width").and_then(Value::as_f64);
    // An unsized or `fill_container` root is the mobile artboard case in
    // practice; treating an unknown width as mobile matches how the planner
    // classifies (`width <= MOBILE_MAX_WIDTH`) once the value is known.
    let is_mobile = width.is_none_or(|value| value <= MOBILE_MAX_WIDTH);
    let repaired = if is_mobile {
        MOBILE_DEFAULT_ROOT_GAP
    } else {
        SECTION_STACK_GAP
    };
    let Some(object) = root.as_object_mut() else {
        return false;
    };
    object.insert("gap".into(), Value::from(repaired));
    true
}

#[cfg(test)]
#[path = "root_section_gap_tests.rs"]
mod root_section_gap_tests;
