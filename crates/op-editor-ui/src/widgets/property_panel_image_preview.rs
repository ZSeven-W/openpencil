//! Shared image-fill preview helpers for the Fill row and popover.

use crate::widgets::PaintCx;
use crate::{ImageAdjustments, ImageDrawMode, Rect};

pub(crate) fn paint_image_preview(
    cx: &mut PaintCx<'_>,
    rect: Rect,
    src: &str,
    summary: &op_editor_core::ImageFillSummary,
) -> bool {
    let Some(bytes) = data_url_bytes(src) else {
        return false;
    };
    cx.backend.save();
    cx.backend.clip_rect(rect);
    cx.backend.draw_image_with_options(
        rect,
        src_hash(src),
        &bytes,
        mode_to_draw_mode(summary.mode),
        summary_adjustments(summary),
    );
    cx.backend.restore();
    true
}

fn summary_adjustments(summary: &op_editor_core::ImageFillSummary) -> ImageAdjustments {
    ImageAdjustments {
        exposure: summary.exposure,
        contrast: summary.contrast,
        saturation: summary.saturation,
        temperature: summary.temperature,
        tint: summary.tint,
        highlights: summary.highlights,
        shadows: summary.shadows,
    }
}

fn mode_to_draw_mode(mode: op_editor_core::ImageFillMode) -> ImageDrawMode {
    match mode {
        op_editor_core::ImageFillMode::Fill => ImageDrawMode::Fill,
        op_editor_core::ImageFillMode::Fit => ImageDrawMode::Fit,
        op_editor_core::ImageFillMode::Crop => ImageDrawMode::Crop,
        op_editor_core::ImageFillMode::Tile => ImageDrawMode::Tile,
    }
}

fn data_url_bytes(src: &str) -> Option<Vec<u8>> {
    let after_scheme = src.strip_prefix("data:")?;
    let comma = after_scheme.find(',')?;
    let meta = &after_scheme[..comma];
    let payload = &after_scheme[comma + 1..];
    if !meta.contains(";base64") {
        return None;
    }
    let clean: String = payload
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .collect();
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine as _;
    B64.decode(clean.as_bytes()).ok()
}

fn src_hash(src: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut h);
    h.finish()
}
