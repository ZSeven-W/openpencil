//! First-frame checks against original nodes, using the Preview expression semantics.

use super::*;
use jian_core::{
    expression::{ast::ExprKind, parser, Expression},
    signal::scheduler::Scheduler,
    state::{conformance::value_conforms, StateGraph},
};
use op_editor_core::PenNodeExt;
use std::rc::Rc;

pub(in crate::run::run_initial_binding_feedback) fn validate_initial_values(
    state: &EditorState,
    roots: &[String],
    findings: &[InitialBindingFinding],
) -> Result<(), RepairFailure> {
    let schema = state
        .doc
        .state
        .as_ref()
        .ok_or_else(|| failure("missing initial state"))?;
    let graph = StateGraph::new(Rc::new(Scheduler::new()));
    for (key, entry) in schema {
        graph.app_set(key, entry.default.clone().unwrap_or(Value::Null));
    }
    let nodes = owned_nodes(state, roots).ok_or(RepairFailure::Ownership)?;
    let mut selection_keys = BTreeSet::new();
    let mut visibly_selected_keys = BTreeSet::new();
    for finding in findings {
        let entry = schema
            .get(&finding.state_key)
            .ok_or_else(|| failure("missing initial declaration"))?;
        let default = entry
            .default
            .as_ref()
            .filter(|value| !value.is_null())
            .ok_or_else(|| failure("initial default must be explicit and non-null"))?;
        if !value_conforms(default, &entry.kind) {
            return Err(failure(
                "initial default does not conform to its declared type",
            ));
        }
        let expression = Expression::compile(&finding.expression)
            .map_err(|_| failure("original initial binding could not be evaluated"))?;
        let (value, warnings) = expression.eval(&graph, None, None);
        if !warnings.is_empty() || value.0.is_null() {
            return Err(failure(
                "original initial binding still has no usable value",
            ));
        }
        if finding.property == "content" {
            // Mirrors op-preview-core::scene_helpers::display_string: numbers and
            // booleans display as JSON text; compare display text, not JSON types.
            let displayed = value
                .0
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.0.to_string());
            let authored = finding
                .authored_value
                .clone()
                .and_then(|value| {
                    serde_json::from_value::<jian_ops_schema::node::TextContent>(value).ok()
                })
                .map(|content| match content {
                    jian_ops_schema::node::TextContent::Plain(text) => text,
                    jian_ops_schema::node::TextContent::Styled(segments) => {
                        segments.into_iter().map(|segment| segment.text).collect()
                    }
                })
                .ok_or_else(|| failure("original authored text could not be verified"))?;
            if authored != displayed {
                return Err(failure("initial default changed original authored copy"));
            }
        } else {
            selection_keys.insert(finding.state_key.as_str());
            if !has_literal_option(&nodes, &format!("$app.{}", finding.state_key), default) {
                // Unknown/conditional option schemes remain visibly pending.
                return Err(failure(
                    "initial selection is not a proven local literal option",
                ));
            }
            if value.0 != finding.initial_value {
                visibly_selected_keys.insert(finding.state_key.as_str());
            }
        }
    }
    if selection_keys != visibly_selected_keys {
        return Err(failure(
            "no original selection binding reflects the proposed default",
        ));
    }
    Ok(())
}

fn failure(message: &str) -> RepairFailure {
    RepairFailure::Candidate(message.into())
}

/// Only an enabled, direct tap assignment proves an available selection. Do not
/// speculate about async actions, conditional branches or dynamic set values.
fn has_literal_option(nodes: &[PenNode], target: &str, expected: &Value) -> bool {
    nodes.iter().any(|node| {
        let json = serde_json::to_value(node).unwrap_or(Value::Null);
        let events = &json["events"];
        let disabled = events["disabledEvents"]
            .as_array()
            .is_some_and(|keys| keys.iter().any(|key| key == "onTap"));
        let direct = !disabled
            && events["onTap"].as_array().is_some_and(|actions| {
                actions.len() == 1
                    && actions.iter().any(|action| {
                        let Some(body) = action
                            .as_object()
                            .filter(|action| action.len() == 1)
                            .and_then(|action| action.get("set"))
                        else {
                            return false;
                        };
                        let source = if body["target"].as_str() == Some(target) {
                            body.get("value")
                        } else {
                            body.get(target)
                        };
                        let Some(expression) = source
                            .and_then(Value::as_str)
                            .and_then(|source| parser::parse(source).ok())
                        else {
                            return false;
                        };
                        let literal = match expression.kind {
                            ExprKind::String(value) => Value::String(value),
                            ExprKind::Bool(value) => Value::Bool(value),
                            ExprKind::Number(value) => serde_json::json!(value),
                            _ => return false,
                        };
                        &literal == expected
                            || (literal.is_number()
                                && expected.is_number()
                                && literal.as_f64() == expected.as_f64())
                    })
            });
        direct
            || node
                .children()
                .is_some_and(|children| has_literal_option(children, target, expected))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_literal_options_compare_as_runtime_numbers() {
        let nodes: Vec<PenNode> = serde_json::from_value(serde_json::json!([
            {"type":"frame","id":"one","events":{"onTap":[{"set":{"$app.choice":"1"}}]}}
        ]))
        .unwrap();
        assert!(has_literal_option(
            &nodes,
            "$app.choice",
            &serde_json::json!(1)
        ));
        assert!(!has_literal_option(
            &nodes,
            "$app.choice",
            &serde_json::json!(2)
        ));
    }
}
