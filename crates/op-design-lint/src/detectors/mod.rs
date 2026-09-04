//! Design-diagnostics detectors, ported from the TS `pen-ai-skills`
//! diagnostics layer. Each detector is a pure recursive walk over the jian
//! `PenNode` tree returning `Vec<Issue>`.
//!
//! `detect_all` runs the detectors in the TS `detectAllIssues` order, followed
//! by the local geometry and accessibility checks, and
//! returns the deduplicated combined issue list.

use std::collections::HashSet;

use jian_ops_schema::node::PenNode;
use jian_ops_schema::PenDocument;

use crate::design_form::{classify_root_form_node, DesignForm};
use crate::issue::Issue;

pub mod empty_filled_panel;
pub mod shader_budget;
pub mod siblings;
pub mod spacing;
pub mod structural_quality;
pub mod structure;
pub mod text;
pub mod top_anchored_bars;
pub mod typography;
pub mod widget_a11y;

#[cfg(test)]
mod siblings_tests;
#[cfg(test)]
mod spacing_edge_tests;

pub use empty_filled_panel::*;
pub use shader_budget::*;
pub use siblings::*;
pub use spacing::*;
pub use structural_quality::*;
pub use structure::*;
pub use text::*;
pub use top_anchored_bars::*;
pub use typography::*;
pub use widget_a11y::*;

/// Port of `detectAllIssues` (`detectors.ts:698-724`). Runs the detectors
/// in the exact TS call order, concatenates their issue lists, and dedups on
/// `{node_id}:{property}` — the first occurrence (in detector order, then
/// tree-walk order) wins.
///
/// The dedup key reuses [`crate::issue::FixProperty::wire_str`] so the
/// `{node_id}:{property}` string is byte-identical to the TS
/// `` `${issue.nodeId}:${issue.property}` `` key.
pub fn detect_all(root: &PenNode, doc: &PenDocument) -> Vec<Issue> {
    detect_all_for_form(root, doc, classify_root_form_node(root))
}

/// [`detect_all`] told what kind of surface `root` is, for callers that
/// already know — the orchestrator holds the planned artboard, and on an
/// append path the node handed here is not always the artboard itself.
///
/// A deck board takes different geometry and typography floors from a
/// scrolling page (deck-system spec §4.1): a projector is read from the back
/// row, so its font floor and margins are absolute rather than relative to a
/// brand's rhythm. **No detector branches on the form yet** — this is the
/// wiring the deck detectors (spec §4.2) land on, and the point of putting it
/// in first is that each of them receives the form from the single classifier
/// instead of re-deriving it from a width comparison of its own.
pub fn detect_all_for_form(root: &PenNode, doc: &PenDocument, form: DesignForm) -> Vec<Issue> {
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
    combined.extend(detect_empty_filled_panel(root));
    combined.extend(detect_top_anchored_bars(root));
    // Structural workability detectors — report-only findings on nesting and layout.
    combined.extend(detect_redundant_wrappers(root));
    combined.extend(detect_excessive_nesting_depth(root));
    combined.extend(detect_absolute_positioning_share(root));
    // GPU budget for shader fills — the only detector that branches on the
    // form today, because a phone and a desktop page genuinely have different
    // fragment-pass headroom.
    combined.extend(detect_shader_budget(root, form));
    // Phase E5 — widget a11y. No TS counterpart; runs last so it never
    // shadows an earlier detector under the `{node_id}:{property}` dedup.
    combined.extend(detect_unlabeled_inputs(root));

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
    use crate::node_util::{children, node_id, node_y, numeric_height};
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

    /// `detect_all` classifies the root itself so every existing caller
    /// becomes form-aware the moment a detector starts branching — and, until
    /// one does, a deck board lints exactly like anything else.
    #[test]
    fn detect_all_classifies_the_root_and_reports_the_same_issues_for_now() {
        let board = node(json!({
            "type": "frame", "id": "board", "width": 1920, "height": 1080,
            "fill": [{"type": "solid", "color": "#FFFFFF"}],
            "children": [
                {"type": "frame", "id": "tilted", "rotation": 12},
                {"type": "path", "id": "empty"}
            ]
        }));
        let document = doc(json!({"version": "1.0", "children": []}));
        assert_eq!(
            crate::design_form::classify_root_form_node(&board),
            crate::design_form::DesignForm::Deck
        );
        assert_eq!(
            detect_all(&board, &document),
            detect_all_for_form(&board, &document, DesignForm::Page),
            "no detector branches on the form yet — the deck floors land later"
        );
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

    /// Reduced copy of the two affected artboards from the K3 deck. The
    /// fixture is intentionally self-contained so the test never reads the
    /// external `.op` sample.
    #[test]
    fn deck_fixture_reports_two_empty_panels_and_repairs_one_bar_group() {
        let mut document = deck_fixture();
        let all_issues: Vec<Issue> = document
            .children
            .iter()
            .flat_map(|root| detect_all(root, &document))
            .collect();

        assert_eq!(
            all_issues
                .iter()
                .filter(|issue| issue.category == IssueCategory::EmptyFilledPanel)
                .count(),
            2
        );
        let bar_issues: Vec<&Issue> = all_issues
            .iter()
            .filter(|issue| issue.category == IssueCategory::TopAnchoredBars)
            .collect();
        assert_eq!(bar_issues.len(), 6);
        assert!(bar_issues
            .iter()
            .all(|issue| issue.suggested_value.is_number()));

        let original_heights: Vec<f64> = children(&document.children[0])
            .iter()
            .filter(|node| node_id(node).starts_with("bar-"))
            .map(|node| numeric_height(node).expect("bar height"))
            .collect();
        let report = crate::apply_fixes(&mut document, &all_issues);
        assert_eq!(report.total, 6);
        assert_eq!(
            report.by_category.get(&IssueCategory::TopAnchoredBars),
            Some(&6)
        );

        let repaired_bars: Vec<&PenNode> = children(&document.children[0])
            .iter()
            .filter(|node| node_id(node).starts_with("bar-"))
            .collect();
        assert_eq!(repaired_bars.len(), 6);
        let baseline =
            node_y(repaired_bars[0]).unwrap() + numeric_height(repaired_bars[0]).unwrap();
        for (bar, original_height) in repaired_bars.iter().zip(original_heights) {
            assert_eq!(numeric_height(bar), Some(original_height));
            assert!((node_y(bar).unwrap() + original_height - baseline).abs() < 1e-9);
        }
    }

    fn deck_fixture() -> PenDocument {
        doc(json!({
            "version": "1.0",
            "children": [
                {
                    "type": "frame", "id": "slide-04", "name": "04-增长趋势",
                    "width": 1920, "height": 1080,
                    "fill": [{"type":"solid", "color":"#F7F4EE"}],
                    "children": [
                        {"type":"rectangle", "id":"axis", "x":120, "y":378, "width":1120, "height":2,
                         "fill":[{"type":"solid","color":"#D3D1CD"}]},
                        {"type":"rectangle", "id":"bar-a", "x":150, "y":380, "width":96, "height":284,
                         "fill":[{"type":"solid","color":"#044A7D"}]},
                        {"type":"rectangle", "id":"bar-b", "x":326, "y":380, "width":96, "height":316,
                         "fill":[{"type":"solid","color":"#266EA4"}]},
                        {"type":"rectangle", "id":"bar-c", "x":502, "y":380, "width":96, "height":342,
                         "fill":[{"type":"solid","color":"#266EA4"}]},
                        {"type":"rectangle", "id":"bar-d", "x":678, "y":380, "width":96, "height":392,
                         "fill":[{"type":"solid","color":"#266EA4"}]},
                        {"type":"rectangle", "id":"bar-e", "x":854, "y":380, "width":96, "height":420,
                         "fill":[{"type":"solid","color":"#266EA4"}]},
                        {"type":"rectangle", "id":"bar-f", "x":1030, "y":380, "width":96, "height":448,
                         "fill":[{"type":"solid","color":"#044A7D"}]},
                        {"type":"rectangle", "id":"panel-04", "x":1292, "y":96, "width":508, "height":840,
                         "fill":[{"type":"solid","color":"#EBE7DF"}]}
                    ]
                },
                {
                    "type": "frame", "id": "slide-07", "name": "07-问题与归因",
                    "width": 1920, "height": 1080,
                    "fill": [{"type":"solid", "color":"#F7F4EE"}],
                    "children": [
                        {"type":"rectangle", "id":"panel-07", "x":1200, "y":260, "width":600, "height":680,
                         "fill":[{"type":"solid","color":"#EBE7DF"}]}
                    ]
                }
            ]
        }))
    }
}
