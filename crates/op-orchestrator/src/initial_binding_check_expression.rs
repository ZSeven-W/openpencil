//! Conservative expression analysis for the initial binding state check.

use jian_core::expression::ast::{BinaryOp, Expr, ExprKind, TemplatePart, UnaryOp};
use jian_core::expression::{parser, DiagKind, Expression};
use jian_core::state::StateGraph;
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Default)]
struct AppExpression {
    keys: BTreeSet<String>,
    guarded: bool,
}

/// Only an explicit, direct app-state read is eligible for this check.
/// The parser represents `$app.key` as Member(ScopeRef("$app", []), "key").
fn direct_app_key(expression: &Expr) -> Option<&str> {
    let ExprKind::Member(base, key) = &expression.kind else {
        return None;
    };
    match &base.kind {
        ExprKind::ScopeRef(scope, path) if scope == "$app" && path.is_empty() => Some(key),
        _ => None,
    }
}

fn truthy_guard(expression: &Expr) -> bool {
    if direct_app_key(expression).is_some() {
        return true;
    }
    match &expression.kind {
        ExprKind::Unary(UnaryOp::Not, inner) => truthy_guard(inner),
        _ => false,
    }
}

fn null_guard(op: BinaryOp, left: &Expr, right: &Expr) -> bool {
    matches!(
        op,
        BinaryOp::Eq | BinaryOp::NotEq | BinaryOp::EqStrict | BinaryOp::NotEqStrict
    ) && ((direct_app_key(left).is_some() && matches!(&right.kind, ExprKind::Null))
        || (matches!(&left.kind, ExprKind::Null) && direct_app_key(right).is_some()))
}

/// Unknown parents invalidate the entire expression. In particular, do not
/// descend into a dynamic index or a nested member and report its app prefix.
fn inspect(expression: &Expr, result: &mut AppExpression) -> Option<()> {
    match &expression.kind {
        ExprKind::Number(_) | ExprKind::String(_) | ExprKind::Bool(_) | ExprKind::Null => {}
        ExprKind::Member(_, _) => {
            result.keys.insert(direct_app_key(expression)?.to_owned());
        }
        ExprKind::Unary(_, inner) => inspect(inner, result)?,
        ExprKind::Binary(op, left, right) => {
            result.guarded |= matches!(op, BinaryOp::Nullish | BinaryOp::Or | BinaryOp::And)
                || null_guard(*op, left, right);
            inspect(left, result)?;
            inspect(right, result)?;
        }
        ExprKind::Ternary(condition, then_branch, else_branch) => {
            // A comparison with a concrete option is not a truthy fallback:
            // `$app.tab == 'home' ? selected : normal` still needs an initial tab.
            result.guarded |= truthy_guard(condition);
            inspect(condition, result)?;
            inspect(then_branch, result)?;
            inspect(else_branch, result)?;
        }
        ExprKind::Template(parts) => {
            for part in parts {
                if let TemplatePart::Expr(inner) = part {
                    inspect(inner, result)?;
                }
            }
        }
        ExprKind::Array(items) => {
            for item in items {
                inspect(item, result)?;
            }
        }
        ExprKind::Object(entries) => {
            for (_, value) in entries {
                inspect(value, result)?;
            }
        }
        ExprKind::Identifier(_)
        | ExprKind::ScopeRef(_, _)
        | ExprKind::Index(_, _)
        | ExprKind::Call(_, _) => return None,
    }
    Some(())
}

fn app_expression(source: &str) -> Option<AppExpression> {
    let expression = parser::parse(source).ok()?;
    let mut result = AppExpression::default();
    inspect(&expression, &mut result)?;
    Some(result)
}

/// A pure app expression may use no keys or several keys, including guards.
/// Bare identifiers, other scopes, calls and indirect access are excluded.
pub(super) fn is_pure_app_expression(source: &str) -> bool {
    app_expression(source).is_some()
}

/// Find one unambiguous app key whose initial value has no explicit fallback.
/// Any guard anywhere in the expression conservatively suppresses the finding.
pub(super) fn unguarded_app_key(source: &str) -> Option<String> {
    let expression = app_expression(source)?;
    if expression.guarded || expression.keys.len() != 1 {
        return None;
    }
    expression.keys.into_iter().next()
}

/// Use the product evaluator, retaining only the expected missing-state warning.
/// The caller separately establishes that an eligible app key is undeclared.
pub(super) fn initial_value(source: &str, state: &StateGraph) -> Option<(Value, bool)> {
    let expression = Expression::compile(source).ok()?;
    let (value, warnings) = expression.eval(state, None, None);
    if warnings
        .iter()
        .any(|warning| warning.kind != DiagKind::UnknownIdentifier)
    {
        return None;
    }
    Some((value.0, !warnings.is_empty()))
}

/// Visibility is known only when every direct app read is declared, including
/// reads in branches that happen not to execute for the current state.
pub(super) fn resolved_visibility(source: &str, state: &StateGraph) -> Option<Value> {
    let expression = app_expression(source)?;
    if expression
        .keys
        .iter()
        .any(|key| state.app_get(key).is_none())
    {
        return None;
    }
    let (value, unknown_identifier) = initial_value(source, state)?;
    (!unknown_identifier).then_some(value)
}

#[cfg(test)]
#[path = "initial_binding_check_expression_tests.rs"]
mod tests;
