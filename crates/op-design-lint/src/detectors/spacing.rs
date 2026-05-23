//! Mobile-page spacing detectors ported from
//! `pen-ai-skills/src/diagnostics/detectors-spacing.ts`.
//!
//! Two aesthetic detectors share a "mobile-page shape" filter — a `frame`
//! whose width is 320–480, whose height is `>= 568` and `>= width * 1.5`,
//! with a `layout` set and `>= 2` children:
//! - `detect_edge_section_padding` — root has 0 h-padding and a content
//!   section glues text / icons to the screen edge.
//! - `detect_stacked_horizontal_padding` — root and a content section both
//!   carry h-padding, producing a doubled gutter.

use jian_ops_schema::node::{Padding, PenNode};
use jian_ops_schema::sizing::SizingBehavior;
use serde_json::Value;

use crate::issue::{FixProperty, Issue, IssueCategory, IssueSeverity};
use crate::node_util::{
    children, fmt_num, json_number, node_id, node_kind, padding, raw_padding_value, role, NodeKind,
};

/// Roles that legitimately span the full mobile viewport width and SHOULD
/// have 0 horizontal padding from the page root. Port of the TS
/// `FULL_BLEED_ROLES` set (`detectors-spacing.ts:10-21`). Flagging these
/// produces false positives the autofix would damage (forcing 16px gutters
/// on a top-nav inset breaks chrome).
const FULL_BLEED_ROLES: &[&str] = &[
    "hero",
    "banner",
    "cover",
    "header",
    "top-nav",
    "bottom-nav",
    "status-bar",
    "tab-bar",
    "tabbar",
    "navbar",
];

/// Port of `getPaddingLeft` (`detectors-spacing.ts:23-32`). Reads the left
/// inset off a node's `padding`.
///
/// jian `Padding` is the typed enum (`Uniform` / `XY` / `LtrB` / `Expression`).
/// The TS helper also accepts a 1-element padding array (`p.length === 1`);
/// jian's `Padding` enum has NO 1-element variant (T0 audit footnote 8), so
/// that TS branch has no jian equivalent and is intentionally dropped here.
fn get_padding_left(node: &PenNode) -> f64 {
    match padding(node) {
        // `LtrB` is `[t, r, b, l]` — left is index 3.
        Some(Padding::LtrB(p)) => p[3],
        // `XY` is `[v, h]` — left is the horizontal component (index 1).
        Some(Padding::XY([_, h])) => *h,
        Some(Padding::Uniform(v)) => *v,
        // `Expression` (a `$variable` ref) is the TS "anything else → 0" case.
        Some(Padding::Expression(_)) | None => 0.0,
    }
}

/// Port of `getPaddingRight` (`detectors-spacing.ts:34-43`). Reads the right
/// inset off a node's `padding`. Same enum mapping as [`get_padding_left`];
/// the 1-element TS branch is intentionally dropped (T0 audit footnote 8).
fn get_padding_right(node: &PenNode) -> f64 {
    match padding(node) {
        // `LtrB` is `[t, r, b, l]` — right is index 1.
        Some(Padding::LtrB(p)) => p[1],
        // `XY` is `[v, h]` — right is the horizontal component (index 1).
        Some(Padding::XY([_, h])) => *h,
        Some(Padding::Uniform(v)) => *v,
        Some(Padding::Expression(_)) | None => 0.0,
    }
}

/// Port of `hasTextOrIconDescendant` (`detectors-spacing.ts:45-54`). True when
/// `node` is — or recursively contains — a `text` or `icon_font` node.
fn has_text_or_icon_descendant(node: &PenNode) -> bool {
    if matches!(node_kind(node), NodeKind::Text | NodeKind::IconFont) {
        return true;
    }
    children(node).iter().any(has_text_or_icon_descendant)
}

/// Port of `isImageOnlySection` (`detectors-spacing.ts:62-72`). True when a
/// node has `>= 1` child and every child is either an `image` node or a node
/// with role `image-placeholder` — a deliberately full-bleed media tile.
fn is_image_only_section(node: &PenNode) -> bool {
    let kids = children(node);
    if kids.is_empty() {
        return false;
    }
    kids.iter().all(|c| {
        if matches!(node_kind(c), NodeKind::Image) {
            return true;
        }
        role(c).unwrap_or("").to_lowercase() == "image-placeholder"
    })
}

/// True when a node looks like a real mobile device frame — the shared shape
/// filter for both detectors (`detectors-spacing.ts:117-124` / `230-237`).
///
/// `frame` kind, width 320–480, height `>= 568` (iPhone SE 1st gen, the
/// shortest production phone) and `>= width * 1.5` (aspect-ratio guard
/// against accidentally tall components like a side drawer). Width / height
/// are jian `SizingBehavior`; only the `Number` variant counts — a
/// `fit_content` / `fill_container` / `$ref` size is correctly NOT a
/// mobile-page dimension (T0 audit DS⁴).
fn looks_like_mobile_page(node: &PenNode) -> bool {
    let PenNode::Frame(frame) = node else {
        return false;
    };
    let Some(SizingBehavior::Number(width)) = frame.container.width else {
        return false;
    };
    let Some(SizingBehavior::Number(height)) = frame.container.height else {
        return false;
    };
    (320.0..=480.0).contains(&width) && height >= 568.0 && height >= width * 1.5
}

/// Port of `detectEdgeSectionPadding` (`detectors-spacing.ts:100-197`).
///
/// Detects the "Categories no padding" bug — a mobile page where the root has
/// 0 left padding AND a content section also has 0 left padding AND that
/// section contains visible text / icon descendants, so content visually
/// butts against the viewport edge.
///
/// False-positive guards: mobile-shaped root only; skip child sections with
/// full-bleed roles (`FULL_BLEED_ROLES`); skip image-only sections; only flag
/// sections that carry text / icon descendants. The two-pass content-child
/// scan also tracks whether any sibling already carries per-section h-padding
/// — if so the design has opted out of root-level gutter and the detector
/// aborts entirely (the `hasPerSectionPaddedSibling` rule, added per 2026-05-11
/// user feedback that root padding stacked on top of per-section gutters).
///
/// Suggested fix: set `root.padding` to `[top, 16, bottom, 16]`, preserving
/// vertical padding — 16 is the iOS HIG / Material default mobile gutter.
pub fn detect_edge_section_padding(root: &PenNode) -> Vec<Issue> {
    let mut issues = Vec::new();
    walk_edge_section_padding(root, &mut issues);
    issues
}

fn walk_edge_section_padding(node: &PenNode, issues: &mut Vec<Issue>) {
    let kids = children(node);
    // A mobile root: page-shaped, `layout` set, `>= 2` children.
    let is_mobile_root = looks_like_mobile_page(node) && has_layout(node) && kids.len() >= 2;

    if is_mobile_root && get_padding_left(node) == 0.0 {
        // Two-pass scan over content children, skipping full-bleed chrome /
        // image-only banners which are intentionally edge-to-edge:
        //   - any with own h-padding > 0  → the design has chosen the
        //                                   per-section gutter pattern; abort.
        //   - any with no padding but text/icon descendants → genuinely glues
        //                                   content to the viewport edge; flag.
        let mut has_per_section_padded_sibling = false;
        let mut offending_children: usize = 0;
        for child in kids {
            if !matches!(node_kind(child), NodeKind::Frame) {
                continue;
            }
            let child_role = role(child).unwrap_or("").to_lowercase();
            if FULL_BLEED_ROLES.contains(&child_role.as_str()) {
                continue;
            }
            if is_image_only_section(child) {
                continue;
            }
            if get_padding_left(child) > 0.0 {
                has_per_section_padded_sibling = true;
                continue;
            }
            if !has_text_or_icon_descendant(child) {
                continue;
            }
            offending_children += 1;
        }
        if offending_children > 0 && !has_per_section_padded_sibling {
            let current_pad = padding(node);
            // Suggested padding — 4 cases mirroring the TS `currentPad` branch.
            // Every numeric element routes through `json_number` so an
            // integral inset serializes as a JSON integer, not a float.
            let suggested: Value = match current_pad {
                Some(Padding::LtrB(p)) => Value::Array(vec![
                    json_number(p[0]),
                    json_number(16.0),
                    json_number(p[2]),
                    json_number(16.0),
                ]),
                Some(Padding::XY([v, _])) => Value::Array(vec![
                    json_number(*v),
                    json_number(16.0),
                    json_number(*v),
                    json_number(16.0),
                ]),
                Some(Padding::Uniform(v)) => Value::Array(vec![
                    json_number(*v),
                    json_number(16.0),
                    json_number(*v),
                    json_number(16.0),
                ]),
                Some(Padding::Expression(_)) | None => Value::Array(vec![
                    json_number(0.0),
                    json_number(16.0),
                    json_number(0.0),
                    json_number(16.0),
                ]),
            };
            issues.push(Issue {
                node_id: node_id(node).to_string(),
                category: IssueCategory::EdgeSectionPadding,
                severity: IssueSeverity::Warning,
                property: FixProperty::Padding,
                current_value: raw_padding_value(current_pad),
                suggested_value: suggested,
                reason: format!(
                    "mobile root has 0 horizontal padding while {offending_children} content section(s) glue text/icon to screen edge"
                ),
            });
        }
    }

    for child in kids {
        walk_edge_section_padding(child, issues);
    }
}

/// True when a node carries a non-null `layout` (`ContainerProps.layout`).
/// Only Frame / Group / Rectangle declare `layout`; every other kind is
/// `false`, matching the TS `'layout' in node && node.layout != null` guard.
fn has_layout(node: &PenNode) -> bool {
    match node {
        PenNode::Frame(n) => n.container.layout.is_some(),
        PenNode::Group(n) => n.container.layout.is_some(),
        PenNode::Rectangle(n) => n.container.layout.is_some(),
        _ => false,
    }
}

/// Port of `detectStackedHorizontalPadding` (`detectors-spacing.ts:222-274`).
///
/// Detects a page root and a direct content section that BOTH apply
/// horizontal padding, producing a doubled inset (e.g. root `[0,16,0,16]` +
/// section `[0,24,0,24]` → 40px effective gutter, leaving content pinched).
///
/// Shares the mobile-page shape filter with [`detect_edge_section_padding`]
/// to avoid firing on components-with-internal-padding. When the root has
/// h-padding `> 0`, every non-full-bleed `frame` child that also has h-padding
/// `> 0` is flagged (the child is the offender — the root is the established
/// gutter holder).
///
/// Severity is `info` (detect-only). An auto-fix is tempting but a section
/// may legitimately want a deeper inset for visual emphasis, so the issue is
/// surfaced for the user / agent to decide; `suggested_value` is `null`.
pub fn detect_stacked_horizontal_padding(root: &PenNode) -> Vec<Issue> {
    let mut issues = Vec::new();
    walk_stacked_horizontal_padding(root, &mut issues);
    issues
}

fn walk_stacked_horizontal_padding(node: &PenNode, issues: &mut Vec<Issue>) {
    let kids = children(node);
    if looks_like_mobile_page(node) && kids.len() >= 2 {
        let root_l = get_padding_left(node);
        let root_r = get_padding_right(node);
        if root_l > 0.0 || root_r > 0.0 {
            for child in kids {
                if !matches!(node_kind(child), NodeKind::Frame) {
                    continue;
                }
                let child_role = role(child).unwrap_or("").to_lowercase();
                if FULL_BLEED_ROLES.contains(&child_role.as_str()) {
                    continue;
                }
                let child_l = get_padding_left(child);
                let child_r = get_padding_right(child);
                if child_l == 0.0 && child_r == 0.0 {
                    continue;
                }
                issues.push(Issue {
                    node_id: node_id(child).to_string(),
                    category: IssueCategory::StackedHorizontalPadding,
                    severity: IssueSeverity::Info,
                    property: FixProperty::Padding,
                    current_value: raw_padding_value(padding(child)),
                    suggested_value: Value::Null,
                    reason: format!(
                        "section h-padding [{}/{}] stacks with root h-padding [{}/{}] — combined gutter {}/{}",
                        fmt_num(child_l),
                        fmt_num(child_r),
                        fmt_num(root_l),
                        fmt_num(root_r),
                        fmt_num(root_l + child_l),
                        fmt_num(root_r + child_r),
                    ),
                });
            }
        }
    }

    for child in kids {
        walk_stacked_horizontal_padding(child, issues);
    }
}

#[cfg(test)]
mod edge_section_padding_tests {
    use super::*;
    use serde_json::json;

    fn node(value: serde_json::Value) -> PenNode {
        serde_json::from_value(value).expect("fixture must deserialize as PenNode")
    }

    /// A mobile-shaped root with 0 left padding and a content child that has
    /// text and 0 padding → one `edge-section-padding` issue.
    #[test]
    fn flags_mobile_root_with_edge_glued_content() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "width": 393, "height": 852, "layout": "vertical",
            "children": [
                {
                    "type": "frame", "id": "s1", "role": "categories",
                    "children": [{"type": "text", "id": "t1", "content": "Categories"}]
                },
                {
                    "type": "frame", "id": "s2", "role": "list",
                    "children": [{"type": "text", "id": "t2", "content": "Item"}]
                }
            ]
        }));
        let issues = detect_edge_section_padding(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "root");
        assert_eq!(issues[0].category, IssueCategory::EdgeSectionPadding);
        assert_eq!(issues[0].property, FixProperty::Padding);
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
        // No prior padding → suggested `[0, 16, 0, 16]`.
        assert_eq!(issues[0].suggested_value, json!([0, 16, 0, 16]));
        assert_eq!(issues[0].current_value, json!(null));
    }

    /// A sibling already carrying per-section h-padding aborts the detector —
    /// the design has opted out of root-level gutter.
    #[test]
    fn aborts_when_a_sibling_has_per_section_padding() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "width": 393, "height": 852, "layout": "vertical",
            "children": [
                {
                    "type": "frame", "id": "s1", "role": "categories",
                    "children": [{"type": "text", "id": "t1", "content": "Categories"}]
                },
                {
                    "type": "frame", "id": "s2", "role": "list", "padding": [0, 24, 0, 24],
                    "children": [{"type": "text", "id": "t2", "content": "Item"}]
                }
            ]
        }));
        assert!(detect_edge_section_padding(&root).is_empty());
    }

    /// A child with a full-bleed role (`hero`) is skipped — only the
    /// non-full-bleed sibling with text/no-padding counts.
    #[test]
    fn skips_full_bleed_role_children() {
        // Only the hero child has text+no-padding; it is skipped → no issue.
        let root = node(json!({
            "type": "frame", "id": "root",
            "width": 393, "height": 852, "layout": "vertical",
            "children": [
                {
                    "type": "frame", "id": "hero", "role": "hero",
                    "children": [{"type": "text", "id": "t1", "content": "Welcome"}]
                },
                {
                    "type": "frame", "id": "banner", "role": "banner",
                    "children": [{"type": "text", "id": "t2", "content": "Sale"}]
                }
            ]
        }));
        assert!(detect_edge_section_padding(&root).is_empty());
    }

    /// An image-only child is skipped (intentional full-bleed media).
    #[test]
    fn skips_image_only_section() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "width": 393, "height": 852, "layout": "vertical",
            "children": [
                {
                    "type": "frame", "id": "media", "role": "gallery",
                    "children": [{"type": "image", "id": "img1", "src": "a.png"}]
                },
                {
                    "type": "frame", "id": "media2", "role": "gallery",
                    "children": [{"type": "image", "id": "img2", "src": "b.png"}]
                }
            ]
        }));
        assert!(detect_edge_section_padding(&root).is_empty());
    }

    /// A non-mobile-shaped frame (393×200 — a card / modal) is never flagged.
    #[test]
    fn never_flags_non_mobile_shaped_frame() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "width": 393, "height": 200, "layout": "vertical",
            "children": [
                {
                    "type": "frame", "id": "s1", "role": "categories",
                    "children": [{"type": "text", "id": "t1", "content": "Categories"}]
                },
                {
                    "type": "frame", "id": "s2", "role": "list",
                    "children": [{"type": "text", "id": "t2", "content": "Item"}]
                }
            ]
        }));
        assert!(detect_edge_section_padding(&root).is_empty());
    }

    /// A child with no text / icon descendants is not an offender.
    #[test]
    fn ignores_section_without_text_or_icon() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "width": 393, "height": 852, "layout": "vertical",
            "children": [
                {
                    "type": "frame", "id": "s1", "role": "divider-area",
                    "children": [{"type": "rectangle", "id": "r1"}]
                },
                {
                    "type": "frame", "id": "s2", "role": "divider-area",
                    "children": [{"type": "rectangle", "id": "r2"}]
                }
            ]
        }));
        assert!(detect_edge_section_padding(&root).is_empty());
    }

    /// An existing root padding is preserved on the vertical axis when the
    /// suggested `[top, 16, bottom, 16]` is built.
    #[test]
    fn preserves_vertical_padding_in_suggestion() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "width": 393, "height": 852, "layout": "vertical",
            "padding": [12, 0, 12, 0],
            "children": [
                {
                    "type": "frame", "id": "s1", "role": "categories",
                    "children": [{"type": "text", "id": "t1", "content": "Categories"}]
                },
                {
                    "type": "frame", "id": "s2", "role": "list",
                    "children": [{"type": "text", "id": "t2", "content": "Item"}]
                }
            ]
        }));
        let issues = detect_edge_section_padding(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].suggested_value, json!([12, 16, 12, 16]));
        assert_eq!(issues[0].current_value, json!([12, 0, 12, 0]));
    }

    /// An icon_font descendant (no text) still counts as content.
    #[test]
    fn flags_section_with_icon_descendant() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "width": 393, "height": 852, "layout": "vertical",
            "children": [
                {
                    "type": "frame", "id": "s1", "role": "toolbar-area",
                    "children": [{"type": "icon_font", "id": "i1", "iconFontName": "star"}]
                },
                {
                    "type": "frame", "id": "s2", "role": "toolbar-area",
                    "children": [{"type": "icon_font", "id": "i2", "iconFontName": "heart"}]
                }
            ]
        }));
        let issues = detect_edge_section_padding(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "root");
    }
}

#[cfg(test)]
mod stacked_horizontal_padding_tests {
    use super::*;
    use serde_json::json;

    fn node(value: serde_json::Value) -> PenNode {
        serde_json::from_value(value).expect("fixture must deserialize as PenNode")
    }

    /// A mobile root with h-padding and a content child with its own
    /// h-padding → the child is flagged with `info` severity.
    #[test]
    fn flags_section_h_padding_stacked_on_root() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "width": 375, "height": 812, "layout": "vertical",
            "padding": [0, 16, 0, 16],
            "children": [
                {
                    "type": "frame", "id": "specials", "role": "section",
                    "padding": [0, 24, 0, 24],
                    "children": [{"type": "text", "id": "t1", "content": "Today's Specials"}]
                },
                {
                    "type": "frame", "id": "plain", "role": "section",
                    "children": [{"type": "text", "id": "t2", "content": "Menu"}]
                }
            ]
        }));
        let issues = detect_stacked_horizontal_padding(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "specials");
        assert_eq!(issues[0].category, IssueCategory::StackedHorizontalPadding);
        assert_eq!(issues[0].property, FixProperty::Padding);
        assert_eq!(issues[0].severity, IssueSeverity::Info);
        assert_eq!(issues[0].suggested_value, json!(null));
        // `current_value` preserves the section's existing padding as integers.
        assert_eq!(issues[0].current_value, json!([0, 24, 0, 24]));
    }

    /// A full-bleed (`hero`) child is not flagged even with its own padding.
    #[test]
    fn skips_full_bleed_role_child() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "width": 375, "height": 812, "layout": "vertical",
            "padding": [0, 16, 0, 16],
            "children": [
                {
                    "type": "frame", "id": "hero", "role": "hero",
                    "padding": [0, 24, 0, 24],
                    "children": [{"type": "text", "id": "t1", "content": "Welcome"}]
                },
                {
                    "type": "frame", "id": "plain", "role": "section",
                    "children": [{"type": "text", "id": "t2", "content": "Menu"}]
                }
            ]
        }));
        assert!(detect_stacked_horizontal_padding(&root).is_empty());
    }

    /// A child with 0 horizontal padding is not flagged.
    #[test]
    fn ignores_child_without_horizontal_padding() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "width": 375, "height": 812, "layout": "vertical",
            "padding": [0, 16, 0, 16],
            "children": [
                {
                    "type": "frame", "id": "s1", "role": "section",
                    "children": [{"type": "text", "id": "t1", "content": "A"}]
                },
                {
                    "type": "frame", "id": "s2", "role": "section",
                    "children": [{"type": "text", "id": "t2", "content": "B"}]
                }
            ]
        }));
        assert!(detect_stacked_horizontal_padding(&root).is_empty());
    }

    /// A non-mobile-shaped root is never flagged.
    #[test]
    fn never_flags_non_mobile_root() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "width": 375, "height": 200, "layout": "vertical",
            "padding": [0, 16, 0, 16],
            "children": [
                {
                    "type": "frame", "id": "s1", "role": "section",
                    "padding": [0, 24, 0, 24],
                    "children": [{"type": "text", "id": "t1", "content": "A"}]
                },
                {
                    "type": "frame", "id": "s2", "role": "section",
                    "padding": [0, 24, 0, 24],
                    "children": [{"type": "text", "id": "t2", "content": "B"}]
                }
            ]
        }));
        assert!(detect_stacked_horizontal_padding(&root).is_empty());
    }

    /// A root with 0 h-padding never triggers the detector (nothing to stack).
    #[test]
    fn ignores_root_without_horizontal_padding() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "width": 375, "height": 812, "layout": "vertical",
            "children": [
                {
                    "type": "frame", "id": "s1", "role": "section",
                    "padding": [0, 24, 0, 24],
                    "children": [{"type": "text", "id": "t1", "content": "A"}]
                },
                {
                    "type": "frame", "id": "s2", "role": "section",
                    "padding": [0, 24, 0, 24],
                    "children": [{"type": "text", "id": "t2", "content": "B"}]
                }
            ]
        }));
        assert!(detect_stacked_horizontal_padding(&root).is_empty());
    }

    /// The combined-gutter math appears in the reason string.
    #[test]
    fn reason_reports_combined_gutter() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "width": 375, "height": 812, "layout": "vertical",
            "padding": [0, 16, 0, 16],
            "children": [
                {
                    "type": "frame", "id": "s1", "role": "section",
                    "padding": [0, 24, 0, 24],
                    "children": [{"type": "text", "id": "t1", "content": "A"}]
                },
                {
                    "type": "frame", "id": "s2", "role": "section",
                    "children": [{"type": "text", "id": "t2", "content": "B"}]
                }
            ]
        }));
        let issues = detect_stacked_horizontal_padding(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(
            issues[0].reason,
            "section h-padding [24/24] stacks with root h-padding [16/16] — combined gutter 40/40"
        );
    }
}
