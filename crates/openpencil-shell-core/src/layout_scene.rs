//! `LayoutScene` — a paint-only, layout-resolved render scene.
//!
//! This is the migration target for `CanvasViewport`: a tree of
//! resolved render nodes that a painter can walk and reproduce the
//! current canvas pixel-for-pixel, WITHOUT depending on the editor's
//! `Document` (which mixes in selection / chat / history / UI state).
//!
//! Distinctions from [`crate::document::Document`]:
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
//! wasm32-clean: only `crate::{Color, Point2D, Rect}` + `document`
//! enum re-exports. The web host will build scenes too.

use crate::document::{Effect, NodeKind};
use crate::{Color, Point2D, Rect};

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
/// `document::Node` today (`kind`, `bounds`, `fill`, `stroke`,
/// `rotation`, `corner_radius`, `text`, `font_size`, `font_weight`,
/// `text_wrap`, `points`, `effects`, `children`, `hidden`) so a
/// painter over `LayoutScene` can reproduce the current canvas
/// pixel-for-pixel.
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
    /// Drop-shadow / effects painted behind the node's fill.
    pub effects: Vec<Effect>,
    /// Whether the node (and its subtree) is hidden — the painter
    /// skips hidden nodes entirely.
    pub hidden: bool,
    /// Child render nodes, in paint order.
    pub children: Vec<SceneNode>,
}

impl SceneNode {
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
            stroke: None,
            text: None,
            font_size: 0.0,
            font_weight: 0,
            text_wrap: false,
            points: Vec::new(),
            effects: Vec::new(),
            hidden: false,
            children: Vec::new(),
        }
    }
}

/// Resolved stroke descriptor — colour already `$ref`-resolved.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneStroke {
    pub color: Color,
    pub width: f32,
}

/// Fill paint mode for a [`SceneNode`]. Mirrors
/// [`crate::document::FillType`]; kept as its own enum so the scene
/// type does not re-export an editor-model enum that may diverge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SceneFillType {
    #[default]
    Solid,
    LinearGradient,
    RadialGradient,
    Image,
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
                ScenePage { id: "a".into(), name: "A".into(), children: Vec::new() },
                ScenePage { id: "b".into(), name: "B".into(), children: Vec::new() },
            ],
            active_page_index: 1,
        };
        assert_eq!(scene.active_page().map(|p| p.id.as_str()), Some("b"));
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
