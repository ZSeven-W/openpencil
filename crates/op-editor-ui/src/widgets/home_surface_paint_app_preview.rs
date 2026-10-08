//! Home's phone showcase: three separate portrait pages with breathing room.

use super::fade;
use super::StudioPalette;
use crate::widgets::canvas_viewport_image::{
    has_cached_image_bytes, note_pending_decode, required_raster_edge, store_remote_image_bytes,
};
use crate::widgets::scene_template_previews::coffee_app_screen_preview;
use crate::widgets::PaintCx;
use crate::{Color, ImageAdjustments, ImageDrawMode, Point2D, Rect};

const PHONE_ASPECT: f32 = 375.0 / 812.0;

pub(super) fn paint(cx: &mut PaintCx<'_>, area: Rect, palette: StudioPalette, opacity: f32) {
    let gap = (area.size.x * 0.045).clamp(6.0, 24.0);
    let pad = (area.size.x * 0.01).clamp(2.0, 6.0);
    let outer = (area.size.x * 0.025).clamp(3.0, 16.0);
    let tape_room = (area.size.y * 0.07).clamp(6.0, 18.0);
    let width = ((area.size.x - outer * 2.0 - gap * 2.0 - pad * 6.0) / 2.76)
        .min((area.size.y - tape_room - pad * 2.0) * PHONE_ASPECT);
    if width <= 0.0 {
        return;
    }
    let group_w = width * 2.76 + pad * 6.0 + gap * 2.0;
    let center_h = width / PHONE_ASPECT + pad * 2.0;
    let top = area.origin.y + tape_room + (area.size.y - tape_room - center_h) / 2.0;
    let mut x = area.origin.x + (area.size.x - group_w) / 2.0;
    // Side pages are slightly smaller and lower. All three remain complete,
    // with a visible gutter between their individual phone frames.
    for (index, scale) in [0.88, 1.0, 0.88].into_iter().enumerate() {
        let image_w = width * scale;
        let image_h = image_w / PHONE_ASPECT;
        let frame_h = image_h + pad * 2.0;
        let frame = Rect::xywh(
            x,
            top + (center_h - frame_h) * 0.65,
            image_w + pad * 2.0,
            frame_h,
        );
        paint_phone(cx, frame, pad, index, palette, opacity);
        if index == 1 {
            paint_tape(cx, frame, opacity);
        }
        x += frame.size.x + gap;
    }
}

fn paint_phone(
    cx: &mut PaintCx<'_>,
    frame: Rect,
    pad: f32,
    index: usize,
    palette: StudioPalette,
    opacity: f32,
) {
    let Some(asset) = coffee_app_screen_preview(index) else {
        return;
    };
    let radius = (pad * 1.5).clamp(4.0, 9.0);
    cx.backend
        .fill_drop_shadow(frame, radius, 10.0, fade(palette.ink, 0.13 * opacity));
    cx.backend
        .fill_round_rect(frame, radius, fade(palette.panel, opacity));
    cx.backend
        .stroke_round_rect(frame, radius, fade(palette.line, opacity), 1.0);
    let image = Rect::xywh(
        frame.origin.x + pad,
        frame.origin.y + pad,
        frame.size.x - pad * 2.0,
        frame.size.y - pad * 2.0,
    );
    let Some(bytes) = asset.bytes else {
        op_editor_core::web_assets::request(asset.route);
        cx.backend
            .fill_round_rect(image, 3.0, fade(palette.preview, opacity));
        return;
    };
    if !has_cached_image_bytes(asset.image_id) {
        store_remote_image_bytes(asset.image_id, bytes.to_vec());
    }
    let max_edge = required_raster_edge(image, cx.backend.dpi_scale());
    let sharp = cx.backend.image_decoded(asset.image_id, bytes, max_edge);
    if !sharp {
        note_pending_decode(asset.image_id, max_edge);
    }
    if sharp || cx.backend.image_resident(asset.image_id) {
        cx.backend.save();
        cx.backend.clip_round_rect(image, 3.0);
        cx.backend.draw_image_with_options(
            image,
            asset.image_id,
            bytes,
            ImageDrawMode::Fill,
            ImageAdjustments::default(),
            opacity,
            0.0,
        );
        cx.backend.restore();
    } else {
        cx.backend
            .fill_round_rect(image, 3.0, fade(palette.preview, opacity));
    }
}

fn paint_tape(cx: &mut PaintCx<'_>, frame: Rect, opacity: f32) {
    let w = (frame.size.x * 0.38).clamp(12.0, 38.0);
    let h = (w * 0.36).max(4.0);
    let tape = Rect::xywh(
        frame.origin.x + (frame.size.x - w) / 2.0,
        frame.origin.y - h * 0.45,
        w,
        h,
    );
    cx.backend.save();
    cx.backend.rotate(
        -8.0_f32.to_radians(),
        Point2D::new(tape.origin.x + w / 2.0, tape.origin.y + h / 2.0),
    );
    cx.backend
        .fill_round_rect(tape, 2.0, Color::rgba_u8(0xE3, 0xCC, 0xA6, 0.49 * opacity));
    cx.backend.restore();
}
