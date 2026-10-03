//! Bottom-breathing-room echo for mobile screens.
//!
//! A phone screen whose last section ends flush against the artboard bottom
//! reads as cut off — the content collides with the device edge instead of
//! resting above it. Screens that end in a bottom navigation bar are the
//! deliberate exception: the nav IS the closing element and is supposed to sit
//! flush.
//!
//! The shared cleanup contract repairs a flush no-nav screen to 28px while
//! preserving every authored business node. The diagnostic uses the same
//! resolved-geometry predicate, so cleanup and echo cannot disagree.
//!
//! No name heuristics: the bottom-nav exception is decided from the authored
//! `role` semantic plus resolved geometry (full-width trailing band, nav-band
//! height, ≥3 evenly-sized tap targets in a horizontal row) — never from what
//! the model happened to call the frame.

use super::*;
use op_editor_core::PenNodeExt;

/// Widest root that still reads as a phone artboard — same threshold the
/// bottom-nav containment echo uses.
const MOBILE_ROOT_MAX_WIDTH: f64 = 480.0;
/// Full screens only. A short mobile-width root is a component, not a screen,
/// and a component is supposed to end at its content.
const MOBILE_ROOT_MIN_HEIGHT: f64 = 500.0;
/// Below this the last element is touching the bottom edge. Deliberately well
/// under the corpus minimum (24px) so the echo states a fact rather than
/// nagging about a slightly tight but clearly intentional inset.
const FLUSH_BOTTOM_GAP: f64 = 12.0;
/// Corpus-compliant minimum before cleanup leaves an authored gap alone.
const MIN_BOTTOM_GAP: f64 = 24.0;
/// Stable midpoint of the documented 24-32px breathing-room range.
const TARGET_BOTTOM_GAP: f64 = 28.0;
/// Height band a bottom navigation bar occupies. The corpus asks for 62-72px;
/// the band is widened at both ends so a nav that came out slightly short or
/// tall still earns its exception instead of drawing a false echo.
const NAV_HEIGHT_RANGE: std::ops::RangeInclusive<f64> = 44.0..=96.0;
/// A tab bar carries at least this many tap targets.
const NAV_MIN_TABS: usize = 3;
/// How far a tab may deviate from the mean tab width and still count as one
/// of an evenly-distributed row.
const NAV_TAB_WIDTH_TOLERANCE: f64 = 0.35;
const FULL_WIDTH_EPS: f64 = 1.0;

/// Echo a mobile screen whose content runs into the bottom edge with no
/// bottom navigation bar to close it.
pub(super) fn push_mobile_bottom_gap_diagnostic(
    root: &Value,
    rects: &HashMap<String, Rect>,
    out: &mut Vec<String>,
) {
    if out.len() >= MAX_DIAGNOSTICS {
        return;
    }
    let Some(gap) = resolved_mobile_bottom_gap(root, rects) else {
        return;
    };
    if gap >= FLUSH_BOTTOM_GAP {
        return;
    }
    out.push(format!(
        "mobile-bottom-flush: the last content edge under {} resolves {:.0}px from the \
         screen bottom, and this screen has no bottom navigation bar to close it — the \
         content runs into the device edge. Give the final section 24-32px of bottom \
         breathing room (bottom padding, or a trailing spacer frame).",
        diag_label(root),
        gap
    ));
}

/// Repair a no-nav mobile screen to the shared 28px bottom-room contract.
///
/// OpenPencil's post-layout reconciliation can grow an explicitly open numeric root
/// to include its content plus padding. Increasing only the root's bottom
/// padding therefore grows the resolved artboard without relocating any
/// business child. Existing compliant gaps, navigation chrome, desktop roots,
/// and expression-authored padding are left unchanged.
pub(crate) fn repair_mobile_bottom_breathing(sink: &mut dyn DocSink, root_id: &str) -> bool {
    let rects = resolved_rects(sink.state());
    let Some(root) = op_editor_core::walkers::find_node(
        sink.state().active_children(),
        &NodeId::new(root_id.to_string()),
    ) else {
        return false;
    };
    let Ok(value) = serde_json::to_value(root) else {
        return false;
    };
    // Implicit root clipping is an artboard viewport contract. Padding alone
    // cannot create room under fixed business children without changing that
    // viewport or cropping content; leave the diagnostic for a real reflow.
    if value.get("height").and_then(Value::as_f64).is_some()
        && value.get("clipContent").and_then(Value::as_bool) != Some(false)
    {
        return false;
    }
    let Some(gap) = resolved_mobile_bottom_gap(&value, &rects) else {
        return false;
    };
    if gap >= MIN_BOTTOM_GAP {
        return false;
    }
    let Some(mut padding) = numeric_padding_sides(&value) else {
        return false;
    };
    if padding[2] >= TARGET_BOTTOM_GAP {
        return false;
    }
    padding[2] = TARGET_BOTTOM_GAP;
    sink.apply(EditorCommand::PatchNodeData {
        node_id: NodeId::new(root_id.to_string()),
        patch_json: serde_json::json!({ "padding": padding }).to_string(),
        page_id: None,
    })
}

pub(crate) fn repair_mobile_bottom_breathing_for_all_roots(sink: &mut dyn DocSink) -> bool {
    let root_ids: Vec<String> = sink
        .state()
        .active_children()
        .iter()
        .map(|root| root.id_str().to_string())
        .collect();
    let mut changed = false;
    for root_id in root_ids {
        changed |= repair_mobile_bottom_breathing(sink, &root_id);
    }
    changed
}

fn resolved_mobile_bottom_gap(root: &Value, rects: &HashMap<String, Rect>) -> Option<f64> {
    let root_rect = resolved(root, rects)?;
    if root_rect.w > MOBILE_ROOT_MAX_WIDTH || root_rect.h < MOBILE_ROOT_MIN_HEIGHT {
        return None;
    }
    // Only a flow-laid screen has a meaningful "last content edge"; an
    // absolutely-positioned root stacks its children wherever it likes.
    if layout_str(root) != Some("vertical") {
        return None;
    }
    let kids = children(root);
    let last = kids.last()?;
    if is_bottom_nav_shape(last, &root_rect, rects) {
        return None;
    }
    // The lowest resolved edge across the root's direct children — not just
    // the last one in document order, since an overlay or a taller sibling can
    // be what actually reaches the bottom.
    let content_bottom = kids
        .iter()
        .filter_map(|child| resolved(child, rects))
        .map(|rect| rect.y + rect.h)
        .fold(f64::NEG_INFINITY, f64::max);
    if !content_bottom.is_finite() {
        return None;
    }
    let gap = root_rect.y + root_rect.h - content_bottom;
    // A negative gap is content OVERFLOWING the root — a different fact, and
    // the spill diagnostics already report it.
    if gap < 0.0 {
        return None;
    }
    Some(gap)
}

/// Does this trailing root child close the screen with bottom navigation?
///
/// Two independent sufficient signals, neither of which reads `name`:
/// the authored `role` semantic the corpus mandates, or the resolved shape a
/// tab bar always has — a full-width band in the nav height range laying out
/// at least three evenly-sized tap targets in a row. A generated screen may
/// keep that bar as the last child of one final content wrapper; treat that
/// one-level shape as the same closing fact so diagnostics, cleanup, and
/// interaction backfill cannot disagree.
pub(super) fn is_bottom_nav_shape(
    child: &Value,
    root_rect: &Rect,
    rects: &HashMap<String, Rect>,
) -> bool {
    if is_bottom_nav_surface_shape(child, root_rect, rects) {
        return true;
    }
    children(child)
        .last()
        .is_some_and(|nested| is_bottom_nav_surface_shape(nested, root_rect, rects))
}

fn is_bottom_nav_surface_shape(
    child: &Value,
    root_rect: &Rect,
    rects: &HashMap<String, Rect>,
) -> bool {
    if child.get("role").and_then(Value::as_str) == Some("bottom-tab-bar") {
        return true;
    }
    if layout_str(child) != Some("horizontal") {
        return false;
    }
    let Some(rect) = resolved(child, rects) else {
        return false;
    };
    if rect.w < root_rect.w - FULL_WIDTH_EPS || !NAV_HEIGHT_RANGE.contains(&rect.h) {
        return false;
    }
    let tabs: Vec<Rect> = children(child)
        .iter()
        .filter_map(|tab| resolved(tab, rects))
        .collect();
    if tabs.len() < NAV_MIN_TABS {
        return false;
    }
    let mean = tabs.iter().map(|tab| tab.w).sum::<f64>() / tabs.len() as f64;
    if mean <= 0.0 {
        return false;
    }
    tabs.iter()
        .all(|tab| (tab.w - mean).abs() <= mean * NAV_TAB_WIDTH_TOLERANCE)
}

fn resolved(v: &Value, rects: &HashMap<String, Rect>) -> Option<Rect> {
    v.get("id")
        .and_then(Value::as_str)
        .and_then(|id| rects.get(id))
        .copied()
}
