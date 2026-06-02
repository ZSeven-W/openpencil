//! `batch_design` write tool + nodes_json parser. Carved off
//! `write_tools.rs` to stay under the 800-line cap once the
//! batch surface landed.

use std::collections::BTreeMap;

use jian_ops_schema::node::PenNode;
use op_editor_core::{NodeId, PenNodeExt};

use super::batch_direct_ops::parse_single_direct_operation;
use super::write_tools::{validate_hex, ALLOWED_KINDS};
use super::{BatchInsertItem, EditorCommand, McpTool, ToolErrorCode, ToolOutcome};

/// First-party `batch_design` tool — insert N leaf nodes on the
/// active page in one atomic shot. Mirrors TS `batch_design` for
/// the leaf subset.
///
/// Wire shape: one scalar string arg `nodes_json` carrying a JSON
/// array of node descriptors. The shell-core parser rejects
/// structured args at the top level (so an LLM can't sneak a
/// nested object past scalar contracts), but a JSON array
/// embedded inside a quoted string round-trips cleanly. Each
/// array entry is `{"kind":"...","name":"...","x":N,"y":N,
/// "width":N,"height":N,"fill_hex":"#..."}` — the same shape
/// `insert_node` accepts, minus the wire wrapping.
///
/// The tool parses the inner JSON, validates EVERY entry, and
/// emits `McpCommand::BatchInsert { items: ... }`. The apply
/// path is all-or-nothing: a single bad entry rejects the whole
/// batch so the LLM never sees a partial design tree.
pub struct BatchDesign;

impl McpTool for BatchDesign {
    fn name(&self) -> &str {
        "batch_design"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        dispatch_batch_design(args, None)
    }
}

pub fn batch_design_snapshot() -> BatchDesign {
    BatchDesign
}

/// Shared core for `design_skeleton` / `design_content` /
/// `design_refine`. Each phase tool dispatches here with a label
/// stamped into the response so the LLM client can correlate the
/// call back to its layered-workflow phase. Today every phase
/// emits the same `BatchInsert` command — the phasing is purely
/// metadata. A future patch may grow per-phase apply semantics
/// (e.g. `design_refine` patching existing nodes via UpdateNode
/// batches) once a richer command exists.
fn dispatch_phase(args: &BTreeMap<String, String>, phase: &'static str) -> ToolOutcome {
    dispatch_batch_design(args, Some(phase))
}

fn dispatch_batch_design(
    args: &BTreeMap<String, String>,
    phase: Option<&'static str>,
) -> ToolOutcome {
    if let Some(operations) = args.get("operations") {
        return match parse_operations(operations) {
            Ok(ParsedOperations::Insert {
                parent_id,
                nodes,
                count,
            }) => {
                let mut out = BTreeMap::new();
                out.insert("wrote".into(), "true".into());
                out.insert("count".into(), count.to_string());
                if let Some(phase) = phase {
                    out.insert("phase".into(), phase.into());
                }
                ToolOutcome::OkWithCommand(out, EditorCommand::InsertSubtree { nodes, parent_id })
            }
            Ok(ParsedOperations::Direct(command)) => {
                let mut out = BTreeMap::new();
                out.insert("wrote".into(), "true".into());
                out.insert("count".into(), "1".into());
                if let Some(phase) = phase {
                    out.insert("phase".into(), phase.into());
                }
                ToolOutcome::OkWithCommand(out, command)
            }
            Err(e) => ToolOutcome::Err(ToolErrorCode::InvalidArgument, e),
        };
    }
    let Some(raw) = args.get("nodes_json") else {
        return ToolOutcome::Err(
            ToolErrorCode::MissingArgument,
            "nodes_json or operations is required".into(),
        );
    };
    match parse_batch_items(raw) {
        Ok(items) if items.is_empty() => ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            "nodes_json must contain at least one descriptor".into(),
        ),
        Ok(items) => {
            let mut out = BTreeMap::new();
            out.insert("wrote".into(), "true".into());
            out.insert("count".into(), items.len().to_string());
            if let Some(phase) = phase {
                out.insert("phase".into(), phase.into());
            }
            ToolOutcome::OkWithCommand(out, EditorCommand::BatchInsert { items })
        }
        Err(e) => ToolOutcome::Err(ToolErrorCode::InvalidArgument, e),
    }
}

struct ParsedInsert {
    binding: String,
    parent: ParentRef,
    node: PenNode,
}

enum ParentRef {
    Root,
    Ref(String),
}

enum ParsedOperations {
    Insert {
        parent_id: NodeId,
        nodes: Vec<PenNode>,
        count: usize,
    },
    Direct(EditorCommand),
}

fn parse_operations(input: &str) -> Result<ParsedOperations, String> {
    let lines = split_operations(input);
    if lines.len() == 1 {
        if let Some(command) = parse_single_direct_operation(&lines[0])? {
            return Ok(ParsedOperations::Direct(command));
        }
    }
    let (parent_id, nodes, count) = parse_insert_operations(input)?;
    Ok(ParsedOperations::Insert {
        parent_id,
        nodes,
        count,
    })
}

fn parse_insert_operations(input: &str) -> Result<(NodeId, Vec<PenNode>, usize), String> {
    let lines = split_operations(input);
    if lines.is_empty() {
        return Err("operations must contain at least one I(parent, node) operation".into());
    }
    let mut inserts = Vec::new();
    let mut binding_to_idx = BTreeMap::new();
    let mut tmp_id = 1usize;
    for (line_idx, line) in lines.iter().enumerate() {
        let (binding, parent, data) = parse_insert_operation(line, line_idx)?;
        if binding_to_idx.contains_key(&binding) {
            return Err(format!("duplicate binding {binding:?}"));
        }
        let mut value: serde_json::Value =
            serde_json::from_str(data).map_err(|e| format!("{binding}: invalid node JSON: {e}"))?;
        normalize_node_shape(&mut value);
        ensure_node_ids(&mut value, &mut tmp_id);
        let node: PenNode = serde_json::from_value(value)
            .map_err(|e| format!("{binding}: invalid PenNode payload: {e}"))?;
        binding_to_idx.insert(binding.clone(), inserts.len());
        inserts.push(ParsedInsert {
            binding,
            parent,
            node,
        });
    }
    assemble_insert_forest(inserts, &binding_to_idx)
}

fn assemble_insert_forest(
    inserts: Vec<ParsedInsert>,
    binding_to_idx: &BTreeMap<String, usize>,
) -> Result<(NodeId, Vec<PenNode>, usize), String> {
    let mut children_by_parent = vec![Vec::<usize>::new(); inserts.len()];
    let mut roots = Vec::<usize>::new();
    let mut real_parent: Option<NodeId> = None;
    for (idx, item) in inserts.iter().enumerate() {
        match &item.parent {
            ParentRef::Root => roots.push(idx),
            ParentRef::Ref(raw) => {
                if let Some(parent_idx) = binding_to_idx.get(raw).copied() {
                    if parent_idx == idx {
                        return Err(format!("{} cannot be inserted under itself", item.binding));
                    }
                    children_by_parent[parent_idx].push(idx);
                } else {
                    let parent_id = root_or_node_id(raw);
                    if parent_id.is_real() {
                        match &real_parent {
                            Some(existing) if existing != &parent_id => {
                                return Err(
                                    "operations can target only one existing parent per call"
                                        .into(),
                                );
                            }
                            None => real_parent = Some(parent_id),
                            _ => {}
                        }
                    }
                    roots.push(idx);
                }
            }
        }
    }
    if roots.is_empty() {
        return Err("operations must include at least one root insert".into());
    }
    let mut visit = vec![0u8; inserts.len()];
    let mut nodes = Vec::with_capacity(roots.len());
    for root in roots {
        nodes.push(build_tree(root, &inserts, &children_by_parent, &mut visit)?);
    }
    Ok((real_parent.unwrap_or(NodeId::NONE), nodes, inserts.len()))
}

fn build_tree(
    idx: usize,
    inserts: &[ParsedInsert],
    children_by_parent: &[Vec<usize>],
    visit: &mut [u8],
) -> Result<PenNode, String> {
    match visit[idx] {
        1 => return Err("operations contain a parent cycle".into()),
        2 => return Ok(inserts[idx].node.clone()),
        _ => {}
    }
    visit[idx] = 1;
    let mut node = inserts[idx].node.clone();
    for child_idx in &children_by_parent[idx] {
        let child = build_tree(*child_idx, inserts, children_by_parent, visit)?;
        let Some(children) = node.children_mut() else {
            return Err(format!(
                "binding {:?} cannot receive children because it is not a container",
                inserts[idx].binding
            ));
        };
        children.push(child);
    }
    visit[idx] = 2;
    Ok(node)
}

fn parse_insert_operation(line: &str, index: usize) -> Result<(String, ParentRef, &str), String> {
    let trimmed = line.trim().trim_end_matches(';').trim();
    let (binding, call) = match find_top_level_char(trimmed, '=') {
        Some(eq) => {
            let binding = trimmed[..eq].trim();
            if !is_binding(binding) {
                return Err(format!("invalid binding {binding:?}"));
            }
            (binding.to_string(), trimmed[eq + 1..].trim())
        }
        None => (format!("_auto_{index}_I"), trimmed),
    };
    if !call.starts_with("I(") || !call.ends_with(')') {
        return Err(format!(
            "{binding}: only I(parent, node) operations are supported"
        ));
    }
    let body = &call[2..call.len() - 1];
    let Some(comma) = find_top_level_char(body, ',') else {
        return Err(format!("{binding}: I() requires parent and node JSON"));
    };
    let parent = parse_parent_ref(body[..comma].trim())?;
    let data = body[comma + 1..].trim();
    if data.is_empty() {
        return Err(format!("{binding}: node JSON is empty"));
    }
    Ok((binding, parent, data))
}

fn split_operations(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut depth = 0i32;
    let mut in_string: Option<char> = None;
    let mut escape = false;
    for ch in raw.chars() {
        if escape {
            buf.push(ch);
            escape = false;
            continue;
        }
        if in_string.is_some() && ch == '\\' {
            buf.push(ch);
            escape = true;
            continue;
        }
        if let Some(quote) = in_string {
            if ch == quote {
                in_string = None;
            }
            buf.push(ch);
            continue;
        }
        match ch {
            '"' | '\'' => {
                in_string = Some(ch);
                buf.push(ch);
            }
            '(' | '[' | '{' => {
                depth += 1;
                buf.push(ch);
            }
            ')' | ']' | '}' => {
                depth -= 1;
                buf.push(ch);
            }
            '\n' if depth == 0 => {
                let line = buf.trim();
                if !line.is_empty() && !line.starts_with("//") {
                    out.push(line.to_string());
                }
                buf.clear();
            }
            _ => buf.push(ch),
        }
    }
    let tail = buf.trim();
    if !tail.is_empty() && !tail.starts_with("//") {
        out.push(tail.to_string());
    }
    out
}

pub(crate) fn find_top_level_char(s: &str, target: char) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_string: Option<char> = None;
    let mut escape = false;
    for (idx, ch) in s.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if in_string.is_some() && ch == '\\' {
            escape = true;
            continue;
        }
        if let Some(quote) = in_string {
            if ch == quote {
                in_string = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => in_string = Some(ch),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ if ch == target && depth == 0 => return Some(idx),
            _ => {}
        }
    }
    None
}

fn parse_parent_ref(raw: &str) -> Result<ParentRef, String> {
    let raw = raw.trim();
    if matches!(raw, "null" | "undefined" | "\"\"" | "''" | "0" | "\"0\"") {
        return Ok(ParentRef::Root);
    }
    if raw.starts_with('"') {
        return serde_json::from_str::<String>(raw)
            .map(ParentRef::Ref)
            .map_err(|e| format!("invalid quoted parent ref: {e}"));
    }
    if raw.starts_with('\'') && raw.ends_with('\'') && raw.len() >= 2 {
        return Ok(ParentRef::Ref(raw[1..raw.len() - 1].to_string()));
    }
    if raw.is_empty() {
        return Ok(ParentRef::Root);
    }
    Ok(ParentRef::Ref(raw.to_string()))
}

fn root_or_node_id(raw: &str) -> NodeId {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "0" {
        NodeId::NONE
    } else {
        NodeId::new(trimmed)
    }
}

fn is_binding(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub(crate) fn normalize_node_shape(value: &mut serde_json::Value) {
    let serde_json::Value::Object(obj) = value else {
        return;
    };
    if let Some(fill) = obj.get_mut("fill") {
        normalize_fill(fill);
    }
    if let Some(stroke) = obj.get_mut("stroke") {
        normalize_stroke(stroke);
    }
    if let Some(serde_json::Value::Array(children)) = obj.get_mut("children") {
        for child in children {
            normalize_node_shape(child);
        }
    }
}

fn normalize_fill(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(color) => {
            *value = serde_json::json!([{ "type": "solid", "color": color }]);
        }
        serde_json::Value::Object(_) => {
            let single = std::mem::take(value);
            *value = serde_json::Value::Array(vec![single]);
        }
        _ => {}
    }
}

fn normalize_stroke(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(color) => {
            *value = serde_json::json!({
                "thickness": 1,
                "fill": [{ "type": "solid", "color": color }]
            });
        }
        serde_json::Value::Object(obj) => {
            if let Some(color) = obj.remove("color") {
                obj.entry("fill")
                    .or_insert_with(|| serde_json::json!([{ "type": "solid", "color": color }]));
            }
            if let Some(fill) = obj.get_mut("fill") {
                normalize_fill(fill);
            }
        }
        _ => {}
    }
}

fn ensure_node_ids(value: &mut serde_json::Value, next: &mut usize) {
    let serde_json::Value::Object(obj) = value else {
        return;
    };
    if obj.contains_key("type") && !obj.contains_key("id") {
        obj.insert(
            "id".into(),
            serde_json::Value::String(format!("__op_tmp_{next}")),
        );
        *next += 1;
    }
    if let Some(serde_json::Value::Array(children)) = obj.get_mut("children") {
        for child in children {
            ensure_node_ids(child, next);
        }
    }
}

/// `design_skeleton` — phase 1 of TS's layered design workflow.
/// Same wire shape as `batch_design`; the result payload carries
/// `phase=skeleton` so clients can phase their prompting.
pub struct DesignSkeleton;
impl McpTool for DesignSkeleton {
    fn name(&self) -> &str {
        "design_skeleton"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        dispatch_phase(args, "skeleton")
    }
}
pub fn design_skeleton_snapshot() -> DesignSkeleton {
    DesignSkeleton
}

/// `design_content` — phase 2 of the layered design workflow.
/// Mirrors `batch_design` apply semantics; tagged `phase=content`.
pub struct DesignContent;
impl McpTool for DesignContent {
    fn name(&self) -> &str {
        "design_content"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        dispatch_phase(args, "content")
    }
}
pub fn design_content_snapshot() -> DesignContent {
    DesignContent
}

/// `design_refine` — phase 3 of the layered design workflow.
/// Mirrors `batch_design` apply semantics; tagged `phase=refine`.
pub struct DesignRefine;
impl McpTool for DesignRefine {
    fn name(&self) -> &str {
        "design_refine"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        dispatch_phase(args, "refine")
    }
}
pub fn design_refine_snapshot() -> DesignRefine {
    DesignRefine
}

/// Hand-rolled parser for the `nodes_json` payload. Shell-core
/// stays serde-free so the wasm32 bundle doesn't grow. Returns a
/// Vec<BatchInsertItem> on success, an English error string on
/// any structural problem.
///
/// Grammar (whitespace ignored):
///   array      = '[' (item (',' item)* )? ']'
///   item       = '{' pair (',' pair)* '}'
///   pair       = string ':' value
///   string     = '"' chars '"'
///   value      = string | number
///
/// Strings handle `\"` and `\\` escapes inline; no `\u` decode
/// (the wire never carries unicode escapes in tool args today).
fn parse_batch_items(input: &str) -> Result<Vec<BatchInsertItem>, String> {
    // The wire-level parser doesn't unescape JSON string contents
    // — `{"nodes_json":"[\"x\"]"}` arrives here as the raw bytes
    // `[\"x\"]` (backslash + quote). Pre-pass: unescape so the
    // grammar below sees real `"` / `\` / `\n` etc.
    let unescaped = unescape_wire_string(input)?;
    let bytes = unescaped.as_bytes();
    let mut i = 0usize;
    skip_ws(bytes, &mut i);
    if i >= bytes.len() || bytes[i] != b'[' {
        return Err("nodes_json must start with `[`".into());
    }
    i += 1;
    skip_ws(bytes, &mut i);
    let mut out = Vec::new();
    if i < bytes.len() && bytes[i] == b']' {
        return Ok(out); // empty array — caller surfaces InvalidArgument
    }
    loop {
        skip_ws(bytes, &mut i);
        let item = parse_item(bytes, &mut i)?;
        out.push(item);
        skip_ws(bytes, &mut i);
        if i >= bytes.len() {
            return Err("unterminated array".into());
        }
        match bytes[i] {
            b',' => {
                i += 1;
            }
            b']' => {
                i += 1;
                skip_ws(bytes, &mut i);
                if i != bytes.len() {
                    return Err("trailing garbage after array".into());
                }
                return Ok(out);
            }
            other => {
                return Err(format!(
                    "expected `,` or `]` after item, got {:?}",
                    other as char
                ));
            }
        }
    }
}

fn parse_item(bytes: &[u8], i: &mut usize) -> Result<BatchInsertItem, String> {
    if *i >= bytes.len() || bytes[*i] != b'{' {
        return Err("expected `{` to start a descriptor".into());
    }
    *i += 1;
    let mut kind: Option<String> = None;
    let mut name: Option<String> = None;
    let mut x: Option<i32> = None;
    let mut y: Option<i32> = None;
    let mut width: Option<i32> = None;
    let mut height: Option<i32> = None;
    let mut fill_hex: Option<String> = None;
    loop {
        skip_ws(bytes, i);
        if *i >= bytes.len() {
            return Err("unterminated descriptor".into());
        }
        if bytes[*i] == b'}' {
            *i += 1;
            break;
        }
        let key = parse_string(bytes, i)?;
        skip_ws(bytes, i);
        if *i >= bytes.len() || bytes[*i] != b':' {
            return Err(format!("expected `:` after key {key:?}"));
        }
        *i += 1;
        skip_ws(bytes, i);
        match key.as_str() {
            "kind" => kind = Some(parse_string(bytes, i)?),
            "name" => name = Some(parse_string(bytes, i)?),
            "fill_hex" => fill_hex = Some(parse_string(bytes, i)?),
            "x" => x = Some(parse_int(bytes, i)?),
            "y" => y = Some(parse_int(bytes, i)?),
            "width" => width = Some(parse_int(bytes, i)?),
            "height" => height = Some(parse_int(bytes, i)?),
            other => return Err(format!("unknown key {other:?} in descriptor")),
        }
        skip_ws(bytes, i);
        if *i < bytes.len() && bytes[*i] == b',' {
            *i += 1;
        }
    }
    let kind = kind.ok_or("descriptor missing `kind`")?;
    if !ALLOWED_KINDS.iter().any(|k| *k == kind) {
        return Err(format!(
            "kind {kind:?} not supported; allowed: {}",
            ALLOWED_KINDS.join(", ")
        ));
    }
    let name = name.ok_or("descriptor missing `name`")?;
    let x = x.ok_or("descriptor missing `x`")?;
    let y = y.ok_or("descriptor missing `y`")?;
    let width = width.ok_or("descriptor missing `width`")?;
    let height = height.ok_or("descriptor missing `height`")?;
    if width < 0 || height < 0 {
        return Err("width / height must be non-negative".into());
    }
    if let Some(ref hex) = fill_hex {
        if !validate_hex(hex) {
            return Err(format!(
                "fill_hex must be #rgb/#rrggbb/#rrggbbaa, got {hex:?}"
            ));
        }
    }
    Ok(BatchInsertItem {
        kind,
        name,
        x,
        y,
        width,
        height,
        fill_hex,
    })
}

/// Reverse the JSON-string escaping the wire parser left intact.
/// Handles `\"` / `\\` / `\n` / `\t` / `\r` / `\/`. Anything else
/// passes through verbatim (no `\u` decode today).
fn unescape_wire_string(input: &str) -> Result<String, String> {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'\\' && i + 1 < bytes.len() {
            let next = bytes[i + 1];
            match next {
                b'"' => out.push('"'),
                b'\\' => out.push('\\'),
                b'n' => out.push('\n'),
                b't' => out.push('\t'),
                b'r' => out.push('\r'),
                b'/' => out.push('/'),
                _ => {
                    // Unknown escape — pass through verbatim so
                    // typos surface as parser errors downstream.
                    out.push('\\');
                    out.push(next as char);
                }
            }
            i += 2;
        } else {
            let start = i;
            while i < bytes.len() && bytes[i] != b'\\' {
                i += 1;
            }
            let slice = std::str::from_utf8(&bytes[start..i])
                .map_err(|_| "invalid UTF-8 in nodes_json".to_string())?;
            out.push_str(slice);
        }
    }
    Ok(out)
}

fn skip_ws(bytes: &[u8], i: &mut usize) {
    while *i < bytes.len() && bytes[*i].is_ascii_whitespace() {
        *i += 1;
    }
}

fn parse_string(bytes: &[u8], i: &mut usize) -> Result<String, String> {
    if *i >= bytes.len() || bytes[*i] != b'"' {
        return Err("expected string".into());
    }
    *i += 1;
    let mut out = String::new();
    while *i < bytes.len() {
        let c = bytes[*i];
        if c == b'"' {
            *i += 1;
            return Ok(out);
        }
        if c == b'\\' {
            *i += 1;
            if *i >= bytes.len() {
                return Err("unterminated escape".into());
            }
            let esc = bytes[*i];
            match esc {
                b'"' => out.push('"'),
                b'\\' => out.push('\\'),
                b'n' => out.push('\n'),
                b't' => out.push('\t'),
                b'r' => out.push('\r'),
                b'/' => out.push('/'),
                other => return Err(format!("unsupported escape \\{}", other as char)),
            }
            *i += 1;
        } else {
            // Find the next escape/quote and slice so multi-byte
            // chars stay intact (per-byte append would split them).
            let start = *i;
            while *i < bytes.len() && bytes[*i] != b'"' && bytes[*i] != b'\\' {
                *i += 1;
            }
            let slice = std::str::from_utf8(&bytes[start..*i])
                .map_err(|_| "invalid UTF-8 in string".to_string())?;
            out.push_str(slice);
        }
    }
    Err("unterminated string".into())
}

fn parse_int(bytes: &[u8], i: &mut usize) -> Result<i32, String> {
    let start = *i;
    if *i < bytes.len() && bytes[*i] == b'-' {
        *i += 1;
    }
    while *i < bytes.len() && bytes[*i].is_ascii_digit() {
        *i += 1;
    }
    if start == *i {
        return Err("expected integer".into());
    }
    let raw = std::str::from_utf8(&bytes[start..*i])
        .map_err(|_| "invalid UTF-8 in integer".to_string())?;
    raw.parse::<i32>()
        .map_err(|_| format!("expected i32, got {raw:?}"))
}
