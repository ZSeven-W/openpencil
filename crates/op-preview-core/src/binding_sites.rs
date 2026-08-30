//! Compile + collect non-`bind:value` bindings on a promoted-document
//! tree, so [`super::PreviewSession::enter`] can re-evaluate them each
//! overlay pass (Task C2) and event writes (`set $app.*`) become
//! visible. Split out of `preview/mod.rs` to keep it under the repo's
//! 800-line-per-file cap.

use jian_core::binding::{BindingTarget, InvalidationKind};
use jian_core::expression::Expression as CompiledExpression;
use jian_ops_schema::node::PenNode;

/// One compiled non-`bind:value` binding on a promoted-document node.
/// Re-evaluated against the live state graph each overlay pass
/// (`PreviewSession::apply_binding_sites`) so event writes
/// (`set $app.*`) become visible. Spec-2 slice: only `content` is
/// applied to the scene today; other props are collected so the count
/// is honest but are skipped at apply time.
pub(super) struct BindingSite {
    pub(super) node_id: String,
    pub(super) target: BindingTarget,
    pub(super) expr: CompiledExpression,
    pub(super) uses_scroll: bool,
    pub(super) scroll_ancestor: Option<String>,
}

/// The node's authored `bindings` map, across all schema variants.
fn node_bindings(node: &PenNode) -> Option<&jian_ops_schema::events::Bindings> {
    match node {
        PenNode::Frame(n) => n.bindings.as_ref(),
        PenNode::Group(n) => n.bindings.as_ref(),
        PenNode::Rectangle(n) => n.bindings.as_ref(),
        PenNode::Ellipse(n) => n.bindings.as_ref(),
        PenNode::Line(n) => n.bindings.as_ref(),
        PenNode::Polygon(n) => n.bindings.as_ref(),
        PenNode::Path(n) => n.bindings.as_ref(),
        PenNode::Text(n) => n.bindings.as_ref(),
        PenNode::TextInput(n) => n.bindings.as_ref(),
        PenNode::TextArea(n) => n.bindings.as_ref(),
        PenNode::Select(n) => n.bindings.as_ref(),
        PenNode::Switch(n) => n.bindings.as_ref(),
        PenNode::Checkbox(n) => n.bindings.as_ref(),
        PenNode::Slider(n) => n.bindings.as_ref(),
        PenNode::RadioGroup(n) => n.bindings.as_ref(),
        PenNode::NumberInput(n) => n.bindings.as_ref(),
        PenNode::Progress(n) => n.bindings.as_ref(),
        PenNode::Tabs(n) => n.bindings.as_ref(),
        PenNode::Image(n) => n.bindings.as_ref(),
        PenNode::IconFont(n) => n.bindings.as_ref(),
        PenNode::Ref(n) => n.bindings.as_ref(),
    }
}

/// Recursively collect compilable bindings. `bind:value` is skipped —
/// the runtime itself owns two-way widget value sync; everything else
/// is re-evaluated in the overlay pass. Compile failures surface as
/// preview warnings, never as enter errors.
pub(super) fn collect_binding_sites(
    nodes: &[PenNode],
    out: &mut Vec<BindingSite>,
    warnings: &mut Vec<String>,
) {
    collect_binding_sites_under(nodes, None, out, warnings);
}

fn collect_binding_sites_under(
    nodes: &[PenNode],
    scroll_ancestor: Option<&str>,
    out: &mut Vec<BindingSite>,
    warnings: &mut Vec<String>,
) {
    use op_editor_core::PenNodeExt;
    for node in nodes {
        let node_id = node.id_str();
        let own_scroll = node_has_on_scroll(node).then_some(node_id);
        let nearest_scroll = own_scroll.or(scroll_ancestor);
        if let Some(bindings) = node_bindings(node) {
            for (property, expression) in bindings {
                if property == "bind:value" {
                    continue;
                }
                let Some(target) = BindingTarget::parse(property) else {
                    push_warning_once(
                        warnings,
                        format!("UnknownBindingTarget: '{node_id}' prop '{property}'"),
                    );
                    continue;
                };
                let uses_scroll = expression_uses_scroll(&expression.0);
                if uses_scroll && target.invalidation() != InvalidationKind::PaintOnly {
                    push_warning_once(
                        warnings,
                        format!("ScrollBindingRequiresPaintOnly: '{node_id}' prop '{property}'"),
                    );
                    continue;
                }
                if uses_scroll && nearest_scroll.is_none() {
                    push_warning_once(
                        warnings,
                        format!(
                            "MissingScrollAncestor: '{node_id}' prop '{property}' uses defaults"
                        ),
                    );
                }
                match CompiledExpression::compile(&expression.0) {
                    Ok(compiled) => out.push(BindingSite {
                        node_id: node_id.to_owned(),
                        target,
                        expr: compiled,
                        uses_scroll,
                        scroll_ancestor: nearest_scroll.map(str::to_owned),
                    }),
                    Err(diagnostic) => push_warning_once(
                        warnings,
                        format!("InvalidBinding: '{node_id}' prop '{property}': {diagnostic:?}"),
                    ),
                }
            }
        }
        if let Some(children) = node.children() {
            collect_binding_sites_under(children, nearest_scroll, out, warnings);
        }
    }
}

fn push_warning_once(warnings: &mut Vec<String>, warning: String) {
    if !warnings.contains(&warning) {
        warnings.push(warning);
    }
}

fn node_has_on_scroll(node: &PenNode) -> bool {
    serde_json::to_value(node)
        .ok()
        .and_then(|value| value.get("events").cloned())
        .and_then(|events| events.get("onScroll").cloned())
        .is_some_and(|handler| {
            handler
                .as_array()
                .is_some_and(|actions| !actions.is_empty())
        })
}

fn expression_uses_scroll(source: &str) -> bool {
    jian_core::expression::parser::parse(source)
        .is_ok_and(|expression| expression_node_uses_scroll(&expression))
}

fn expression_node_uses_scroll(expression: &jian_core::expression::ast::Expr) -> bool {
    use jian_core::expression::ast::{AccessPath, ExprKind, TemplatePart};
    match &expression.kind {
        ExprKind::ScopeRef(scope, access) => {
            scope == "$scroll"
                || access.iter().any(|part| match part {
                    AccessPath::Field(_) => false,
                    AccessPath::Index(index) => expression_node_uses_scroll(index),
                })
        }
        ExprKind::Array(items) => items.iter().any(expression_node_uses_scroll),
        ExprKind::Object(items) => items
            .iter()
            .any(|(_, value)| expression_node_uses_scroll(value)),
        ExprKind::Template(parts) => parts.iter().any(|part| match part {
            TemplatePart::Text(_) => false,
            TemplatePart::Expr(expression) => expression_node_uses_scroll(expression),
        }),
        ExprKind::Unary(_, value) | ExprKind::Member(value, _) => {
            expression_node_uses_scroll(value)
        }
        ExprKind::Binary(_, left, right) | ExprKind::Index(left, right) => {
            expression_node_uses_scroll(left) || expression_node_uses_scroll(right)
        }
        ExprKind::Ternary(condition, yes, no) => {
            expression_node_uses_scroll(condition)
                || expression_node_uses_scroll(yes)
                || expression_node_uses_scroll(no)
        }
        ExprKind::Call(callee, arguments) => {
            expression_node_uses_scroll(callee) || arguments.iter().any(expression_node_uses_scroll)
        }
        ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Bool(_)
        | ExprKind::Null
        | ExprKind::Identifier(_) => false,
    }
}
