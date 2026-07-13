//! First-party MCP read tools (part 2). Carved off `tools.rs` to keep
//! both files under the 800-line cap as the read surface grew.
//!
//! Ported off shell-core's `Document` onto `op_editor_core::
//! EditorState` (canonical `PenDocument`).

use std::collections::BTreeMap;

use jian_ops_schema::node::PenNode;
use jian_scene::layout_scene::{NodeKind, SceneNode};
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::walkers::find_node;
use op_editor_core::{EditorState, NodeId};
use serde_json::{json, Map, Value};

use super::tools::kind_label;
use super::{McpTool, ToolErrorCode, ToolOutcome};

type ToolCallError = (ToolErrorCode, String);

// --- snapshot_layout -------------------------------------------------

/// First-party `snapshot_layout` tool — bounding box of every
/// node on the active page, optionally filtered by page / parent /
/// depth for TS CLI compatibility.
pub struct SnapshotLayout {
    state: EditorState,
}

#[derive(Clone)]
struct PageLayout {
    id: String,
    records: Vec<LayoutRecord>,
}

#[derive(Clone)]
struct LayoutRecord {
    id: String,
    name: Option<String>,
    type_label: String,
    ancestors: Vec<String>,
    depth: u32,
    // Document-space coords kept as f64 (TS `LayoutEntry` returns raw
    // numbers); truncating to i32 would drop fractional positions/sizes.
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[derive(Clone)]
struct NodeMeta {
    name: Option<String>,
    type_label: String,
}

impl McpTool for SnapshotLayout {
    fn name(&self) -> &str {
        "snapshot_layout"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let (pages, active_page_id) = page_layout_snapshots(&self.state);
        let page = match page_layout(&pages, &active_page_id, args) {
            Ok(page) => page,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let max_depth = match arg_alias(args, &["maxDepth", "depth"]) {
            Some(raw) => match raw.parse::<u32>() {
                Ok(v) => v,
                Err(_) => {
                    return ToolOutcome::Err(
                        ToolErrorCode::InvalidArgument,
                        format!("maxDepth must be a u32, got {raw:?}"),
                    );
                }
            },
            None => 1,
        };
        let parent_id = arg_alias(args, &["parentId", "parent_id", "parent"]);
        // TS calls `computeLayoutTree(parent.children, …)` with the default
        // `parentX/parentY = 0`, so a `parentId` query returns coords RELATIVE
        // to that parent (not document-absolute). Our records are page-rooted
        // absolute, so subtract the parent's absolute origin to match.
        let (parent_depth, parent_offset) = match parent_id {
            Some(id) => match page.records.iter().find(|r| r.id == id) {
                Some(parent) => (Some(parent.depth), (parent.x, parent.y)),
                None => {
                    // TS snapshot-layout.ts:33 throws `Node not found: ${id}`.
                    return ToolOutcome::Err(
                        ToolErrorCode::ToolFailed,
                        format!("Node not found: {id}"),
                    );
                }
            },
            None => (None, (0.0, 0.0)),
        };
        let selected: Vec<&LayoutRecord> = page
            .records
            .iter()
            .filter(|record| match (parent_id, parent_depth) {
                (Some(parent), Some(depth)) => {
                    record.ancestors.iter().any(|id| id == parent)
                        && record.depth <= depth + max_depth + 1
                }
                _ => record.depth <= max_depth,
            })
            .collect();
        // TS `snapshot_layout` returns `{ layout: LayoutEntry[] }` — a nested
        // tree of {id,name?,type,x,y,width,height,children?} with absolute
        // coords. Roots are the page's top-level nodes (or `parentId`'s direct
        // children); children nest up to maxDepth. The flat `selected` set is
        // already depth-bounded, so we just re-nest it by parent.
        let root_key = parent_id.unwrap_or("");
        let layout = layout_children_values(&selected, root_key, parent_offset);
        ToolOutcome::OkJson(json!({ "layout": layout }).to_string())
    }
}

/// Build the ordered list of `LayoutEntry` JSON objects whose immediate
/// parent (in document order) is `parent_key` (`""` for page-roots), each
/// recursively carrying its own children — mirrors TS `computeLayoutTree`.
/// `offset` is subtracted from every coord so a `parentId` query returns
/// parent-relative coords (TS passes `parentX/Y = 0`); it is `(0, 0)` for a
/// page-level query (coords stay document-absolute).
fn layout_children_values(
    selected: &[&LayoutRecord],
    parent_key: &str,
    offset: (f64, f64),
) -> Vec<Value> {
    selected
        .iter()
        .filter(|record| record.ancestors.last().map(String::as_str).unwrap_or("") == parent_key)
        .map(|record| layout_entry_value(record, selected, offset))
        .collect()
}

/// Serialize a document coordinate the way JS `JSON.stringify` does: a
/// whole number as an integer (`100`, not `100.0`), a fractional value as a
/// decimal — so the wire bytes match TS `LayoutEntry` numbers exactly.
fn js_number(v: f64) -> Value {
    if v.is_finite() && v.fract() == 0.0 && v.abs() < 9_007_199_254_740_992.0 {
        json!(v as i64)
    } else {
        json!(v)
    }
}

fn layout_entry_value(
    record: &LayoutRecord,
    selected: &[&LayoutRecord],
    offset: (f64, f64),
) -> Value {
    let mut obj = Map::new();
    obj.insert("id".into(), json!(record.id));
    // TS `LayoutEntry.name` is optional — omit when the node is unnamed
    // (JSON.stringify drops `undefined`).
    if let Some(name) = &record.name {
        obj.insert("name".into(), json!(name));
    }
    obj.insert("type".into(), json!(&record.type_label));
    obj.insert("x".into(), js_number(record.x - offset.0));
    obj.insert("y".into(), js_number(record.y - offset.1));
    obj.insert("width".into(), js_number(record.w));
    obj.insert("height".into(), js_number(record.h));
    let children = layout_children_values(selected, &record.id, offset);
    if !children.is_empty() {
        obj.insert("children".into(), Value::Array(children));
    }
    Value::Object(obj)
}

pub fn snapshot_layout_snapshot(state: &EditorState) -> SnapshotLayout {
    SnapshotLayout {
        state: state.clone(),
    }
}

fn page_layout<'a>(
    pages: &'a [PageLayout],
    active_page_id: &'a str,
    args: &BTreeMap<String, String>,
) -> Result<&'a PageLayout, ToolCallError> {
    let id = arg_alias(args, &["pageId", "page_id", "page"]);
    let target = id.unwrap_or(active_page_id);
    pages.iter().find(|page| page.id == target).ok_or_else(|| {
        (
            ToolErrorCode::ToolFailed,
            format!("page not found: {target}"),
        )
    })
}

// --- find_empty_space -----------------------------------------------

/// First-party `find_empty_space` tool — find a padded position for a
/// new rectangle relative to either the page content or one node.
pub struct FindEmptySpace {
    pages: Vec<PageSpace>,
    active_page_id: String,
}

#[derive(Clone)]
struct PageSpace {
    id: String,
    roots: Vec<BoundsRecord>,
    all: Vec<BoundsRecord>,
}

#[derive(Clone)]
struct BoundsRecord {
    id: String,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl McpTool for FindEmptySpace {
    fn name(&self) -> &str {
        "find_empty_space"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let width = match required_i32_arg(args, "width") {
            Ok(v) => v,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let height = match required_i32_arg(args, "height") {
            Ok(v) => v,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let padding = match optional_i32_arg(args, "padding", 50) {
            Ok(v) => v,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let Some(direction) = args.get("direction") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "direction is required".into(),
            );
        };
        let page = match self.page_space(args) {
            Ok(page) => page,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let node_id = arg_alias(args, &["nodeId", "node_id", "node"]);
        let nodes: Vec<&BoundsRecord> = match node_id {
            Some(id) => match page.all.iter().find(|record| record.id == id) {
                Some(record) => vec![record],
                None => {
                    return ToolOutcome::Err(
                        ToolErrorCode::ToolFailed,
                        format!("node not found: {id}"),
                    );
                }
            },
            None => page.roots.iter().collect(),
        };
        let (x, y) = match find_padded_position(&nodes, direction, width, height, padding) {
            Ok(pos) => pos,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let mut out = BTreeMap::new();
        out.insert("x".into(), x.to_string());
        out.insert("y".into(), y.to_string());
        ToolOutcome::Ok(out)
    }
}

pub fn find_empty_space_snapshot(state: &EditorState) -> FindEmptySpace {
    let (pages, active_page_id) = page_space_snapshots(state);
    FindEmptySpace {
        pages,
        active_page_id,
    }
}

impl FindEmptySpace {
    fn page_space(&self, args: &BTreeMap<String, String>) -> Result<&PageSpace, ToolCallError> {
        let id = arg_alias(args, &["pageId", "page_id", "page"]);
        let target = id.unwrap_or(self.active_page_id.as_str());
        self.pages
            .iter()
            .find(|page| page.id == target)
            .ok_or_else(|| {
                (
                    ToolErrorCode::ToolFailed,
                    format!("page not found: {target}"),
                )
            })
    }
}

fn page_layout_snapshots(state: &EditorState) -> (Vec<PageLayout>, String) {
    let scene = op_pen_loader::editor_state_to_layout_scene(state);
    match state.doc.pages.as_ref() {
        Some(pages) if !pages.is_empty() => {
            let active = state.ui.active_page_index.min(pages.len() - 1);
            let out = pages
                .iter()
                .enumerate()
                .map(|(idx, page)| {
                    let mut meta = BTreeMap::new();
                    collect_node_meta(&page.children, &mut meta);
                    let records = scene
                        .pages
                        .get(idx)
                        .map(|scene_page| scene_layout_records(&scene_page.children, &meta))
                        .unwrap_or_default();
                    PageLayout {
                        id: page.id.clone(),
                        records,
                    }
                })
                .collect();
            (out, pages[active].id.clone())
        }
        _ => {
            let mut meta = BTreeMap::new();
            collect_node_meta(&state.doc.children, &mut meta);
            let records = scene
                .pages
                .first()
                .map(|page| scene_layout_records(&page.children, &meta))
                .unwrap_or_default();
            (
                vec![PageLayout {
                    id: "0".into(),
                    records,
                }],
                "0".into(),
            )
        }
    }
}

fn page_space_snapshots(state: &EditorState) -> (Vec<PageSpace>, String) {
    let scene = op_pen_loader::editor_state_to_layout_scene(state);
    match state.doc.pages.as_ref() {
        Some(pages) if !pages.is_empty() => {
            let active = state.ui.active_page_index.min(pages.len() - 1);
            let out = pages
                .iter()
                .enumerate()
                .map(|(idx, page)| {
                    let roots = scene
                        .pages
                        .get(idx)
                        .map(|scene_page| scene_page.children.as_slice())
                        .unwrap_or_default();
                    page_space(&page.id, roots)
                })
                .collect();
            (out, pages[active].id.clone())
        }
        _ => {
            let roots = scene
                .pages
                .first()
                .map(|page| page.children.as_slice())
                .unwrap_or_default();
            (vec![page_space("0", roots)], "0".into())
        }
    }
}

/// `(x, y, w, h)` for one node mirroring TS `getNodeBounds` (tree-utils.ts):
/// authored `base.x/y` (parent-relative), and numeric width/height or 100
/// when missing/keyword/zero (TS `w || 100`). A `ref` node with no numeric
/// width adopts the referenced node's dimensions (looked up in `all_nodes`),
/// exactly as TS does.
fn node_bounds(n: &PenNode, all_nodes: &[PenNode]) -> (f64, f64, f64, f64) {
    let base = n.base();
    let numeric = |px: Option<f64>| px.filter(|&v| v != 0.0);
    let mut w = numeric(n.width_px());
    let mut h = numeric(n.height_px());
    // TS: `if (node.type === 'ref' && !w)` → take the referenced component's
    // numeric dims (or 100). The lookup walks the whole page tree (allNodes).
    if w.is_none() {
        if let PenNode::Ref(r) = n {
            if let Some(target) = NodeId::new_opt(r.target.clone()) {
                if let Some(comp) = find_node(all_nodes, &target) {
                    w = numeric(comp.width_px());
                    h = numeric(comp.height_px());
                }
            }
        }
    }
    (
        base.x.unwrap_or(0.0),
        base.y.unwrap_or(0.0),
        w.unwrap_or(100.0),
        h.unwrap_or(100.0),
    )
}

fn collect_node_meta(nodes: &[PenNode], out: &mut BTreeMap<String, NodeMeta>) {
    for node in nodes {
        out.insert(
            node.id_str().to_string(),
            NodeMeta {
                name: node.base().name.clone(),
                type_label: kind_label(node).to_string(),
            },
        );
        if let Some(children) = node.children() {
            collect_node_meta(children, out);
        }
    }
}

fn scene_layout_records(
    roots: &[SceneNode],
    meta: &BTreeMap<String, NodeMeta>,
) -> Vec<LayoutRecord> {
    fn walk(
        nodes: &[SceneNode],
        meta: &BTreeMap<String, NodeMeta>,
        ancestors: &[String],
        depth: u32,
        out: &mut Vec<LayoutRecord>,
    ) {
        for node in nodes {
            let id = node.id.clone();
            let node_meta = meta.get(&id);
            let bounds = node.aggregate_bounds();
            out.push(LayoutRecord {
                id: id.clone(),
                name: node_meta.and_then(|m| m.name.clone()),
                type_label: node_meta
                    .map(|m| m.type_label.clone())
                    .unwrap_or_else(|| scene_kind_label(&node.kind).to_string()),
                ancestors: ancestors.to_vec(),
                depth,
                x: f64::from(bounds.origin.x),
                y: f64::from(bounds.origin.y),
                w: f64::from(bounds.size.x),
                h: f64::from(bounds.size.y),
            });
            let mut child_ancestors = ancestors.to_vec();
            child_ancestors.push(id);
            walk(&node.children, meta, &child_ancestors, depth + 1, out);
        }
    }

    let mut out = Vec::new();
    walk(roots, meta, &[], 0, &mut out);
    out
}

fn scene_kind_label(kind: &NodeKind) -> &str {
    match kind {
        NodeKind::Frame => "frame",
        NodeKind::Group => "group",
        NodeKind::Rect => "rectangle",
        NodeKind::Ellipse => "ellipse",
        NodeKind::Polygon => "polygon",
        NodeKind::Line => "line",
        NodeKind::Path => "path",
        NodeKind::Text => "text",
        NodeKind::Other(label) => label.as_str(),
    }
}

/// Build a TS `computeLayoutTree(roots, all_nodes, max_depth)`-equivalent
/// `LayoutEntry[]` JSON for an arbitrary subtree (depth-limited, document-
/// absolute coords). Used by `design_refine` for its `layoutSnapshot` result.
pub(crate) fn subtree_layout_value(
    roots: &[PenNode],
    all_nodes: &[PenNode],
    max_depth: u32,
) -> Value {
    let records = layout_records_in(roots, all_nodes);
    let selected: Vec<&LayoutRecord> = records.iter().filter(|r| r.depth <= max_depth).collect();
    Value::Array(layout_children_values(&selected, "", (0.0, 0.0)))
}

/// Like [`layout_records`] but with a distinct `all_nodes` scope for ref-node
/// dim lookup (mirrors TS `computeLayoutTree(roots, allChildren, …)`), so a
/// subtree can be laid out against the whole page's ref targets.
fn layout_records_in(roots: &[PenNode], all_nodes: &[PenNode]) -> Vec<LayoutRecord> {
    fn walk(
        nodes: &[PenNode],
        all_nodes: &[PenNode],
        ancestors: &[String],
        depth: u32,
        parent_x: f64,
        parent_y: f64,
        out: &mut Vec<LayoutRecord>,
    ) {
        for n in nodes {
            // Mirror TS `getNodeBounds`: x/y are the node's OWN authored
            // offset (relative to parent), w/h are the numeric width/height or
            // 100 when missing/keyword (`w || 100`). Then accumulate the
            // parent's absolute position for document-absolute x/y, exactly as
            // TS `computeLayoutTree`'s `absX = parentX + bounds.x`. (kept f64:
            // TS returns raw numbers, never i32-truncated.)
            let b = node_bounds(n, all_nodes);
            let x = parent_x + b.0;
            let y = parent_y + b.1;
            let id = n.id_str().to_string();
            out.push(LayoutRecord {
                id: id.clone(),
                name: n.base().name.clone(),
                type_label: kind_label(n).to_string(),
                ancestors: ancestors.to_vec(),
                depth,
                x,
                y,
                w: b.2,
                h: b.3,
            });
            if let Some(children) = n.children() {
                let mut child_ancestors = ancestors.to_vec();
                child_ancestors.push(id.clone());
                walk(children, all_nodes, &child_ancestors, depth + 1, x, y, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(roots, all_nodes, &[], 0, 0.0, 0.0, &mut out);
    out
}

fn page_space(id: &str, roots: &[SceneNode]) -> PageSpace {
    fn walk(nodes: &[SceneNode], out: &mut Vec<BoundsRecord>) {
        for n in nodes {
            out.push(bounds_record(n));
            walk(&n.children, out);
        }
    }
    let root_bounds = roots.iter().map(bounds_record).collect();
    let mut all = Vec::new();
    walk(roots, &mut all);
    PageSpace {
        id: id.to_string(),
        roots: root_bounds,
        all,
    }
}

fn bounds_record(node: &SceneNode) -> BoundsRecord {
    let b = node.aggregate_bounds();
    BoundsRecord {
        id: node.id.clone(),
        x: b.origin.x as i32,
        y: b.origin.y as i32,
        w: b.size.x as i32,
        h: b.size.y as i32,
    }
}

fn find_padded_position(
    nodes: &[&BoundsRecord],
    direction: &str,
    width: i32,
    height: i32,
    padding: i32,
) -> Result<(i32, i32), ToolCallError> {
    if nodes.is_empty() {
        return Ok((0, 0));
    }
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    for n in nodes {
        min_x = min_x.min(n.x);
        min_y = min_y.min(n.y);
        max_x = max_x.max(n.x + n.w);
        max_y = max_y.max(n.y + n.h);
    }
    match direction {
        "right" => Ok((max_x + padding, min_y)),
        "left" => Ok((min_x - padding - width, min_y)),
        "bottom" => Ok((min_x, max_y + padding)),
        "top" => Ok((min_x, min_y - padding - height)),
        other => Err((
            ToolErrorCode::InvalidArgument,
            format!("direction must be top/right/bottom/left, got {other:?}"),
        )),
    }
}

fn arg_alias<'a>(args: &'a BTreeMap<String, String>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| args.get(*key).map(String::as_str))
}

fn required_i32_arg(args: &BTreeMap<String, String>, key: &str) -> Result<i32, ToolCallError> {
    let Some(raw) = args.get(key) else {
        return Err((ToolErrorCode::MissingArgument, format!("{key} is required")));
    };
    raw.parse::<i32>().map_err(|_| {
        (
            ToolErrorCode::InvalidArgument,
            format!("{key} must be an i32, got {raw:?}"),
        )
    })
}

fn optional_i32_arg(
    args: &BTreeMap<String, String>,
    key: &str,
    default: i32,
) -> Result<i32, ToolCallError> {
    match args.get(key) {
        Some(raw) => raw.parse::<i32>().map_err(|_| {
            (
                ToolErrorCode::InvalidArgument,
                format!("{key} must be an i32, got {raw:?}"),
            )
        }),
        None => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bottom_args(page_id: Option<&str>) -> BTreeMap<String, String> {
        let mut args = BTreeMap::from([
            ("width".into(), "50".into()),
            ("height".into(), "50".into()),
            ("padding".into(), "10".into()),
            ("direction".into(), "bottom".into()),
        ]);
        if let Some(id) = page_id {
            args.insert("pageId".into(), id.into());
        }
        args
    }

    #[test]
    fn empty_space_uses_resolved_omitted_height_and_requested_page() {
        let src = r#"{
          "version":"1.0.0","pages":[
            {"id":"hug-page","name":"Hug","children":[
              {"type":"frame","id":"hug","x":10,"y":20,"width":390,
               "layout":"vertical","gap":5,"padding":[10,20],"children":[
                 {"type":"rectangle","id":"a","width":"fill_container","height":40},
                 {"type":"rectangle","id":"b","width":"fill_container","height":50}
               ]},
              {"type":"frame","id":"fixed","x":500,"y":30,"width":200,"height":20}
            ]},
            {"id":"other-page","name":"Other","children":[
              {"type":"frame","id":"other","x":900,"y":100,"width":20,"height":30}
            ]}
          ],"children":[]
        }"#;
        let parsed = jian_ops_schema::load_str(src).expect("parse fixture");
        let mut state = EditorState::from_document(parsed.value);
        state.ui.active_page_index = 1;
        let tool = find_empty_space_snapshot(&state);

        let ToolOutcome::Ok(active) = tool.call(&bottom_args(None)) else {
            panic!("active-page lookup should succeed");
        };
        assert_eq!(active.get("x").map(String::as_str), Some("900"));
        assert_eq!(active.get("y").map(String::as_str), Some("140"));

        let ToolOutcome::Ok(hug) = tool.call(&bottom_args(Some("hug-page"))) else {
            panic!("requested-page lookup should succeed");
        };
        assert_eq!(hug.get("x").map(String::as_str), Some("10"));
        assert_eq!(
            hug.get("y").map(String::as_str),
            Some("145"),
            "bottom placement must include the resolved 115px Hug root"
        );
    }
}
