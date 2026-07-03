//! Compile + collect non-`bind:value` bindings on a promoted-document
//! tree, so [`super::PreviewSession::enter`] can re-evaluate them each
//! overlay pass (Task C2) and event writes (`set $app.*`) become
//! visible. Split out of `preview/mod.rs` to keep it under the repo's
//! 800-line-per-file cap.

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
    pub(super) prop: String,
    pub(super) expr: CompiledExpression,
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
    use op_editor_core::PenNodeExt;
    for node in nodes {
        if let Some(bindings) = node_bindings(node) {
            for (prop, expr) in bindings {
                if prop == "bind:value" {
                    continue;
                }
                match CompiledExpression::compile(&expr.0) {
                    Ok(compiled) => out.push(BindingSite {
                        node_id: node.id_str().to_string(),
                        prop: prop.clone(),
                        expr: compiled,
                    }),
                    Err(d) => warnings.push(format!(
                        "InvalidBinding: '{}' prop '{prop}': {d:?}",
                        node.id_str()
                    )),
                }
            }
        }
        if let Some(children) = node.children() {
            collect_binding_sites(children, out, warnings);
        }
    }
}
