//! Cross-detector tree-walk + field-access helpers, plus Rust ports of the
//! three `pen-core` helpers the typography detector depends on.
//!
//! The accessor helpers (`node_id`, `node_kind`, `children`, `role`,
//! `rotation`, `opacity`) wrap jian's typed `PenNode` enum so the detector
//! code reads uniformly regardless of node kind. The TS detectors did the
//! same thing with loose `'field' in node` guards; here the kind matching
//! happens once, in this module.
//!
//! Ports (with TS origin):
//! - `first_fill_color` ← `getFirstFillColor` (`detectors.ts:8-14`)
//! - `has_stroke` ← `hasStroke` (`detectors.ts:17-21`); uses
//!   `stroke_thickness_max` per T0 audit blocker **B1**
//! - `is_node_visible` ← pen-core `isNodeVisible` (`layout/engine.ts:44-49`);
//!   typed-enum adaptation per T0 audit blocker **B3**
//! - `resolve_color_ref` / `default_theme` ← pen-core `resolveColorRef` /
//!   `getDefaultTheme` (`variables/resolve.ts`)

use std::collections::BTreeMap;

use jian_ops_schema::node::base::NumberOrExpression;
use jian_ops_schema::node::{BoolOrExpression, CornerRadius, Padding, PenNode};
use jian_ops_schema::sizing::SizingBehavior;
use jian_ops_schema::style::{PenFill, StrokeThickness};
use jian_ops_schema::variable::{VariableDefinition, VariableScalar, VariableValue};
use serde_json::Value;

/// The 17 jian `PenNode` kinds, as a flat `Copy` enum. jian models kind via
/// the `PenNode` enum variant + the `"type"` tag; detectors need a cheap
/// kind discriminant for skip-sets and grouping keys, so this mirrors it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    Frame,
    Group,
    Rectangle,
    Ellipse,
    Line,
    Polygon,
    Path,
    Text,
    TextInput,
    Image,
    IconFont,
    TextArea,
    Select,
    Switch,
    Checkbox,
    Slider,
    RadioGroup,
    NumberInput,
    Progress,
    Tabs,
    Ref,
}

/// An active theme map — axis name → selected theme value. Mirrors the TS
/// `Theme = Record<string, string>` used by `resolveColorRef`.
pub type Theme = BTreeMap<String, String>;

/// The document variable table — variable name → definition. Mirrors the
/// jian `PenDocument.variables` map value type.
pub type Variables = BTreeMap<String, VariableDefinition>;

/// Borrow a node's id (`PenNodeBase.id`).
pub fn node_id(node: &PenNode) -> &str {
    base(node).id.as_str()
}

/// The node's kind discriminant.
pub fn node_kind(node: &PenNode) -> NodeKind {
    match node {
        PenNode::Frame(_) => NodeKind::Frame,
        PenNode::Group(_) => NodeKind::Group,
        PenNode::Rectangle(_) => NodeKind::Rectangle,
        PenNode::Ellipse(_) => NodeKind::Ellipse,
        PenNode::Line(_) => NodeKind::Line,
        PenNode::Polygon(_) => NodeKind::Polygon,
        PenNode::Path(_) => NodeKind::Path,
        PenNode::Text(_) => NodeKind::Text,
        PenNode::TextInput(_) => NodeKind::TextInput,
        PenNode::Image(_) => NodeKind::Image,
        PenNode::IconFont(_) => NodeKind::IconFont,
        PenNode::TextArea(_) => NodeKind::TextArea,
        PenNode::Select(_) => NodeKind::Select,
        PenNode::Switch(_) => NodeKind::Switch,
        PenNode::Checkbox(_) => NodeKind::Checkbox,
        PenNode::Slider(_) => NodeKind::Slider,
        PenNode::RadioGroup(_) => NodeKind::RadioGroup,
        PenNode::NumberInput(_) => NodeKind::NumberInput,
        PenNode::Progress(_) => NodeKind::Progress,
        PenNode::Tabs(_) => NodeKind::Tabs,
        PenNode::Ref(_) => NodeKind::Ref,
    }
}

/// The lowercase, snake_case kind string — identical to the TS `node.type`
/// value and the jian `"type"` wire tag (`"frame"`, `"text"`, `"icon_font"`,
/// `"text_input"`, …). Used verbatim in detector `reason` strings.
pub fn node_kind_str(node: &PenNode) -> &'static str {
    match node_kind(node) {
        NodeKind::Frame => "frame",
        NodeKind::Group => "group",
        NodeKind::Rectangle => "rectangle",
        NodeKind::Ellipse => "ellipse",
        NodeKind::Line => "line",
        NodeKind::Polygon => "polygon",
        NodeKind::Path => "path",
        NodeKind::Text => "text",
        NodeKind::TextInput => "text_input",
        NodeKind::Image => "image",
        NodeKind::IconFont => "icon_font",
        NodeKind::TextArea => "text_area",
        NodeKind::Select => "select",
        NodeKind::Switch => "switch",
        NodeKind::Checkbox => "checkbox",
        NodeKind::Slider => "slider",
        NodeKind::RadioGroup => "radio_group",
        NodeKind::NumberInput => "number_input",
        NodeKind::Progress => "progress",
        NodeKind::Tabs => "tabs",
        NodeKind::Ref => "ref",
    }
}

impl NodeKind {
    /// True when this kind is a structural container — Frame, Group, or Rectangle.
    pub fn is_container(self) -> bool {
        matches!(
            self,
            NodeKind::Frame | NodeKind::Group | NodeKind::Rectangle
        )
    }
}

/// Read an authored x coordinate. A missing coordinate is not treated as zero
/// because flex layout may supply it later.
pub fn node_x(node: &PenNode) -> Option<f64> {
    base(node).x
}

/// Read an authored y coordinate. A missing coordinate is not treated as zero
/// because flex layout may supply it later.
pub fn node_y(node: &PenNode) -> Option<f64> {
    base(node).y
}

/// Read a numeric authored width from the node kinds that have one.
pub fn numeric_width(node: &PenNode) -> Option<f64> {
    numeric_sizing(width_sizing(node))
}

/// Read a numeric authored height from the node kinds that have one.
pub fn numeric_height(node: &PenNode) -> Option<f64> {
    numeric_sizing(height_sizing(node))
}

fn width_sizing(node: &PenNode) -> Option<&SizingBehavior> {
    match node {
        PenNode::Frame(n) => n.container.width.as_ref(),
        PenNode::Group(n) => n.container.width.as_ref(),
        PenNode::Rectangle(n) => n.container.width.as_ref(),
        PenNode::Ellipse(n) => n.width.as_ref(),
        PenNode::Polygon(n) => n.width.as_ref(),
        PenNode::Path(n) => n.width.as_ref(),
        PenNode::Image(n) => n.width.as_ref(),
        _ => None,
    }
}

fn numeric_sizing(sizing: Option<&SizingBehavior>) -> Option<f64> {
    match sizing {
        Some(SizingBehavior::Number(value)) if value.is_finite() && *value > 0.0 => Some(*value),
        _ => None,
    }
}

/// A node's children. In jian, only Frame / Group / Rectangle declare a
/// `children` field at all; every other kind yields an empty slice. A
/// Frame / Group / Rectangle whose `children` is unset also yields an empty
/// slice — matching the TS `'children' in node ? … : []` guard.
pub fn children(node: &PenNode) -> &[PenNode] {
    let kids = match node {
        PenNode::Frame(n) => n.children.as_ref(),
        PenNode::Group(n) => n.children.as_ref(),
        PenNode::Rectangle(n) => n.children.as_ref(),
        PenNode::Tabs(n) => n.children.as_ref(),
        _ => None,
    };
    kids.map(Vec::as_slice).unwrap_or(&[])
}

/// A node's `role` (`PenNodeBase.role`), if set.
pub fn role(node: &PenNode) -> Option<&str> {
    base(node).role.as_deref()
}

/// A node's `rotation` in degrees. jian `rotation` is `Option<f64>`; a
/// missing value is `0.0`, matching the TS `typeof rotation === 'number'`
/// guard treating `undefined` as not-rotated.
pub fn rotation(node: &PenNode) -> f64 {
    base(node).rotation.unwrap_or(0.0)
}

/// A node's `opacity`. jian `opacity` is `Option<NumberOrExpression>`; a
/// missing value, or an `Expression` (`$variable` ref), resolves to the
/// fully-opaque default `1.0` — an unresolved expression is treated as
/// visible, matching the TS loose handling.
pub fn opacity(node: &PenNode) -> f64 {
    match &base(node).opacity {
        Some(NumberOrExpression::Number(n)) => *n,
        _ => 1.0,
    }
}

/// Port of `getFirstFillColor` (`detectors.ts:8-14`). Returns the first
/// fill's color string (raw — `$variable` refs included), or `None` when
/// the node has no `fill`, an empty `fill`, or a first fill with no color
/// (an `Image` fill, or a `Solid` fill with an empty-string color). Later
/// fills are ignored. The empty-string guard mirrors the TS truthiness
/// check `first.color &&`, which yields `null` for `""`.
pub fn first_fill_color(node: &PenNode) -> Option<&str> {
    let first = node_fills(node)?.first()?;
    match first {
        PenFill::Solid(body) if !body.color.is_empty() => Some(body.color.as_str()),
        _ => None,
    }
}

/// Port of `hasStroke` (`detectors.ts:17-21`). True when the node carries a
/// stroke whose thickness is `> 0` on any side. A missing stroke is `false`.
/// jian `StrokeThickness` is a 3-variant enum (T0 audit blocker **B1**), so
/// thickness is reduced via [`stroke_thickness_max`].
pub fn has_stroke(node: &PenNode) -> bool {
    match stroke(node) {
        Some(s) => stroke_thickness_max(&s.thickness) > 0.0,
        None => false,
    }
}

/// Max thickness across all sides of a [`StrokeThickness`] (T0 audit B1).
/// `Uniform` → its value; `PerSide` → max of the 4; `Sided` → max of the
/// present sides (a missing side is `0.0`).
pub fn stroke_thickness_max(thickness: &StrokeThickness) -> f32 {
    match thickness {
        StrokeThickness::Uniform(v) => *v,
        StrokeThickness::PerSide(sides) => sides.iter().copied().fold(0.0_f32, f32::max),
        StrokeThickness::Sided(s) => [s.top, s.right, s.bottom, s.left]
            .into_iter()
            .flatten()
            .fold(0.0_f32, f32::max),
    }
}

/// Port of pen-core `isNodeVisible` (`layout/engine.ts:44-49`):
/// `visible !== false && enabled !== false`.
///
/// T0 audit blocker **B3**: jian `visible` is `Option<bool>` and `enabled`
/// is `Option<BoolOrExpression>`. A node is hidden only when `visible` is
/// explicitly `Some(false)` or `enabled` is explicitly
/// `Some(BoolOrExpression::Bool(false))`. A `None` or an `enabled`
/// expression is treated as visible — matching the TS loose `!== false`.
pub fn is_node_visible(node: &PenNode) -> bool {
    let b = base(node);
    let visible_ok = b.visible != Some(false);
    let enabled_ok = !matches!(&b.enabled, Some(BoolOrExpression::Bool(false)));
    visible_ok && enabled_ok
}

/// Port of pen-core `getDefaultTheme` (`variables/resolve.ts`). Builds the
/// default theme map by taking the first value of each axis from a
/// `PenDocument.themes` map (axis name → ordered theme values). Empty axes
/// are skipped.
pub fn default_theme(themes: Option<&BTreeMap<String, Vec<String>>>) -> Theme {
    let mut result = Theme::new();
    let Some(themes) = themes else {
        return result;
    };
    for (axis, values) in themes {
        if let Some(first) = values.first() {
            result.insert(axis.clone(), first.clone());
        }
    }
    result
}

/// Port of pen-core `resolveColorRef` (`variables/resolve.ts`). When `raw`
/// is not a `$variable` ref it is returned as-is. When it is a ref, the
/// referenced variable is resolved against `variables` + `theme`; the
/// resolved string color is returned, or `None` if the variable is missing,
/// is non-string, or resolves to another ref (circular guard).
///
/// Deviation from the TS original: the TS `resolveVariableRef` consults a
/// generated `DEFAULT_PALETTE_FALLBACK` table when the variable is absent
/// from `variables`. That table (and its `semantic-palette` generator) is a
/// separate pen-core sub-module outside Task 1's scope; this port resolves
/// only against the document's own variable table. An un-seeded `$color-*`
/// ref therefore yields `None` here where the TS oracle may yield a
/// fallback hex — the golden-parity harness must account for this.
pub fn resolve_color_ref(raw: &str, variables: &Variables, theme: &Theme) -> Option<String> {
    if !is_variable_ref(raw) {
        return Some(raw.to_string());
    }
    match resolve_variable_ref(raw, variables, theme)? {
        VariableScalar::Str(s) => Some(s),
        VariableScalar::Bool(_) | VariableScalar::Num(_) => None,
    }
}

/// True when `value` is a `$variable` reference string. Mirrors TS
/// `isVariableRef`.
fn is_variable_ref(value: &str) -> bool {
    value.starts_with('$')
}

/// Resolve a single `$name` reference to its concrete scalar. Mirrors the
/// scalar path of TS `resolveVariableRef` (minus the palette fallback — see
/// [`resolve_color_ref`]). Returns `None` for an unknown variable or a
/// value that is itself a `$ref` (circular guard).
fn resolve_variable_ref(
    reference: &str,
    variables: &Variables,
    theme: &Theme,
) -> Option<VariableScalar> {
    let name = reference.strip_prefix('$')?;
    let def = variables.get(name)?;
    let resolved = match &def.value {
        VariableValue::Scalar(scalar) => scalar.clone(),
        VariableValue::Themed(values) => resolve_themed_value(values, theme)?,
    };
    // Circular guard — a resolved value that is itself a ref stops here.
    if let VariableScalar::Str(s) = &resolved {
        if is_variable_ref(s) {
            return None;
        }
    }
    Some(resolved)
}

/// Pick the concrete scalar from a themed-value list for `theme`. Mirrors TS
/// `resolveThemedValue`: a value whose `theme` map is a subset-match of the
/// active theme wins; otherwise the first entry is used.
fn resolve_themed_value(
    values: &[jian_ops_schema::variable::ThemedValue],
    theme: &Theme,
) -> Option<VariableScalar> {
    if !theme.is_empty() {
        let matched = values.iter().find(|v| {
            v.theme.as_ref().is_some_and(|vt| {
                theme
                    .iter()
                    .all(|(axis, expected)| vt.get(axis) == Some(expected))
            })
        });
        if let Some(m) = matched {
            return Some(m.value.clone());
        }
    }
    values.first().map(|v| v.value.clone())
}

/// Borrow the flattened `PenNodeBase` of any node kind.
fn base(node: &PenNode) -> &jian_ops_schema::node::PenNodeBase {
    match node {
        PenNode::Frame(n) => &n.base,
        PenNode::Group(n) => &n.base,
        PenNode::Rectangle(n) => &n.base,
        PenNode::Ellipse(n) => &n.base,
        PenNode::Line(n) => &n.base,
        PenNode::Polygon(n) => &n.base,
        PenNode::Path(n) => &n.base,
        PenNode::Text(n) => &n.base,
        PenNode::TextInput(n) => &n.base,
        PenNode::Image(n) => &n.base,
        PenNode::IconFont(n) => &n.base,
        PenNode::TextArea(n) => &n.base,
        PenNode::Select(n) => &n.base,
        PenNode::Switch(n) => &n.base,
        PenNode::Checkbox(n) => &n.base,
        PenNode::Slider(n) => &n.base,
        PenNode::RadioGroup(n) => &n.base,
        PenNode::NumberInput(n) => &n.base,
        PenNode::Progress(n) => &n.base,
        PenNode::Tabs(n) => &n.base,
        PenNode::Ref(n) => &n.base,
    }
}

/// Borrow a node's `fill` list across the kinds that carry one. `Image` and
/// `Line` have no fill; container kinds carry it via `ContainerProps`. Backs
/// `first_fill_color` and the typography detector's `first_solid_color`, which
/// needs the full fill list (gradient stops, fill-level opacity) rather than
/// just the first solid color.
pub fn node_fills(node: &PenNode) -> Option<&Vec<PenFill>> {
    match node {
        PenNode::Frame(n) => n.container.fill.as_ref(),
        PenNode::Group(n) => n.container.fill.as_ref(),
        PenNode::Rectangle(n) => n.container.fill.as_ref(),
        PenNode::Ellipse(n) => n.fill.as_ref(),
        PenNode::Polygon(n) => n.fill.as_ref(),
        PenNode::Path(n) => n.fill.as_ref(),
        PenNode::Text(n) => n.fill.as_ref(),
        PenNode::IconFont(n) => n.fill.as_ref(),
        PenNode::TextInput(n) => n.fill.as_ref(),
        PenNode::TextArea(n) => n.fill.as_ref(),
        PenNode::Select(n) => n.fill.as_ref(),
        PenNode::Switch(n) => n.fill.as_ref(),
        PenNode::Checkbox(n) => n.fill.as_ref(),
        PenNode::Slider(n) => n.fill.as_ref(),
        PenNode::RadioGroup(n) => n.fill.as_ref(),
        PenNode::NumberInput(n) => n.fill.as_ref(),
        PenNode::Progress(n) => n.fill.as_ref(),
        PenNode::Tabs(n) => n.fill.as_ref(),
        PenNode::Line(_) | PenNode::Image(_) | PenNode::Ref(_) => None,
    }
}

/// Borrow a node's `stroke` across the kinds that carry one. Container
/// kinds carry it via `ContainerProps`; `Text` / `Group` / `Image` do not.
fn stroke(node: &PenNode) -> Option<&jian_ops_schema::style::PenStroke> {
    match node {
        PenNode::Frame(n) => n.container.stroke.as_ref(),
        PenNode::Rectangle(n) => n.container.stroke.as_ref(),
        PenNode::Group(n) => n.container.stroke.as_ref(),
        PenNode::Ellipse(n) => n.stroke.as_ref(),
        PenNode::Line(n) => n.stroke.as_ref(),
        PenNode::Polygon(n) => n.stroke.as_ref(),
        PenNode::Path(n) => n.stroke.as_ref(),
        PenNode::IconFont(n) => n.stroke.as_ref(),
        PenNode::TextInput(n) => n.stroke.as_ref(),
        PenNode::TextArea(n) => n.stroke.as_ref(),
        PenNode::Select(n) => n.stroke.as_ref(),
        PenNode::Switch(n) => n.stroke.as_ref(),
        PenNode::Checkbox(n) => n.stroke.as_ref(),
        PenNode::Slider(n) => n.stroke.as_ref(),
        PenNode::RadioGroup(n) => n.stroke.as_ref(),
        PenNode::NumberInput(n) => n.stroke.as_ref(),
        PenNode::Progress(n) => n.stroke.as_ref(),
        PenNode::Tabs(n) => n.stroke.as_ref(),
        PenNode::Text(_) | PenNode::Image(_) | PenNode::Ref(_) => None,
    }
}

/// Encode an `f64` as a JSON number, preferring an integer encoding when the
/// value is integral so the wire shape matches the TS `number` exactly
/// (e.g. `rotation: 12` not `12.0`). Shared by the structure / text / siblings
/// detectors, all of which emit numeric `current_value` / `suggested_value`.
pub fn json_number(value: f64) -> Value {
    if value.fract() == 0.0 && value.is_finite() {
        Value::from(value as i64)
    } else {
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .unwrap_or(Value::Null)
    }
}

/// Read a named property off a node as a raw `serde_json::Value`, or `None`
/// when the node kind does not carry the field or the field is unset.
///
/// This backs `check_consistency` (`detectors.ts` `checkConsistency`), which
/// groups siblings by `JSON.stringify(node[property])`. The TS detector reads
/// the property loosely off any node; here the property name is mapped onto
/// the typed jian field and re-encoded to a `Value` so the group key matches
/// the TS canonical-JSON key. Recognised properties: `height`, `cornerRadius`,
/// `fontSize` — the three the strict/loose passes of `detectSiblingInconsistencies`
/// ever pass. An unrecognised name yields `None`.
///
/// The encoded value preserves the jian field's wire shape — a
/// `SizingBehavior::Number` re-encodes through [`json_number`] so an integral
/// height is `40` not `40.0`, matching the TS `JSON.stringify(40)`.
pub fn node_property_value(node: &PenNode, property: &str) -> Option<Value> {
    match property {
        "height" => sizing_value(height_sizing(node)?),
        "cornerRadius" => corner_radius_value(node),
        "fontSize" => match node {
            PenNode::Text(t) => t.font_size.map(json_number),
            _ => None,
        },
        _ => None,
    }
}

/// Borrow a node's `height` as a `SizingBehavior` — Frame/Group/Rectangle via
/// `ContainerProps`, `Text` via its own field. Other kinds have no height.
fn height_sizing(node: &PenNode) -> Option<&SizingBehavior> {
    match node {
        PenNode::Frame(n) => n.container.height.as_ref(),
        PenNode::Group(n) => n.container.height.as_ref(),
        PenNode::Rectangle(n) => n.container.height.as_ref(),
        PenNode::Text(n) => n.height.as_ref(),
        _ => None,
    }
}

/// Encode a `SizingBehavior` to its wire `Value`. A `Number` re-encodes
/// through [`json_number`]; `Keyword` / `Expression` serialize to strings.
fn sizing_value(sizing: &SizingBehavior) -> Option<Value> {
    match sizing {
        SizingBehavior::Number(n) => Some(json_number(*n)),
        other => serde_json::to_value(other).ok(),
    }
}

/// Encode a node's `corner_radius` to its wire `Value`. Frame/Group/Rectangle
/// and `Image` carry the `CornerRadius` enum; `Ellipse` / `Polygon` carry a
/// plain `Option<f64>`. Absent → `None`.
fn corner_radius_value(node: &PenNode) -> Option<Value> {
    match node {
        PenNode::Frame(n) => n.container.corner_radius.as_ref().map(corner_radius_json),
        PenNode::Group(n) => n.container.corner_radius.as_ref().map(corner_radius_json),
        PenNode::Rectangle(n) => n.container.corner_radius.as_ref().map(corner_radius_json),
        PenNode::Image(n) => n.corner_radius.as_ref().map(corner_radius_json),
        PenNode::Ellipse(n) => n.corner_radius.map(json_number),
        PenNode::Polygon(n) => n.corner_radius.map(json_number),
        _ => None,
    }
}

/// Encode a `CornerRadius`: `Uniform` through [`json_number`], `PerCorner`
/// as a 4-element JSON array.
fn corner_radius_json(radius: &CornerRadius) -> Value {
    match radius {
        CornerRadius::Uniform(v) => json_number(*v),
        CornerRadius::PerCorner(corners) => {
            Value::Array(corners.iter().map(|c| json_number(*c)).collect())
        }
    }
}

/// The numeric `cornerRadius` of a node, with TS's loose coercion: a
/// non-numeric value counts as `0`. jian `CornerRadius::Uniform` and the
/// plain `f64` (Ellipse/Polygon) unwrap to their value; `CornerRadius::PerCorner`
/// is NOT a TS `number` and counts as `0` (T0 audit footnote 7); an absent
/// `cornerRadius` is `0`. Backs `detect_mixed_sibling_corner_radius`.
pub fn corner_radius_numeric(node: &PenNode) -> f64 {
    let radius = match node {
        PenNode::Frame(n) => n.container.corner_radius.as_ref(),
        PenNode::Group(n) => n.container.corner_radius.as_ref(),
        PenNode::Rectangle(n) => n.container.corner_radius.as_ref(),
        PenNode::Image(n) => n.corner_radius.as_ref(),
        PenNode::Ellipse(n) => return n.corner_radius.unwrap_or(0.0),
        PenNode::Polygon(n) => return n.corner_radius.unwrap_or(0.0),
        _ => return 0.0,
    };
    match radius {
        Some(CornerRadius::Uniform(v)) => *v,
        // `PerCorner` is `typeof !== 'number'` in TS — counts as 0.
        Some(CornerRadius::PerCorner(_)) | None => 0.0,
    }
}

/// Borrow a node's `padding`. Only Frame/Group/Rectangle (`ContainerProps`)
/// carry it; every other kind yields `None`, matching the TS reading
/// `undefined` off a non-container sibling. Backs `detect_mixed_sibling_padding`.
pub fn padding(node: &PenNode) -> Option<&Padding> {
    match node {
        PenNode::Frame(n) => n.container.padding.as_ref(),
        PenNode::Group(n) => n.container.padding.as_ref(),
        PenNode::Rectangle(n) => n.container.padding.as_ref(),
        _ => None,
    }
}

/// A node's existing `padding` as raw JSON, or `Value::Null` — mirrors the
/// TS `currentValue: (s as { padding?: unknown }).padding`. Numeric components
/// route through [`json_number`] so an integral padding serializes as `20`
/// (a JSON integer), not `20.0` — a bare `serde_json::to_value` would re-emit
/// the float, which does NOT compare equal to the TS oracle's integer under
/// `serde_json::Value` equality. Shared by the siblings / spacing detectors.
pub fn raw_padding_value(padding: Option<&Padding>) -> Value {
    match padding {
        Some(Padding::Uniform(v)) => json_number(*v),
        Some(Padding::XY(xy)) => Value::Array(xy.iter().map(|v| json_number(*v)).collect()),
        Some(Padding::LtrB(ltrb)) => Value::Array(ltrb.iter().map(|v| json_number(*v)).collect()),
        Some(Padding::Expression(expr)) => Value::String(expr.clone()),
        None => Value::Null,
    }
}

/// Format an `f64` like the TS template-literal `${num}` — integral values
/// without a trailing `.0`.
pub fn fmt_num(value: f64) -> String {
    if value.fract() == 0.0 && value.is_finite() {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

#[cfg(test)]
#[path = "node_util_tests.rs"]
mod tests;
