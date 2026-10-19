//! Resolve deterministic text bindings for a static normal-mode presentation.
//! Never run events, lifecycle hooks, persisted state, or a Preview session.

use jian_core::{
    expression::{bytecode::OpCode, Expression},
    state::StateGraph,
};
use jian_ops_schema::{
    node::{text::TextContent, PenNode},
    PenDocument,
};
use op_editor_core::PenNodeExt;

pub(crate) fn apply(roots: &mut [PenNode], doc: &PenDocument, page_id: Option<&str>) {
    fn has_binding(nodes: &[PenNode]) -> bool {
        nodes.iter().any(|node| {
            matches!(node, PenNode::Text(text) if text.bindings.as_ref().is_some_and(|b| b.contains_key("content")))
                || node.children().is_some_and(|children| has_binding(children))
        })
    }
    if !has_binding(roots) {
        return;
    }
    // Seed exactly the same app/page/self defaults as Preview, without mounting it.
    let mut schema = doc.clone();
    schema.children = roots.to_vec();
    schema.pages = page_id.and_then(|id| {
        doc.pages.as_ref()?.iter().find(|p| p.id == id).map(|page| {
            let mut page = page.clone();
            page.children = roots.to_vec();
            vec![page]
        })
    });
    let state = StateGraph::new(std::rc::Rc::new(
        jian_core::signal::scheduler::Scheduler::new(),
    ));
    if jian_core::document::loader::build(schema, &state).is_err() {
        return;
    }
    visit(roots, &state, page_id);
}

fn visit(nodes: &mut [PenNode], state: &StateGraph, page_id: Option<&str>) {
    for node in nodes {
        if let PenNode::Text(text) = node {
            if let Some(source) = text.bindings.as_ref().and_then(|b| b.get("content")) {
                if let Ok(expr) = Expression::compile(&source.0) {
                    let known_inputs = expr.chunk.scope_paths.iter().all(|path| {
                        jian_core::state::StatePath::parse(path)
                            .ok()
                            .is_some_and(|path| {
                                state.resolve(&path, page_id, Some(&text.base.id)).is_some()
                            })
                    });
                    if deterministic(&expr) && known_inputs {
                        let (value, warnings) = expr.eval(state, page_id, Some(&text.base.id));
                        if warnings.is_empty() {
                            if let Some(value) = value.as_str() {
                                text.content = TextContent::Plain(value.to_owned());
                            }
                        }
                    }
                }
            }
        }
        if let Some(children) = node.children_mut() {
            visit(children, state, page_id);
        }
    }
}

fn deterministic(expr: &Expression) -> bool {
    expr.chunk.scope_paths.iter().all(|path| {
        matches!(
            path.split('.').next(),
            Some("$app" | "$state" | "$page" | "$self")
        )
    }) && expr.chunk.ops.iter().all(|op| match op {
        OpCode::CallBuiltin(index, _) => matches!(
            expr.chunk.strings.get(*index as usize).map(String::as_str),
            Some("to_str" | "to_num" | "round" | "floor" | "ceil" | "abs" | "min" | "max" | "len")
        ),
        _ => true,
    })
}

#[cfg(test)]
#[path = "initial_text_snapshot_tests.rs"]
mod tests;
