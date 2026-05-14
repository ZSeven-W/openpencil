//! First-party MCP tools. Each tool is a snapshot-at-registration
//! struct: the host snapshots the relevant slice of the document on
//! every state change + re-registers the tool. `McpTool::call` then
//! formats the cached state without re-walking the tree.
//!
//! Pulled out of `mcp.rs` to honor the 800-line cap as the tool
//! surface grows. New first-party tools land in this file.

use std::collections::BTreeMap;

use super::{McpTool, ToolErrorCode, ToolOutcome};

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
    pub selected_id: u64,
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
        out.insert("selected_id".into(), self.selected_id.to_string());
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
    let selected_id = doc.selected.raw();
    if selected_id == 0 {
        return GetSelection {
            selected_id: 0,
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
    pub nodes: BTreeMap<u64, NodeRecord>,
}

#[derive(Debug, Clone)]
pub struct NodeRecord {
    pub kind: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub parent_id: u64,
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
        let id: u64 = match raw.parse() {
            Ok(n) => n,
            Err(_) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("node_id must be a decimal u64, got {raw:?}"),
                );
            }
        };
        let Some(rec) = self.nodes.get(&id) else {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("node {id} not found"),
            );
        };
        let mut out = BTreeMap::new();
        out.insert("kind".into(), rec.kind.clone());
        out.insert("name".into(), rec.name.clone());
        out.insert("x".into(), rec.x.to_string());
        out.insert("y".into(), rec.y.to_string());
        out.insert("width".into(), rec.width.to_string());
        out.insert("height".into(), rec.height.to_string());
        out.insert("parent_id".into(), rec.parent_id.to_string());
        out.insert("fill_ref".into(), rec.fill_ref.clone());
        out.insert("stroke_ref".into(), rec.stroke_ref.clone());
        ToolOutcome::Ok(out)
    }
}

pub fn get_node_snapshot(doc: &crate::document::Document) -> GetNode {
    let mut nodes: BTreeMap<u64, NodeRecord> = BTreeMap::new();
    for page in &doc.pages {
        for node in &page.children {
            walk_node(node, 0, &doc.var_table, &mut nodes);
        }
    }
    GetNode { nodes }
}

fn walk_node(
    node: &crate::document::Node,
    parent_id: u64,
    var_table: &crate::document::VariableTable,
    out: &mut BTreeMap<u64, NodeRecord>,
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
        node.id.raw(),
        NodeRecord {
            kind: kind_label.into(),
            name: node.name.clone(),
            x: bounds.origin.x as i32,
            y: bounds.origin.y as i32,
            width: bounds.size.x as i32,
            height: bounds.size.y as i32,
            parent_id,
            fill_ref,
            stroke_ref,
        },
    );
    for child in &node.children {
        walk_node(child, node.id.raw(), var_table, out);
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

/// Escape `\` / `;` / `|` so the encoded `name|kind|value` triplets
/// stay unambiguous regardless of variable content. Backslash-prefix
/// is the standard pattern — clients invert it by walking bytes and
/// promoting `\X` → `X` whenever they see an escape introducer.
fn escape_record_field(s: &str) -> String {
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
                    values.iter().map(|v| escape_record_field(v)).collect();
                format!("{}|{}", escape_record_field(axis), escaped_values.join(","))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn doc_with_variables() -> crate::document::Document {
        use crate::document::{Document, Variable, VariableKind, VariableScalar, VariableValue};
        let mut doc = Document::empty();
        doc.var_table.variables.push(Variable {
            name: "color-1".into(),
            kind: VariableKind::Color,
            value: VariableValue::Scalar(VariableScalar::Str("#ff0000".into())),
        });
        doc.var_table.variables.push(Variable {
            name: "spacing".into(),
            kind: VariableKind::Number,
            value: VariableValue::Scalar(VariableScalar::Num(16.0)),
        });
        doc.var_table.variables.push(Variable {
            name: "compact".into(),
            kind: VariableKind::Boolean,
            value: VariableValue::Scalar(VariableScalar::Bool(true)),
        });
        doc
    }

    #[test]
    fn list_variables_reports_count_and_records() {
        let doc = doc_with_variables();
        let tool = list_variables_snapshot(&doc);
        assert_eq!(tool.variables.len(), 3);
        assert_eq!(tool.variables[0].name, "color-1");
        assert_eq!(tool.variables[0].kind, "color");
        assert_eq!(tool.variables[0].value, "#ff0000");
        assert_eq!(tool.variables[1].kind, "number");
        assert_eq!(tool.variables[1].value, "16");
        assert_eq!(tool.variables[2].kind, "boolean");
        assert_eq!(tool.variables[2].value, "true");
    }

    #[test]
    fn list_variables_encodes_for_wire() {
        let doc = doc_with_variables();
        let tool = list_variables_snapshot(&doc);
        match tool.call(&BTreeMap::new()) {
            ToolOutcome::Ok(out) => {
                assert_eq!(out.get("count"), Some(&"3".to_string()));
                let encoded = out.get("variables").expect("variables field");
                assert!(encoded.contains("color-1|color|#ff0000"));
                assert!(encoded.contains("spacing|number|16"));
                assert!(encoded.contains("compact|boolean|true"));
                // Three records separated by `;`.
                assert_eq!(encoded.matches(';').count(), 2);
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn get_active_theme_reports_active_axes_and_options() {
        use crate::document::{Document, ThemeAxis};
        let mut doc = Document::empty();
        doc.var_table.themes.push(ThemeAxis {
            name: "mode".into(),
            values: vec!["light".into(), "dark".into(), "sepia".into()],
        });
        doc.var_table.themes.push(ThemeAxis {
            name: "density".into(),
            values: vec!["compact".into(), "comfortable".into()],
        });
        doc.var_table.set_active_theme("mode", "dark");
        // Only `mode` is in active_theme; `density` is defined but
        // not yet selected.
        let tool = get_active_theme_snapshot(&doc);
        match tool.call(&BTreeMap::new()) {
            ToolOutcome::Ok(out) => {
                assert_eq!(out.get("axis_count"), Some(&"2".to_string()));
                // Active selection: only mode|dark.
                assert_eq!(out.get("axes"), Some(&"mode|dark".to_string()));
                // Options carry both axes + full value lists.
                let opts = out.get("options").expect("options field");
                assert!(opts.contains("density|compact,comfortable"));
                assert!(opts.contains("mode|light,dark,sepia"));
                // Two records → one separator.
                assert_eq!(opts.matches(';').count(), 1);
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn get_active_theme_empty_document_is_zero() {
        let doc = crate::document::Document::empty();
        let tool = get_active_theme_snapshot(&doc);
        match tool.call(&BTreeMap::new()) {
            ToolOutcome::Ok(out) => {
                assert_eq!(out.get("axis_count"), Some(&"0".to_string()));
                assert_eq!(out.get("axes"), Some(&String::new()));
                assert_eq!(out.get("options"), Some(&String::new()));
            }
            _ => panic!("expected Ok"),
        }
    }

    #[test]
    fn get_active_theme_escapes_pipe_and_semicolon_in_values() {
        // Theme axis values with weird payloads (unusual but valid
        // per the canonical schema — values are strings). The
        // backslash-escape rules from list_variables apply here too.
        use crate::document::{Document, ThemeAxis};
        let mut doc = Document::empty();
        doc.var_table.themes.push(ThemeAxis {
            name: "weird".into(),
            values: vec!["a|b".into(), "c;d".into(), "e\\f".into()],
        });
        let tool = get_active_theme_snapshot(&doc);
        let opts = match tool.call(&BTreeMap::new()) {
            ToolOutcome::Ok(o) => o.get("options").unwrap().clone(),
            _ => panic!(),
        };
        // Pipe inside values is escaped → split on `|` yields exactly
        // 2 fields (axis | comma-joined-values).
        let mut depth_safe_split: Vec<String> = Vec::new();
        let mut cur = String::new();
        let mut chars = opts.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                if let Some(&n) = chars.peek() {
                    if matches!(n, '\\' | ';' | '|') {
                        cur.push(chars.next().unwrap());
                        continue;
                    }
                }
                cur.push(c);
            } else if c == '|' {
                depth_safe_split.push(std::mem::take(&mut cur));
            } else {
                cur.push(c);
            }
        }
        depth_safe_split.push(cur);
        assert_eq!(depth_safe_split.len(), 2, "expected axis|values, got {depth_safe_split:?}");
        assert_eq!(depth_safe_split[0], "weird");
        // Values comma-joined; each value individually escaped.
        // Decode the comma-separated values and verify each survived.
        let decoded: Vec<String> = depth_safe_split[1]
            .split(',')
            .map(unescape_record_field)
            .collect();
        assert_eq!(decoded, vec!["a|b", "c;d", "e\\f"]);
    }

    #[test]
    fn list_variables_empty_document_returns_zero_count() {
        let doc = crate::document::Document::empty();
        let tool = list_variables_snapshot(&doc);
        match tool.call(&BTreeMap::new()) {
            ToolOutcome::Ok(out) => {
                assert_eq!(out.get("count"), Some(&"0".to_string()));
                assert_eq!(out.get("variables"), Some(&String::new()));
            }
            _ => panic!("expected Ok"),
        }
    }

    #[test]
    fn escape_record_field_round_trips_pipe_semicolon_backslash() {
        // Codex stop-gate caught: the canonical `.op` schema does
        // NOT forbid `;`, `|`, or `\` in variable names or string
        // values. The encoding must round-trip every such payload.
        for raw in &[
            "plain",
            "a|b",
            "a;b",
            "a\\b",
            "a|b;c\\d",
            "\\",
            ";;|||",
            "",
            "color/primary",
            "label with space",
        ] {
            let escaped = escape_record_field(raw);
            let back = unescape_record_field(&escaped);
            assert_eq!(&back, raw, "round-trip failed for {raw:?}");
        }
    }

    #[test]
    fn list_variables_encodes_string_value_with_special_chars() {
        // String variable whose value contains every delimiter.
        // The wire format must keep the record boundary clear AND
        // recover the original payload on decode.
        use crate::document::{Document, Variable, VariableKind, VariableScalar, VariableValue};
        let mut doc = Document::empty();
        doc.var_table.variables.push(Variable {
            name: "msg".into(),
            kind: VariableKind::String,
            value: VariableValue::Scalar(VariableScalar::Str("a|b;c\\d".into())),
        });
        let tool = list_variables_snapshot(&doc);
        let out = match tool.call(&BTreeMap::new()) {
            ToolOutcome::Ok(o) => o,
            other => panic!("expected Ok, got {other:?}"),
        };
        assert_eq!(out.get("count"), Some(&"1".to_string()));
        let encoded = out.get("variables").expect("variables field");
        // Pipe inside the value is escaped, so splitting on `|`
        // still yields exactly 3 fields (name | kind | value).
        // Decode each field + verify the original payload.
        let mut fields: Vec<String> = Vec::new();
        let mut cur = String::new();
        let mut chars = encoded.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                if let Some(&n) = chars.peek() {
                    if matches!(n, '\\' | ';' | '|') {
                        cur.push(chars.next().unwrap());
                        continue;
                    }
                }
                cur.push(c);
            } else if c == '|' {
                fields.push(std::mem::take(&mut cur));
            } else {
                cur.push(c);
            }
        }
        fields.push(cur);
        assert_eq!(fields.len(), 3, "expected name|kind|value, got {fields:?}");
        assert_eq!(fields[0], "msg");
        assert_eq!(fields[1], "string");
        assert_eq!(fields[2], "a|b;c\\d");
    }
}
