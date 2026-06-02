//! First-party MCP read tools (part 2). Carved off `tools.rs` to keep
//! both files under the 800-line cap as the read surface grew.
//!
//! Ported off shell-core's `Document` onto `op_editor_core::
//! EditorState` (canonical `PenDocument`).

use std::collections::BTreeMap;

use jian_ops_schema::node::PenNode;
use op_editor_core::geometry::aggregate_bounds;
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::EditorState;

use super::tools::{active_children, kind_label};
use super::{McpTool, ToolErrorCode, ToolOutcome};

type ToolCallError = (ToolErrorCode, String);

// --- snapshot_layout -------------------------------------------------

/// First-party `snapshot_layout` tool — bounding box of every
/// node on the active page, optionally filtered by page / parent /
/// depth for TS CLI compatibility.
pub struct SnapshotLayout {
    pub items: Vec<(String, i32, i32, i32, i32)>,
    pages: Vec<PageLayout>,
    active_page_id: String,
}

#[derive(Clone)]
struct PageLayout {
    id: String,
    records: Vec<LayoutRecord>,
}

#[derive(Clone)]
struct LayoutRecord {
    id: String,
    ancestors: Vec<String>,
    depth: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl McpTool for SnapshotLayout {
    fn name(&self) -> &str {
        "snapshot_layout"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let page = match self.page_layout(args) {
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
        let parent_depth = match parent_id {
            Some(id) => match page.records.iter().find(|r| r.id == id) {
                Some(parent) => Some(parent.depth),
                None => {
                    return ToolOutcome::Err(
                        ToolErrorCode::ToolFailed,
                        format!("node not found: {id}"),
                    );
                }
            },
            None => None,
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
        let layout_items = self.layout_items(args, &selected);
        let encoded: Vec<String> = layout_items
            .iter()
            .map(|(id, x, y, w, h)| format!("{id}|{x}|{y}|{w}|{h}"))
            .collect();
        let mut out = BTreeMap::new();
        out.insert("count".into(), layout_items.len().to_string());
        out.insert("layout".into(), encoded.join(";"));
        ToolOutcome::Ok(out)
    }
}

pub fn snapshot_layout_snapshot(state: &EditorState) -> SnapshotLayout {
    let items = active_children(state)
        .iter()
        .map(|n| {
            let b = aggregate_bounds(n);
            (
                n.id_str().to_string(),
                b.x as i32,
                b.y as i32,
                b.w as i32,
                b.h as i32,
            )
        })
        .collect();
    let (pages, active_page_id) = page_layout_snapshots(state);
    SnapshotLayout {
        items,
        pages,
        active_page_id,
    }
}

impl SnapshotLayout {
    fn page_layout(&self, args: &BTreeMap<String, String>) -> Result<&PageLayout, ToolCallError> {
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

    fn layout_items(
        &self,
        args: &BTreeMap<String, String>,
        selected: &[&LayoutRecord],
    ) -> Vec<(String, i32, i32, i32, i32)> {
        if args.is_empty() {
            return self.items.clone();
        }
        selected
            .iter()
            .map(|r| (r.id.clone(), r.x, r.y, r.w, r.h))
            .collect()
    }
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
    match state.doc.pages.as_ref() {
        Some(pages) if !pages.is_empty() => {
            let active = state.ui.active_page_index.min(pages.len() - 1);
            let out = pages
                .iter()
                .map(|page| PageLayout {
                    id: page.id.clone(),
                    records: layout_records(&page.children),
                })
                .collect();
            (out, pages[active].id.clone())
        }
        _ => (
            vec![PageLayout {
                id: "0".into(),
                records: layout_records(&state.doc.children),
            }],
            "0".into(),
        ),
    }
}

fn page_space_snapshots(state: &EditorState) -> (Vec<PageSpace>, String) {
    match state.doc.pages.as_ref() {
        Some(pages) if !pages.is_empty() => {
            let active = state.ui.active_page_index.min(pages.len() - 1);
            let out = pages
                .iter()
                .map(|page| page_space(&page.id, &page.children))
                .collect();
            (out, pages[active].id.clone())
        }
        _ => (vec![page_space("0", &state.doc.children)], "0".into()),
    }
}

fn layout_records(nodes: &[PenNode]) -> Vec<LayoutRecord> {
    fn walk(
        nodes: &[PenNode],
        ancestors: &[String],
        depth: u32,
        parent_x: i32,
        parent_y: i32,
        out: &mut Vec<LayoutRecord>,
    ) {
        for n in nodes {
            let b = aggregate_bounds(n);
            let x = parent_x + b.x as i32;
            let y = parent_y + b.y as i32;
            let id = n.id_str().to_string();
            out.push(LayoutRecord {
                id: id.clone(),
                ancestors: ancestors.to_vec(),
                depth,
                x,
                y,
                w: b.w as i32,
                h: b.h as i32,
            });
            if let Some(children) = n.children() {
                let mut child_ancestors = ancestors.to_vec();
                child_ancestors.push(id.clone());
                walk(children, &child_ancestors, depth + 1, x, y, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(nodes, &[], 0, 0, 0, &mut out);
    out
}

fn page_space(id: &str, roots: &[PenNode]) -> PageSpace {
    fn walk(nodes: &[PenNode], out: &mut Vec<BoundsRecord>) {
        for n in nodes {
            out.push(bounds_record(n));
            if let Some(children) = n.children() {
                walk(children, out);
            }
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

fn bounds_record(node: &PenNode) -> BoundsRecord {
    let b = aggregate_bounds(node);
    BoundsRecord {
        id: node.id_str().to_string(),
        x: b.x as i32,
        y: b.y as i32,
        w: b.w as i32,
        h: b.h as i32,
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

// --- get_canvas_bounds -----------------------------------------------

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
