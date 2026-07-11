//! Structural detectors ported from `pen-ai-skills/src/diagnostics/detectors.ts`.

use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::PenEffect;
use jian_ops_schema::PenDocument;
use serde_json::{json, Value};

use crate::color::{color_contrast, parse_hex_color, relative_luminance};
use crate::issue::{FixProperty, Issue, IssueCategory, IssueSeverity};
use crate::node_util::{
    children, first_fill_color, has_stroke, json_number, node_id, node_kind, node_kind_str,
    rotation, NodeKind,
};

/// Port of `detectEmptyPaths` (`detectors.ts:125-149`). Flags `path` nodes
/// with no `d` geometry — they render as invisible empty rectangles.
///
/// T0 audit blocker **B2**: jian `PathNode.d` is `Option<String>`; the TS
/// detector treats `d` as falsy when `undefined` *or* `""`, so this port
/// flags both `None` and `Some("")`. The `anchors` alternate geometry is
/// ignored, matching the TS detector which only inspects `d`.
pub fn detect_empty_paths(root: &PenNode) -> Vec<Issue> {
    let mut issues = Vec::new();
    walk_empty_paths(root, &mut issues);
    issues
}

fn walk_empty_paths(node: &PenNode, issues: &mut Vec<Issue>) {
    if let PenNode::Path(p) = node {
        let has_d = p.d.as_deref().map(|s| !s.is_empty()).unwrap_or(false);
        if !has_d {
            issues.push(Issue {
                node_id: node_id(node).to_string(),
                category: IssueCategory::EmptyPath,
                severity: IssueSeverity::Warning,
                property: FixProperty::Remove,
                current_value: Value::Null,
                suggested_value: Value::Bool(true),
                reason: "path node without geometry (renders invisible)".to_string(),
            });
        }
    }
    for child in children(node) {
        walk_empty_paths(child, issues);
    }
}

/// Port of `detectUnexpectedRotation` (`detectors.ts:325-361`). Flags nodes
/// carrying a rotation that is neither `0` nor a near-multiple of `90` — UI
/// nodes are rarely tilted on purpose. `path` / `line` / `polygon` / `image`
/// kinds are skipped (rotation is expected on geometry/media). A missing
/// `rotation` is treated as `0` by [`crate::node_util::rotation`].
pub fn detect_unexpected_rotation(root: &PenNode) -> Vec<Issue> {
    let mut issues = Vec::new();
    walk_unexpected_rotation(root, &mut issues);
    issues
}

fn walk_unexpected_rotation(node: &PenNode, issues: &mut Vec<Issue>) {
    let skipped = matches!(
        node_kind(node),
        NodeKind::Path | NodeKind::Line | NodeKind::Polygon | NodeKind::Image
    );
    let rot = rotation(node);
    if !skipped && rot != 0.0 && (rot % 90.0).abs() > 0.5 {
        let kind = node_kind_str(node);
        issues.push(Issue {
            node_id: node_id(node).to_string(),
            category: IssueCategory::UnexpectedRotation,
            severity: IssueSeverity::Warning,
            property: FixProperty::Rotation,
            current_value: json_number(rot),
            suggested_value: Value::from(0),
            reason: format!(
                "{kind} node has rotation={rot}; UI nodes are rarely tilted on purpose"
            ),
        });
    }
    for child in children(node) {
        walk_unexpected_rotation(child, issues);
    }
}

/// Largest "typical UI" shadow blur. Threshold is strictly-greater: a
/// production modal-shell scrim legitimately uses `blur = 40`.
const TYPICAL_BLUR_MAX: f32 = 40.0;

/// Port of `detectExcessiveFrameEffects` (`detectors.ts:642-696`). Flags a
/// `frame` whose `effects` array has 3+ entries, or carries a shadow with an
/// atypical `blur` (> 40) or any positive `spread`.
///
/// T0 audit footnotes 9/10: jian `PenEffect` is `#[serde(tag="type")]` —
/// `blur`/`spread` live only on the `Shadow(ShadowBody)` variant. A
/// `Blur`/`BackgroundBlur` effect (which carries only `radius`) has no
/// `blur`/`spread` to inspect, so it can only trip the `len >= 3` count —
/// matching the TS, where `r.blur` on a blur-type effect is `undefined`.
pub fn detect_excessive_frame_effects(root: &PenNode) -> Vec<Issue> {
    let mut issues = Vec::new();
    walk_excessive_frame_effects(root, &mut issues);
    issues
}

fn walk_excessive_frame_effects(node: &PenNode, issues: &mut Vec<Issue>) {
    if let PenNode::Frame(frame) = node {
        if let Some(effects) = frame.container.effects.as_ref() {
            if !effects.is_empty() {
                if let Some(reason) = excessive_effects_reason(effects) {
                    issues.push(Issue {
                        node_id: node_id(node).to_string(),
                        category: IssueCategory::ExcessiveFrameEffects,
                        severity: IssueSeverity::Warning,
                        property: FixProperty::Effects,
                        current_value: serde_json::to_value(effects).unwrap_or(Value::Null),
                        suggested_value: Value::Null,
                        reason,
                    });
                }
            }
        }
    }
    for child in children(node) {
        walk_excessive_frame_effects(child, issues);
    }
}

/// The trigger reason for a non-empty effects list, or `None` when the list
/// is unremarkable. The `len >= 3` check wins first and short-circuits;
/// otherwise the first shadow with an atypical blur or positive spread wins.
fn excessive_effects_reason(effects: &[PenEffect]) -> Option<String> {
    if effects.len() >= 3 {
        return Some(format!(
            "{} stacked effects on one frame (typical UI uses 0-2)",
            effects.len()
        ));
    }
    for effect in effects {
        if let PenEffect::Shadow(shadow) = effect {
            if shadow.blur > TYPICAL_BLUR_MAX {
                return Some(format!(
                    "effect blur {} > {} (glow / halo, atypical for UI)",
                    shadow.blur, TYPICAL_BLUR_MAX
                ));
            }
            if shadow.spread > 0.0 {
                return Some(format!(
                    "effect spread {} > 0 (bleeds outward, \"spiked\" appearance)",
                    shadow.spread
                ));
            }
        }
    }
    None
}

/// Perceptual contrast ratio at or below which two colors count as "the
/// same" — the `DetectInvisibleContainersOptions` default. S1 does not
/// expose the options struct (YAGNI); the perceptual-mode default is inlined.
const INVISIBLE_CONTRAST_THRESHOLD: f64 = 1.1;

/// Port of `detectInvisibleContainers` (`detectors.ts:65-122`). Flags a
/// layout `frame` with children and no stroke whose fill is perceptually
/// indistinguishable from its parent's fill — the container renders
/// "invisible". Suggests a subtle border stroke.
///
/// Severity is `info` when the parent fill is dark (a light-gray border
/// would damage a dark design — detect-only), else `warning` (auto-fixable).
pub fn detect_invisible_containers(root: &PenNode, doc: &PenDocument) -> Vec<Issue> {
    let mut issues = Vec::new();
    let border_stroke = border_stroke(doc);
    walk_invisible_containers(root, None, false, &border_stroke, &mut issues);
    issues
}

fn walk_invisible_containers(
    node: &PenNode,
    parent_fill: Option<&str>,
    in_chart_context: bool,
    border_stroke: &Value,
    issues: &mut Vec<Issue>,
) {
    let node_fill = first_fill_color(node);
    let in_chart_context = in_chart_context || is_chart_context_node(node);

    if let (Some(parent), Some(fill)) = (parent_fill, node_fill) {
        let is_layout_frame = matches!(node, PenNode::Frame(f) if f.container.layout.is_some());
        if is_layout_frame
            && !has_stroke(node)
            && !children(node).is_empty()
            && !is_chart_visual_container(node, in_chart_context)
        {
            let ratio = color_contrast(fill, parent);
            if ratio <= INVISIBLE_CONTRAST_THRESHOLD {
                let is_dark_on_dark = parse_hex_color(parent)
                    .map(|c| relative_luminance(c) < 0.1)
                    .unwrap_or(false);
                let severity = if is_dark_on_dark {
                    IssueSeverity::Info
                } else {
                    IssueSeverity::Warning
                };
                issues.push(Issue {
                    node_id: node_id(node).to_string(),
                    category: IssueCategory::InvisibleContainer,
                    severity,
                    property: FixProperty::Stroke,
                    current_value: existing_stroke(node),
                    suggested_value: border_stroke.clone(),
                    reason: format!("same fill as parent ({fill} ≈ {parent}, contrast={ratio:.2})"),
                });
            }
        }
    }

    let inherited = node_fill.or(parent_fill);
    for child in children(node) {
        walk_invisible_containers(child, inherited, in_chart_context, border_stroke, issues);
    }
}

fn is_chart_visual_container(node: &PenNode, in_chart_context: bool) -> bool {
    if !in_chart_context {
        return false;
    }
    let label = node_identity_label(node);
    if ["card", "panel", "section", "sidebar", "container"]
        .iter()
        .any(|needle| label.contains(needle))
    {
        return false;
    }
    if is_chart_context_node(node) {
        return true;
    }
    let barish = label.contains("bar")
        || label.contains("track")
        || label.contains("column")
        || label.contains("柱");
    barish || (children(node).len() <= 3 && is_slender_chart_mark(node))
}

fn is_chart_context_node(node: &PenNode) -> bool {
    let label = node_identity_label(node);
    label.contains("chart")
        || label.contains("graph")
        || label.contains("histogram")
        || label.contains("sparkline")
        || label.contains("bar chart")
        || label.contains("weekly bars")
        || label.contains("activity chart")
        || label.contains("activity graph")
        || label.contains("activity bars")
        || label.contains("图表")
        || label.contains("柱状")
}

fn is_slender_chart_mark(node: &PenNode) -> bool {
    let Some(width) = numeric_node_prop(node, "width") else {
        return false;
    };
    let Some(height) = numeric_node_prop(node, "height") else {
        return false;
    };
    width > 0.0
        && height > 0.0
        && ((width <= 110.0 && height >= width * 1.25)
            || (height <= 110.0 && width >= height * 1.25))
}

fn numeric_node_prop(node: &PenNode, key: &str) -> Option<f64> {
    serde_json::to_value(node).ok()?.get(key)?.as_f64()
}

fn node_identity_label(node: &PenNode) -> String {
    let mut label = node_id(node).to_ascii_lowercase();
    if let Ok(value) = serde_json::to_value(node) {
        for key in ["role", "name"] {
            if let Some(text) = value.get(key).and_then(Value::as_str) {
                label.push(' ');
                label.push_str(&text.to_ascii_lowercase());
            }
        }
    }
    label
}

/// Port of `getBorderStroke` (`detectors.ts:27-34`). Prefers the
/// `$color-border` variable when the document defines it, else a neutral
/// light-gray hex.
fn border_stroke(doc: &PenDocument) -> Value {
    let has_border_var = doc
        .variables
        .as_ref()
        .is_some_and(|vars| vars.contains_key("color-border"));
    let color = if has_border_var {
        "$color-border"
    } else {
        "#E2E8F0"
    };
    json!({ "thickness": 1, "fill": [{ "type": "solid", "color": color }] })
}

/// The node's existing `stroke` as raw JSON, or `Value::Null` — mirrors the
/// TS `(node as { stroke?: unknown }).stroke ?? null`.
fn existing_stroke(node: &PenNode) -> Value {
    let stroke = match node {
        PenNode::Frame(n) => n.container.stroke.as_ref(),
        PenNode::Group(n) => n.container.stroke.as_ref(),
        PenNode::Rectangle(n) => n.container.stroke.as_ref(),
        _ => None,
    };
    stroke
        .and_then(|s| serde_json::to_value(s).ok())
        .unwrap_or(Value::Null)
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
    fn flags_path_node_without_geometry() {
        let root = node(json!({"type": "path", "id": "p1"}));
        let issues = detect_empty_paths(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "p1");
        assert_eq!(issues[0].category, IssueCategory::EmptyPath);
        assert_eq!(issues[0].property, FixProperty::Remove);
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
        assert_eq!(issues[0].suggested_value, json!(true));
    }

    #[test]
    fn flags_path_node_with_empty_d_string() {
        // B2: `Some("")` is falsy in TS — must be flagged.
        let root = node(json!({"type": "path", "id": "p1", "d": ""}));
        assert_eq!(detect_empty_paths(&root).len(), 1);
    }

    #[test]
    fn ignores_path_with_geometry() {
        let root = node(json!({"type": "path", "id": "p1", "d": "M0 0L1 1"}));
        assert!(detect_empty_paths(&root).is_empty());
    }

    #[test]
    fn walks_nested_path_nodes() {
        let root = node(json!({
            "type": "frame",
            "id": "root",
            "children": [
                {"type": "frame", "id": "inner", "children": [
                    {"type": "path", "id": "deep"}
                ]}
            ]
        }));
        let issues = detect_empty_paths(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "deep");
    }

    #[test]
    fn flags_frame_with_unexpected_rotation() {
        let root = node(json!({"type": "frame", "id": "f1", "rotation": 12}));
        let issues = detect_unexpected_rotation(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "f1");
        assert_eq!(issues[0].category, IssueCategory::UnexpectedRotation);
        assert_eq!(issues[0].property, FixProperty::Rotation);
        assert_eq!(issues[0].suggested_value, json!(0));
    }

    #[test]
    fn ignores_zero_rotation() {
        let root = node(json!({"type": "frame", "id": "f1", "rotation": 0}));
        assert!(detect_unexpected_rotation(&root).is_empty());

        // Missing rotation → treated as 0.
        let bare = node(json!({"type": "frame", "id": "f1"}));
        assert!(detect_unexpected_rotation(&bare).is_empty());
    }

    #[test]
    fn ignores_multiple_of_ninety() {
        let root = node(json!({"type": "frame", "id": "f1", "rotation": 90}));
        assert!(detect_unexpected_rotation(&root).is_empty());

        let r270 = node(json!({"type": "frame", "id": "f1", "rotation": 270}));
        assert!(detect_unexpected_rotation(&r270).is_empty());
    }

    #[test]
    fn ignores_skipped_kinds() {
        let path = node(json!({"type": "path", "id": "p1", "rotation": 12, "d": "M0 0"}));
        assert!(detect_unexpected_rotation(&path).is_empty());
    }

    #[test]
    fn flags_nested_unexpected_rotation() {
        let root = node(json!({
            "type": "frame",
            "id": "root",
            "children": [
                {"type": "frame", "id": "inner", "children": [
                    {"type": "text", "id": "tilted", "content": "hi", "rotation": 12}
                ]}
            ]
        }));
        let issues = detect_unexpected_rotation(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "tilted");
    }

    /// A shadow effect JSON with the given `blur` / `spread`.
    fn shadow(blur: f64, spread: f64) -> serde_json::Value {
        json!({
            "type": "shadow",
            "offsetX": 0, "offsetY": 0,
            "blur": blur, "spread": spread,
            "color": "#000000"
        })
    }

    #[test]
    fn flags_frame_with_three_stacked_effects() {
        let root = node(json!({
            "type": "frame", "id": "f1",
            "effects": [shadow(8.0, 0.0), shadow(8.0, 0.0), shadow(8.0, 0.0)]
        }));
        let issues = detect_excessive_frame_effects(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].category, IssueCategory::ExcessiveFrameEffects);
        assert_eq!(issues[0].property, FixProperty::Effects);
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
        assert_eq!(issues[0].suggested_value, json!(null));
        assert!(issues[0].reason.contains("stacked effects"));
    }

    #[test]
    fn flags_frame_with_atypical_blur() {
        let root = node(json!({
            "type": "frame", "id": "f1",
            "effects": [shadow(41.0, 0.0)]
        }));
        let issues = detect_excessive_frame_effects(&root);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].reason.contains("blur"));
    }

    #[test]
    fn flags_frame_with_positive_spread() {
        let root = node(json!({
            "type": "frame", "id": "f1",
            "effects": [shadow(16.0, 4.0)]
        }));
        let issues = detect_excessive_frame_effects(&root);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].reason.contains("spiked"));
    }

    #[test]
    fn ignores_typical_shadow() {
        let root = node(json!({
            "type": "frame", "id": "f1",
            "effects": [shadow(16.0, 0.0)]
        }));
        assert!(detect_excessive_frame_effects(&root).is_empty());
    }

    #[test]
    fn ignores_blur_effect_without_shadow_fields() {
        // A `blur` effect (BlurBody) carries only `radius` — no blur/spread.
        let root = node(json!({
            "type": "frame", "id": "f1",
            "effects": [{"type": "blur", "radius": 50}]
        }));
        assert!(detect_excessive_frame_effects(&root).is_empty());
    }

    #[test]
    fn ignores_frame_with_no_effects() {
        let bare = node(json!({"type": "frame", "id": "f1"}));
        assert!(detect_excessive_frame_effects(&bare).is_empty());

        let empty = node(json!({"type": "frame", "id": "f1", "effects": []}));
        assert!(detect_excessive_frame_effects(&empty).is_empty());
    }

    /// Deserialize a JSON value into a `PenDocument`.
    fn doc(value: serde_json::Value) -> PenDocument {
        serde_json::from_value(value).expect("fixture must deserialize as PenDocument")
    }

    /// A document with a single root node and no variables.
    fn doc_no_vars() -> PenDocument {
        doc(json!({"version": "1.0", "children": []}))
    }

    /// A layout-frame root holding a same-fill layout-frame child that itself
    /// contains a child. `parent_fill` is the root frame's fill color.
    fn invisible_fixture(parent_fill: &str) -> PenNode {
        node(json!({
            "type": "frame", "id": "root", "layout": "vertical",
            "fill": [{"type": "solid", "color": parent_fill}],
            "children": [
                {
                    "type": "frame", "id": "wrapper", "layout": "vertical",
                    "fill": [{"type": "solid", "color": parent_fill}],
                    "children": [
                        {"type": "text", "id": "label", "content": "hi"}
                    ]
                }
            ]
        }))
    }

    #[test]
    fn flags_layout_frame_with_same_fill_as_parent() {
        let root = invisible_fixture("#FFFFFF");
        let issues = detect_invisible_containers(&root, &doc_no_vars());
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "wrapper");
        assert_eq!(issues[0].category, IssueCategory::InvisibleContainer);
        assert_eq!(issues[0].property, FixProperty::Stroke);
    }

    #[test]
    fn light_parent_yields_warning_severity() {
        let root = invisible_fixture("#FFFFFF");
        let issues = detect_invisible_containers(&root, &doc_no_vars());
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
    }

    #[test]
    fn dark_parent_yields_info_severity() {
        let root = invisible_fixture("#0A0A0A");
        let issues = detect_invisible_containers(&root, &doc_no_vars());
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, IssueSeverity::Info);
    }

    #[test]
    fn ignores_frame_with_stroke() {
        let root = node(json!({
            "type": "frame", "id": "root", "layout": "vertical",
            "fill": [{"type": "solid", "color": "#FFFFFF"}],
            "children": [
                {
                    "type": "frame", "id": "wrapper", "layout": "vertical",
                    "fill": [{"type": "solid", "color": "#FFFFFF"}],
                    "stroke": {"thickness": 1},
                    "children": [{"type": "text", "id": "label", "content": "hi"}]
                }
            ]
        }));
        assert!(detect_invisible_containers(&root, &doc_no_vars()).is_empty());
    }

    #[test]
    fn ignores_frame_with_different_fill() {
        let root = node(json!({
            "type": "frame", "id": "root", "layout": "vertical",
            "fill": [{"type": "solid", "color": "#FFFFFF"}],
            "children": [
                {
                    "type": "frame", "id": "wrapper", "layout": "vertical",
                    "fill": [{"type": "solid", "color": "#1A1A1A"}],
                    "children": [{"type": "text", "id": "label", "content": "hi"}]
                }
            ]
        }));
        assert!(detect_invisible_containers(&root, &doc_no_vars()).is_empty());
    }

    #[test]
    fn ignores_frame_with_no_children() {
        let root = node(json!({
            "type": "frame", "id": "root", "layout": "vertical",
            "fill": [{"type": "solid", "color": "#FFFFFF"}],
            "children": [
                {
                    "type": "frame", "id": "wrapper", "layout": "vertical",
                    "fill": [{"type": "solid", "color": "#FFFFFF"}]
                }
            ]
        }));
        assert!(detect_invisible_containers(&root, &doc_no_vars()).is_empty());
    }

    #[test]
    fn suggested_value_uses_color_border_variable_when_present() {
        let with_var = doc(json!({
            "version": "1.0",
            "children": [],
            "variables": {
                "color-border": {"type": "color", "value": "#CCCCCC"}
            }
        }));
        let root = invisible_fixture("#FFFFFF");
        let issues = detect_invisible_containers(&root, &with_var);
        assert_eq!(
            issues[0].suggested_value,
            json!({"thickness": 1, "fill": [{"type": "solid", "color": "$color-border"}]})
        );
    }

    #[test]
    fn suggested_value_falls_back_to_hex_without_variable() {
        let root = invisible_fixture("#FFFFFF");
        let issues = detect_invisible_containers(&root, &doc_no_vars());
        assert_eq!(
            issues[0].suggested_value,
            json!({"thickness": 1, "fill": [{"type": "solid", "color": "#E2E8F0"}]})
        );
    }

    #[test]
    fn invisible_detector_skips_chart_bar_tracks() {
        let day_track = |id: &str, label: &str, fill: &str| {
            json!({
                "type": "frame",
                "id": id,
                "name": format!("{label} Bar Track"),
                "layout": "vertical",
                "width": 96,
                "height": 250,
                "fill": [{"type": "solid", "color": "#171717"}],
                "children": [{
                    "type": "frame",
                    "id": format!("{id}-fill"),
                    "name": format!("{label} Bar Fill"),
                    "width": "fill_container",
                    "height": 120,
                    "cornerRadius": 22,
                    "fill": [{"type": "solid", "color": fill}],
                    "children": []
                }]
            })
        };
        let root = node(json!({
            "type": "frame",
            "id": "root",
            "name": "Health Dashboard",
            "layout": "vertical",
            "fill": [{"type": "solid", "color": "#171717"}],
            "children": [{
                "type": "frame",
                "id": "weekly-chart",
                "name": "Weekly Activity Chart",
                "layout": "horizontal",
                "fill": [{"type": "solid", "color": "#171717"}],
                "children": [
                    day_track("mon", "Monday", "#2B2B2B"),
                    day_track("tue", "Tuesday", "#2B2B2B"),
                    day_track("wed", "Wednesday", "#2B2B2B"),
                    day_track("thu", "Thursday", "#2B2B2B"),
                    day_track("fri", "Friday", "#00D15E"),
                    day_track("sat", "Saturday", "#2B2B2B"),
                    day_track("sun", "Sunday", "#2B2B2B")
                ]
            }]
        }));

        let issues = detect_invisible_containers(&root, &doc_no_vars());

        assert!(
            issues.is_empty(),
            "chart bar tracks are marks, not card surfaces needing strokes: {issues:?}"
        );
    }

    #[test]
    fn invisible_detector_keeps_bar_chart_card_surface_fixable() {
        let root = node(json!({
            "type": "frame",
            "id": "root",
            "name": "Health Dashboard",
            "layout": "vertical",
            "fill": [{"type": "solid", "color": "#FFFFFF"}],
            "children": [{
                "type": "frame",
                "id": "chart-card",
                "name": "Bar Chart Card",
                "layout": "vertical",
                "width": 320,
                "height": 300,
                "fill": [{"type": "solid", "color": "#FFFFFF"}],
                "children": [{
                    "type": "text",
                    "id": "title",
                    "content": "Weekly activity"
                }]
            }]
        }));

        let issues = detect_invisible_containers(&root, &doc_no_vars());

        assert_eq!(issues.len(), 1, "chart card should remain border-fixable");
        assert_eq!(issues[0].node_id, "chart-card");
    }
}
