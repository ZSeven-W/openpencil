//! TS-compatible `open_document` metadata/context read tool.

use std::collections::BTreeMap;

use jian_ops_schema::node::PenNode;
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::EditorState;
use serde_json::{json, Value};

use super::{get_design_prompt_snapshot, McpTool, ToolOutcome};

const LIVE_CANVAS_PATH: &str = "live://canvas";

pub struct OpenDocument {
    document_json: String,
    context: String,
    design_prompt: String,
}

impl McpTool for OpenDocument {
    fn name(&self) -> &str {
        "open_document"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        let file_path = args
            .get("filePath")
            .filter(|path| !path.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| LIVE_CANVAS_PATH.into());
        out.insert("filePath".into(), file_path);
        out.insert("document".into(), self.document_json.clone());
        out.insert("context".into(), self.context.clone());
        out.insert("designPrompt".into(), self.design_prompt.clone());
        ToolOutcome::Ok(out)
    }
}

pub fn open_document_snapshot(state: &EditorState) -> OpenDocument {
    OpenDocument {
        document_json: build_document_json(state),
        context: build_document_context(state),
        design_prompt: build_open_design_prompt(state),
    }
}

fn build_document_json(state: &EditorState) -> String {
    let pages_value = state.doc.pages.as_ref().map(|pages| {
        Value::Array(
            pages
                .iter()
                .map(|page| {
                    json!({
                        "id": page.id,
                        "name": page.name,
                        "childCount": page.children.len(),
                    })
                })
                .collect(),
        )
    });
    let total_children = state
        .doc
        .pages
        .as_ref()
        .map(|pages| pages.iter().map(|p| p.children.len()).sum())
        .unwrap_or_else(|| state.doc.children.len());
    let mut document = json!({
        "version": state.doc.version,
        "childCount": total_children,
        "pageCount": state.page_count(),
        "hasVariables": state.doc.variables.as_ref().is_some_and(|v| !v.is_empty()),
        "hasThemes": state.doc.themes.as_ref().is_some_and(|t| !t.is_empty()),
    });
    if let Some(name) = &state.doc.name {
        document["name"] = Value::String(name.clone());
    }
    if let Some(pages) = pages_value {
        document["pages"] = pages;
    }
    serde_json::to_string(&document).unwrap_or_else(|_| "{}".into())
}

fn build_document_context(state: &EditorState) -> String {
    let roots = state.active_children();
    let mut nodes = Vec::new();
    collect_nodes(roots, &mut nodes);
    if nodes.is_empty() {
        return "Empty document. No existing nodes.".into();
    }
    let summary = nodes
        .iter()
        .take(20)
        .map(|node| {
            let name = node.base().name.as_deref().unwrap_or_else(|| node.id_str());
            format!("{}:{name}", node_type(node))
        })
        .collect::<Vec<_>>()
        .join(", ");
    let (width, height) = estimate_canvas_size(roots);
    [
        "DOCUMENT SUMMARY:".to_string(),
        format!("- Total nodes: {}", nodes.len()),
        format!("- Canvas size: {width}x{height}"),
        format!("- Nodes (first 20): {summary}"),
    ]
    .join("\n")
}

fn build_open_design_prompt(state: &EditorState) -> String {
    let roots = state.active_children();
    let total_children = state
        .doc
        .pages
        .as_ref()
        .map(|pages| pages.iter().map(|p| p.children.len()).sum())
        .unwrap_or_else(|| state.doc.children.len());
    let empty_or_empty_frame = total_children == 0
        || (roots.len() == 1
            && matches!(roots[0], PenNode::Frame(_))
            && roots[0].children().is_none_or(Vec::is_empty));
    if empty_or_empty_frame {
        let prompt_tool = get_design_prompt_snapshot(state);
        if let ToolOutcome::Ok(out) = prompt_tool.call(&BTreeMap::new()) {
            return out.get("designPrompt").cloned().unwrap_or_default();
        }
    }
    "Document has existing content. Match your action to the user intent:\n\
     - READ/INSPECT: Use batch_get (search by type/name/ID) or snapshot_layout to see what is on the canvas.\n\
     - DELETE/REMOVE: Use batch_get to find the target node ID, then delete_node to remove it.\n\
     - MODIFY: Use update_node to change properties of existing nodes.\n\
     - ADD NEW: Use batch_design or insert_node.\n\
     For complex multi-section designs, use the layered workflow: design_skeleton -> design_content -> design_refine."
        .into()
}

fn collect_nodes<'a>(nodes: &'a [PenNode], out: &mut Vec<&'a PenNode>) {
    for node in nodes {
        out.push(node);
        if let Some(children) = node.children() {
            collect_nodes(children, out);
        }
    }
}

fn estimate_canvas_size(nodes: &[PenNode]) -> (i32, i32) {
    for node in nodes {
        if matches!(node, PenNode::Frame(_)) {
            let width = node.width_px().unwrap_or(1200.0) as i32;
            let height = node.height_px().unwrap_or(800.0) as i32;
            if width <= 500 && height >= 700 {
                return (375, 812);
            }
            return (width, height);
        }
    }
    (1200, 800)
}

fn node_type(node: &PenNode) -> &'static str {
    match node {
        PenNode::Frame(_) => "frame",
        PenNode::Group(_) => "group",
        PenNode::Rectangle(_) => "rectangle",
        PenNode::Ellipse(_) => "ellipse",
        PenNode::Line(_) => "line",
        PenNode::Polygon(_) => "polygon",
        PenNode::Path(_) => "path",
        PenNode::Text(_) => "text",
        PenNode::TextInput(_) => "text_input",
        PenNode::Image(_) => "image",
        PenNode::IconFont(_) => "icon_font",
        PenNode::Ref(_) => "ref",
    }
}
