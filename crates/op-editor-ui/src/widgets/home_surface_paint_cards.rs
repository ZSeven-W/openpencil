//! Family-card artwork for the 制图台 Home surface: the App card's
//! three-phone flow and the raster template previews for the others.

use super::{home_palette, HomeSurface};
use crate::widgets::canvas_viewport_image::{
    has_cached_image_bytes, note_pending_decode, required_raster_edge, store_remote_image_bytes,
};
use crate::widgets::PaintCx;
use crate::{ImageDrawMode, Point2D, Rect};
use op_editor_core::{HomeFamily, HomeHit};

pub(super) fn paint_app_flow(surface: &HomeSurface<'_>, cx: &mut PaintCx<'_>, art: Rect) {
    let palette = home_palette(surface);
    let phone_w = 56.0;
    let phone_h = art.size.y.min(136.0) - 30.0;
    let y = art.origin.y + (art.size.y - phone_h) / 2.0;
    let start_x = art.origin.x + (art.size.x - phone_w * 3.0 - 48.0) / 2.0;
    for index in 0..3 {
        let x = start_x + index as f32 * (phone_w + 24.0);
        let nudged = if surface.state.hover == Some(HomeHit::Card(HomeFamily::AppUi)) && index == 1
        {
            Rect::xywh(x + 3.0, y - 3.0, phone_w, phone_h)
        } else {
            Rect::xywh(x, y, phone_w, phone_h)
        };
        let phone = nudged;
        cx.backend.fill_round_rect(phone, 8.0, palette.sheet);
        cx.backend
            .stroke_round_rect(phone, 8.0, palette.graphite, 1.2);
        cx.backend.fill_rect(
            Rect::xywh(x + 7.0, y + 12.0, phone_w - 14.0, 7.0),
            palette.line,
        );
        cx.backend.fill_rect(
            Rect::xywh(x + 7.0, y + 28.0, phone_w - 14.0, 6.0),
            palette.line,
        );
        cx.backend.fill_rect(
            Rect::xywh(x + 7.0, y + 42.0, phone_w - 14.0, 6.0),
            palette.line,
        );
        cx.backend.fill_round_rect(
            Rect::xywh(x + 7.0, y + phone_h - 22.0, phone_w - 14.0, 10.0),
            3.0,
            if index == 1 {
                palette.blue
            } else {
                palette.graphite.with_alpha(0.35)
            },
        );
        if index < 2 {
            cx.backend.stroke_line(
                Point2D::new(x + phone_w + 5.0, y + phone_h / 2.0),
                Point2D::new(x + phone_w + 18.0, y + phone_h / 2.0),
                palette.blue,
                1.5,
            );
            cx.backend.stroke_line(
                Point2D::new(x + phone_w + 14.0, y + phone_h / 2.0 - 4.0),
                Point2D::new(x + phone_w + 18.0, y + phone_h / 2.0),
                palette.blue,
                1.5,
            );
        }
    }
}

pub(super) fn paint_template_preview(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    rect: Rect,
    id: &str,
) {
    let Some(asset) = crate::widgets::scene_template_previews::scene_template_preview(id) else {
        return;
    };
    let Some(bytes) = asset.bytes else {
        op_editor_core::web_assets::request(asset.route);
        return;
    };
    if !has_cached_image_bytes(asset.image_id) {
        store_remote_image_bytes(asset.image_id, bytes.to_vec());
    }
    let max_edge = required_raster_edge(rect, cx.backend.dpi_scale());
    let sharp = cx.backend.image_decoded(asset.image_id, bytes, max_edge);
    if !sharp {
        note_pending_decode(asset.image_id, max_edge);
    }
    if sharp || cx.backend.image_resident(asset.image_id) {
        cx.backend
            .draw_image_with_mode(rect, asset.image_id, bytes, ImageDrawMode::Fill);
    } else {
        cx.backend.fill_rect(rect, home_palette(surface).paper_2);
    }
}
