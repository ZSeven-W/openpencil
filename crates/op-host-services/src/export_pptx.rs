//! Structured PPTX export — the render + IO behind File ▸ "Export
//! PowerPoint".
//!
//! # What this is
//!
//! One `.pptx` in which every board is a slide and every text node is a
//! real PowerPoint text box. Someone who opens the file in PowerPoint,
//! Keynote or WPS can fix a typo, restyle a heading or reflow a bullet —
//! the deck arrives as a document, not as a folder of screenshots.
//!
//! It shares its底料 with [`crate::export_html_structured`]: the
//! resolved [`LayoutScene`](op_editor_ui::layout_scene::LayoutScene),
//! whose rects jian's taffy pass has already computed. **No layout is
//! ever recomputed here.** Coordinates come from `SceneNode::bounds`,
//! translated by the board origin and converted to EMU, and nothing
//! else. If PowerPoint disagrees with the editor about how wide a word
//! is, the word still starts at the same point on the slide.
//!
//! # Self-containment
//!
//! The package must present on a laptop with no network. Nothing here
//! may emit a URL: an image reaches a slide only as bytes inside
//! `ppt/media/`, and a `http(s)` source therefore takes the raster path
//! rather than riding as a link that will not resolve on stage. Fonts
//! are the documented exception — they are NAMED, not embedded, because
//! bundling faces would add megabytes to a file that gets emailed, and
//! every text box is pinned by absolute position so a substituted face
//! drifts inside its own box rather than across the slide.
//!
//! # The fallback invariant
//!
//! Every emitter may say "I cannot express this". When it does,
//! [`fallback::render`] paints that node alone through the shared scene
//! painter and embeds it as a picture at its exact rect, and its subtree
//! is not walked. So a node is never silently dropped and never guessed
//! at, and a board with nothing expressible degrades on its own to a
//! single full-slide image.

use op_editor_core::preview_slideshow::active_page_boards;
use op_editor_core::EditorState;
use op_editor_ui::layout_scene::{NodeKind, SceneFillType, SceneImageFit, SceneNode, ScenePage};
use op_editor_ui::{ImageAdjustments, ImageBlendMode, Point2D, Rect};
use std::path::Path;

use crate::export::ExportError;
use crate::export_html::board_name;

mod fallback;
mod media;
mod package;
mod picture;
mod shape;
mod text;
mod units;
mod xml;

use media::MediaLibrary;
use package::{MediaFile, SlidePart};

#[cfg(test)]
#[path = "export_pptx/tests.rs"]
mod tests;

/// What one export produced.
///
/// The fallback count is not decoration: a deck that came out mostly
/// rastered opens fine but is no longer editable, and that is worth
/// being able to see without unzipping the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeckPptxExport {
    /// Slides written — what the host reports back to the user.
    pub slides: usize,
    /// Nodes emitted as real shapes across the whole deck.
    pub structured_nodes: usize,
    /// Nodes that had to be embedded as a picture instead.
    pub raster_fallbacks: usize,
}

/// Export the active page's boards as a PowerPoint deck at `target`.
///
/// A board that fails to render aborts the whole export rather than
/// being dropped: the artifact is a single file whose slide numbering
/// would silently close over the hole, and the presenter would only
/// discover the missing slide on stage.
pub fn export_deck_pptx(state: &EditorState, target: &Path) -> Result<DeckPptxExport, ExportError> {
    let (bytes, summary) = build_deck_pptx(state)?;
    std::fs::write(target, bytes).map_err(|e| ExportError::Write(e.to_string()))?;
    Ok(summary)
}

/// The package bytes plus the summary, without touching the filesystem.
pub fn build_deck_pptx(state: &EditorState) -> Result<(Vec<u8>, DeckPptxExport), ExportError> {
    let scene = op_pen_loader::editor_state_to_active_page_export_layout_scene(state);
    let page = scene.active_page().ok_or(ExportError::NoActivePage)?;

    let mut library = MediaLibrary::default();
    let mut slides: Vec<SlidePart> = Vec::new();
    let mut slide_px: Option<(f32, f32)> = None;
    let mut summary = DeckPptxExport::default();

    for board_id in active_page_boards(state) {
        let Some(node) = page.find(&board_id) else {
            return Err(ExportError::NodeNotFoundOnPage {
                node_id: board_id,
                page_id: page.id.clone(),
            });
        };
        // Hidden boards are skipped, not failed: hiding a board is the
        // author saying it is not part of the deck.
        if node.hidden {
            continue;
        }
        let board = board_slide(page, &board_id, &mut library)?;
        summary.structured_nodes += board.structured_nodes;
        summary.raster_fallbacks += board.fallback_reasons.len();
        slide_px.get_or_insert((board.width, board.height));
        slides.push(SlidePart {
            name: board_name(state, &board_id),
            shapes: board.shapes,
            media: board.media,
        });
    }
    if slides.is_empty() {
        return Err(ExportError::NothingToExport);
    }
    summary.slides = slides.len();

    let media: Vec<MediaFile> = library.into_files();
    let bytes = package::build(slide_px.unwrap_or((1920.0, 1080.0)), &slides, &media)?;
    Ok((bytes, summary))
}

/// One board's shapes, plus what it cost to get them.
struct BoardShapes {
    shapes: String,
    width: f32,
    height: f32,
    media: Vec<usize>,
    structured_nodes: usize,
    fallback_reasons: Vec<&'static str>,
}

/// Build the shape list for one board.
///
/// Coordinates in the result are board-local: the slide's top-left is
/// `(0, 0)`, so a board can be placed without knowing where on the
/// infinite canvas it happened to live.
fn board_slide(
    page: &ScenePage,
    board_id: &str,
    library: &mut MediaLibrary,
) -> Result<BoardShapes, ExportError> {
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
        library,
        media: Vec::new(),
        // Shape id 1 is reserved for the slide's own group shape.
        next_id: 2,
        origin: bounds.origin,
        structured_nodes: 0,
        fallback_reasons: Vec::new(),
    };
    emitter.emit(board, 1.0, true)?;

    Ok(BoardShapes {
        shapes: emitter.out,
        width: bounds.size.x,
        height: bounds.size.y,
        media: emitter.media,
        structured_nodes: emitter.structured_nodes,
        fallback_reasons: emitter.fallback_reasons,
    })
}

struct Emitter<'a> {
    out: String,
    /// Package-wide media table — shared across slides so a logo on
    /// every slide is stored once.
    library: &'a mut MediaLibrary,
    /// Media indices this slide references, in relationship order.
    media: Vec<usize>,
    next_id: u32,
    /// Doc-space origin of the board. Shapes are flattened, so this is
    /// the ONLY offset applied — unlike the HTML exporter, which tracks
    /// the current parent because CSS resolves against it.
    origin: Point2D,
    structured_nodes: usize,
    fallback_reasons: Vec<&'static str>,
}

impl Emitter<'_> {
    /// `is_board` marks the slide's own root frame, whose clipping is
    /// the slide edge PowerPoint already enforces.
    fn emit(&mut self, node: &SceneNode, alpha: f32, is_board: bool) -> Result<(), ExportError> {
        if node.hidden {
            return Ok(());
        }
        if let Some(reason) = unexpressible(node, is_board) {
            return self.raster(node, reason);
        }
        // An image is resolved before anything is written: a source that
        // cannot become bytes has to take the raster path as a whole
        // node, not leave a half-emitted picture behind.
        let picture = match node.image_src.as_deref() {
            Some(src) => match self.intern_image(src) {
                Some(resolved) => Some(resolved),
                None => return self.raster(node, "image bytes could not be embedded"),
            },
            None => None,
        };

        let alpha = alpha * clamp_unit(node.composite_opacity);
        let rect = self.local(node.bounds);
        self.structured_nodes += 1;
        match &node.kind {
            NodeKind::Text => {
                let id = self.take_id();
                text::emit(&mut self.out, node, rect, alpha, id);
            }
            NodeKind::Line => {
                shape::emit_line(&mut self.out, node, self.origin, alpha, &mut self.next_id)
            }
            kind => {
                if let Some((rel_id, source_px)) = picture {
                    // A `Fit` image leaves the box's margins empty, and
                    // the loader parks a neutral grey under every image
                    // node so a failed decode reads as a placeholder.
                    // Painting that colour first reproduces the canvas;
                    // for every other mode the bitmap covers it.
                    if node.image_fit == SceneImageFit::Fit && node.fill.is_some() {
                        shape::emit_box(
                            &mut self.out,
                            node,
                            rect,
                            alpha,
                            xml::Geom::Rect,
                            &mut self.next_id,
                        );
                    }
                    let id = self.take_id();
                    picture::emit(&mut self.out, node, rect, alpha, &rel_id, source_px, id);
                } else if shape::paints_anything(node) {
                    let geom = if matches!(kind, NodeKind::Ellipse) {
                        xml::Geom::Ellipse
                    } else {
                        xml::Geom::Rect
                    };
                    shape::emit_box(&mut self.out, node, rect, alpha, geom, &mut self.next_id);
                }
                self.emit_children(node, alpha)?;
            }
        }
        Ok(())
    }

    /// Walk a container's children.
    ///
    /// Scene children are stored topmost-first (layer-panel order) and
    /// the canvas painter walks them in reverse. `<p:spTree>` order IS
    /// paint order — a later shape covers an earlier one — so the same
    /// reversal is what keeps the z-order the author sees.
    fn emit_children(&mut self, node: &SceneNode, alpha: f32) -> Result<(), ExportError> {
        for child in node.children.iter().rev() {
            self.emit(child, alpha, false)?;
        }
        Ok(())
    }

    /// Render `node` alone into the media table and place it as a
    /// picture. Its subtree is not walked afterwards.
    fn raster(&mut self, node: &SceneNode, reason: &'static str) -> Result<(), ExportError> {
        let Some((png, doc_rect)) = fallback::render(node)? else {
            // Nothing paints, so nothing is lost by writing nothing —
            // and it is not a fidelity note either.
            return Ok(());
        };
        let index = self.library.intern("png", png);
        let rel_id = self.rel_for(index);
        let id = self.take_id();
        let rect = self.local(doc_rect);
        picture::emit_raster(&mut self.out, &node.id, rect, &rel_id, id);
        self.fallback_reasons.push(reason);
        Ok(())
    }

    /// Decode an image source into the media table, returning its
    /// slide-local relationship id and the bitmap's real pixel size.
    ///
    /// `None` for a source that cannot be embedded OR cannot be
    /// measured: the placement maths for a covering image is stated in
    /// percentages of the source, so an unmeasurable bitmap would have
    /// to be cropped by guess.
    fn intern_image(&mut self, src: &str) -> Option<(String, (f32, f32))> {
        let (ext, bytes) = media::decode_data_url(src)?;
        let size = media::image_size(ext, &bytes)?;
        let index = self.library.intern(ext, bytes);
        Some((self.rel_for(index), size))
    }

    /// The slide-local relationship id for a package media index,
    /// registering it on this slide the first time it is used.
    fn rel_for(&mut self, media_index: usize) -> String {
        let position = match self.media.iter().position(|i| *i == media_index) {
            Some(existing) => existing,
            None => {
                self.media.push(media_index);
                self.media.len() - 1
            }
        };
        package::slide_media_rel_id(position)
    }

    fn take_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn local(&self, rect: Rect) -> Rect {
        let normalized = op_editor_ui::scene_bounds::normalize_rect(rect);
        Rect {
            origin: Point2D::new(
                normalized.origin.x - self.origin.x,
                normalized.origin.y - self.origin.y,
            ),
            size: normalized.size,
        }
    }
}

fn clamp_unit(v: f32) -> f32 {
    if v.is_finite() {
        v.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

/// Why this node cannot be expressed as DrawingML, or `None` when it
/// can.
///
/// The list is deliberately conservative: a paint feature DrawingML can
/// only approximate is still allowed through when the approximation is
/// bounded and documented (radial gradient extent, stroke alignment,
/// uneven corner radii), but anything whose visual result would be a
/// GUESS is refused here so the raster path can be exact instead.
fn unexpressible(n: &SceneNode, is_board: bool) -> Option<&'static str> {
    use op_editor_ui::layout_scene::Effect;

    if n.is_mask || n.mask_type.is_some() {
        // A mask reshapes its front siblings' pixels. DrawingML has no
        // equivalent relationship between shapes.
        return Some("mask");
    }
    if n.widget.is_some() {
        // Switch knobs, slider tracks, select chevrons — the canvas
        // draws a composite visual from the widget descriptor that has
        // no scene geometry to read back.
        return Some("composite widget");
    }
    if n.fill_layers.len() > 1 {
        return Some("layered fill stack");
    }
    if n.blend_mode != ImageBlendMode::Normal || n.image_blend_mode != ImageBlendMode::Normal {
        // DrawingML composites shapes source-over and offers no
        // per-shape blend operation at all.
        return Some("blend mode");
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
    for effect in &n.effects {
        match effect {
            // `a:blur` is a different operation from the painter's
            // Gaussian, and a background blur has no DrawingML spelling
            // whatsoever.
            Effect::Blur(b) if b.radius > 0.0 => return Some("layer blur"),
            Effect::BackgroundBlur { radius } if *radius > 0.0 => return Some("background blur"),
            _ => {}
        }
    }
    match &n.kind {
        // Preset geometry cannot state an arbitrary polygon or path.
        // `custGeom` could, but the path grammar is long-tail enough
        // (bezier segments, winding rules, imported SVG `d` strings)
        // that a wrong curve is likelier than a right one; v1 rasters
        // and keeps the option open.
        NodeKind::Polygon => return Some("polygon geometry"),
        NodeKind::Path => return Some("vector path"),
        NodeKind::Other(tag) if tag == "icon_font" => return Some("icon glyph"),
        NodeKind::Other(_) => return Some("unknown node kind"),
        NodeKind::Ellipse => {
            if n.arc_start_angle.is_some() || n.arc_sweep_angle.is_some() {
                // Pie / donut arcs. `prstGeom ellipse` is a full oval.
                return Some("ellipse arc");
            }
            if n.arc_inner_radius.is_some_and(|r| r > 0.0) {
                return Some("ellipse inner radius");
            }
        }
        _ => {}
    }
    if let Some(src) = n.image_src.as_deref() {
        if let Some(reason) = image_unexpressible(n, src) {
            return Some(reason);
        }
    }
    if !is_board && clipping_bites(n) {
        // A group does not clip in PowerPoint, and neither does a shape
        // with children (there is no such thing — the tree is flat), so
        // a container whose content actually overflows can only keep its
        // clip by being painted.
        return Some("clipped overflow");
    }
    None
}

fn image_unexpressible(n: &SceneNode, src: &str) -> Option<&'static str> {
    if !src.trim_start().starts_with("data:") {
        // The package has to present offline. A remote or filesystem
        // reference is not reachable there; the raster path resolves it
        // now, at export time, instead.
        return Some("image source is not embedded bytes");
    }
    if n.image_transform.is_some() {
        // Figma's normalized-UV affine crop. `srcRect` is an
        // axis-aligned inset and cannot express a rotation or shear of
        // the sampled region.
        return Some("image crop transform");
    }
    if n.image_adjustments != ImageAdjustments::default() {
        // Exposure / contrast / temperature curves are applied by the
        // painter's colour matrix; DrawingML's `duotone` / `lum` are not
        // the same curves.
        return Some("image colour adjustments");
    }
    if n.image_fit == SceneImageFit::Tile {
        // `a:tile` states its frequency as a percentage of the SHAPE,
        // while the scene states it as a scale of the source; without
        // both the repeat lands at a visibly wrong size.
        return Some("tiled image fill");
    }
    None
}

/// Whether a clipping container actually clips anything.
///
/// Most `clipContent` frames in a deck are cards whose content fits, and
/// rasterising every one of them would cost the deck its editable text
/// for nothing. So the question asked is not "does this node clip" but
/// "does any child stick out" — and only then is the node painted.
fn clipping_bites(n: &SceneNode) -> bool {
    if !n.clip_content || n.children.is_empty() {
        return false;
    }
    // Half a pixel of slop: a child sized to its parent can land a
    // rounding step outside it, and that is not an overflow anybody sees.
    const SLOP: f32 = 0.5;
    let rect = op_editor_ui::scene_bounds::normalize_rect(n.bounds);
    if rect.size.x <= 0.0 || rect.size.y <= 0.0 {
        return false;
    }
    n.children.iter().filter(|c| !c.hidden).any(|child| {
        let b = op_editor_ui::scene_bounds::normalize_rect(child.visual_bounds());
        b.size.x > 0.0
            && b.size.y > 0.0
            && (b.origin.x < rect.origin.x - SLOP
                || b.origin.y < rect.origin.y - SLOP
                || b.origin.x + b.size.x > rect.origin.x + rect.size.x + SLOP
                || b.origin.y + b.size.y > rect.origin.y + rect.size.y + SLOP)
    })
}
