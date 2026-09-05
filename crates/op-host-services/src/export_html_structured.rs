//! Structured slide markup — one deck board becomes a tree of
//! absolutely positioned HTML elements instead of one flat image.
//!
//! # What this is, and what it is not
//!
//! `op-codegen` also turns a design into HTML, and the two must not be
//! confused. `op-codegen` emits AUTHORED-TREE code for a developer to
//! own and edit: semantic tags, flex containers, class names, the
//! layout intent restated in a form a human continues from. This module
//! emits RESOLVED-SCENE markup for a projector: every element is pinned
//! by `position:absolute` at the rect jian's taffy pass already
//! computed, and the browser is never asked to lay anything out. The
//! trade is deliberate — codegen optimises for editability, this
//! optimises for the slide looking exactly like the canvas.
//!
//! The corollary is that **no layout is ever recomputed here**.
//! Coordinates come from `SceneNode::bounds`, translated by the board
//! origin and nothing else. If a browser disagrees with the editor
//! about how wide a word is, the word still starts at the same pixel.
//!
//! # Self-containment
//!
//! The exported file must present on a laptop with no network. Nothing
//! here may emit a URL: no CDN, no `@font-face`, no remote image. Two
//! consequences are load-bearing:
//!
//! - **Fonts are named, not embedded.** Bundling the faces a deck uses
//!   would add megabytes to a file the presenter emails or carries on a
//!   stick, so the authored family is named first with a system stack
//!   behind it. A substituted face changes glyph advances, but every
//!   text box is pinned by absolute position, so the drift stays inside
//!   the box — a line wrapping a word early, never a caption sliding
//!   across the slide.
//! - **Images must carry their own bytes.** A `data:` URI rides as a
//!   background; anything else — an `http(s)` link, a bare file path —
//!   is not reachable offline and takes the raster path below.
//!
//! # The fallback invariant
//!
//! Every emitter is allowed to say "I cannot express this". When it
//! does, [`fallback::emit`] renders that node alone through the shared
//! scene painter and embeds it as a `data:` PNG at its exact rect, and
//! its subtree is not walked (the pixels already contain it). So a node
//! is never silently dropped and never guessed at, and a board with
//! nothing expressible degrades on its own to a single full-slide
//! image — the artifact this exporter replaced.

use op_editor_ui::layout_scene::{NodeKind, SceneFillType, SceneImageFit, SceneNode, ScenePage};
use op_editor_ui::{ImageAdjustments, Point2D, Rect};

use crate::export::ExportError;

mod css;
mod fallback;
mod shape;
mod text;
mod vector;
mod video;

/// Length formatting, shared with the page template so a board size in
/// a `data-` attribute is spelled the same way as one in a style rule.
pub use css::num as css_num;

#[cfg(test)]
#[path = "export_html_structured/tests.rs"]
mod tests;
#[cfg(test)]
#[path = "export_html_structured/video_tests.rs"]
mod video_tests;

/// One board rendered as markup, plus what it cost to get there.
#[derive(Debug, Clone)]
pub struct SlideMarkup {
    /// Authored board name — the slide's accessible label.
    pub name: String,
    /// Board size in doc px. The player scales the slide from this.
    pub width: f32,
    pub height: f32,
    /// The slide's inner markup (the board element and its subtree).
    pub body: String,
    /// Nodes emitted as real elements.
    pub structured_nodes: usize,
    /// Nodes that took the raster path, and why. Same length as the
    /// fallback count; the reasons make "why is my slide a picture"
    /// answerable without re-running the export under a debugger.
    pub fallback_reasons: Vec<&'static str>,
}

impl SlideMarkup {
    pub fn raster_fallbacks(&self) -> usize {
        self.fallback_reasons.len()
    }
}

/// Build the markup for one board on `page`.
///
/// Coordinates in the result are board-local: the board's own element
/// sits at `(0, 0)` and every descendant is offset from the board
/// origin, so the player can place a slide without knowing where on the
/// infinite canvas its board happened to live.
pub fn board_slide_markup(
    page: &ScenePage,
    board_id: &str,
    name: String,
) -> Result<SlideMarkup, ExportError> {
    let board = page
        .find(board_id)
        .ok_or_else(|| ExportError::NodeNotFoundOnPage {
            node_id: board_id.to_string(),
            page_id: page.id.clone(),
        })?;
    if board.hidden {
        return Err(ExportError::NodeHidden {
            node_id: board_id.to_string(),
        });
    }
    let bounds = op_editor_ui::scene_bounds::normalize_rect(board.aggregate_bounds());
    if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
        return Err(ExportError::NodePaintsNothing {
            node_id: board_id.to_string(),
        });
    }

    let mut emitter = Emitter {
        out: String::with_capacity(8192),
        parent_origin: bounds.origin,
        structured_nodes: 0,
        fallback_reasons: Vec::new(),
    };
    emitter.emit(board)?;

    Ok(SlideMarkup {
        name,
        width: bounds.size.x,
        height: bounds.size.y,
        body: emitter.out,
        structured_nodes: emitter.structured_nodes,
        fallback_reasons: emitter.fallback_reasons,
    })
}

struct Emitter {
    out: String,
    /// Doc-space origin of the box the element being emitted is
    /// positioned against.
    ///
    /// Scene bounds are ABSOLUTE, but `position:absolute` resolves
    /// against the nearest positioned ancestor — which, since every
    /// emitted element is itself absolutely positioned, is the parent
    /// element. So this tracks the current parent's origin down the
    /// walk, not the board's: subtracting the board origin at every
    /// depth would add each ancestor's offset a second time, and a
    /// nested label would drift further off the deeper it sat.
    parent_origin: Point2D,
    structured_nodes: usize,
    fallback_reasons: Vec<&'static str>,
}

impl Emitter {
    fn emit(&mut self, node: &SceneNode) -> Result<(), ExportError> {
        if node.hidden {
            return Ok(());
        }
        if let Some(reason) = unexpressible(node) {
            if fallback::emit(&mut self.out, node, self.parent_origin)? {
                self.fallback_reasons.push(reason);
            }
            return Ok(());
        }

        let rect = self.local(node.bounds);
        self.structured_nodes += 1;
        match &node.kind {
            NodeKind::Text => text::emit(&mut self.out, node, rect),
            NodeKind::Line => vector::emit_line(&mut self.out, node, self.parent_origin),
            NodeKind::Polygon => vector::emit_polygon(&mut self.out, node, self.parent_origin),
            NodeKind::Path => vector::emit_path(&mut self.out, node, self.parent_origin),
            NodeKind::Other(tag) if tag == "icon_font" => {
                vector::emit_icon(&mut self.out, node, rect)
            }
            kind => {
                let ellipse = matches!(kind, NodeKind::Ellipse);
                shape::open(&mut self.out, node, rect, ellipse);
                self.emit_children(node)?;
                shape::close(&mut self.out, node, rect, ellipse);
            }
        }
        Ok(())
    }

    /// Emit a container's children against the container's own box.
    ///
    /// Scene children are stored topmost-first (layer-panel order) and
    /// the canvas painter walks them in reverse. DOM order IS paint
    /// order — a later sibling covers an earlier one — so the same
    /// reversal is what keeps the z-order the author sees.
    fn emit_children(&mut self, node: &SceneNode) -> Result<(), ExportError> {
        let outer = self.parent_origin;
        self.parent_origin = op_editor_ui::scene_bounds::normalize_rect(node.bounds).origin;
        let result = (|emitter: &mut Self| {
            for child in node.children.iter().rev() {
                emitter.emit(child)?;
            }
            Ok(())
        })(self);
        // Restored even on the error path: the container's closing tag
        // is still written by the caller, and a leaked origin would
        // misplace every later sibling.
        self.parent_origin = outer;
        result
    }

    fn local(&self, rect: Rect) -> Rect {
        let normalized = op_editor_ui::scene_bounds::normalize_rect(rect);
        Rect {
            origin: Point2D::new(
                normalized.origin.x - self.parent_origin.x,
                normalized.origin.y - self.parent_origin.y,
            ),
            size: normalized.size,
        }
    }
}

/// Why this node cannot be expressed as markup, or `None` when it can.
///
/// The list is deliberately conservative: a paint feature that CSS can
/// only approximate is still allowed through when the approximation is
/// bounded and documented (gradient line length, blend backdrop scope),
/// but anything whose visual result would be a GUESS is refused here so
/// the raster path can be exact instead.
fn unexpressible(n: &SceneNode) -> Option<&'static str> {
    if n.is_mask || n.mask_type.is_some() {
        // A mask reshapes its front siblings' pixels. CSS masking works
        // on a different tree relationship entirely.
        return Some("mask");
    }
    if n.widget.is_some() {
        // Switch knobs, slider tracks, select chevrons — the canvas
        // draws a composite visual from the widget descriptor that has
        // no scene geometry to read back.
        return Some("composite widget");
    }
    if n.fill_layers.len() > 1 {
        // Per-layer blending inside one node's fill stack; the scene's
        // single `fill` / `gradient` summary would drop the rest.
        return Some("layered fill stack");
    }
    match n.fill_type {
        SceneFillType::Shader => return Some("sksl shader fill"),
        SceneFillType::MeshGradient => return Some("mesh gradient fill"),
        SceneFillType::LinearGradient | SceneFillType::RadialGradient => {
            if n.gradient.is_none() {
                return Some("gradient fill without a resolved body");
            }
        }
        SceneFillType::Solid | SceneFillType::Image => {}
    }
    if matches!(n.kind, NodeKind::Ellipse)
        && (n.arc_start_angle.is_some() || n.arc_sweep_angle.is_some())
    {
        // Pie / donut arcs. A `border-radius:50%` div is a full oval.
        return Some("ellipse arc");
    }
    if matches!(n.kind, NodeKind::Ellipse) && n.arc_inner_radius.is_some_and(|r| r > 0.0) {
        return Some("ellipse inner radius");
    }
    if let NodeKind::Other(tag) = &n.kind {
        if tag != "icon_font" {
            // An unknown tag is one this emitter has no model for, and
            // the painter may well special-case it (as it does for
            // `icon_font` and for widgets). Assuming "it is a box" is
            // the silent guess the fallback invariant exists to avoid.
            return Some("unknown node kind");
        }
    }
    if let Some(src) = n.image_src.as_deref() {
        if let Some(reason) = image_unexpressible(n, src) {
            return Some(reason);
        }
    }
    None
}

fn image_unexpressible(n: &SceneNode, src: &str) -> Option<&'static str> {
    if n.video.is_some() && src.trim().is_empty() {
        // A video alias may omit its poster. There is no poster URL to
        // externalize, so the video remains structured over the fill colour.
        return None;
    }
    if !src.trim_start().starts_with("data:") {
        // The whole point of the export is that it presents offline. A
        // remote or filesystem reference is not reachable there; the
        // raster path resolves it now, at export time, instead.
        return Some("image source is not embedded bytes");
    }
    if n.image_transform.is_some() {
        // Figma's normalized-UV affine crop. `background-size` cannot
        // express a rotation or shear of the sampled region.
        return Some("image crop transform");
    }
    if n.image_adjustments != ImageAdjustments::default() {
        // Exposure / contrast / temperature curves are applied by the
        // painter's colour matrix; CSS filters are not the same curves.
        return Some("image colour adjustments");
    }
    if n.image_fit == SceneImageFit::Tile && n.image_original_size.is_none() {
        // Without the source dimensions the repeat frequency would be
        // invented, and a wrong tile size is a visibly wrong texture.
        return Some("tiled image without source dimensions");
    }
    None
}
