//! `LayoutScene` — a paint-only, layout-resolved render scene.
//!
//! This is the migration target for `CanvasViewport`: a tree of
//! resolved render nodes that a painter can walk and reproduce the
//! current canvas pixel-for-pixel, WITHOUT depending on the editor's
//! `Document` (which mixes in selection / chat / history / UI state).
//!
//! Distinctions from [`op_editor_core::EditorState`]:
//!
//! - **No editor state.** `LayoutScene` carries no `selected`,
//!   `tool`, `viewport`, `chat`, `history`, `components`, `ui`. The
//!   selection overlay + grid + viewport transform are the host
//!   painter's concern, layered on top of the scene.
//! - **Layout-resolved geometry.** Every [`SceneNode::bounds`] is the
//!   absolute doc-space AABB produced by jian's taffy `LayoutEngine` —
//!   NOT the authored `(x, y, w, h)`. A future painter draws straight
//!   from `bounds` with no second layout pass.
//! - **Fills are concrete.** Variable `$ref` fills / strokes are
//!   resolved against the editor's variables + active theme at build
//!   time, so [`SceneNode::fill`] is always a final paintable colour.
//!
//! The builder lives in `op-pen-loader`
//! (`editor_state_to_layout_scene`); nothing consumes `LayoutScene`
//! yet — `CanvasViewport` is flipped onto it in a later step.
//!
//! wasm32-clean: only `crate::{Color, Point2D, Rect}` + the scene's
//! own paint enums (`NodeKind` / `Effect`). The web host builds
//! scenes too.

use crate::{Color, Point2D, Rect};

/// Node kinds the canvas painter draws. `Other` round-trips unknown
/// kinds so an unfamiliar serialized node never errors the painter.
///
/// This is a paint-time scene enum — it lives with [`LayoutScene`]
/// because the resolved render tree is its only consumer. It is
/// deliberately NOT the canonical `jian_ops_schema` node model; the
/// scene builder maps `PenNode` variants onto it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Frame,
    Group,
    Rect,
    Ellipse,
    Polygon,
    Line,
    /// Pen-tool polyline. Geometry in `SceneNode.points`.
    Path,
    Text,
    Other(String),
}

impl NodeKind {
    /// Human-facing label for LayerPanel + PropertyPanel.
    pub fn label(&self) -> &str {
        match self {
            NodeKind::Frame => "Frame",
            NodeKind::Group => "Group",
            NodeKind::Rect => "Rect",
            NodeKind::Ellipse => "Ellipse",
            NodeKind::Polygon => "Polygon",
            NodeKind::Line => "Line",
            NodeKind::Path => "Path",
            NodeKind::Text => "Text",
            NodeKind::Other(s) => s.as_str(),
        }
    }
}

/// Shadow effect — offset + blur + colour, doc-px units. Mirrors the
/// TS `PenEffect` shadow variant (`offsetX` / `offsetY` / `blur` /
/// `color` / `inner`). An outer shadow paints behind the node's fill;
/// an `inner` shadow paints inset, clipped to the node silhouette.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropShadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub color: Color,
    /// `true` = inset (inner) shadow; `false` = outer drop shadow.
    pub inner: bool,
}

/// One resolved gradient stop — offset 0.0..=1.0 plus the stop's
/// concrete colour (already hex-parsed and `$ref`-resolved by the
/// scene builder).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneGradientStop {
    pub offset: f32,
    pub color: Color,
}

/// A paintable gradient — either linear (canonical `.op` angle in
/// degrees, where 0° = bottom→top, 90° = left→right, 180° = top→bottom
/// — i.e. CSS `to-top` convention; the renderer subtracts 90° before
/// projecting endpoints) or radial (centre + radius as 0.0..=1.0
/// fractions of the node's bounds, with radius scaled against
/// `max(w, h)` for parity with the TS `pen-renderer`). Carries the
/// per-fill `opacity` so the painter can fold it into every stop's
/// alpha without mutating the stops themselves.
#[derive(Debug, Clone, PartialEq)]
pub enum SceneGradient {
    Linear {
        /// Canonical `.op` angle (degrees, 0° = bottom→top).
        angle_deg: f32,
        opacity: f32,
        stops: Vec<SceneGradientStop>,
    },
    Radial {
        /// Centre x as a 0.0..=1.0 fraction of bounds width (0.5 = centre).
        cx: f32,
        /// Centre y as a 0.0..=1.0 fraction of bounds height.
        cy: f32,
        /// Outer radius as a 0.0..=1.0 fraction of `max(w, h)` — matches
        /// `pen-renderer/src/node-renderer.ts::node_renderer` so `.op`
        /// files import / export at the same radial size as the TS app.
        radius: f32,
        opacity: f32,
        stops: Vec<SceneGradientStop>,
    },
}

/// A visual effect painted with a [`SceneNode`]. v1 ships drop
/// shadow (what the property panel's effects section needs).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Effect {
    DropShadow(DropShadow),
}

/// A paint-only, layout-resolved render scene.
///
/// Built from an `op_editor_core::EditorState` by running jian's flex
/// layout pass and resolving variable `$ref` colours. Carries the
/// multi-page structure the canvas lays out plus, per page, the
/// resolved render-node tree.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LayoutScene {
    /// Pages, in document order. The canvas paints one page at a time
    /// (`active_page_index`), but every page is resolved so page
    /// switches don't need a rebuild.
    pub pages: Vec<ScenePage>,
    /// Index into `pages` of the page the editor currently shows.
    /// Clamped into range by the builder.
    pub active_page_index: usize,
}

impl LayoutScene {
    /// The page the editor currently shows, or `None` when the scene
    /// has no pages.
    pub fn active_page(&self) -> Option<&ScenePage> {
        self.pages.get(self.active_page_index)
    }

    /// Union AABB of every top-level node on the active page, in
    /// document space — the extent a "zoom to fit" frames. `None`
    /// when the page is empty (nothing to fit).
    pub fn content_bounds(&self) -> Option<Rect> {
        let page = self.active_page()?;
        let mut acc: Option<Rect> = None;
        for child in &page.children {
            let b = child.aggregate_bounds();
            if b.size.x <= 0.0 && b.size.y <= 0.0 {
                continue;
            }
            acc = Some(match acc {
                None => b,
                Some(a) => {
                    let min_x = a.origin.x.min(b.origin.x);
                    let min_y = a.origin.y.min(b.origin.y);
                    let max_x = (a.origin.x + a.size.x).max(b.origin.x + b.size.x);
                    let max_y = (a.origin.y + a.size.y).max(b.origin.y + b.size.y);
                    Rect {
                        origin: Point2D::new(min_x, min_y),
                        size: Point2D::new(max_x - min_x, max_y - min_y),
                    }
                }
            });
        }
        acc
    }

    /// Translate selected render subtrees in-place without rebuilding the
    /// full layout scene. Used by live node drags: the canonical
    /// `EditorState` is still updated first, but simple absolute moves can
    /// keep paint in sync by patching the current scene until release-time
    /// layout reconciliation.
    pub fn translate_nodes(&mut self, ids: &[String], dx: f32, dy: f32) -> bool {
        if ids.is_empty() || (dx == 0.0 && dy == 0.0) {
            return false;
        }
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return false;
        };
        page.translate_nodes(ids, dx, dy)
    }
}

impl ScenePage {
    /// Depth-first search for the node with `id` anywhere in this
    /// page's render tree. Mirrors `ScenePage::find` so the
    /// selection-overlay + pen-rubber-band painters can map an editor
    /// selection id onto the resolved scene node.
    pub fn find(&self, id: &str) -> Option<&SceneNode> {
        for child in &self.children {
            if let Some(found) = child.find(id) {
                return Some(found);
            }
        }
        None
    }

    fn translate_nodes(&mut self, ids: &[String], dx: f32, dy: f32) -> bool {
        translate_matching_nodes(&mut self.children, ids, dx, dy)
    }
}

fn translate_matching_nodes(nodes: &mut [SceneNode], ids: &[String], dx: f32, dy: f32) -> bool {
    let mut changed = false;
    for node in nodes {
        if ids.iter().any(|id| id == &node.id) {
            node.translate_subtree(dx, dy);
            changed = true;
        } else if translate_matching_nodes(&mut node.children, ids, dx, dy) {
            changed = true;
        }
    }
    changed
}

/// One resolved page — an artboard / page id + name + the top-level
/// resolved render nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenePage {
    /// Page id (the `.op` page id, or `"page-1"` for the single-page
    /// fallback). Identity only — the painter does not key off it.
    pub id: String,
    /// Page name — surfaced by the layer panel, not painted on canvas.
    pub name: String,
    /// Top-level resolved render nodes for this page.
    pub children: Vec<SceneNode>,
}

/// A resolved render node — everything the canvas painter reads to
/// draw one node, with geometry already baked by the layout pass and
/// fills already resolved to concrete colours.
///
/// Mirrors the fields `CanvasViewport`'s painter reads off
/// `SceneNode` today (`kind`, `bounds`, `fill`, `stroke`,
/// `rotation`, `corner_radius`, `text`, `font_size`, `font_weight`,
/// `text_wrap`, `points`, `effects`, `children`, `hidden`) so a
/// painter over `LayoutScene` can reproduce the current canvas
/// pixel-for-pixel.
/// How a path anchor's two control handles relate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenePointType {
    /// Handles independent; no smoothing across the anchor.
    Corner,
    /// Handles collinear + equal length.
    Mirrored,
    /// Handles move freely + independently.
    Independent,
}

/// A path bezier anchor resolved into absolute doc coords — the
/// anchor point plus its (optional) incoming / outgoing control
/// handles. Handle positions are absolute, not anchor-relative.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneAnchor {
    pub pos: Point2D,
    pub handle_in: Option<Point2D>,
    pub handle_out: Option<Point2D>,
    pub point_type: ScenePointType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneTextAlign {
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneTextVerticalAlign {
    Top,
    Middle,
    Bottom,
}

/// One styled run of a Text node's flattened content — a byte range
/// into [`SceneNode::text`] plus the per-segment style overrides the
/// canonical schema's `TextContent::Styled` segments carry. Inherited
/// (unset) attributes use the sentinel conventions of the node-level
/// fields (`0.0` font size / `0` weight / `None` fill = inherit).
///
/// Runs exist ALONGSIDE the flat string: line wrapping, the text
/// editor, and hit-testing all stay on the flat text; the painter maps
/// each wrapped line back onto run byte ranges so painted slices carry
/// their segment's style. A wrapped line that splits mid-segment keeps
/// per-slice styling exact; only the WRAP POSITION itself is computed
/// with node-level metrics (documented v1 approximation).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneTextRun {
    /// Inclusive start byte offset into the node's flat `text`.
    pub start: usize,
    /// Exclusive end byte offset into the node's flat `text`.
    pub end: usize,
    /// Per-run font size in doc-px. `0.0` = inherit the node's.
    pub font_size: f32,
    /// Per-run CSS-style weight (100-900). `0` = inherit the node's.
    pub font_weight: u16,
    /// Per-run fill colour. `None` = inherit the node's resolved fill.
    pub fill: Option<Color>,
    /// Render this run with an italic (slanted) face.
    pub italic: bool,
    /// Stroke an underline beneath this run's painted slices.
    pub underline: bool,
    /// Stroke a strikethrough across this run's painted slices.
    pub strikethrough: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneNode {
    /// Stable node id (the `.op` schema id). Identity for hit-test /
    /// selection mapping done by the host on top of the scene.
    pub id: String,
    /// Node kind — drives per-kind paint (Frame fill, Ellipse oval,
    /// Polygon triangle, Line diagonal, Path polyline, Text run, …).
    /// `Other("icon_font")` carries a lucide glyph name in `text`.
    pub kind: NodeKind,
    /// Layout-resolved absolute doc-space rect. Already offset by the
    /// page-root's authored `(x, y)` — paint applies only the
    /// viewport transform.
    pub bounds: Rect,
    /// Precomputed aggregate subtree bounds for unbounded containers.
    /// `Rect::ZERO` means "not precomputed" so hand-built tests and
    /// helpers that mutate `children` after [`SceneNode::leaf`] keep
    /// the existing recursive behaviour. The loader fills this once,
    /// bottom up, so hot paint / hit-test paths do not repeatedly
    /// union large subtrees every frame.
    pub aggregate_bounds_cache: Rect,
    /// Cumulative node opacity (0.0..=1.0), already multiplied down
    /// the subtree. Fill / stroke / gradient / shadow colours have it
    /// baked into their alpha at scene-build; the painter applies it
    /// directly only for raster images (which carry no colour to
    /// bake into). 1.0 = fully opaque.
    pub opacity: f32,
    /// Rotation in radians, clockwise about the node's bounds centre.
    pub rotation: f32,
    /// Mirror horizontally around the node's aggregate-bounds centre.
    pub flip_x: bool,
    /// Mirror vertically around the node's aggregate-bounds centre.
    pub flip_y: bool,
    /// Corner radius in doc-px — honoured by Rect / Frame paint.
    pub corner_radius: f32,
    /// Whether this container clips its children to its bounds
    /// (rounded by `corner_radius`, clamped to half the height — TS
    /// flattener rule). The scene builder also sets this for root
    /// frames, which clip like artboards even without an authored
    /// `clipContent: true` (TS `document-flattener.ts` parity).
    pub clip_content: bool,
    /// Resolved fill colour. `$ref` variable fills are already
    /// resolved against the editor's variables + active theme; a
    /// gradient keeps its first stop here (parity with the current
    /// canvas, which paints the first solid colour). `None` = no fill.
    pub fill: Option<Color>,
    /// Fill paint mode — `Solid` / `LinearGradient` / `RadialGradient`
    /// / `Image`. The current canvas paints all of them as the solid
    /// `fill` colour; carried so a richer painter can branch later.
    pub fill_type: SceneFillType,
    /// Resolved gradient body — populated when `fill_type` is
    /// `LinearGradient` or `RadialGradient`. `fill` still holds the
    /// first stop's colour so non-gradient code paths (Polygon /
    /// Path / Ellipse / Line painters that have no gradient overload
    /// yet) keep a sensible fallback.
    pub gradient: Option<SceneGradient>,
    /// Resolved stroke (colour + width). `$ref` stroke colours are
    /// resolved at build time. `None` = no stroke.
    pub stroke: Option<SceneStroke>,
    /// Text content — `Some` for Text nodes (and the lucide glyph
    /// name for `icon_font`). `None` for non-text kinds. Styled text
    /// is flattened into this string; per-segment styling rides in
    /// `text_runs`.
    pub text: Option<String>,
    /// Per-segment style runs over `text` (byte ranges). Empty for
    /// plain (single-style) text — the painter then uses the
    /// node-level style for the whole content.
    pub text_runs: Vec<SceneTextRun>,
    /// CSS font-family stack for text nodes. Empty = renderer default.
    pub font_family: String,
    /// Text size in doc-px. `0.0` = the painter's default (13 px).
    pub font_size: f32,
    /// CSS-style font weight (100-900). `0` = default (400).
    pub font_weight: u16,
    /// Node-level italic (`fontStyle: italic`). Per-run overrides ride
    /// in `text_runs`.
    pub italic: bool,
    /// Node-level underline decoration.
    pub underline: bool,
    /// Node-level strikethrough decoration.
    pub strikethrough: bool,
    /// Line-height multiplier (`1.2` = 120%). `0.0` = renderer default.
    pub line_height: f32,
    /// Extra tracking in doc-px between glyphs.
    pub letter_spacing: f32,
    /// Horizontal text alignment inside `bounds`.
    pub text_align: SceneTextAlign,
    /// Vertical text alignment inside `bounds`.
    pub text_vertical_align: SceneTextVerticalAlign,
    /// Whether the painter wraps the text to `bounds.size.x`.
    pub text_wrap: bool,
    /// Polyline / path geometry in absolute doc-space coords —
    /// populated for `Path` (and any kind the painter walks as
    /// points). Empty otherwise.
    pub points: Vec<Point2D>,
    /// Bezier anchors for `Path` nodes — parallel to `points` but
    /// carrying the editable control handles + point type. Empty for
    /// non-Path kinds.
    pub path_anchors: Vec<SceneAnchor>,
    /// Whether a `Path` node is closed (last anchor links to first).
    pub path_closed: bool,
    /// Preserved SVG path data for imported Path nodes. Coordinates
    /// are local doc-px relative to `bounds.origin`.
    pub svg_path: Option<String>,
    /// Ellipse arc start angle in degrees. `None` = full ellipse.
    pub arc_start_angle: Option<f32>,
    /// Ellipse arc sweep angle in degrees. `None` = full ellipse.
    pub arc_sweep_angle: Option<f32>,
    /// Ellipse donut-hole radius (0.0..=1.0 fraction). `None` / 0 =
    /// solid.
    pub arc_inner_radius: Option<f32>,
    /// Polygon side count. `3` is the canonical triangle default.
    pub polygon_sides: u32,
    /// Image source for nodes that paint a bitmap (`PenNode::Image`).
    /// Carries the canonical schema's `src` field verbatim — usually
    /// a `data:image/...;base64,...` URL produced by the host's file
    /// picker, or a plain file path / remote URL on documents that
    /// reference external media. `None` for non-image nodes.
    pub image_src: Option<String>,
    /// Stable content hash for `image_src`. The canvas painter uses it
    /// as a per-frame cache key without hashing large data URLs again.
    pub image_src_id: u64,
    /// How `image_src` is placed into `bounds`.
    pub image_fit: SceneImageFit,
    /// Per-image colour adjustments from the image-fill editor.
    pub image_adjustments: crate::ImageAdjustments,
    /// Drop-shadow / effects painted behind the node's fill.
    pub effects: Vec<Effect>,
    /// Whether the node (and its subtree) is hidden — the painter
    /// skips hidden nodes entirely.
    pub hidden: bool,
    /// Whether the node is locked. Paint ignores this; the host's
    /// canvas hit-test reads it so a locked node's body opts out of
    /// selection while its children stay hittable (parity with the
    /// `Document`-bound `hit_test_walk`).
    pub locked: bool,
    /// Composite-widget descriptor for the interactive node family
    /// (switch / checkbox / slider / progress / select / radio_group /
    /// text_input / text_area / number_input / tabs). When set, the
    /// design-surface painter draws a recognizable static visual
    /// (track + knob, box + check, chevron, bar, …) instead of the
    /// bare degraded rect / text. `None` for ordinary shapes.
    pub widget: Option<SceneWidget>,
    /// Child render nodes, in paint order.
    pub children: Vec<SceneNode>,
}

/// Static composite-widget props carried on a [`SceneNode`]. Mirrors
/// `op_pen_loader::WidgetPayload`; the canvas painter reads it to draw
/// the non-interactive design-time visual that matches the widget's
/// preview-mode rendering (jian-core's `emit_widget_visual`). Numeric
/// sentinels fall back to the runtime defaults (`min=0`, `max=100`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SceneWidget {
    /// Widget family tag (`switch` / `checkbox` / `slider` / `progress`
    /// / `select` / `radio_group` / `text_input` / `text_area` /
    /// `number_input` / `tabs`).
    pub kind: String,
    /// On/off state for switch / checkbox.
    pub checked: Option<bool>,
    /// Numeric value for slider / progress / number_input.
    pub value_num: Option<f32>,
    /// String value for select / radio_group / tabs / text inputs.
    pub value_str: Option<String>,
    /// Placeholder text for inputs / select.
    pub placeholder: Option<String>,
    /// Adjacent label (checkbox).
    pub label: Option<String>,
    /// Range minimum (slider / number_input). `None` = 0.
    pub min: Option<f32>,
    /// Range maximum (slider / progress / number_input). `None` = 100.
    pub max: Option<f32>,
    /// Range step. `None` = 1.
    pub step: Option<f32>,
    /// `(value, label)` rows for select / radio_group / tabs.
    pub options: Vec<SceneWidgetOption>,
}

/// One `value` + `label` option for select / radio_group / tabs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SceneWidgetOption {
    pub value: String,
    pub label: String,
}

impl SceneNode {
    /// Depth-first search for the node with `id` in this subtree
    /// (self included). Mirrors `SceneNode::find`.
    pub fn find(&self, id: &str) -> Option<&SceneNode> {
        if self.id == id {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find(id) {
                return Some(found);
            }
        }
        None
    }

    fn translate_subtree(&mut self, dx: f32, dy: f32) {
        self.bounds.origin.x += dx;
        self.bounds.origin.y += dy;
        if rect_has_extent(self.aggregate_bounds_cache) {
            self.aggregate_bounds_cache.origin.x += dx;
            self.aggregate_bounds_cache.origin.y += dy;
        }
        for point in &mut self.points {
            point.x += dx;
            point.y += dy;
        }
        for anchor in &mut self.path_anchors {
            anchor.pos.x += dx;
            anchor.pos.y += dy;
            if let Some(handle) = anchor.handle_in.as_mut() {
                handle.x += dx;
                handle.y += dy;
            }
            if let Some(handle) = anchor.handle_out.as_mut() {
                handle.x += dx;
                handle.y += dy;
            }
        }
        for child in &mut self.children {
            child.translate_subtree(dx, dy);
        }
    }

    /// Resolved bounds for selection / rotation-pivot math: the node's
    /// own `bounds` when it is bounded, otherwise a loader-precomputed
    /// subtree rect when available, otherwise the union of children's
    /// aggregate bounds for hand-built scenes.
    pub fn aggregate_bounds(&self) -> Rect {
        if rect_has_extent(self.bounds) {
            return self.bounds;
        }
        if rect_has_extent(self.aggregate_bounds_cache) {
            return self.aggregate_bounds_cache;
        }
        Self::compute_aggregate_bounds(self.bounds, &self.children)
    }

    /// Compute the aggregate subtree rect from resolved own bounds and
    /// children. Used by the loader during scene construction and by
    /// [`aggregate_bounds`](Self::aggregate_bounds) as the uncached
    /// fallback for hand-built test scenes.
    pub fn compute_aggregate_bounds(bounds: Rect, children: &[SceneNode]) -> Rect {
        if rect_has_extent(bounds) {
            return bounds;
        }
        let mut iter = children
            .iter()
            .map(SceneNode::aggregate_bounds)
            .filter(|r| rect_has_extent(*r));
        let Some(first) = iter.next() else {
            return Rect::ZERO;
        };
        let (mut min_x, mut min_y) = (first.origin.x, first.origin.y);
        let (mut max_x, mut max_y) = (first.origin.x + first.size.x, first.origin.y + first.size.y);
        for r in iter {
            min_x = min_x.min(r.origin.x);
            min_y = min_y.min(r.origin.y);
            max_x = max_x.max(r.origin.x + r.size.x);
            max_y = max_y.max(r.origin.y + r.size.y);
        }
        Rect::xywh(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    /// Construct a leaf render node with all paint fields cleared.
    /// Builders set `bounds` / `fill` / `text` / … after.
    pub fn leaf(id: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            id: id.into(),
            kind,
            bounds: Rect::ZERO,
            aggregate_bounds_cache: Rect::ZERO,
            opacity: 1.0,
            rotation: 0.0,
            flip_x: false,
            flip_y: false,
            corner_radius: 0.0,
            clip_content: false,
            fill: None,
            fill_type: SceneFillType::Solid,
            gradient: None,
            stroke: None,
            text: None,
            text_runs: Vec::new(),
            font_family: String::new(),
            font_size: 0.0,
            font_weight: 0,
            italic: false,
            underline: false,
            strikethrough: false,
            line_height: 0.0,
            letter_spacing: 0.0,
            text_align: SceneTextAlign::Left,
            text_vertical_align: SceneTextVerticalAlign::Top,
            text_wrap: false,
            points: Vec::new(),
            path_anchors: Vec::new(),
            path_closed: false,
            svg_path: None,
            arc_start_angle: None,
            arc_sweep_angle: None,
            arc_inner_radius: None,
            polygon_sides: 3,
            image_src: None,
            image_src_id: 0,
            image_fit: SceneImageFit::Fill,
            image_adjustments: crate::ImageAdjustments::default(),
            effects: Vec::new(),
            hidden: false,
            locked: false,
            widget: None,
            children: Vec::new(),
        }
    }
}

fn rect_has_extent(rect: Rect) -> bool {
    rect.size.x > 0.0 || rect.size.y > 0.0
}

pub fn stable_image_source_id(src: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut h);
    h.finish()
}

/// Vertices for a regular polygon fitted inside `rect`.
pub fn regular_polygon_points(rect: Rect, sides: u32) -> Vec<Point2D> {
    let n = sides.clamp(3, 100) as usize;
    let cx = rect.origin.x + rect.size.x / 2.0;
    let cy = rect.origin.y + rect.size.y / 2.0;
    let rx = rect.size.x / 2.0;
    let ry = rect.size.y / 2.0;
    let start = -std::f32::consts::FRAC_PI_2;
    (0..n)
        .map(|i| {
            let angle = start + i as f32 * std::f32::consts::TAU / n as f32;
            Point2D::new(cx + rx * angle.cos(), cy + ry * angle.sin())
        })
        .collect()
}

/// Resolved stroke descriptor — colour already `$ref`-resolved.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneStroke {
    pub color: Color,
    pub width: f32,
}

/// Fill paint mode for a [`SceneNode`]. Mirrors
/// `op_editor_core::FillType`; kept as its own enum so the scene
/// type does not re-export an editor-model enum that may diverge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SceneFillType {
    #[default]
    Solid,
    LinearGradient,
    RadialGradient,
    Image,
}

/// Image placement mode carried by a [`SceneNode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SceneImageFit {
    #[default]
    Fill,
    Fit,
    Crop,
    Tile,
    Stretch,
}

impl SceneImageFit {
    pub fn to_draw_mode(self) -> crate::ImageDrawMode {
        match self {
            Self::Fill => crate::ImageDrawMode::Fill,
            Self::Fit => crate::ImageDrawMode::Fit,
            Self::Crop => crate::ImageDrawMode::Crop,
            Self::Tile => crate::ImageDrawMode::Tile,
            Self::Stretch => crate::ImageDrawMode::Stretch,
        }
    }
}

#[cfg(test)]
#[path = "layout_scene_unit_tests.rs"]
mod tests;
