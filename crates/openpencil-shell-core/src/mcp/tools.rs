//! First-party MCP tools. Each tool is a snapshot-at-registration
//! struct: the host snapshots the relevant slice of the document on
//! every state change + re-registers the tool. `McpTool::call` then
//! formats the cached state without re-walking the tree.
//!
//! Pulled out of `mcp.rs` to honor the 800-line cap as the tool
//! surface grows. New first-party tools land in this file.

use std::collections::BTreeMap;

use super::{McpCommand, McpTool, ToolErrorCode, ToolOutcome};

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

/// Snapshot the document into a `GetDocumentInfo` tool. Counts all
/// nodes recursively across every page. The MCP server registers
/// one of these per document; replays on every `get_document_info`
/// call without re-walking the tree.
pub fn document_info_snapshot(doc: &crate::document::Document) -> GetDocumentInfo {
    let total_nodes: usize = doc
        .pages
        .iter()
        .map(|p| p.children.iter().map(count_subtree).sum::<usize>())
        .sum();
    GetDocumentInfo {
        page_count: doc.pages.len(),
        active_page_index: doc.active_page_index,
        total_nodes,
    }
}

fn count_subtree(n: &crate::document::Node) -> usize {
    1 + n.children.iter().map(count_subtree).sum::<usize>()
}

/// First-party `get_selection` tool — reports the currently selected
/// node's id, kind, and bounds. Empty fields when nothing is
/// selected (id="0", kind="none"). Snapshot pattern matches
/// `GetDocumentInfo` so the host can re-register on selection
/// change.
pub struct GetSelection {
    /// The selected node's string id (the canonical `.op` schema
    /// id). Empty string when nothing is selected.
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

/// Snapshot the document selection into a `GetSelection` tool. When
/// nothing's selected returns an `id=0, kind="none"` placeholder so
/// LLM clients can distinguish "no selection" from a parse error.
pub fn selection_snapshot(doc: &crate::document::Document) -> GetSelection {
    let selected_id = doc.selected.as_str().to_string();
    if !doc.selected.is_real() {
        return GetSelection {
            selected_id,
            kind: "none".into(),
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        };
    }
    if let Some(node) = doc.selected_node() {
        let bounds = node.aggregate_bounds();
        let kind_label = match &node.kind {
            crate::document::NodeKind::Frame => "frame",
            crate::document::NodeKind::Group => "group",
            crate::document::NodeKind::Rect => "rect",
            crate::document::NodeKind::Ellipse => "ellipse",
            crate::document::NodeKind::Polygon => "polygon",
            crate::document::NodeKind::Line => "line",
            crate::document::NodeKind::Text => "text",
            crate::document::NodeKind::Path => "path",
            crate::document::NodeKind::Other(_) => "other",
        };
        GetSelection {
            selected_id,
            kind: kind_label.into(),
            x: bounds.origin.x as i32,
            y: bounds.origin.y as i32,
            width: bounds.size.x as i32,
            height: bounds.size.y as i32,
        }
    } else {
        GetSelection {
            selected_id,
            kind: "missing".into(),
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }
}

/// First-party `list_pages` tool — reports the page count, active
/// index, and a comma-separated list of page names. LLM clients use
/// this to pick a target page for `insert_node` / `batch_design`.
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

pub fn list_pages_snapshot(doc: &crate::document::Document) -> ListPages {
    let names = doc
        .pages
        .iter()
        .map(|p| p.name.clone())
        .collect::<Vec<_>>()
        .join(",");
    ListPages {
        page_count: doc.pages.len(),
        active_page_index: doc.active_page_index,
        names,
    }
}

/// First-party `get_node` tool — given a `node_id` argument
/// (decimal string), returns kind / bounds / parent. LLM clients
/// use this after `get_document_info` + `list_pages` to drill into
/// the specific node they're about to modify. The snapshot pattern
/// pre-computes a map of every node id → its details so calls are
/// O(1) lookup; the host re-registers on document mutations.
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
    /// String id of this node's parent (the canonical `.op` schema
    /// id). Empty string when the node is a top-level page child.
    pub parent_id: String,
    /// Name of the variable driving this node's fill colour at
    /// paint time, if any. Empty when the node's fill is a literal
    /// colour. LLM clients use this to decide whether to bump the
    /// variable (theme-wide change) or write a per-node override.
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
        // `node_id` is the canonical `.op` schema string id —
        // editor-minted ids look like `n12`, canonical loads keep
        // whatever the file authored. No numeric parse step.
        let Some(rec) = self.nodes.get(raw) else {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("node {raw} not found"),
            );
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

pub fn get_node_snapshot(doc: &crate::document::Document) -> GetNode {
    let mut nodes: BTreeMap<String, NodeRecord> = BTreeMap::new();
    for page in &doc.pages {
        for node in &page.children {
            walk_node(node, "", &doc.var_table, &mut nodes);
        }
    }
    GetNode { nodes }
}

fn walk_node(
    node: &crate::document::Node,
    parent_id: &str,
    var_table: &crate::document::VariableTable,
    out: &mut BTreeMap<String, NodeRecord>,
) {
    let bounds = node.aggregate_bounds();
    let kind_label = match &node.kind {
        crate::document::NodeKind::Frame => "frame",
        crate::document::NodeKind::Group => "group",
        crate::document::NodeKind::Rect => "rect",
        crate::document::NodeKind::Ellipse => "ellipse",
        crate::document::NodeKind::Polygon => "polygon",
        crate::document::NodeKind::Line => "line",
        crate::document::NodeKind::Text => "text",
        crate::document::NodeKind::Path => "path",
        crate::document::NodeKind::Other(_) => "other",
    };
    let fill_ref = var_table
        .fill_refs
        .get(&node.id)
        .cloned()
        .unwrap_or_default();
    let stroke_ref = var_table
        .stroke_refs
        .get(&node.id)
        .cloned()
        .unwrap_or_default();
    out.insert(
        node.id.as_str().to_string(),
        NodeRecord {
            kind: kind_label.into(),
            name: node.name.clone(),
            x: bounds.origin.x as i32,
            y: bounds.origin.y as i32,
            width: bounds.size.x as i32,
            height: bounds.size.y as i32,
            parent_id: parent_id.to_string(),
            fill_ref,
            stroke_ref,
        },
    );
    for child in &node.children {
        walk_node(child, node.id.as_str(), var_table, out);
    }
}

/// First-party `list_variables` tool — reports every variable in the
/// document's `var_table` along with its kind and resolved value
/// under the active theme. LLM clients use this after `get_document_
/// info` / `list_pages` to discover design tokens before issuing
/// `$ref`-bearing node mutations.
pub struct ListVariables {
    pub variables: Vec<VariableRecord>,
}

#[derive(Debug, Clone)]
pub struct VariableRecord {
    pub name: String,
    pub kind: String,
    /// Resolved scalar under the active theme. For Color kind this is
    /// the parsed hex; for Number / Boolean / String it's the literal
    /// stringified scalar. Empty when the variable doesn't resolve
    /// (themed entries with no matching active-theme axis).
    pub value: String,
}

impl McpTool for ListVariables {
    fn name(&self) -> &str {
        "list_variables"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        out.insert("count".into(), self.variables.len().to_string());
        // Encode the list as `name|kind|value` triplets joined by
        // `;`. The canonical `.op` schema doesn't actually forbid
        // `;` / `|` / `\` in variable names or string values
        // (codex stop-gate flagged my earlier claim), so escape
        // them: `\` → `\\`, `;` → `\;`, `|` → `\|`. Clients can
        // decode unambiguously by walking the bytes and treating
        // a `\` as the escape introducer. Empty list → empty
        // `variables` field (clients should consult `count`).
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

/// Escape `\` / `;` / `|` so two-level wire formats stay
/// unambiguous. Used by `list_variables` (`;`-separated records of
/// `|`-separated fields). Clients invert by walking bytes and
/// promoting `\X` → `X` whenever they see an escape introducer.
///
/// Comma is **not** in the set so the wire output stays
/// backward-compatible with clients that have been decoding
/// `list_variables` since the v1 schema. `get_active_theme` uses
/// a deeper escape set ([`escape_layered_field`]) because its
/// `options` field needs three-level separation (`;|,`).
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

/// Inverse of `escape_record_field`. Exposed for clients written in
/// Rust (the TS / Python MCP client side rolls its own decoder; this
/// is the canonical reference impl + the test fixture).
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

/// Three-level escape set: `\`, `;`, `|`, `,`. Used only by
/// `get_active_theme.options` which is `axis|v1,v2,v3;axis2|...`.
/// Decoding is layered: outer `;` split → middle `|` split →
/// inner `,` split, where each level unescapes only its own
/// delimiter (see the `layered_split` test helper).
fn escape_layered_field(s: &str) -> String {
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

pub fn list_variables_snapshot(doc: &crate::document::Document) -> ListVariables {
    use crate::document::{VariableKind, VariableScalar};
    let variables = doc
        .var_table
        .variables
        .iter()
        .map(|var| {
            let kind = match var.kind {
                VariableKind::Color => "color",
                VariableKind::Number => "number",
                VariableKind::Boolean => "boolean",
                VariableKind::String => "string",
            };
            let value = match var.resolve(&doc.var_table.active_theme) {
                Some(VariableScalar::Str(s)) => s.clone(),
                Some(VariableScalar::Num(n)) => format!("{n}"),
                Some(VariableScalar::Bool(b)) => if *b { "true" } else { "false" }.into(),
                None => String::new(),
            };
            VariableRecord {
                name: var.name.clone(),
                kind: kind.into(),
                value,
            }
        })
        .collect();
    ListVariables { variables }
}

/// First-party `get_active_theme` tool — reports the document's
/// current theme-axis selection AND the available options per axis.
/// LLM clients use this to know which axes can be flipped (per
/// `cycle_active_axis_value`) and what values they can be set to.
///
/// Wire shape:
///   axes  — `axis|value;axis2|value2` (active selection, with the
///           same backslash-escape rules as `list_variables`).
///   options — `axis|v1,v2,v3;axis2|v1,v2` (every axis defined in
///           `themes` + its full value list). `axis` appears here
///           even when not yet in `active_theme`, so clients can
///           seed it via `set_active_axis_value` (future write tool).
pub struct GetActiveTheme {
    pub active: Vec<(String, String)>,
    pub options: Vec<(String, Vec<String>)>,
}

impl McpTool for GetActiveTheme {
    fn name(&self) -> &str {
        "get_active_theme"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        // `axes` is a 2-level format (`;`-records of `|`-pairs) —
        // same shape as list_variables, so it MUST use the legacy
        // `escape_record_field` (3-char set) for wire compatibility.
        // Clients decode with the standard `unescape_record_field`
        // and would otherwise see stray `\,` sequences (codex stop-
        // gate fix).
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
        // `options` is a 3-level format (`;`-records of `|`-pairs
        // whose value side is `,`-joined). Only THIS field uses the
        // layered escape, because the inner `,` separator needs
        // protection from literal commas inside individual values.
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

pub fn get_active_theme_snapshot(doc: &crate::document::Document) -> GetActiveTheme {
    let active: Vec<(String, String)> = doc
        .var_table
        .active_theme
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let options: Vec<(String, Vec<String>)> = doc
        .var_table
        .themes
        .iter()
        .map(|t| (t.name.clone(), t.values.clone()))
        .collect();
    GetActiveTheme { active, options }
}

/// First-party `list_components` tool — reports the document's
/// registered components (saved Frames / Groups promoted via
/// "Save as Component"). LLM clients use this to discover what
/// reusable subtrees they can instantiate (instance insertion is
/// still UI-only in the Rust shell; an MCP write tool to spawn
/// instances is a future patch).
///
/// Wire shape:
///   count — total number of components in the library.
///   components — `;`-records of `name|id` pairs, with the legacy
///     2-level escape (\;|) on each side. Matches the
///     list_variables / list_pages wire convention so clients
///     can reuse their existing decoder.
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

pub fn list_components_snapshot(doc: &crate::document::Document) -> ListComponents {
    let items = doc
        .components
        .components
        .iter()
        .map(|c| (c.name.clone(), c.id.as_str().to_string()))
        .collect();
    ListComponents { items }
}

/// First-party `get_component` tool — fetches one component by id
/// with deeper detail than `list_components` (which only returns
/// names + ids). Returns the component's name, root node kind,
/// and the subtree's leaf count so LLM clients can size before
/// instantiating.
///
/// Wire shape:
///   args   — { "component_id": "<positive u64>" }
///   result — { name, kind, leaf_count } or ToolFailed when the
///            id doesn't resolve.
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
        // `component_id` is the canonical `.op` schema string id.
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

/// First-party `snapshot_layout` tool — return the bounding box
/// of every top-level node on the active page. LLM clients use
/// this to lay out new content without overlapping (combined
/// with future `find_empty_space`).
///
/// Wire shape:
///   layout — `;`-records of `id|x|y|w|h`, ints in doc px. Each
///     bbox is the resolved top-level node bounds (post-layout
///     for free-positioned nodes, post-engine for flex children
///     of the page root).
///   count — top-level node count.
pub struct SnapshotLayout {
    pub items: Vec<(String, i32, i32, i32, i32)>,
}

impl McpTool for SnapshotLayout {
    fn name(&self) -> &str {
        "snapshot_layout"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let encoded: Vec<String> = self
            .items
            .iter()
            .map(|(id, x, y, w, h)| format!("{id}|{x}|{y}|{w}|{h}"))
            .collect();
        let mut out = BTreeMap::new();
        out.insert("count".into(), self.items.len().to_string());
        out.insert("layout".into(), encoded.join(";"));
        ToolOutcome::Ok(out)
    }
}

/// First-party `get_canvas_bounds` tool — returns the union
/// bounding box of every top-level node on the active page.
/// LLM clients use this to know "where the design lives" so
/// they can place new content nearby (or `set_active_page` then
/// build elsewhere). Empty pages return all-zero.
///
/// Wire shape:
///   x / y / w / h — i32 doc px (zeros when the page is empty).
///   has_content — `"true"` / `"false"` so clients don't have
///     to interpret zeros as "empty vs origin-anchored 0x0".
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

/// First-party `find_node_by_name` tool — locate the first
/// node whose `name` matches the arg, anywhere in the active
/// page (top-level + descendants). Returns the matched node's
/// id + kind. LLM clients use this when the prompt references
/// a node by user-visible name rather than id ("update the
/// Title text…").
///
/// Wire shape:
///   args   — { "name": "<exact match, case-sensitive>" }
///   result — { id, kind } on success, ToolFailed when no match.
pub struct FindNodeByName {
    pub index: Vec<(String, String, String)>,
}

impl McpTool for FindNodeByName {
    fn name(&self) -> &str {
        "find_node_by_name"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(query) = args.get("name") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "name is required".into(),
            );
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

/// First-party `get_node_parent` tool — return the parent id
/// of `node_id` anywhere in the active page. Returns `parent_id`
/// = 0 when the node is at the page root. ToolFailed when the
/// id doesn't resolve. LLM clients use this to walk up from a
/// known leaf id to its container (e.g. find the Frame around
/// a Text node).
///
/// Wire shape:
///   args   — { "node_id": "<positive u64>" }
///   result — { parent_id, depth } on success; parent_id = "0"
///     when the node sits at the page root. depth is the
///     distance from page root (0 = top-level node).
pub struct GetNodeParent {
    pub index: Vec<(String, String, u32)>,
}

impl McpTool for GetNodeParent {
    fn name(&self) -> &str {
        "get_node_parent"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw) = args.get("node_id") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "node_id is required".into(),
            );
        };
        // `node_id` is the canonical `.op` schema string id.
        let node_id: &str = raw.as_str();
        let Some((_, parent_id, depth)) = self.index.iter().find(|(id, _, _)| id == node_id)
        else {
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

/// First-party `count_nodes` tool — return total node count
/// (top-level + every descendant) across all pages plus a
/// per-page breakdown. LLM clients use this for size estimation
/// before bulk operations.
///
/// Wire shape:
///   total — int, every node across every page.
///   per_page — `;`-records of `index|count` (0-based page idx).
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

/// First-party `list_node_kinds` tool — return a per-kind
/// histogram of nodes on the active page (top-level + every
/// descendant). LLM clients use this to discover whether a
/// design has e.g. any Text nodes before iterating.
///
/// Wire shape:
///   kinds — `;`-records of `kind|count` (kind in the standard
///     enum string set: frame / group / rect / ellipse /
///     polygon / line / text / path / other).
///   distinct — number of distinct kinds present.
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

/// First-party `get_history_depth` tool — return undo + redo
/// stack sizes. LLM clients use this to decide whether undo is
/// safe (or how many steps it'd take to roll back a recent
/// burst of writes).
///
/// Wire shape:
///   past — int undo stack depth (steps available to undo)
///   future — int redo stack depth (steps available to redo)
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

/// First-party `get_viewport` tool — return current pan + zoom
/// state. LLM clients use this to know where the user is
/// looking before suggesting `set_active_page` / inserts at
/// off-screen coordinates.
///
/// Wire shape:
///   pan_x / pan_y — i32 doc px (truncated from f32 for wire
///     stability; sub-pixel pan is invisible to LLM intent).
///   zoom — float * 100, int (so `1.0` → `100`, `1.5` → `150`).
///     Lossy but enough resolution for "what's the zoom level"
///     decisions; the actual Document stores f32.
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

/// First-party `get_selection_set` tool — return every id in
/// the multi-select set (vs `get_selection` which returns only
/// the anchor). LLM clients use this when the user pre-selects
/// a group and asks to "align these" — the prompt needs all
/// the ids, not just one.
///
/// Wire shape:
///   count — int multi-select size (0 when nothing selected).
///   ids — `,`-separated u64 ids in the selection order.
///   anchor — the single anchor id (matches get_selection's
///     `selected_id`, 0 when nothing selected).
pub struct GetSelectionSet {
    pub anchor: String,
    pub ids: Vec<String>,
}

impl McpTool for GetSelectionSet {
    fn name(&self) -> &str {
        "get_selection_set"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let encoded = self
            .ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let mut out = BTreeMap::new();
        out.insert("count".into(), self.ids.len().to_string());
        out.insert("ids".into(), encoded);
        out.insert("anchor".into(), self.anchor.to_string());
        ToolOutcome::Ok(out)
    }
}

pub fn get_selection_set_snapshot(doc: &crate::document::Document) -> GetSelectionSet {
    GetSelectionSet {
        anchor: doc.selected.as_str().to_string(),
        ids: doc.selected_set.iter().map(|id| id.as_str().to_string()).collect(),
    }
}

pub fn get_viewport_snapshot(doc: &crate::document::Document) -> GetViewport {
    let v = doc.viewport;
    GetViewport {
        pan_x: v.pan_x as i32,
        pan_y: v.pan_y as i32,
        zoom_percent: (v.zoom * 100.0).round() as i32,
    }
}

pub fn get_history_depth_snapshot(doc: &crate::document::Document) -> GetHistoryDepth {
    GetHistoryDepth {
        past: doc.history.past.len(),
        future: doc.history.future.len(),
    }
}

pub fn list_node_kinds_snapshot(doc: &crate::document::Document) -> ListNodeKinds {
    fn walk(nodes: &[crate::document::Node], counts: &mut BTreeMap<&'static str, u32>) {
        for n in nodes {
            let key: &'static str = match n.kind {
                crate::document::NodeKind::Frame => "frame",
                crate::document::NodeKind::Group => "group",
                crate::document::NodeKind::Rect => "rect",
                crate::document::NodeKind::Ellipse => "ellipse",
                crate::document::NodeKind::Polygon => "polygon",
                crate::document::NodeKind::Line => "line",
                crate::document::NodeKind::Text => "text",
                crate::document::NodeKind::Path => "path",
                _ => "other",
            };
            *counts.entry(key).or_insert(0) += 1;
            walk(&n.children, counts);
        }
    }
    let mut counts: BTreeMap<&'static str, u32> = BTreeMap::new();
    if let Some(page) = doc.active_page() {
        walk(&page.children, &mut counts);
    }
    let histogram = counts
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    ListNodeKinds { histogram }
}

pub fn count_nodes_snapshot(doc: &crate::document::Document) -> CountNodes {
    fn walk(nodes: &[crate::document::Node]) -> u32 {
        nodes
            .iter()
            .map(|n| 1u32 + walk(&n.children))
            .sum()
    }
    let per_page = doc
        .pages
        .iter()
        .map(|p| walk(&p.children))
        .collect();
    CountNodes { per_page }
}

pub fn get_node_parent_snapshot(doc: &crate::document::Document) -> GetNodeParent {
    fn walk(
        nodes: &[crate::document::Node],
        parent: &str,
        depth: u32,
        out: &mut Vec<(String, String, u32)>,
    ) {
        for n in nodes {
            out.push((n.id.as_str().to_string(), parent.to_string(), depth));
            walk(&n.children, n.id.as_str(), depth + 1, out);
        }
    }
    let mut index = Vec::new();
    if let Some(page) = doc.active_page() {
        walk(&page.children, "", 0, &mut index);
    }
    GetNodeParent { index }
}

pub fn find_node_by_name_snapshot(doc: &crate::document::Document) -> FindNodeByName {
    fn walk(
        nodes: &[crate::document::Node],
        out: &mut Vec<(String, String, String)>,
    ) {
        for n in nodes {
            let kind = match n.kind {
                crate::document::NodeKind::Frame => "frame",
                crate::document::NodeKind::Group => "group",
                crate::document::NodeKind::Rect => "rect",
                crate::document::NodeKind::Ellipse => "ellipse",
                crate::document::NodeKind::Polygon => "polygon",
                crate::document::NodeKind::Line => "line",
                crate::document::NodeKind::Text => "text",
                crate::document::NodeKind::Path => "path",
                _ => "other",
            };
            out.push((n.name.clone(), n.id.as_str().to_string(), kind.to_string()));
            walk(&n.children, out);
        }
    }
    let mut index = Vec::new();
    if let Some(page) = doc.active_page() {
        walk(&page.children, &mut index);
    }
    FindNodeByName { index }
}

pub fn get_canvas_bounds_snapshot(doc: &crate::document::Document) -> GetCanvasBounds {
    let Some(page) = doc.active_page() else {
        return GetCanvasBounds { bounds: None };
    };
    if page.children.is_empty() {
        return GetCanvasBounds { bounds: None };
    }
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    // Codex stop-gate: containers with ZERO own bounds, OR
    // containers with non-zero own bounds whose children paint
    // outside (no clip), both under-reported on the raw `n.bounds`
    // read. `visible_bounds_for_mcp` unions own + every descendant
    // so the LLM-visible extent matches what the canvas paints.
    for n in &page.children {
        let b = visible_bounds_for_mcp(n);
        if b.size.x <= 0.0 && b.size.y <= 0.0 {
            continue; // truly empty container — no descendants either
        }
        min_x = min_x.min(b.origin.x);
        min_y = min_y.min(b.origin.y);
        max_x = max_x.max(b.origin.x + b.size.x);
        max_y = max_y.max(b.origin.y + b.size.y);
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

/// Compute the LLM-visible extent of a node: union of its own
/// rect (if non-zero) AND every descendant's `visible_bounds`.
/// Mirrors what the canvas actually paints — `canvas_viewport`
/// renders Frame/Group children without clipping, so overflow
/// is part of the visible extent. Codex stop-gate fix for both
/// `snapshot_layout` and `get_canvas_bounds` under-reporting
/// when containers had ZERO own bounds or had overflow children.
fn visible_bounds_for_mcp(n: &crate::document::Node) -> crate::Rect {
    let mut union: Option<crate::Rect> = None;
    if n.bounds.size.x > 0.0 || n.bounds.size.y > 0.0 {
        union = Some(n.bounds);
    }
    for c in &n.children {
        let cb = visible_bounds_for_mcp(c);
        if cb.size.x <= 0.0 && cb.size.y <= 0.0 {
            continue;
        }
        union = Some(match union {
            None => cb,
            Some(u) => {
                let min_x = u.origin.x.min(cb.origin.x);
                let min_y = u.origin.y.min(cb.origin.y);
                let max_x = (u.origin.x + u.size.x).max(cb.origin.x + cb.size.x);
                let max_y = (u.origin.y + u.size.y).max(cb.origin.y + cb.size.y);
                crate::Rect::xywh(min_x, min_y, max_x - min_x, max_y - min_y)
            }
        });
    }
    union.unwrap_or(crate::Rect::ZERO)
}

pub fn snapshot_layout_snapshot(doc: &crate::document::Document) -> SnapshotLayout {
    let effective_bounds = visible_bounds_for_mcp;
    let items = doc
        .active_page()
        .map(|p| {
            p.children
                .iter()
                .map(|n| {
                    let b = effective_bounds(n);
                    (
                        n.id.as_str().to_string(),
                        b.origin.x as i32,
                        b.origin.y as i32,
                        b.size.x as i32,
                        b.size.y as i32,
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    SnapshotLayout { items }
}

pub fn get_component_snapshot(doc: &crate::document::Document) -> GetComponent {
    fn count_leaves(n: &crate::document::Node) -> usize {
        if n.children.is_empty() {
            1
        } else {
            n.children.iter().map(count_leaves).sum()
        }
    }
    let snapshot = doc
        .components
        .components
        .iter()
        .map(|c| {
            let kind = match c.root.kind {
                crate::document::NodeKind::Frame => "frame",
                crate::document::NodeKind::Group => "group",
                crate::document::NodeKind::Rect => "rect",
                crate::document::NodeKind::Ellipse => "ellipse",
                crate::document::NodeKind::Polygon => "polygon",
                crate::document::NodeKind::Line => "line",
                crate::document::NodeKind::Text => "text",
                crate::document::NodeKind::Path => "path",
                _ => "other",
            };
            (
                c.id.as_str().to_string(),
                c.name.clone(),
                kind.to_string(),
                count_leaves(&c.root),
            )
        })
        .collect();
    GetComponent { snapshot }
}
