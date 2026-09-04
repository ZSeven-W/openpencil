//! Shared collaboration-avatar painter for the TopBar and session panel.

use crate::collab_avatar_runtime::{
    account_avatar_image, collab_avatar_image, CollabAvatarImage, AVATAR_DECODE_EDGE_PX,
};
use crate::widgets::canvas_viewport_image::note_pending_decode;
use crate::widgets::collab_ui::CollabAvatarModel;
use crate::widgets::text_metrics;
use crate::widgets::PaintCx;
use crate::{Color, ImageDrawMode, Point2D, Rect, TextLayout};

impl std::fmt::Debug for CollabAvatarModel {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CollabAvatarModel")
            .field("participant_key", &"[REDACTED]")
            .field("display_name", &"[REDACTED]")
            .field("initials", &"[REDACTED]")
            .field("color_rgba", &self.color_rgba)
            .field("role", &self.role)
            .field("is_self", &self.is_self)
            .finish()
    }
}

/// Paint the initials fallback first, then overlay a ready raster. Fetch and
/// decode misses are merely recorded; paint never blocks on either.
pub(super) fn paint_collab_avatar(
    cx: &mut PaintCx<'_>,
    participant: &CollabAvatarModel,
    rect: Rect,
    text_size: f32,
    text_baseline_y: f32,
) -> bool {
    let radius = rect.size.x.min(rect.size.y) / 2.0;
    cx.backend
        .fill_round_rect(rect, radius, rgba_u32(participant.color_rgba));
    let initials = TextLayout::single_run(
        &participant.initials,
        "system-ui",
        text_size,
        Color::WHITE.to_jian(),
        Point2D::ZERO,
    )
    .with_font_weight(600);
    let initials_w =
        text_metrics::measure_chrome_weighted(cx.backend, &participant.initials, text_size, 600);
    cx.backend.draw_text(
        &initials,
        Point2D::new(
            rect.origin.x + (rect.size.x - initials_w) / 2.0,
            text_baseline_y,
        ),
    );

    let Some(image) = participant_avatar_image(participant) else {
        return false;
    };
    let sharp = cx.backend.image_decoded(
        image.image_id,
        image.encoded.as_ref(),
        AVATAR_DECODE_EDGE_PX,
    );
    if !sharp {
        note_pending_decode(image.image_id, AVATAR_DECODE_EDGE_PX);
    }
    if !sharp && !cx.backend.image_resident(image.image_id) {
        return false;
    }
    cx.backend.save();
    cx.backend.clip_round_rect(rect, radius);
    cx.backend.draw_image_with_mode(
        rect,
        image.image_id,
        image.encoded.as_ref(),
        ImageDrawMode::Crop,
    );
    cx.backend.restore();
    true
}

fn participant_avatar_image(participant: &CollabAvatarModel) -> Option<CollabAvatarImage> {
    if participant.is_self {
        account_avatar_image().or_else(|| collab_avatar_image(&participant.participant_key))
    } else {
        collab_avatar_image(&participant.participant_key)
    }
}

fn rgba_u32(value: u32) -> Color {
    Color {
        r: ((value >> 24) & 0xff) as f32 / 255.0,
        g: ((value >> 16) & 0xff) as f32 / 255.0,
        b: ((value >> 8) & 0xff) as f32 / 255.0,
        a: (value & 0xff) as f32 / 255.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collab_avatar_runtime::{
        complete_collab_avatar_request, lock_collab_avatar_registry_for_tests,
        take_collab_avatar_requests,
    };
    use crate::{RenderBackend, TextLayout};

    #[derive(Default)]
    struct AvatarBackend {
        image_ready: bool,
        image_draws: usize,
        text_draws: usize,
    }

    impl RenderBackend for AvatarBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {
            self.text_draws += 1;
        }
        fn clip_rect(&mut self, _: Rect) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn image_decoded(&mut self, _: u64, _: &[u8], _: u32) -> bool {
            self.image_ready
        }
        fn image_resident(&mut self, _: u64) -> bool {
            self.image_ready
        }
        fn draw_image_with_mode(&mut self, _: Rect, _: u64, _: &[u8], mode: ImageDrawMode) {
            assert_eq!(mode, ImageDrawMode::Crop);
            self.image_draws += 1;
        }
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    fn model() -> CollabAvatarModel {
        CollabAvatarModel {
            participant_key: "epoch-participant".into(),
            display_name: "Ada Lovelace".into(),
            initials: "AL".into(),
            color_rgba: 0x3366ffff,
            role: op_editor_core::CollabUiRole::Owner,
            is_self: true,
        }
    }

    fn png_header() -> Vec<u8> {
        let mut bytes = vec![0; 32];
        bytes[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        bytes[8..12].copy_from_slice(&13_u32.to_be_bytes());
        bytes[12..16].copy_from_slice(b"IHDR");
        bytes[16..20].copy_from_slice(&16_u32.to_be_bytes());
        bytes[20..24].copy_from_slice(&16_u32.to_be_bytes());
        bytes
    }

    #[test]
    fn avatar_model_debug_redacts_participant_profile() {
        let debug = format!("{:?}", model());
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("Ada Lovelace"));
        assert!(!debug.contains("epoch-participant"));
    }

    #[test]
    fn pending_or_failed_avatar_keeps_initials_fallback() {
        let _guard = lock_collab_avatar_registry_for_tests();
        assert!(crate::collab_avatar_runtime::register_collab_avatar_url(
            "epoch-participant",
            Some("https://cdn.example/avatar.png")
        ));
        let mut backend = AvatarBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        assert!(!paint_collab_avatar(
            &mut cx,
            &model(),
            Rect::xywh(0.0, 0.0, 22.0, 22.0),
            9.0,
            15.0
        ));
        let request = take_collab_avatar_requests(1).pop().unwrap();
        assert!(complete_collab_avatar_request(&request, None));
        assert!(!paint_collab_avatar(
            &mut cx,
            &model(),
            Rect::xywh(0.0, 0.0, 22.0, 22.0),
            9.0,
            15.0
        ));
        assert_eq!(backend.image_draws, 0);
        assert_eq!(backend.text_draws, 2);
    }

    #[test]
    fn ready_avatar_overlays_the_fallback_without_gui_decode() {
        let _guard = lock_collab_avatar_registry_for_tests();
        assert!(crate::collab_avatar_runtime::register_collab_avatar_url(
            "epoch-participant",
            Some("https://cdn.example/avatar.png")
        ));
        let mut backend = AvatarBackend {
            image_ready: true,
            ..Default::default()
        };
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        assert!(!paint_collab_avatar(
            &mut cx,
            &model(),
            Rect::xywh(0.0, 0.0, 22.0, 22.0),
            9.0,
            15.0
        ));
        let request = take_collab_avatar_requests(1).pop().unwrap();
        assert!(complete_collab_avatar_request(&request, Some(png_header())));
        assert!(paint_collab_avatar(
            &mut cx,
            &model(),
            Rect::xywh(0.0, 0.0, 22.0, 22.0),
            9.0,
            15.0
        ));
        assert_eq!(backend.image_draws, 1);
    }

    #[test]
    fn self_participant_reuses_the_ready_account_avatar() {
        let _guard = lock_collab_avatar_registry_for_tests();
        assert!(crate::collab_avatar_runtime::register_account_avatar_url(
            Some("https://cdn.example/account.png")
        ));
        let request = take_collab_avatar_requests(1).pop().unwrap();
        assert!(request.is_current_account());
        assert!(complete_collab_avatar_request(&request, Some(png_header())));

        let mut backend = AvatarBackend {
            image_ready: true,
            ..Default::default()
        };
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        assert!(paint_collab_avatar(
            &mut cx,
            &model(),
            Rect::xywh(0.0, 0.0, 22.0, 22.0),
            9.0,
            15.0
        ));
        assert_eq!(backend.image_draws, 1);
    }

    #[test]
    fn remote_participant_never_reuses_the_local_account_avatar() {
        let _guard = lock_collab_avatar_registry_for_tests();
        assert!(crate::collab_avatar_runtime::register_account_avatar_url(
            Some("https://cdn.example/account.png")
        ));
        let request = take_collab_avatar_requests(1).pop().unwrap();
        assert!(complete_collab_avatar_request(&request, Some(png_header())));

        let mut remote = model();
        remote.is_self = false;
        let mut backend = AvatarBackend {
            image_ready: true,
            ..Default::default()
        };
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        assert!(!paint_collab_avatar(
            &mut cx,
            &remote,
            Rect::xywh(0.0, 0.0, 22.0, 22.0),
            9.0,
            15.0
        ));
        assert_eq!(backend.image_draws, 0);
    }
}
