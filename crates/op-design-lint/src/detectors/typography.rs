//! Typography detector ported from
//! `pen-ai-skills/src/diagnostics/detectors-typography.ts`.
//!
//! `detect_text_bg_contrast` walks the tree carrying the ancestor chain; for
//! each visible `text` node it resolves the text fill color and the nearest
//! ancestor solid background color (both through `resolve_color_ref`),
//! computes the WCAG contrast ratio, and flags text whose ratio falls below
//! a (deliberately looser-than-WCAG-AA) threshold.

use jian_ops_schema::node::{FontWeight, PenNode, TextNode};
use jian_ops_schema::style::PenFill;
use jian_ops_schema::PenDocument;
use serde_json::Value;

use crate::color::color_contrast;
use crate::issue::{FixProperty, Issue, IssueCategory, IssueSeverity};
use crate::node_util::{
    children, default_theme, is_node_visible, node_fills, node_id, resolve_color_ref, Theme,
    Variables,
};

/// Contrast ratio below which normal-size text is flagged. Port of the TS
/// `DEFAULT_NORMAL_THRESHOLD` (`detectors-typography.ts:26`). Looser than WCAG
/// 2.x AA on purpose — calibrated against the 2026-05-08 GPT-5.5 corpus to
/// report physical readability, not WCAG compliance.
const DEFAULT_NORMAL_THRESHOLD: f64 = 2.5;
/// Contrast ratio below which large text (`>=24px` or `>=19px` bold) is
/// flagged. Port of the TS `DEFAULT_LARGE_THRESHOLD` (`detectors-typography.ts:27`).
const DEFAULT_LARGE_THRESHOLD: f64 = 2.0;

/// Port of `detectTextBgContrast` (`detectors-typography.ts:123-213`).
///
/// Walks the tree carrying the ancestor chain. For each visible `text` node:
/// resolves its fill color and the nearest ancestor solid background color
/// (both through the document variable table + active theme), computes
/// `color_contrast`, and flags an `info`-severity `text-bg-contrast` issue
/// when the ratio is below the size-dependent threshold.
///
/// T0 audit blocker **B3**: the walk prunes on `!is_node_visible(node)` so a
/// `visible:false` / `enabled:false` subtree is skipped entirely — matching
/// the canonical render-time visibility check.
///
/// Severity is always `info` (detect-only): the right replacement color
/// depends on the design system + theme + intent, so `suggested_value` is
/// `Null` and the issue is surfaced for the user / agent to decide.
pub fn detect_text_bg_contrast(root: &PenNode, doc: &PenDocument) -> Vec<Issue> {
    let mut issues = Vec::new();
    let empty_vars = Variables::new();
    let variables = doc.variables.as_ref().unwrap_or(&empty_vars);
    let theme = default_theme(doc.themes.as_ref());
    walk(root, &[], variables, &theme, &mut issues);
    issues
}

/// Recursive walk carrying the ancestor chain (closest ancestor last).
fn walk<'a>(
    node: &'a PenNode,
    ancestors: &[&'a PenNode],
    variables: &Variables,
    theme: &Theme,
    issues: &mut Vec<Issue>,
) {
    // Align the prune with the canonical render-time visibility check
    // (pen-core `isNodeVisible`). A `visible:false` / `enabled:false` subtree
    // is never walked. opacity=0 is intentionally NOT a prune condition — the
    // renderer treats opacity as a paint alpha, not a skip flag; an
    // opacity=0 wrapper is still walked, and `ancestor_bg_color` skips its
    // fill so contrast is computed against the real bg behind it.
    if !is_node_visible(node) {
        return;
    }
    if let PenNode::Text(text) = node {
        check_text(node, text, ancestors, variables, theme, issues);
    }
    let mut next = ancestors.to_vec();
    next.push(node);
    for child in children(node) {
        walk(child, &next, variables, theme, issues);
    }
}

/// Resolve the nearest ancestor solid background color. Walks ancestors
/// closest-first, skipping `opacity:0` / `visible:false` wrappers and
/// fills with no usable solid color; the first resolvable solid color wins.
/// Defaults to `#FFFFFF` (the canvas bg) when the chain runs out.
fn ancestor_bg_color(ancestors: &[&PenNode], variables: &Variables, theme: &Theme) -> String {
    for ancestor in ancestors.iter().rev() {
        // A node-level opacity=0 / visible=false hides the WHOLE wrapper
        // including its fill — skip it so the real bg further up wins.
        if crate::node_util::opacity(ancestor) == 0.0 {
            continue;
        }
        if !is_node_visible(ancestor) {
            continue;
        }
        let Some(raw) = first_solid_color(node_fills(ancestor)) else {
            continue;
        };
        if let Some(resolved) = resolve_color_ref(&raw, variables, theme) {
            return resolved;
        }
    }
    "#FFFFFF".to_string()
}

/// Port of the `checkText` closure (`detectors-typography.ts:185-212`).
fn check_text(
    node: &PenNode,
    text: &TextNode,
    ancestors: &[&PenNode],
    variables: &Variables,
    theme: &Theme,
    issues: &mut Vec<Issue>,
) {
    let Some(raw_text) = first_solid_color(text.fill.as_ref()) else {
        return; // no fill or non-solid — renderer default applies, skip
    };
    let Some(text_color) = resolve_color_ref(&raw_text, variables, theme) else {
        return; // unresolvable ref
    };

    let bg_color = ancestor_bg_color(ancestors, variables, theme);
    let ratio = color_contrast(&text_color, &bg_color);
    if !ratio.is_finite() {
        return; // either color failed to parse
    }

    let threshold = if is_large_text(text) {
        DEFAULT_LARGE_THRESHOLD
    } else {
        DEFAULT_NORMAL_THRESHOLD
    };
    if ratio >= threshold {
        return;
    }

    issues.push(Issue {
        node_id: node_id(node).to_string(),
        category: IssueCategory::TextBgContrast,
        // Detect-only — auto-replacing a fill with a "nearest brand token"
        // is the path the 2026-05-09 review explicitly rejected.
        severity: IssueSeverity::Info,
        property: FixProperty::Fill,
        current_value: text_fill_value(text),
        // No suggested value: the right replacement depends on design system
        // + theme + intent, which only the user / agent can decide.
        suggested_value: Value::Null,
        reason: format!(
            "text/bg contrast {ratio:.2}:1 below {threshold}:1 (text={text_color} on bg={bg_color})"
        ),
    });
}

/// Port of `firstSolidColor` (`detectors-typography.ts:54-81`). Pulls the
/// first usable color out of a fill list: skips effectively-transparent fills
/// (`opacity == 0`, or a 9-char `#RRGGBBAA` with alpha `00`), returns the
/// first solid color, or the first stop of the first gradient. `None` when
/// there is no usable color (image fills, missing fills, transparent wrappers).
fn first_solid_color(fills: Option<&Vec<PenFill>>) -> Option<String> {
    let fills = fills?;
    for fill in fills {
        match fill {
            PenFill::Solid(body) => {
                if body.opacity == Some(0.0) {
                    continue;
                }
                if is_transparent_hex(&body.color) {
                    continue;
                }
                return Some(body.color.clone());
            }
            PenFill::LinearGradient(body) => {
                if body.opacity == Some(0.0) {
                    continue;
                }
                if let Some(stop) = body.stops.first() {
                    return Some(stop.color.clone());
                }
            }
            PenFill::RadialGradient(body) => {
                if body.opacity == Some(0.0) {
                    continue;
                }
                if let Some(stop) = body.stops.first() {
                    return Some(stop.color.clone());
                }
            }
            PenFill::Image(_) => continue,
            // Mesh gradients / SkSL shaders have no single representative
            // color for contrast checks — treated like image fills.
            PenFill::MeshGradient(_) | PenFill::Shader(_) => continue,
        }
    }
    None
}

/// True for a 9-char `#RRGGBBAA` hex whose alpha pair is `00` — a fully
/// transparent solid, treated the same as `opacity == 0`.
fn is_transparent_hex(color: &str) -> bool {
    color.len() == 9 && color[7..].eq_ignore_ascii_case("00")
}

/// Port of `isLargeText` (`detectors-typography.ts:87-99`). WCAG 2.x large
/// text: `fontSize >= 24`, or `fontSize >= 19` with `fontWeight >= 700`.
///
/// T0 audit footnote 12: jian `FontWeight` is `Number(u32)` | `Keyword(String)`.
/// Only the numeric path is large-eligible — a `Keyword` weight cannot
/// `parseInt` to a finite number, matching the TS `NaN` result.
fn is_large_text(text: &TextNode) -> bool {
    let Some(font_size) = text.font_size else {
        return false;
    };
    if font_size >= 24.0 {
        return true;
    }
    let weight = match &text.font_weight {
        Some(FontWeight::Number(w)) => Some(*w),
        Some(FontWeight::Keyword(_)) | None => None,
    };
    font_size >= 19.0 && weight.is_some_and(|w| w >= 700)
}

/// The text node's existing `fill` as raw JSON, or `Value::Null` — mirrors the
/// TS `currentValue: textFill ?? null`.
fn text_fill_value(text: &TextNode) -> Value {
    match &text.fill {
        Some(fills) => serde_json::to_value(fills).unwrap_or(Value::Null),
        None => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Deserialize a JSON value into a `PenDocument`.
    fn doc(value: serde_json::Value) -> PenDocument {
        serde_json::from_value(value).expect("fixture must deserialize as PenDocument")
    }

    /// Deserialize a JSON value into a `PenNode`.
    fn node(value: serde_json::Value) -> PenNode {
        serde_json::from_value(value).expect("fixture must deserialize as PenNode")
    }

    /// White text on a white-ish background → contrast ~1.0, well below the
    /// 2.5 normal threshold → one `text-bg-contrast` issue.
    #[test]
    fn flags_low_contrast_white_on_white() {
        let root = node(json!({
            "type": "frame", "id": "page",
            "fill": [{"type": "solid", "color": "#FFFFFF"}],
            "children": [
                {
                    "type": "text", "id": "t1", "content": "Hello",
                    "fill": [{"type": "solid", "color": "#FCFCFC"}]
                }
            ]
        }));
        let issues =
            detect_text_bg_contrast(&root, &doc(json!({"version": "1.0", "children": []})));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "t1");
        assert_eq!(issues[0].category, IssueCategory::TextBgContrast);
        assert_eq!(issues[0].property, FixProperty::Fill);
        assert_eq!(issues[0].severity, IssueSeverity::Info);
        assert_eq!(issues[0].suggested_value, json!(null));
    }

    /// Black text on a white background → contrast 21:1 → no issue.
    #[test]
    fn ignores_high_contrast_black_on_white() {
        let root = node(json!({
            "type": "frame", "id": "page",
            "fill": [{"type": "solid", "color": "#FFFFFF"}],
            "children": [
                {
                    "type": "text", "id": "t1", "content": "Hello",
                    "fill": [{"type": "solid", "color": "#000000"}]
                }
            ]
        }));
        assert!(
            detect_text_bg_contrast(&root, &doc(json!({"version": "1.0", "children": []})))
                .is_empty()
        );
    }

    /// A `visible:false` text node is pruned by the walk → never checked.
    #[test]
    fn prunes_invisible_text_node() {
        let root = node(json!({
            "type": "frame", "id": "page",
            "fill": [{"type": "solid", "color": "#FFFFFF"}],
            "children": [
                {
                    "type": "text", "id": "t1", "content": "Hello",
                    "visible": false,
                    "fill": [{"type": "solid", "color": "#FCFCFC"}]
                }
            ]
        }));
        assert!(
            detect_text_bg_contrast(&root, &doc(json!({"version": "1.0", "children": []})))
                .is_empty()
        );
    }

    /// An `opacity:0` wrapper fill is skipped so contrast is computed against
    /// the real bg behind it — a white wrapper with opacity 0 over a cream
    /// page must NOT mask the cream-on-cream failure.
    #[test]
    fn skips_transparent_wrapper_fill_to_real_bg() {
        // Cream page > opacity-0 white wrapper > cream-ish text.
        // Without the skip, the wrapper would read as a #FFFFFF bg and the
        // cream text would look fine.
        let root = node(json!({
            "type": "frame", "id": "page",
            "fill": [{"type": "solid", "color": "#FFF8E1"}],
            "children": [
                {
                    "type": "frame", "id": "wrap",
                    "fill": [{"type": "solid", "color": "#FFFFFF", "opacity": 0}],
                    "children": [
                        {
                            "type": "text", "id": "t1", "content": "Hi",
                            "fill": [{"type": "solid", "color": "#FFFDF5"}]
                        }
                    ]
                }
            ]
        }));
        let issues =
            detect_text_bg_contrast(&root, &doc(json!({"version": "1.0", "children": []})));
        // Cream text on cream page → low contrast → flagged. The reason names
        // the real page bg, not the transparent wrapper.
        assert_eq!(issues.len(), 1);
        assert!(issues[0].reason.contains("#FFF8E1"));
    }

    /// A `$ref` fill resolving via `doc.variables` is compared correctly.
    #[test]
    fn resolves_variable_ref_fills() {
        let document = doc(json!({
            "version": "1.0",
            "children": [],
            "variables": {
                "color-bg": {"type": "color", "value": "#FFFFFF"},
                "color-text": {"type": "color", "value": "#F5F5F5"}
            }
        }));
        let root = node(json!({
            "type": "frame", "id": "page",
            "fill": [{"type": "solid", "color": "$color-bg"}],
            "children": [
                {
                    "type": "text", "id": "t1", "content": "Hi",
                    "fill": [{"type": "solid", "color": "$color-text"}]
                }
            ]
        }));
        let issues = detect_text_bg_contrast(&root, &document);
        // #F5F5F5 on #FFFFFF → ~1.07:1 → flagged.
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "t1");
    }

    /// An unresolvable `$ref` text fill skips the check rather than guessing.
    #[test]
    fn skips_unresolvable_ref_fill() {
        let root = node(json!({
            "type": "frame", "id": "page",
            "fill": [{"type": "solid", "color": "#FFFFFF"}],
            "children": [
                {
                    "type": "text", "id": "t1", "content": "Hi",
                    "fill": [{"type": "solid", "color": "$color-typo"}]
                }
            ]
        }));
        assert!(
            detect_text_bg_contrast(&root, &doc(json!({"version": "1.0", "children": []})))
                .is_empty()
        );
    }

    /// Large text (`fontSize >= 24`) uses the looser 2.0 threshold — a ratio
    /// between 2.0 and 2.5 that would flag normal text passes for large text.
    #[test]
    fn large_text_uses_lower_threshold() {
        // #9A9A9A on #FFFFFF ≈ 2.6:1 — above 2.5 (passes either way).
        // Pick a pair landing between 2.0 and 2.5 so size matters:
        // #8C8C8C on #FFFFFF ≈ 3.0... use a tighter pair.
        // #B0B0B0 on #FFFFFF ≈ 2.1:1 — below 2.5 (normal flags), above 2.0
        // (large passes).
        let large = node(json!({
            "type": "frame", "id": "page",
            "fill": [{"type": "solid", "color": "#FFFFFF"}],
            "children": [
                {
                    "type": "text", "id": "big", "content": "Title", "fontSize": 28,
                    "fill": [{"type": "solid", "color": "#B0B0B0"}]
                }
            ]
        }));
        assert!(
            detect_text_bg_contrast(&large, &doc(json!({"version": "1.0", "children": []})))
                .is_empty(),
            "large text at ~2.1:1 is above the 2.0 large threshold"
        );

        // The same color on a normal-size text node IS flagged (2.5 threshold).
        let normal = node(json!({
            "type": "frame", "id": "page",
            "fill": [{"type": "solid", "color": "#FFFFFF"}],
            "children": [
                {
                    "type": "text", "id": "small", "content": "Body", "fontSize": 14,
                    "fill": [{"type": "solid", "color": "#B0B0B0"}]
                }
            ]
        }));
        let issues =
            detect_text_bg_contrast(&normal, &doc(json!({"version": "1.0", "children": []})));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "small");
    }
}
