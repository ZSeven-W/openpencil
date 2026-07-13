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
    fn section_revealed_long_after_root_gets_its_own_waypoint() {
        // Regression: the dwell comparison was reversed, so the page root
        // (revealed first at scaffold time) suppressed EVERY later section
        // and the cursor sat on the root's center for the whole run.
        let mut root = frame_with_children("root", &["section"]);
        root.bounds = Rect::xywh(0.0, 0.0, 1440.0, 900.0);
        root.children[0].bounds = Rect::xywh(40.0, 600.0, 400.0, 200.0);
        let roots = vec![root];
        let mut ind = AgentIndicators::default();
        ind.reveals.insert("root".into(), 1_000);
        ind.reveals.insert("section".into(), 5_000);

        let sprites = cursor_sprites(&roots, &ind, Point2D::ZERO, 1.0, 5_000);

        assert_eq!(sprites.len(), 1);
        let rect = sprites[0]
            .current_rect
            .expect("late section waypoint is current");
        assert!(
            (rect.origin.x - 40.0).abs() < 0.01 && (rect.origin.y - 600.0).abs() < 0.01,
            "cursor must move to the late section, not stay parked on the root: {rect:?}"
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

    #[test]
    fn nested_reveal_targets_the_node_not_its_wrappers() {
        // root(600h) > AppContent wrapper(560h) > Header(80h) > revealing
        // element. The cursor follows the ELEMENT itself — never a wrapper
        // or root centre — and a tiny standalone leaf (below the waypoint
        // area floor) is skipped entirely instead of yanking the pointer.
        let mut element = SceneNode::leaf("element", NodeKind::Rect);
        element.bounds = Rect::xywh(10.0, 30.0, 120.0, 50.0);
        let mut header = SceneNode::leaf("header", NodeKind::Frame);
        header.bounds = Rect::xywh(0.0, 20.0, 200.0, 80.0);
        header.children = vec![element];
        let mut feed = SceneNode::leaf("feed", NodeKind::Frame);
        feed.bounds = Rect::xywh(0.0, 120.0, 200.0, 300.0);
        let mut wrapper = SceneNode::leaf("wrapper", NodeKind::Frame);
        wrapper.bounds = Rect::xywh(0.0, 20.0, 200.0, 560.0);
        wrapper.children = vec![header, feed];
        let mut root = SceneNode::leaf("root", NodeKind::Frame);
        root.bounds = Rect::xywh(0.0, 0.0, 200.0, 600.0);
        root.children = vec![wrapper];
        let roots = vec![root];

        let mut ind = AgentIndicators::default();
        ind.reveals.insert("element".into(), 1_000);
        let sprites = cursor_sprites(&roots, &ind, Point2D::ZERO, 1.0, 1_500);
        assert_eq!(sprites.len(), 1);
        let pos = sprites[0].pos;
        assert!(
            (pos.x - 70.0).abs() < 0.01 && (pos.y - 55.0).abs() < 0.01,
            "cursor sits on the element's centre (70,55) — got ({}, {})",
            pos.x,
            pos.y
        );

        // A tiny standalone leaf never becomes a waypoint.
        let mut tiny = AgentIndicators::default();
        tiny.reveals.insert("tiny".into(), 1_000);
        let mut tiny_leaf = SceneNode::leaf("tiny", NodeKind::Rect);
        tiny_leaf.bounds = Rect::xywh(0.0, 0.0, 20.0, 20.0);
        let tiny_roots = vec![tiny_leaf];
        assert!(
            cursor_sprites(&tiny_roots, &tiny, Point2D::ZERO, 1.0, 1_500).is_empty(),
            "tiny standalone leaves are not chased"
        );
    }

    #[test]
    fn cursor_follows_each_revealing_node() {
        // Root → section frame → two leaves revealing far apart. The cursor
        // follows the ELEMENT being output: it sits on leaf-a while that is
        // the latest reveal, then eases to leaf-b when its reveal starts.
        let mut leaf_a = SceneNode::leaf("leaf-a", NodeKind::Rect);
        leaf_a.bounds = Rect::xywh(10.0, 110.0, 80.0, 60.0);
        let mut leaf_b = SceneNode::leaf("leaf-b", NodeKind::Rect);
        leaf_b.bounds = Rect::xywh(100.0, 300.0, 80.0, 60.0);
        let mut section = SceneNode::leaf("section", NodeKind::Frame);
        section.bounds = Rect::xywh(0.0, 100.0, 200.0, 400.0);
        section.children = vec![leaf_a, leaf_b];
        let mut root = SceneNode::leaf("root", NodeKind::Frame);
        root.bounds = Rect::xywh(0.0, 0.0, 200.0, 600.0);
        root.children = vec![section];
        let roots = vec![root];

        let mut ind = AgentIndicators::default();
        ind.reveals.insert("leaf-a".into(), 1_000);
        ind.reveals.insert("leaf-b".into(), 3_000);

        // Parked on leaf-a's centre after its reveal, before leaf-b's.
        let sprites = cursor_sprites(&roots, &ind, Point2D::ZERO, 1.0, 2_000);
        assert_eq!(sprites.len(), 1);
        let pos = sprites[0].pos;
        assert!(
            (pos.x - 50.0).abs() < 0.01 && (pos.y - 140.0).abs() < 0.01,
            "cursor parks on leaf-a's centre, got ({}, {})",
            pos.x,
            pos.y
        );
        // After leaf-b's reveal the cursor has moved to ITS centre, and the
        // current-element rect (breathing border target) is leaf-b's rect.
        let sprites = cursor_sprites(&roots, &ind, Point2D::ZERO, 1.0, 5_000);
        let pos = sprites[0].pos;
        assert!(
            (pos.x - 140.0).abs() < 0.01 && (pos.y - 330.0).abs() < 0.01,
            "cursor follows leaf-b, got ({}, {})",
            pos.x,
            pos.y
        );
        let rect = sprites[0].current_rect.expect("current element rect");
        assert!(
            (rect.origin.x - 100.0).abs() < 0.01 && (rect.origin.y - 300.0).abs() < 0.01,
            "breathing border targets the revealing node"
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
        paint_agent_cursors(
            &mut cx,
            &roots,
            Point2D::new(0.0, 0.0),
            1.0,
            1_050,
            &ind,
            Default::default(),
        );
        assert_eq!(
            backend.polygons.len(),
            7,
            "4 halo feather layers + the white rim + body + tip wedge"
        );
        let (pts, color) = &backend.polygons[5];
        assert_eq!(
            pts.len(),
            19,
            "rounded pencil body is a densely sampled arc polygon"
        );
        assert!((color.g - 0.804).abs() < 0.01);
        assert!(
            (pts[0].x - 40.0).abs() < 0.01 && (pts[0].y - 60.0).abs() < 0.01,
            "pencil tip sits on the current element's centre"
        );
        // The rim is FILLED geometry, not a stroke: the trait's fallback
        // polygon stroke drew each edge as its own capped segment, which
        // notched every vertex of the densely-sampled arc (user report
        // 2026-07-12: "不要有锯齿感"). It must still be white, and sit
        // between the halo and the body.
        let (rim_pts, rim_color) = &backend.polygons[4];
        assert!(
            (rim_color.r - 1.0).abs() < 0.01
                && (rim_color.g - 1.0).abs() < 0.01
                && (rim_color.b - 1.0).abs() < 0.01,
            "the rim is white: {rim_color:?}"
        );
        assert_eq!(
            rim_pts.len(),
            pts.len(),
            "the rim is the body silhouette, outset — same vertex count"
        );
        assert!(
            backend.round_strokes.is_empty(),
            "the per-agent breathing border is retired — the generation \
             skeleton owns the working-area affordance now"
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
        paint_agent_cursors(
            &mut cx,
            &roots,
            Point2D::ZERO,
            1.0,
            1_050,
            &ind,
            Default::default(),
        );
        assert_eq!(backend.polygons.len(), 7, "fallback pencil still paints");
        let (_, color) = &backend.polygons[5];
        assert!(
            (color.r - 1.0).abs() < 0.01 && (color.g - 0.419).abs() < 0.01,
            "fallback red fill"
        );
        assert!(backend.labels.is_empty(), "no name pill without a tag");
        assert!(backend.round_fills.is_empty(), "no pill capsule either");
        assert!(
            backend.round_strokes.is_empty(),
            "no breathing border for untagged reveals either (retired)"
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
        paint_agent_cursors(
            &mut cx,
            &roots,
            Point2D::ZERO,
            1.0,
            1_000,
            &ind,
            Default::default(),
        );
        assert!(
            backend.polygons.is_empty()
                && backend.round_fills.is_empty()
                && backend.round_strokes.is_empty()
        );
    }
}

mod scan_gate_tests {
    use crate::layout_scene::{NodeKind, SceneNode};
    use crate::widgets::canvas_generation_scan::generating_paint_sets;
    use crate::Rect;
    use op_editor_core::agent_indicators::AgentIndicators;

    /// Pencil lights sections in WORK ORDER: while an earlier shell is
    /// still empty, later regions (however large) stay plain; once it is
    /// the first empty shell, even the dashboard's main column washes.
    #[test]
    fn dominant_empty_region_gets_no_scan_but_small_shells_do() {
        let mut sidebar = SceneNode::leaf("sidebar", NodeKind::Frame);
        sidebar.bounds = Rect::xywh(0.0, 0.0, 260.0, 900.0);
        let mut sidebar_child = SceneNode::leaf("nav", NodeKind::Frame);
        sidebar_child.bounds = Rect::xywh(0.0, 0.0, 260.0, 48.0);
        sidebar.children = vec![sidebar_child];
        let mut main = SceneNode::leaf("main", NodeKind::Frame);
        main.bounds = Rect::xywh(260.0, 0.0, 1180.0, 900.0);
        let mut header_shell = SceneNode::leaf("header-shell", NodeKind::Frame);
        header_shell.bounds = Rect::xywh(0.0, 0.0, 1440.0, 90.0);
        let mut root = SceneNode::leaf("root", NodeKind::Frame);
        root.bounds = Rect::xywh(0.0, 0.0, 1440.0, 900.0);
        root.children = vec![header_shell, sidebar, main];
        let roots = vec![root];

        let mut ind = AgentIndicators::default();
        ind.run_active = true;
        ind.frames.insert(
            "root".into(),
            op_editor_core::agent_indicators::AgentTag {
                color: "#FF6B6B".into(),
                name: "Kiki".into(),
            },
        );
        let sets = generating_paint_sets(&roots, Some(&ind)).expect("active run");
        let ids = &sets.scan;
        assert!(
            !ids.contains("main"),
            "the dominant empty region stays plain background"
        );
        assert!(
            ids.contains("header-shell"),
            "a small full-width shell keeps the scan"
        );
        assert!(
            ids.contains("sidebar"),
            "a filled column is unaffected by the gate"
        );

        // Once the earlier shells fill, the big main column is ON DECK and
        // must wash — it is now where work happens (user report: the right
        // side jumped from plain black to finished content with no scan).
        let mut filled_header = SceneNode::leaf("header-shell", NodeKind::Frame);
        filled_header.bounds = Rect::xywh(0.0, 0.0, 1440.0, 90.0);
        filled_header.children = vec![SceneNode::leaf("h-child", NodeKind::Text)];
        let mut sidebar2 = SceneNode::leaf("sidebar", NodeKind::Frame);
        sidebar2.bounds = Rect::xywh(0.0, 0.0, 260.0, 900.0);
        // Filled through: the deck is GLOBAL in document order, so a still-
        // empty shell nested in the sidebar would legitimately outrank the
        // main column (it comes first in the fill order).
        let mut nav2 = SceneNode::leaf("nav2", NodeKind::Frame);
        nav2.children = vec![SceneNode::leaf("nav2-child", NodeKind::Text)];
        sidebar2.children = vec![nav2];
        let mut main2 = SceneNode::leaf("main", NodeKind::Frame);
        main2.bounds = Rect::xywh(260.0, 0.0, 1180.0, 900.0);
        let mut root2 = SceneNode::leaf("root", NodeKind::Frame);
        root2.bounds = Rect::xywh(0.0, 0.0, 1440.0, 900.0);
        root2.children = vec![filled_header, sidebar2, main2];
        let sets = generating_paint_sets(&[root2], Some(&ind)).expect("active run");
        let ids = &sets.scan;
        assert!(
            ids.contains("main"),
            "the main column washes once it is the first empty shell"
        );
    }

    /// The deck is GLOBAL to the generating root, in document (pre-order)
    /// position — NOT per container. A per-container gate lit the root's
    /// trailing bottom-nav shell while the model was still filling the
    /// header nested inside the content wrapper (user report 2026-07-12:
    /// the bottom band glowed and took the cursor first).
    #[test]
    fn deck_follows_document_order_across_containers_not_per_container() {
        let mut header = SceneNode::leaf("header", NodeKind::Frame);
        header.bounds = Rect::xywh(0.0, 60.0, 390.0, 80.0);
        let mut search = SceneNode::leaf("search", NodeKind::Frame);
        search.bounds = Rect::xywh(0.0, 150.0, 390.0, 60.0);
        let mut wrapper = SceneNode::leaf("wrapper", NodeKind::Frame);
        wrapper.bounds = Rect::xywh(0.0, 60.0, 390.0, 700.0);
        wrapper.children = vec![header, search];

        let mut status = SceneNode::leaf("status", NodeKind::Frame);
        status.bounds = Rect::xywh(0.0, 0.0, 390.0, 44.0);
        status.children = vec![SceneNode::leaf("clock", NodeKind::Text)];
        let mut bottom_nav = SceneNode::leaf("bottom-nav", NodeKind::Frame);
        bottom_nav.bounds = Rect::xywh(0.0, 770.0, 390.0, 74.0);

        let mut root = SceneNode::leaf("root", NodeKind::Frame);
        root.bounds = Rect::xywh(0.0, 0.0, 390.0, 844.0);
        root.children = vec![status, wrapper, bottom_nav];

        let mut ind = AgentIndicators::default();
        ind.run_active = true;
        ind.frames.insert(
            "root".into(),
            op_editor_core::agent_indicators::AgentTag {
                color: "#FF6B6B".into(),
                name: "Fern".into(),
            },
        );
        let sets = generating_paint_sets(&[root], Some(&ind)).expect("active run");
        assert!(
            sets.scan.contains("header"),
            "the header is first in fill order — it is on deck"
        );
        assert!(
            sets.queued.contains("bottom-nav"),
            "the trailing nav shell shows a QUIET skeleton and waits, even \
             though it is its container's first empty child"
        );
        assert!(
            sets.queued.contains("search"),
            "a queued sibling of the deck keeps its skeleton, quietly"
        );
    }

    /// Work-order gate: only the FIRST empty shell in fill order is "on
    /// deck" — a queued sibling shell must not glow while an earlier one
    /// still awaits its content.
    #[test]
    fn only_the_first_empty_shell_in_fill_order_scans() {
        let mut shell_a = SceneNode::leaf("shell-a", NodeKind::Frame);
        shell_a.bounds = Rect::xywh(0.0, 0.0, 390.0, 200.0);
        let mut shell_b = SceneNode::leaf("shell-b", NodeKind::Frame);
        shell_b.bounds = Rect::xywh(0.0, 210.0, 390.0, 200.0);
        let mut root = SceneNode::leaf("root", NodeKind::Frame);
        root.bounds = Rect::xywh(0.0, 0.0, 390.0, 844.0);
        root.children = vec![shell_a, shell_b];
        let roots = vec![root];

        let mut ind = AgentIndicators::default();
        ind.run_active = true;
        ind.frames.insert(
            "root".into(),
            op_editor_core::agent_indicators::AgentTag {
                color: "#FF6B6B".into(),
                name: "Kiki".into(),
            },
        );
        let sets = generating_paint_sets(&roots, Some(&ind)).expect("active run");
        let ids = &sets.scan;
        assert!(ids.contains("shell-a"), "the on-deck shell scans");
        assert!(
            !ids.contains("shell-b"),
            "a queued later shell does not take the ACTIVE radar"
        );
        assert!(
            sets.queued.contains("shell-b"),
            "the queued shell still shows its skeleton — as a quiet wireframe"
        );
        assert!(
            !sets.queued.contains("shell-a"),
            "the on-deck shell is visible"
        );
    }
}
