//! Sibling test file for `canvas_agent_cursor.rs` (800-line cap
//! convention) — kinematics, sprite building, paint.

mod kinematics_tests {
    use crate::widgets::canvas_agent_cursor::{cursor_kinematics, ease_out_cubic, Waypoint};
    use crate::{Point2D, Rect};

    fn wp(start_ms: u64, x: f32, y: f32) -> Waypoint {
        Waypoint {
            start_ms,
            pos: Point2D::new(x, y),
            rect: Rect::xywh(x - 5.0, y - 5.0, 10.0, 10.0),
        }
    }

    #[test]
    fn empty_queue_yields_no_cursor() {
        assert!(cursor_kinematics(&[], 1_000).is_none());
    }

    #[test]
    fn cursor_arrives_exactly_at_reveal_start() {
        let wps = [wp(1_000, 100.0, 50.0), wp(1_400, 300.0, 90.0)];
        let kin = cursor_kinematics(&wps, 1_400).unwrap();
        assert!((kin.pos.x - 300.0).abs() < 0.01 && (kin.pos.y - 90.0).abs() < 0.01);
        assert_eq!(kin.current, Some(1));
    }

    #[test]
    fn cursor_flies_between_waypoints_while_current_stays_on_departed() {
        let wps = [wp(1_000, 100.0, 50.0), wp(1_300, 300.0, 50.0)];
        let kin = cursor_kinematics(&wps, 1_150).unwrap();
        assert!(kin.pos.x > 100.0 && kin.pos.x < 300.0);
        assert_eq!(
            kin.current,
            Some(0),
            "mid-flight the previous placement is still the current element"
        );
    }

    #[test]
    fn long_gaps_depart_late_and_arrive_on_time() {
        let wps = [wp(1_000, 100.0, 50.0), wp(5_000, 300.0, 50.0)];
        let hold = cursor_kinematics(&wps, 1_500).unwrap();
        assert!(
            (hold.pos.x - 100.0).abs() < 0.01,
            "cursor waits at the previous node until depart"
        );
        assert!((hold.alpha - 1.0).abs() < 0.01, "parked cursor never fades");
        let arrive = cursor_kinematics(&wps, 5_000).unwrap();
        assert!((arrive.pos.x - 300.0).abs() < 0.01);
    }

    #[test]
    fn entry_fades_in_toward_first_waypoint() {
        let wps = [wp(2_000, 100.0, 50.0)];
        assert!(
            cursor_kinematics(&wps, 1_700).is_none(),
            "hidden before the entry window"
        );
        let kin = cursor_kinematics(&wps, 1_875).unwrap();
        assert!(kin.alpha > 0.0 && kin.alpha < 1.0);
        assert!(
            kin.pos.x < 100.0 && kin.pos.y < 50.0,
            "slides in from the entry offset"
        );
        assert!(kin.current.is_none(), "no current element during entry");
    }

    #[test]
    fn parked_cursor_persists_after_queue_exhausts() {
        let wps = [wp(1_000, 100.0, 50.0)];
        for probe in [1_500u64, 2_500, 30_000] {
            let kin = cursor_kinematics(&wps, probe).unwrap();
            assert!(
                (kin.alpha - 1.0).abs() < 0.01,
                "cursor must not fade while the run is alive (probe {probe})"
            );
            assert!((kin.pos.x - 100.0).abs() < 0.01);
            assert_eq!(kin.current, Some(0));
        }
    }

    #[test]
    fn equal_start_waypoints_coalesce_on_the_last_placement() {
        let wps = [
            wp(1_000, 100.0, 50.0),
            wp(1_000, 200.0, 80.0),
            wp(1_400, 400.0, 90.0),
        ];
        let at_shared_start = cursor_kinematics(&wps, 1_000).unwrap();
        assert!(
            (at_shared_start.pos.x - 200.0).abs() < 0.01,
            "cursor parks on the last equal-start placement in sort order"
        );
        let mid_flight = cursor_kinematics(&wps, 1_200).unwrap();
        assert!(
            mid_flight.pos.x > 200.0,
            "flight to the next waypoint departs from the coalesced placement"
        );
    }

    #[test]
    fn ease_out_cubic_hits_endpoints() {
        assert!(ease_out_cubic(0.0).abs() < 1e-6);
        assert!((ease_out_cubic(1.0) - 1.0).abs() < 1e-6);
        assert!(ease_out_cubic(0.5) > 0.5, "ease-out front-loads the motion");
    }
}

mod hex_tests {
    use crate::widgets::canvas_agent_cursor::parse_hex;

    #[test]
    fn parses_agent_palette_hex() {
        let c = parse_hex("#FF6B6B").unwrap();
        assert!((c.r - 1.0).abs() < 1e-6);
        assert!((c.g - 0.419).abs() < 0.01);
        assert!((c.b - 0.419).abs() < 0.01);
    }

    #[test]
    fn rejects_short_or_non_ascii_hex() {
        assert!(parse_hex("#FFF").is_none());
        assert!(parse_hex("#非ascii").is_none());
    }
}

mod sprite_tests {
    use crate::layout_scene::{NodeKind, SceneNode};
    use crate::widgets::canvas_agent_cursor::cursor_sprites;
    use crate::{Point2D, Rect};
    use op_editor_core::agent_indicators::{AgentIndicators, AgentTag};

    fn frame_with_child(frame_id: &str, child_id: &str, x: f32) -> SceneNode {
        let mut child = SceneNode::leaf(child_id, NodeKind::Rect);
        child.bounds = Rect::xywh(x + 10.0, 30.0, 60.0, 60.0);
        let mut frame = SceneNode::leaf(frame_id, NodeKind::Frame);
        frame.bounds = Rect::xywh(x, 20.0, 200.0, 300.0);
        frame.children = vec![child];
        frame
    }

    fn tag(color: &str, name: &str) -> AgentTag {
        AgentTag {
            color: color.to_string(),
            name: name.to_string(),
        }
    }

    fn frame_with_children(frame_id: &str, child_ids: &[&str]) -> SceneNode {
        let mut frame = SceneNode::leaf(frame_id, NodeKind::Frame);
        frame.bounds = Rect::xywh(10.0, 20.0, 240.0, 180.0);
        frame.children = child_ids
            .iter()
            .enumerate()
            .map(|(idx, id)| {
                let mut child = SceneNode::leaf(*id, NodeKind::Rect);
                child.bounds = Rect::xywh(30.0 + idx as f32 * 52.0, 48.0, 44.0, 44.0);
                child
            })
            .collect();
        frame
    }

    fn root_with_leaf_children(child_ids: &[&str], size: f32) -> SceneNode {
        let mut root = SceneNode::leaf("root", NodeKind::Frame);
        root.bounds = Rect::xywh(0.0, 0.0, 640.0, 160.0);
        root.children = child_ids
            .iter()
            .enumerate()
            .map(|(idx, id)| {
                let mut child = SceneNode::leaf(*id, NodeKind::Rect);
                child.bounds.origin = Point2D::new(20.0 + idx as f32 * 90.0, 40.0);
                child.bounds.size = Point2D::new(size, size);
                child
            })
            .collect();
        root
    }

    #[test]
    fn reveal_under_tagged_frame_inherits_agent_colour_and_name() {
        let roots = vec![frame_with_child("f1", "c1", 0.0)];
        let mut ind = AgentIndicators::default();
        ind.frames.insert("f1".into(), tag("#4ECDC4", "Mochi"));
        ind.reveals.insert("c1".into(), 1_000);
        let sprites = cursor_sprites(&roots, &ind, Point2D::new(0.0, 0.0), 1.0, 1_050);
        assert_eq!(sprites.len(), 1);
        let s = &sprites[0];
        assert_eq!(s.name.as_deref(), Some("Mochi"));
        assert!((s.color.g - 0.804).abs() < 0.01, "#4ECDC4 green channel");
        // c1 centre = (40, 60) at zoom 1, origin (0, 0).
        assert!((s.pos.x - 40.0).abs() < 0.01 && (s.pos.y - 60.0).abs() < 0.01);
        let rect = s
            .current_rect
            .expect("started reveal is the current element");
        assert!(
            (rect.origin.x - 10.0).abs() < 0.01
                && (rect.origin.y - 30.0).abs() < 0.01
                && (rect.size.x - 60.0).abs() < 0.01,
            "breathing border targets the current element's screen rect"
        );
    }

    #[test]
    fn viewport_transform_folds_into_cursor_position() {
        let roots = vec![frame_with_child("f1", "c1", 0.0)];
        let mut ind = AgentIndicators::default();
        ind.reveals.insert("c1".into(), 1_000);
        let sprites = cursor_sprites(&roots, &ind, Point2D::new(100.0, 50.0), 2.0, 1_000);
        assert_eq!(sprites.len(), 1);
        assert!((sprites[0].pos.x - 180.0).abs() < 0.01);
        assert!((sprites[0].pos.y - 170.0).abs() < 0.01);
        assert!(
            sprites[0].name.is_none(),
            "untagged reveals get the fallback cursor without a pill"
        );
        assert!(
            (sprites[0].color.r - 1.0).abs() < 0.01 && (sprites[0].color.g - 0.419).abs() < 0.01,
            "untagged reveals use the fallback red"
        );
        let rect = sprites[0].current_rect.expect("current element rect");
        assert!(
            (rect.origin.x - 120.0).abs() < 0.01
                && (rect.origin.y - 110.0).abs() < 0.01
                && (rect.size.x - 120.0).abs() < 0.01,
            "border rect folds in pan + zoom"
        );
    }

    #[test]
    fn two_agents_get_two_cursors() {
        let roots = vec![
            frame_with_child("f1", "c1", 0.0),
            frame_with_child("f2", "c2", 1_000.0),
        ];
        let mut ind = AgentIndicators::default();
        ind.frames.insert("f1".into(), tag("#4ECDC4", "Mochi"));
        ind.frames.insert("f2".into(), tag("#FF6B6B", "Nova"));
        ind.reveals.insert("c1".into(), 1_000);
        ind.reveals.insert("c2".into(), 1_020);
        let mut sprites = cursor_sprites(&roots, &ind, Point2D::new(0.0, 0.0), 1.0, 1_060);
        sprites.sort_by(|a, b| a.pos.x.total_cmp(&b.pos.x));
        assert_eq!(sprites.len(), 2);
        assert_eq!(sprites[0].name.as_deref(), Some("Mochi"));
        assert_eq!(sprites[1].name.as_deref(), Some("Nova"));
        assert!(
            sprites[1].pos.x > 900.0,
            "each cursor stays inside its own agent's frame"
        );
    }

    #[test]
    fn revealed_parent_suppresses_child_waypoints_in_same_window() {
        let roots = vec![frame_with_children("section", &["a", "b", "c"])];
        let mut ind = AgentIndicators::default();
        ind.frames.insert("section".into(), tag("#4ECDC4", "Mochi"));
        ind.reveals.insert("section".into(), 1_000);
        ind.reveals.insert("a".into(), 1_010);
        ind.reveals.insert("b".into(), 1_020);
        ind.reveals.insert("c".into(), 1_030);

        let sprites = cursor_sprites(&roots, &ind, Point2D::ZERO, 1.0, 1_030);

        assert_eq!(sprites.len(), 1);
        let rect = sprites[0].current_rect.expect("parent waypoint is current");
        assert!(
            (rect.origin.x - 10.0).abs() < 0.01
                && (rect.origin.y - 20.0).abs() < 0.01
                && (rect.size.x - 240.0).abs() < 0.01,
            "cursor should park on the revealed section while children pop in"
        );
    }

    #[test]
    fn dense_leaf_reveals_coalesce_to_last_dwell_waypoint() {
        let ids = ["a", "b", "c", "d", "e"];
        let roots = vec![root_with_leaf_children(&ids, 60.0)];
        let mut ind = AgentIndicators::default();
        for (idx, id) in ids.iter().enumerate() {
            ind.reveals.insert((*id).into(), 1_000 + idx as u64 * 50);
        }

        let sprites = cursor_sprites(&roots, &ind, Point2D::ZERO, 1.0, 1_000);

        assert_eq!(sprites.len(), 1);
        assert!(
            sprites[0].current_rect.is_none(),
            "coalesced dwell window should still be entering toward the last waypoint"
        );
        assert!(
            sprites[0].pos.x > 350.0,
            "cursor should aim at the last dense waypoint, not the first"
        );
    }

    #[test]
    fn standalone_large_reveal_keeps_waypoint_but_tiny_reveal_does_not() {
        let large_roots = vec![root_with_leaf_children(&["card"], 60.0)];
        let mut large = AgentIndicators::default();
        large.reveals.insert("card".into(), 1_000);
        let large_sprites = cursor_sprites(&large_roots, &large, Point2D::ZERO, 1.0, 1_000);
        assert_eq!(large_sprites.len(), 1);
        assert!(large_sprites[0].current_rect.is_some());

        let tiny_roots = vec![root_with_leaf_children(&["dot"], 12.0)];
        let mut tiny = AgentIndicators::default();
        tiny.reveals.insert("dot".into(), 1_000);
        let tiny_sprites = cursor_sprites(&tiny_roots, &tiny, Point2D::ZERO, 1.0, 1_000);
        assert!(
            tiny_sprites.is_empty(),
            "standalone tiny reveals animate normally but do not get cursor waypoints"
        );
    }

    #[test]
    fn current_rect_tracks_latest_started_element() {
        let mut frame = frame_with_child("f1", "c1", 0.0);
        let mut second = SceneNode::leaf("c2", NodeKind::Rect);
        second.bounds = Rect::xywh(110.0, 30.0, 60.0, 60.0);
        frame.children.push(second);
        let roots = vec![frame];
        let mut ind = AgentIndicators::default();
        ind.reveals.insert("c1".into(), 1_000);
        ind.reveals.insert("c2".into(), 1_300);
        let flying = cursor_sprites(&roots, &ind, Point2D::new(0.0, 0.0), 1.0, 1_250);
        assert_eq!(flying.len(), 1, "same agent keeps a single cursor");
        let rect = flying[0].current_rect.expect("current rect mid-flight");
        assert!(
            (rect.origin.x - 10.0).abs() < 0.01,
            "border stays on the departed element until the next one starts"
        );
        let arrived = cursor_sprites(&roots, &ind, Point2D::new(0.0, 0.0), 1.0, 1_300);
        let rect = arrived[0].current_rect.expect("current rect at arrival");
        assert!(
            (rect.origin.x - 110.0).abs() < 0.01,
            "border hands off to the newly started element at its reveal instant"
        );
    }
}

mod paint_tests {
    use crate::layout_scene::{NodeKind, SceneNode};
    use crate::widgets::canvas_agent_cursor::paint_agent_cursors;
    use crate::widgets::PaintCx;
    use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
    use op_editor_core::agent_indicators::{AgentIndicators, AgentTag};

    #[derive(Default)]
    struct CursorCaptureBackend {
        polygons: Vec<(Vec<Point2D>, Color)>,
        polygon_strokes: Vec<(Vec<Point2D>, Color)>,
        round_fills: Vec<(Rect, Color)>,
        round_strokes: Vec<(Rect, Color, f32)>,
        labels: Vec<String>,
    }

    impl RenderBackend for CursorCaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, layout: &TextLayout, _: Point2D) {
            if let Some(run) = layout.runs().first() {
                self.labels.push(run.content.clone());
            }
        }
        fn clip_rect(&mut self, _: Rect) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, rect: Rect, _: f32, color: Color) {
            self.round_fills.push((rect, color));
        }
        fn stroke_round_rect(&mut self, rect: Rect, _: f32, color: Color, width: f32) {
            self.round_strokes.push((rect, color, width));
        }
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
        fn fill_polygon(&mut self, points: &[Point2D], color: Color) {
            self.polygons.push((points.to_vec(), color));
        }
        fn stroke_polygon(&mut self, points: &[Point2D], color: Color, _: f32) {
            self.polygon_strokes.push((points.to_vec(), color));
        }
    }

    fn scene() -> Vec<SceneNode> {
        let mut child = SceneNode::leaf("c1", NodeKind::Rect);
        child.bounds = Rect::xywh(10.0, 30.0, 60.0, 60.0);
        let mut frame = SceneNode::leaf("f1", NodeKind::Frame);
        frame.bounds = Rect::xywh(0.0, 20.0, 200.0, 300.0);
        frame.children = vec![child];
        vec![frame]
    }

    #[test]
    fn paints_arrow_border_and_name_pill_for_tagged_agent() {
        let roots = scene();
        let mut ind = AgentIndicators::default();
        ind.frames.insert(
            "f1".into(),
            AgentTag {
                color: "#4ECDC4".into(),
                name: "Mochi".into(),
            },
        );
        ind.reveals.insert("c1".into(), 1_000);
        let mut backend = CursorCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        paint_agent_cursors(&mut cx, &roots, Point2D::new(0.0, 0.0), 1.0, 1_050, &ind);
        assert_eq!(
            backend.polygons.len(),
            1,
            "arrow body is one filled polygon"
        );
        let (pts, color) = &backend.polygons[0];
        assert_eq!(pts.len(), 7, "arrow pointer has 7 vertices");
        assert!((color.g - 0.804).abs() < 0.01);
        assert!(
            (pts[0].x - 40.0).abs() < 0.01 && (pts[0].y - 60.0).abs() < 0.01,
            "arrow tip sits on the current element's centre"
        );
        assert!(
            !backend.polygon_strokes.is_empty(),
            "white outline strokes the arrow"
        );
        assert_eq!(
            backend.round_strokes.len(),
            2,
            "breathing border paints wash + crisp ring on the current element"
        );
        let (rect, border_color, _) = &backend.round_strokes[0];
        assert!(
            (rect.origin.x - 10.0).abs() < 0.01 && (rect.origin.y - 30.0).abs() < 0.01,
            "border wraps the current element"
        );
        assert!(
            (border_color.g - 0.804).abs() < 0.01,
            "border uses agent colour"
        );
        assert_eq!(backend.labels, vec!["Mochi".to_string()]);
        assert!(!backend.round_fills.is_empty(), "name pill paints");
    }

    #[test]
    fn untagged_cursor_paints_arrow_without_pill() {
        let roots = scene();
        let mut ind = AgentIndicators::default();
        ind.reveals.insert("c1".into(), 1_000);
        let mut backend = CursorCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        paint_agent_cursors(&mut cx, &roots, Point2D::ZERO, 1.0, 1_050, &ind);
        assert_eq!(backend.polygons.len(), 1, "fallback arrow still paints");
        let (_, color) = &backend.polygons[0];
        assert!(
            (color.r - 1.0).abs() < 0.01 && (color.g - 0.419).abs() < 0.01,
            "fallback red fill"
        );
        assert!(backend.labels.is_empty(), "no name pill without a tag");
        assert!(backend.round_fills.is_empty(), "no pill capsule either");
        assert!(
            !backend.round_strokes.is_empty(),
            "breathing border still marks the current element"
        );
    }

    #[test]
    fn no_reveals_paints_nothing() {
        let roots = scene();
        let ind = AgentIndicators::default();
        let mut backend = CursorCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        paint_agent_cursors(&mut cx, &roots, Point2D::ZERO, 1.0, 1_000, &ind);
        assert!(
            backend.polygons.is_empty()
                && backend.round_fills.is_empty()
                && backend.round_strokes.is_empty()
        );
    }
}
