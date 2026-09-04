//! Detect sibling bar charts whose varying heights lack baseline alignment.
//!
//! Detects three cases:
//! 1. Bottoms aligned (±1) — correct, no issue
//! 2. Tops aligned (±1), bottoms vary — top-anchored bars, emit TopAnchoredBars
//! 3. Neither aligned, not waterfall — staggered floats, emit NoBaselineBars
//!
//! Both cases 2 and 3 suggest aligning bottoms. Case 2 requires axis evidence
//! for auto-fix; case 3 is contract-tier without axis.
//! Waterfalls (consecutive bars chaining by top or bottom) are intentional
//! and emit nothing.

use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::PenFill;
use serde_json::{json, Value};

use crate::issue::{FixProperty, Issue, IssueCategory, IssueSeverity};
use crate::node_util::{
    children, node_fills, node_id, node_kind, node_x, node_y, numeric_height, numeric_width,
    stroke_thickness_max, NodeKind,
};

const WIDTH_EPSILON: f64 = 1.0;
const GAP_EPSILON: f64 = 2.0;
const Y_EPSILON: f64 = 1.0;
const CHAINING_EPSILON: f64 = 2.0;
const AXIS_Y_EPSILON: f64 = 4.0;
const AXIS_COVERAGE: f64 = 0.80;
const HEIGHT_RATIO: f64 = 1.10;

#[derive(Clone, Copy)]
struct Bar<'a> {
    node: &'a PenNode,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Clone, Copy, Debug)]
enum BarGroupClass {
    BottomsAligned,
    TopsAligned,
    NoBaseline,
}

/// Find bar groups that lack proper baseline alignment.
///
/// Detects three cases: bottoms aligned (correct, no issue), tops aligned with
/// axis evidence (auto-fix via bottom alignment), and staggered floats with no
/// chaining (waterfall guard against intentional designs). Case 2 is
/// TopAnchoredBars; case 3 is NoBaselineBars and contract-tier.
pub fn detect_top_anchored_bars(root: &PenNode) -> Vec<Issue> {
    let mut issues = Vec::new();
    walk_bar_parents(root, &mut issues);
    issues
}

fn walk_bar_parents(node: &PenNode, issues: &mut Vec<Issue>) {
    for group in find_bar_groups(children(node)) {
        if let Some(class) = classify_group(&group) {
            match class {
                BarGroupClass::BottomsAligned => {
                    // Correct alignment; emit nothing.
                }
                BarGroupClass::TopsAligned => {
                    let has_axis = has_axis_evidence(node, &group);
                    let base = group
                        .iter()
                        .map(|bar| bar.y + bar.height)
                        .fold(f64::NEG_INFINITY, f64::max);
                    for bar in group {
                        issues.push(bar_issue_top_anchored(bar, base, has_axis));
                    }
                }
                BarGroupClass::NoBaseline => {
                    let base = group
                        .iter()
                        .map(|bar| bar.y + bar.height)
                        .fold(f64::NEG_INFINITY, f64::max);
                    for bar in group {
                        // No baseline case: auto-fix without axis evidence.
                        issues.push(bar_issue_no_baseline(bar, base));
                    }
                }
            }
        }
    }
    for child in children(node) {
        walk_bar_parents(child, issues);
    }
}

fn find_bar_groups<'a>(siblings: &'a [PenNode]) -> Vec<Vec<Bar<'a>>> {
    let mut bars: Vec<Bar<'a>> = siblings.iter().filter_map(bar_geometry).collect();
    bars.sort_by(|left, right| left.x.total_cmp(&right.x));

    let mut groups = Vec::new();
    let mut start = 0;
    while start < bars.len() {
        let mut group = vec![bars[start]];
        let mut expected_gap: Option<f64> = None;
        let mut cursor = start + 1;
        while cursor < bars.len() {
            let candidate = bars[cursor];
            if !same_bar_width(&group, candidate) {
                break;
            }
            let previous = *group.last().expect("group starts with one bar");
            let gap = candidate.x - (previous.x + previous.width);
            if let Some(expected) = expected_gap {
                if (gap - expected).abs() > GAP_EPSILON {
                    break;
                }
            } else {
                expected_gap = Some(gap);
            }
            group.push(candidate);
            cursor += 1;
        }

        if group.len() >= 3 && has_varying_heights(&group) {
            groups.push(group);
        }
        // A complete run is consumed at once. For a short run, advance by one
        // so a later node can still begin a valid group.
        start = if cursor > start + 1 {
            cursor
        } else {
            start + 1
        };
    }
    groups
}

fn bar_geometry(node: &PenNode) -> Option<Bar<'_>> {
    if node_kind(node) != NodeKind::Rectangle
        || !children(node).is_empty()
        || !has_visible_fill(node)
    {
        return None;
    }
    Some(Bar {
        node,
        x: node_x(node)?,
        y: node_y(node)?,
        width: numeric_width(node)?,
        height: numeric_height(node)?,
    })
}

/// Check if all bars in a group share the same width (±WIDTH_EPSILON).
/// This generalizes the width requirement, dropping the y-alignment check
/// so we can classify groups into three cases afterward.
fn same_bar_width(group: &[Bar<'_>], candidate: Bar<'_>) -> bool {
    group
        .iter()
        .all(|bar| (bar.width - candidate.width).abs() <= WIDTH_EPSILON)
}

/// Classify a bar group into one of three baseline cases.
/// Returns None only if the group is a waterfall (intentional, not a bug).
fn classify_group(group: &[Bar<'_>]) -> Option<BarGroupClass> {
    if is_bottoms_aligned(group) {
        Some(BarGroupClass::BottomsAligned)
    } else if is_tops_aligned(group) {
        Some(BarGroupClass::TopsAligned)
    } else if !is_waterfall_chart(group) {
        Some(BarGroupClass::NoBaseline)
    } else {
        None
    }
}

/// Check if all bars share the same bottom edge (±Y_EPSILON).
fn is_bottoms_aligned(group: &[Bar<'_>]) -> bool {
    let first_bottom = group[0].y + group[0].height;
    group
        .iter()
        .all(|bar| (bar.y + bar.height - first_bottom).abs() <= Y_EPSILON)
}

/// Check if all bars share the same top edge (±Y_EPSILON).
fn is_tops_aligned(group: &[Bar<'_>]) -> bool {
    let first_y = group[0].y;
    group.iter().all(|bar| (bar.y - first_y).abs() <= Y_EPSILON)
}

/// Check if this is a waterfall chart (consecutive bars intentionally chain).
///
/// A waterfall has majority of consecutive pairs where bar[i+1]'s top or bottom
/// aligns within CHAINING_EPSILON of bar[i]'s top or bottom. If true, the
/// staggered float layout is intentional and should not be flagged.
fn is_waterfall_chart(group: &[Bar<'_>]) -> bool {
    if group.len() < 2 {
        return false;
    }
    let mut chained_count = 0;
    for i in 0..group.len() - 1 {
        let bar_i = group[i];
        let bar_next = group[i + 1];
        let this_top = bar_i.y;
        let this_bottom = bar_i.y + bar_i.height;
        let next_top = bar_next.y;
        let next_bottom = bar_next.y + bar_next.height;
        // Chain if next top aligns with this bottom, or next bottom with this bottom,
        // or next top with this top — any consecutive edge pairing within tolerance.
        if (next_top - this_bottom).abs() <= CHAINING_EPSILON
            || (next_bottom - this_bottom).abs() <= CHAINING_EPSILON
            || (next_top - this_top).abs() <= CHAINING_EPSILON
        {
            chained_count += 1;
        }
    }
    // If majority of consecutive pairs chain, treat as intentional waterfall.
    chained_count * 2 > group.len() - 1
}

fn has_varying_heights(group: &[Bar<'_>]) -> bool {
    let min_height = group
        .iter()
        .map(|bar| bar.height)
        .fold(f64::INFINITY, f64::min);
    let max_height = group
        .iter()
        .map(|bar| bar.height)
        .fold(f64::NEG_INFINITY, f64::max);
    min_height > 0.0 && max_height / min_height > HEIGHT_RATIO
}

fn has_axis_evidence(parent: &PenNode, group: &[Bar<'_>]) -> bool {
    let group_start = group.iter().map(|bar| bar.x).fold(f64::INFINITY, f64::min);
    let group_end = group
        .iter()
        .map(|bar| bar.x + bar.width)
        .fold(f64::NEG_INFINITY, f64::max);
    let group_width = group_end - group_start;
    let top = group[0].y;
    if group_width <= 0.0 {
        return false;
    }

    children(parent)
        .iter()
        .filter_map(horizontal_axis)
        .any(|axis| {
            if (axis.y - top).abs() > AXIS_Y_EPSILON {
                return false;
            }
            let overlap = (axis.end.min(group_end) - axis.start.max(group_start)).max(0.0);
            overlap / group_width >= AXIS_COVERAGE
        })
}

struct HorizontalAxis {
    start: f64,
    end: f64,
    y: f64,
}

fn horizontal_axis(node: &PenNode) -> Option<HorizontalAxis> {
    match node {
        PenNode::Line(line) => {
            let stroke = line.stroke.as_ref()?;
            if stroke_thickness_max(&stroke.thickness) > 4.0 {
                return None;
            }
            let x = line.base.x?;
            let y = line.base.y?;
            let x2 = line.x2?;
            let y2 = line.y2?;
            if (y2 - y).abs() > AXIS_Y_EPSILON || (x2 - x).abs() <= f64::EPSILON {
                return None;
            }
            Some(HorizontalAxis {
                start: x.min(x2),
                end: x.max(x2),
                y: (y + y2) / 2.0,
            })
        }
        PenNode::Rectangle(_) => {
            if !has_visible_fill(node) {
                return None;
            }
            let x = node_x(node)?;
            let y = node_y(node)?;
            let width = numeric_width(node)?;
            let height = numeric_height(node)?;
            if height > 4.0 || width < height {
                return None;
            }
            Some(HorizontalAxis {
                start: x,
                end: x + width,
                y: y + height / 2.0,
            })
        }
        _ => None,
    }
}

/// Issue for top-anchored bars (case 2): tops aligned, bottoms vary.
/// Requires axis evidence to auto-fix.
fn bar_issue_top_anchored(bar: Bar<'_>, base: f64, has_axis: bool) -> Issue {
    let suggested_y = base - bar.height;
    Issue {
        node_id: node_id(bar.node).to_string(),
        category: IssueCategory::TopAnchoredBars,
        severity: IssueSeverity::Warning,
        property: FixProperty::Y,
        current_value: json!(bar.y),
        suggested_value: if has_axis {
            json!(suggested_y)
        } else {
            Value::Null
        },
        reason: if has_axis {
            format!(
                "bar is top-anchored at y={:.2}; axis evidence supports baseline y={:.2}",
                bar.y, suggested_y
            )
        } else {
            format!(
                "bar group has varying heights but no axis evidence; review top anchoring at y={:.2}",
                bar.y
            )
        },
    }
}

/// Issue for no-baseline bars (case 3): neither aligned, not waterfall.
/// Auto-fixes without requiring axis evidence — staggered floats are not
/// an intentional layout, so the fix is contract-tier.
fn bar_issue_no_baseline(bar: Bar<'_>, base: f64) -> Issue {
    let suggested_y = base - bar.height;
    Issue {
        node_id: node_id(bar.node).to_string(),
        category: IssueCategory::NoBaselineBars,
        severity: IssueSeverity::Warning,
        property: FixProperty::Y,
        current_value: json!(bar.y),
        suggested_value: json!(suggested_y),
        reason: format!(
            "bar floats at y={:.2} with no baseline alignment; suggest y={:.2} (bottom-aligned)",
            bar.y, suggested_y
        ),
    }
}

fn has_visible_fill(node: &PenNode) -> bool {
    node_fills(node).is_some_and(|fills| {
        fills.iter().any(|fill| match fill {
            PenFill::Solid(body) => {
                body.opacity.unwrap_or(1.0) > 0.0 && !is_transparent_color(&body.color)
            }
            _ => true,
        })
    })
}

fn is_transparent_color(color: &str) -> bool {
    let color = color.trim();
    if color.eq_ignore_ascii_case("transparent") {
        return true;
    }
    color.len() == 9
        && color.is_ascii()
        && color.starts_with('#')
        && u8::from_str_radix(&color[7..9], 16).is_ok_and(|alpha| alpha == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn node(value: serde_json::Value) -> PenNode {
        serde_json::from_value(value).expect("fixture must deserialize as PenNode")
    }

    fn bar(id: &str, x: f64, height: f64) -> serde_json::Value {
        json!({
            "type":"rectangle", "id":id, "x":x, "y":380, "width":96, "height":height,
            "fill":[{"type":"solid","color":"#266EA4"}]
        })
    }

    fn chart(children: serde_json::Value) -> PenNode {
        node(json!({
            "type":"frame", "id":"chart", "width":1920, "height":1080,
            "children":children
        }))
    }

    #[test]
    fn flags_varying_top_anchored_bars_with_axis() {
        let root = chart(json!([
            {"type":"rectangle", "id":"axis", "x":120, "y":378, "width":1120, "height":2,
             "fill":[{"type":"solid","color":"#D3D1CD"}]},
            bar("bar-a", 150.0, 284.0),
            bar("bar-b", 326.0, 316.0),
            bar("bar-c", 502.0, 342.0)
        ]));
        let issues = detect_top_anchored_bars(&root);
        assert_eq!(issues.len(), 3);
        assert!(issues.iter().all(|issue| {
            issue.category == IssueCategory::TopAnchoredBars
                && issue.property == FixProperty::Y
                && issue.suggested_value.is_number()
        }));
        assert_eq!(issues[0].suggested_value, json!(438.0));
    }

    #[test]
    fn ignores_equal_height_table_rows() {
        let root = chart(json!([
            bar("row-a", 100.0, 80.0),
            bar("row-b", 220.0, 80.0),
            bar("row-c", 340.0, 80.0)
        ]));
        assert!(detect_top_anchored_bars(&root).is_empty());
    }

    #[test]
    fn ignores_groups_with_only_two_elements() {
        let root = chart(json!([
            bar("bar-a", 100.0, 80.0),
            bar("bar-b", 220.0, 120.0)
        ]));
        assert!(detect_top_anchored_bars(&root).is_empty());
    }

    #[test]
    fn reports_without_fix_suggestion_when_axis_is_absent() {
        let root = chart(json!([
            bar("bar-a", 100.0, 80.0),
            bar("bar-b", 220.0, 120.0),
            bar("bar-c", 340.0, 160.0)
        ]));
        let issues = detect_top_anchored_bars(&root);
        assert_eq!(issues.len(), 3);
        assert!(issues.iter().all(|issue| issue.suggested_value.is_null()));
    }

    #[test]
    fn detect_top_anchored_bars_catches_misaligned_bars() {
        let root = node(json!({
            "type": "frame", "id": "root", "width": 1920.0, "height": 1080.0,
            "children": [
                {
                    "type": "rectangle", "id": "bar1", "x": 100.0, "y": 500.0,
                    "width": 80.0, "height": 200.0,
                    "fill": [{"type": "solid", "color": "#FF0000"}],
                    "children": []
                },
                {
                    "type": "rectangle", "id": "bar2", "x": 250.0, "y": 500.0,
                    "width": 80.0, "height": 250.0,
                    "fill": [{"type": "solid", "color": "#FF0000"}],
                    "children": []
                },
                {
                    "type": "rectangle", "id": "bar3", "x": 400.0, "y": 500.0,
                    "width": 80.0, "height": 300.0,
                    "fill": [{"type": "solid", "color": "#FF0000"}],
                    "children": []
                }
            ]
        }));
        let issues = detect_top_anchored_bars(&root);
        assert_eq!(issues.len(), 3);
        assert!(issues
            .iter()
            .all(|issue| issue.category == IssueCategory::TopAnchoredBars));
    }

    #[test]
    fn detect_top_anchored_bars_ignores_equal_height_bars() {
        let root = node(json!({
            "type": "frame", "id": "root", "width": 1920.0, "height": 1080.0,
            "children": [
                {
                    "type": "rectangle", "id": "bar1", "x": 100.0, "y": 500.0,
                    "width": 80.0, "height": 200.0,
                    "fill": [{"type": "solid", "color": "#FF0000"}],
                    "children": []
                },
                {
                    "type": "rectangle", "id": "bar2", "x": 250.0, "y": 500.0,
                    "width": 80.0, "height": 200.0,
                    "fill": [{"type": "solid", "color": "#FF0000"}],
                    "children": []
                },
                {
                    "type": "rectangle", "id": "bar3", "x": 400.0, "y": 500.0,
                    "width": 80.0, "height": 200.0,
                    "fill": [{"type": "solid", "color": "#FF0000"}],
                    "children": []
                }
            ]
        }));
        assert!(detect_top_anchored_bars(&root).is_empty());
    }

    #[test]
    fn detect_top_anchored_bars_ignores_groups_under_3() {
        let root = node(json!({
            "type": "frame", "id": "root", "width": 1920.0, "height": 1080.0,
            "children": [
                {
                    "type": "rectangle", "id": "bar1", "x": 100.0, "y": 500.0,
                    "width": 80.0, "height": 200.0,
                    "fill": [{"type": "solid", "color": "#FF0000"}],
                    "children": []
                },
                {
                    "type": "rectangle", "id": "bar2", "x": 250.0, "y": 500.0,
                    "width": 80.0, "height": 250.0,
                    "fill": [{"type": "solid", "color": "#FF0000"}],
                    "children": []
                }
            ]
        }));
        assert!(detect_top_anchored_bars(&root).is_empty());
    }

    #[test]
    fn detects_no_baseline_staggered_bars() {
        // Six-bar fixture from the requirement: descending diagonal baseline,
        // heights proportional but y varies. No chaining → not a waterfall.
        let root = node(json!({
            "type": "frame", "id": "root", "width": 1920.0, "height": 1080.0,
            "children": [
                {
                    "type": "rectangle", "id": "bar1", "x": 120.0, "y": 348.0,
                    "width": 96.0, "height": 112.0,
                    "fill": [{"type": "solid", "color": "#266EA4"}],
                    "children": []
                },
                {
                    "type": "rectangle", "id": "bar2", "x": 312.0, "y": 370.0,
                    "width": 96.0, "height": 134.0,
                    "fill": [{"type": "solid", "color": "#266EA4"}],
                    "children": []
                },
                {
                    "type": "rectangle", "id": "bar3", "x": 504.0, "y": 403.0,
                    "width": 96.0, "height": 175.0,
                    "fill": [{"type": "solid", "color": "#266EA4"}],
                    "children": []
                },
                {
                    "type": "rectangle", "id": "bar4", "x": 696.0, "y": 421.0,
                    "width": 96.0, "height": 211.0,
                    "fill": [{"type": "solid", "color": "#266EA4"}],
                    "children": []
                },
                {
                    "type": "rectangle", "id": "bar5", "x": 888.0, "y": 459.0,
                    "width": 96.0, "height": 278.0,
                    "fill": [{"type": "solid", "color": "#266EA4"}],
                    "children": []
                },
                {
                    "type": "rectangle", "id": "bar6", "x": 1080.0, "y": 475.0,
                    "width": 96.0, "height": 307.0,
                    "fill": [{"type": "solid", "color": "#266EA4"}],
                    "children": []
                }
            ]
        }));
        let issues = detect_top_anchored_bars(&root);
        assert_eq!(issues.len(), 6, "expected one issue per bar");
        assert!(
            issues.iter().all(|issue| {
                issue.category == IssueCategory::NoBaselineBars
                    && issue.property == FixProperty::Y
                    && issue.suggested_value.is_number()
            }),
            "all issues should be NoBaselineBars with numeric y suggestions"
        );
        // Base (max bottom) = 782.0; expected y = 782 - height
        assert_eq!(issues[0].suggested_value, json!(670.0)); // 782 - 112
        assert_eq!(issues[1].suggested_value, json!(648.0)); // 782 - 134
        assert_eq!(issues[2].suggested_value, json!(607.0)); // 782 - 175
        assert_eq!(issues[3].suggested_value, json!(571.0)); // 782 - 211
        assert_eq!(issues[4].suggested_value, json!(504.0)); // 782 - 278
        assert_eq!(issues[5].suggested_value, json!(475.0)); // 782 - 307
    }

    #[test]
    fn ignores_bottoms_aligned_bars() {
        // All bars have the same bottom edge → correct, no issue.
        let root = node(json!({
            "type": "frame", "id": "root", "width": 1920.0, "height": 1080.0,
            "children": [
                {
                    "type": "rectangle", "id": "bar1", "x": 100.0, "y": 300.0,
                    "width": 80.0, "height": 200.0,
                    "fill": [{"type": "solid", "color": "#FF0000"}],
                    "children": []
                },
                {
                    "type": "rectangle", "id": "bar2", "x": 250.0, "y": 250.0,
                    "width": 80.0, "height": 250.0,
                    "fill": [{"type": "solid", "color": "#FF0000"}],
                    "children": []
                },
                {
                    "type": "rectangle", "id": "bar3", "x": 400.0, "y": 200.0,
                    "width": 80.0, "height": 300.0,
                    "fill": [{"type": "solid", "color": "#FF0000"}],
                    "children": []
                }
            ]
        }));
        let issues = detect_top_anchored_bars(&root);
        assert_eq!(
            issues.len(),
            0,
            "bottoms aligned is correct; should emit no issues"
        );
    }

    #[test]
    fn ignores_waterfall_charts() {
        // Waterfall: consecutive bars chain (next top = prev bottom).
        // bar1 bottom=400, bar2 top=400; bar2 bottom=520, bar3 top=520 → 100% chaining.
        let root = node(json!({
            "type": "frame", "id": "root", "width": 1920.0, "height": 1080.0,
            "children": [
                {
                    "type": "rectangle", "id": "bar1", "x": 100.0, "y": 300.0,
                    "width": 80.0, "height": 100.0,
                    "fill": [{"type": "solid", "color": "#FF0000"}],
                    "children": []
                },
                {
                    "type": "rectangle", "id": "bar2", "x": 220.0, "y": 400.0,
                    "width": 80.0, "height": 120.0,
                    "fill": [{"type": "solid", "color": "#FF0000"}],
                    "children": []
                },
                {
                    "type": "rectangle", "id": "bar3", "x": 340.0, "y": 520.0,
                    "width": 80.0, "height": 80.0,
                    "fill": [{"type": "solid", "color": "#FF0000"}],
                    "children": []
                }
            ]
        }));
        let issues = detect_top_anchored_bars(&root);
        assert_eq!(
            issues.len(),
            0,
            "waterfall is intentional; should emit no issues"
        );
    }

    #[test]
    fn two_bar_group_below_threshold() {
        // Only 2 bars (need ≥3 for a group) → no issue.
        let root = node(json!({
            "type": "frame", "id": "root", "width": 1920.0, "height": 1080.0,
            "children": [
                {
                    "type": "rectangle", "id": "bar1", "x": 100.0, "y": 300.0,
                    "width": 80.0, "height": 100.0,
                    "fill": [{"type": "solid", "color": "#FF0000"}],
                    "children": []
                },
                {
                    "type": "rectangle", "id": "bar2", "x": 220.0, "y": 400.0,
                    "width": 80.0, "height": 150.0,
                    "fill": [{"type": "solid", "color": "#FF0000"}],
                    "children": []
                }
            ]
        }));
        let issues = detect_top_anchored_bars(&root);
        assert_eq!(
            issues.len(),
            0,
            "2-bar group below ≥3 threshold; should emit no issues"
        );
    }
}
