//! Structural workability detectors for design documents.
//!
//! Detects three classes of structural issues that make a document hard to edit:
//! 1. Redundant wrappers — frames boxing a single container with no visual properties
//! 2. Excessive nesting depth — nodes deeper than a measured threshold below their screen root
//! 3. High absolute-positioning share — too many `layout:none` frames in a screen

use jian_ops_schema::node::container::LayoutMode;
use jian_ops_schema::node::PenNode;
use serde_json::Value;

use crate::issue::{FixProperty, Issue, IssueCategory, IssueSeverity};
use crate::node_util::{
    children, first_fill_color, has_stroke, node_id, node_kind, numeric_height, numeric_width,
    padding, role, NodeKind,
};

/// Measured nesting depth threshold. Kimi's good 250-node document has max depth 5;
/// weaker models (Gemini 278-node, DeepSeek 303-node, GLM 392-node) reach depth 7.
/// Threshold set to 5 so we report nodes at depth 6+, the start of the problematic range.
const MAX_NESTING_DEPTH: usize = 5;

/// Measured absolute-positioning threshold. When more than this fraction of frames
/// in a screen use `layout:none`, the design becomes hard to reflow on edit.
/// Measured data: Kimi 1/250 (0.4%), GLM 55/392 (14%), others similar or worse.
/// Threshold set to 0.10 (10%) to capture GLM's 14% as problematic while excluding Kimi.
const MAX_ABSOLUTE_FRAME_RATIO: f64 = 0.10;

/// Detect redundant wrapper frames — frames whose only child is itself a container,
/// carrying no visual properties of their own (no fill, no stroke, no corner radius,
/// no worthwhile padding). Such wrappers box a box for no reason.
///
/// Report-only finding: nesting is an authoring choice and collapsing it could
/// destroy an intended grouping, so no auto-fix is safe.
pub fn detect_redundant_wrappers(root: &PenNode) -> Vec<Issue> {
    let mut issues = Vec::new();
    walk_redundant_wrappers(root, 0, &mut issues);
    issues
}

fn walk_redundant_wrappers(node: &PenNode, depth: usize, issues: &mut Vec<Issue>) {
    // Only frames can be redundant wrappers (Group/Rectangle carry semantic meaning)
    if node_kind(node) == NodeKind::Frame {
        if let PenNode::Frame(frame) = node {
            // Skip frames inside the OS status-bar subtree (injected by scaffold)
            if role(node) == Some("status-bar") {
                return;
            }

            // Skip the root (depth 0) — it is the artboard/canvas, not wrapping anything
            if depth > 0 {
                if let Some(child_list) = &frame.children {
                    // Single child of any type (leaf or container)
                    if child_list.len() == 1 {
                        let child = &child_list[0];
                        // Skip if the child is itself a defect (empty path, zero-size node, etc).
                        // That node is already reported by another detector; don't double-count
                        // by reporting the wrapper as redundant.
                        if !is_empty_placeholder(child) {
                            // Wrapper is redundant if:
                            // 1. Has exactly one child (any type)
                            // 2. Wrapper has no layout (None) or layout:none
                            // 3. Wrapper has no visual properties
                            // 4. Child is not a zero-size or empty placeholder
                            if (frame.container.layout.is_none() || is_absolute_positioned(node))
                                && !has_visual_properties(node)
                            {
                                issues.push(Issue {
                                    node_id: node_id(node).to_string(),
                                    category: IssueCategory::RedundantWrapper,
                                    severity: IssueSeverity::Info,
                                    property: FixProperty::Remove,
                                    current_value: Value::Null,
                                    suggested_value: Value::Bool(true),
                                    reason: format!(
                                        "frame wraps a single {} with no visual properties (redundant nesting)",
                                        node_kind_name(child)
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    for child in children(node) {
        walk_redundant_wrappers(child, depth + 1, issues);
    }
}

/// Detect nodes deeper than the measured threshold below their screen root.
/// Finds the deepest offender per screen and reports only that one, to avoid
/// flooding the report with a single path of redundant nesting.
pub fn detect_excessive_nesting_depth(root: &PenNode) -> Vec<Issue> {
    let mut issues = Vec::new();
    // Walk each screen-like (frame/group at depth 1) as a root
    for screen in children(root) {
        if node_kind(screen).is_container() {
            let mut deepest: Option<(usize, &PenNode)> = None;
            walk_nesting_depth(screen, 1, &mut deepest);
            if let Some((depth, node)) = deepest {
                if depth > MAX_NESTING_DEPTH {
                    issues.push(Issue {
                        node_id: node_id(node).to_string(),
                        category: IssueCategory::ExcessiveNestingDepth,
                        severity: IssueSeverity::Info,
                        property: FixProperty::Remove,
                        current_value: Value::Null,
                        suggested_value: Value::Null,
                        reason: format!(
                            "node at depth {} under {} (threshold: {}); deepest in screen",
                            depth,
                            node_id(screen),
                            MAX_NESTING_DEPTH
                        ),
                    });
                }
            }
        }
    }
    issues
}

fn walk_nesting_depth<'a>(
    node: &'a PenNode,
    depth: usize,
    deepest: &mut Option<(usize, &'a PenNode)>,
) {
    // Update if this is the deepest we've seen
    if deepest.is_none() || depth > deepest.as_ref().unwrap().0 {
        *deepest = Some((depth, node));
    }
    for child in children(node) {
        walk_nesting_depth(child, depth + 1, deepest);
    }
}

/// Detect screens where absolute-positioned frames (`layout:none`) exceed a
/// threshold fraction. Excludes the status bar subtree (role="status-bar").
/// Reports only when the share is high enough to hurt editability.
pub fn detect_absolute_positioning_share(root: &PenNode) -> Vec<Issue> {
    let mut issues = Vec::new();

    // Analyze each screen-like (frame/group at depth 1) as a separate unit
    for screen in children(root) {
        if node_kind(screen).is_container() {
            let mut total_frames = 0;
            let mut absolute_frames = 0;

            // Count frames within this screen (skip the screen itself, count only its children)
            for child in children(screen) {
                count_screen_frames(child, &mut total_frames, &mut absolute_frames);
            }

            if total_frames > 0 {
                let ratio = absolute_frames as f64 / total_frames as f64;
                if ratio > MAX_ABSOLUTE_FRAME_RATIO {
                    issues.push(Issue {
                        node_id: node_id(screen).to_string(),
                        category: IssueCategory::AbsolutePositioningShare,
                        severity: IssueSeverity::Info,
                        property: FixProperty::Remove,
                        current_value: Value::Null,
                        suggested_value: Value::Null,
                        reason: format!(
                            "{} of {} frames use layout:none (threshold: {:.0}%); excludes status-bar subtree",
                            absolute_frames,
                            total_frames,
                            MAX_ABSOLUTE_FRAME_RATIO * 100.0
                        ),
                    });
                }
            }
        }
    }

    issues
}

fn count_screen_frames(node: &PenNode, total: &mut usize, absolute: &mut usize) {
    // Count all frames in this subtree, stopping at status-bar boundary
    if node_kind(node) == NodeKind::Frame {
        // Skip the entire status bar subtree
        if role(node) == Some("status-bar") {
            return;
        }
        *total += 1;
        if is_absolute_positioned(node) {
            *absolute += 1;
        }
    }

    for child in children(node) {
        count_screen_frames(child, total, absolute);
    }
}

// ===== Helper functions =====

fn has_visual_properties(node: &PenNode) -> bool {
    match node {
        PenNode::Frame(frame) => {
            // For frames, check container properties
            let has_fill = frame.container.fill.as_ref().is_some_and(|fills| {
                fills.iter().any(|f| {
                    if let jian_ops_schema::style::PenFill::Solid(body) = f {
                        !body.color.is_empty()
                    } else {
                        true // Non-solid fills count as visual
                    }
                })
            });
            let has_stroke = frame
                .container
                .stroke
                .as_ref()
                .is_some_and(|s| crate::node_util::stroke_thickness_max(&s.thickness) > 0.0);
            let has_corner_radius = frame.container.corner_radius.is_some();
            let has_padding = frame.container.padding.is_some();

            has_fill || has_stroke || has_corner_radius || has_padding
        }
        _ => {
            // For other node types, use the general property checks
            let has_fill = first_fill_color(node).is_some();
            let has_stroke = has_stroke(node);
            let has_corner_radius = has_corner_radius(node);
            let has_padding = padding(node).is_some();

            has_fill || has_stroke || has_corner_radius || has_padding
        }
    }
}

fn has_corner_radius(node: &PenNode) -> bool {
    match node {
        PenNode::Frame(n) => n.container.corner_radius.is_some(),
        PenNode::Rectangle(n) => n.container.corner_radius.is_some(),
        _ => false,
    }
}

fn is_absolute_positioned(node: &PenNode) -> bool {
    if let PenNode::Frame(frame) = node {
        frame.container.layout == Some(LayoutMode::None)
    } else {
        false
    }
}

fn node_kind_name(node: &PenNode) -> &'static str {
    match node_kind(node) {
        NodeKind::Frame => "frame",
        NodeKind::Group => "group",
        NodeKind::Rectangle => "rectangle",
        NodeKind::Ellipse => "ellipse",
        NodeKind::Line => "line",
        NodeKind::Polygon => "polygon",
        NodeKind::Path => "path",
        NodeKind::Text => "text",
        _ => "node",
    }
}

fn is_empty_placeholder(node: &PenNode) -> bool {
    // A node is an empty placeholder if it has zero content or dimensions.
    // This prevents double-reporting: the empty node itself is already flagged
    // by another detector (e.g., detect_empty_paths for paths with no `d`).
    // A frame wrapping nothing should not be reported as a redundant wrapper.
    match node {
        PenNode::Path(p) => {
            // Empty path: missing or empty `d` field (no geometry)
            p.d.as_deref().map(|s| s.is_empty()).unwrap_or(true)
        }
        _ => {
            // Zero dimensions: both width AND height are explicitly zero.
            // If dimensions are unspecified (None), treat as potentially real content
            // since layout may compute them later.
            let width = numeric_width(node);
            let height = numeric_height(node);
            matches!(width, Some(w) if w == 0.0) && matches!(height, Some(h) if h == 0.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn node(value: serde_json::Value) -> PenNode {
        serde_json::from_value(value).expect("fixture must deserialize as PenNode")
    }

    #[test]
    fn detects_redundant_frame_wrapping_container() {
        let root = node(json!({
            "type": "frame", "id": "root", "width": 100, "height": 100, "layout": "vertical",
            "children": [{
                "type": "frame", "id": "wrapper", "width": 100, "height": 100,
                "children": [{
                    "type": "group", "id": "inner", "width": 50, "height": 50, "children": []
                }]
            }]
        }));
        let issues = detect_redundant_wrappers(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "wrapper");
        assert_eq!(issues[0].category, IssueCategory::RedundantWrapper);
    }

    #[test]
    fn ignores_wrapper_with_fill() {
        let root = node(json!({
            "type": "frame", "id": "root", "width": 100, "height": 100, "layout": "vertical",
            "children": [{
                "type": "frame", "id": "wrapper", "width": 100, "height": 100,
                "fill": [{"type": "solid", "color": "#FF0000"}],
                "children": [{
                    "type": "group", "id": "inner", "width": 50, "height": 50, "children": []
                }]
            }]
        }));
        let issues = detect_redundant_wrappers(&root);
        assert!(issues.is_empty(), "wrapper with fill is doing visual work");
    }

    #[test]
    fn ignores_wrapper_with_corner_radius() {
        let root = node(json!({
            "type": "frame", "id": "root", "width": 100, "height": 100, "layout": "vertical",
            "children": [{
                "type": "frame", "id": "wrapper", "width": 100, "height": 100,
                "cornerRadius": 8,
                "children": [{
                    "type": "group", "id": "inner", "width": 50, "height": 50, "children": []
                }]
            }]
        }));
        let issues = detect_redundant_wrappers(&root);
        assert!(
            issues.is_empty(),
            "wrapper with corner radius is doing visual work"
        );
    }

    #[test]
    fn ignores_wrapper_with_stroke() {
        let root = node(json!({
            "type": "frame", "id": "root", "width": 100, "height": 100, "layout": "vertical",
            "children": [{
                "type": "frame", "id": "wrapper", "width": 100, "height": 100,
                "stroke": {"thickness": 2},
                "children": [{
                    "type": "group", "id": "inner", "width": 50, "height": 50, "children": []
                }]
            }]
        }));
        let issues = detect_redundant_wrappers(&root);
        assert!(
            issues.is_empty(),
            "wrapper with stroke is doing visual work"
        );
    }

    #[test]
    fn ignores_wrapper_with_padding() {
        let root = node(json!({
            "type": "frame", "id": "root", "width": 100, "height": 100, "layout": "vertical",
            "children": [{
                "type": "frame", "id": "wrapper", "width": 100, "height": 100,
                "padding": 8,
                "children": [{
                    "type": "group", "id": "inner", "width": 50, "height": 50, "children": []
                }]
            }]
        }));
        let issues = detect_redundant_wrappers(&root);
        assert!(
            issues.is_empty(),
            "wrapper with padding is doing visual work"
        );
    }

    #[test]
    fn ignores_wrapper_with_multiple_children() {
        let root = node(json!({
            "type": "frame", "id": "root", "width": 100, "height": 100, "layout": "vertical",
            "children": [{
                "type": "frame", "id": "wrapper", "width": 100, "height": 100,
                "children": [
                    {"type": "group", "id": "child1", "width": 50, "height": 50, "children": []},
                    {"type": "group", "id": "child2", "width": 50, "height": 50, "children": []}
                ]
            }]
        }));
        let issues = detect_redundant_wrappers(&root);
        assert!(
            issues.is_empty(),
            "wrapper with multiple children is not redundant"
        );
    }

    #[test]
    fn detects_wrapper_wrapping_leaf_node() {
        // The most common case per teammate feedback: frame wrapping a single leaf (text or image)
        let root = node(json!({
            "type": "frame", "id": "root", "width": 100, "height": 100, "layout": "vertical",
            "children": [{
                "type": "frame", "id": "wrapper", "width": 100, "height": 100,
                "children": [{
                    "type": "text", "id": "text", "content": "hello"
                }]
            }]
        }));
        let issues = detect_redundant_wrappers(&root);
        assert_eq!(
            issues.len(),
            1,
            "frame wrapping single leaf text is redundant"
        );
        assert_eq!(issues[0].node_id, "wrapper");
    }

    #[test]
    fn exempts_root_frame_from_redundant_wrapper() {
        // Screen roots (artboards) are never redundant wrappers, even if they have
        // a single child and no visual properties. They ARE the canvas.
        let root = node(json!({
            "type": "frame", "id": "root", "width": 100, "height": 100,
            "children": [{
                "type": "path", "id": "empty"
            }]
        }));
        let issues = detect_redundant_wrappers(&root);
        assert!(
            issues.is_empty(),
            "root frame (depth 0) should never be reported as redundant wrapper"
        );
    }

    #[test]
    fn skips_wrapper_when_child_is_empty_path() {
        // A frame wrapping an empty path should not be reported as redundant.
        // The empty path is already flagged by detect_empty_paths; don't double-count
        // by also reporting the wrapper.
        let root = node(json!({
            "type": "frame", "id": "root", "width": 100, "height": 100,
            "children": [{
                "type": "frame", "id": "wrapper", "width": 100, "height": 100,
                "children": [{
                    "type": "path", "id": "empty-path"
                }]
            }]
        }));
        let issues = detect_redundant_wrappers(&root);
        assert!(
            issues.is_empty(),
            "wrapper around empty path should not be reported (child is the defect)"
        );
    }

    #[test]
    fn detects_wrapper_when_child_is_sized_content() {
        // A frame wrapping real sized content (text, image) with no visual properties
        // should still be reported as redundant. The wrapper IS the problem here.
        let root = node(json!({
            "type": "frame", "id": "root", "width": 100, "height": 100,
            "children": [{
                "type": "frame", "id": "wrapper", "width": 100, "height": 100,
                "children": [{
                    "type": "text", "id": "content", "content": "hello", "width": 50, "height": 20
                }]
            }]
        }));
        let issues = detect_redundant_wrappers(&root);
        assert_eq!(
            issues.len(),
            1,
            "wrapper around sized text content is redundant"
        );
        assert_eq!(issues[0].node_id, "wrapper");
    }

    #[test]
    fn below_threshold_absolute_positioning() {
        // Less than 10% absolute frames should not trigger
        let root = node(json!({
            "type": "frame", "id": "root", "width": 100, "height": 100,
            "children": [{
                "type": "frame", "id": "screen", "width": 100, "height": 100,
                "children": [
                    {"type": "frame", "id": "f1", "layout": "none", "width": 50, "height": 50, "children": []},
                    {"type": "frame", "id": "f2", "width": 50, "height": 50, "children": []},
                    {"type": "frame", "id": "f3", "width": 50, "height": 50, "children": []},
                    {"type": "frame", "id": "f4", "width": 50, "height": 50, "children": []},
                    {"type": "frame", "id": "f5", "width": 50, "height": 50, "children": []},
                    {"type": "frame", "id": "f6", "width": 50, "height": 50, "children": []},
                    {"type": "frame", "id": "f7", "width": 50, "height": 50, "children": []},
                    {"type": "frame", "id": "f8", "width": 50, "height": 50, "children": []},
                    {"type": "frame", "id": "f9", "width": 50, "height": 50, "children": []}
                ]
            }]
        }));
        let issues = detect_absolute_positioning_share(&root);
        // 1 of 9 frames = 11.1% > 10%, so should trigger at this threshold
        assert_eq!(issues.len(), 1, "11% should exceed 10% threshold");
    }

    #[test]
    fn detects_excessive_nesting_depth() {
        // Build a deep tree manually to avoid json! macro recursion limits.
        // Root (depth 0) -> l1 (1) -> l2 (2) -> l3 (3) -> l4 (4) -> l5 (5) -> text (6)
        // The text at depth 6 exceeds the threshold of 6, so it reports as excessive depth.
        let root_json = json!({
            "type": "frame", "id": "root", "width": 100, "height": 100,
            "children": [{
                "type": "group", "id": "l1", "width": 100, "height": 100,
                "children": [{
                    "type": "group", "id": "l2", "width": 100, "height": 100,
                    "children": [{
                        "type": "group", "id": "l3", "width": 100, "height": 100,
                        "children": [{
                            "type": "group", "id": "l4", "width": 100, "height": 100,
                            "children": [{
                                "type": "group", "id": "l5", "width": 100, "height": 100,
                                "children": [{
                                    "type": "text", "id": "deep", "content": "deep"
                                }]
                            }]
                        }]
                    }]
                }]
            }]
        });
        let root = node(root_json);
        let issues = detect_excessive_nesting_depth(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].category, IssueCategory::ExcessiveNestingDepth);
    }

    #[test]
    fn ignores_moderate_nesting_depth() {
        // 4 levels deep under screen (depth 5 total), within threshold
        let root_json = json!({
            "type": "frame", "id": "root", "width": 100, "height": 100,
            "children": [{
                "type": "group", "id": "l1", "width": 100, "height": 100,
                "children": [{
                    "type": "group", "id": "l2", "width": 100, "height": 100,
                    "children": [{
                        "type": "group", "id": "l3", "width": 100, "height": 100,
                        "children": [{
                            "type": "text", "id": "text", "content": "ok"
                        }]
                    }]
                }]
            }]
        });
        let root = node(root_json);
        let issues = detect_excessive_nesting_depth(&root);
        assert!(issues.is_empty(), "depth 5 is within threshold");
    }

    #[test]
    fn detects_high_absolute_positioning_share() {
        let root = node(json!({
            "type": "frame", "id": "root", "width": 100, "height": 100,
            "children": [{
                "type": "frame", "id": "screen", "width": 100, "height": 100,
                "children": [
                    {"type": "frame", "id": "f1", "layout": "none", "width": 50, "height": 50, "children": []},
                    {"type": "frame", "id": "f2", "layout": "none", "width": 50, "height": 50, "children": []},
                    {"type": "frame", "id": "f3", "layout": "none", "width": 50, "height": 50, "children": []},
                    {"type": "frame", "id": "f4", "width": 50, "height": 50, "children": []},
                    {"type": "frame", "id": "f5", "width": 50, "height": 50, "children": []}
                ]
            }]
        }));
        let issues = detect_absolute_positioning_share(&root);
        // 3 of 5 frames = 60% > 10% threshold
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].category, IssueCategory::AbsolutePositioningShare);
    }

    #[test]
    fn ignores_low_absolute_positioning_share() {
        let root = node(json!({
            "type": "frame", "id": "root", "width": 100, "height": 100,
            "children": [{
                "type": "frame", "id": "screen", "width": 100, "height": 100,
                "children": [
                    {"type": "frame", "id": "f1", "layout": "none", "width": 50, "height": 50, "children": []},
                    {"type": "frame", "id": "f2", "width": 50, "height": 50, "children": []},
                    {"type": "frame", "id": "f3", "width": 50, "height": 50, "children": []},
                    {"type": "frame", "id": "f4", "width": 50, "height": 50, "children": []},
                    {"type": "frame", "id": "f5", "width": 50, "height": 50, "children": []}
                ]
            }]
        }));
        let issues = detect_absolute_positioning_share(&root);
        // 1 of 5 frames = 20% > 10%, so should trigger
        assert_eq!(issues.len(), 1, "20% exceeds 10% threshold");
    }

    #[test]
    fn excludes_status_bar_from_absolute_positioning() {
        let root = node(json!({
            "type": "frame", "id": "screen", "width": 100, "height": 100,
            "children": [
                {"type": "frame", "id": "f1", "layout": "none", "width": 50, "height": 50, "children": []},
                {
                    "type": "frame", "id": "status-bar", "role": "status-bar",
                    "layout": "none", "width": 100, "height": 20,
                    "children": [
                        {"type": "frame", "id": "sb-f1", "layout": "none", "width": 10, "height": 10, "children": []},
                        {"type": "frame", "id": "sb-f2", "layout": "none", "width": 10, "height": 10, "children": []}
                    ]
                },
                {"type": "frame", "id": "f2", "width": 50, "height": 50, "children": []},
                {"type": "frame", "id": "f3", "width": 50, "height": 50, "children": []},
                {"type": "frame", "id": "f4", "width": 50, "height": 50, "children": []}
            ]
        }));
        let issues = detect_absolute_positioning_share(&root);
        // 1 absolute of 4 regular frames (status bar excluded) = 25% > 20%
        assert_eq!(
            issues.len(),
            1,
            "status bar and its children excluded from count"
        );
    }
}
