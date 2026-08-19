//! Detect sibling bar charts whose varying heights are anchored at the top.

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

/// Find top-anchored bar groups below an existing horizontal axis.
///
/// A group with axis evidence is contract-tier and receives a numeric `y`
/// suggestion. Without that evidence the issue remains report-only: its
/// suggestion is null, so neither the direct nor planned fix path mutates it.
pub fn detect_top_anchored_bars(root: &PenNode) -> Vec<Issue> {
    let mut issues = Vec::new();
    walk_bar_parents(root, &mut issues);
    issues
}

fn walk_bar_parents(node: &PenNode, issues: &mut Vec<Issue>) {
    for group in find_bar_groups(children(node)) {
        let has_axis = has_axis_evidence(node, &group);
        let base = group
            .iter()
            .map(|bar| bar.y + bar.height)
            .fold(f64::NEG_INFINITY, f64::max);
        for bar in group {
            issues.push(bar_issue(bar, base, has_axis));
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
            if !same_bar_dimensions(&group, candidate) {
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

fn same_bar_dimensions(group: &[Bar<'_>], candidate: Bar<'_>) -> bool {
    group.iter().all(|bar| {
        (bar.width - candidate.width).abs() <= WIDTH_EPSILON
            && (bar.y - candidate.y).abs() <= Y_EPSILON
    })
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

fn bar_issue(bar: Bar<'_>, base: f64, has_axis: bool) -> Issue {
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
}
