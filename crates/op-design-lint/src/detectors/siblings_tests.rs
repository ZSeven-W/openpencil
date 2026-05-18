//! Tests for `detectors::siblings`.
//!
//! Relocated from `siblings.rs` to keep that file under the workspace
//! 800-line ceiling (matching the `*_tests.rs` sibling-test convention).
//! The three test modules below are unchanged — only moved.

use super::siblings::{
    detect_mixed_sibling_corner_radius, detect_mixed_sibling_padding,
    detect_sibling_inconsistencies,
};
use crate::issue::{FixProperty, IssueCategory, IssueSeverity};
use jian_ops_schema::node::PenNode;

#[cfg(test)]
mod sibling_inconsistency_tests {
    use super::*;
    use serde_json::json;

    fn node(value: serde_json::Value) -> PenNode {
        serde_json::from_value(value).expect("fixture must deserialize as PenNode")
    }

    #[test]
    fn flags_strict_height_outlier_among_same_role_frames() {
        // 3 same-role frames; one has a different height.
        let root = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "frame", "id": "a", "role": "card", "height": 100},
                {"type": "frame", "id": "b", "role": "card", "height": 100},
                {"type": "frame", "id": "c", "role": "card", "height": 140}
            ]
        }));
        let issues = detect_sibling_inconsistencies(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "c");
        assert_eq!(issues[0].category, IssueCategory::SiblingInconsistency);
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
        assert_eq!(issues[0].property, FixProperty::Height);
        assert_eq!(issues[0].suggested_value, json!(100));
        assert_eq!(issues[0].current_value, json!(140));
    }

    #[test]
    fn flags_strict_corner_radius_outlier() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "frame", "id": "a", "role": "card", "cornerRadius": 8},
                {"type": "frame", "id": "b", "role": "card", "cornerRadius": 8},
                {"type": "frame", "id": "c", "role": "card", "cornerRadius": 16}
            ]
        }));
        let issues = detect_sibling_inconsistencies(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "c");
        assert_eq!(issues[0].property, FixProperty::CornerRadius);
    }

    #[test]
    fn never_flags_a_two_sibling_group() {
        // Fewer than 3 siblings — never flagged.
        let root = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "frame", "id": "a", "role": "card", "height": 100},
                {"type": "frame", "id": "b", "role": "card", "height": 140}
            ]
        }));
        assert!(detect_sibling_inconsistencies(&root).is_empty());
    }

    #[test]
    fn does_not_flag_three_way_split() {
        // 1-1-1: no >= 2/3 majority → nothing flagged.
        let root = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "frame", "id": "a", "role": "card", "height": 100},
                {"type": "frame", "id": "b", "role": "card", "height": 120},
                {"type": "frame", "id": "c", "role": "card", "height": 140}
            ]
        }));
        assert!(detect_sibling_inconsistencies(&root).is_empty());
    }

    #[test]
    fn divider_and_spacer_siblings_are_excluded_from_groups() {
        // 3 cards consistent + 1 divider with a wild height. The divider must
        // not enter any group; the cards stay consistent → no issue.
        let root = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "frame", "id": "a", "role": "card", "height": 100},
                {"type": "frame", "id": "b", "role": "card", "height": 100},
                {"type": "frame", "id": "c", "role": "card", "height": 100},
                {"type": "frame", "id": "d", "role": "divider", "height": 1},
                {"type": "frame", "id": "e", "role": "spacer", "height": 999}
            ]
        }));
        assert!(detect_sibling_inconsistencies(&root).is_empty());
    }

    #[test]
    fn loose_pass_flags_singleton_role_corner_radius_outlier_as_info() {
        // 3 frames, each a unique role — strict groups are all singletons.
        // The loose pass groups them by type and flags the cornerRadius
        // outlier with `info` severity.
        let root = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "frame", "id": "hero", "role": "hero", "cornerRadius": 12},
                {"type": "frame", "id": "feat", "role": "features", "cornerRadius": 12},
                {"type": "frame", "id": "cta", "role": "cta", "cornerRadius": 24}
            ]
        }));
        let issues = detect_sibling_inconsistencies(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "cta");
        assert_eq!(issues[0].property, FixProperty::CornerRadius);
        assert_eq!(issues[0].severity, IssueSeverity::Info);
    }

    #[test]
    fn dedup_keeps_strict_pass_when_both_passes_match() {
        // 3 same-role frames with a cornerRadius outlier: caught by the strict
        // pass (warning) AND the loose pass (info). Dedup keeps the strict one.
        let root = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "frame", "id": "a", "role": "card", "cornerRadius": 8},
                {"type": "frame", "id": "b", "role": "card", "cornerRadius": 8},
                {"type": "frame", "id": "c", "role": "card", "cornerRadius": 16}
            ]
        }));
        let issues = detect_sibling_inconsistencies(&root);
        // Exactly one issue for node `c` — the strict-pass `warning`, not a
        // duplicate `info`.
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "c");
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
    }

    #[test]
    fn text_siblings_check_font_size() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "text", "id": "a", "role": "label", "content": "x", "fontSize": 14},
                {"type": "text", "id": "b", "role": "label", "content": "y", "fontSize": 14},
                {"type": "text", "id": "c", "role": "label", "content": "z", "fontSize": 18}
            ]
        }));
        let issues = detect_sibling_inconsistencies(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "c");
        assert_eq!(issues[0].property, FixProperty::FontSize);
    }
}

#[cfg(test)]
mod mixed_corner_radius_tests {
    use super::*;
    use serde_json::json;

    fn node(value: serde_json::Value) -> PenNode {
        serde_json::from_value(value).expect("fixture must deserialize as PenNode")
    }

    #[test]
    fn flags_corner_radius_outlier_against_modal() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "frame", "id": "a", "role": "card", "cornerRadius": 8},
                {"type": "frame", "id": "b", "role": "card", "cornerRadius": 8},
                {"type": "frame", "id": "c", "role": "card", "cornerRadius": 12}
            ]
        }));
        let issues = detect_mixed_sibling_corner_radius(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "c");
        assert_eq!(issues[0].category, IssueCategory::MixedSiblingCornerRadius);
        assert_eq!(issues[0].property, FixProperty::CornerRadius);
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
        assert_eq!(issues[0].current_value, json!(12));
        assert_eq!(issues[0].suggested_value, json!(8));
    }

    #[test]
    fn ignores_three_way_split_without_modal() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "frame", "id": "a", "role": "card", "cornerRadius": 8},
                {"type": "frame", "id": "b", "role": "card", "cornerRadius": 12},
                {"type": "frame", "id": "c", "role": "card", "cornerRadius": 16}
            ]
        }));
        assert!(detect_mixed_sibling_corner_radius(&root).is_empty());
    }

    #[test]
    fn ignores_two_sibling_group() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "frame", "id": "a", "role": "card", "cornerRadius": 8},
                {"type": "frame", "id": "b", "role": "card", "cornerRadius": 12}
            ]
        }));
        assert!(detect_mixed_sibling_corner_radius(&root).is_empty());
    }

    #[test]
    fn ignores_all_equal_group() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "frame", "id": "a", "role": "card", "cornerRadius": 8},
                {"type": "frame", "id": "b", "role": "card", "cornerRadius": 8},
                {"type": "frame", "id": "c", "role": "card", "cornerRadius": 8}
            ]
        }));
        assert!(detect_mixed_sibling_corner_radius(&root).is_empty());
    }
}

#[cfg(test)]
mod mixed_padding_tests {
    use super::*;
    use serde_json::json;

    fn node(value: serde_json::Value) -> PenNode {
        serde_json::from_value(value).expect("fixture must deserialize as PenNode")
    }

    #[test]
    fn flags_padding_outlier_with_shorthand_normalised_modal() {
        // `16` and `[16,16,16,16]` normalise equal; `20` is the outlier.
        let root = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "frame", "id": "a", "role": "card", "padding": 16},
                {"type": "frame", "id": "b", "role": "card", "padding": [16, 16, 16, 16]},
                {"type": "frame", "id": "c", "role": "card", "padding": 20}
            ]
        }));
        let issues = detect_mixed_sibling_padding(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "c");
        assert_eq!(issues[0].category, IssueCategory::MixedSiblingPadding);
        assert_eq!(issues[0].property, FixProperty::Padding);
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
        // All four sides equal → suggested as a single number.
        assert_eq!(issues[0].suggested_value, json!(16));
    }

    #[test]
    fn flags_padding_outlier_with_xy_modal() {
        // `[8,16]` (XY) normalises to `[8,16,8,16]`; `[8,20]` is the outlier.
        let root = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "frame", "id": "a", "role": "card", "padding": [8, 16]},
                {"type": "frame", "id": "b", "role": "card", "padding": [8, 16]},
                {"type": "frame", "id": "c", "role": "card", "padding": [8, 20]}
            ]
        }));
        let issues = detect_mixed_sibling_padding(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "c");
        // Sides differ → suggested as a 4-element array.
        assert_eq!(issues[0].suggested_value, json!([8, 16, 8, 16]));
    }

    #[test]
    fn ignores_three_way_padding_split() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "frame", "id": "a", "role": "card", "padding": 8},
                {"type": "frame", "id": "b", "role": "card", "padding": 16},
                {"type": "frame", "id": "c", "role": "card", "padding": 24}
            ]
        }));
        assert!(detect_mixed_sibling_padding(&root).is_empty());
    }

    #[test]
    fn current_value_normalizes_integral_padding_to_json_integer() {
        // jian deserializes JSON `20` into `Padding`'s f64 fields; the
        // `current_value` must serialize back as the integer `20`, not the
        // float `20.0` — `serde_json::Value` does NOT treat 20.0 == 20.

        // Uniform-number outlier → current_value is a JSON integer.
        let uniform = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "frame", "id": "a", "role": "card", "padding": 16},
                {"type": "frame", "id": "b", "role": "card", "padding": 16},
                {"type": "frame", "id": "c", "role": "card", "padding": 20}
            ]
        }));
        let issues = detect_mixed_sibling_padding(&uniform);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].current_value, json!(20));

        // Array-form outlier → current_value is an integer 4-array (the
        // sibling carried an explicit `[8,20,8,20]`).
        let array = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "frame", "id": "a", "role": "card", "padding": [8, 16, 8, 16]},
                {"type": "frame", "id": "b", "role": "card", "padding": [8, 16, 8, 16]},
                {"type": "frame", "id": "c", "role": "card", "padding": [8, 20, 8, 20]}
            ]
        }));
        let issues = detect_mixed_sibling_padding(&array);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].current_value, json!([8, 20, 8, 20]));
    }
}
