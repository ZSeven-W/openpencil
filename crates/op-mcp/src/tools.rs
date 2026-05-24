//! First-party MCP read tools (part 1) + shared helpers.
//!
//! Each tool is a snapshot-at-registration struct: the host snapshots
//! the relevant slice of the editor state on every state change +
//! re-registers the tool. `McpTool::call` then formats the cached
//! state without re-walking the tree.
//!
//! Ported off shell-core's `Document` onto `op_editor_core::
//! EditorState` (canonical `PenDocument`). Carved at the 800-line cap:
//! the remaining read tools live in `read_tools.rs`.

use std::collections::BTreeMap;

use jian_ops_schema::node::PenNode;
use jian_ops_schema::variable::{VariableKind, VariableScalar};
use op_editor_core::geometry::{aggregate_bounds, DocRect};
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::{EditorState, NodeId};

use super::{McpTool, ToolErrorCode, ToolOutcome};

// --- Shared helpers --------------------------------------------------

/// Canonical lowercase label for a `PenNode` variant — the string set
/// every read tool emits + every write tool accepts.
pub(crate) fn kind_label(node: &PenNode) -> &'static str {
    match node {
        PenNode::Frame(_) => "frame",
        PenNode::Group(_) => "group",
        PenNode::Rectangle(_) => "rect",
        PenNode::Ellipse(_) => "ellipse",
        PenNode::Polygon(_) => "polygon",
        PenNode::Line(_) => "line",
        PenNode::Text(_) => "text",
        PenNode::Path(_) => "path",
        PenNode::TextInput(_) | PenNode::Image(_) | PenNode::IconFont(_) | PenNode::Ref(_) => {
            "other"
        }
    }
}

/// The active page's top-level children, threading the single-page
/// fallback (`pages == None` → root `children`).
pub(crate) fn active_children(state: &EditorState) -> &[PenNode] {
    state.active_children()
}

/// Truncate a `DocRect` to i32 doc-px (x, y, w, h).
fn rect_i32(r: DocRect) -> (i32, i32, i32, i32) {
    (r.x as i32, r.y as i32, r.w as i32, r.h as i32)
}

// --- get_document_info ----------------------------------------------

pub struct GetDocumentInfo {
    pub page_count: usize,
    pub active_page_index: usize,
    pub total_nodes: usize,
}

impl McpTool for GetDocumentInfo {
    fn name(&self) -> &str {
        "get_document_info"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        out.insert("page_count".into(), self.page_count.to_string());
        out.insert(
            "active_page_index".into(),
            self.active_page_index.to_string(),
        );
        out.insert("total_nodes".into(), self.total_nodes.to_string());
        ToolOutcome::Ok(out)
    }
}

fn count_subtree(n: &PenNode) -> usize {
    1 + n
        .children()
        .map(|c| c.iter().map(count_subtree).sum::<usize>())
        .unwrap_or(0)
}

/// Snapshot the document into a `GetDocumentInfo` tool. Counts all
/// nodes recursively across every page (or the single-page root).
pub fn document_info_snapshot(state: &EditorState) -> GetDocumentInfo {
    let total_nodes: usize = match state.doc.pages.as_ref() {
        Some(pages) => pages
            .iter()
            .map(|p| p.children.iter().map(count_subtree).sum::<usize>())
            .sum(),
        None => state.doc.children.iter().map(count_subtree).sum(),
    };
    GetDocumentInfo {
        page_count: state.page_count(),
        active_page_index: state.ui.active_page_index,
        total_nodes,
    }
}

// --- get_selection ---------------------------------------------------

/// First-party `get_selection` tool — reports the currently selected
/// node's id, kind, and bounds. Empty fields when nothing is selected.
pub struct GetSelection {
    pub selected_id: String,
    pub kind: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl McpTool for GetSelection {
    fn name(&self) -> &str {
        "get_selection"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        out.insert("selected_id".into(), self.selected_id.clone());
        out.insert("kind".into(), self.kind.clone());
        out.insert("x".into(), self.x.to_string());
        out.insert("y".into(), self.y.to_string());
        out.insert("width".into(), self.width.to_string());
        out.insert("height".into(), self.height.to_string());
        ToolOutcome::Ok(out)
    }
}

/// Snapshot the editor selection into a `GetSelection` tool.
pub fn selection_snapshot(state: &EditorState) -> GetSelection {
    let selected_id = state.selection.anchor.as_str().to_string();
    if !state.selection.anchor.is_real() {
        return GetSelection {
            selected_id,
            kind: "none".into(),
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        };
    }
    match state.selected_node() {
        Some(node) => {
            let (x, y, w, h) = rect_i32(aggregate_bounds(node));
            GetSelection {
                selected_id,
                kind: kind_label(node).into(),
                x,
                y,
                width: w,
                height: h,
            }
        }
        None => GetSelection {
            selected_id,
            kind: "missing".into(),
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        },
    }
}

// --- list_pages ------------------------------------------------------

/// First-party `list_pages` tool — page count, active index, and a
/// comma-separated list of page names.
pub struct ListPages {
    pub page_count: usize,
    pub active_page_index: usize,
    pub names: String,
}

impl McpTool for ListPages {
    fn name(&self) -> &str {
        "list_pages"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        out.insert("page_count".into(), self.page_count.to_string());
        out.insert(
            "active_page_index".into(),
            self.active_page_index.to_string(),
        );
        out.insert("names".into(), self.names.clone());
        ToolOutcome::Ok(out)
    }
}

pub fn list_pages_snapshot(state: &EditorState) -> ListPages {
    let names = match state.doc.pages.as_ref() {
        Some(pages) => pages
            .iter()
            .map(|p| p.name.clone())
            .collect::<Vec<_>>()
            .join(","),
        // Single-page fallback: one implicit "Page 1".
        None => "Page 1".to_string(),
    };
    ListPages {
        page_count: state.page_count(),
        active_page_index: state.ui.active_page_index,
        names,
    }
}

// --- get_node --------------------------------------------------------

/// First-party `get_node` tool — given a `node_id` argument (the
/// canonical `.op` string id), returns kind / bounds / parent. The
/// snapshot pattern pre-computes a map of every node id → its details.
pub struct GetNode {
    pub nodes: BTreeMap<String, NodeRecord>,
}

#[derive(Debug, Clone)]
pub struct NodeRecord {
    pub kind: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    /// String id of this node's parent. Empty when top-level.
    pub parent_id: String,
    /// Variable driving this node's fill colour, if any.
    pub fill_ref: String,
    /// Stroke parallel to `fill_ref`.
    pub stroke_ref: String,
}

impl McpTool for GetNode {
    fn name(&self) -> &str {
        "get_node"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let raw = match args.get("node_id") {
            Some(s) => s,
            None => {
                return ToolOutcome::Err(
                    ToolErrorCode::MissingArgument,
                    "node_id is required".into(),
                );
            }
        };
        let Some(rec) = self.nodes.get(raw) else {
            return ToolOutcome::Err(ToolErrorCode::ToolFailed, format!("node {raw} not found"));
        };
        let mut out = BTreeMap::new();
        out.insert("kind".into(), rec.kind.clone());
        out.insert("name".into(), rec.name.clone());
        out.insert("x".into(), rec.x.to_string());
        out.insert("y".into(), rec.y.to_string());
        out.insert("width".into(), rec.width.to_string());
        out.insert("height".into(), rec.height.to_string());
        out.insert("parent_id".into(), rec.parent_id.clone());
        out.insert("fill_ref".into(), rec.fill_ref.clone());
        out.insert("stroke_ref".into(), rec.stroke_ref.clone());
        ToolOutcome::Ok(out)
    }
}

pub fn get_node_snapshot(state: &EditorState) -> GetNode {
    let mut nodes: BTreeMap<String, NodeRecord> = BTreeMap::new();
    let fill_refs = &state.ui.variables.fill_refs;
    let stroke_refs = &state.ui.variables.stroke_refs;
    match state.doc.pages.as_ref() {
        Some(pages) => {
            for page in pages {
                for node in &page.children {
                    walk_node(node, "", fill_refs, stroke_refs, &mut nodes);
                }
            }
        }
        None => {
            for node in &state.doc.children {
                walk_node(node, "", fill_refs, stroke_refs, &mut nodes);
            }
        }
    }
    GetNode { nodes }
}

fn walk_node(
    node: &PenNode,
    parent_id: &str,
    fill_refs: &std::collections::HashMap<NodeId, String>,
    stroke_refs: &std::collections::HashMap<NodeId, String>,
    out: &mut BTreeMap<String, NodeRecord>,
) {
    let (x, y, w, h) = rect_i32(aggregate_bounds(node));
    let id = node.id_str().to_string();
    let (fill_ref, stroke_ref) = match NodeId::new_opt(id.as_str()) {
        Some(nid) => (
            fill_refs.get(&nid).cloned().unwrap_or_default(),
            stroke_refs.get(&nid).cloned().unwrap_or_default(),
        ),
        None => (String::new(), String::new()),
    };
    out.insert(
        id.clone(),
        NodeRecord {
            kind: kind_label(node).into(),
            name: node.base().name.clone().unwrap_or_default(),
            x,
            y,
            width: w,
            height: h,
            parent_id: parent_id.to_string(),
            fill_ref,
            stroke_ref,
        },
    );
    if let Some(children) = node.children() {
        for child in children {
            walk_node(child, &id, fill_refs, stroke_refs, out);
        }
    }
}

// --- list_variables --------------------------------------------------

/// First-party `list_variables` tool — every variable in the document
/// with its kind + resolved value under the active theme.
pub struct ListVariables {
    pub variables: Vec<VariableRecord>,
}

#[derive(Debug, Clone)]
pub struct VariableRecord {
    pub name: String,
    pub kind: String,
    pub value: String,
}

impl McpTool for ListVariables {
    fn name(&self) -> &str {
        "list_variables"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        out.insert("count".into(), self.variables.len().to_string());
        let encoded: Vec<String> = self
            .variables
            .iter()
            .map(|v| {
                format!(
                    "{}|{}|{}",
                    escape_record_field(&v.name),
                    escape_record_field(&v.kind),
                    escape_record_field(&v.value),
                )
            })
            .collect();
        out.insert("variables".into(), encoded.join(";"));
        ToolOutcome::Ok(out)
    }
}

/// Escape `\` / `;` / `|` so the two-level wire format stays
/// unambiguous. Used by `list_variables`.
pub(crate) fn escape_record_field(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str("\\;"),
            '|' => out.push_str("\\|"),
            c => out.push(c),
        }
    }
    out
}

/// Inverse of `escape_record_field`. Exposed for Rust clients.
pub fn unescape_record_field(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some(esc @ ('\\' | ';' | '|')) => out.push(esc),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Three-level escape set (`\`, `;`, `|`, `,`) — only `get_active_
/// theme.options` needs it.
pub(crate) fn escape_layered_field(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str("\\;"),
            '|' => out.push_str("\\|"),
            ',' => out.push_str("\\,"),
            c => out.push(c),
        }
    }
    out
}

pub fn list_variables_snapshot(state: &EditorState) -> ListVariables {
    let variables = state
        .doc
        .variables
        .as_ref()
        .map(|vars| {
            vars.iter()
                .map(|(name, def)| {
                    let kind = match def.kind {
                        VariableKind::Color => "color",
                        VariableKind::Number => "number",
                        VariableKind::Boolean => "boolean",
                        VariableKind::String => "string",
                    };
                    let value = match state.resolve_variable(name) {
                        Some(VariableScalar::Str(s)) => s.clone(),
                        Some(VariableScalar::Num(n)) => format!("{n}"),
                        Some(VariableScalar::Bool(b)) => if *b { "true" } else { "false" }.into(),
                        None => String::new(),
                    };
                    VariableRecord {
                        name: name.clone(),
                        kind: kind.into(),
                        value,
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    ListVariables { variables }
}

// --- get_active_theme ------------------------------------------------

/// First-party `get_active_theme` tool — current theme-axis selection
/// AND the available options per axis.
pub struct GetActiveTheme {
    pub active: Vec<(String, String)>,
    pub options: Vec<(String, Vec<String>)>,
}

impl McpTool for GetActiveTheme {
    fn name(&self) -> &str {
        "get_active_theme"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let active_encoded: Vec<String> = self
            .active
            .iter()
            .map(|(axis, value)| {
                format!(
                    "{}|{}",
                    escape_record_field(axis),
                    escape_record_field(value)
                )
            })
            .collect();
        let options_encoded: Vec<String> = self
            .options
            .iter()
            .map(|(axis, values)| {
                let escaped_values: Vec<String> =
                    values.iter().map(|v| escape_layered_field(v)).collect();
                format!(
                    "{}|{}",
                    escape_layered_field(axis),
                    escaped_values.join(",")
                )
            })
            .collect();
        let mut out = BTreeMap::new();
        out.insert("axes".into(), active_encoded.join(";"));
        out.insert("options".into(), options_encoded.join(";"));
        out.insert("axis_count".into(), self.options.len().to_string());
        ToolOutcome::Ok(out)
    }
}

pub fn get_active_theme_snapshot(state: &EditorState) -> GetActiveTheme {
    let active: Vec<(String, String)> = state
        .ui
        .variables
        .active_theme
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let options: Vec<(String, Vec<String>)> = state
        .doc
        .themes
        .as_ref()
        .map(|themes| {
            themes
                .iter()
                .map(|(name, values)| (name.clone(), values.clone()))
                .collect()
        })
        .unwrap_or_default();
    GetActiveTheme { active, options }
}

// --- list_components / get_component ---------------------------------

/// First-party `list_components` tool.
pub struct ListComponents {
    pub items: Vec<(String, String)>,
}

impl McpTool for ListComponents {
    fn name(&self) -> &str {
        "list_components"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let encoded: Vec<String> = self
            .items
            .iter()
            .map(|(name, id)| format!("{}|{}", escape_record_field(name), id))
            .collect();
        let mut out = BTreeMap::new();
        out.insert("count".into(), self.items.len().to_string());
        out.insert("components".into(), encoded.join(";"));
        ToolOutcome::Ok(out)
    }
}

pub fn list_components_snapshot(state: &EditorState) -> ListComponents {
    ListComponents {
        items: state
            .components
            .components
            .iter()
            .map(|c| (c.name.clone(), c.id.as_str().to_string()))
            .collect(),
    }
}

/// First-party `get_component` tool.
pub struct GetComponent {
    pub snapshot: Vec<(String, String, String, usize)>,
}

impl McpTool for GetComponent {
    fn name(&self) -> &str {
        "get_component"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw) = args.get("component_id") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "component_id is required".into(),
            );
        };
        let component_id: &str = raw.as_str();
        let Some((_, name, kind, leaf_count)) = self
            .snapshot
            .iter()
            .find(|(id, _, _, _)| id == component_id)
        else {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("component {component_id} not found"),
            );
        };
        let mut out = BTreeMap::new();
        out.insert("name".into(), name.clone());
        out.insert("kind".into(), kind.clone());
        out.insert("leaf_count".into(), leaf_count.to_string());
        ToolOutcome::Ok(out)
    }
}

pub fn get_component_snapshot(state: &EditorState) -> GetComponent {
    GetComponent {
        snapshot: state
            .components
            .components
            .iter()
            .map(|c| {
                (
                    c.id.as_str().to_string(),
                    c.name.clone(),
                    kind_label(&c.root).to_string(),
                    count_subtree(&c.root),
                )
            })
            .collect(),
    }
}

// The remaining read tools live in `read_tools.rs` (800-line cap).
pub use super::read_tools::{
    count_nodes_snapshot, find_node_by_name_snapshot, get_canvas_bounds_snapshot,
    get_history_depth_snapshot, get_node_parent_snapshot, get_selection_set_snapshot,
    get_viewport_snapshot, list_node_kinds_snapshot, snapshot_layout_snapshot, CountNodes,
    FindNodeByName, GetCanvasBounds, GetHistoryDepth, GetNodeParent, GetSelectionSet, GetViewport,
    ListNodeKinds, SnapshotLayout,
};
