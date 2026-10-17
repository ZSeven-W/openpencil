//! Conservative, read-only checks for undeclared initial local UI state.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use jian_core::signal::scheduler::Scheduler;
use jian_core::state::StateGraph;
use op_editor_core::EditorState;
use serde_json::Value;

#[path = "initial_binding_check_actions.rs"]
mod actions;
#[path = "initial_binding_check_expression.rs"]
mod expression;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InitialBindingFinding {
    pub node_id: String,
    pub property: String,
    pub expression: String,
    pub state_key: String,
    pub initial_value: Value,
    pub authored_value: Option<Value>,
}

/// Inspect only nodes owned by this run, but resolve declarations and possible
/// initialization against the entire assembled document (including append state).
/// No action, lifecycle, host service, or document mutation runs here.
pub(crate) fn missing_initial_bindings(
    state: &EditorState,
    owned_roots: &[String],
) -> Vec<InitialBindingFinding> {
    if owned_roots.is_empty() {
        return Vec::new();
    }
    let Ok(document) = serde_json::to_value(&state.doc) else {
        return Vec::new();
    };
    let graph = StateGraph::new(Rc::new(Scheduler::new()));
    if let Some(schema) = &state.doc.state {
        for (key, entry) in schema {
            graph.app_set(key, entry.default.clone().unwrap_or(Value::Null));
        }
    }
    if state
        .doc
        .logic_modules
        .as_ref()
        .is_some_and(|modules| !modules.is_empty())
    {
        return Vec::new();
    }
    let mut evidence = actions::WriteEvidence::default();
    evidence.lifecycle(&document, "onLaunch");
    let mut counts = HashMap::new();
    for node in document_nodes(&document) {
        scan_evidence(node, &mut evidence, &mut counts);
    }
    if let Some(pages) = document.get("pages").and_then(Value::as_array) {
        for page in pages {
            evidence.lifecycle(page, "onEnter")
        }
    }
    if evidence.opaque_initializer {
        return Vec::new();
    }
    let owned: HashSet<&str> = owned_roots
        .iter()
        .map(String::as_str)
        .filter(|id| counts.get(*id) == Some(&1))
        .collect();
    let mut findings = Vec::new();
    for node in document_nodes(&document) {
        inspect(
            node,
            false,
            &owned,
            &counts,
            &graph,
            &evidence,
            &mut findings,
        );
    }
    findings
}

fn document_nodes(document: &Value) -> impl Iterator<Item = &Value> {
    children(document).iter().chain(
        document
            .get("pages")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .flat_map(|page| children(page).iter()),
    )
}

fn scan_evidence(
    node: &Value,
    evidence: &mut actions::WriteEvidence,
    counts: &mut HashMap<String, usize>,
) {
    if let Some(id) = node.get("id").and_then(Value::as_str) {
        *counts.entry(id.to_owned()).or_default() += 1;
    }
    evidence.lifecycle(node, "onMount");
    evidence.events(node);
    for child in children(node) {
        scan_evidence(child, evidence, counts)
    }
}

fn inspect(
    node: &Value,
    inherited_ownership: bool,
    owned: &HashSet<&str>,
    counts: &HashMap<String, usize>,
    graph: &StateGraph,
    evidence: &actions::WriteEvidence,
    findings: &mut Vec<InitialBindingFinding>,
) {
    // Unknown visibility/scoped component projection cannot prove a first-view defect.
    if !initially_visible(node, graph) || node.get("type").and_then(Value::as_str) == Some("ref") {
        return;
    }
    let id = node.get("id").and_then(Value::as_str);
    if id.is_some_and(|id| evidence.uncertain_visibility.contains(id)) {
        return;
    }
    let owned_here = inherited_ownership || id.is_some_and(|id| owned.contains(id));
    if owned_here && id.is_some_and(|id| counts.get(id) == Some(&1)) {
        if let Some(bindings) = node.get("bindings").and_then(Value::as_object) {
            for (property, source) in bindings {
                // Geometry, visibility, input values and action-only expressions
                // have different initial/empty semantics. Keep this gate narrow.
                if !matches!(property.as_str(), "content" | "fill" | "stroke") {
                    continue;
                }
                let Some(source) = source.as_str() else {
                    continue;
                };
                let Some(key) = expression::unguarded_app_key(source) else {
                    continue;
                };
                if graph.app_get(&key).is_some()
                    || !evidence.user_keys.contains(&key)
                    || evidence.deferred_keys.contains(&key)
                {
                    continue;
                }
                let Some((initial_value, true)) = expression::initial_value(source, graph) else {
                    continue;
                };
                findings.push(InitialBindingFinding {
                    node_id: id.unwrap_or_default().to_owned(),
                    property: property.clone(),
                    expression: source.to_owned(),
                    state_key: key,
                    initial_value,
                    authored_value: node.get(property).cloned(),
                });
            }
        }
    }
    for child in children(node) {
        inspect(child, owned_here, owned, counts, graph, evidence, findings);
    }
}

fn initially_visible(node: &Value, graph: &StateGraph) -> bool {
    if node.get("visible").and_then(Value::as_bool) == Some(false) {
        return false;
    }
    if let Some(opacity) = node.get("opacity") {
        if opacity.as_f64().is_none_or(|opacity| opacity <= 0.0) {
            return false;
        }
    }
    if let Some(bindings) = node.get("bindings") {
        for property in ["visible", "opacity"] {
            if let Some(source) = bindings.get(property) {
                let Some(value) = source
                    .as_str()
                    .and_then(|source| expression::resolved_visibility(source, graph))
                else {
                    return false;
                };
                let visible = match property {
                    "visible" => value.as_bool() == Some(true),
                    _ => value.as_f64().is_some_and(|opacity| opacity > 0.0),
                };
                if !visible {
                    return false;
                }
            }
        }
    }
    true
}

fn children(node: &Value) -> &[Value] {
    node.get("children")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

#[cfg(test)]
#[path = "initial_binding_check_tests.rs"]
mod tests;
