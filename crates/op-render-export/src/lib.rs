//! Mobile-safe raster, SVG, and PDF export.
//!
//! Raster (PNG/JPEG/WEBP) renders through the SAME scene painter the
//! live editor canvas uses — `canvas_viewport_paint::paint_node`
//! driven over an offscreen `skia_safe::Surface` via
//! `op_host_native::NativeFrameBackend` (see `scene_painter.rs`). The
//! exported pixels therefore match the canvas exactly:
//! image fills (fit/crop/tile + corner-radius + dashed placeholder),
//! `clip_content` scopes, styled text runs (weight / italic /
//! underline / strike / justify), gradients and effects.
//!
//! SVG goes through a separate hand-rolled serializer (`export_svg`)
//! — vector output, divergences documented there.
//!
//! The scene is layout-resolved: every `SceneNode::bounds` is the
//! absolute doc-space AABB jian's flex pass produced, and fills are
//! already `$ref`-resolved. Export draws straight from `bounds` with
//! no second layout pass — same input the on-screen canvas paints.
//!
//! Background: PNG / WEBP transparent; JPEG forced white (TS parity
//! — JPEG has no alpha so a "transparent" JPEG would read as black).
//! Scale: caller picks @1x / @2x / @3x (TS export dialog parity).
//! Output size is hard-capped (see [`MAX_RASTER_SIDE_PX`] /
//! [`MAX_RASTER_TOTAL_PX`]) so a huge document or screenshot padding
//! can't force a giant UI-thread allocation.

use op_editor_ui::layout_scene::{LayoutScene, SceneNode, ScenePage};
use op_editor_ui::Rect;
use skia_safe::{Canvas, EncodedImageFormat};
use std::path::Path as StdPath;

mod editor;
mod error;
pub mod motion;
pub mod pdf;
mod scene_painter;
mod svg;

pub use editor::{
    default_export_file_name, export_editor_state, export_editor_state_to_path, EditorExportScope,
    ExportArtifact,
};
pub use error::ExportError;
pub use scene_painter::{paint_node, paint_nodes};
pub use svg::{export_node_svg, export_svg, render_node_svg_bytes, render_svg_bytes};

const MARGIN: f32 = 0.0;

use op_motion_export::{clamp_scale, collect_raster_bounds as collect_bounds};
/// Hard ceilings for the offscreen raster surface. A huge page (or a
/// `debug_screenshot` request with extreme padding/scale) would
/// otherwise force a giant UI-thread allocation: 16384 px per side is
/// the common GPU/skia texture ceiling, and 64 MPx total (~256 MB of
/// N32 pixels) bounds the worst case. Exceeding either returns a
/// structured error; the MCP screenshot glue wraps it in the TS-parity
/// "Renderer reported failure: …" envelope.
pub use op_motion_export::{MAX_RASTER_SIDE_PX, MAX_RASTER_TOTAL_PX};

/// Raster export format. Matches TS ExportDialog's PNG / JPEG / WEBP
/// options; SVG has its own entry point (`export_svg`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RasterFormat {
    Png,
    Jpeg,
    Webp,
}

impl RasterFormat {
    /// Skia encoder format.
    fn skia(self) -> EncodedImageFormat {
        match self {
            RasterFormat::Png => EncodedImageFormat::PNG,
            RasterFormat::Jpeg => EncodedImageFormat::JPEG,
            RasterFormat::Webp => EncodedImageFormat::WEBP,
        }
    }
    /// Encoder quality. PNG ignores it (lossless); JPEG/WEBP land on
    /// 92 to match TS (quality 0.92 in canvas.toDataURL).
    fn quality(self) -> u32 {
        match self {
            RasterFormat::Png => 100,
            RasterFormat::Jpeg | RasterFormat::Webp => 92,
        }
    }
    /// True when the format supports transparency. JPEG doesn't, so
    /// the background must be filled before drawing.
    fn supports_alpha(self) -> bool {
        !matches!(self, RasterFormat::Jpeg)
    }
    /// Lookup by file extension (lowercase). Returns None for unknown
    /// extensions. Used by future drag-drop / scripted-export paths.
    #[allow(dead_code)]
    pub fn from_extension(ext: &str) -> Option<RasterFormat> {
        match ext {
            "png" => Some(RasterFormat::Png),
            "jpg" | "jpeg" => Some(RasterFormat::Jpeg),
            "webp" => Some(RasterFormat::Webp),
            _ => None,
        }
    }
    /// Human-readable label for error messages ("PNG" / "JPEG" / "WEBP").
    fn user_label(self) -> &'static str {
        match self {
            RasterFormat::Png => "PNG",
            RasterFormat::Jpeg => "JPEG",
            RasterFormat::Webp => "WEBP",
        }
    }

    fn ensure_supported(self) -> Result<(), ExportError> {
        // The pinned iOS and Android Skia archives omit WebP. Desktop keeps
        // the historical best-effort Skia encode path without enabling a new
        // feature that would change its pinned binary key.
        if matches!(self, RasterFormat::Webp) && cfg!(any(target_os = "ios", target_os = "android"))
        {
            return Err(ExportError::UnsupportedFormat { format: "WEBP" });
        }
        Ok(())
    }
}

/// Raster export with explicit format + scale. Scale clamped to
/// [0.5, 8.0] to keep surface allocation sane; NaN / inf fall back
/// to 2× (NaN reaching `canvas.scale` produces a garbage transform).
pub fn export_raster(
    scene: &LayoutScene,
    target: &StdPath,
    format: RasterFormat,
    scale: f32,
) -> Result<(), ExportError> {
    format.ensure_supported()?;
    let scale = clamp_scale(scale);
    let page = scene.active_page().ok_or(ExportError::NoActivePage)?;
    let bounds = page_bounds(page).ok_or(ExportError::NothingToExport)?;
    render_raster(bounds, target, format, scale, MARGIN, |canvas| {
        paint_nodes(canvas, &page.children);
    })
}

/// Raster-export a single node + its subtree by id — the "export this
/// layer" path (TS parity: `exportLayerToRaster`). The surface is
/// cropped to the node's painted bounds via the same `collect_bounds`
/// traversal `export_raster` uses for the whole page. Errors when the
/// id is unknown on the active page or the node paints nothing.
pub fn export_node_raster(
    scene: &LayoutScene,
    node_id: &str,
    target: &StdPath,
    format: RasterFormat,
    scale: f32,
) -> Result<(), ExportError> {
    export_node_raster_with_margin(scene, node_id, target, format, scale, MARGIN)
}

/// Like [`export_node_raster`] but with a caller-chosen transparent
/// border (`margin`, doc px) around the node bounds. `margin = 0.0`
/// gives a tight crop matching other tools' node exports (e.g. Pencil
/// `export_nodes`), which the render-parity benchmark needs so per-node
/// bbox comparisons aren't skewed by the default 16 px export frame.
pub fn export_node_raster_with_margin(
    scene: &LayoutScene,
    node_id: &str,
    target: &StdPath,
    format: RasterFormat,
    scale: f32,
    margin: f32,
) -> Result<(), ExportError> {
    format.ensure_supported()?;
    let scale = clamp_scale(scale);
    let page = scene.active_page().ok_or(ExportError::NoActivePage)?;
    let node = page
        .find(node_id)
        .ok_or_else(|| ExportError::NodeNotFoundOnActivePage {
            node_id: node_id.to_string(),
        })?;
    let mut acc = BoundsAcc::new();
    collect_bounds(node, glam::Affine2::IDENTITY, &mut acc);
    let bounds = acc
        .into_rect()
        .ok_or_else(|| ExportError::NodePaintsNothing {
            node_id: node_id.to_string(),
        })?;
    render_raster(bounds, target, format, scale, margin.max(0.0), |canvas| {
        paint_node(canvas, node);
    })
}

/// In-memory variant of [`export_node_raster`] — renders a single node
/// by id and returns the encoded bytes instead of writing a file. Used
/// by the `get_screenshot` MCP tool so the render core is shared with
/// the file-export path. Errors on unknown id, empty-paint node, or
/// surface-size overrun (see [`MAX_RASTER_SIDE_PX`] / [`MAX_RASTER_TOTAL_PX`]).
pub fn render_node_raster_bytes(
    scene: &LayoutScene,
    node_id: &str,
    format: RasterFormat,
    scale: f32,
) -> Result<Vec<u8>, ExportError> {
    format.ensure_supported()?;
    let page = scene.active_page().ok_or(ExportError::NoActivePage)?;
    render_node_on_page_raster_bytes(page, node_id, format, scale)
}

/// Render one resolved page to encoded raster bytes without changing or
/// consulting [`LayoutScene::active_page_index`].
pub fn render_page_raster_bytes(
    page: &ScenePage,
    format: RasterFormat,
    scale: f32,
) -> Result<Vec<u8>, ExportError> {
    format.ensure_supported()?;
    let bounds = page_bounds(page).ok_or_else(|| ExportError::PageEmpty {
        page_id: page.id.clone(),
    })?;
    render_raster_bytes(bounds, format, clamp_scale(scale), MARGIN, |canvas| {
        paint_nodes(canvas, &page.children);
    })
}

/// Render one node from its resolved containing page to encoded raster bytes.
pub fn render_node_on_page_raster_bytes(
    page: &ScenePage,
    node_id: &str,
    format: RasterFormat,
    scale: f32,
) -> Result<Vec<u8>, ExportError> {
    format.ensure_supported()?;
    let node = page
        .find(node_id)
        .ok_or_else(|| ExportError::NodeNotFoundOnPage {
            node_id: node_id.to_string(),
            page_id: page.id.clone(),
        })?;
    if node.hidden {
        return Err(ExportError::NodeHidden {
            node_id: node_id.to_string(),
        });
    }
    let mut acc = BoundsAcc::new();
    collect_bounds(node, glam::Affine2::IDENTITY, &mut acc);
    let bounds = acc
        .into_rect()
        .ok_or_else(|| ExportError::NodePaintsNothing {
            node_id: node_id.to_string(),
        })?;
    render_raster_bytes(bounds, format, clamp_scale(scale), MARGIN, |canvas| {
        paint_node(canvas, node);
    })
}

/// Shared surface-alloc + background-clear + encode + write path for
/// the whole-page and single-node raster exporters. `bounds` is the
/// painted-content rect (doc-space); `paint` draws into the canvas
/// after it has been scaled + translated so `bounds` sits at `MARGIN`.
fn render_raster(
    bounds: Rect,
    target: &StdPath,
    format: RasterFormat,
    scale: f32,
    margin: f32,
    paint: impl FnOnce(&Canvas),
) -> Result<(), ExportError> {
    let data = render_raster_bytes(bounds, format, scale, margin, paint)?;
    std::fs::write(target, data).map_err(|e| ExportError::Write(e.to_string()))?;
    Ok(())
}

/// In-memory variant of [`render_raster`] — encodes and returns the
/// raster bytes instead of writing a file. `margin` is the doc-px
/// border around `bounds` (file exports use the fixed [`MARGIN`]; the
/// MCP `debug_screenshot` path passes the caller's `padding`).
pub fn render_raster_bytes(
    bounds: Rect,
    format: RasterFormat,
    scale: f32,
    margin: f32,
    paint: impl FnOnce(&Canvas),
) -> Result<Vec<u8>, ExportError> {
    format.ensure_supported()?;
    let size = raster_size(bounds, scale, margin)?;
    let mut surface = raster_surface(size, bounds, scale, margin, format.supports_alpha(), paint)?;
    encode_surface(&mut surface, format)
}

fn raster_size(bounds: Rect, scale: f32, margin: f32) -> Result<(i32, i32), ExportError> {
    Ok(op_motion_export::raster_size(bounds, scale, margin)?)
}

fn raster_surface(
    size: (i32, i32),
    bounds: Rect,
    scale: f32,
    margin: f32,
    transparent: bool,
    paint: impl FnOnce(&Canvas),
) -> Result<skia_safe::Surface, ExportError> {
    let info = skia_safe::ImageInfo::new(
        size,
        skia_safe::ColorType::N32,
        skia_safe::AlphaType::Premul,
        None,
    );
    let mut surface =
        skia_safe::surfaces::raster(&info, None, None).ok_or(ExportError::SurfaceAlloc)?;
    let canvas = surface.canvas();
    if transparent {
        canvas.clear(skia_safe::Color::TRANSPARENT);
    } else {
        canvas.clear(skia_safe::Color::WHITE);
    }
    canvas.scale((scale, scale));
    canvas.translate((margin - bounds.origin.x, margin - bounds.origin.y));
    paint(canvas);
    Ok(surface)
}

fn encode_surface(
    surface: &mut skia_safe::Surface,
    format: RasterFormat,
) -> Result<Vec<u8>, ExportError> {
    let image = surface.image_snapshot();
    let data = image
        .encode(None, format.skia(), format.quality())
        .ok_or(ExportError::Encode {
            format: format.user_label(),
        })?;
    Ok(data.as_bytes().to_vec())
}

/// Bounding rect over every paintable node on `page`. The scene is
/// already layout-resolved, so each node's `bounds` is its absolute
/// doc-space AABB — but rotation, strokes and `node.points` can push
/// painted pixels outside `bounds`, so traversal mirrors `paint_node`
/// exactly and threads the cumulative transform.
pub fn page_bounds(page: &ScenePage) -> Option<Rect> {
    let mut acc = BoundsAcc::new();
    for n in &page.children {
        collect_bounds(n, glam::Affine2::IDENTITY, &mut acc);
    }
    acc.into_rect()
}

/// Painted bounds for one node and its visible subtree.
pub fn node_bounds(node: &SceneNode) -> Option<Rect> {
    let mut acc = BoundsAcc::new();
    collect_bounds(node, glam::Affine2::IDENTITY, &mut acc);
    acc.into_rect()
}

pub(crate) use op_editor_ui::scene_bounds::BoundsAcc;

#[cfg(test)]
pub mod test_support {
    //! Shared helpers for the export test modules — build a
    //! `LayoutScene` directly without going through `op-pen-loader`.

    use op_editor_ui::layout_scene::NodeKind;
    use op_editor_ui::layout_scene::{LayoutScene, SceneNode, ScenePage};
    use op_editor_ui::{Color, Rect};

    /// A single-page scene holding `children` as top-level nodes.
    pub fn scene_with(children: Vec<SceneNode>) -> LayoutScene {
        LayoutScene {
            pages: vec![ScenePage {
                id: "p1".into(),
                name: "Page 1".into(),
                children,
            }],
            active_page_index: 0,
        }
    }

    /// A filled rectangle scene node at `(x, y, w, h)`.
    pub fn filled_rect(id: &str, x: f32, y: f32, w: f32, h: f32, fill: Color) -> SceneNode {
        let mut n = SceneNode::leaf(id, NodeKind::Rect);
        n.bounds = Rect::xywh(x, y, w, h);
        n.fill = Some(fill);
        n
    }
}

#[cfg(test)]
mod tests;
