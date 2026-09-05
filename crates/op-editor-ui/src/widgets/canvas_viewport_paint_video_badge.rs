//! Paint geometry for the play badge shown on video poster nodes.

use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};

const BADGE_DIAMETER_FRACTION: f32 = 0.22;
const BADGE_MIN_DIAMETER: f32 = 24.0;
const BADGE_MAX_DIAMETER: f32 = 64.0;
const TRIANGLE_SIZE: f32 = 0.45;
const TRIANGLE_SHIFT: f32 = 0.06;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct VideoBadgeGeometry {
    pub circle: Rect,
    pub triangle: [Point2D; 3],
}

/// Compute the badge in viewport coordinates while sizing it in scene units.
/// This keeps the 24–64 scene-pixel clamp stable and lets zoom scale the badge
/// together with the poster and the rest of the canvas.
pub(crate) fn video_badge_geometry(
    scene_rect: Rect,
    world_rect: Rect,
    zoom: f32,
) -> VideoBadgeGeometry {
    let scene_diameter = (scene_rect.size.x.abs().min(scene_rect.size.y.abs())
        * BADGE_DIAMETER_FRACTION)
        .clamp(BADGE_MIN_DIAMETER, BADGE_MAX_DIAMETER);
    let diameter = scene_diameter * zoom.abs();
    let center = Point2D::new(
        world_rect.origin.x + world_rect.size.x / 2.0,
        world_rect.origin.y + world_rect.size.y / 2.0,
    );
    let circle = Rect::xywh(
        center.x - diameter / 2.0,
        center.y - diameter / 2.0,
        diameter,
        diameter,
    );
    let shift = diameter * TRIANGLE_SHIFT;
    let half_height = diameter * TRIANGLE_SIZE / 2.0;
    let base_x = center.x - diameter * TRIANGLE_SIZE / 3.0 + shift;
    let point_x = base_x + diameter * TRIANGLE_SIZE;
    let triangle = [
        Point2D::new(base_x, center.y - half_height),
        Point2D::new(base_x, center.y + half_height),
        Point2D::new(point_x, center.y),
    ];
    VideoBadgeGeometry { circle, triangle }
}

pub(crate) fn paint_video_badge(
    cx: &mut PaintCx<'_>,
    scene_rect: Rect,
    world_rect: Rect,
    zoom: f32,
) {
    let geometry = video_badge_geometry(scene_rect, world_rect, zoom);
    cx.backend
        .fill_oval(geometry.circle, Color::rgba_u8(0, 0, 0, 0.55));
    cx.backend.fill_polygon(&geometry.triangle, Color::WHITE);
}

#[cfg(test)]
mod tests {
    use super::{paint_video_badge, video_badge_geometry};
    use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};

    #[derive(Default)]
    struct BadgeRecorder {
        circles: Vec<(Rect, Color)>,
        triangles: Vec<Vec<Point2D>>,
    }

    impl RenderBackend for BadgeRecorder {
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
        fn fill_oval(&mut self, bounds: Rect, color: Color) {
            self.circles.push((bounds, color));
        }
        fn fill_polygon(&mut self, points: &[Point2D], _: Color) {
            self.triangles.push(points.to_vec());
        }
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    fn close(a: f32, b: f32) {
        assert!((a - b).abs() < 0.001, "{a} != {b}");
    }

    #[test]
    fn badge_diameter_clamps_in_scene_units_before_zoom() {
        let small = video_badge_geometry(
            Rect::xywh(0.0, 0.0, 100.0, 100.0),
            Rect::xywh(10.0, 20.0, 100.0, 100.0),
            1.0,
        );
        close(small.circle.size.x, 24.0);

        let large = video_badge_geometry(
            Rect::xywh(0.0, 0.0, 500.0, 500.0),
            Rect::xywh(10.0, 20.0, 100.0, 100.0),
            2.0,
        );
        close(large.circle.size.x, 128.0);
    }

    #[test]
    fn triangle_is_shifted_right_of_the_badge_center() {
        let geometry = video_badge_geometry(
            Rect::xywh(0.0, 0.0, 200.0, 200.0),
            Rect::xywh(0.0, 0.0, 200.0, 200.0),
            1.0,
        );
        let triangle_center = geometry.triangle.iter().map(|point| point.x).sum::<f32>() / 3.0;
        let circle_center = geometry.circle.origin.x + geometry.circle.size.x / 2.0;
        close(
            triangle_center - circle_center,
            geometry.circle.size.x * 0.06,
        );
    }

    #[test]
    fn painting_emits_one_circle_and_one_triangle() {
        let mut backend = BadgeRecorder::default();
        let mut cx = crate::widgets::PaintCx {
            backend: &mut backend,
        };
        paint_video_badge(
            &mut cx,
            Rect::xywh(0.0, 0.0, 200.0, 100.0),
            Rect::xywh(20.0, 30.0, 400.0, 200.0),
            2.0,
        );

        assert_eq!(backend.circles.len(), 1);
        assert_eq!(backend.circles[0].1, Color::rgba_u8(0, 0, 0, 0.55));
        assert_eq!(backend.triangles.len(), 1);
        assert_eq!(backend.triangles[0].len(), 3);
    }
}
