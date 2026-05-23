//! Design-diagnostics detectors, ported from the TS `pen-ai-skills`
//! diagnostics layer. Each detector is a pure recursive walk over the jian
//! `PenNode` tree returning `Vec<Issue>`.
//!
//! `detect_all` runs all 14 detectors in the TS `detectAllIssues` order and
//! returns the deduplicated combined issue list.

use std::collections::HashSet;

use jian_ops_schema::node::PenNode;
use jian_ops_schema::PenDocument;

use crate::issue::Issue;

pub mod siblings;
pub mod spacing;
pub mod structure;
pub mod text;
pub mod typography;

#[cfg(test)]
mod siblings_tests;

pub use siblings::*;
pub use spacing::*;
pub use structure::*;
pub use text::*;
pub use typography::*;

/// Port of `detectAllIssues` (`detectors.ts:698-724`). Runs the 14 detectors
/// in the exact TS call order, concatenates their issue lists, and dedups on
/// `{node_id}:{property}` — the first occurrence (in detector order, then
/// tree-walk order) wins.
///
/// The dedup key reuses [`crate::issue::FixProperty::wire_str`] so the
/// `{node_id}:{property}` string is byte-identical to the TS
/// `` `${issue.nodeId}:${issue.property}` `` key.
pub fn detect_all(root: &PenNode, doc: &PenDocument) -> Vec<Issue> {
    let mut combined = Vec::new();
    combined.extend(detect_invisible_containers(root, doc));
    combined.extend(detect_empty_paths(root));
    combined.extend(detect_text_explicit_heights(root));
    combined.extend(detect_sibling_inconsistencies(root));
    combined.extend(detect_unexpected_rotation(root));
    combined.extend(detect_text_corner_radius(root));
    combined.extend(detect_mixed_sibling_corner_radius(root));
    combined.extend(detect_text_effect(root));
    combined.extend(detect_text_stroke(root));
    combined.extend(detect_mixed_sibling_padding(root));
    combined.extend(detect_excessive_frame_effects(root));
    combined.extend(detect_edge_section_padding(root));
    combined.extend(detect_stacked_horizontal_padding(root));
    combined.extend(detect_text_bg_contrast(root, doc));

    let mut seen = HashSet::new();
    combined
        .into_iter()
        .filter(|issue| seen.insert(format!("{}:{}", issue.node_id, issue.property.wire_str())))
        .collect()
}

#[cfg(test)]
mod detect_all_tests {
    use super::*;
    use crate::issue::{FixProperty, IssueCategory, IssueSeverity};
    use serde_json::json;

    fn node(value: serde_json::Value) -> PenNode {
        serde_json::from_value(value).expect("fixture must deserialize as PenNode")
    }

    fn doc(value: serde_json::Value) -> PenDocument {
        serde_json::from_value(value).expect("fixture must deserialize as PenDocument")
    }

    /// A clean document with no detectable issues → empty list.
    #[test]
    fn detect_all_on_clean_doc_returns_empty() {
        let root = node(json!({
            "type": "frame", "id": "root", "layout": "vertical",
            "fill": [{"type": "solid", "color": "#FFFFFF"}],
            "children": [
                {
                    "type": "text", "id": "t1", "content": "Hello",
                    "fill": [{"type": "solid", "color": "#000000"}]
                }
            ]
        }));
        assert!(detect_all(&root, &doc(json!({"version": "1.0", "children": []}))).is_empty());
    }

    /// A `cornerRadius` outlier among same-role frames is caught by BOTH
    /// `detect_sibling_inconsistencies` (4th, `warning`) and
    /// `detect_mixed_sibling_corner_radius` (7th, `warning`) on the same
    /// `(node_id, "cornerRadius")` key. `detect_all` dedups to the
    /// sibling-inconsistency issue — it runs earlier in TS order.
    #[test]
    fn detect_all_dedups_collision_keeping_earlier_detector() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "frame", "id": "a", "role": "card", "cornerRadius": 8},
                {"type": "frame", "id": "b", "role": "card", "cornerRadius": 8},
                {"type": "frame", "id": "c", "role": "card", "cornerRadius": 16}
            ]
        }));
        let issues = detect_all(&root, &doc(json!({"version": "1.0", "children": []})));
        // Exactly one issue for node `c` — the earlier sibling-inconsistency
        // detector wins; the mixed-corner-radius duplicate is dropped.
        assert_eq!(
            issues
                .iter()
                .filter(|i| i.node_id == "c" && i.property == FixProperty::CornerRadius)
                .count(),
            1
        );
        let c_issue = issues
            .iter()
            .find(|i| i.node_id == "c")
            .expect("node c flagged");
        assert_eq!(c_issue.category, IssueCategory::SiblingInconsistency);
        assert_eq!(c_issue.severity, IssueSeverity::Warning);
    }

    /// `detect_all` runs the 14 detectors in TS order — assert the combined
    /// list keeps that order across detectors. An empty path (detector 2)
    /// must precede an unexpected rotation (detector 5) on a doc carrying
    /// both, regardless of tree position.
    #[test]
    fn detect_all_preserves_ts_detector_order() {
        // The rotated frame is the FIRST child; the empty path is the SECOND.
        // Tree-walk order would emit rotation before empty-path, but
        // `detect_all`'s detector order (empty_paths is 2nd, rotation is 5th)
        // must put the empty-path issue first.
        let root = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "frame", "id": "tilted", "rotation": 12},
                {"type": "path", "id": "empty"}
            ]
        }));
        let issues = detect_all(&root, &doc(json!({"version": "1.0", "children": []})));
        let categories: Vec<IssueCategory> = issues.iter().map(|i| i.category).collect();
        assert_eq!(
            categories,
            vec![IssueCategory::EmptyPath, IssueCategory::UnexpectedRotation]
        );
    }
}
