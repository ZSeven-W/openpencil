//! Absolute-child clamp — shift a fixed-size control that pokes out of its
//! `layout: "none"` parent back INSIDE the parent's resolved box, instead of
//! letting the card-overflow clip fallback crop it. Its sibling rule, the
//! symmetric-inset SHRINK, covers the case a shift provably cannot fix: a
//! pinned child WIDER than the parent gets its authored left inset mirrored
//! on the right (`width = parent_w - 2·x`).
//!
//! Measured on a real cleanup run: a 44x44 "locate me" button pinned at
//! x=307 inside a 327px-wide map block (`layout: "none"`) hung past the
//! map's right edge. The only repair that saw it was
//! [`collect_card_overflow_clips`], which set `clipContent: true` on the map
//! and chopped the button's right half. The button FITS the map — it was
//! simply pinned too far — so the correct repair is to move it, not crop it.
//!
//! The shrink's measured case (app-18 fitness hero): a `space_between`
//! "Hero Top Controls" row authored 343px wide for a 375-wide full-bleed
//! hero, pinned at x=16 inside a hero stack that RESOLVED to 327px inside
//! the section's 24px side padding. 16 + 343 = 359 > 327 and 343 > 327, so
//! no shift can fit it and the clip fallback cut the bookmark button off
//! the right edge. Mirroring the inset (327 - 2·16 = 295) re-centres the
//! row and keeps both controls visible.
//!
//! Runs as its own cleanup steps BEFORE the geometry-validation loop
//! (checkpointed under `absolute-child-clamp` / `absolute-child-shrink`),
//! and again inside the loop right before the clip fallback for callers
//! that drive `geometry_validate_and_fix` directly. Either way the clip
//! fallback only ever sees the cases a shift cannot fix and a shrink must
//! refuse: children without a fixed size, children so far inset that
//! mirroring would crush them below half the parent's width, and
//! flex-layout overflow.
//!
//! Geometry edits go through `EditorCommand::UpdateNode` — the project rule
//! is that numeric geometry (`x` / numeric `width`) is set via `UpdateNode`,
//! never `SetNodeLayoutProp`.

use super::*;

/// Slack so a child a hair over the edge is not needlessly moved.
pub(super) const CLAMP_OVERFLOW_EPS: f64 = 2.0;
/// Tolerance for the "child fits the parent" gate — jian's resolved floats
/// can land a fraction past the authored edge on an exactly-filling child.
const FIT_EPS: f64 = 0.5;

/// A numeric (non-keyword) prop — `x`, `y`, `height`. `None` for absent,
/// null, or keyword (`fill_container` / `fit_content`) values. Widths use
/// the shared [`fixed_width`] reader.
fn numeric_prop(v: &Value, key: &str) -> Option<f64> {
    match v.get(key) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.parse::<f64>().ok(),
        _ => None,
    }
}

/// Collect an `UpdateNode` shift for every fixed-size, absolutely-positioned
/// child of a `layout: "none"` (or layout-less) parent whose resolved box
/// overflows the parent's right/bottom edge or sits at a negative offset.
pub(super) fn collect_absolute_child_clamp_fixes(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    // Only an ABSOLUTE parent pins children by x/y; a flex parent owns its
    // children's placement and the flex overflow fixers own its overflows.
    if matches!(layout_str(v), None | Some("none")) {
        if let Some(pr) = v
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| rects.get(id))
        {
            if pr.w > 1.0 && pr.h > 1.0 {
                for c in children(v) {
                    push_clamp_command(c, pr, rects, cmds);
                }
            }
        }
    }
    for c in children(v) {
        collect_absolute_child_clamp_fixes(c, rects, cmds);
    }
}

fn push_clamp_command(
    c: &Value,
    pr: &Rect,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    // The child must be absolutely pinned (numeric x/y) at a FIXED numeric
    // size; a keyword-sized child has no authored box to preserve.
    let (Some(x), Some(y), Some(_), Some(_)) = (
        numeric_prop(c, "x"),
        numeric_prop(c, "y"),
        fixed_width(c),
        numeric_prop(c, "height"),
    ) else {
        return;
    };
    let Some(cid) = c.get("id").and_then(Value::as_str) else {
        return;
    };
    let Some(cr) = rects.get(cid) else {
        return;
    };
    // A child bigger than its parent can never be shifted inside — leave it
    // to the clip fallback untouched.
    if cr.w > pr.w + FIT_EPS || cr.h > pr.h + FIT_EPS {
        return;
    }
    let overflows = cr.x + cr.w > pr.x + pr.w + CLAMP_OVERFLOW_EPS
        || cr.y + cr.h > pr.y + pr.h + CLAMP_OVERFLOW_EPS
        || cr.x < pr.x - CLAMP_OVERFLOW_EPS
        || cr.y < pr.y - CLAMP_OVERFLOW_EPS;
    if !overflows {
        return;
    }
    // Authored x/y are relative to the parent's origin; the parent's RESOLVED
    // box is the truth about how much room there is, so the fit position is
    // computed from resolved sizes, not the authored `fill_container` strings.
    let fit_x = x.min(pr.w - cr.w).max(0.0);
    let fit_y = y.min(pr.h - cr.h).max(0.0);
    let new_x = ((fit_x - x).abs() > FIT_EPS).then(|| fit_x.round() as i32);
    let new_y = ((fit_y - y).abs() > FIT_EPS).then(|| fit_y.round() as i32);
    if new_x.is_none() && new_y.is_none() {
        return;
    }
    cmds.push(EditorCommand::UpdateNode {
        node_id: NodeId::new(cid.to_string()),
        x: new_x,
        y: new_y,
        width: None,
        height: None,
        name: None,
        fill_hex: None,
        page_id: None,
    });
}

/// Shift every clampable absolute child under `root_id` back inside its
/// parent. Returns the number of edits the sink accepted.
pub(crate) fn clamp_absolute_children_into_parent(sink: &mut dyn DocSink, root_id: &str) -> usize {
    let rects = resolved_rects(sink.state());
    let cmds = {
        let Some(root) = op_editor_core::walkers::find_node(
            sink.state().active_children(),
            &NodeId::new(root_id.to_string()),
        ) else {
            return 0;
        };
        let Ok(v) = serde_json::to_value(root) else {
            return 0;
        };
        let mut cmds = Vec::new();
        collect_absolute_child_clamp_fixes(&v, &rects, &mut cmds);
        cmds
    };
    let mut applied = 0;
    for cmd in cmds {
        if sink.apply(cmd) {
            applied += 1;
        }
    }
    applied
}

// ── Symmetric-inset shrink ──

/// A shrunk child must keep at least this fraction of the parent's width.
/// Below it the mirrored inset would crush the control into a sliver, and
/// the honest repair is the clip fallback, not a resize.
const SHRINK_MIN_FRACTION: f64 = 0.5;

/// Collect an `UpdateNode` width shrink for every fixed-width child pinned
/// at a numeric `x >= 0` inside a `layout: "none"` (or layout-less) parent
/// when the child is WIDER than the parent's resolved width — the one case
/// the shift rule above provably cannot fix (its own `cr.w > pr.w` gate is
/// this rule's entry condition, so the two can never both claim a child).
///
/// The repair keeps the authored LEFT inset and mirrors it on the right
/// (`width = parent_w - 2·x`), re-centring a row authored for a wider
/// full-bleed parent instead of cropping its trailing edge. Height, x, and
/// y are never touched — vertical overflow of pinned rows is a different
/// story.
pub(super) fn collect_absolute_child_shrink_fixes(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    // Same parent gate as the shift rule: only an ABSOLUTE parent pins
    // children by x; a flex parent owns its children's placement.
    if matches!(layout_str(v), None | Some("none")) {
        if let Some(pr) = v
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| rects.get(id))
        {
            if pr.w > 1.0 {
                for c in children(v) {
                    push_shrink_command(c, pr, rects, cmds);
                }
            }
        }
    }
    for c in children(v) {
        collect_absolute_child_shrink_fixes(c, rects, cmds);
    }
}

fn push_shrink_command(
    c: &Value,
    pr: &Rect,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    // Status bar / tab bar chrome owns the full bleed; its width is a
    // contract the chrome passes enforce, never this rule's to rewrite.
    if is_full_bleed_chrome(c) {
        return;
    }
    // Only a non-negative numeric pin with a FIXED numeric width has an
    // authored box this rule can mirror; keyword widths and negative pins
    // are left to the clip fallback.
    let (Some(x), Some(w)) = (numeric_prop(c, "x"), fixed_width(c)) else {
        return;
    };
    if x < 0.0 {
        return;
    }
    let Some(cid) = c.get("id").and_then(Value::as_str) else {
        return;
    };
    let Some(cr) = rects.get(cid) else {
        return;
    };
    // The exact negation of the shift rule's fit gate, on the same resolved
    // widths: the child must be WIDER than its parent, or a shift can fix
    // it and this rule must stand down.
    if cr.w <= pr.w + FIT_EPS {
        return;
    }
    // The overflow itself, on the authored pin: the child's right edge must
    // provably cross the parent's resolved right edge.
    if x + w <= pr.w + CLAMP_OVERFLOW_EPS {
        return;
    }
    // Mirror the left inset on the right: x + new_w + x == parent width.
    let new_w = pr.w - 2.0 * x;
    // Below half the parent the mirrored inset would crush the child —
    // leave it to the clip fallback untouched.
    if new_w < pr.w * SHRINK_MIN_FRACTION {
        return;
    }
    cmds.push(EditorCommand::UpdateNode {
        node_id: NodeId::new(cid.to_string()),
        x: None,
        y: None,
        width: Some(new_w.round() as i32),
        height: None,
        name: None,
        fill_hex: None,
        page_id: None,
    });
}

/// Status-bar / tab-bar chrome by role, plus the status-bar name aliases.
fn is_full_bleed_chrome(v: &Value) -> bool {
    crate::cleanup::is_status_bar_from_json(v)
        || matches!(
            v.get("role").and_then(Value::as_str),
            Some("tab-bar" | "bottom-tab-bar" | "tab-row")
        )
}

/// Shrink every wider-than-parent absolute child under `root_id` to its
/// mirrored-inset width. Returns the number of edits the sink accepted.
pub(crate) fn shrink_oversized_absolute_children_into_parent(
    sink: &mut dyn DocSink,
    root_id: &str,
) -> usize {
    let rects = resolved_rects(sink.state());
    let cmds = {
        let Some(root) = op_editor_core::walkers::find_node(
            sink.state().active_children(),
            &NodeId::new(root_id.to_string()),
        ) else {
            return 0;
        };
        let Ok(v) = serde_json::to_value(root) else {
            return 0;
        };
        let mut cmds = Vec::new();
        collect_absolute_child_shrink_fixes(&v, &rects, &mut cmds);
        cmds
    };
    let mut applied = 0;
    for cmd in cmds {
        if sink.apply(cmd) {
            applied += 1;
        }
    }
    applied
}

#[cfg(test)]
#[path = "absolute_child_clamp_tests.rs"]
mod tests;
