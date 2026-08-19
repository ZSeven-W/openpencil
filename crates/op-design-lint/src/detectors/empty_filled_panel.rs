//! Detect large, solid-filled shape containers that contain no content.

use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::PenFill;
use serde_json::{json, Value};

use crate::color::color_contrast;
use crate::issue::{FixProperty, Issue, IssueCategory, IssueSeverity};
use crate::node_util::{
    children, node_fills, node_id, node_kind, node_x, node_y, numeric_height, numeric_width,
    opacity, NodeKind,
};

const MIN_SIDE: f64 = 120.0;
const MIN_ROOT_AREA_FRACTION: f64 = 0.04;
const SAME_GEOMETRY_EPSILON: f64 = 1.0;
const INVISIBLE_CONTRAST: f64 = 1.1;

/// Find large, visibly filled, childless shape containers inside `root`.
///
/// This is an intent-tier diagnostic: a large color block can be deliberate,
/// so the emitted `Fill` property is intentionally a no-op in the fix path.
pub fn detect_empty_filled_panel(root: &PenNode) -> Vec<Issue> {
    let Some(root_width) = numeric_width(root) else {
        return Vec::new();
    };
    let Some(root_height) = numeric_height(root) else {
        return Vec::new();
    };
    let root_area = root_width * root_height;
    if !root_area.is_finite() || root_area <= 0.0 {
        return Vec::new();
    }

    let mut issues = Vec::new();
    walk_empty_panels(root, root, root_area, &mut issues);
    issues
}

fn walk_empty_panels(
    node: &PenNode,
    artboard: &PenNode,
    artboard_area: f64,
    issues: &mut Vec<Issue>,
) {
    let siblings = children(node);
    for child in siblings {
        if is_empty_filled_panel(child, node, siblings, artboard, artboard_area) {
            issues.push(panel_issue(child));
        }
        walk_empty_panels(child, artboard, artboard_area, issues);
    }
}

fn is_empty_filled_panel(
    node: &PenNode,
    parent: &PenNode,
    siblings: &[PenNode],
    artboard: &PenNode,
    artboard_area: f64,
) -> bool {
    if !matches!(
        node_kind(node),
        NodeKind::Rectangle | NodeKind::Ellipse | NodeKind::Frame
    ) {
        return false;
    }
    if !children(node).is_empty() || visible_solid_color(node).is_none() {
        return false;
    }
    if !is_explicitly_visible(node) {
        return false;
    }
    let (Some(width), Some(height)) = (numeric_width(node), numeric_height(node)) else {
        return false;
    };
    if width.min(height) < MIN_SIDE || width * height < artboard_area * MIN_ROOT_AREA_FRACTION {
        return false;
    }
    if is_artboard_background(node, parent, artboard, width, height) {
        return false;
    }
    if is_filled_shape_series_member(node, siblings) {
        return false;
    }
    if let (Some(panel_color), Some(parent_color)) =
        (visible_solid_color(node), visible_solid_color(parent))
    {
        if color_contrast(panel_color, parent_color) <= INVISIBLE_CONTRAST {
            return false;
        }
    }
    true
}

fn panel_issue(node: &PenNode) -> Issue {
    let color = visible_solid_color(node).unwrap_or("");
    let width = numeric_width(node).unwrap_or_default();
    let height = numeric_height(node).unwrap_or_default();
    Issue {
        node_id: node_id(node).to_string(),
        category: IssueCategory::EmptyFilledPanel,
        severity: IssueSeverity::Warning,
        // Intent-tier: Fill is deliberately a no-op in both the direct and
        // planned fix paths. The model receives the finding, not a rewrite.
        property: FixProperty::Fill,
        current_value: json!(color),
        suggested_value: Value::Null,
        reason: format!(
            "large filled {kind} has no children ({width:.0}x{height:.0}); review whether it is an empty panel",
            kind = crate::node_util::node_kind_str(node),
        ),
    }
}

fn is_filled_shape_series_member(node: &PenNode, siblings: &[PenNode]) -> bool {
    if node_kind(node) != NodeKind::Rectangle {
        return false;
    }
    let Some(width) = numeric_width(node) else {
        return false;
    };
    let Some(height) = numeric_height(node) else {
        return false;
    };
    let matching_brothers = siblings
        .iter()
        .filter(|sibling| node_id(sibling) != node_id(node))
        .filter(|sibling| node_kind(sibling) == NodeKind::Rectangle)
        .filter(|sibling| children(sibling).is_empty())
        .filter(|sibling| visible_solid_color(sibling).is_some())
        .filter(|sibling| {
            let Some(sibling_width) = numeric_width(sibling) else {
                return false;
            };
            let Some(sibling_height) = numeric_height(sibling) else {
                return false;
            };
            (sibling_width - width).abs() <= SAME_GEOMETRY_EPSILON
                || (sibling_height - height).abs() <= SAME_GEOMETRY_EPSILON
        })
        .count();
    matching_brothers >= 2
}

fn is_artboard_background(
    node: &PenNode,
    parent: &PenNode,
    artboard: &PenNode,
    width: f64,
    height: f64,
) -> bool {
    if node_id(parent) != node_id(artboard) {
        return false;
    }
    let artboard_width = numeric_width(artboard).unwrap_or_default();
    let artboard_height = numeric_height(artboard).unwrap_or_default();
    let x = node_x(node).unwrap_or(0.0);
    let y = node_y(node).unwrap_or(0.0);
    x.abs() <= SAME_GEOMETRY_EPSILON
        && y.abs() <= SAME_GEOMETRY_EPSILON
        && (width - artboard_width).abs() <= SAME_GEOMETRY_EPSILON
        && (height - artboard_height).abs() <= SAME_GEOMETRY_EPSILON
}

fn is_explicitly_visible(node: &PenNode) -> bool {
    opacity(node) != 0.0
}

fn visible_solid_color(node: &PenNode) -> Option<&str> {
    node_fills(node)?.iter().find_map(|fill| match fill {
        PenFill::Solid(body)
            if body.opacity.unwrap_or(1.0) > 0.0 && !is_transparent_color(&body.color) =>
        {
            Some(body.color.as_str())
        }
        _ => None,
    })
}

fn is_transparent_color(color: &str) -> bool {
    let color = color.trim();
    if color.eq_ignore_ascii_case("transparent") {
        return true;
    }
    if color.len() == 9 && color.is_ascii() && color.starts_with('#') {
        return u8::from_str_radix(&color[7..9], 16).is_ok_and(|alpha| alpha == 0);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn node(value: serde_json::Value) -> PenNode {
        serde_json::from_value(value).expect("fixture must deserialize as PenNode")
    }

    fn root(children: serde_json::Value) -> PenNode {
        node(json!({
            "type": "frame", "id": "board", "width": 1000, "height": 1000,
            "fill": [{"type": "solid", "color": "#F7F4EE"}],
            "children": children
        }))
    }

    #[test]
    fn flags_large_empty_filled_panel() {
        let board = root(json!([
            {"type":"text", "id":"title", "content":"Report"},
            {"type":"rectangle", "id":"panel", "x":500, "y":100, "width":300, "height":500,
             "fill":[{"type":"solid","color":"#EBE7DF"}]}
        ]));
        let issues = detect_empty_filled_panel(&board);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "panel");
        assert_eq!(issues[0].category, IssueCategory::EmptyFilledPanel);
        assert_eq!(issues[0].property, FixProperty::Fill);
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
        assert!(issues[0].suggested_value.is_null());
    }

    #[test]
    fn ignores_divider_and_filled_bar_series() {
        let board = root(json!([
            {"type":"rectangle", "id":"divider", "x":100, "y":100, "width":300, "height":3,
             "fill":[{"type":"solid","color":"#266EA4"}]},
            {"type":"rectangle", "id":"bar-a", "x":100, "y":200, "width":150, "height":300,
             "fill":[{"type":"solid","color":"#266EA4"}]},
            {"type":"rectangle", "id":"bar-b", "x":300, "y":200, "width":150, "height":350,
             "fill":[{"type":"solid","color":"#266EA4"}]},
            {"type":"rectangle", "id":"bar-c", "x":500, "y":200, "width":150, "height":400,
             "fill":[{"type":"solid","color":"#266EA4"}]}
        ]));
        assert!(detect_empty_filled_panel(&board).is_empty());
    }

    #[test]
    fn ignores_transparent_and_same_color_background_shapes() {
        let board = root(json!([
            {"type":"rectangle", "id":"transparent", "x":100, "y":100, "width":300, "height":300,
             "opacity":0, "fill":[{"type":"solid","color":"#000000"}]},
            {"type":"rectangle", "id":"same-tone", "x":500, "y":100, "width":300, "height":300,
             "fill":[{"type":"solid","color":"#F7F4EE"}]}
        ]));
        assert!(detect_empty_filled_panel(&board).is_empty());
    }

    #[test]
    fn detect_empty_filled_panel_catches_large_empty_rectangle() {
        let board = node(json!({
            "type": "frame", "id": "root", "width": 1920.0, "height": 1080.0,
            "fill": [{"type": "solid", "color": "#FFFFFF"}],
            "children": [
                {
                    "type": "rectangle", "id": "empty_rect", "x": 100.0, "y": 100.0,
                    "width": 500.0, "height": 300.0,
                    "fill": [{"type": "solid", "color": "#CCCCCC"}],
                    "children": []
                }
            ]
        }));
        let issues = detect_empty_filled_panel(&board);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "empty_rect");
        assert_eq!(issues[0].category, IssueCategory::EmptyFilledPanel);
    }

    #[test]
    fn detect_empty_filled_panel_ignores_small_rectangles() {
        let board = node(json!({
            "type": "frame", "id": "root", "width": 1920.0, "height": 1080.0,
            "fill": [{"type": "solid", "color": "#FFFFFF"}],
            "children": [
                {
                    "type": "rectangle", "id": "small_rect", "x": 100.0, "y": 100.0,
                    "width": 50.0, "height": 50.0,
                    "fill": [{"type": "solid", "color": "#CCCCCC"}],
                    "children": []
                }
            ]
        }));
        assert!(detect_empty_filled_panel(&board).is_empty());
    }

    #[test]
    fn detect_empty_filled_panel_ignores_rectangles_with_children() {
        let board = node(json!({
            "type": "frame", "id": "root", "width": 1920.0, "height": 1080.0,
            "fill": [{"type": "solid", "color": "#FFFFFF"}],
            "children": [
                {
                    "type": "rectangle", "id": "filled_container", "x": 100.0, "y": 100.0,
                    "width": 500.0, "height": 300.0,
                    "fill": [{"type": "solid", "color": "#CCCCCC"}],
                    "children": [
                        {"type": "text", "id": "t1", "content": "Hello"}
                    ]
                }
            ]
        }));
        assert!(detect_empty_filled_panel(&board).is_empty());
    }
}
