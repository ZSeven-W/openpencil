//! Tests for `CanvasViewport` paint + the selection-overlay
//! hit-tests (`selection_handle_at_point` / `rotation_corner_at_point`).
//! Split into a sibling file (`#[path]`-included from
//! `canvas_viewport.rs`) to keep that file under the 800-line ceiling.

use super::*;
use crate::layout_scene::{LayoutScene, SceneFillType, SceneNode, ScenePage, SceneStroke};
use crate::{Color, Point2D, Rect, TextLayout};
use std::collections::HashMap;

/// Records op order; clip-isolated paint = `Save, Clip, Fill, …, Restore`.
#[derive(Debug, PartialEq, Eq)]
enum Op {
    Save,
    Restore,
    Clip,
    Scale,
    Fill,
    Stroke,
    Text,
}

#[derive(Default)]
struct RecordingBackend {
    ops: Vec<Op>,
    rects: usize,
    strokes: usize,
    text: usize,
    dots: usize,
}

impl crate::RenderBackend for RecordingBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {
        self.rects += 1;
        self.ops.push(Op::Fill);
    }
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {
        self.strokes += 1;
        self.ops.push(Op::Stroke);
    }
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {
        self.text += 1;
        self.ops.push(Op::Text);
    }
    fn clip_rect(&mut self, _: Rect) {
        self.ops.push(Op::Clip);
    }
    fn save(&mut self) {
        self.ops.push(Op::Save);
    }
    fn restore(&mut self) {
        self.ops.push(Op::Restore);
    }
    fn translate(&mut self, _: Point2D) {}
    fn scale(&mut self, _: Point2D, _: Point2D) {
        self.ops.push(Op::Scale);
    }
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {
        self.strokes += 1;
        self.ops.push(Op::Stroke);
    }
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {
        self.rects += 1;
        self.ops.push(Op::Fill);
    }
    fn fill_dots(&mut self, centers: &[Point2D], _: f32, _: Color) {
        self.dots += centers.len();
        self.ops.push(Op::Fill);
    }
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {
        self.strokes += 1;
        self.ops.push(Op::Stroke);
    }
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {
        self.strokes += 1;
        self.ops.push(Op::Stroke);
    }
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

/// A leaf scene node with bounds + optional fill.
fn leaf(id: &str, kind: NodeKind, bounds: Rect, fill: Option<Color>) -> SceneNode {
    let mut n = SceneNode::leaf(id, kind);
    n.bounds = bounds;
    n.fill = fill;
    n
}

/// A one-page scene mirroring `Document::sample`: a Frame with a
/// stroke, a filled Rect child, and two Text nodes.
fn sample_scene() -> LayoutScene {
    let mut frame = SceneNode::leaf("n1", NodeKind::Frame);
    frame.bounds = Rect::xywh(40.0, 40.0, 320.0, 200.0);
    frame.fill = Some(Color {
        r: 0.16,
        g: 0.16,
        b: 0.2,
        a: 1.0,
    });
    frame.stroke = Some(SceneStroke {
        color: Color::WHITE,
        width: 1.0,
    });
    frame.fill_type = SceneFillType::Solid;
    let mut button = leaf(
        "n2",
        NodeKind::Rect,
        Rect::xywh(60.0, 80.0, 120.0, 40.0),
        Some(Color::BLUE),
    );
    button.stroke = None;
    let mut title = SceneNode::leaf("n3", NodeKind::Text);
    title.bounds = Rect::xywh(60.0, 60.0, 200.0, 20.0);
    title.text = Some("Title".to_string());
    let mut label = SceneNode::leaf("n4", NodeKind::Text);
    label.bounds = Rect::xywh(70.0, 90.0, 100.0, 16.0);
    label.text = Some("Button".to_string());
    frame.children = vec![button, title, label];
    LayoutScene {
        pages: vec![ScenePage {
            id: "p1".into(),
            name: "Page 1".into(),
            children: vec![frame],
        }],
        active_page_index: 0,
    }
}

fn sample_state() -> EditorState {
    EditorState::sample()
}

#[test]
fn from_sample_scene_paints_expected_primitives() {
    let state = sample_state();
    let scene = sample_scene();
    let mut viewport = CanvasViewport::from_editor(&state, &scene);
    // Select the Frame so the overlay stroke paints.
    viewport.selected = "n1".into();
    viewport.selected_set = vec!["n1".into()];
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 800.0, 600.0));
    }
    // ≥3 fills (canvas bg, frame fill, button rect), ≥2 strokes
    // (frame outline + selection overlay), 2 text draws.
    assert!(
        backend.rects >= 3,
        "expected ≥3 fills, got {}",
        backend.rects
    );
    assert!(
        backend.strokes >= 2,
        "expected ≥2 strokes (frame + selection overlay), got {}",
        backend.strokes
    );
    assert_eq!(backend.text, 2, "two text nodes draw two text runs");
}

#[test]
fn empty_scene_paints_canvas_background_and_grid_only() {
    let state = sample_state();
    let scene = LayoutScene::default();
    let viewport = CanvasViewport::from_editor(&state, &scene);
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 100.0, 100.0));
    }
    // Infinite-canvas: bg + grid dots, no document-side strokes
    // / text.
    assert!(backend.rects >= 1, "canvas bg + grid dots");
    assert_eq!(backend.strokes, 0);
    assert_eq!(backend.text, 0);
}

#[test]
fn grid_dot_count_matches_painted_dot_batch() {
    let state = sample_state();
    let scene = LayoutScene::default();
    let rect = Rect::xywh(0.0, 0.0, 320.0, 240.0);
    let paint_dots = |viewport: &CanvasViewport<'_>| -> usize {
        let mut backend = RecordingBackend::default();
        {
            let mut cx = PaintCx {
                backend: &mut backend,
            };
            viewport.paint(&mut cx, rect);
        }
        backend.dots
    };

    let viewport = CanvasViewport::from_editor(&state, &scene);
    assert_eq!(
        paint_dots(&viewport),
        crate::widgets::canvas_viewport_grid::grid_dot_count(rect, &viewport.viewport),
        "grid allocation capacity should match the dot batch exactly"
    );

    let mut viewport = CanvasViewport::from_editor(&state, &scene);
    viewport.viewport.pan_x = 17.0;
    viewport.viewport.pan_y = -23.0;
    viewport.viewport.zoom = 0.37;
    assert_eq!(
        paint_dots(&viewport),
        crate::widgets::canvas_viewport_grid::grid_dot_count(rect, &viewport.viewport),
        "panned and zoomed grid count should still match the painted batch"
    );
}

#[test]
fn empty_reveals_use_plain_node_paint_path() {
    let empty = HashMap::new();
    assert!(
        reveal_schedule_for_paint(&empty, 1_000).is_none(),
        "idle canvas paint should not give every node an empty reveal lookup"
    );

    let active = HashMap::from([("n1".to_string(), 1_000)]);
    let schedule = reveal_schedule_for_paint(&active, 1_250).expect("active reveal schedule");
    assert_eq!(schedule.now_ms, 1_250);
    assert!(std::ptr::eq(schedule.starts, &active));
}

#[test]
fn unselected_scene_skips_overlay_stroke() {
    let state = sample_state();
    let scene = sample_scene();
    // No selection — only the frame's own stroke paints.
    let viewport = CanvasViewport::from_editor(&state, &scene);
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 800.0, 600.0));
    }
    assert_eq!(backend.strokes, 1, "no selection => only the frame stroke");
}

#[test]
fn access_node_advertises_canvas_role() {
    let state = sample_state();
    let scene = sample_scene();
    let viewport = CanvasViewport::from_editor(&state, &scene);
    let node = viewport.access_node();
    assert_eq!(node.role(), accesskit::Role::Canvas);
    assert_eq!(node.label(), Some("Canvas"));
}

#[test]
fn paint_is_clip_isolated_save_clip_then_restore() {
    let state = sample_state();
    let scene = sample_scene();
    let viewport = CanvasViewport::from_editor(&state, &scene);
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 800.0, 600.0));
    }
    // First three ops: Save, Clip, Fill (the canvas bg).
    assert_eq!(
        &backend.ops[..3],
        &[Op::Save, Op::Clip, Op::Fill],
        "canvas paint must open with Save → Clip → bg Fill"
    );
    assert_eq!(
        backend.ops.last(),
        Some(&Op::Restore),
        "canvas paint must close with Restore"
    );
    let saves = backend.ops.iter().filter(|o| **o == Op::Save).count();
    let restores = backend.ops.iter().filter(|o| **o == Op::Restore).count();
    assert_eq!(saves, restores, "balanced save/restore");
    // One outer canvas Save/Clip wraps the whole paint; on top of it
    // each Text node opens its own save/translate/scale/restore for
    // the viewport transform (flip/rotate nodes do the same). This
    // fixture paints two Text nodes, so 1 canvas + 2 text = 3 saves.
    assert_eq!(saves, 3);
}

#[test]
fn paint_with_zero_size_rect_skips_entirely() {
    let state = sample_state();
    let scene = sample_scene();
    let viewport = CanvasViewport::from_editor(&state, &scene);
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 0.0, 0.0));
    }
    assert!(backend.ops.is_empty(), "zero-size rect must paint nothing");
}

#[test]
fn group_kind_recurses_without_own_paint() {
    let _guard = crate::agent_indicator_test_support::lock();
    op_editor_core::agent_indicators::clear();
    let state = sample_state();
    let inner = leaf(
        "n2",
        NodeKind::Rect,
        Rect::xywh(0.0, 0.0, 50.0, 50.0),
        Some(Color::RED),
    );
    let mut group = SceneNode::leaf("n3", NodeKind::Group);
    group.bounds = Rect::xywh(10.0, 10.0, 80.0, 80.0);
    group.fill = Some(Color::BLUE); // fill on group should be ignored
    group.children = vec![inner];
    let scene = LayoutScene {
        pages: vec![ScenePage {
            id: "n1".into(),
            name: "p".into(),
            children: vec![group],
        }],
        active_page_index: 0,
    };
    let viewport = CanvasViewport::from_editor(&state, &scene);
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 200.0, 200.0));
    }
    // canvas bg (1) + grid dots (variable) + leaf rect fill (1)
    // — group fill skipped.
    assert!(backend.rects >= 2, "canvas bg + at least the leaf");
}

#[test]
fn selection_overlay_waits_for_future_reveal_nodes() {
    let _guard = crate::agent_indicator_test_support::lock();
    let epoch = op_editor_core::agent_indicators::begin();
    op_editor_core::agent_indicators::add_reveal(epoch, "n2", 1_200);

    let child = leaf(
        "n2",
        NodeKind::Rect,
        Rect::xywh(10.0, 10.0, 50.0, 30.0),
        Some(Color::RED),
    );
    let mut frame = SceneNode::leaf("n1", NodeKind::Frame);
    frame.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
    frame.children = vec![child];
    let scene = LayoutScene {
        pages: vec![ScenePage {
            id: "p".into(),
            name: "p".into(),
            children: vec![frame],
        }],
        active_page_index: 0,
    };
    let mut state = sample_state();
    state.set_single_selection(op_editor_core::NodeId::new("n2"));
    let mut viewport = CanvasViewport::from_editor(&state, &scene);

    let mut pending_backend = RecordingBackend::default();
    viewport.now_ms = 1_000;
    {
        let mut cx = PaintCx {
            backend: &mut pending_backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 200.0, 200.0));
    }
    assert_eq!(pending_backend.strokes, 0);

    let mut started_backend = RecordingBackend::default();
    viewport.now_ms = 1_200;
    {
        let mut cx = PaintCx {
            backend: &mut started_backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 200.0, 200.0));
    }
    assert!(
        started_backend.strokes > 0,
        "selection overlay should paint once the node starts revealing"
    );
    op_editor_core::agent_indicators::end_if_epoch(epoch);
}

#[test]
fn flipped_node_applies_scale_transform() {
    let state = sample_state();
    let mut node = leaf(
        "flipped",
        NodeKind::Rect,
        Rect::xywh(20.0, 30.0, 50.0, 40.0),
        Some(Color::RED),
    );
    node.flip_x = true;
    let scene = LayoutScene {
        pages: vec![ScenePage {
            id: "p".into(),
            name: "p".into(),
            children: vec![node],
        }],
        active_page_index: 0,
    };
    let viewport = CanvasViewport::from_editor(&state, &scene);
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 200.0, 200.0));
    }

    assert!(
        backend.ops.contains(&Op::Scale),
        "flipX/flipY must apply a canvas scale transform"
    );
}

/// `selection_handle_at_point` + `rotation_corner_at_point` are
/// gated to single-select — multi-select paints an outline-only
/// overlay, so the hit-test must return `None` to match.
#[test]
fn selection_overlay_hit_tests_gate_to_single_select() {
    let scene = sample_scene();
    let canvas_rect = Rect::xywh(0.0, 0.0, 800.0, 600.0);
    // Frame "n1" bounds = (40, 40, 320, 200); at zoom 1, pan 0 the
    // top-left handle sits at the canvas-rect-relative origin.
    let handle_point = Point2D::new(40.0, 40.0);

    // Multi-select → both hit-tests return None.
    let mut multi = sample_state();
    multi.selection.set = vec![
        op_editor_core::NodeId::new("n1"),
        op_editor_core::NodeId::new("n2"),
    ];
    multi.selection.anchor = op_editor_core::NodeId::new("n1");
    assert!(
        selection_handle_at_point(canvas_rect, &scene, &multi, handle_point).is_none(),
        "multi-select must not expose handle hit-tests"
    );
    assert!(
        rotation_corner_at_point(canvas_rect, &scene, &multi, handle_point).is_none(),
        "multi-select must not expose rotation hit-tests"
    );

    // Single-select → the top-left handle is interactive again.
    let mut single = sample_state();
    single.set_single_selection(op_editor_core::NodeId::new("n1"));
    assert_eq!(
        selection_handle_at_point(canvas_rect, &scene, &single, handle_point),
        Some(SelectionHandle::TopLeft),
    );
}

/// The rotation ring is the annulus just OUTSIDE each corner —
/// a point beyond the 6 px handle slop but within 16 px hits it.
#[test]
fn rotation_corner_hit_tests_the_outer_annulus() {
    let scene = sample_scene();
    let canvas_rect = Rect::xywh(0.0, 0.0, 800.0, 600.0);
    let mut single = sample_state();
    single.set_single_selection(op_editor_core::NodeId::new("n1"));
    // 10 px diagonally outside the top-left corner (40, 40).
    let rot_point = Point2D::new(40.0 - 7.0, 40.0 - 7.0);
    assert_eq!(
        rotation_corner_at_point(canvas_rect, &scene, &single, rot_point),
        Some(SelectionHandle::TopLeft),
    );
}

#[test]
fn arc_handle_positions_places_three_handles() {
    use super::{arc_handle_positions, ArcHandle};
    // 100×100 ellipse at origin → centre (50, 50), radii 50.
    let mut node = SceneNode::leaf("e1", NodeKind::Ellipse);
    node.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
    node.arc_start_angle = Some(0.0);
    node.arc_sweep_angle = Some(90.0);
    node.arc_inner_radius = Some(0.5);
    let handles = arc_handle_positions(&node).expect("ellipse yields handles");
    // Start handle at 0° → +X perimeter (100, 50).
    assert_eq!(handles[0].0, ArcHandle::Start);
    assert!((handles[0].1.x - 100.0).abs() < 0.01);
    assert!((handles[0].1.y - 50.0).abs() < 0.01);
    // Sweep handle at 90° → +Y perimeter (50, 100).
    assert_eq!(handles[1].0, ArcHandle::Sweep);
    assert!((handles[1].1.x - 50.0).abs() < 0.01);
    assert!((handles[1].1.y - 100.0).abs() < 0.01);
    // Inner handle at start angle, half radius → (75, 50).
    assert_eq!(handles[2].0, ArcHandle::Inner);
    assert!((handles[2].1.x - 75.0).abs() < 0.01);
}

#[test]
fn arc_handle_positions_none_for_non_ellipse() {
    let mut node = SceneNode::leaf("r1", NodeKind::Rect);
    node.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
    assert!(super::arc_handle_positions(&node).is_none());
}
