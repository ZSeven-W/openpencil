use jian_ops_schema::node::PenNode;
use op_editor_ui::{layout_scene::LayoutScene, Rect};
use skia_safe::{AlphaType, Color, ColorType, ImageInfo};

use super::{build_clip, render::FrameRenderer, ClipTrigger};
use crate::{clamp_scale, encode_surface, raster_size, ExportError, RasterFormat};

/// One MCP vision result stays within the same surface budget as a still.
pub const MAX_CONTACT_SHEET_FRAMES: u32 = 8;

pub struct MotionContactSheet {
    pub png: Vec<u8>,
    pub frames: Vec<MotionFrameCell>,
}

pub struct MotionFrameCell {
    pub time_ms: u64,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// One PNG containing exact clip samples in input order, at a fixed crop.
pub fn render_motion_contact_sheet(
    scene: &LayoutScene,
    root: &PenNode,
    trigger: &ClipTrigger,
    times_ms: &[u64],
    scale: f32,
) -> Result<MotionContactSheet, ExportError> {
    if times_ms.is_empty() {
        return Err(ExportError::NothingToCapture);
    }
    if times_ms.len() > MAX_CONTACT_SHEET_FRAMES as usize {
        return Err(ExportError::TooManyMotionFrames {
            frames: times_ms.len() as u64,
            limit: MAX_CONTACT_SHEET_FRAMES,
        });
    }
    let clip = build_clip(root, trigger)?;
    let root_id = jian_core::document::tree::node_schema_id(root);
    let renderer = FrameRenderer::new(scene, root_id, clamp_scale(scale), clip)?;
    let (width, height) = renderer.size();
    let columns = if times_ms.len() <= 2 {
        times_ms.len()
    } else {
        2
    };
    let rows = times_ms.len().div_ceil(columns);
    let gap = if times_ms.len() > 1 { 8 } else { 0 };
    let sheet_width = width * columns as i32 + gap * (columns as i32 - 1);
    let sheet_height = height * rows as i32 + gap * (rows as i32 - 1);
    let size = raster_size(
        Rect::xywh(0.0, 0.0, sheet_width as f32, sheet_height as f32),
        1.0,
        0.0,
    )?;
    let info = ImageInfo::new(size, ColorType::N32, AlphaType::Premul, None);
    let mut surface =
        skia_safe::surfaces::raster(&info, None, None).ok_or(ExportError::SurfaceAlloc)?;
    surface.canvas().clear(Color::TRANSPARENT);
    let mut frames = Vec::with_capacity(times_ms.len());
    for (index, &time_ms) in times_ms.iter().enumerate() {
        let x = (index % columns) as i32 * (width + gap);
        let y = (index / columns) as i32 * (height + gap);
        let image = renderer.render(time_ms)?.image_snapshot();
        surface
            .canvas()
            .draw_image(image, (x as f32, y as f32), None);
        frames.push(MotionFrameCell {
            time_ms,
            x,
            y,
            width,
            height,
        });
    }
    let png = encode_surface(&mut surface, RasterFormat::Png)?;
    Ok(MotionContactSheet { png, frames })
}
