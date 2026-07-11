//! Preview device-frame painter entries: paint one selected page root
//! (optionally skipping one node) and paint a single subtree at an
//! explicit screen origin. Thin wrappers over the canvas painter's
//! node walk so the device frame reuses the exact design-canvas paint.

use crate::layout_scene::ScenePage;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};

/// Options for [`paint_scene_page_with`].
pub struct PaintSceneOptions<'a> {
    /// Paint only this page-root subtree (device frame pass 1). `None`
    /// paints every root, exactly like `paint_scene_page`.
    pub only_root: Option<&'a str>,
    /// Skip one node id (the pinned bottom nav) during the walk.
    pub skip_node: Option<&'a str>,
}

/// `paint_scene_page` with root selection + single-node exclusion.
/// Children paint back-to-front (`.rev()`), matching the design
/// canvas walk so z-order is identical.
pub fn paint_scene_page_with(
    cx: &mut PaintCx<'_>,
    page: &ScenePage,
    viewport_origin: Point2D,
    zoom: f32,
    cull: Rect,
    options: PaintSceneOptions<'_>,
) {
    for child in page.children.iter().rev() {
        if let Some(root) = options.only_root {
            if child.id != root {
                continue;
            }
        }
        super::canvas_viewport_paint::paint_node_with_options_hiding(
            cx,
            child,
            viewport_origin,
            zoom,
            None,
            cull,
            None,
            None,
            None,
            None,
            options.skip_node,
            0,
            None,
            None,
            None,
        );
    }
}

/// Paint ONE subtree at an explicit SCREEN origin (the pinned-nav
/// pass): the walk offsets the subtree's authored scene origin out,
/// so `origin` is where the subtree's top-left lands on screen.
pub fn paint_scene_subtree(
    cx: &mut PaintCx<'_>,
    page: &ScenePage,
    node_id: &str,
    origin: Point2D,
    zoom: f32,
) {
    let Some(node) = page.find(node_id) else {
        return;
    };
    let viewport_origin = Point2D::new(
        origin.x - node.bounds.origin.x * zoom,
        origin.y - node.bounds.origin.y * zoom,
    );
    let cull = Rect {
        origin: Point2D::new(f32::MIN / 2.0, f32::MIN / 2.0),
        size: Point2D::new(f32::MAX, f32::MAX),
    };
    super::canvas_viewport_paint::paint_node_with_options_hiding(
        cx,
        node,
        viewport_origin,
        zoom,
        None,
        cull,
        None,
        None,
        None,
        None,
        None,
        0,
        None,
        None,
        None,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout_scene::{NodeKind, SceneNode};
    use crate::{Color, ImageDrawMode, RenderBackend, TextLayout};

    /// Records fill_rect calls; every other backend method is a no-op.
    /// Same pattern as `canvas_viewport_paint_tests::TextCaptureBackend`.
    #[derive(Default)]
    struct FillCaptureBackend {
        fill_rects: Vec<Rect>,
    }

    impl RenderBackend for FillCaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, rect: Rect, _: Color) {
            self.fill_rects.push(rect);
        }
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn scale(&mut self, _: Point2D, _: Point2D) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, rect: Rect, _: f32, _: Color) {
            self.fill_rects.push(rect);
        }
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn draw_image(&mut self, _: Rect, _: u64, _: &[u8]) {}
        fn draw_image_with_mode(&mut self, _: Rect, _: u64, _: &[u8], _: ImageDrawMode) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    fn rect_node(id: &str, x: f32, y: f32, w: f32, h: f32) -> SceneNode {
        let bounds = Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(w, h),
        };
        let mut n = SceneNode::leaf(id, NodeKind::Rect);
        n.bounds = bounds;
        n.aggregate_bounds_cache = bounds;
        n.opacity = 1.0;
        n.fill = Some(Color {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        });
        n
    }

    fn two_root_page() -> ScenePage {
        let mut root_a = rect_node("root_a", 0.0, 0.0, 100.0, 100.0);
        root_a.children = vec![rect_node("c1", 10.0, 10.0, 20.0, 20.0)];
        let root_b = rect_node("root_b", 500.0, 0.0, 100.0, 100.0);
        ScenePage {
            id: "p".into(),
            name: "p".into(),
            children: vec![root_a, root_b],
        }
    }

    fn wide_cull() -> Rect {
        Rect {
            origin: Point2D::new(-10_000.0, -10_000.0),
            size: Point2D::new(20_000.0, 20_000.0),
        }
    }

    #[test]
    fn only_root_paints_exactly_that_root() {
        let page = two_root_page();
        let mut be = FillCaptureBackend::default();
        {
            let mut cx = PaintCx { backend: &mut be };
            paint_scene_page_with(
                &mut cx,
                &page,
                Point2D::new(0.0, 0.0),
                1.0,
                wide_cull(),
                PaintSceneOptions {
                    only_root: Some("root_a"),
                    skip_node: None,
                },
            );
        }
        assert!(
            be.fill_rects.iter().all(|r| r.origin.x < 400.0),
            "root_b leaked into an only_root=root_a paint: {:?}",
            be.fill_rects
        );
        assert_eq!(be.fill_rects.len(), 2, "root_a + c1 exactly");
    }

    #[test]
    fn skip_node_excludes_the_subtree() {
        let page = two_root_page();
        let mut be = FillCaptureBackend::default();
        {
            let mut cx = PaintCx { backend: &mut be };
            paint_scene_page_with(
                &mut cx,
                &page,
                Point2D::new(0.0, 0.0),
                1.0,
                wide_cull(),
                PaintSceneOptions {
                    only_root: Some("root_a"),
                    skip_node: Some("c1"),
                },
            );
        }
        assert_eq!(be.fill_rects.len(), 1, "c1 must be skipped");
    }

    #[test]
    fn subtree_paints_at_the_given_screen_origin() {
        let page = two_root_page();
        let mut be = FillCaptureBackend::default();
        {
            let mut cx = PaintCx { backend: &mut be };
            paint_scene_subtree(&mut cx, &page, "c1", Point2D::new(200.0, 300.0), 2.0);
        }
        assert_eq!(be.fill_rects.len(), 1);
        let r = be.fill_rects[0];
        assert!((r.origin.x - 200.0).abs() < 0.5, "got {:?}", r);
        assert!((r.origin.y - 300.0).abs() < 0.5, "got {:?}", r);
        assert!((r.size.x - 40.0).abs() < 0.5, "20 * zoom 2");
    }

    #[test]
    fn subtree_with_unknown_id_paints_nothing() {
        let page = two_root_page();
        let mut be = FillCaptureBackend::default();
        {
            let mut cx = PaintCx { backend: &mut be };
            paint_scene_subtree(&mut cx, &page, "nope", Point2D::new(0.0, 0.0), 1.0);
        }
        assert!(be.fill_rects.is_empty());
    }
}
