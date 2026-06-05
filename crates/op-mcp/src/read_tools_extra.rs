//! Misc query/read MCP tools, split out of `read_tools.rs` to keep that file
//! under the repo 800-line cap: get_canvas_bounds / find_node_by_name /
//! get_node_parent / count_nodes / list_node_kinds / get_history_depth /
//! get_viewport / get_selection_set. These are self-contained snapshot tools
//! (no dependency on the layout/empty-space helpers that remain in
//! `read_tools.rs`).

use std::collections::BTreeMap;

use jian_ops_schema::node::PenNode;
use op_editor_core::geometry::aggregate_bounds;
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::EditorState;

use super::tools::{active_children, kind_label};
use super::{McpTool, ToolErrorCode, ToolOutcome};

/// First-party `get_canvas_bounds` tool — union bounding box of every
/// top-level node on the active page.
pub struct GetCanvasBounds {
    pub bounds: Option<(i32, i32, i32, i32)>,
}

impl McpTool for GetCanvasBounds {
    fn name(&self) -> &str {
        "get_canvas_bounds"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        match self.bounds {
            Some((x, y, w, h)) => {
                out.insert("x".into(), x.to_string());
                out.insert("y".into(), y.to_string());
                out.insert("w".into(), w.to_string());
                out.insert("h".into(), h.to_string());
                out.insert("has_content".into(), "true".into());
            }
            None => {
                out.insert("x".into(), "0".into());
                out.insert("y".into(), "0".into());
                out.insert("w".into(), "0".into());
                out.insert("h".into(), "0".into());
                out.insert("has_content".into(), "false".into());
            }
        }
        ToolOutcome::Ok(out)
    }
}

pub fn get_canvas_bounds_snapshot(state: &EditorState) -> GetCanvasBounds {
    let children = active_children(state);
    if children.is_empty() {
        return GetCanvasBounds { bounds: None };
    }
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for n in children {
        let b = aggregate_bounds(n);
        if b.is_empty() {
            continue;
        }
        min_x = min_x.min(b.x);
        min_y = min_y.min(b.y);
        max_x = max_x.max(b.x + b.w);
        max_y = max_y.max(b.y + b.h);
    }
    if !min_x.is_finite() {
        return GetCanvasBounds { bounds: None };
    }
    GetCanvasBounds {
        bounds: Some((
            min_x as i32,
            min_y as i32,
            (max_x - min_x) as i32,
            (max_y - min_y) as i32,
        )),
    }
}

// --- find_node_by_name -----------------------------------------------

/// First-party `find_node_by_name` tool — locate the first node whose
/// `name` matches the arg, anywhere on the active page.
pub struct FindNodeByName {
    pub index: Vec<(String, String, String)>,
}

impl McpTool for FindNodeByName {
    fn name(&self) -> &str {
        "find_node_by_name"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(query) = args.get("name") else {
            return ToolOutcome::Err(ToolErrorCode::MissingArgument, "name is required".into());
        };
        let Some((_, id, kind)) = self.index.iter().find(|(n, _, _)| n == query) else {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("no node named {query:?} on the active page"),
            );
        };
        let mut out = BTreeMap::new();
        out.insert("id".into(), id.to_string());
        out.insert("kind".into(), kind.clone());
        ToolOutcome::Ok(out)
    }
}

pub fn find_node_by_name_snapshot(state: &EditorState) -> FindNodeByName {
    fn walk(nodes: &[PenNode], out: &mut Vec<(String, String, String)>) {
        for n in nodes {
            out.push((
                n.base().name.clone().unwrap_or_default(),
                n.id_str().to_string(),
                kind_label(n).to_string(),
            ));
            if let Some(children) = n.children() {
                walk(children, out);
            }
        }
    }
    let mut index = Vec::new();
    walk(active_children(state), &mut index);
    FindNodeByName { index }
}

// --- get_node_parent -------------------------------------------------

/// First-party `get_node_parent` tool — parent id of `node_id` on the
/// active page.
pub struct GetNodeParent {
    pub index: Vec<(String, String, u32)>,
}

impl McpTool for GetNodeParent {
    fn name(&self) -> &str {
        "get_node_parent"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw) = args.get("node_id") else {
            return ToolOutcome::Err(ToolErrorCode::MissingArgument, "node_id is required".into());
        };
        let node_id: &str = raw.as_str();
        let Some((_, parent_id, depth)) = self.index.iter().find(|(id, _, _)| id == node_id) else {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("node {node_id} not found on active page"),
            );
        };
        let mut out = BTreeMap::new();
        out.insert("parent_id".into(), parent_id.to_string());
        out.insert("depth".into(), depth.to_string());
        ToolOutcome::Ok(out)
    }
}

pub fn get_node_parent_snapshot(state: &EditorState) -> GetNodeParent {
    fn walk(nodes: &[PenNode], parent: &str, depth: u32, out: &mut Vec<(String, String, u32)>) {
        for n in nodes {
            out.push((n.id_str().to_string(), parent.to_string(), depth));
            if let Some(children) = n.children() {
                walk(children, n.id_str(), depth + 1, out);
            }
        }
    }
    let mut index = Vec::new();
    walk(active_children(state), "", 0, &mut index);
    GetNodeParent { index }
}

// --- count_nodes -----------------------------------------------------

/// First-party `count_nodes` tool — total node count + per-page
/// breakdown.
pub struct CountNodes {
    pub per_page: Vec<u32>,
}

impl McpTool for CountNodes {
    fn name(&self) -> &str {
        "count_nodes"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let total: u32 = self.per_page.iter().sum();
        let encoded: Vec<String> = self
            .per_page
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{i}|{c}"))
            .collect();
        let mut out = BTreeMap::new();
        out.insert("total".into(), total.to_string());
        out.insert("per_page".into(), encoded.join(";"));
        ToolOutcome::Ok(out)
    }
}

pub fn count_nodes_snapshot(state: &EditorState) -> CountNodes {
    fn walk(nodes: &[PenNode]) -> u32 {
        nodes
            .iter()
            .map(|n| 1u32 + n.children().map(|c| walk(c)).unwrap_or(0))
            .sum()
    }
    let per_page = match state.doc.pages.as_ref() {
        Some(pages) => pages.iter().map(|p| walk(&p.children)).collect(),
        None => vec![walk(&state.doc.children)],
    };
    CountNodes { per_page }
}

// --- list_node_kinds -------------------------------------------------

/// First-party `list_node_kinds` tool — per-kind histogram of nodes on
/// the active page.
pub struct ListNodeKinds {
    pub histogram: Vec<(String, u32)>,
}

impl McpTool for ListNodeKinds {
    fn name(&self) -> &str {
        "list_node_kinds"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let encoded: Vec<String> = self
            .histogram
            .iter()
            .map(|(k, c)| format!("{k}|{c}"))
            .collect();
        let mut out = BTreeMap::new();
        out.insert("distinct".into(), self.histogram.len().to_string());
        out.insert("kinds".into(), encoded.join(";"));
        ToolOutcome::Ok(out)
    }
}

pub fn list_node_kinds_snapshot(state: &EditorState) -> ListNodeKinds {
    fn walk(nodes: &[PenNode], counts: &mut BTreeMap<&'static str, u32>) {
        for n in nodes {
            *counts.entry(kind_label(n)).or_insert(0) += 1;
            if let Some(children) = n.children() {
                walk(children, counts);
            }
        }
    }
    let mut counts: BTreeMap<&'static str, u32> = BTreeMap::new();
    walk(active_children(state), &mut counts);
    let histogram = counts
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    ListNodeKinds { histogram }
}

// --- get_history_depth -----------------------------------------------

/// First-party `get_history_depth` tool — undo + redo stack sizes.
pub struct GetHistoryDepth {
    pub past: usize,
    pub future: usize,
}

impl McpTool for GetHistoryDepth {
    fn name(&self) -> &str {
        "get_history_depth"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        out.insert("past".into(), self.past.to_string());
        out.insert("future".into(), self.future.to_string());
        ToolOutcome::Ok(out)
    }
}

pub fn get_history_depth_snapshot(state: &EditorState) -> GetHistoryDepth {
    GetHistoryDepth {
        past: state.history.past.len(),
        future: state.history.future.len(),
    }
}

// --- get_viewport ----------------------------------------------------

/// First-party `get_viewport` tool — current pan + zoom state.
pub struct GetViewport {
    pub pan_x: i32,
    pub pan_y: i32,
    pub zoom_percent: i32,
}

impl McpTool for GetViewport {
    fn name(&self) -> &str {
        "get_viewport"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        out.insert("pan_x".into(), self.pan_x.to_string());
        out.insert("pan_y".into(), self.pan_y.to_string());
        out.insert("zoom_percent".into(), self.zoom_percent.to_string());
        ToolOutcome::Ok(out)
    }
}

pub fn get_viewport_snapshot(state: &EditorState) -> GetViewport {
    let v = state.viewport;
    GetViewport {
        pan_x: v.pan_x as i32,
        pan_y: v.pan_y as i32,
        zoom_percent: (v.zoom * 100.0).round() as i32,
    }
}

// --- get_selection_set -----------------------------------------------

/// First-party `get_selection_set` tool — every id in the multi-select
/// set (vs `get_selection` which returns only the anchor).
pub struct GetSelectionSet {
    pub anchor: String,
    pub ids: Vec<String>,
}

impl McpTool for GetSelectionSet {
    fn name(&self) -> &str {
        "get_selection_set"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let encoded = self.ids.join(",");
        let mut out = BTreeMap::new();
        out.insert("count".into(), self.ids.len().to_string());
        out.insert("ids".into(), encoded);
        out.insert("anchor".into(), self.anchor.clone());
        ToolOutcome::Ok(out)
    }
}

pub fn get_selection_set_snapshot(state: &EditorState) -> GetSelectionSet {
    GetSelectionSet {
        anchor: state.selection.anchor.as_str().to_string(),
        ids: state
            .selection
            .set
            .iter()
            .map(|id| id.as_str().to_string())
            .collect(),
    }
}
