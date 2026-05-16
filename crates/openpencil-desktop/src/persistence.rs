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
    /// Design-token table. `#[serde(default)]` so a legacy `.op`
    /// saved before variables round-tripped still loads (empty
    /// table). Codex stop-gate: without this field every
    /// `set_variable_*` / `create|delete|rename_variable` change
    /// was dropped on save.
    #[serde(default)]
    pub var_table: crate::persistence_variables::VarTablePayload,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PagePayload {
    pub id: String,
    pub name: String,
    pub children: Vec<NodePayload>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodePayload {
    pub id: String,
    /// Original schema id (the string `id` in the `.op` file). Only
    /// populated when the payload was built from a canonical-schema
    /// load — used to look up layout rects after jian-core's
    /// LayoutEngine computes them. Empty for `DocPayload` that was
    /// round-tripped from a previous Save.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub schema_id: String,
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
    /// Text size in doc-px (matches `Node.font_size`). 0 = use the
    /// renderer's default 13 px. Text-only.
    #[serde(default)]
    pub font_size: f32,
    /// CSS-style font weight (100-900). 0 = default 400. Text-only.
    #[serde(default)]
    pub font_weight: u16,
    #[serde(default)]
    pub text_wrap: bool,
    /// Drop-shadow effects. `#[serde(default)]` so a legacy `.op`
    /// saved before effects round-tripped still loads (empty vec).
    #[serde(default)]
    pub effects: Vec<crate::persistence_effects::ShadowPayload>,
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
                id: p.id.raw().to_string(),
                name: p.name.clone(),
                children: p.children.iter().map(node_to_payload).collect(),
            })
            .collect(),
        var_table: crate::persistence_variables::var_table_to_payload(&doc.var_table),
    }
}

fn node_to_payload(n: &Node) -> NodePayload {
    NodePayload {
        id: n.id.raw().to_string(),
        schema_id: String::new(),
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
        font_size: n.font_size,
        font_weight: n.font_weight,
        text_wrap: n.text_wrap,
        effects: crate::persistence_effects::effects_to_payload(&n.effects),
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
            id: NodeId::new("n1"),
            name: "Page 1".into(),
            children: Vec::new(),
        });
        doc.active_page_index = 0;
    } else {
        doc.active_page_index = payload.active_page_index.min(doc.pages.len() - 1);
    }
    // Open replaces the document tree — wipe per-document state so
    // nothing from the previous doc leaks into the new one. Codex
    // BLOCK: var_table + components weren't being reset, so opening
    // a native saved file after a variable-bearing canonical file
    // surfaced stale variables in subsequent codegen.
    doc.clear_selection();
    doc.history.past.clear();
    doc.history.future.clear();
    // Restore the design-token table from the payload. A legacy
    // `.op` saved before variables round-tripped carries an empty
    // `var_table` (serde default), which correctly resets to a
    // blank table — same effect as the old unconditional wipe.
    doc.var_table =
        crate::persistence_variables::var_table_from_payload(&payload.var_table);
    doc.components = openpencil_shell_core::document::ComponentLibrary::default();
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
    // Reset viewport so loaded content is visible — without this
    // the previous session's pan/zoom can leave the new document
    // entirely off-screen, which surfaces as "Open did nothing".
    fit_viewport_to_active_page(doc);
    Ok(())
}

/// Pan + zoom so the active page's content fits the default
/// viewport area. Falls back to identity when the page is empty.
/// Uses a conservative AABB walk (same shape as `export::page_bounds`)
/// to find content extents.
fn fit_viewport_to_active_page(doc: &mut Document) {
    use openpencil_shell_core::Point2D;
    let Some(page) = doc.pages.get(doc.active_page_index) else {
        doc.viewport.pan_x = 0.0;
        doc.viewport.pan_y = 0.0;
        doc.viewport.zoom = 1.0;
        return;
    };
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    fn visit(
        n: &openpencil_shell_core::document::Node,
        min_x: &mut f32,
        min_y: &mut f32,
        max_x: &mut f32,
        max_y: &mut f32,
    ) {
        let r = n.aggregate_bounds();
        if r.size.x.abs() > 0.0 || r.size.y.abs() > 0.0 {
            let x0 = r.origin.x.min(r.origin.x + r.size.x);
            let y0 = r.origin.y.min(r.origin.y + r.size.y);
            let x1 = x0 + r.size.x.abs();
            let y1 = y0 + r.size.y.abs();
            *min_x = min_x.min(x0);
            *min_y = min_y.min(y0);
            *max_x = max_x.max(x1);
            *max_y = max_y.max(y1);
        }
        for c in &n.children {
            visit(c, min_x, min_y, max_x, max_y);
        }
    }
    for n in &page.children {
        visit(n, &mut min_x, &mut min_y, &mut max_x, &mut max_y);
    }
    if !min_x.is_finite() {
        // Empty page — leave the viewport at identity so a fresh
        // doc opens at world (0, 0).
        doc.viewport.pan_x = 0.0;
        doc.viewport.pan_y = 0.0;
        doc.viewport.zoom = 1.0;
        return;
    }
    // Centre the content's bounding box at world (0, 0) effectively
    // by panning so `min` lands at a small inset, and keep zoom=1.
    // This matches "open file = see the content" without trying to
    // be too clever about screen size (we don't have viewport_w/h
    // at this call site; main.rs's first paint applies DPI).
    let inset = 80.0;
    doc.viewport.pan_x = inset - min_x;
    doc.viewport.pan_y = inset - min_y;
    doc.viewport.zoom = 1.0;
    let _ = (max_x, max_y, Point2D::new(0.0, 0.0));
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
    node.font_size = n.font_size;
    node.font_weight = n.font_weight;
    node.text_wrap = n.text_wrap;
    node.effects = crate::persistence_effects::effects_from_payload(n.effects);
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

pub fn load_from_path(doc: &mut Document, path: &std::path::Path) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    // Try the desktop's private `DocPayload` first (round-trip with
    // its own Save), then fall back to the canonical `.op` /
    // `.pen` schema so files from the TS editor, Jian apps, or
    // anything else emitting the format load through the shared
    // parser instead of a private adapter. Side-channel: when the
    // canonical path fires, also harvest `variables` + `themes` into
    // a VariableTable for assignment after apply_payload (which
    // resets Document state via Default).
    let mut canonical_var_table: Option<openpencil_shell_core::document::VariableTable> = None;
    let payload = match serde_json::from_slice::<DocPayload>(&bytes) {
        Ok(p) => p,
        Err(_) => {
            let src = std::str::from_utf8(&bytes)
                .map_err(|e| format!("file is not valid UTF-8: {e}"))?;
            let loaded = load_canonical(src).map_err(|e| e.to_string())?;
            for w in &loaded.warnings {
                eprintln!("[open] schema warning: {:?}", w);
            }
            canonical_var_table = Some(crate::pen_doc_adapter::build_var_table(&loaded.value));
            let adapted = crate::pen_doc_adapter::pen_document_to_payload(&loaded.value);
            adapted.payload
        }
    };
    let page_count = payload.pages.len();
    let node_count: usize = payload.pages.iter().map(|p| count_nodes(&p.children)).sum();
    apply_payload(doc, payload)?;
    if let Some(tbl) = canonical_var_table {
        doc.var_table = tbl;
    }
    let bb = active_page_bbox(doc);
    eprintln!(
        "[open] {} pages, {} nodes; content bbox {:?}; viewport pan=({:.1},{:.1}) zoom={:.2}",
        page_count, node_count, bb, doc.viewport.pan_x, doc.viewport.pan_y, doc.viewport.zoom
    );
    Ok(())
}

/// Wrapper around `jian_ops_schema::load_str` that retries with the
/// document's `version` field rewritten to "1.0" when the canonical
/// loader rejects on an unrecognised major (e.g. the TS app's
/// `version: "2.8"` for the legacy pre-Jian format). The schema is
/// intentionally strict on rejection so callers can opt in to
/// "best-effort parse"; the desktop's Open dialog is one of those
/// callers — better to surface partial geometry than refuse the
/// file outright.
pub(crate) fn load_canonical(
    src: &str,
) -> Result<
    jian_ops_schema::LoadResult<jian_ops_schema::PenDocument>,
    jian_ops_schema::OpsSchemaError,
> {
    match jian_ops_schema::load_str(src) {
        Ok(loaded) => Ok(loaded),
        Err(jian_ops_schema::OpsSchemaError::UnsupportedFormatVersion { found, .. }) => {
            eprintln!(
                "[open] file version {} is outside the canonical schema's supported range; \
                 retrying as v1.0 (best-effort)",
                found
            );
            let mut value: serde_json::Value = serde_json::from_str(src)?;
            if let Some(obj) = value.as_object_mut() {
                obj.insert(
                    "version".to_string(),
                    serde_json::Value::String("1.0".to_string()),
                );
                obj.remove("formatVersion");
            }
            let patched = serde_json::to_string(&value)?;
            jian_ops_schema::load_str(&patched)
        }
        Err(other) => Err(other),
    }
}

fn count_nodes(nodes: &[NodePayload]) -> usize {
    nodes
        .iter()
        .map(|n| 1 + count_nodes(&n.children))
        .sum()
}

fn active_page_bbox(doc: &Document) -> Option<(f32, f32, f32, f32)> {
    let page = doc.pages.get(doc.active_page_index)?;
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    fn visit(
        n: &openpencil_shell_core::document::Node,
        mn_x: &mut f32,
        mn_y: &mut f32,
        mx_x: &mut f32,
        mx_y: &mut f32,
    ) {
        let r = n.bounds;
        let x0 = r.origin.x.min(r.origin.x + r.size.x);
        let y0 = r.origin.y.min(r.origin.y + r.size.y);
        let x1 = x0 + r.size.x.abs();
        let y1 = y0 + r.size.y.abs();
        if r.size.x.abs() > 0.0 || r.size.y.abs() > 0.0 {
            *mn_x = mn_x.min(x0);
            *mn_y = mn_y.min(y0);
            *mx_x = mx_x.max(x1);
            *mx_y = mx_y.max(y1);
        }
        for c in &n.children {
            visit(c, mn_x, mn_y, mx_x, mx_y);
        }
    }
    for n in &page.children {
        visit(n, &mut min_x, &mut min_y, &mut max_x, &mut max_y);
    }
    if !min_x.is_finite() {
        None
    } else {
        Some((min_x, min_y, max_x, max_y))
    }
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
            show_error_dialog(host.document(), ErrorKind::Save, Some(&path), &e);
        } else {
            crate::settings_io::touch_recent(host.document_mut(), &path);
            set_display_name(host.document_mut(), Some(&path));
        }
        return false;
    }
    handle_save_as(host, current_path, window)
}

fn set_display_name(doc: &mut openpencil_shell_core::document::Document, path: Option<&std::path::Path>) {
    doc.ui.file_name_display = path
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned());
}

/// Cmd+Shift+S — always pop the Save dialog.
pub fn handle_save_as(
    host: &mut openpencil_shell_native::WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> bool {
    match save_as_dialog(host.document()) {
        Ok(Some(path)) => {
            crate::settings_io::touch_recent(host.document_mut(), &path);
            *current_path = Some(path);
            refresh_title(current_path, window);
            true
        }
        Ok(None) => false,
        Err(e) => {
            eprintln!("[save as] {e}");
            show_error_dialog(host.document(), ErrorKind::Save, None, &e);
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
    // `open_dialog` borrows the document mutably to call
    // `load_from_path`; the user picks a path inside, so we can't
    // know it until after the call. Failed-parse dialogs include
    // the path the user just chose, which `open_dialog` doesn't
    // hand back — open the file dialog directly here so the path
    // is available for the error dialog.
    let path = match rfd::FileDialog::new()
        .set_title("Open document")
        .add_filter("OpenPencil", &["pen", "op"])
        .pick_file()
    {
        Some(p) => p,
        None => return false,
    };
    match load_from_path(host.document_mut(), &path) {
        Ok(()) => {
            crate::settings_io::touch_recent(host.document_mut(), &path);
            *current_path = Some(path);
            refresh_title(current_path, window);
            true
        }
        Err(e) => {
            eprintln!("[open] {e}");
            show_error_dialog(host.document(), ErrorKind::Open, Some(&path), &e);
            false
        }
    }
}

/// Route a `FileAction` raised by the file-menu dispatcher to the
/// matching dialog flow. Ignores `OpenRecent` / `ClearRecent` for
/// now — recents land in the next pass.
pub fn run_action(
    action: openpencil_shell_core::document::FileAction,
    host: &mut openpencil_shell_native::WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) {
    use openpencil_shell_core::document::FileAction;
    match action {
        FileAction::New => {
            // Reset the document tree to a fresh untitled page.
            let payload = DocPayload {
                version: 1,
                active_page_index: 0,
                pages: vec![PagePayload {
                    id: "n1".to_string(),
                    name: "Page 1".into(),
                    children: Vec::new(),
                }],
                // Fresh document — no design tokens.
                var_table: crate::persistence_variables::VarTablePayload::default(),
            };
            let _ = apply_payload(host.document_mut(), payload);
            *current_path = None;
            refresh_title(current_path, window);
        }
        FileAction::Open => {
            handle_open(host, current_path, window);
        }
        FileAction::Save => {
            handle_save(host, current_path, window);
        }
        FileAction::SaveAs => {
            handle_save_as(host, current_path, window);
        }
        FileAction::ExportImage => {
            // Should not normally hit run_action — main.rs intercepts
            // ExportImage to open the picker dialog instead. Keep as
            // a fallback so external callers still work.
            host.document_mut().ui.export_dialog_open = true;
        }
        FileAction::ExportImageConfirm => {
            use openpencil_shell_core::widgets::export_dialog::ExportFormat as Fmt;
            let fmt = host.document().ui.export_format;
            let scale = host.document().ui.export_scale;
            let (filter_label, filter_exts): (&str, &[&str]) = match fmt {
                Fmt::Png => ("PNG", &["png"]),
                Fmt::Jpeg => ("JPEG", &["jpg", "jpeg"]),
                Fmt::Webp => ("WEBP", &["webp"]),
                Fmt::Svg => ("SVG", &["svg"]),
                Fmt::Pdf => ("PDF", &["pdf"]),
            };
            let default_name = format!("openpencil-export.{}", fmt.extension());
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Export image")
                .add_filter(filter_label, filter_exts)
                .set_file_name(&default_name)
                .save_file()
            {
                let result: Result<(), String> = match fmt {
                    Fmt::Png => crate::export::export_raster(
                        host.document(),
                        &path,
                        crate::export::RasterFormat::Png,
                        scale,
                    ),
                    Fmt::Jpeg => crate::export::export_raster(
                        host.document(),
                        &path,
                        crate::export::RasterFormat::Jpeg,
                        scale,
                    ),
                    Fmt::Webp => crate::export::export_raster(
                        host.document(),
                        &path,
                        crate::export::RasterFormat::Webp,
                        scale,
                    ),
                    Fmt::Svg => crate::export::export_svg(host.document(), &path),
                    Fmt::Pdf => crate::export_pdf::export_pdf(host.document(), &path),
                };
                if let Err(e) = result {
                    eprintln!("[export-image] {e}");
                    show_error_dialog(host.document(), ErrorKind::Export, Some(&path), &e);
                }
            }
        }
        FileAction::OpenRecent(i) => {
            let Some(entry) = host.document().ui.recent_files.get(i).cloned() else {
                return;
            };
            let path = std::path::PathBuf::from(&entry.path);
            match load_from_path(host.document_mut(), &path) {
                Ok(()) => {
                    crate::settings_io::touch_recent(host.document_mut(), &path);
                    *current_path = Some(path);
                    refresh_title(current_path, window);
                }
                Err(e) => {
                    // File missing / parse failure → tell the user and
                    // drop the entry from recents so a stale path
                    // doesn't keep haunting the menu.
                    eprintln!("[open-recent] {e}; pruning {}", entry.path);
                    show_error_dialog(host.document(), ErrorKind::Open, Some(&path), &e);
                    host.document_mut()
                        .ui
                        .recent_files
                        .retain(|r| r.path != entry.path);
                }
            }
        }
        FileAction::ClearRecent => {
            host.document_mut().ui.recent_files.clear();
        }
        FileAction::ImportFigma => {
            eprintln!("[file-action] {action:?} — not yet wired (UI only)");
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

/// Pop a native error dialog. Used by Open / Save / Export when the
/// underlying IO or parse step fails so the user gets feedback
/// instead of a silent no-op + stderr line they'll never see.
fn show_error_dialog(
    doc: &openpencil_shell_core::document::Document,
    kind: ErrorKind,
    path: Option<&std::path::Path>,
    detail: &str,
) {
    use openpencil_shell_core::document::Locale;
    let zh = matches!(doc.ui.locale, Locale::ZhCn | Locale::ZhTw);
    let (title, lead) = match (kind, zh) {
        (ErrorKind::Open, true) => ("无法打开文件", "OpenPencil 无法解析该文件。"),
        (ErrorKind::Open, false) => ("Couldn't open file", "OpenPencil could not parse the file."),
        (ErrorKind::Save, true) => ("保存失败", "写入文件时出错。"),
        (ErrorKind::Save, false) => ("Save failed", "An error occurred while writing the file."),
        (ErrorKind::Export, true) => ("导出失败", "渲染图像时出错。"),
        (ErrorKind::Export, false) => {
            ("Export failed", "An error occurred while rendering the image.")
        }
    };
    let mut body = lead.to_string();
    if let Some(p) = path {
        body.push_str("\n\n");
        body.push_str(&p.display().to_string());
    }
    body.push_str("\n\n");
    body.push_str(detail);
    rfd::MessageDialog::new()
        .set_title(title)
        .set_description(&body)
        .set_level(rfd::MessageLevel::Error)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

#[derive(Debug, Clone, Copy)]
enum ErrorKind {
    Open,
    Save,
    Export,
}
