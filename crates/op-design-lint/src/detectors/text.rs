//! Text-node detectors ported from `pen-ai-skills/src/diagnostics/detectors.ts`.

use jian_ops_schema::node::{PenNode, TextGrowth};
use jian_ops_schema::sizing::SizingBehavior;
use serde_json::Value;

use crate::issue::{FixProperty, Issue, IssueCategory, IssueSeverity};
use crate::node_util::{children, json_number, node_id};

/// Port of `detectTextExplicitHeights` (`detectors.ts:156-180`). Flags `text`
/// nodes that carry a numeric explicit `height` while `textGrowth` is not
/// `fixed-width-height` — a pinned pixel height clips text whose intrinsic
/// size differs from the box.
///
/// T0 audit detector 3: jian `TextNode.height` is `Option<SizingBehavior>`.
/// The TS `typeof height === 'number'` guard maps to the
/// `SizingBehavior::Number` variant only — `Keyword` (`fit_content` /
/// `fill_container`) and `Expression` (`$variable` ref) are not "an explicit
/// pixel height". `textGrowth` maps to `TextGrowth::FixedWidthHeight`.
pub fn detect_text_explicit_heights(root: &PenNode) -> Vec<Issue> {
    let mut issues = Vec::new();
    walk_text_explicit_heights(root, &mut issues);
    issues
}

fn walk_text_explicit_heights(node: &PenNode, issues: &mut Vec<Issue>) {
    if let PenNode::Text(text) = node {
        if let Some(SizingBehavior::Number(height)) = text.height {
            let is_fixed = matches!(text.text_growth, Some(TextGrowth::FixedWidthHeight));
            if !is_fixed {
                issues.push(Issue {
                    node_id: node_id(node).to_string(),
                    category: IssueCategory::TextExplicitHeight,
                    severity: IssueSeverity::Warning,
                    property: FixProperty::Height,
                    current_value: json_number(height),
                    suggested_value: Value::String("fit_content".to_string()),
                    reason: format!("text node has explicit height={height}px — causes clipping"),
                });
            }
        }
    }
    for child in children(node) {
        walk_text_explicit_heights(child, issues);
    }
}

/// Port of `detectTextEffect` (`detectors.ts:468-490`). Flags `text` nodes
/// carrying a non-empty `effects` array — shadow / blur on a UI label is
/// almost always an AI hallucination.
///
/// T0 audit detector 8: jian `TextNode` DOES carry an `effects` field
/// (`Option<Vec<PenEffect>>`) — unlike `stroke` / `cornerRadius`, this is not
/// a structural no-op. The TS `Array.isArray(eff) && eff.length > 0` guard
/// maps to `Some(effects)` with `!effects.is_empty()`.
pub fn detect_text_effect(root: &PenNode) -> Vec<Issue> {
    let mut issues = Vec::new();
    walk_text_effect(root, &mut issues);
    issues
}

fn walk_text_effect(node: &PenNode, issues: &mut Vec<Issue>) {
    if let PenNode::Text(text) = node {
        if let Some(effects) = text.effects.as_ref() {
            if !effects.is_empty() {
                issues.push(Issue {
                    node_id: node_id(node).to_string(),
                    category: IssueCategory::TextEffect,
                    severity: IssueSeverity::Warning,
                    property: FixProperty::Effects,
                    current_value: serde_json::to_value(effects).unwrap_or(Value::Null),
                    suggested_value: Value::Null,
                    reason: "text node carries shadow / blur effects — almost always an AI \
                             hallucination on UI labels"
                        .to_string(),
                });
            }
        }
    }
    for child in children(node) {
        walk_text_effect(child, issues);
    }
}

/// Port of `detectTextCornerRadius` (`detectors.ts:364-389`).
///
/// The TS detector flags `text` nodes carrying a stray `cornerRadius`. jian's
/// strict `TextNode` has no `cornerRadius` field (T0 audit §3, decision (a))
/// — the bug class is structurally unrepresentable, so this detector is a
/// correct no-op on jian-native documents. `serde_json` silently drops a
/// `cornerRadius` key on a text node at deserialize time. Kept for the
/// 14-detector count and `detect_all` ordering parity.
pub fn detect_text_corner_radius(root: &PenNode) -> Vec<Issue> {
    let mut issues = Vec::new();
    walk_text_corner_radius(root, &mut issues);
    issues
}

fn walk_text_corner_radius(node: &PenNode, _issues: &mut Vec<Issue>) {
    // jian `TextNode` exposes no `cornerRadius` — nothing to inspect. The
    // `_issues` accumulator is threaded for shape-parity with the real
    // detectors but is never written (clippy: `only_used_in_recursion`).
    for child in children(node) {
        walk_text_corner_radius(child, _issues);
    }
}

/// Port of `detectTextStroke` (`detectors.ts:506-535`).
///
/// The TS detector flags `text` nodes carrying a stray `stroke`. jian's
/// strict `TextNode` has no `stroke` field (T0 audit §3, decision (a)) — the
/// bug class is structurally unrepresentable, so this detector is a correct
/// no-op on jian-native documents. `serde_json` silently drops a `stroke`
/// key on a text node at deserialize time. Kept for the 14-detector count
/// and `detect_all` ordering parity.
pub fn detect_text_stroke(root: &PenNode) -> Vec<Issue> {
    let mut issues = Vec::new();
    walk_text_stroke(root, &mut issues);
    issues
}

fn walk_text_stroke(node: &PenNode, _issues: &mut Vec<Issue>) {
    // jian `TextNode` exposes no `stroke` — nothing to inspect. The `_issues`
    // accumulator is threaded for shape-parity with the real detectors but is
    // never written (clippy: `only_used_in_recursion`).
    for child in children(node) {
        walk_text_stroke(child, _issues);
    }
}

#[cfg(test)]
mod structural_noop_tests {
    use super::*;
    use serde_json::json;

    fn node(value: serde_json::Value) -> PenNode {
        serde_json::from_value(value).expect("fixture must deserialize as PenNode")
    }

    /// A frame holding two text children — exercises the tree walk for both
    /// structural-no-op detectors.
    fn frame_with_text_children_fixture() -> PenNode {
        node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "text", "id": "t1", "content": "label one"},
                {"type": "frame", "id": "inner", "children": [
                    {"type": "text", "id": "t2", "content": "label two"}
                ]}
            ]
        }))
    }

    #[test]
    fn text_corner_radius_is_structural_noop_on_jian_docs() {
        // jian TextNode cannot carry cornerRadius (audit §3 decision (a)).
        let root = frame_with_text_children_fixture();
        assert!(detect_text_corner_radius(&root).is_empty());
    }

    #[test]
    fn text_stroke_is_structural_noop_on_jian_docs() {
        // jian TextNode cannot carry stroke (audit §3 decision (a)).
        let root = frame_with_text_children_fixture();
        assert!(detect_text_stroke(&root).is_empty());
    }
}

#[cfg(test)]
mod text_effect_tests {
    use super::*;
    use serde_json::json;

    fn node(value: serde_json::Value) -> PenNode {
        serde_json::from_value(value).expect("fixture must deserialize as PenNode")
    }

    fn shadow() -> serde_json::Value {
        json!({
            "type": "shadow",
            "offsetX": 0, "offsetY": 2,
            "blur": 8, "spread": 0,
            "color": "#000000"
        })
    }

    #[test]
    fn flags_text_with_effects() {
        let root = node(json!({
            "type": "text", "id": "t1", "content": "hi",
            "effects": [shadow()]
        }));
        let issues = detect_text_effect(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "t1");
        assert_eq!(issues[0].category, IssueCategory::TextEffect);
        assert_eq!(issues[0].property, FixProperty::Effects);
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
        assert_eq!(issues[0].suggested_value, json!(null));
    }

    #[test]
    fn ignores_text_without_effects() {
        // No `effects` field.
        let bare = node(json!({"type": "text", "id": "t1", "content": "hi"}));
        assert!(detect_text_effect(&bare).is_empty());

        // Empty `effects` array.
        let empty = node(json!({
            "type": "text", "id": "t1", "content": "hi", "effects": []
        }));
        assert!(detect_text_effect(&empty).is_empty());
    }

    #[test]
    fn ignores_frame_with_effects() {
        let root = node(json!({
            "type": "frame", "id": "f1", "effects": [shadow()]
        }));
        assert!(detect_text_effect(&root).is_empty());
    }

    #[test]
    fn walks_nested_text_nodes() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "frame", "id": "inner", "children": [
                    {"type": "text", "id": "deep", "content": "hi", "effects": [shadow()]}
                ]}
            ]
        }));
        let issues = detect_text_effect(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "deep");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Deserialize a JSON value into a `PenNode` — doubles as a schema
    /// round-trip check for every fixture.
    fn node(value: serde_json::Value) -> PenNode {
        serde_json::from_value(value).expect("fixture must deserialize as PenNode")
    }

    #[test]
    fn flags_text_with_explicit_numeric_height() {
        let root = node(json!({
            "type": "text", "id": "t1", "content": "hi", "height": 40
        }));
        let issues = detect_text_explicit_heights(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "t1");
        assert_eq!(issues[0].category, IssueCategory::TextExplicitHeight);
        assert_eq!(issues[0].property, FixProperty::Height);
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
        assert_eq!(issues[0].suggested_value, json!("fit_content"));
        assert_eq!(issues[0].current_value, json!(40));
    }

    #[test]
    fn ignores_text_with_keyword_sizing() {
        let root = node(json!({
            "type": "text", "id": "t1", "content": "hi", "height": "fit_content"
        }));
        assert!(detect_text_explicit_heights(&root).is_empty());

        // No `height` field at all → none.
        let bare = node(json!({"type": "text", "id": "t1", "content": "hi"}));
        assert!(detect_text_explicit_heights(&bare).is_empty());
    }

    #[test]
    fn ignores_text_with_fixed_width_height_growth() {
        let root = node(json!({
            "type": "text", "id": "t1", "content": "hi",
            "height": 40, "textGrowth": "fixed-width-height"
        }));
        assert!(detect_text_explicit_heights(&root).is_empty());
    }

    #[test]
    fn ignores_non_text_node_with_explicit_height() {
        let root = node(json!({"type": "frame", "id": "f1", "height": 40}));
        assert!(detect_text_explicit_heights(&root).is_empty());
    }

    #[test]
    fn walks_nested_text_nodes() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "frame", "id": "inner", "children": [
                    {"type": "text", "id": "deep", "content": "hi", "height": 24}
                ]}
            ]
        }));
        let issues = detect_text_explicit_heights(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "deep");
    }
}
