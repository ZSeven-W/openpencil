//! High-confidence semantic check for progress-ring containers whose visible
//! ring geometry was omitted by generation.
//!
//! This detector intentionally reports only. A missing visual is an authored
//! design decision, so deterministic code must not guess the ring's geometry,
//! value, palette, or stroke treatment.

use jian_ops_schema::node::{PenNode, TextContent};
use jian_ops_schema::sizing::SizingBehavior;

use crate::node_util::{
    children, corner_radius_numeric, has_stroke, is_node_visible, node_fills, node_id, role,
};

const MIN_RING_SIZE: f64 = 48.0;
const MAX_RING_SIZE: f64 = 480.0;
const MAX_ASPECT_RATIO: f64 = 1.2;
const METRIC_CONTEXT_WORDS: &[&str] = &[
    "activity",
    "calorie",
    "calories",
    "chart",
    "completion",
    "distance",
    "fitness",
    "goal",
    "kcal",
    "kpi",
    "metric",
    "percent",
    "percentage",
    "progress",
    "score",
    "step",
    "steps",
    "workout",
];

/// A metric container that strongly signals a progress ring but contains no
/// visible circle, arc, or painted circular frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingProgressRing {
    pub node_id: String,
    pub node_name: String,
}

/// Find generated progress-ring wrappers that contain a numeric metric but no
/// ring visual. The heuristic is deliberately conservative:
///
/// - the wrapper must have explicit, near-square numeric dimensions;
/// - its name/role must explicitly signal ring/donut/gauge/radial geometry;
/// - its subtree must explicitly identify a progress/activity metric;
/// - its subtree must contain a numeric text metric; and
/// - its own subtree must not contain visible ring geometry.
///
/// Only the deepest missing wrapper is returned when semantic wrappers nest.
pub fn detect_missing_progress_rings(nodes: &[PenNode]) -> Vec<MissingProgressRing> {
    let mut issues = Vec::new();
    for node in nodes {
        walk(node, &mut issues);
    }
    issues
}

fn walk(node: &PenNode, issues: &mut Vec<MissingProgressRing>) {
    let subtree_has_ring_visual = near_square_numeric_size(node)
        .is_some_and(|(width, height)| has_ring_visual(node, width.min(height)));
    let descendant_is_ring_metric = children(node).iter().any(subtree_has_ring_metric_candidate);
    let issues_before_children = issues.len();

    for child in children(node) {
        walk(child, issues);
    }

    let descendant_already_reported = issues.len() > issues_before_children;
    if !descendant_already_reported
        && !descendant_is_ring_metric
        && is_ring_metric_candidate(node)
        && !subtree_has_ring_visual
    {
        issues.push(MissingProgressRing {
            node_id: node_id(node).to_string(),
            node_name: node_name(node)
                .unwrap_or("unnamed progress ring")
                .to_string(),
        });
    }
}

fn subtree_has_ring_metric_candidate(node: &PenNode) -> bool {
    is_ring_metric_candidate(node) || children(node).iter().any(subtree_has_ring_metric_candidate)
}

fn is_ring_metric_candidate(node: &PenNode) -> bool {
    near_square_numeric_size(node).is_some()
        && contains_ring_visual_word(&semantic_label(node))
        && subtree_contains_metric_context(node)
        && subtree_contains_numeric_text(node)
}

fn contains_ring_visual_word(label: &str) -> bool {
    contains_semantic_word(label, &["ring", "donut", "doughnut", "gauge", "radial"])
}

fn contains_semantic_word(label: &str, words: &[&str]) -> bool {
    label
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .any(|token| words.contains(&token))
}

fn semantic_label(node: &PenNode) -> String {
    [node_name(node), role(node)]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn subtree_contains_numeric_text(node: &PenNode) -> bool {
    let own_text_has_number = match node {
        PenNode::Text(text) => match &text.content {
            TextContent::Plain(content) => content.chars().any(|c| c.is_ascii_digit()),
            TextContent::Styled(segments) => segments
                .iter()
                .any(|segment| segment.text.chars().any(|c| c.is_ascii_digit())),
        },
        _ => false,
    };
    own_text_has_number || children(node).iter().any(subtree_contains_numeric_text)
}

fn subtree_contains_metric_context(node: &PenNode) -> bool {
    let own_label_matches = contains_semantic_word(&semantic_label(node), METRIC_CONTEXT_WORDS);
    let own_text_matches = match node {
        PenNode::Text(text) => match &text.content {
            TextContent::Plain(content) => {
                contains_semantic_word(&content.to_ascii_lowercase(), METRIC_CONTEXT_WORDS)
            }
            TextContent::Styled(segments) => segments.iter().any(|segment| {
                contains_semantic_word(&segment.text.to_ascii_lowercase(), METRIC_CONTEXT_WORDS)
            }),
        },
        _ => false,
    };
    own_label_matches
        || own_text_matches
        || children(node).iter().any(subtree_contains_metric_context)
}

fn has_ring_visual(node: &PenNode, candidate_min_size: f64) -> bool {
    has_filled_donut_pair(node, candidate_min_size)
        || is_visible_ring_visual(node, candidate_min_size)
        || children(node)
            .iter()
            .any(|child| has_ring_visual(child, candidate_min_size))
}

/// Some generators construct a donut from an outer filled disc plus a smaller
/// background-coloured disc instead of using an ellipse inner radius. Treat
/// that pair as authored ring geometry while keeping a lone avatar/status dot
/// insufficient.
fn has_filled_donut_pair(node: &PenNode, candidate_min_size: f64) -> bool {
    let diameters: Vec<f64> = children(node)
        .iter()
        .filter_map(|child| filled_disc_diameter(child, candidate_min_size))
        .collect();
    diameters.iter().any(|outer| {
        *outer >= candidate_min_size * 0.9
            && diameters
                .iter()
                .any(|inner| *inner >= *outer * 0.45 && *inner <= *outer * 0.85)
    })
}

fn filled_disc_diameter(node: &PenNode, candidate_min_size: f64) -> Option<f64> {
    let PenNode::Ellipse(ellipse) = node else {
        return None;
    };
    if !is_node_visible(node)
        || !node_fills(node)
            .map(|fills| !fills.is_empty())
            .unwrap_or(false)
        || has_stroke(node)
        || ellipse.inner_radius.unwrap_or(0.0) != 0.0
        || ellipse.sweep_angle.is_some()
    {
        return None;
    }
    let (width, height) = explicit_numeric_size(node)?;
    let min = width.min(height);
    let max = width.max(height);
    if min >= candidate_min_size * 0.4 && max / min <= MAX_ASPECT_RATIO {
        Some(min)
    } else {
        None
    }
}

fn is_visible_ring_visual(node: &PenNode, candidate_min_size: f64) -> bool {
    if !is_node_visible(node) || !has_visible_paint(node) {
        return false;
    }

    let Some((width, height)) = explicit_numeric_size(node) else {
        return false;
    };
    let min = width.min(height);
    let max = width.max(height);
    if min < candidate_min_size * 0.6 || max / min > MAX_ASPECT_RATIO {
        return false;
    }

    match node {
        PenNode::Ellipse(ellipse) => {
            has_stroke(node)
                || ellipse.inner_radius.unwrap_or(0.0) > 0.0
                || ellipse.sweep_angle.is_some_and(|angle| angle.abs() < 359.5)
        }
        PenNode::Path(_) => {
            let label = semantic_label(node);
            contains_ring_visual_word(&label)
                || contains_semantic_word(&label, &["arc", "track", "progress"])
        }
        PenNode::Frame(_) | PenNode::Group(_) | PenNode::Rectangle(_) => {
            has_stroke(node) && corner_radius_numeric(node) >= width.min(height) * 0.4
        }
        _ => false,
    }
}

fn has_visible_paint(node: &PenNode) -> bool {
    node_fills(node)
        .map(|fills| !fills.is_empty())
        .unwrap_or(false)
        || has_stroke(node)
}

fn near_square_numeric_size(node: &PenNode) -> Option<(f64, f64)> {
    let (width, height) = explicit_numeric_size(node)?;
    let min = width.min(height);
    let max = width.max(height);
    if min < MIN_RING_SIZE || max > MAX_RING_SIZE || max / min > MAX_ASPECT_RATIO {
        return None;
    }
    Some((width, height))
}

fn explicit_numeric_size(node: &PenNode) -> Option<(f64, f64)> {
    let (width, height) = match node {
        PenNode::Frame(frame) => (&frame.container.width, &frame.container.height),
        PenNode::Group(group) => (&group.container.width, &group.container.height),
        PenNode::Rectangle(rectangle) => (&rectangle.container.width, &rectangle.container.height),
        PenNode::Ellipse(ellipse) => (&ellipse.width, &ellipse.height),
        PenNode::Path(path) => (&path.width, &path.height),
        _ => return None,
    };
    match (width, height) {
        (Some(SizingBehavior::Number(width)), Some(SizingBehavior::Number(height))) => {
            Some((*width, *height))
        }
        _ => None,
    }
}

fn node_name(node: &PenNode) -> Option<&str> {
    match node {
        PenNode::Frame(n) => n.base.name.as_deref(),
        PenNode::Group(n) => n.base.name.as_deref(),
        PenNode::Rectangle(n) => n.base.name.as_deref(),
        PenNode::Ellipse(n) => n.base.name.as_deref(),
        PenNode::Line(n) => n.base.name.as_deref(),
        PenNode::Polygon(n) => n.base.name.as_deref(),
        PenNode::Path(n) => n.base.name.as_deref(),
        PenNode::Text(n) => n.base.name.as_deref(),
        PenNode::TextInput(n) => n.base.name.as_deref(),
        PenNode::Image(n) => n.base.name.as_deref(),
        PenNode::IconFont(n) => n.base.name.as_deref(),
        PenNode::TextArea(n) => n.base.name.as_deref(),
        PenNode::Select(n) => n.base.name.as_deref(),
        PenNode::Switch(n) => n.base.name.as_deref(),
        PenNode::Checkbox(n) => n.base.name.as_deref(),
        PenNode::Slider(n) => n.base.name.as_deref(),
        PenNode::RadioGroup(n) => n.base.name.as_deref(),
        PenNode::NumberInput(n) => n.base.name.as_deref(),
        PenNode::Progress(n) => n.base.name.as_deref(),
        PenNode::Tabs(n) => n.base.name.as_deref(),
        PenNode::Ref(n) => n.base.name.as_deref(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn nodes(value: serde_json::Value) -> Vec<PenNode> {
        serde_json::from_value(value).expect("fixture must deserialize as PenNode list")
    }

    #[test]
    fn detects_0714_numeric_wrapper_without_ring_geometry() {
        let fixture = nodes(json!([{
            "type": "frame", "id": "card", "name": "Today Activity Ring",
            "width": 342, "height": 376, "children": [{
                "type": "frame", "id": "steps-ring", "name": "Steps Ring",
                "width": 124, "height": 124, "layout": "vertical",
                "alignItems": "center", "justifyContent": "center", "children": [
                    {"type": "text", "id": "value", "content": "8,432"},
                    {"type": "text", "id": "label", "content": "steps"}
                ]
            }]
        }]));

        assert_eq!(
            detect_missing_progress_rings(&fixture),
            vec![MissingProgressRing {
                node_id: "steps-ring".into(),
                node_name: "Steps Ring".into(),
            }]
        );
    }

    #[test]
    fn accepts_ring_with_visible_ellipse_geometry() {
        let fixture = nodes(json!([{
            "type": "frame", "id": "steps-ring", "name": "Steps Ring",
            "width": 124, "height": 124, "layout": "none", "children": [
                {"type": "ellipse", "id": "track", "name": "Ring Track",
                 "width": 124, "height": 124,
                 "stroke": {"fill": [{"type": "solid", "color": "#22332A"}], "thickness": 10}},
                {"type": "ellipse", "id": "arc", "name": "Progress Arc",
                 "width": 124, "height": 124, "innerRadius": 0.82, "sweepAngle": 270,
                 "fill": [{"type": "solid", "color": "#00D46A"}]},
                {"type": "text", "id": "value", "content": "8,432"}
            ]
        }]));

        assert!(detect_missing_progress_rings(&fixture).is_empty());
    }

    #[test]
    fn valid_child_ring_suppresses_larger_semantic_wrapper() {
        let fixture = nodes(json!([{
            "type": "frame", "id": "card", "name": "Today Activity Ring",
            "width": 342, "height": 376, "children": [{
                "type": "frame", "id": "steps-ring", "name": "Steps Ring",
                "width": 124, "height": 124, "layout": "none", "children": [
                    {"type": "ellipse", "id": "track", "name": "Ring Track",
                     "width": 124, "height": 124,
                     "stroke": {"fill": [{"type": "solid", "color": "#22332A"}], "thickness": 10}},
                    {"type": "text", "id": "value", "content": "8,432"}
                ]
            }]
        }]));

        assert!(detect_missing_progress_rings(&fixture).is_empty());
    }

    #[test]
    fn accepts_donut_built_from_two_filled_discs() {
        let fixture = nodes(json!([{
            "type": "frame", "id": "steps-ring", "name": "Steps Ring",
            "width": 124, "height": 124, "layout": "none", "children": [
                {"type": "ellipse", "id": "outer", "name": "Outer Disc",
                 "width": 124, "height": 124,
                 "fill": [{"type": "solid", "color": "#00D46A"}]},
                {"type": "ellipse", "id": "inner", "name": "Inner Disc",
                 "width": 82, "height": 82,
                 "fill": [{"type": "solid", "color": "#101412"}]},
                {"type": "text", "id": "value", "content": "8,432"}
            ]
        }]));

        assert!(detect_missing_progress_rings(&fixture).is_empty());
    }

    #[test]
    fn ignores_ordinary_numeric_activity_card() {
        let fixture = nodes(json!([{
            "type": "frame", "id": "summary", "name": "Activity Progress Card",
            "role": "card", "width": 124, "height": 124,
            "layout": "vertical", "alignItems": "start", "justifyContent": "start",
            "cornerRadius": 16,
            "fill": [{"type": "solid", "color": "#181A19"}],
            "children": [
                {"type": "text", "id": "value", "content": "8,432"},
                {"type": "text", "id": "label", "content": "steps"}
            ]
        }]));

        assert!(detect_missing_progress_rings(&fixture).is_empty());
    }

    #[test]
    fn generic_centered_progress_card_does_not_imply_a_ring() {
        let fixture = nodes(json!([{
            "type": "frame", "id": "summary", "name": "Activity Progress",
            "role": "progressbar", "width": 124, "height": 124,
            "layout": "vertical", "alignItems": "center", "justifyContent": "center",
            "children": [
                {"type": "text", "id": "value", "content": "72%"}
            ]
        }]));

        assert!(detect_missing_progress_rings(&fixture).is_empty());
    }

    #[test]
    fn word_boundaries_do_not_turn_hiring_or_spring_cards_into_rings() {
        let fixture = nodes(json!([
            {
                "type": "frame", "id": "hiring", "name": "Hiring KPI",
                "width": 124, "height": 124, "layout": "vertical",
                "alignItems": "center", "justifyContent": "center", "children": [
                    {"type": "text", "id": "hiring-value", "content": "12 hires"}
                ]
            },
            {
                "type": "frame", "id": "spring", "name": "Spring Sale",
                "width": 124, "height": 124, "layout": "vertical",
                "alignItems": "center", "justifyContent": "center", "children": [
                    {"type": "text", "id": "spring-value", "content": "50%"}
                ]
            }
        ]));

        assert!(detect_missing_progress_rings(&fixture).is_empty());
    }

    #[test]
    fn product_ring_and_donut_names_do_not_imply_progress_metrics() {
        let fixture = nodes(json!([
            {
                "type": "frame", "id": "jewelry", "name": "Diamond Ring",
                "width": 124, "height": 124, "children": [
                    {"type": "text", "id": "price", "content": "$1,299"}
                ]
            },
            {
                "type": "frame", "id": "size", "name": "Ring Size",
                "width": 124, "height": 124, "children": [
                    {"type": "text", "id": "size-value", "content": "7"}
                ]
            },
            {
                "type": "frame", "id": "pastry", "name": "Donut Deal",
                "width": 124, "height": 124, "children": [
                    {"type": "text", "id": "pastry-price", "content": "$4"}
                ]
            }
        ]));

        assert!(detect_missing_progress_rings(&fixture).is_empty());
    }

    #[test]
    fn unrelated_small_ellipse_does_not_hide_a_missing_ring() {
        let fixture = nodes(json!([{
            "type": "frame", "id": "steps-ring", "name": "Steps Ring",
            "width": 124, "height": 124, "layout": "vertical",
            "alignItems": "center", "justifyContent": "center", "children": [
                {"type": "ellipse", "id": "avatar", "name": "Avatar",
                 "width": 40, "height": 40,
                 "fill": [{"type": "solid", "color": "#FFAA00"}]},
                {"type": "text", "id": "value", "content": "8,432"}
            ]
        }]));

        assert_eq!(
            detect_missing_progress_rings(&fixture),
            vec![MissingProgressRing {
                node_id: "steps-ring".into(),
                node_name: "Steps Ring".into(),
            }]
        );
    }

    #[test]
    fn valid_ring_sibling_does_not_hide_independent_missing_ring() {
        let fixture = nodes(json!([{
            "type": "frame", "id": "activity-card", "name": "Activity Ring Group",
            "width": 300, "height": 160, "children": [
                {
                    "type": "frame", "id": "calorie-ring", "name": "Calories Ring",
                    "width": 124, "height": 124, "children": [
                        {"type": "ellipse", "id": "calorie-track", "width": 124, "height": 124,
                         "stroke": {"thickness": 10}},
                        {"type": "text", "id": "calorie-value", "content": "540"}
                    ]
                },
                {
                    "type": "frame", "id": "steps-ring", "name": "Steps Ring",
                    "width": 124, "height": 124, "layout": "vertical",
                    "alignItems": "center", "justifyContent": "center", "children": [
                        {"type": "text", "id": "steps-value", "content": "8,432"},
                        {"type": "text", "id": "steps-label", "content": "steps"}
                    ]
                }
            ]
        }]));

        assert_eq!(
            detect_missing_progress_rings(&fixture),
            vec![MissingProgressRing {
                node_id: "steps-ring".into(),
                node_name: "Steps Ring".into(),
            }]
        );
    }
}
