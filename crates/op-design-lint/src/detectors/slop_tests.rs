//! Tests for `detectors::slop` — the three report-only "AI tells" rules.
//! Fixture style mirrors `siblings_tests.rs`: JSON values deserialized into
//! `PenNode` / `PenDocument`, one positive and one negative case per rule,
//! plus a `detect_all` integration check.

use super::slop::{
    detect_purple_glow_gradient, detect_rounded_card_wall, detect_three_card_feature_row,
};
use crate::issue::{FixProperty, IssueCategory, IssueSeverity};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::PenDocument;
use serde_json::json;

fn node(value: serde_json::Value) -> PenNode {
    serde_json::from_value(value).expect("fixture must deserialize as PenNode")
}

fn doc(value: serde_json::Value) -> PenDocument {
    serde_json::from_value(value).expect("fixture must deserialize as PenDocument")
}

/// A feature card: frame with icon + title + body text.
fn feature_card(id: &str) -> serde_json::Value {
    json!({
        "type": "frame", "id": id, "layout": "vertical",
        "children": [
            {"type": "icon_font", "id": format!("{id}-icon"), "iconFontName": "star"},
            {"type": "text", "id": format!("{id}-title"), "content": "Title"},
            {"type": "text", "id": format!("{id}-body"), "content": "Body copy"}
        ]
    })
}

/// A painted rounded card for the card-wall fixtures.
fn rounded_card(id: &str, width: f64, height: f64) -> serde_json::Value {
    json!({
        "type": "frame", "id": id, "width": width, "height": height,
        "cornerRadius": 20,
        "fill": [{"type": "solid", "color": "#FFFFFF"}],
        "children": [{"type": "text", "id": format!("{id}-t"), "content": "Card"}]
    })
}

#[cfg(test)]
mod purple_glow_gradient_tests {
    use super::*;

    /// #8B5CF6 → #7C3AED are both violet (hue ≈ 258° / 262°, sat ≈ 0.9);
    /// a 400x200 wash on a 400x800 board covers exactly the 25% threshold.
    #[test]
    fn flags_purple_gradient_wash_covering_quarter_of_board() {
        let root = node(json!({
            "type": "frame", "id": "board", "width": 400, "height": 800,
            "fill": [{"type": "solid", "color": "#FFFFFF"}],
            "children": [
                {
                    "type": "rectangle", "id": "wash", "width": 400, "height": 200,
                    "fill": [{
                        "type": "linear_gradient", "angle": 135,
                        "stops": [
                            {"offset": 0, "color": "#8B5CF6"},
                            {"offset": 1, "color": "#7C3AED"}
                        ]
                    }]
                }
            ]
        }));
        let issues = detect_purple_glow_gradient(&root, &doc(json!({"version": "1.0"})));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "wash");
        assert_eq!(issues[0].category, IssueCategory::SlopPurpleGlowGradient);
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
        assert_eq!(issues[0].property, FixProperty::Fill);
        assert_eq!(issues[0].suggested_value, json!(null));
        assert_eq!(issues[0].reason, "purple-blue gradient wash covering 25% of the board — the AI-default look; use the style guide's accent on one element instead");
        // The category's wire code keeps the `slop/` prefix.
        assert_eq!(
            serde_json::to_value(issues[0].category).unwrap(),
            json!("slop/purple-glow-gradient")
        );
    }

    /// A blue → cyan gradient is NOT the AI purple wash: blue hue ≈ 217° and
    /// cyan ≈ 187° both sit below the 235° window.
    #[test]
    fn ignores_blue_cyan_gradient() {
        let root = node(json!({
            "type": "frame", "id": "board", "width": 400, "height": 800,
            "children": [
                {
                    "type": "frame", "id": "hero", "width": 400, "height": 400,
                    "fill": [{
                        "type": "linear_gradient", "angle": 180,
                        "stops": [
                            {"offset": 0, "color": "#3B82F6"},
                            {"offset": 1, "color": "#22D3EE"}
                        ]
                    }]
                }
            ]
        }));
        assert!(detect_purple_glow_gradient(&root, &doc(json!({"version": "1.0"}))).is_empty());
    }

    /// Same purple wash but below the 25% coverage gate — a small accent is
    /// legitimate use, not slop.
    #[test]
    fn ignores_small_purple_accent() {
        let root = node(json!({
            "type": "frame", "id": "board", "width": 400, "height": 800,
            "children": [
                {
                    "type": "rectangle", "id": "chip", "width": 80, "height": 40,
                    "fill": [{
                        "type": "radial_gradient",
                        "stops": [
                            {"offset": 0, "color": "#8B5CF6"},
                            {"offset": 1, "color": "#7C3AED"}
                        ]
                    }]
                }
            ]
        }));
        assert!(detect_purple_glow_gradient(&root, &doc(json!({"version": "1.0"}))).is_empty());
    }

    /// A wash whose stops are unresolvable `$--var` refs is skipped entirely
    /// (fewer than 2 evaluable stops).
    #[test]
    fn skips_unresolvable_variable_stops() {
        let root = node(json!({
            "type": "frame", "id": "board", "width": 400, "height": 800,
            "children": [
                {
                    "type": "rectangle", "id": "wash", "width": 400, "height": 400,
                    "fill": [{
                        "type": "linear_gradient",
                        "stops": [
                            {"offset": 0, "color": "$--brand-a"},
                            {"offset": 1, "color": "$--brand-b"}
                        ]
                    }]
                }
            ]
        }));
        assert!(detect_purple_glow_gradient(&root, &doc(json!({"version": "1.0"}))).is_empty());
    }
}

#[cfg(test)]
mod three_card_feature_row_tests {
    use super::*;

    /// Three structurally identical icon + title + body cards in a horizontal
    /// row → one warning on the row.
    #[test]
    fn flags_identical_three_card_row() {
        let root = node(json!({
            "type": "frame", "id": "page", "layout": "vertical",
            "children": [
                {
                    "type": "frame", "id": "features", "layout": "horizontal",
                    "children": [
                        feature_card("a"),
                        feature_card("b"),
                        feature_card("c")
                    ]
                }
            ]
        }));
        let issues = detect_three_card_feature_row(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "features");
        assert_eq!(issues[0].category, IssueCategory::SlopThreeCardFeatureRow);
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
        assert_eq!(issues[0].property, FixProperty::Layout);
        assert_eq!(issues[0].suggested_value, json!(null));
        assert_eq!(
            issues[0].reason,
            "generic three-card feature row; vary weight (one lead card + two supporting) or use a list"
        );
    }

    /// Three cards whose structures differ (one drops the body text) do not
    /// share a signature → not the generic row.
    #[test]
    fn ignores_structurally_different_cards() {
        let root = node(json!({
            "type": "frame", "id": "page", "layout": "vertical",
            "children": [
                {
                    "type": "frame", "id": "features", "layout": "horizontal",
                    "children": [
                        feature_card("a"),
                        feature_card("b"),
                        {
                            "type": "frame", "id": "c", "layout": "vertical",
                            "children": [
                                {"type": "icon_font", "id": "c-icon", "iconFontName": "star"},
                                {"type": "text", "id": "c-title", "content": "Title"}
                            ]
                        }
                    ]
                }
            ]
        }));
        assert!(detect_three_card_feature_row(&root).is_empty());
    }

    /// A row of FOUR identical feature cards is not the three-card tell.
    #[test]
    fn ignores_four_card_row() {
        let root = node(json!({
            "type": "frame", "id": "page", "layout": "vertical",
            "children": [
                {
                    "type": "frame", "id": "features", "layout": "horizontal",
                    "children": [
                        feature_card("a"),
                        feature_card("b"),
                        feature_card("c"),
                        feature_card("d")
                    ]
                }
            ]
        }));
        assert!(detect_three_card_feature_row(&root).is_empty());
    }

    /// The icon + label row inside a bottom tab bar is chrome, not slop.
    #[test]
    fn ignores_row_inside_bottom_tab_bar() {
        let root = node(json!({
            "type": "frame", "id": "page", "layout": "vertical",
            "children": [
                {
                    "type": "frame", "id": "tabs", "role": "bottom-tab-bar", "layout": "horizontal",
                    "children": [
                        feature_card("a"),
                        feature_card("b"),
                        feature_card("c")
                    ]
                }
            ]
        }));
        assert!(detect_three_card_feature_row(&root).is_empty());
    }
}

#[cfg(test)]
mod rounded_card_wall_tests {
    use super::*;

    /// Six 360x120 cards on a 400x800 screen: 6 cards >= 6 and their summed
    /// area (259200) covers 81% of the root (320000) >= 60% → one issue on
    /// the root.
    #[test]
    fn flags_wall_of_six_rounded_cards() {
        let root = node(json!({
            "type": "frame", "id": "screen", "width": 400, "height": 800,
            "fill": [{"type": "solid", "color": "#F5F5F5"}],
            "children": [
                rounded_card("c1", 360.0, 120.0),
                rounded_card("c2", 360.0, 120.0),
                rounded_card("c3", 360.0, 120.0),
                rounded_card("c4", 360.0, 120.0),
                rounded_card("c5", 360.0, 120.0),
                rounded_card("c6", 360.0, 120.0)
            ]
        }));
        let issues = detect_rounded_card_wall(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "screen");
        assert_eq!(issues[0].category, IssueCategory::SlopRoundedCardWall);
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
        assert_eq!(issues[0].property, FixProperty::CornerRadius);
        assert_eq!(issues[0].suggested_value, json!(null));
        assert_eq!(
            issues[0].reason,
            "6 rounded cards cover 81% of the screen; let some content sit on the page surface"
        );
    }

    /// Five cards stay below the count gate — not a wall.
    #[test]
    fn ignores_five_cards() {
        let root = node(json!({
            "type": "frame", "id": "screen", "width": 400, "height": 800,
            "children": [
                rounded_card("c1", 360.0, 120.0),
                rounded_card("c2", 360.0, 120.0),
                rounded_card("c3", 360.0, 120.0),
                rounded_card("c4", 360.0, 120.0),
                rounded_card("c5", 360.0, 120.0)
            ]
        }));
        assert!(detect_rounded_card_wall(&root).is_empty());
    }

    /// Six square-corner panels (radius 8 < 16) are surfaces, not cards.
    #[test]
    fn ignores_sharp_corner_panels() {
        let panel = |id: &str| {
            json!({
                "type": "frame", "id": id, "width": 360, "height": 120,
                "cornerRadius": 8,
                "fill": [{"type": "solid", "color": "#FFFFFF"}]
            })
        };
        let root = node(json!({
            "type": "frame", "id": "screen", "width": 400, "height": 800,
            "children": [
                panel("p1"), panel("p2"), panel("p3"),
                panel("p4"), panel("p5"), panel("p6")
            ]
        }));
        assert!(detect_rounded_card_wall(&root).is_empty());
    }
}

#[cfg(test)]
mod detect_all_tests {
    use super::*;

    /// `detect_all` runs the slop detectors after the existing ones: a
    /// document carrying a purple wash and a three-card row surfaces both as
    /// Warning-severity slop issues.
    #[test]
    fn detect_all_includes_slop_issues_as_warnings() {
        let root = node(json!({
            "type": "frame", "id": "board", "width": 400, "height": 800,
            "fill": [{"type": "solid", "color": "#FFFFFF"}],
            "children": [
                {
                    "type": "rectangle", "id": "wash", "width": 400, "height": 300,
                    "fill": [{
                        "type": "linear_gradient",
                        "stops": [
                            {"offset": 0, "color": "#8B5CF6"},
                            {"offset": 1, "color": "#7C3AED"}
                        ]
                    }]
                },
                {
                    "type": "frame", "id": "features", "layout": "horizontal",
                    "fill": [{"type": "solid", "color": "#FAFAFA"}],
                    "children": [
                        feature_card("a"),
                        feature_card("b"),
                        feature_card("c")
                    ]
                }
            ]
        }));
        let issues =
            super::super::detect_all(&root, &doc(json!({"version": "1.0", "children": []})));
        let slop: Vec<&crate::issue::Issue> = issues
            .iter()
            .filter(|issue| {
                matches!(
                    issue.category,
                    IssueCategory::SlopPurpleGlowGradient
                        | IssueCategory::SlopThreeCardFeatureRow
                        | IssueCategory::SlopRoundedCardWall
                )
            })
            .collect();
        assert_eq!(
            slop.len(),
            2,
            "expected the wash and the row, got {issues:?}"
        );
        assert!(slop
            .iter()
            .all(|issue| issue.severity == IssueSeverity::Warning));
        assert!(slop.iter().all(|issue| issue.suggested_value.is_null()));
        assert!(slop.iter().any(
            |issue| issue.category == IssueCategory::SlopPurpleGlowGradient
                && issue.node_id == "wash"
        ));
        assert!(slop.iter().any(
            |issue| issue.category == IssueCategory::SlopThreeCardFeatureRow
                && issue.node_id == "features"
        ));
    }
}
