//! Current-account avatar overlay shared by account-facing widgets.

use crate::collab_avatar_runtime::account_avatar_image;
use crate::widgets::canvas_viewport_image::{note_pending_decode, required_raster_edge};
use crate::widgets::PaintCx;
use crate::{ImageDrawMode, Rect};
use op_editor_core::AccountState;

/// Overlay a ready account image on top of the caller's initials fallback.
///
/// Fetch and decode misses are recorded only; paint never blocks. Returning
/// `false` means the initials remain the visible fallback.
pub(super) fn paint_account_avatar_image(
    cx: &mut PaintCx<'_>,
    account: &AccountState,
    rect: Rect,
) -> bool {
    if matches!(account, AccountState::Anonymous) {
        return false;
    }
    let Some(image) = account_avatar_image() else {
        return false;
    };
    let decode_edge = required_raster_edge(rect, cx.backend.dpi_scale());
    let sharp = cx
        .backend
        .image_decoded(image.image_id, image.encoded.as_ref(), decode_edge);
    if !sharp {
        note_pending_decode(image.image_id, decode_edge);
    }
    if !sharp && !cx.backend.image_resident(image.image_id) {
        return false;
    }

    let radius = rect.size.x.min(rect.size.y) / 2.0;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collab_avatar_runtime::{
        complete_collab_avatar_request, lock_collab_avatar_registry_for_tests,
        register_account_avatar_url, take_collab_avatar_requests,
    };
    use crate::{Color, Point2D, RenderBackend, TextLayout};

    #[derive(Default)]
    struct AvatarBackend {
        image_ready: bool,
        image_draws: usize,
        decode_edges: Vec<u32>,
        dpi_scale: f32,
    }

    impl RenderBackend for AvatarBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn image_decoded(&mut self, _: u64, _: &[u8], max_edge_px: u32) -> bool {
            self.decode_edges.push(max_edge_px);
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
            self.dpi_scale
        }
    }

    fn account() -> AccountState {
        AccountState::SignedIn {
            display_name: "Kayshen".into(),
            username: "kayshen".into(),
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
    fn ready_profile_image_overlays_the_initials_fallback() {
        let _guard = lock_collab_avatar_registry_for_tests();
        assert!(register_account_avatar_url(Some(
            "https://cdn.example/account.png"
        )));
        let account = account();
        let rect = Rect::xywh(0.0, 0.0, 44.0, 44.0);
        let mut backend = AvatarBackend {
            image_ready: true,
            dpi_scale: 2.0,
            ..Default::default()
        };
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        assert!(!paint_account_avatar_image(&mut cx, &account, rect));
        let request = take_collab_avatar_requests(1).pop().unwrap();
        assert!(complete_collab_avatar_request(&request, Some(png_header())));
        assert!(paint_account_avatar_image(&mut cx, &account, rect));
        assert_eq!(backend.image_draws, 1);
        assert_eq!(backend.decode_edges, vec![128]);
    }

    #[test]
    fn absent_or_unsafe_url_keeps_the_initials_fallback() {
        let _guard = lock_collab_avatar_registry_for_tests();
        let rect = Rect::xywh(0.0, 0.0, 20.0, 20.0);
        let mut backend = AvatarBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        assert!(!paint_account_avatar_image(
            &mut cx,
            &AccountState::Anonymous,
            rect
        ));
        assert!(!register_account_avatar_url(Some(
            "http://127.0.0.1/avatar.png"
        )));
        assert!(!paint_account_avatar_image(&mut cx, &account(), rect));
        assert!(take_collab_avatar_requests(1).is_empty());
        assert_eq!(backend.image_draws, 0);
    }
}
