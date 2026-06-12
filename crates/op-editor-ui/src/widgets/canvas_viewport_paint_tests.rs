//! Sibling test file for `canvas_viewport_paint.rs` (800-line cap
//! convention) — arc tessellation, text-node paint, SVG-path paint,
//! path flattening and `clipContent` child clipping.

mod arc_tests {
    use crate::widgets::canvas_viewport_paint::arc_polygon;
    use crate::Rect;

    #[test]
    fn pie_polygon_starts_at_centre() {
        let poly = arc_polygon(Rect::xywh(0.0, 0.0, 100.0, 100.0), 0.0, 90.0, 0.0);
        assert_eq!(poly[0].x, 50.0);
        assert_eq!(poly[0].y, 50.0);
        assert!((poly[1].x - 100.0).abs() < 0.01);
        assert!((poly[1].y - 50.0).abs() < 0.01);
    }

    #[test]
    fn donut_polygon_has_outer_and_inner_rings() {
        let poly = arc_polygon(Rect::xywh(0.0, 0.0, 100.0, 100.0), 0.0, 360.0, 0.5);
        assert_eq!(poly.len(), 2 * (90 + 1));
        let last = poly[poly.len() - 1];
        let dist = ((last.x - 50.0).powi(2) + (last.y - 50.0).powi(2)).sqrt();
        assert!((dist - 25.0).abs() < 0.5, "inner radius ~25, got {dist}");
    }

    #[test]
    fn quarter_sweep_end_point_at_90_degrees() {
        let poly = arc_polygon(Rect::xywh(0.0, 0.0, 100.0, 100.0), 0.0, 90.0, 0.0);
        let last = poly[poly.len() - 1];
        assert!((last.x - 50.0).abs() < 0.01);
        assert!((last.y - 100.0).abs() < 0.01);
    }
}

mod text_tests {
    use crate::layout_scene::{NodeKind, SceneNode, SceneTextAlign, SceneTextVerticalAlign};
    use crate::widgets::canvas_viewport_paint::{paint_node, paint_svg_path_node};
    use crate::widgets::canvas_viewport_text::paint_text_node;
    use crate::widgets::PaintCx;
    use crate::{Color, ImageDrawMode, Point2D, Rect, RenderBackend, TextLayout};

    #[derive(Default)]
    struct TextCaptureBackend {
        origins: Vec<Point2D>,
        families: Vec<String>,
        font_sizes: Vec<f32>,
        lines: Vec<String>,
        translates: Vec<Point2D>,
        scales: Vec<(Point2D, Point2D)>,
        fill_rects: Vec<Rect>,
        round_rects: Vec<Rect>,
    }

    impl RenderBackend for TextCaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, rect: Rect, _: Color) {
            self.fill_rects.push(rect);
        }
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, layout: &TextLayout, origin: Point2D) {
            self.origins.push(origin);
            if let Some(run) = layout.runs().first() {
                self.families.push(run.font_family.clone());
                self.font_sizes.push(run.font_size);
                self.lines.push(run.content.clone());
            }
        }
        fn clip_rect(&mut self, _: Rect) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, offset: Point2D) {
            self.translates.push(offset);
        }
        fn scale(&mut self, scale: Point2D, pivot: Point2D) {
            self.scales.push((scale, pivot));
        }
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, rect: Rect, _: f32, _: Color) {
            self.round_rects.push(rect);
        }
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn draw_image(&mut self, _: Rect, _: u64, _: &[u8]) {}
        fn draw_image_with_mode(&mut self, _: Rect, _: u64, _: &[u8], _: ImageDrawMode) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
        fn measure_text_weighted(&mut self, text: &str, font_size: f32, _: u16) -> f32 {
            if text.is_ascii() {
                text.chars().count() as f32 * font_size * 0.5
            } else {
                text.chars().count() as f32 * font_size * 0.7 + font_size * font_size * 0.07
            }
        }
    }

    #[test]
    fn text_node_paint_honors_horizontal_alignment_and_ts_top_baseline() {
        let mut node = SceneNode::leaf("t", NodeKind::Text);
        node.bounds = Rect::xywh(0.0, 0.0, 200.0, 80.0);
        node.text = Some("Hi".to_string());
        node.font_family = "Georgia".to_string();
        node.font_size = 20.0;
        node.line_height = 1.0;
        node.text_align = SceneTextAlign::Center;
        node.text_vertical_align = SceneTextVerticalAlign::Middle;
        let mut backend = TextCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        paint_text_node(&mut cx, &node, node.bounds, 1.0, &None);

        assert_eq!(backend.families, vec!["Georgia".to_string()]);
        let origin = backend.origins[0];
        assert!(
            origin.x > 80.0,
            "center-aligned text should move away from the left edge"
        );
        assert_eq!(
            origin.y, 20.0,
            "canvas text follows TS paint parity: authored y is the top edge even when textAlignVertical=middle"
        );
    }

    #[test]
    fn text_wrap_is_stable_across_canvas_zoom() {
        let mut node = SceneNode::leaf("t", NodeKind::Text);
        node.bounds = Rect::xywh(0.0, 0.0, 100.0, 40.0);
        node.text = Some("可忽略风险".to_string());
        node.font_size = 20.0;
        node.text_wrap = true;

        let mut backend_1x = TextCaptureBackend::default();
        let mut cx_1x = PaintCx {
            backend: &mut backend_1x,
        };
        paint_node(
            &mut cx_1x,
            &node,
            Point2D::ZERO,
            1.0,
            None,
            Rect::xywh(0.0, 0.0, 800.0, 600.0),
        );

        let mut backend_2x = TextCaptureBackend::default();
        let mut cx_2x = PaintCx {
            backend: &mut backend_2x,
        };
        paint_node(
            &mut cx_2x,
            &node,
            Point2D::ZERO,
            2.0,
            None,
            Rect::xywh(0.0, 0.0, 800.0, 600.0),
        );

        assert_eq!(
            backend_2x.lines, backend_1x.lines,
            "canvas zoom must not change authored text wrapping"
        );
    }

    #[test]
    fn text_node_uses_viewport_transform_instead_of_zoomed_font_size() {
        let mut node = SceneNode::leaf("t", NodeKind::Text);
        node.bounds = Rect::xywh(12.0, 24.0, 100.0, 40.0);
        node.text = Some("Zoom".to_string());
        node.font_size = 20.0;

        let viewport_origin = Point2D::new(80.0, 40.0);
        let mut backend = TextCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        paint_node(
            &mut cx,
            &node,
            viewport_origin,
            2.0,
            None,
            Rect::xywh(0.0, 0.0, 800.0, 600.0),
        );

        assert_eq!(
            backend.font_sizes,
            vec![20.0],
            "canvas zoom should be a transform; text layout keeps the authored font size"
        );
        assert_eq!(backend.translates, vec![viewport_origin]);
        assert_eq!(
            backend.scales,
            vec![(Point2D::new(2.0, 2.0), Point2D::ZERO)]
        );
    }

    fn edit_caret(
        caret: Option<usize>,
        anchor: Option<usize>,
    ) -> crate::widgets::canvas_viewport::EditCaret {
        crate::widgets::canvas_viewport::EditCaret {
            editing: "t".to_string(),
            anchor_ms: 0,
            now_ms: 0, // blink phase 0 → caret visible
            select_all: false,
            caret,
            anchor,
            selection_color: Color::BLUE,
        }
    }

    #[test]
    fn edit_caret_paints_at_caret_offset() {
        let mut node = SceneNode::leaf("t", NodeKind::Text);
        node.bounds = Rect::xywh(0.0, 0.0, 200.0, 80.0);
        node.text = Some("hello\nworld".to_string());
        node.font_size = 20.0;
        node.line_height = 1.0;
        let mut backend = TextCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        // Caret at byte 8 — line 1, col 2 (10 px/char capture metric).
        paint_text_node(
            &mut cx,
            &node,
            node.bounds,
            1.0,
            &Some(edit_caret(Some(8), None)),
        );

        assert_eq!(
            backend.fill_rects,
            vec![Rect::xywh(20.0, 22.0, 1.0, 23.0)],
            "caret paints at the second line's col-2 advance, not at the text end"
        );
        assert!(backend.round_rects.is_empty(), "no selection → no wash");
    }

    #[test]
    fn edit_selection_paints_per_line_rects_and_hides_caret() {
        let mut node = SceneNode::leaf("t", NodeKind::Text);
        node.bounds = Rect::xywh(0.0, 0.0, 200.0, 80.0);
        node.text = Some("hello\nworld".to_string());
        node.font_size = 20.0;
        node.line_height = 1.0;
        let mut backend = TextCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        // anchor 3 .. caret 8 spans the line break.
        paint_text_node(
            &mut cx,
            &node,
            node.bounds,
            1.0,
            &Some(edit_caret(Some(8), Some(3))),
        );

        assert_eq!(
            backend.round_rects,
            vec![
                Rect::xywh(30.0, 0.0, 20.0, 24.0),
                Rect::xywh(0.0, 20.0, 20.0, 24.0),
            ],
            "one wash per intersected line"
        );
        assert!(
            backend.fill_rects.is_empty(),
            "caret hides while a selection is active"
        );
    }

    #[derive(Default)]
    struct SvgCaptureBackend {
        fill_rects: Vec<Rect>,
    }

    impl RenderBackend for SvgCaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn fill_svg_path_in_rect(&mut self, _: &str, rect: Rect, _: Color) {
            self.fill_rects.push(rect);
        }
        fn draw_image(&mut self, _: Rect, _: u64, _: &[u8]) {}
        fn draw_image_with_mode(&mut self, _: Rect, _: u64, _: &[u8], _: ImageDrawMode) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    #[test]
    fn svg_path_node_paint_fits_path_to_node_rect() {
        let mut node = SceneNode::leaf("p", NodeKind::Path);
        node.fill = Some(Color::BLACK);
        let rect = Rect::xywh(10.0, 20.0, 28.0, 28.0);
        let mut backend = SvgCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        paint_svg_path_node(&mut cx, &node, rect, 1.0, "M10 0 L0 -5 L0 5 Z");

        assert_eq!(backend.fill_rects, vec![rect]);
    }

    #[derive(Default)]
    struct GradientPathCaptureBackend {
        solid_fills: Vec<Rect>,
        linear_gradients: Vec<(Rect, f32, usize)>,
        radial_gradients: Vec<(Rect, usize)>,
        inner_shadows: Vec<(Rect, Color)>,
    }

    impl RenderBackend for GradientPathCaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn fill_svg_path_in_rect(&mut self, _: &str, rect: Rect, _: Color) {
            self.solid_fills.push(rect);
        }
        fn fill_svg_path_in_rect_linear_gradient(
            &mut self,
            _: &str,
            rect: Rect,
            stops: &[(f32, Color)],
            angle_deg: f32,
            _: f32,
        ) {
            self.linear_gradients.push((rect, angle_deg, stops.len()));
        }
        fn fill_svg_path_in_rect_radial_gradient(
            &mut self,
            _: &str,
            rect: Rect,
            stops: &[(f32, Color)],
            _: f32,
            _: f32,
            _: f32,
            _: f32,
        ) {
            self.radial_gradients.push((rect, stops.len()));
        }
        fn fill_inner_shadow_svg_path(
            &mut self,
            _: &str,
            rect: Rect,
            _: f32,
            _: f32,
            _: f32,
            color: Color,
        ) {
            self.inner_shadows.push((rect, color));
        }
        fn draw_image(&mut self, _: Rect, _: u64, _: &[u8]) {}
        fn draw_image_with_mode(&mut self, _: Rect, _: u64, _: &[u8], _: ImageDrawMode) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    #[test]
    fn svg_path_node_with_linear_gradient_paints_gradient_not_solid() {
        use crate::layout_scene::{SceneFillType, SceneGradient, SceneGradientStop};
        let mut node = SceneNode::leaf("p", NodeKind::Path);
        node.fill = Some(Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        });
        node.fill_type = SceneFillType::LinearGradient;
        node.gradient = Some(SceneGradient::Linear {
            angle_deg: 90.0,
            opacity: 1.0,
            stops: vec![
                SceneGradientStop {
                    offset: 0.0,
                    color: Color {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 1.0,
                    },
                },
                SceneGradientStop {
                    offset: 1.0,
                    color: Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    },
                },
            ],
        });
        let rect = Rect::xywh(0.0, 0.0, 10.0, 10.0);
        let mut backend = GradientPathCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        paint_svg_path_node(&mut cx, &node, rect, 1.0, "M0 0 L10 0 L10 10 Z");

        assert_eq!(
            backend.linear_gradients,
            vec![(rect, 90.0, 2)],
            "linear-gradient path must paint via the gradient method"
        );
        assert!(
            backend.solid_fills.is_empty(),
            "gradient path must not fall back to the solid fill"
        );
    }

    #[test]
    fn svg_path_node_with_inner_shadow_paints_inset_shadow() {
        use crate::layout_scene::{DropShadow, Effect};
        let mut node = SceneNode::leaf("p", NodeKind::Path);
        node.fill = Some(Color {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        let shadow_color = Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.5,
        };
        node.effects = vec![Effect::DropShadow(DropShadow {
            offset_x: 0.0,
            offset_y: 0.0,
            blur: 4.0,
            color: shadow_color,
            inner: true,
        })];
        let rect = Rect::xywh(0.0, 0.0, 20.0, 20.0);
        let mut backend = GradientPathCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        paint_svg_path_node(&mut cx, &node, rect, 1.0, "M0 0 L20 0 L20 20 L0 20 Z");

        assert_eq!(
            backend.inner_shadows,
            vec![(rect, shadow_color)],
            "inner-shadow path must route to the inset-shadow painter"
        );
    }
}

mod path_tests {
    use crate::layout_scene::{NodeKind, SceneAnchor, SceneNode, ScenePointType};
    use crate::widgets::canvas_viewport_paint::flatten_path;
    use crate::{Point2D, Rect};

    fn anchor(x: f32, y: f32, hout: Option<Point2D>) -> SceneAnchor {
        SceneAnchor {
            pos: Point2D::new(x, y),
            handle_in: None,
            handle_out: hout,
            point_type: ScenePointType::Corner,
        }
    }

    #[test]
    fn handle_free_path_falls_back_to_points() {
        let mut n = SceneNode::leaf("p", NodeKind::Path);
        n.points = vec![Point2D::new(0.0, 0.0), Point2D::new(10.0, 0.0)];
        n.path_anchors = vec![anchor(0.0, 0.0, None), anchor(10.0, 0.0, None)];
        assert_eq!(flatten_path(&n), n.points);
    }

    #[test]
    fn curved_segment_tessellates_into_many_points() {
        let mut n = SceneNode::leaf("p", NodeKind::Path);
        n.points = vec![Point2D::new(0.0, 0.0), Point2D::new(100.0, 0.0)];
        n.path_anchors = vec![
            anchor(0.0, 0.0, Some(Point2D::new(0.0, 50.0))),
            anchor(100.0, 0.0, None),
        ];
        let poly = flatten_path(&n);
        assert_eq!(poly.len(), 17);
        assert_eq!(poly[0], Point2D::new(0.0, 0.0));
        assert_eq!(poly[poly.len() - 1], Point2D::new(100.0, 0.0));
        assert!(poly[8].y > 1.0, "curve bows toward the handle");
    }

    #[test]
    fn bounds_kept_so_helper_is_pure() {
        let mut n = SceneNode::leaf("p", NodeKind::Path);
        n.bounds = Rect::xywh(1.0, 2.0, 3.0, 4.0);
        let _ = flatten_path(&n);
        assert_eq!(n.bounds, Rect::xywh(1.0, 2.0, 3.0, 4.0));
    }
}

mod clip_tests {
    use crate::layout_scene::{NodeKind, SceneNode};
    use crate::widgets::canvas_viewport_paint::paint_node;
    use crate::widgets::PaintCx;
    use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};

    /// Records the paint-op sequence so the test can assert the clip
    /// brackets the children (and only the children).
    #[derive(Default)]
    struct ClipCaptureBackend {
        ops: Vec<String>,
    }

    impl RenderBackend for ClipCaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, rect: Rect, _: Color) {
            self.ops
                .push(format!("fill({},{})", rect.origin.x, rect.origin.y));
        }
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, rect: Rect) {
            self.ops.push(format!(
                "clip({},{},{},{})",
                rect.origin.x, rect.origin.y, rect.size.x, rect.size.y
            ));
        }
        fn clip_round_rect(&mut self, rect: Rect, radius: f32) {
            self.ops.push(format!(
                "clip_rr({},{},{},{},r={radius})",
                rect.origin.x, rect.origin.y, rect.size.x, rect.size.y
            ));
        }
        fn save(&mut self) {
            self.ops.push("save".into());
        }
        fn restore(&mut self) {
            self.ops.push("restore".into());
        }
        fn translate(&mut self, _: Point2D) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    fn frame_with_child(clip: bool, corner_radius: f32) -> SceneNode {
        let mut child = SceneNode::leaf("c", NodeKind::Rect);
        child.bounds = Rect::xywh(10.0, 10.0, 500.0, 20.0);
        child.fill = Some(Color::RED);
        let mut frame = SceneNode::leaf("f", NodeKind::Frame);
        frame.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
        frame.fill = Some(Color::WHITE);
        frame.clip_content = clip;
        frame.corner_radius = corner_radius;
        frame.children = vec![child];
        frame
    }

    fn paint(node: &SceneNode) -> Vec<String> {
        let mut backend = ClipCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        paint_node(
            &mut cx,
            node,
            Point2D::ZERO,
            1.0,
            None,
            Rect::xywh(0.0, 0.0, 4000.0, 4000.0),
        );
        backend.ops
    }

    #[test]
    fn clip_content_frame_brackets_children_with_sharp_clip() {
        let ops = paint(&frame_with_child(true, 0.0));
        // Own fill paints UN-clipped, then save → clip → child → restore.
        assert_eq!(
            ops,
            vec![
                "fill(0,0)".to_string(),
                "save".to_string(),
                "clip(0,0,100,100)".to_string(),
                "fill(10,10)".to_string(),
                "restore".to_string(),
            ]
        );
    }

    #[test]
    fn clip_content_uses_rounded_clip_clamped_to_half_height() {
        // Authored radius 80 clamps to h/2 = 50 (TS flattener rule).
        let ops = paint(&frame_with_child(true, 80.0));
        assert!(
            ops.contains(&"clip_rr(0,0,100,100,r=50)".to_string()),
            "{ops:?}"
        );
    }

    #[test]
    fn frame_without_clip_content_paints_children_unclipped() {
        let ops = paint(&frame_with_child(false, 0.0));
        assert_eq!(
            ops,
            vec!["fill(0,0)".to_string(), "fill(10,10)".to_string()]
        );
    }

    #[test]
    fn clip_content_group_clips_children_too() {
        let mut group = frame_with_child(true, 0.0);
        group.kind = NodeKind::Group;
        group.fill = None;
        let ops = paint(&group);
        assert_eq!(
            ops,
            vec![
                "save".to_string(),
                "clip(0,0,100,100)".to_string(),
                "fill(10,10)".to_string(),
                "restore".to_string(),
            ]
        );
    }
}
