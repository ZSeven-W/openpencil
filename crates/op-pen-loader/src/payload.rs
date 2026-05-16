//! `.pen` / `.op` payload DTO + `PenDocument` → shell `Document`
//! conversion.
//!
//! Persists the document tree (pages + nodes) to JSON via a small
//! hand-rolled DTO. shell-core stays serde-free; the conversion
//! lives here so Color / Rect / Point2D (external types) don't need
//! wrapper plumbing on the library side.
//!
//! Format: `{ "version": 1, "active_page_index": 0, "pages": [...] }`.
//! Bumping the version requires a migration step in `apply_payload`.
//!
//! Carved out of `openpencil-desktop/src/persistence.rs` so library
//! crates can reuse the canonical `.op` loader; the desktop binary's
//! `rfd` Save/Open dialogs + error dialogs stay desktop-side.

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
    pub var_table: crate::variables::VarTablePayload,
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
    pub effects: Vec<crate::effects::ShadowPayload>,
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
        var_table: crate::variables::var_table_to_payload(&doc.var_table),
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
        effects: crate::effects::effects_to_payload(&n.effects),
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
    doc.var_table = crate::variables::var_table_from_payload(&payload.var_table);
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
    node.effects = crate::effects::effects_from_payload(n.effects);
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

/// Wrapper around `jian_ops_schema::load_str` that retries with the
/// document's `version` field rewritten to "1.0" when the canonical
/// loader rejects on an unrecognised major (e.g. the TS app's
/// `version: "2.8"` for the legacy pre-Jian format). The schema is
/// intentionally strict on rejection so callers can opt in to
/// "best-effort parse"; the desktop's Open dialog is one of those
/// callers — better to surface partial geometry than refuse the
/// file outright.
pub fn load_canonical(
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

/// Canonical entry point: convert a parsed `PenDocument` directly
/// into a shell-core `Document`. Runs each page-root through
/// jian-core's `LayoutEngine` (baking flex layout into AABB rects),
/// then folds the result + the design-token table into a fresh
/// `Document`. UI state stays at its `Document::empty()` values
/// apart from the viewport, which `apply_payload` fits to content.
pub fn pen_document_to_document(doc: &jian_ops_schema::PenDocument) -> Document {
    let loaded = crate::adapter::pen_document_to_payload(doc);
    let var_table = crate::adapter::build_var_table(doc);
    let mut out = Document::empty();
    // apply_payload only ever fails on an unknown `version`; the
    // payload we just built is always `CURRENT_VERSION`, so the
    // result is infallible here.
    let _ = apply_payload(&mut out, loaded.payload);
    out.var_table = var_table;
    out
}
