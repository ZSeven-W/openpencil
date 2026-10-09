//! Render the initial state of owned table prototypes without modifying data.
use jian_ops_schema::node::PenNode;
use op_editor_core::PenNodeExt;
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn owned(nodes: &[PenNode]) -> bool {
    nodes.iter().any(|node| {
        node.base()
            .explain
            .as_deref()
            .is_some_and(|s| s.starts_with(op_editor_core::table_filter_contract::TABLE_MARKER))
            || node.children().is_some_and(|kids| owned(kids))
    })
}
fn evaluate(
    value: &mut Value,
    state: &jian_core::state::StateGraph,
    counters: &mut BTreeMap<String, String>,
) {
    if op_editor_core::table_filter_contract::is_row(value) {
        if let Some(source) = value["bindings"]["visible"].as_str() {
            if let Ok(expr) = jian_core::expression::Expression::compile(source) {
                let (result, warnings) = expr.eval(state, None, None);
                if warnings.is_empty() {
                    if let Some(visible) = result.as_bool() {
                        value["visible"] = json!(visible);
                    }
                }
            }
        }
    }
    if value["type"] == "text" {
        if let (Some(id), Some(expr)) =
            (value["id"].as_str(), value["bindings"]["content"].as_str())
        {
            if expr.starts_with("\"Local records ")
                || expr.starts_with("\"当前记录 ")
                || expr.starts_with("\"Rows on this page: ")
                || expr.starts_with("\"当前页数据：")
            {
                counters.insert(id.to_owned(), expr.to_owned());
            }
        }
    }
    if let Some(kids) = value["children"].as_array_mut() {
        for child in kids {
            evaluate(child, state, counters);
        }
    }
}
fn update_counters(value: &mut Value, counters: &BTreeMap<String, Value>) {
    if let Some(id) = value["id"].as_str() {
        if let Some(copy) = counters.get(id) {
            value["content"] = copy.clone();
        }
    }
    if let Some(kids) = value["children"].as_array_mut() {
        for child in kids {
            update_counters(child, counters);
        }
    }
}
/// Snapshot only generated table contracts. Unrelated bindings keep their
/// historical canvas semantics and professional editing retains every record.
pub(crate) fn roots(
    roots: &[PenNode],
    doc: &jian_ops_schema::PenDocument,
    paged: bool,
) -> Option<Vec<PenNode>> {
    if !owned(roots) {
        return None;
    }
    let state = jian_core::state::StateGraph::new(std::rc::Rc::new(
        jian_core::signal::scheduler::Scheduler::new(),
    ));
    if let Some(entries) = &doc.state {
        for (key, entry) in entries {
            if let Some(value) = &entry.default {
                state.app_set(key, value.clone());
            }
        }
    }
    let original = json!({"children":roots});
    let mut projected = original.clone();
    let mut expressions = BTreeMap::new();
    evaluate(&mut projected, &state, &mut expressions);
    op_editor_core::table_filter_contract::materialize(&mut projected);
    for (key, value) in op_editor_core::table_pagination::materialize(&mut projected, &|key| {
        state.app_get(key).map(|v| v.0)
    }) {
        state.app_set(&key, value);
    }
    let copies: BTreeMap<_, _> = expressions
        .into_iter()
        .filter_map(|(id, source)| {
            let expression = jian_core::expression::Expression::compile(&source).ok()?;
            let (copy, warnings) = expression.eval(&state, None, None);
            (warnings.is_empty() && copy.as_str().is_some()).then_some((id, copy.0))
        })
        .collect();
    let mut result = if paged { projected } else { original };
    update_counters(&mut result, &copies);
    serde_json::from_value(result["children"].clone()).ok()
}

#[cfg(test)]
#[path = "table_default_snapshot_tests.rs"]
mod tests;
