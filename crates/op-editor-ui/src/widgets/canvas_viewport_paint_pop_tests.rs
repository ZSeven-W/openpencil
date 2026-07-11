//! Sibling test file for the reveal scale-pop in
//! `canvas_viewport_paint.rs` (800-line cap convention).

mod reveal_pop_tests {
    use crate::layout_scene::{NodeKind, SceneNode};
    use crate::widgets::canvas_viewport_paint::{
        paint_node_with_options, reveal_pop_scale, RevealSchedule, REVEAL_POP_MS,
        REVEAL_WIREFRAME_MS,
    };
    use crate::widgets::PaintCx;
    use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
    use std::collections::HashMap;

    #[derive(Default)]
    struct PopCaptureBackend {
        scales: Vec<(Point2D, Point2D)>,
        saves: usize,
    }

    impl RenderBackend for PopCaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {}
        fn save(&mut self) {
            self.saves += 1;
        }
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn scale(&mut self, scale: Point2D, pivot: Point2D) {
            self.scales.push((scale, pivot));
        }
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    fn rect_node() -> SceneNode {
        let mut node = SceneNode::leaf("new-node", NodeKind::Rect);
        node.bounds = Rect::xywh(10.0, 20.0, 120.0, 48.0);
        node
    }

    fn paint_at(now_ms: u64) -> PopCaptureBackend {
        let node = rect_node();
        let mut reveals = HashMap::new();
        reveals.insert("new-node".to_string(), 1_000u64);
        let mut backend = PopCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        let cull = Rect::xywh(-10_000.0, -10_000.0, 40_000.0, 40_000.0);
        let _ = paint_node_with_options(
            &mut cx,
            &node,
            Point2D::new(0.0, 0.0),
            1.0,
            None,
            cull,
            Some(RevealSchedule {
                starts: &reveals,
                now_ms,
            }),
            None,
            None,
            None,
        );
        backend
    }

    #[test]
    fn fresh_reveal_paints_through_a_scale_pop() {
        // Probe inside the pop window, which begins after the wireframe
        // ghost beat (REVEAL_WIREFRAME_MS).
        let backend = paint_at(1_000 + REVEAL_WIREFRAME_MS + 50);
        assert_eq!(backend.scales.len(), 1, "pop applies exactly one scale");
        let (scale, pivot) = backend.scales[0];
        assert!((scale.x - scale.y).abs() < 1e-6, "uniform scale");
        assert!(scale.x > 0.84 && scale.x < 1.03);
        assert!(
            (pivot.x - 70.0).abs() < 0.01 && (pivot.y - 44.0).abs() < 0.01,
            "pop pivots on the node centre"
        );
        assert_eq!(backend.saves, 1, "pop wraps paint in save/restore");
    }

    #[test]
    fn settled_reveal_paints_without_transform() {
        let backend = paint_at(1_000 + REVEAL_WIREFRAME_MS + REVEAL_POP_MS + 100);
        assert!(backend.scales.is_empty());
        assert_eq!(backend.saves, 0);
    }

    #[test]
    fn pending_reveal_still_hides_the_node() {
        let backend = paint_at(900);
        assert!(backend.scales.is_empty());
        assert_eq!(backend.saves, 0, "pending node paints nothing at all");
    }

    #[test]
    fn pop_scale_curve_starts_small_overshoots_then_settles() {
        assert!((reveal_pop_scale(0).unwrap() - 0.85).abs() < 0.01);
        let mid = reveal_pop_scale(90).unwrap();
        assert!(mid > 0.85 && mid < 1.05);
        let late = reveal_pop_scale(REVEAL_POP_MS - 1).unwrap();
        assert!((late - 1.0).abs() < 0.03);
        assert!(reveal_pop_scale(REVEAL_POP_MS).is_none());
    }
}
