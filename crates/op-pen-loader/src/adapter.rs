//! Canonical `.op` / `.pen` loader.
//!
//! Bridges the `jian-ops-schema` canonical `PenDocument` into the
//! desktop's private `DocPayload`. Two responsibilities:
//!
//! 1. Convert each `PenNode` variant into a `NodePayload` carrying
//!    geometry + style. All 12 schema variants are routed.
//! 2. Defer flex layout to `jian-core::LayoutEngine` — the same
//!    taffy-backed solver that drives the (read-only) jian runtime
//!    against the same schema. Reusing it keeps OpenPencil's
//!    rendering bit-identical with what the TS editor and the
//!    canonical jian renderer produce.
//!
//! OpenPencil's canvas is infinite + unrouted, so each page-root
//! gets its own `LayoutEngine::compute` pass with `available =
//! (root_w, root_h)` (or a generous default when the root is
//! `fit_content`). Computed absolute scene-coord rects are baked
//! into each `NodePayload.bounds`.

use std::collections::BTreeMap;

use jian_core::document::NodeTree;
use jian_core::layout::LayoutEngine;
use jian_ops_schema::{
    node::{
        EllipseNode, FrameNode, FontWeight, GroupNode, IconFontNode, ImageNode, LineNode,
        PathNode, PenNode, PolygonNode, RectangleNode, TextNode, TextInputNode,
    },
    node::base::PenNodeBase,
    node::container::CornerRadius,
    node::text::TextContent,
    sizing::SizingBehavior,
    style::{PenFill, PenStroke, StrokeThickness},
    PenDocument,
};

use crate::payload::{DocPayload, NodePayload, PagePayload, StrokePayload};

/// Default canvas allotment for a page-root sized with flex tokens
/// (`fill_container` / `fit_content`) and no authored bounds — large
/// enough to let real designs fill out without truncating layout,
/// small enough to avoid pathological taffy work.
const ROOT_FALLBACK_W: f32 = 1440.0;
const ROOT_FALLBACK_H: f32 = 900.0;

pub struct LoadedDoc {
    pub payload: DocPayload,
}

/// Convert a parsed `PenDocument` into the desktop's `DocPayload`,
/// running each page-root through jian-core's `LayoutEngine` so
/// flex sizes resolve to absolute scene-coord rects before paint.
pub fn pen_document_to_payload(doc: &PenDocument) -> LoadedDoc {
    let pages: Vec<PagePayload> = if let Some(pages) = &doc.pages {
        pages
            .iter()
            .enumerate()
            .map(|(i, p)| build_page(&p.id, &p.name, &p.children, i))
            .collect()
    } else if !doc.children.is_empty() {
        // Single-page fallback (TS shape: top-level `children`).
        vec![build_page(
            "page-1",
            doc.name.as_deref().unwrap_or("Page 1"),
            &doc.children,
            0,
        )]
    } else {
        vec![PagePayload {
            id: "n1".to_string(),
            name: "Page 1".into(),
            children: Vec::new(),
        }]
    };
    LoadedDoc {
        payload: DocPayload {
            version: 1,
            active_page_index: 0,
            pages,
            // Canonical-schema variables are harvested separately
            // by `build_var_table` and assigned after apply_payload;
            // this private-payload field stays empty for that path.
            var_table: crate::variables::VarTablePayload::default(),
        },
    }
}

/// Copy `PenDocument.variables` + `.themes` into a shell-core
/// `VariableTable`. Caller assigns the result to `Document.var_table`
/// AFTER `apply_payload` (which clears it via Default). Lossless on
/// the supported `VariableDefinition` variants; unknown future
/// `VariableKind`s round-trip via their `Color/Number/Boolean/String`
/// label since the enums are isomorphic.
pub fn build_var_table(doc: &PenDocument) -> openpencil_shell_core::document::VariableTable {
    use openpencil_shell_core::document::{
        ThemeAxis, ThemedValue, Variable, VariableKind, VariableTable, VariableValue,
    };
    let mut out = VariableTable::default();
    if let Some(themes) = &doc.themes {
        for (axis_name, values) in themes {
            out.themes.push(ThemeAxis {
                name: axis_name.clone(),
                values: values.clone(),
            });
        }
    }
    if let Some(vars) = &doc.variables {
        for (name, def) in vars {
            let kind = match def.kind {
                jian_ops_schema::variable::VariableKind::Color => VariableKind::Color,
                jian_ops_schema::variable::VariableKind::Number => VariableKind::Number,
                jian_ops_schema::variable::VariableKind::Boolean => VariableKind::Boolean,
                jian_ops_schema::variable::VariableKind::String => VariableKind::String,
            };
            let value = match &def.value {
                jian_ops_schema::variable::VariableValue::Scalar(s) => {
                    VariableValue::Scalar(map_scalar(s))
                }
                jian_ops_schema::variable::VariableValue::Themed(arr) => VariableValue::Themed(
                    arr.iter()
                        .map(|tv| ThemedValue {
                            value: map_scalar(&tv.value),
                            theme: tv.theme.clone(),
                        })
                        .collect(),
                ),
            };
            out.variables.push(Variable {
                name: name.clone(),
                kind,
                value,
            });
        }
    }
    out
}

fn map_scalar(
    s: &jian_ops_schema::variable::VariableScalar,
) -> openpencil_shell_core::document::VariableScalar {
    use openpencil_shell_core::document::VariableScalar;
    match s {
        jian_ops_schema::variable::VariableScalar::Bool(b) => VariableScalar::Bool(*b),
        jian_ops_schema::variable::VariableScalar::Num(n) => VariableScalar::Num(*n),
        jian_ops_schema::variable::VariableScalar::Str(s) => VariableScalar::Str(s.clone()),
    }
}

fn build_page(id: &str, name: &str, roots: &[PenNode], page_idx: usize) -> PagePayload {
    let mut layout_rects: BTreeMap<String, [f32; 4]> = BTreeMap::new();
    for root in roots {
        compute_layout(root, &mut layout_rects);
    }
    let _ = page_idx;
    PagePayload {
        id: id.to_string(),
        name: name.to_string(),
        children: roots
            .iter()
            .map(|n| node_to_payload(n, &layout_rects))
            .collect(),
    }
}

/// Run jian-core's `LayoutEngine` on `root` and harvest absolute
/// rects per schema id into `out`. Each page root gets its own
/// `LayoutEngine` instance — OpenPencil's canvas is infinite, so
/// roots don't share a coordinate frame; we offset every harvested
/// rect by the root's authored `(base.x, base.y)` so multiple
/// designs on a page (e.g. pencil-demo.op's 14 side-by-side
/// mock-ups) don't all collapse to (0, 0).
fn compute_layout(root: &PenNode, out: &mut BTreeMap<String, [f32; 4]>) {
    let (root_w, root_h) = root_available_size(root);
    let (root_ox, root_oy) = root_authored_origin(root);
    let mut tree = NodeTree::new();
    tree.insert_subtree(root.clone(), None);
    // Real-skia text measurement via jian-skia's `SkiaMeasure`
    // (paragraph shaper). The default `EstimateBackend` is a
    // character-count heuristic accurate to ~10% — for any
    // `fit_content` frame whose size depends on text length the
    // 10% error cascades through every flex parent. SkiaMeasure
    // matches what the canvas painter actually draws so the
    // engine + paint agree on widths.
    let mut engine = LayoutEngine::with_backend(std::rc::Rc::new(jian_skia::SkiaMeasure::new()));
    let Ok(taffy_roots) = engine.build(&tree) else {
        return;
    };
    let Some(root_id) = taffy_roots.first() else {
        return;
    };
    if engine.compute(*root_id, (root_w, root_h)).is_err() {
        return;
    }
    // Walk every node by SlotMap iteration, looking up its absolute
    // rect via `node_rect`. Keyed back by the schema id we stashed
    // in `tree.by_id` during insertion. Adds the root's authored
    // canvas offset so each design sits where the file placed it.
    for (id_str, node_key) in tree.by_id.iter() {
        if let Some(rect) = engine.node_rect(*node_key) {
            out.insert(
                id_str.clone(),
                [
                    rect.origin.x + root_ox,
                    rect.origin.y + root_oy,
                    rect.size.width,
                    rect.size.height,
                ],
            );
        }
    }
}

/// `(base.x, base.y)` for any `PenNode`, defaulting to `(0, 0)`
/// when the schema didn't author them. Used by `compute_layout`
/// to offset taffy's root-relative rects onto the infinite canvas.
fn root_authored_origin(n: &PenNode) -> (f32, f32) {
    let base = match n {
        PenNode::Frame(f) => &f.base,
        PenNode::Group(g) => &g.base,
        PenNode::Rectangle(r) => &r.base,
        PenNode::Ellipse(e) => &e.base,
        PenNode::Line(l) => &l.base,
        PenNode::Polygon(p) => &p.base,
        PenNode::Path(p) => &p.base,
        PenNode::Text(t) => &t.base,
        PenNode::TextInput(t) => &t.base,
        PenNode::Image(i) => &i.base,
        PenNode::IconFont(i) => &i.base,
        PenNode::Ref(r) => &r.base,
    };
    (
        base.x.unwrap_or(0.0) as f32,
        base.y.unwrap_or(0.0) as f32,
    )
}

/// Choose the (available_width, available_height) the layout engine
/// solves the root against. Numeric roots use their authored size;
/// flex-token roots fall back to a generous default that doesn't
/// drive taffy into trying to wrap a 1×1 canvas.
fn root_available_size(root: &PenNode) -> (f32, f32) {
    let (w_sizing, h_sizing) = root_sizing(root);
    let w = match w_sizing {
        Some(SizingBehavior::Number(n)) => n as f32,
        _ => ROOT_FALLBACK_W,
    };
    let h = match h_sizing {
        Some(SizingBehavior::Number(n)) => n as f32,
        _ => ROOT_FALLBACK_H,
    };
    (w, h)
}

fn root_sizing(root: &PenNode) -> (Option<SizingBehavior>, Option<SizingBehavior>) {
    match root {
        PenNode::Frame(f) => (f.container.width.clone(), f.container.height.clone()),
        PenNode::Group(g) => (g.container.width.clone(), g.container.height.clone()),
        PenNode::Rectangle(r) => (r.container.width.clone(), r.container.height.clone()),
        PenNode::Ellipse(e) => (e.width.clone(), e.height.clone()),
        PenNode::Text(t) => (t.width.clone(), t.height.clone()),
        PenNode::TextInput(t) => (t.width.clone(), t.height.clone()),
        PenNode::Image(i) => (i.width.clone(), i.height.clone()),
        PenNode::IconFont(i) => (i.width.clone(), i.height.clone()),
        PenNode::Polygon(p) => (p.width.clone(), p.height.clone()),
        PenNode::Path(p) => (p.width.clone(), p.height.clone()),
        _ => (None, None),
    }
}

fn node_to_payload(node: &PenNode, rects: &BTreeMap<String, [f32; 4]>) -> NodePayload {
    let mut p = match node {
        PenNode::Frame(n) => frame_to_payload(n, rects),
        PenNode::Group(n) => group_to_payload(n, rects),
        PenNode::Rectangle(n) => rect_to_payload(n, rects),
        PenNode::Ellipse(n) => ellipse_to_payload(n),
        PenNode::Line(n) => line_to_payload(n),
        PenNode::Polygon(n) => polygon_to_payload(n),
        PenNode::Path(n) => path_to_payload(n),
        PenNode::Text(n) => text_to_payload(n),
        PenNode::TextInput(n) => text_input_to_payload(n),
        PenNode::Image(n) => image_to_payload(n),
        PenNode::IconFont(n) => icon_font_to_payload(n),
        PenNode::Ref(n) => empty_group(&n.base, "ref"),
    };
    // The line painter is special-cased on signed bounds, so don't
    // overwrite its hand-encoded geometry with the taffy AABB.
    if !matches!(node, PenNode::Line(_)) {
        apply_computed_rect(&mut p, rects);
    }
    // Canonical `PathNode.anchors` need the same transform the TS
    // renderer applies in `pen-renderer/node-renderer.ts::drawPath`:
    // compute the local geometry bounds (including Bezier handles
    // and cubic curve extrema, per
    // `pen-core/path-anchors.ts::getPathBoundsFromAnchors`), then
    // map each anchor onto canvas-absolute via
    // `(x + (anchor.x - bounds_min_x) * sx, …)`. Endpoint-only
    // bounds are wrong for curved paths because cubic Beziers can
    // extend well past their anchor endpoints.
    if let PenNode::Path(path) = node {
        if !p.points.is_empty() {
            absolutize_path_anchors(&mut p, path);
        }
    }
    // Carry canonical drop-shadow effects across — without this a
    // `.op` authored with shadows lost them on import (codex
    // stop-gate). Blur / background-blur are skipped: the shell's
    // `Effect` model is drop-shadow-only today.
    p.effects = crate::effects::shadows_from_canonical(node);
    p
}

/// Translate `p.points` from local-to-`base.x/base.y` into the
/// canvas-absolute frame the shell's path painter expects.
/// Mirrors `pen-renderer/node-renderer.ts::drawPath`:
/// - Local geometry bounds come from `path_bounds_from_anchors`
///   (curve extrema + handle-extended segments), not just the
///   anchor endpoints, so a path whose handles bow well past its
///   endpoints still scales correctly.
/// - Scale = explicit `width`/`height` over native span.
/// - Translate so the local geometry's top-left lands at `(p.x, p.y)`.
fn absolutize_path_anchors(p: &mut NodePayload, path: &PathNode) {
    let closed = path.closed.unwrap_or(false);
    let bounds = path_bounds_from_anchors(path.anchors.as_deref().unwrap_or(&[]), closed);
    let (min_x, min_y, native_w, native_h) = bounds;
    let sx = if native_w > 0.01 && p.w > 0.0 { p.w / native_w } else { 1.0 };
    let sy = if native_h > 0.01 && p.h > 0.0 { p.h / native_h } else { 1.0 };
    let (ox, oy) = (p.x, p.y);
    for pt in &mut p.points {
        pt[0] = ox + (pt[0] - min_x) * sx;
        pt[1] = oy + (pt[1] - min_y) * sy;
    }
}

/// Replace `(x, y, w, h)` on `p` with the absolute scene-coord rect
/// the layout engine resolved for this node. Width / height fall
/// back to authored size only when taffy reports `Size::ZERO` (the
/// `leaf_size` resolver covers text / text_input / icon_font /
/// image but returns `(None, None)` for ellipse / polygon / path).
/// Position always uses the layout engine's `(x, y)` so the root's
/// canvas offset propagates onto zero-size shape fallbacks too —
/// otherwise an authored `x=20, y=30` ellipse inside a root at
/// `(-1098, 2963)` would paint at world `(20, 30)` instead of
/// `(-1078, 2993)` and detach from its parent design.
fn apply_computed_rect(p: &mut NodePayload, rects: &BTreeMap<String, [f32; 4]>) {
    if let Some([x, y, w, h]) = rects.get(&p.schema_id).copied() {
        p.x = x;
        p.y = y;
        if w > 0.0 {
            p.w = w;
        }
        if h > 0.0 {
            p.h = h;
        }
    }
}

/// Numeric width/height from a schema sizing field. Flex tokens
/// (`fill_container` / `fit_content`) and expressions collapse to
/// 0; jian-core's taffy compute fills those in via the layout map.
fn sizing_to_f32(s: &Option<SizingBehavior>) -> f32 {
    match s {
        Some(SizingBehavior::Number(n)) => *n as f32,
        _ => 0.0,
    }
}

fn frame_to_payload(n: &FrameNode, _rects: &BTreeMap<String, [f32; 4]>) -> NodePayload {
    let mut p = base_payload(&n.base, "frame");
    apply_container_style(&mut p, n.container.fill.as_deref(), n.container.stroke.as_ref(), n.container.corner_radius.as_ref());
    p.children = n
        .children
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|c| node_to_payload(c, _rects))
        .collect();
    p
}

fn group_to_payload(n: &GroupNode, _rects: &BTreeMap<String, [f32; 4]>) -> NodePayload {
    let mut p = base_payload(&n.base, "group");
    apply_container_style(&mut p, n.container.fill.as_deref(), n.container.stroke.as_ref(), n.container.corner_radius.as_ref());
    p.children = n
        .children
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|c| node_to_payload(c, _rects))
        .collect();
    p
}

fn rect_to_payload(n: &RectangleNode, _rects: &BTreeMap<String, [f32; 4]>) -> NodePayload {
    let mut p = base_payload(&n.base, "rect");
    apply_container_style(&mut p, n.container.fill.as_deref(), n.container.stroke.as_ref(), n.container.corner_radius.as_ref());
    p.children = n
        .children
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|c| node_to_payload(c, _rects))
        .collect();
    p
}

fn ellipse_to_payload(n: &EllipseNode) -> NodePayload {
    let mut p = base_payload(&n.base, "ellipse");
    // jian-core's `leaf_size` doesn't expose ellipse dimensions to
    // taffy, so the computed rect comes back zero. Seed authored
    // numeric size here as the fallback for `apply_computed_rect`.
    p.w = sizing_to_f32(&n.width);
    p.h = sizing_to_f32(&n.height);
    p.fill = first_solid_color(n.fill.as_deref());
    p.fill_type = first_fill_type(n.fill.as_deref());
    p.stroke = stroke_to_payload(n.stroke.as_ref());
    p.corner_radius = n.corner_radius.unwrap_or(0.0) as f32;
    p
}

fn line_to_payload(n: &LineNode) -> NodePayload {
    let mut p = base_payload(&n.base, "line");
    // The shell's line painter draws from `bounds.origin` to
    // `bounds.origin + bounds.size`, so encoding `(x2, y2)` as
    // a signed size puts both endpoints exactly where the
    // canonical schema says they are. Negative components shift
    // `origin` so `aggregate_bounds` (which gates on `size > 0`)
    // still picks the rect up — a horizontal/vertical/diagonal
    // line is direction-invariant for paint purposes anyway.
    let x2 = n.x2.unwrap_or(0.0) as f32;
    let y2 = n.y2.unwrap_or(0.0) as f32;
    if x2 < 0.0 {
        p.x += x2;
        p.w = -x2;
    } else {
        p.w = x2;
    }
    if y2 < 0.0 {
        p.y += y2;
        p.h = -y2;
    } else {
        p.h = y2;
    }
    if p.w == 0.0 && p.h == 0.0 {
        p.w = 1.0;
    }
    p.stroke = stroke_to_payload(n.stroke.as_ref());
    if p.stroke.is_none() {
        p.stroke = Some(StrokePayload {
            color: [0.0, 0.0, 0.0, 1.0],
            width: 1.0,
        });
    }
    p
}

fn polygon_to_payload(n: &PolygonNode) -> NodePayload {
    let mut p = base_payload(&n.base, "polygon");
    // Same as ellipse — jian-core's leaf_size doesn't expose
    // polygon dimensions to taffy, so we must seed authored size.
    p.w = sizing_to_f32(&n.width);
    p.h = sizing_to_f32(&n.height);
    p.fill = first_solid_color(n.fill.as_deref());
    p.fill_type = first_fill_type(n.fill.as_deref());
    p.stroke = stroke_to_payload(n.stroke.as_ref());
    p.corner_radius = n.corner_radius.unwrap_or(0.0) as f32;
    p
}

fn path_to_payload(n: &PathNode) -> NodePayload {
    let mut p = base_payload(&n.base, "path");
    p.w = sizing_to_f32(&n.width);
    p.h = sizing_to_f32(&n.height);
    p.fill = first_solid_color(n.fill.as_deref());
    p.fill_type = first_fill_type(n.fill.as_deref());
    p.stroke = stroke_to_payload(n.stroke.as_ref());
    if let Some(anchors) = &n.anchors {
        p.points = anchors
            .iter()
            .map(|a| [a.x as f32, a.y as f32])
            .collect();
        // Anchor-bounded path: when width/height weren't authored,
        // derive size from the anchor bbox span (max - min) — using
        // raw `max` assumed min=0, which is wrong for paths whose
        // first anchor sits at, e.g., (10, 20).
        if p.w == 0.0 && p.h == 0.0 && !p.points.is_empty() {
            let (mut min_x, mut min_y) = (f32::INFINITY, f32::INFINITY);
            let (mut max_x, mut max_y) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
            for [x, y] in &p.points {
                min_x = min_x.min(*x);
                min_y = min_y.min(*y);
                max_x = max_x.max(*x);
                max_y = max_y.max(*y);
            }
            p.w = (max_x - min_x).max(0.0);
            p.h = (max_y - min_y).max(0.0);
        }
    }
    p
}

fn text_to_payload(n: &TextNode) -> NodePayload {
    let mut p = base_payload(&n.base, "text");
    p.text = Some(match &n.content {
        TextContent::Plain(s) => s.clone(),
        TextContent::Styled(segments) => segments
            .iter()
            .map(|seg| seg.text.clone())
            .collect::<Vec<_>>()
            .join(""),
    });
    p.fill = first_solid_color(n.fill.as_deref());
    p.fill_type = first_fill_type(n.fill.as_deref());
    p.font_size = n.font_size.unwrap_or(0.0) as f32;
    p.font_weight = resolve_font_weight(n.font_weight.as_ref());
    // Only wrap text when the schema explicitly authored
    // `textGrowth: fixed-width` (or fixed-width-and-height) —
    // matches canonical paint behaviour. Default-growth text was
    // authored against the TS app's measureText for a font we
    // may not have bundled; wrapping it would mis-break lines
    // the TS app shows on one line.
    use jian_ops_schema::node::TextGrowth;
    p.text_wrap = matches!(
        n.text_growth,
        Some(TextGrowth::FixedWidth) | Some(TextGrowth::FixedWidthHeight)
    );
    p
}

/// CSS-style numeric weight from the schema's `FontWeight` union.
/// Returns 0 when the field is absent so the renderer falls back
/// to its default — keeps the payload free of duplicate defaults.
///
/// Real `.op` files often emit `"fontWeight":"700"` as a JSON
/// STRING (the canonical untagged enum picks `Keyword(String)`
/// when the JSON type is a string, even with numeric contents).
/// Parse numeric keywords first, then fall back to lucide-style
/// named weights. Stays in sync with `jian_core::layout::resolve_weight`.
fn resolve_font_weight(w: Option<&FontWeight>) -> u16 {
    match w {
        Some(FontWeight::Number(n)) => *n as u16,
        Some(FontWeight::Keyword(s)) => {
            if let Ok(n) = s.parse::<u16>() {
                return n;
            }
            match s.as_str() {
                "bold" => 700,
                "semibold" | "semi-bold" | "demibold" => 600,
                "medium" => 500,
                "normal" | "regular" => 400,
                "light" => 300,
                "extralight" | "extra-light" | "ultralight" | "ultra-light" => 200,
                "thin" | "hairline" => 100,
                "black" | "heavy" => 900,
                "extrabold" | "extra-bold" | "ultrabold" | "ultra-bold" => 800,
                _ => 0,
            }
        }
        None => 0,
    }
}

fn text_input_to_payload(n: &TextInputNode) -> NodePayload {
    let mut p = base_payload(&n.base, "text");
    p.text = n.value.clone().or_else(|| n.placeholder.clone());
    p.fill = first_solid_color(n.fill.as_deref());
    p.fill_type = first_fill_type(n.fill.as_deref());
    p.stroke = stroke_to_payload(n.stroke.as_ref());
    p
}

fn image_to_payload(n: &ImageNode) -> NodePayload {
    let mut p = base_payload(&n.base, "rect");
    if let Some(CornerRadius::Uniform(r)) = &n.corner_radius {
        p.corner_radius = *r as f32;
    } else if let Some(CornerRadius::PerCorner(corners)) = &n.corner_radius {
        p.corner_radius = corners[0] as f32;
    }
    p.fill = Some([0.85, 0.86, 0.88, 1.0]);
    p.name = if n.base.name.as_deref().unwrap_or("").is_empty() {
        format!("Image ({})", short_src(&n.src))
    } else {
        p.name
    };
    p
}

fn icon_font_to_payload(n: &IconFontNode) -> NodePayload {
    // Route through a dedicated `icon_font` kind tag so the canvas
    // renderer can look up the lucide glyph by name. TS parity:
    // `node-renderer.ts::drawIconFont` resolves `iconFontName` via
    // `lookupIconByName` (icon-dictionary.ts) and paints with
    // scale-to-fit + stroke style.
    let mut p = base_payload(&n.base, "icon_font");
    p.text = Some(n.icon_font_name.clone());
    p.fill = first_solid_color(n.fill.as_deref());
    p.fill_type = first_fill_type(n.fill.as_deref());
    p
}

fn empty_group(base: &PenNodeBase, kind: &str) -> NodePayload {
    let mut p = base_payload(base, kind);
    p.children = Vec::new();
    p
}

fn base_payload(base: &PenNodeBase, kind: &str) -> NodePayload {
    NodePayload {
        id: base.id.clone(),
        schema_id: base.id.clone(),
        kind: kind.to_string(),
        name: base.name.clone().unwrap_or_else(|| base.id.clone()),
        x: base.x.unwrap_or(0.0) as f32,
        y: base.y.unwrap_or(0.0) as f32,
        w: 0.0,
        h: 0.0,
        fill: None,
        stroke: None,
        text: None,
        rotation: (base.rotation.unwrap_or(0.0) as f32).to_radians(),
        corner_radius: 0.0,
        hidden: !base.visible.unwrap_or(true),
        locked: base.locked.unwrap_or(false),
        collapsed: false,
        fill_type: "solid".into(),
        points: Vec::new(),
        font_size: 0.0,
        font_weight: 0,
        text_wrap: false,
        effects: Vec::new(),
        children: Vec::new(),
    }
}

fn apply_container_style(
    p: &mut NodePayload,
    fill: Option<&[PenFill]>,
    stroke: Option<&PenStroke>,
    corner_radius: Option<&CornerRadius>,
) {
    p.fill = first_solid_color(fill);
    p.fill_type = first_fill_type(fill);
    p.stroke = stroke_to_payload(stroke);
    p.corner_radius = match corner_radius {
        Some(CornerRadius::Uniform(r)) => *r as f32,
        Some(CornerRadius::PerCorner(corners)) => corners[0] as f32,
        None => 0.0,
    };
}

fn first_solid_color(fills: Option<&[PenFill]>) -> Option<[f32; 4]> {
    let fills = fills?;
    for fill in fills {
        match fill {
            PenFill::Solid(body) => {
                if let Some(rgba) = parse_hex(&body.color) {
                    return Some(apply_alpha(rgba, body.opacity));
                }
            }
            PenFill::LinearGradient(body) => {
                if let Some(stop) = body.stops.first() {
                    if let Some(rgba) = parse_hex(&stop.color) {
                        return Some(apply_alpha(rgba, body.opacity));
                    }
                }
            }
            PenFill::RadialGradient(body) => {
                if let Some(stop) = body.stops.first() {
                    if let Some(rgba) = parse_hex(&stop.color) {
                        return Some(apply_alpha(rgba, body.opacity));
                    }
                }
            }
            PenFill::Image(_) => {
                return Some([0.85, 0.86, 0.88, 1.0]);
            }
        }
    }
    None
}

fn first_fill_type(fills: Option<&[PenFill]>) -> String {
    let Some(fills) = fills else { return "solid".into() };
    match fills.first() {
        Some(PenFill::LinearGradient(_)) => "linear".into(),
        Some(PenFill::RadialGradient(_)) => "radial".into(),
        Some(PenFill::Image(_)) => "image".into(),
        _ => "solid".into(),
    }
}

fn stroke_to_payload(s: Option<&PenStroke>) -> Option<StrokePayload> {
    let s = s?;
    let width = match &s.thickness {
        StrokeThickness::Uniform(n) => *n,
        StrokeThickness::PerSide(sides) => sides.iter().fold(0.0_f32, |a, b| a.max(*b)),
        StrokeThickness::Sided(sided) => [sided.top, sided.right, sided.bottom, sided.left]
            .into_iter()
            .flatten()
            .fold(0.0_f32, |a, b| a.max(b)),
    };
    let color = first_solid_color(s.fill.as_deref()).unwrap_or([0.0, 0.0, 0.0, 1.0]);
    Some(StrokePayload { color, width })
}

fn apply_alpha(rgba: [f32; 4], opacity: Option<f32>) -> [f32; 4] {
    let a = opacity.unwrap_or(1.0).clamp(0.0, 1.0);
    [rgba[0], rgba[1], rgba[2], rgba[3] * a]
}

fn parse_hex(s: &str) -> Option<[f32; 4]> {
    let s = s.trim().trim_start_matches('#');
    let (r, g, b, a) = match s.len() {
        3 => (
            u8::from_str_radix(&s[0..1].repeat(2), 16).ok()?,
            u8::from_str_radix(&s[1..2].repeat(2), 16).ok()?,
            u8::from_str_radix(&s[2..3].repeat(2), 16).ok()?,
            255u8,
        ),
        6 => (
            u8::from_str_radix(&s[0..2], 16).ok()?,
            u8::from_str_radix(&s[2..4], 16).ok()?,
            u8::from_str_radix(&s[4..6], 16).ok()?,
            255u8,
        ),
        8 => (
            u8::from_str_radix(&s[0..2], 16).ok()?,
            u8::from_str_radix(&s[2..4], 16).ok()?,
            u8::from_str_radix(&s[4..6], 16).ok()?,
            u8::from_str_radix(&s[6..8], 16).ok()?,
        ),
        _ => return None,
    };
    Some([
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        a as f32 / 255.0,
    ])
}

fn short_src(src: &str) -> String {
    let s = src.rsplit('/').next().unwrap_or(src);
    if s.len() > 24 {
        format!("{}…", &s[..24])
    } else {
        s.to_string()
    }
}

use crate::path_bounds::path_bounds_from_anchors;

#[cfg(test)]
#[path = "adapter_tests.rs"]
mod tests;
