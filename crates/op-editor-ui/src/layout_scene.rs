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

/// Drop-shadow effect — offset + blur + colour, doc-px units.
/// Painted behind the node's fill. Mirrors the TS `PenEffect`
/// shadow variant (`offsetX` / `offsetY` / `blur` / `color`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropShadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub color: Color,
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
    /// Rotation in radians, clockwise about the node's bounds centre.
    pub rotation: f32,
    /// Corner radius in doc-px — honoured by Rect / Frame paint.
    pub corner_radius: f32,
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
    /// name for `icon_font`). `None` for non-text kinds.
    pub text: Option<String>,
    /// Text size in doc-px. `0.0` = the painter's default (13 px).
    pub font_size: f32,
    /// CSS-style font weight (100-900). `0` = default (400).
    pub font_weight: u16,
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
    /// Child render nodes, in paint order.
    pub children: Vec<SceneNode>,
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

    /// Resolved bounds for selection / rotation-pivot math: the node's
    /// own `bounds` when it is bounded, otherwise the union of its
    /// children's aggregate bounds. Mirrors `SceneNode::aggregate_bounds` so the overlay painter reads the same rect a
    /// `Document`-bound painter did. Pure geometry over `bounds` +
    /// `children` — no extra scene state needed.
    pub fn aggregate_bounds(&self) -> Rect {
        if self.bounds.size.x > 0.0 || self.bounds.size.y > 0.0 {
            return self.bounds;
        }
        let mut iter = self
            .children
            .iter()
            .map(SceneNode::aggregate_bounds)
            .filter(|r| r.size.x > 0.0 || r.size.y > 0.0);
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
            rotation: 0.0,
            corner_radius: 0.0,
            fill: None,
            fill_type: SceneFillType::Solid,
            gradient: None,
            stroke: None,
            text: None,
            font_size: 0.0,
            font_weight: 0,
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
            image_fit: SceneImageFit::Fill,
            image_adjustments: crate::ImageAdjustments::default(),
            effects: Vec::new(),
            hidden: false,
            locked: false,
            children: Vec::new(),
        }
    }
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
mod tests {
    use super::*;

    #[test]
    fn empty_scene_has_no_active_page() {
        let scene = LayoutScene::default();
        assert!(scene.pages.is_empty());
        assert!(scene.active_page().is_none());
    }

    #[test]
    fn active_page_indexes_into_pages() {
        let scene = LayoutScene {
            pages: vec![
                ScenePage {
                    id: "a".into(),
                    name: "A".into(),
                    children: Vec::new(),
                },
                ScenePage {
                    id: "b".into(),
                    name: "B".into(),
                    children: Vec::new(),
                },
            ],
            active_page_index: 1,
        };
        assert_eq!(scene.active_page().map(|p| p.id.as_str()), Some("b"));
    }

    #[test]
    fn find_locates_a_nested_node() {
        let mut leaf = SceneNode::leaf("deep", NodeKind::Rect);
        leaf.bounds = Rect::xywh(0.0, 0.0, 10.0, 10.0);
        let mut group = SceneNode::leaf("g", NodeKind::Group);
        group.children = vec![leaf];
        let page = ScenePage {
            id: "p".into(),
            name: "P".into(),
            children: vec![group],
        };
        assert_eq!(page.find("deep").map(|n| n.id.as_str()), Some("deep"));
        assert!(page.find("missing").is_none());
    }

    #[test]
    fn aggregate_bounds_unions_children_for_unbounded_container() {
        let mut a = SceneNode::leaf("a", NodeKind::Rect);
        a.bounds = Rect::xywh(10.0, 10.0, 20.0, 20.0);
        let mut b = SceneNode::leaf("b", NodeKind::Rect);
        b.bounds = Rect::xywh(50.0, 5.0, 10.0, 40.0);
        let mut group = SceneNode::leaf("g", NodeKind::Group);
        group.children = vec![a, b];
        // Unbounded group → union of children: x 10..60, y 5..45.
        assert_eq!(group.aggregate_bounds(), Rect::xywh(10.0, 5.0, 50.0, 40.0));
    }

    #[test]
    fn aggregate_bounds_keeps_own_bounds_when_bounded() {
        let mut frame = SceneNode::leaf("f", NodeKind::Frame);
        frame.bounds = Rect::xywh(0.0, 0.0, 100.0, 200.0);
        let mut child = SceneNode::leaf("c", NodeKind::Rect);
        child.bounds = Rect::xywh(0.0, 0.0, 999.0, 999.0);
        frame.children = vec![child];
        assert_eq!(frame.aggregate_bounds(), Rect::xywh(0.0, 0.0, 100.0, 200.0));
    }

    #[test]
    fn leaf_node_clears_paint_fields() {
        let n = SceneNode::leaf("n1", NodeKind::Rect);
        assert_eq!(n.bounds, Rect::ZERO);
        assert!(n.fill.is_none());
        assert!(n.stroke.is_none());
        assert!(n.children.is_empty());
        assert_eq!(n.fill_type, SceneFillType::Solid);
    }
}
