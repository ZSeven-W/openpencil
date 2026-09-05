//! Mobile-page spacing detectors ported from
//! `pen-ai-skills/src/diagnostics/detectors-spacing.ts`.
//!
//! Two aesthetic detectors share a "mobile-page shape" filter — a `frame`
//! whose width is 320–480 and whose numeric height is phone-like, or whose
//! `fit_content` height carries explicit screen structure, with a `layout`
//! set and `>= 2` children:
//! - `detect_edge_section_padding` — root has 0 h-padding and one or more
//!   direct content sections glue text / icons to the screen edge.
//! - `detect_stacked_horizontal_padding` — root and a content section both
//!   carry h-padding, producing a doubled gutter.

use jian_ops_schema::node::{container::LayoutMode, Padding, PenNode};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use serde_json::Value;

use crate::issue::{FixProperty, Issue, IssueCategory, IssueSeverity};
use crate::node_util::{
    children, fmt_num, has_stroke, is_mobile_screen_chrome, json_number, node_fills, node_id,
    node_kind, padding, raw_padding_value, role, NodeKind,
};

const DEFAULT_MOBILE_SECTION_RAIL: f64 = 24.0;
const MIN_MOBILE_SECTION_RAIL: f64 = 16.0;
const MAX_MOBILE_SECTION_RAIL: f64 = 28.0;

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
    "bottom-tab-bar",
    "bottom-navigation-bar",
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

fn get_padding_top(node: &PenNode) -> f64 {
    match padding(node) {
        Some(Padding::LtrB(p)) => p[0],
        Some(Padding::XY([v, _])) | Some(Padding::Uniform(v)) => *v,
        Some(Padding::Expression(_)) | None => 0.0,
    }
}

fn get_padding_bottom(node: &PenNode) -> f64 {
    match padding(node) {
        Some(Padding::LtrB(p)) => p[2],
        Some(Padding::XY([v, _])) | Some(Padding::Uniform(v)) => *v,
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

fn is_full_bleed_section(node: &PenNode, root_width: f64) -> bool {
    let child_role = role(node).unwrap_or("").to_lowercase();
    FULL_BLEED_ROLES.contains(&child_role.as_str())
        || is_image_only_section(node)
        || has_transparent_full_bleed_media_child(node, root_width)
}

fn has_transparent_full_bleed_media_child(node: &PenNode, root_width: f64) -> bool {
    is_transparent_container(node)
        && children(node).iter().any(|child| {
            let child_role = role(child).unwrap_or("").trim().to_ascii_lowercase();
            let role_is_media = matches!(
                child_role.as_str(),
                "hero" | "banner" | "cover" | "media" | "image-placeholder"
            );
            let image_like = matches!(node_kind(child), NodeKind::Image)
                || serde_json::to_value(child)
                    .ok()
                    .and_then(|value| value.get("fill").cloned())
                    .and_then(|fill| fill.as_array().cloned())
                    .is_some_and(|fills| {
                        fills
                            .iter()
                            .any(|fill| fill.get("type").and_then(Value::as_str) == Some("image"))
                    });
            (role_is_media || image_like) && node_spans_width(child, root_width)
        })
}

fn node_spans_width(node: &PenNode, root_width: f64) -> bool {
    node_width(node).is_some_and(|width| {
        matches!(width, SizingBehavior::Keyword(SizingKeyword::FillContainer))
            || matches!(width, SizingBehavior::Number(width) if *width >= root_width - 1.0)
    })
}

fn node_width(node: &PenNode) -> Option<&SizingBehavior> {
    match node {
        PenNode::Frame(node) => node.container.width.as_ref(),
        PenNode::Group(node) => node.container.width.as_ref(),
        PenNode::Rectangle(node) => node.container.width.as_ref(),
        PenNode::Image(node) => node.width.as_ref(),
        _ => None,
    }
}

fn numeric_width(node: &PenNode) -> Option<f64> {
    match node_width(node) {
        Some(SizingBehavior::Number(width)) => Some(*width),
        _ => None,
    }
}

fn has_concrete_zero_horizontal_padding(node: &PenNode) -> bool {
    !matches!(padding(node), Some(Padding::Expression(_)))
        && get_padding_left(node) == 0.0
        && get_padding_right(node) == 0.0
}

fn is_intentional_horizontal_scroller(node: &PenNode) -> bool {
    match node {
        PenNode::Frame(node) => {
            node.container.layout == Some(LayoutMode::Horizontal)
                && node.container.clip_content == Some(true)
        }
        PenNode::Group(node) => {
            node.container.layout == Some(LayoutMode::Horizontal)
                && node.container.clip_content == Some(true)
        }
        PenNode::Rectangle(node) => {
            node.container.layout == Some(LayoutMode::Horizontal)
                && node.container.clip_content == Some(true)
        }
        _ => false,
    }
}

fn contains_intentional_horizontal_scroller(node: &PenNode) -> bool {
    is_intentional_horizontal_scroller(node)
        || children(node)
            .iter()
            .any(contains_intentional_horizontal_scroller)
}

fn is_transparent_container(node: &PenNode) -> bool {
    if !matches!(
        node_kind(node),
        NodeKind::Frame | NodeKind::Group | NodeKind::Rectangle
    ) || node_fills(node).is_some_and(|fills| !fills.is_empty())
        || has_stroke(node)
    {
        return false;
    }
    serde_json::to_value(node).ok().is_some_and(|value| {
        let has_effects = value
            .get("effects")
            .and_then(Value::as_array)
            .is_some_and(|effects| !effects.is_empty());
        let has_radius = value
            .get("cornerRadius")
            .and_then(Value::as_f64)
            .is_some_and(|radius| radius > 0.0);
        !has_effects && !has_radius
    })
}

fn infer_mobile_section_rail(kids: &[PenNode], root_width: f64) -> f64 {
    let candidates: Vec<f64> = kids
        .iter()
        .filter(|child| matches!(node_kind(child), NodeKind::Frame))
        .filter(|child| !is_full_bleed_section(child, root_width))
        .filter_map(|child| {
            let left = get_padding_left(child);
            let right = get_padding_right(child);
            let rail = (left + right) / 2.0;
            ((left - right).abs() <= 0.5
                && (MIN_MOBILE_SECTION_RAIL..=MAX_MOBILE_SECTION_RAIL).contains(&rail))
            .then_some(rail)
        })
        .collect();

    let mut best = DEFAULT_MOBILE_SECTION_RAIL;
    let mut best_count = 0usize;
    let mut best_distance = f64::INFINITY;
    for candidate in &candidates {
        let count = candidates
            .iter()
            .filter(|other| (**other - *candidate).abs() <= 0.5)
            .count();
        let distance = (*candidate - DEFAULT_MOBILE_SECTION_RAIL).abs();
        if count > best_count || (count == best_count && distance < best_distance) {
            best = *candidate;
            best_count = count;
            best_distance = distance;
        }
    }
    best
}

fn suggested_symmetric_padding(node: &PenNode, rail: f64) -> Value {
    Value::Array(vec![
        json_number(get_padding_top(node)),
        json_number(rail),
        json_number(get_padding_bottom(node)),
        json_number(rail),
    ])
}

fn suggested_leading_padding(node: &PenNode, rail: f64) -> Value {
    Value::Array(vec![
        json_number(get_padding_top(node)),
        json_number(0.0),
        json_number(get_padding_bottom(node)),
        json_number(rail),
    ])
}

fn push_edge_section_issue(node: &PenNode, rail: f64, reason: String, issues: &mut Vec<Issue>) {
    issues.push(Issue {
        node_id: node_id(node).to_string(),
        category: IssueCategory::EdgeSectionPadding,
        severity: IssueSeverity::Warning,
        property: FixProperty::Padding,
        current_value: raw_padding_value(padding(node)),
        suggested_value: suggested_symmetric_padding(node, rail),
        reason,
    });
}

fn push_scroller_leading_issue(node: &PenNode, rail: f64, issues: &mut Vec<Issue>) {
    issues.push(Issue {
        node_id: node_id(node).to_string(),
        category: IssueCategory::EdgeSectionPadding,
        severity: IssueSeverity::Warning,
        property: FixProperty::Padding,
        current_value: raw_padding_value(padding(node)),
        suggested_value: suggested_leading_padding(node, rail),
        reason: format!(
            "mobile scroller viewport has 0 leading padding; apply a {}px leading rail while keeping the trailing edge flush",
            fmt_num(rail)
        ),
    });
}

/// True when a node looks like a real mobile device frame — the shared shape
/// filter for both detectors (`detectors-spacing.ts:117-124` / `230-237`).
///
/// Numeric roots must be at least 568px tall and 1.5× their width. A final
/// `fit_content` root also counts when it has screen structure (four or more
/// direct sections, or explicit status/bottom chrome); generation intentionally
/// converts ordinary completed mobile pages from a numeric construction seed
/// to this hugging height.
fn looks_like_mobile_page(node: &PenNode, depth: usize) -> bool {
    let PenNode::Frame(frame) = node else {
        return false;
    };
    let Some(SizingBehavior::Number(width)) = frame.container.width else {
        return false;
    };
    if !(320.0..=480.0).contains(&width) {
        return false;
    }
    match frame.container.height.as_ref() {
        Some(SizingBehavior::Number(height)) => *height >= 568.0 && *height >= width * 1.5,
        None | Some(SizingBehavior::Keyword(SizingKeyword::FitContent)) => {
            let kids = children(node);
            (depth == 0 && kids.len() >= 4) || kids.iter().any(is_mobile_screen_chrome)
        }
        _ => false,
    }
}

/// Port of `detectEdgeSectionPadding` (`detectors-spacing.ts:100-197`).
///
/// Detects the "Categories no padding" bug — a mobile page whose root owns no
/// horizontal gutter and whose direct content sections inconsistently apply
/// their own page rail. Each repairable unpadded content section is reported
/// separately, so one correctly padded sibling no longer suppresses repairs
/// for the rest of the page.
///
/// False-positive guards: mobile-shaped root only; skip child sections with
/// full-bleed roles (`FULL_BLEED_ROLES`); skip image-only sections; only flag
/// sections that carry text / icon descendants.
///
/// The symmetric rail is inferred from direct padded content siblings when
/// they agree on a value in the 16–28px mobile range; otherwise the current
/// 24px mobile rail policy is used. Existing vertical padding is preserved.
///
/// A section that directly owns an intentional clipped horizontal scroller
/// keeps its full-width section geometry. Transparent direct header siblings
/// get a symmetric rail, while the clipped viewport gets a leading-only rail
/// so its trailing edge remains flush.
pub fn detect_edge_section_padding(root: &PenNode) -> Vec<Issue> {
    let mut issues = Vec::new();
    walk_edge_section_padding(root, 0, &mut issues);
    issues
}

fn walk_edge_section_padding(node: &PenNode, depth: usize, issues: &mut Vec<Issue>) {
    let kids = children(node);
    // A mobile root: page-shaped, `layout` set, `>= 2` children.
    let is_mobile_root = looks_like_mobile_page(node, depth) && has_layout(node) && kids.len() >= 2;

    if is_mobile_root && has_concrete_zero_horizontal_padding(node) {
        let root_width = numeric_width(node).unwrap_or_default();
        let rail = infer_mobile_section_rail(kids, root_width);
        for child in kids {
            if !matches!(node_kind(child), NodeKind::Frame) {
                continue;
            }

            // Horizontal viewports own an asymmetric rail even when their
            // contents are image-only and therefore have no text/icon nodes.
            if is_intentional_horizontal_scroller(child) {
                if has_concrete_zero_horizontal_padding(child) {
                    push_scroller_leading_issue(child, rail, issues);
                }
                continue;
            }

            if is_full_bleed_section(child, root_width) {
                continue;
            }
            // Padding a surfaced root-direct card changes its interior but
            // leaves the card border glued to the viewport. Without an outer
            // wrapper there is no safe automatic inset, so report/fix only
            // transparent section owners here; whole-doc cleanup can repair
            // generated wrapper structure first.
            if !is_transparent_container(child) {
                continue;
            }
            if !has_concrete_zero_horizontal_padding(child) {
                continue;
            }
            let direct_scroller = children(child)
                .iter()
                .any(is_intentional_horizontal_scroller);
            if direct_scroller {
                for header in children(child).iter().filter(|header| {
                    !is_intentional_horizontal_scroller(header)
                        && is_transparent_container(header)
                        && has_concrete_zero_horizontal_padding(header)
                        && has_text_or_icon_descendant(header)
                }) {
                    push_edge_section_issue(
                        header,
                        rail,
                        format!(
                            "mobile scroller header has 0 horizontal padding; apply a {}px page rail without changing clipped scroll geometry",
                            fmt_num(rail)
                        ),
                        issues,
                    );
                }
                for viewport in children(child)
                    .iter()
                    .filter(|viewport| is_intentional_horizontal_scroller(viewport))
                {
                    if has_concrete_zero_horizontal_padding(viewport) {
                        push_scroller_leading_issue(viewport, rail, issues);
                    }
                }
                continue;
            }
            if contains_intentional_horizontal_scroller(child) {
                continue;
            }
            if !has_text_or_icon_descendant(child) {
                continue;
            }

            push_edge_section_issue(
                child,
                rail,
                format!(
                    "mobile content section has 0 horizontal padding; apply a {}px symmetric page rail",
                    fmt_num(rail)
                ),
                issues,
            );
        }
    }

    for child in kids {
        walk_edge_section_padding(child, depth + 1, issues);
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
    walk_stacked_horizontal_padding(root, 0, &mut issues);
    issues
}

fn walk_stacked_horizontal_padding(node: &PenNode, depth: usize, issues: &mut Vec<Issue>) {
    let kids = children(node);
    if looks_like_mobile_page(node, depth) && kids.len() >= 2 {
        let root_l = get_padding_left(node);
        let root_r = get_padding_right(node);
        let root_width = numeric_width(node).unwrap_or_default();
        if root_l > 0.0 || root_r > 0.0 {
            for child in kids {
                if !matches!(node_kind(child), NodeKind::Frame) {
                    continue;
                }
                if is_full_bleed_section(child, root_width) {
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
        walk_stacked_horizontal_padding(child, depth + 1, issues);
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
