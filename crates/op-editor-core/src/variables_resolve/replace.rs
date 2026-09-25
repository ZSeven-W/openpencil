//! In-place `$variable` reference rewriting across a node tree — used
//! when a variable is renamed or deleted.

use super::*;

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
        PenFill::MeshGradient(body) => {
            for stop in &mut body.stops {
                replace_color(&mut stop.color, token, new, vars, theme);
            }
        }
        PenFill::Shader(body) => {
            for value in body.uniforms.iter_mut().flat_map(|u| u.values_mut()) {
                if let jian_ops_schema::style::ShaderUniformValue::Color(color) = value {
                    replace_color(color, token, new, vars, theme);
                }
            }
        }
        PenFill::Image(_) => {}
    }
}

/// Shared mutable access to the `ContainerProps` of the variants
/// that carry one (Frame / Group / Rectangle).
pub(super) fn node_container_mut(
    node: &mut PenNode,
) -> Option<&mut jian_ops_schema::node::container::ContainerProps> {
    match node {
        PenNode::Frame(n) => Some(&mut n.container),
        PenNode::Group(n) => Some(&mut n.container),
        PenNode::Rectangle(n) => Some(&mut n.container),
        _ => None,
    }
}
