//! Render-time `$variable` resolution — the canonical port of
//! `pen-core/src/variables/resolve.ts` (resolveNodeForCanvas) and
//! `pen-core/src/variables/replace-refs.ts` (replaceVariableRefsInTree).
//!
//! The scene builder calls [`resolve_document_for_canvas`] before the
//! jian layout pass so every `$token` in the tree — solid fill /
//! gradient-stop / stroke / shadow colours, opacity / gap / padding
//! expressions, font-weight keywords, and `$text` content — lands as
//! a concrete value the painters and the flex solver can consume.
//! Tokens missing from the document's variable table fall back to the
//! built-in semantic palette ([`DEFAULT_PALETTE_FALLBACK`]) so AI
//! "system mode" output renders sensibly on un-seeded documents (TS
//! spec §5.3).
//!
//! Stays wasm-clean: plain data walks over `jian_ops_schema` types.

use std::collections::BTreeMap;

use jian_ops_schema::node::base::NumberOrExpression;
use jian_ops_schema::node::container::Padding;
use jian_ops_schema::node::text::{FontWeight, TextContent};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::{PenEffect, PenFill};
use jian_ops_schema::variable::{VariableDefinition, VariableScalar, VariableValue};
use jian_ops_schema::PenDocument;

use crate::fills::{node_effects_opt_mut, node_fills_opt_mut, node_stroke_mut};
use crate::pen_node_ext::PenNodeExt;

type Vars = BTreeMap<String, VariableDefinition>;
type Theme = BTreeMap<String, String>;

/// Built-in fallback for the 56 semantic-palette tokens — generated
/// from `pen-core/src/variables/default-palette-fallback.ts` (which
/// derives it from `semantic-palette.ts`). Light/dark colour tokens
/// key off the `Mode` theme axis; chart colours and numeric
/// typography / spacing / radius tokens are mode-independent.
pub(crate) enum FallbackValue {
    LightDark {
        light: &'static str,
        dark: &'static str,
    },
    Single(&'static str),
    Num(f64),
}

pub(crate) const DEFAULT_PALETTE_FALLBACK: &[(&str, FallbackValue)] = &[
    (
        "color-accent",
        FallbackValue::LightDark {
            light: "#2563EB",
            dark: "#60A5FA",
        },
    ),
    (
        "color-bg-deep",
        FallbackValue::LightDark {
            light: "#F8FAFC",
            dark: "#0F172A",
        },
    ),
    (
        "color-border",
        FallbackValue::LightDark {
            light: "#E2E8F0",
            dark: "#334155",
        },
    ),
    (
        "color-border-strong",
        FallbackValue::LightDark {
            light: "#CBD5E1",
            dark: "#475569",
        },
    ),
    ("color-chart-1", FallbackValue::Single("#3B82F6")),
    ("color-chart-2", FallbackValue::Single("#8B5CF6")),
    ("color-chart-3", FallbackValue::Single("#EC4899")),
    ("color-chart-4", FallbackValue::Single("#14B8A6")),
    ("color-chart-5", FallbackValue::Single("#F59E0B")),
    ("color-chart-6", FallbackValue::Single("#F97316")),
    (
        "color-danger-bg",
        FallbackValue::LightDark {
            light: "#FEE2E2",
            dark: "#7F1D1D",
        },
    ),
    (
        "color-danger-text",
        FallbackValue::LightDark {
            light: "#991B1B",
            dark: "#FECACA",
        },
    ),
    (
        "color-destructive",
        FallbackValue::LightDark {
            light: "#EF4444",
            dark: "#F87171",
        },
    ),
    (
        "color-info-bg",
        FallbackValue::LightDark {
            light: "#DBEAFE",
            dark: "#1E3A8A",
        },
    ),
    (
        "color-info-text",
        FallbackValue::LightDark {
            light: "#1E40AF",
            dark: "#BFDBFE",
        },
    ),
    (
        "color-scrim",
        FallbackValue::LightDark {
            light: "#00000080",
            dark: "#00000099",
        },
    ),
    (
        "color-success",
        FallbackValue::LightDark {
            light: "#10B981",
            dark: "#34D399",
        },
    ),
    (
        "color-success-bg",
        FallbackValue::LightDark {
            light: "#DCFCE7",
            dark: "#14532D",
        },
    ),
    (
        "color-success-text",
        FallbackValue::LightDark {
            light: "#166534",
            dark: "#BBF7D0",
        },
    ),
    (
        "color-surface",
        FallbackValue::LightDark {
            light: "#FFFFFF",
            dark: "#1E293B",
        },
    ),
    (
        "color-surface-2",
        FallbackValue::LightDark {
            light: "#F1F5F9",
            dark: "#334155",
        },
    ),
    (
        "color-surface-3",
        FallbackValue::LightDark {
            light: "#F3F4F6",
            dark: "#475569",
        },
    ),
    (
        "color-text-body",
        FallbackValue::LightDark {
            light: "#334155",
            dark: "#CBD5E1",
        },
    ),
    (
        "color-text-muted",
        FallbackValue::LightDark {
            light: "#64748B",
            dark: "#94A3B8",
        },
    ),
    (
        "color-text-primary",
        FallbackValue::LightDark {
            light: "#0F172A",
            dark: "#F1F5F9",
        },
    ),
    (
        "color-text-subtle",
        FallbackValue::LightDark {
            light: "#94A3B8",
            dark: "#64748B",
        },
    ),
    (
        "color-warning-bg",
        FallbackValue::LightDark {
            light: "#FEF3C7",
            dark: "#78350F",
        },
    ),
    (
        "color-warning-text",
        FallbackValue::LightDark {
            light: "#92400E",
            dark: "#FDE68A",
        },
    ),
    ("radius-lg", FallbackValue::Num(12f64)),
    ("radius-md", FallbackValue::Num(8f64)),
    ("radius-sm", FallbackValue::Num(4f64)),
    ("spacing-1", FallbackValue::Num(4f64)),
    ("spacing-2", FallbackValue::Num(8f64)),
    ("spacing-3", FallbackValue::Num(12f64)),
    ("spacing-4", FallbackValue::Num(16f64)),
    ("spacing-5", FallbackValue::Num(24f64)),
    ("type-body-line-height", FallbackValue::Num(1.5f64)),
    ("type-body-size", FallbackValue::Num(14f64)),
    ("type-body-weight", FallbackValue::Num(400f64)),
    ("type-caption-line-height", FallbackValue::Num(1.4f64)),
    ("type-caption-size", FallbackValue::Num(12f64)),
    ("type-caption-weight", FallbackValue::Num(400f64)),
    ("type-display-letter-spacing", FallbackValue::Num(-0.5f64)),
    ("type-display-line-height", FallbackValue::Num(1f64)),
    ("type-display-size", FallbackValue::Num(64f64)),
    ("type-display-weight", FallbackValue::Num(700f64)),
    ("type-h1-line-height", FallbackValue::Num(1.2f64)),
    ("type-h1-size", FallbackValue::Num(24f64)),
    ("type-h1-weight", FallbackValue::Num(600f64)),
    ("type-h2-line-height", FallbackValue::Num(1.25f64)),
    ("type-h2-size", FallbackValue::Num(20f64)),
    ("type-h2-weight", FallbackValue::Num(600f64)),
    ("type-h3-line-height", FallbackValue::Num(1.3f64)),
    ("type-h3-size", FallbackValue::Num(16f64)),
    ("type-h3-weight", FallbackValue::Num(600f64)),
    (
        "type-uppercase-label-letter-spacing",
        FallbackValue::Num(1.5f64),
    ),
];

/// True when `value` is a `$variable` reference string.
pub fn is_variable_ref(value: &str) -> bool {
    value.starts_with('$') && value.len() > 1
}

/// Default theme map: first value per axis (TS `getDefaultTheme`).
pub fn default_theme(themes: Option<&BTreeMap<String, Vec<String>>>) -> Theme {
    let mut out = Theme::new();
    if let Some(themes) = themes {
        for (axis, values) in themes {
            if let Some(first) = values.first() {
                out.insert(axis.clone(), first.clone());
            }
        }
    }
    out
}

/// The theme map the renderer resolves against: the document's
/// default theme (first value per axis) with the user's transient
/// axis selections layered on top. Guarantees every axis has a value
/// even right after load, when the editor's `active_theme` is empty —
/// the fix for the "fully-themed value lists resolve to None until
/// the user picks an axis" gap.
pub fn effective_theme(doc: &PenDocument, active: &Theme) -> Theme {
    let mut theme = default_theme(doc.themes.as_ref());
    for (axis, value) in active {
        theme.insert(axis.clone(), value.clone());
    }
    theme
}

/// Pick the concrete scalar from a themed list for the given theme
/// (TS `resolveThemedValue`): the first entry whose theme map carries
/// every active axis at the active value, else the FIRST entry.
fn resolve_themed_value<'a>(
    values: &'a [jian_ops_schema::variable::ThemedValue],
    theme: &Theme,
) -> Option<&'a VariableScalar> {
    if !theme.is_empty() {
        let hit = values.iter().find(|v| {
            v.theme.as_ref().is_some_and(|t| {
                theme
                    .iter()
                    .all(|(axis, expected)| t.get(axis) == Some(expected))
            })
        });
        if let Some(hit) = hit {
            return Some(&hit.value);
        }
    }
    values.first().map(|v| &v.value)
}

/// Whether the built-in semantic palette can answer `name` on its own — a
/// reference the document's own table does not define is not necessarily
/// broken. Callers that repair broken references ask this first.
pub fn has_palette_fallback(name: &str) -> bool {
    DEFAULT_PALETTE_FALLBACK
        .iter()
        .any(|(token, _)| *token == name)
}

/// Resolve a single `$name` reference to its concrete scalar
/// (TS `resolveVariableRef`). Unknown tokens consult the built-in
/// semantic palette; a resolved value that is itself a `$ref` is a
/// circular guard miss.
pub fn resolve_variable_ref(
    reference: &str,
    vars: Option<&Vars>,
    theme: &Theme,
) -> Option<VariableScalar> {
    let name = reference.strip_prefix('$')?;
    let def = vars.and_then(|v| v.get(name));
    let Some(def) = def else {
        let (_, fallback) = DEFAULT_PALETTE_FALLBACK
            .iter()
            .find(|(token, _)| *token == name)?;
        return Some(match fallback {
            FallbackValue::Single(hex) => VariableScalar::Str((*hex).to_string()),
            FallbackValue::Num(n) => VariableScalar::Num(*n),
            FallbackValue::LightDark { light, dark } => {
                let mode = theme.get("Mode").map(String::as_str).unwrap_or("Light");
                let hex = if mode == "Dark" { dark } else { light };
                VariableScalar::Str((*hex).to_string())
            }
        });
    };
    let scalar = match &def.value {
        VariableValue::Scalar(s) => s,
        VariableValue::Themed(entries) => resolve_themed_value(entries, theme)?,
    };
    // Circular guard: a variable resolving to another `$ref` stops.
    if let VariableScalar::Str(s) = scalar {
        if is_variable_ref(s) {
            return None;
        }
    }
    Some(scalar.clone())
}

/// Resolve a colour string that may be a `$ref`; non-refs pass
/// through unchanged (TS `resolveColorRef`).
pub fn resolve_color_ref(color: &str, vars: Option<&Vars>, theme: &Theme) -> Option<String> {
    if !is_variable_ref(color) {
        return Some(color.to_string());
    }
    match resolve_variable_ref(color, vars, theme)? {
        VariableScalar::Str(s) => Some(s),
        _ => None,
    }
}

/// Resolve a numeric `$ref` to f64 (TS `resolveNumericRef`).
pub fn resolve_numeric_ref(reference: &str, vars: Option<&Vars>, theme: &Theme) -> Option<f64> {
    match resolve_variable_ref(reference, vars, theme)? {
        VariableScalar::Num(n) => Some(n),
        _ => None,
    }
}

fn resolve_fill(fill: &mut PenFill, vars: Option<&Vars>, theme: &Theme) -> bool {
    let mut changed = false;
    let fix = |color: &mut String| {
        if is_variable_ref(color) {
            // TS falls back to #000000 for an unresolvable fill ref.
            *color = resolve_color_ref(color, vars, theme).unwrap_or_else(|| "#000000".into());
            true
        } else {
            false
        }
    };
    match fill {
        PenFill::Solid(body) => changed |= fix(&mut body.color),
        PenFill::LinearGradient(body) => {
            for stop in &mut body.stops {
                changed |= fix(&mut stop.color);
            }
        }
        PenFill::RadialGradient(body) => {
            for stop in &mut body.stops {
                changed |= fix(&mut stop.color);
            }
        }
        _ => {}
    }
    changed
}

fn resolve_number_or_expression(
    value: &mut NumberOrExpression,
    vars: Option<&Vars>,
    theme: &Theme,
    fallback: f64,
) -> bool {
    if let NumberOrExpression::Expression(expr) = value {
        if is_variable_ref(expr) {
            let resolved = resolve_numeric_ref(expr, vars, theme).unwrap_or(fallback);
            *value = NumberOrExpression::Number(resolved);
            return true;
        }
    }
    false
}

/// Resolve every `$variable` reference in `node` (and its subtree)
/// in place — the TS `resolveNodeForCanvas` walk. Returns whether
/// anything changed.
pub fn resolve_node_for_canvas(node: &mut PenNode, vars: Option<&Vars>, theme: &Theme) -> bool {
    let mut changed = false;

    // Opacity — `NumberOrExpression` on the shared base.
    if let Some(opacity) = node.base_mut().opacity.as_mut() {
        changed |= resolve_number_or_expression(opacity, vars, theme, 1.0);
    }

    // Container gap + padding (Frame / Group / Rectangle).
    if let Some(container) = node_container_mut(node) {
        if let Some(gap) = container.gap.as_mut() {
            changed |= resolve_number_or_expression(gap, vars, theme, 0.0);
        }
        if let Some(Padding::Expression(expr)) = container.padding.as_ref() {
            if is_variable_ref(expr) {
                let resolved = resolve_numeric_ref(expr, vars, theme).unwrap_or(0.0);
                container.padding = Some(Padding::Uniform(resolved));
                changed = true;
            }
        }
    }

    // Fills — solid colours + gradient stops.
    if let Some(fills) = node_fills_opt_mut(node) {
        for fill in fills {
            changed |= resolve_fill(fill, vars, theme);
        }
    }

    // Stroke fill colours (thickness stays — the schema carries it
    // as concrete numbers only; a `$ref` thickness cannot round-trip
    // through `jian_ops_schema` today).
    if let Some(stroke_slot) = node_stroke_mut(node) {
        if let Some(stroke) = stroke_slot.as_mut() {
            if let Some(fills) = stroke.fill.as_mut() {
                for fill in fills {
                    changed |= resolve_fill(fill, vars, theme);
                }
            }
        }
    }

    // Effects — shadow colour (`offset/blur/spread` are concrete f32
    // in the schema; numeric effect refs land with the schema bump).
    if let Some(effects) = node_effects_opt_mut(node) {
        for effect in effects {
            if let PenEffect::Shadow(body) = effect {
                if is_variable_ref(&body.color) {
                    body.color = resolve_color_ref(&body.color, vars, theme)
                        .unwrap_or_else(|| "#000000".into());
                    changed = true;
                }
            }
        }
    }

    // Text — `$token` content, `$ref` font-weight keyword, styled
    // segment fills.
    if let PenNode::Text(text) = node {
        match &mut text.content {
            TextContent::Plain(content) => {
                if is_variable_ref(content) {
                    if let Some(VariableScalar::Str(resolved)) =
                        resolve_variable_ref(content, vars, theme)
                    {
                        *content = resolved;
                        changed = true;
                    }
                }
            }
            TextContent::Styled(segments) => {
                for segment in segments {
                    if let Some(fill) = segment.fill.as_mut() {
                        if is_variable_ref(fill) {
                            *fill = resolve_color_ref(fill, vars, theme)
                                .unwrap_or_else(|| "#000000".into());
                            changed = true;
                        }
                    }
                }
            }
        }
        if let Some(FontWeight::Keyword(keyword)) = text.font_weight.as_ref() {
            if is_variable_ref(keyword) {
                if let Some(weight) = resolve_numeric_ref(keyword, vars, theme) {
                    text.font_weight = Some(FontWeight::Number(weight.round().max(1.0) as u32));
                    changed = true;
                }
            }
        }
    }

    // Recurse.
    if let Some(children) = node.children_mut() {
        for child in children {
            changed |= resolve_node_for_canvas(child, vars, theme);
        }
    }
    changed
}

/// True when any `$token` exists anywhere `resolve_node_for_canvas`
/// would look — the scene builder's early-out so token-free documents
/// (the common case) skip the resolution pass's full-tree clone.
pub fn document_has_tokens(doc: &PenDocument) -> bool {
    fn fill_has_token(fill: &PenFill) -> bool {
        match fill {
            PenFill::Solid(body) => is_variable_ref(&body.color),
            PenFill::LinearGradient(body) => body.stops.iter().any(|s| is_variable_ref(&s.color)),
            PenFill::RadialGradient(body) => body.stops.iter().any(|s| is_variable_ref(&s.color)),
            _ => false,
        }
    }
    fn expr_has_token(value: &NumberOrExpression) -> bool {
        matches!(value, NumberOrExpression::Expression(e) if is_variable_ref(e))
    }
    fn container_has_token(container: &jian_ops_schema::node::container::ContainerProps) -> bool {
        container.gap.as_ref().is_some_and(expr_has_token)
            || matches!(&container.padding, Some(Padding::Expression(e)) if is_variable_ref(e))
    }
    fn node_has_token(node: &PenNode) -> bool {
        let base = crate::pen_node_ext::PenNodeExt::base(node);
        if base.opacity.as_ref().is_some_and(expr_has_token) {
            return true;
        }
        if let Some(fills) = crate::fills::node_fills(node) {
            if fills.iter().any(fill_has_token) {
                return true;
            }
        }
        if crate::fills::node_stroke_fills(node)
            .is_some_and(|fills| fills.iter().any(fill_has_token))
        {
            return true;
        }
        for effect in crate::fills::node_effects(node) {
            if let PenEffect::Shadow(body) = effect {
                if is_variable_ref(&body.color) {
                    return true;
                }
            }
        }
        let container_token = match node {
            PenNode::Frame(n) => container_has_token(&n.container),
            PenNode::Group(n) => container_has_token(&n.container),
            PenNode::Rectangle(n) => container_has_token(&n.container),
            _ => false,
        };
        if container_token {
            return true;
        }
        if let PenNode::Text(text) = node {
            let content_token = match &text.content {
                TextContent::Plain(content) => is_variable_ref(content),
                TextContent::Styled(segments) => segments
                    .iter()
                    .any(|s| s.fill.as_deref().is_some_and(is_variable_ref)),
            };
            if content_token
                || matches!(&text.font_weight, Some(FontWeight::Keyword(k)) if is_variable_ref(k))
            {
                return true;
            }
        }
        crate::pen_node_ext::PenNodeExt::children(node)
            .is_some_and(|children| children.iter().any(node_has_token))
    }
    doc.pages
        .as_ref()
        .is_some_and(|pages| pages.iter().any(|p| p.children.iter().any(node_has_token)))
        || doc.children.iter().any(node_has_token)
}

/// Clone `doc` with every `$variable` reference resolved against the
/// document's variable table under `active` (merged over the default
/// theme). The scene builder feeds the resolved clone to the layout +
/// paint pipeline so loaded documents render their `$refs` without
/// any transient editor cache.
pub fn resolve_document_for_canvas(doc: &PenDocument, active: &Theme) -> PenDocument {
    let theme = effective_theme(doc, active);
    let mut resolved = doc.clone();
    let vars = doc.variables.as_ref();
    if let Some(pages) = resolved.pages.as_mut() {
        for page in pages {
            for node in &mut page.children {
                resolve_node_for_canvas(node, vars, &theme);
            }
        }
    }
    for node in &mut resolved.children {
        resolve_node_for_canvas(node, vars, &theme);
    }
    resolved
}

/// Replace every `$old` token in the tree with `$new` (rename) or
/// its resolved concrete value (`new == None`, delete) — the TS
/// `replaceVariableRefsInTree` walk. Numeric refs that fail to
/// resolve keep the dangling token (TS keeps `val` likewise).
pub fn replace_variable_refs_in_tree(
    nodes: &mut [PenNode],
    old: &str,
    new: Option<&str>,
    vars: Option<&Vars>,
    theme: &Theme,
) {
    let token = format!("${old}");
    for node in nodes {
        replace_in_node(node, &token, new, vars, theme);
    }
}

fn replace_color(
    value: &mut String,
    token: &str,
    new: Option<&str>,
    vars: Option<&Vars>,
    theme: &Theme,
) {
    if value != token {
        return;
    }
    match new {
        Some(new) => *value = format!("${new}"),
        None => {
            if let Some(VariableScalar::Str(resolved)) = resolve_variable_ref(token, vars, theme) {
                *value = resolved;
            }
        }
    }
}

fn replace_in_node(
    node: &mut PenNode,
    token: &str,
    new: Option<&str>,
    vars: Option<&Vars>,
    theme: &Theme,
) {
    // Opacity / gap / padding expressions.
    let replace_expr = |value: &mut NumberOrExpression| {
        if let NumberOrExpression::Expression(expr) = value {
            if expr == token {
                match new {
                    Some(new) => *expr = format!("${new}"),
                    None => {
                        if let Some(n) = resolve_numeric_ref(token, vars, theme) {
                            *value = NumberOrExpression::Number(n);
                        }
                    }
                }
            }
        }
    };
    if let Some(opacity) = node.base_mut().opacity.as_mut() {
        replace_expr(opacity);
    }
    if let Some(container) = node_container_mut(node) {
        if let Some(gap) = container.gap.as_mut() {
            replace_expr(gap);
        }
        if let Some(Padding::Expression(expr)) = container.padding.as_mut() {
            if expr == token {
                match new {
                    Some(new) => *expr = format!("${new}"),
                    None => {
                        if let Some(n) = resolve_numeric_ref(token, vars, theme) {
                            container.padding = Some(Padding::Uniform(n));
                        }
                    }
                }
            }
        }
    }
    // Fills + stroke fills + shadow colours.
    if let Some(fills) = node_fills_opt_mut(node) {
        for fill in fills {
            replace_in_fill(fill, token, new, vars, theme);
        }
    }
    if let Some(stroke) = node_stroke_mut(node) {
        if let Some(stroke) = stroke.as_mut() {
            if let Some(fills) = stroke.fill.as_mut() {
                for fill in fills {
                    replace_in_fill(fill, token, new, vars, theme);
                }
            }
        }
    }
    if let Some(effects) = node_effects_opt_mut(node) {
        for effect in effects {
            if let PenEffect::Shadow(body) = effect {
                replace_color(&mut body.color, token, new, vars, theme);
            }
        }
    }
    // Text content + styled segment fills.
    if let PenNode::Text(text) = node {
        match &mut text.content {
            TextContent::Plain(content) => {
                if content == token {
                    match new {
                        Some(new) => *content = format!("${new}"),
                        None => {
                            if let Some(VariableScalar::Str(resolved)) =
                                resolve_variable_ref(token, vars, theme)
                            {
                                *content = resolved;
                            }
                        }
                    }
                }
            }
            TextContent::Styled(segments) => {
                for segment in segments {
                    if let Some(fill) = segment.fill.as_mut() {
                        replace_color(fill, token, new, vars, theme);
                    }
                }
            }
        }
    }
    if let Some(children) = node.children_mut() {
        for child in children {
            replace_in_node(child, token, new, vars, theme);
        }
    }
}

fn replace_in_fill(
    fill: &mut PenFill,
    token: &str,
    new: Option<&str>,
    vars: Option<&Vars>,
    theme: &Theme,
) {
    match fill {
        PenFill::Solid(body) => replace_color(&mut body.color, token, new, vars, theme),
        PenFill::LinearGradient(body) => {
            for stop in &mut body.stops {
                replace_color(&mut stop.color, token, new, vars, theme);
            }
        }
        PenFill::RadialGradient(body) => {
            for stop in &mut body.stops {
                replace_color(&mut stop.color, token, new, vars, theme);
            }
        }
        _ => {}
    }
}

/// Shared mutable access to the `ContainerProps` of the variants
/// that carry one (Frame / Group / Rectangle).
fn node_container_mut(
    node: &mut PenNode,
) -> Option<&mut jian_ops_schema::node::container::ContainerProps> {
    match node {
        PenNode::Frame(n) => Some(&mut n.container),
        PenNode::Group(n) => Some(&mut n.container),
        PenNode::Rectangle(n) => Some(&mut n.container),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jian_ops_schema::variable::{ThemedValue, VariableKind};

    fn color_var(value: VariableValue) -> VariableDefinition {
        VariableDefinition {
            kind: VariableKind::Color,
            value,
        }
    }

    fn vars_with(name: &str, def: VariableDefinition) -> Vars {
        let mut vars = Vars::new();
        vars.insert(name.to_string(), def);
        vars
    }

    fn doc_from(json: &str) -> PenDocument {
        jian_ops_schema::load_str(json)
            .expect("fixture parses")
            .value
    }

    #[test]
    fn scalar_ref_resolves_and_non_ref_passes_through() {
        let vars = vars_with(
            "brand",
            color_var(VariableValue::Scalar(VariableScalar::Str("#ff8800".into()))),
        );
        let theme = Theme::new();
        assert_eq!(
            resolve_color_ref("$brand", Some(&vars), &theme),
            Some("#ff8800".to_string())
        );
        assert_eq!(
            resolve_color_ref("#123456", Some(&vars), &theme),
            Some("#123456".to_string())
        );
    }

    #[test]
    fn themed_ref_matches_active_then_falls_back_to_first_entry() {
        let entries = vec![
            ThemedValue {
                theme: Some(BTreeMap::from([("Mode".to_string(), "Light".to_string())])),
                value: VariableScalar::Str("#ffffff".into()),
            },
            ThemedValue {
                theme: Some(BTreeMap::from([("Mode".to_string(), "Dark".to_string())])),
                value: VariableScalar::Str("#000000".into()),
            },
        ];
        let vars = vars_with("surface", color_var(VariableValue::Themed(entries)));
        let dark = Theme::from([("Mode".to_string(), "Dark".to_string())]);
        assert_eq!(
            resolve_color_ref("$surface", Some(&vars), &dark),
            Some("#000000".to_string())
        );
        // Empty active theme: fully-themed list still resolves to
        // the FIRST entry (the post-load state before any axis pick).
        assert_eq!(
            resolve_color_ref("$surface", Some(&vars), &Theme::new()),
            Some("#ffffff".to_string())
        );
    }

    #[test]
    fn unknown_token_falls_back_to_semantic_palette_per_mode() {
        let theme_light = Theme::new();
        assert_eq!(
            resolve_color_ref("$color-surface", None, &theme_light),
            Some("#FFFFFF".to_string())
        );
        let theme_dark = Theme::from([("Mode".to_string(), "Dark".to_string())]);
        assert_eq!(
            resolve_color_ref("$color-surface", None, &theme_dark),
            Some("#1E293B".to_string())
        );
        assert_eq!(
            resolve_numeric_ref("$spacing-2", None, &theme_light),
            Some(8.0)
        );
        assert_eq!(resolve_color_ref("$not-a-token", None, &theme_light), None);
    }

    #[test]
    fn nested_namespaced_ref_resolves_against_literal_keyed_var() {
        // Pencil emits namespaced tokens like `$surface/surface` and
        // keys its variable table with the same literal nested string.
        // `strip_prefix('$')` + a direct `vars.get("surface/surface")`
        // hit already resolves them — no slash special-casing needed.
        // This is the exact fill the converted Pencil cards carry, so
        // it guards the resolver against a future nested-token regress.
        let vars = vars_with(
            "surface/surface",
            color_var(VariableValue::Scalar(VariableScalar::Str("#f5f5f5".into()))),
        );
        let theme = Theme::new();
        assert_eq!(
            resolve_color_ref("$surface/surface", Some(&vars), &theme),
            Some("#f5f5f5".to_string())
        );
        // A flat token sharing no slash still resolves unchanged.
        let flat = vars_with(
            "accent",
            color_var(VariableValue::Scalar(VariableScalar::Str("#2563eb".into()))),
        );
        assert_eq!(
            resolve_color_ref("$accent", Some(&flat), &theme),
            Some("#2563eb".to_string())
        );
    }

    #[test]
    fn circular_ref_is_guarded() {
        let vars = vars_with(
            "loop",
            color_var(VariableValue::Scalar(VariableScalar::Str("$loop".into()))),
        );
        assert_eq!(resolve_color_ref("$loop", Some(&vars), &Theme::new()), None);
    }

    #[test]
    fn document_pass_resolves_loaded_fill_and_gap_refs() {
        // The exact P0 repro: a TS-authored doc whose fill is a
        // `$ref` string and whose gap is a `$spacing` token must
        // render concrete values with NO transient editor cache.
        let mut doc = doc_from(
            r##"{"version":"0.8.0","variables":{"brand":{"type":"color","value":"#ff8800"}},
                "children":[{"type":"frame","id":"f1","name":"f1","x":0,"y":0,"width":100,"height":50,
                  "fill":[{"type":"solid","color":"$brand"}],"layout":"vertical","gap":"$spacing-2",
                  "children":[{"type":"text","id":"t1","name":"t1","content":"$brand"}]}]}"##,
        );
        doc = resolve_document_for_canvas(&doc, &Theme::new());
        let PenNode::Frame(frame) = &doc.children[0] else {
            panic!("frame survives");
        };
        let Some(PenFill::Solid(body)) = frame.container.fill.as_ref().and_then(|f| f.first())
        else {
            panic!("solid fill survives");
        };
        assert_eq!(body.color, "#ff8800");
        assert_eq!(
            frame.container.gap,
            Some(NumberOrExpression::Number(8.0)),
            "palette spacing token resolves for the flex solver"
        );
        let Some(PenNode::Text(text)) = frame.children.as_ref().and_then(|c| c.first()) else {
            panic!("text child survives");
        };
        assert_eq!(text.content, TextContent::Plain("#ff8800".to_string()));
    }

    #[test]
    fn replace_refs_renames_tokens_and_freezes_values_on_delete() {
        let mut doc = doc_from(
            r##"{"version":"0.8.0","variables":{"brand":{"type":"color","value":"#ff8800"}},
                "children":[{"type":"rectangle","id":"r1","name":"r1","x":0,"y":0,"width":10,"height":10,
                  "fill":[{"type":"solid","color":"$brand"}]}]}"##,
        );
        let vars = doc.variables.clone();
        // Rename: `$brand` → `$primary`.
        replace_variable_refs_in_tree(
            &mut doc.children,
            "brand",
            Some("primary"),
            vars.as_ref(),
            &Theme::new(),
        );
        let fill_color = |doc: &PenDocument| -> String {
            let PenNode::Rectangle(r) = &doc.children[0] else {
                panic!("rect survives");
            };
            let Some(PenFill::Solid(body)) = r.container.fill.as_ref().and_then(|f| f.first())
            else {
                panic!("solid fill survives");
            };
            body.color.clone()
        };
        assert_eq!(fill_color(&doc), "$primary");
        // Delete (`new = None`): freeze the resolved concrete value.
        let mut renamed_vars = Vars::new();
        renamed_vars.insert(
            "primary".to_string(),
            color_var(VariableValue::Scalar(VariableScalar::Str("#ff8800".into()))),
        );
        replace_variable_refs_in_tree(
            &mut doc.children,
            "primary",
            None,
            Some(&renamed_vars),
            &Theme::new(),
        );
        assert_eq!(fill_color(&doc), "#ff8800");
    }

    #[test]
    fn effective_theme_layers_active_over_axis_defaults() {
        let doc = doc_from(
            r#"{"version":"0.8.0","themes":{"Mode":["Light","Dark"],"Density":["Comfort","Compact"]},"children":[]}"#,
        );
        let active = Theme::from([("Mode".to_string(), "Dark".to_string())]);
        let theme = effective_theme(&doc, &active);
        assert_eq!(theme.get("Mode"), Some(&"Dark".to_string()));
        assert_eq!(theme.get("Density"), Some(&"Comfort".to_string()));
    }
}
