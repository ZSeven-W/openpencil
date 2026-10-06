//! Preview video placement for the CanvasKit DOM media layer.
//!
//! The preview session owns scene-space video metadata. This host module only
//! maps those node rects through the active Canvas, slideshow, or device-frame
//! presentation so `video_overlay` can reconcile CSS positions after paint.

use super::WidgetHost;
use crate::video_overlay::{scene_video_rect, VideoOverlayPlacement};
use op_editor_ui::Point2D;

impl WidgetHost {
    /// Build the current preview video placements in logical host-screen
    /// coordinates. A missing preview session intentionally returns an empty
    /// list, which removes any stale DOM videos on the next repaint.
    pub(crate) fn preview_video_overlay_placements(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Vec<VideoOverlayPlacement> {
        let Some(session) = self.preview.as_ref() else {
            return Vec::new();
        };

        if self.device_mode_active() {
            return self.device_video_placements(session);
        }

        let canvas = self.preview_canvas_rect(viewport_w, viewport_h);
        let viewport = self.editor_state.viewport;
        let origin = Point2D::new(
            canvas.origin.x + viewport.pan_x,
            canvas.origin.y + viewport.pan_y,
        );
        let root = if self.preview_slideshow_active() {
            self.editor_state
                .preview_slideshow()
                .and_then(|slideshow| slideshow.current_board())
        } else {
            None
        };
        session
            .video_overlays(root)
            .into_iter()
            .map(|video| VideoOverlayPlacement {
                node_id: video.node_id.clone(),
                screen_rect: scene_video_rect(video.scene_rect, origin, viewport.zoom),
                video,
            })
            .collect()
    }

    fn device_video_placements(
        &self,
        session: &op_preview_core::PreviewSession,
    ) -> Vec<VideoOverlayPlacement> {
        let Some(frame) = self.preview_device_frame.as_ref() else {
            return Vec::new();
        };
        let Some((root_id, _)) = session.framed_root() else {
            return Vec::new();
        };
        let videos = session.video_overlays(Some(root_id.as_str()));
        videos
            .into_iter()
            .map(|video| {
                let screen_rect = if let Some(pinned) = frame.pinned.as_ref().filter(|pinned| {
                    session.video_overlay_is_in_subtree(&pinned.node_id, &video.node_id)
                }) {
                    scene_video_rect(
                        video.scene_rect,
                        Point2D::new(
                            pinned.paint_origin.x - pinned.node_scene.origin.x * frame.fit,
                            pinned.paint_origin.y - pinned.node_scene.origin.y * frame.fit,
                        ),
                        frame.fit,
                    )
                } else if let Some(pinned_top) = frame.pinned_top.as_ref().filter(|pinned| {
                    session.video_overlay_is_in_subtree(&pinned.node_id, &video.node_id)
                }) {
                    scene_video_rect(
                        video.scene_rect,
                        Point2D::new(
                            pinned_top.paint_origin.x - pinned_top.node_scene.origin.x * frame.fit,
                            pinned_top.paint_origin.y - pinned_top.node_scene.origin.y * frame.fit,
                        ),
                        frame.fit,
                    )
                } else {
                    scene_video_rect(
                        video.scene_rect,
                        Point2D::new(
                            frame.content_origin.x,
                            frame.content_origin.y - self.preview_scroll_y * frame.fit,
                        ),
                        frame.fit,
                    )
                };
                VideoOverlayPlacement {
                    node_id: video.node_id.clone(),
                    screen_rect,
                    video,
                }
            })
            .collect()
    }
}
