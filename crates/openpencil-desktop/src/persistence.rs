//! `.pen` / `.op` file Save / Open.
//!
//! Persists the document tree (pages + nodes) to JSON via a small
//! hand-rolled DTO. shell-core stays serde-free; the conversion
//! lives entirely in the desktop binary so Color / Rect / Point2D
//! (external types) don't need wrapper plumbing on the library
//! side.
//!
//! Format: `{ "version": 1, "active_page_index": 0, "pages": [...] }`.
//! Bumping the version requires a migration step in `from_payload`.

use std::path::PathBuf;

use openpencil_shell_core::document::{Document, FillType, Node, NodeId, NodeKind, Page, Stroke};
use openpencil_shell_core::{Color, Point2D, Rect};
use serde::{Deserialize, Serialize};

const CURRENT_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct DocPayload {
    pub version: u32,
    pub active_page_index: usize,
    pub pages: Vec<PagePayload>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PagePayload {
    pub id: u64,
    pub name: String,
    pub children: Vec<NodePayload>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodePayload {
    pub id: u64,
    pub kind: String,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    #[serde(default)]
    pub fill: Option<[f32; 4]>,
    #[serde(default)]
    pub stroke: Option<StrokePayload>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub rotation: f32,
    #[serde(default)]
    pub corner_radius: f32,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub collapsed: bool,
    #[serde(default)]
    pub fill_type: String,
    #[serde(default)]
    pub points: Vec<[f32; 2]>,
    #[serde(default)]
    pub children: Vec<NodePayload>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StrokePayload {
    pub color: [f32; 4],
    pub width: f32,
}

/// Build a serializable snapshot from the live `Document`.
pub fn to_payload(doc: &Document) -> DocPayload {
    DocPayload {
        version: CURRENT_VERSION,
        active_page_index: doc.active_page_index,
        pages: doc
            .pages
            .iter()
            .map(|p| PagePayload {
                id: p.id.raw(),
                name: p.name.clone(),
                children: p.children.iter().map(node_to_payload).collect(),
            })
            .collect(),
    }
}

fn node_to_payload(n: &Node) -> NodePayload {
    NodePayload {
        id: n.id.raw(),
        kind: kind_to_string(&n.kind),
        name: n.name.clone(),
        x: n.bounds.origin.x,
        y: n.bounds.origin.y,
        w: n.bounds.size.x,
        h: n.bounds.size.y,
        fill: n.fill.map(color_to_array),
        stroke: n.stroke.map(|s| StrokePayload {
            color: color_to_array(s.color),
            width: s.width,
        }),
        text: n.text.clone(),
        rotation: n.rotation,
        corner_radius: n.corner_radius,
        hidden: n.hidden,
        locked: n.locked,
        collapsed: n.collapsed,
        fill_type: fill_type_to_str(n.fill_type).to_string(),
        points: n.points.iter().map(|p| [p.x, p.y]).collect(),
        children: n.children.iter().map(node_to_payload).collect(),
    }
}

/// Replace `doc.pages` + `active_page_index` with `payload`. Returns
/// `Err` on unknown version. UI state (selection / viewport / chat
/// / modals) is left untouched — open should land you on a fresh
/// view of the loaded doc.
pub fn apply_payload(doc: &mut Document, payload: DocPayload) -> Result<(), String> {
    if payload.version != CURRENT_VERSION {
        return Err(format!(
            "unsupported file version: {} (this build expects {})",
            payload.version, CURRENT_VERSION
        ));
    }
    doc.pages = payload
        .pages
        .into_iter()
        .map(|p| Page {
            id: NodeId::new(p.id),
            name: p.name,
            children: p.children.into_iter().map(payload_to_node).collect(),
        })
        .collect();
    if doc.pages.is_empty() {
        // Defensive — never leave the document with zero pages.
        doc.pages.push(Page {
            id: NodeId::new(1),
            name: "Page 1".into(),
            children: Vec::new(),
        });
        doc.active_page_index = 0;
    } else {
        doc.active_page_index = payload.active_page_index.min(doc.pages.len() - 1);
    }
    // Open replaces the document tree — every reference to a NodeId
    // from the previous doc would point at a dead row. Wipe the
    // undo stack + any in-progress UI state so a stale `pen_in_progress
    // = Some(NodeId(42))` left over from the old doc doesn't trigger
    // a phantom pen render or a panic on the next press.
    doc.clear_selection();
    doc.history.past.clear();
    doc.history.future.clear();
    doc.ui.pen_in_progress = None;
    doc.ui.pen_cursor_doc = None;
    doc.ui.pending_pen_history = None;
    doc.ui.text_editing = None;
    doc.ui.layer_rename = None;
    doc.ui.color_picker = None;
    doc.ui.property_focus = None;
    doc.ui.property_input_draft.clear();
    doc.ui.settings_input_draft.clear();
    doc.ui.agent_settings.focus = None;
    doc.ui.agent_settings_drag = None;
    doc.ui.layer_context_menu = None;
    doc.ui.locale_picker_open = false;
    doc.ui.shape_picker_open = false;
    doc.ui.fill_type_picker_open = false;
    // Cross-doc clipboard contents are nodes carrying old NodeIds —
    // pasting them into the freshly-loaded doc would re-introduce
    // ids the new allocator doesn't know about (or collide with
    // freshly-loaded rows). Empty the clipboard on Open.
    doc.clipboard.clear();
    Ok(())
}

fn payload_to_node(n: NodePayload) -> Node {
    let kind = str_to_kind(&n.kind);
    let mut node = match &kind {
        NodeKind::Group | NodeKind::Frame => Node::with_children(
            n.id,
            kind.clone(),
            n.name,
            n.children.into_iter().map(payload_to_node).collect(),
        ),
        _ => Node::leaf(n.id, kind, n.name),
    };
    node.bounds = Rect {
        origin: Point2D::new(n.x, n.y),
        size: Point2D::new(n.w, n.h),
    };
    node.fill = n.fill.map(array_to_color);
    node.stroke = n.stroke.map(|s| Stroke {
        color: array_to_color(s.color),
        width: s.width,
    });
    node.text = n.text;
    node.rotation = n.rotation;
    node.corner_radius = n.corner_radius;
    node.hidden = n.hidden;
    node.locked = n.locked;
    node.collapsed = n.collapsed;
    node.fill_type = str_to_fill_type(&n.fill_type);
    node.points = n.points.iter().map(|p| Point2D::new(p[0], p[1])).collect();
    node
}

fn color_to_array(c: Color) -> [f32; 4] {
    [c.r, c.g, c.b, c.a]
}

fn array_to_color(a: [f32; 4]) -> Color {
    Color {
        r: a[0],
        g: a[1],
        b: a[2],
        a: a[3],
    }
}

fn kind_to_string(k: &NodeKind) -> String {
    match k {
        NodeKind::Frame => "frame".into(),
        NodeKind::Group => "group".into(),
        NodeKind::Rect => "rect".into(),
        NodeKind::Ellipse => "ellipse".into(),
        NodeKind::Polygon => "polygon".into(),
        NodeKind::Line => "line".into(),
        NodeKind::Text => "text".into(),
        NodeKind::Path => "path".into(),
        NodeKind::Other(s) => s.clone(),
    }
}

fn str_to_kind(s: &str) -> NodeKind {
    match s {
        "frame" => NodeKind::Frame,
        "group" => NodeKind::Group,
        "rect" => NodeKind::Rect,
        "ellipse" => NodeKind::Ellipse,
        "polygon" => NodeKind::Polygon,
        "line" => NodeKind::Line,
        "text" => NodeKind::Text,
        "path" => NodeKind::Path,
        other => NodeKind::Other(other.to_string()),
    }
}

fn fill_type_to_str(f: FillType) -> &'static str {
    match f {
        FillType::Solid => "solid",
        FillType::LinearGradient => "linear",
        FillType::RadialGradient => "radial",
        FillType::Image => "image",
    }
}

fn str_to_fill_type(s: &str) -> FillType {
    match s {
        "linear" => FillType::LinearGradient,
        "radial" => FillType::RadialGradient,
        "image" => FillType::Image,
        _ => FillType::Solid,
    }
}

/// Pop a Save dialog (rfd native) and write the current document
/// to the chosen path. Returns `Ok(Some(path))` on success,
/// `Ok(None)` on user cancel, `Err` on IO / encode failure.
pub fn save_as_dialog(doc: &Document) -> Result<Option<PathBuf>, String> {
    let path = rfd::FileDialog::new()
        .set_title("Save document")
        .add_filter("OpenPencil", &["pen", "op"])
        .set_file_name("untitled.pen")
        .save_file();
    let Some(path) = path else {
        return Ok(None);
    };
    save_to_path(doc, &path)?;
    Ok(Some(path))
}

/// Pop an Open dialog (rfd native) and load the chosen file into
/// `doc`. Returns `Ok(Some(path))` on success, `Ok(None)` on
/// cancel.
pub fn open_dialog(doc: &mut Document) -> Result<Option<PathBuf>, String> {
    let path = rfd::FileDialog::new()
        .set_title("Open document")
        .add_filter("OpenPencil", &["pen", "op"])
        .pick_file();
    let Some(path) = path else {
        return Ok(None);
    };
    load_from_path(doc, &path)?;
    Ok(Some(path))
}

/// Write the document to `path` without prompting. Used by Cmd+S
/// once the document already has a known path.
pub fn save_to_path(doc: &Document, path: &std::path::Path) -> Result<(), String> {
    let payload = to_payload(doc);
    let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    // Write through a sibling temp file so a crash mid-write
    // doesn't leave a half-written .pen on disk.
    let mut tmp = path.to_path_buf();
    tmp.set_extension(match path.extension().and_then(|s| s.to_str()) {
        Some(ext) => format!("{}.tmp", ext),
        None => "tmp".to_string(),
    });
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

fn load_from_path(doc: &mut Document, path: &std::path::Path) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let payload: DocPayload = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    apply_payload(doc, payload)
}

/// Cmd+S — save to `current_path` if known, else fall through to
/// Save As. Updates `current_path` + window title on success.
pub fn handle_save(
    host: &mut openpencil_shell_native::WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> bool {
    if let Some(path) = current_path.clone() {
        if let Err(e) = save_to_path(host.document(), &path) {
            eprintln!("[save] {e}");
        }
        return false;
    }
    handle_save_as(host, current_path, window)
}

/// Cmd+Shift+S — always pop the Save dialog.
pub fn handle_save_as(
    host: &mut openpencil_shell_native::WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> bool {
    match save_as_dialog(host.document()) {
        Ok(Some(path)) => {
            *current_path = Some(path);
            refresh_title(current_path, window);
            true
        }
        Ok(None) => false,
        Err(e) => {
            eprintln!("[save as] {e}");
            false
        }
    }
}

/// Cmd+O — pop the Open dialog and replace the current document.
pub fn handle_open(
    host: &mut openpencil_shell_native::WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> bool {
    match open_dialog(host.document_mut()) {
        Ok(Some(path)) => {
            *current_path = Some(path);
            refresh_title(current_path, window);
            true
        }
        Ok(None) => false,
        Err(e) => {
            eprintln!("[open] {e}");
            false
        }
    }
}

fn refresh_title(current_path: &Option<PathBuf>, window: Option<&winit::window::Window>) {
    let Some(window) = window else { return };
    let title = match current_path.as_ref().and_then(|p| p.file_name()) {
        Some(name) => format!("{} — OpenPencil", name.to_string_lossy()),
        None => "OpenPencil".to_string(),
    };
    window.set_title(&title);
}
