use super::*;
use crate::layout_scene::{
    LayoutScene, SceneAnchor, SceneFillType, SceneNode, ScenePage, ScenePointType, SceneStroke,
    SceneStrokeAlign,
};
use crate::{Color, Point2D, Rect, TextLayout};
use jian_ops_schema::node::{ContainerProps, FrameNode, PenNode, PenNodeBase, RectangleNode};
use jian_ops_schema::sizing::SizingBehavior;
use std::collections::HashMap;

#[test]
fn generating_label_text_preserves_plain_names_when_idle() {
    assert_eq!(generating_label_text("Checkout", false), "Checkout");
}

#[test]
fn generating_label_text_prefixes_active_frame_names() {
    assert_eq!(
        generating_label_text("Checkout", true),
        "Generating: Checkout"
    );
    assert_eq!(
        generating_label_text("移动端首页", true),
        "Generating: 移动端首页"
    );
}

/// Records op order; clip-isolated paint = `Save, Clip, Fill, …, Restore`.
#[derive(Debug, PartialEq, Eq)]
enum Op {
    Save,
    Restore,
    Clip,
    Scale,
    Rotate,
    Fill,
    Stroke,
    StrokeOval,
    Text,
}

#[derive(Default)]
struct RecordingBackend {
    ops: Vec<Op>,
    rects: usize,
    strokes: usize,
    text: usize,
    texts: Vec<String>,
    text_colors: Vec<jian_core::scene::Color>,
    text_positions: Vec<Point2D>,
    fill_rects: Vec<Rect>,
    dots: usize,
    mesh_fills: usize,
    shader_fills: usize,
    /// One `(radians, pivot)` per [`Op::Rotate`], in op order.
    rotations: Vec<(f32, Point2D)>,
    /// One `(scale, pivot)` per [`Op::Scale`], in op order.
    scales: Vec<(Point2D, Point2D)>,
    /// One color per [`Op::Stroke`], in op order.
    stroke_colors: Vec<Color>,
    /// Whether each stroke was emitted through `stroke_line`.
    stroke_is_line: Vec<bool>,
    stroke_rects: Vec<(Rect, Color)>,
    stroke_lines: Vec<(Point2D, Point2D, Color)>,
}

impl crate::RenderBackend for RecordingBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, rect: Rect, _: Color) {
        self.fill_rects.push(rect);
        self.rects += 1;
        self.ops.push(Op::Fill);
    }
    fn stroke_rect(&mut self, rect: Rect, color: Color, _: f32) {
        self.strokes += 1;
        self.stroke_colors.push(color);
        self.stroke_is_line.push(false);
        self.stroke_rects.push((rect, color));
        self.ops.push(Op::Stroke);
    }
    fn draw_text(&mut self, layout: &TextLayout, point: Point2D) {
        self.text += 1;
        self.texts
            .extend(layout.runs().iter().map(|run| run.content.clone()));
        self.text_colors
            .extend(layout.runs().iter().map(|run| run.color));
        self.text_positions
            .extend(layout.runs().iter().map(|_| point));
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
    fn scale(&mut self, scale: Point2D, pivot: Point2D) {
        self.scales.push((scale, pivot));
        self.ops.push(Op::Scale);
    }
    fn rotate(&mut self, radians: f32, pivot: Point2D) {
        self.rotations.push((radians, pivot));
        self.ops.push(Op::Rotate);
    }
    fn stroke_line(&mut self, from: Point2D, to: Point2D, color: Color, _: f32) {
        self.strokes += 1;
        self.stroke_colors.push(color);
        self.stroke_is_line.push(true);
        self.stroke_lines.push((from, to, color));
        self.ops.push(Op::Stroke);
    }
    fn fill_round_rect(&mut self, rect: Rect, _: f32, _: Color) {
        self.fill_rects.push(rect);
        self.rects += 1;
        self.ops.push(Op::Fill);
    }
    fn fill_dots(&mut self, centers: &[Point2D], _: f32, _: Color) {
        self.dots += centers.len();
        self.ops.push(Op::Fill);
    }
    fn stroke_round_rect(&mut self, _: Rect, _: f32, color: Color, _: f32) {
        self.strokes += 1;
        self.stroke_colors.push(color);
        self.stroke_is_line.push(false);
        self.ops.push(Op::Stroke);
    }
    fn fill_oval(&mut self, _: Rect, _: Color) {
        self.ops.push(Op::Fill);
    }
    fn stroke_oval(&mut self, _: Rect, color: Color, _: f32) {
        self.strokes += 1;
        self.stroke_colors.push(color);
        self.stroke_is_line.push(false);
        self.ops.push(Op::StrokeOval);
    }
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, color: Color, _: f32) {
        self.strokes += 1;
        self.stroke_colors.push(color);
        self.stroke_is_line.push(false);
        self.ops.push(Op::Stroke);
    }
    fn fill_round_rect_mesh_gradient(
        &mut self,
        _: Rect,
        _: f32,
        _: u32,
        _: u32,
        _: &[Color],
        _: f32,
    ) {
        self.mesh_fills += 1;
        self.ops.push(Op::Fill);
    }
    fn fill_round_rect_shader(
        &mut self,
        _: Rect,
        _: f32,
        _: &str,
        _: &[(&str, &[f32])],
        _: f32,
        _: Color,
    ) {
        self.shader_fills += 1;
        self.ops.push(Op::Fill);
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

fn named_rect_node(id: &str, name: &str) -> PenNode {
    PenNode::Rectangle(RectangleNode {
        base: PenNodeBase {
            id: id.to_string(),
            name: Some(name.to_string()),
            ..Default::default()
        },
        container: ContainerProps {
            width: Some(SizingBehavior::Number(100.0)),
            height: Some(SizingBehavior::Number(40.0)),
            ..Default::default()
        },
        children: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

fn named_frame_node(id: &str, name: &str) -> PenNode {
    PenNode::Frame(FrameNode {
        base: PenNodeBase {
            id: id.to_string(),
            name: Some(name.to_string()),
            ..Default::default()
        },
        container: ContainerProps {
            width: Some(SizingBehavior::Number(320.0)),
            height: Some(SizingBehavior::Number(200.0)),
            ..Default::default()
        },
        children: Some(Vec::new()),
        image_search_query: None,
        reusable: None,
        screen: None,
        slot: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
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
        sides: None,
        align: SceneStrokeAlign::Center,
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

/// A focus frame with two visible direct children, one hidden direct
/// child, and one grandchild. Hover hierarchy tests use the distinct
/// edge coordinates to prove that only immediate visible children
/// receive dashed hints.
fn hover_hierarchy_scene() -> LayoutScene {
    let direct_a = leaf(
        "direct-a",
        NodeKind::Rect,
        Rect::xywh(30.0, 40.0, 48.0, 32.0),
        None,
    );
    let grandchild = leaf(
        "grandchild",
        NodeKind::Rect,
        Rect::xywh(122.0, 78.0, 24.0, 20.0),
        None,
    );
    let mut direct_b = SceneNode::leaf("direct-b", NodeKind::Frame);
    direct_b.bounds = Rect::xywh(100.0, 40.0, 80.0, 100.0);
    direct_b.children = vec![grandchild];
    let mut hidden = leaf(
        "hidden-direct",
        NodeKind::Rect,
        Rect::xywh(188.0, 40.0, 24.0, 32.0),
        None,
    );
    hidden.hidden = true;

    let mut focus = SceneNode::leaf("focus", NodeKind::Frame);
    focus.bounds = Rect::xywh(10.0, 10.0, 220.0, 180.0);
    focus.children = vec![direct_a, direct_b, hidden];
    LayoutScene {
        pages: vec![ScenePage {
            id: "p".into(),
            name: "Page".into(),
            children: vec![focus],
        }],
        active_page_index: 0,
    }
}

fn sample_state() -> EditorState {
    EditorState::sample()
}

#[test]
fn single_selection_overlay_omits_name_and_paints_dimensions() {
    let _guard = crate::agent_indicator_test_support::lock();
    op_editor_core::agent_indicators::clear();
    let mut state = EditorState::new();
    state.doc.children = vec![named_rect_node("n2", "Schedule Card 1")];
    state.set_single_selection(op_editor_core::NodeId::new("n2"));
    let scene = sample_scene();
    let viewport = CanvasViewport::from_editor(&state, &scene);
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 800.0, 600.0));
    }

    assert!(!backend.texts.iter().any(|text| text == "Schedule Card 1"));
    assert!(
        backend
            .texts
            .iter()
            .all(|text| !text.starts_with("Selected:")),
        "single-select overlay must not paint a count label"
    );
    assert!(
        backend.texts.iter().any(|text| text == "120 × 40"),
        "single-select overlay should paint bottom dimensions; texts: {:?}",
        backend.texts
    );
}

#[test]
fn single_selection_dimension_label_uses_active_color() {
    let _guard = crate::agent_indicator_test_support::lock();
    op_editor_core::agent_indicators::clear();
    let mut state = EditorState::new();
    state.doc.children = vec![named_rect_node("n2", "Schedule Card 1")];
    state.set_single_selection(op_editor_core::NodeId::new("n2"));
    let scene = sample_scene();
    let viewport = CanvasViewport::from_editor(&state, &scene);
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 800.0, 600.0));
    }

    let color = backend
        .texts
        .iter()
        .zip(backend.text_colors.iter())
        .find_map(|(text, color)| (text == "120 × 40").then_some(*color))
        .expect("selected dimensions label should be painted");
    assert_eq!(color, viewport.theme.primary.to_jian());
}

#[test]
fn active_node_drag_hides_selection_chrome_and_dimension_label() {
    let mut state = EditorState::new();
    state.doc.children = vec![named_rect_node("n2", "Schedule Card 1")];
    state.set_single_selection(op_editor_core::NodeId::new("n2"));
    let scene = sample_scene();
    let mut viewport = CanvasViewport::from_editor(&state, &scene);
    viewport.node_drag_active = true;
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 800.0, 600.0));
    }

    assert!(
        backend.texts.iter().all(|text| text != "120 × 40"),
        "dragging should not paint the selected-size capsule; texts: {:?}",
        backend.texts
    );
    assert_eq!(
        backend.strokes, 1,
        "dragging should hide selection outlines/handles, leaving only the frame stroke"
    );
}

#[test]
fn active_node_drag_suppresses_focus_and_child_hover_outlines() {
    let _guard = crate::agent_indicator_test_support::lock();
    op_editor_core::agent_indicators::clear();
    let mut state = EditorState::new();
    state.doc.children = vec![named_frame_node("n1", "Frame")];
    state.set_single_selection(op_editor_core::NodeId::new("n2"));
    state.editor_ui.canvas_hover_node = Some(op_editor_core::NodeId::new("n1"));
    let scene = sample_scene();
    let mut viewport = CanvasViewport::from_editor(&state, &scene);
    viewport.node_drag_active = true;
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 800.0, 600.0));
    }

    assert!(
        backend
            .stroke_colors
            .iter()
            .all(|color| *color != HOVER_OUTLINE_COLOR),
        "dragging should suppress both the focus outline and all direct-child hierarchy hints"
    );
    let frame_label_color = backend
        .texts
        .iter()
        .zip(backend.text_colors.iter())
        .find_map(|(text, color)| (text == "Frame").then_some(*color))
        .expect("root frame label should paint");
    assert_ne!(
        frame_label_color,
        viewport.theme.primary.to_jian(),
        "dragging should suppress the transient root-title hover tint too"
    );
}

#[test]
fn active_node_drag_overlay_paints_selected_node_at_absolute_drag_origin() {
    let mut state = EditorState::new();
    state.doc.children = vec![named_rect_node("n2", "Schedule Card 1")];
    state.set_single_selection(op_editor_core::NodeId::new("n2"));
    let scene = sample_scene();
    let mut viewport = CanvasViewport::from_editor(&state, &scene);
    viewport.node_drag_active = true;
    viewport.node_drag_overlay = Some(CanvasNodeDragOverlay {
        node_id: "n2".to_string(),
        target_origin_doc: Point2D::new(100.0, 100.0),
    });
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 800.0, 600.0));
    }

    assert!(
        !backend
            .fill_rects
            .iter()
            .any(|rect| *rect == Rect::xywh(60.0, 80.0, 120.0, 40.0)),
        "drag overlay should hide the selected node at its layout slot"
    );
    assert!(
        backend
            .fill_rects
            .iter()
            .any(|rect| *rect == Rect::xywh(100.0, 100.0, 120.0, 40.0)),
        "drag overlay should paint the selected node at the absolute cursor-following origin; fills: {:?}",
        backend.fill_rects
    );
}

#[test]
fn active_node_drag_drop_indicator_omits_dragged_ghost_rect() {
    let mut state = EditorState::new();
    state.doc.children = vec![named_rect_node("n2", "Schedule Card 1")];
    state.set_single_selection(op_editor_core::NodeId::new("n2"));
    let scene = sample_scene();
    let mut viewport = CanvasViewport::from_editor(&state, &scene);
    viewport.node_drag_active = true;
    viewport.drop_indicator = Some(op_editor_core::editor_ui_state::CanvasDropIndicator {
        ghost: op_editor_core::editor_ui_state::CanvasOverlayRect::new(420.0, 30.0, 96.0, 48.0),
        target: Some(op_editor_core::editor_ui_state::CanvasOverlayRect::new(
            540.0, 30.0, 120.0, 80.0,
        )),
        insertion: Some(op_editor_core::editor_ui_state::CanvasOverlayLine::new(
            528.0, 24.0, 528.0, 116.0,
        )),
    });

    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 800.0, 600.0));
    }

    assert!(
        !backend
            .fill_rects
            .iter()
            .any(|rect| *rect == Rect::xywh(420.0, 30.0, 96.0, 48.0)),
        "dragging should not paint a ghost fill around the dragged element; fills: {:?}",
        backend.fill_rects
    );
    assert!(
        backend
            .fill_rects
            .iter()
            .any(|rect| *rect == Rect::xywh(540.0, 30.0, 120.0, 80.0)),
        "dragging should still paint the target container preview; fills: {:?}",
        backend.fill_rects
    );
}

#[test]
fn selected_root_frame_paints_one_name_label() {
    let mut state = EditorState::new();
    state.doc.children = vec![named_frame_node("n1", "Frame")];
    state.set_single_selection(op_editor_core::NodeId::new("n1"));
    let scene = sample_scene();
    let viewport = CanvasViewport::from_editor(&state, &scene);
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 800.0, 600.0));
    }

    let painted_count = backend.texts.iter().filter(|text| *text == "Frame").count();
    assert_eq!(
        painted_count, 1,
        "selected root frame should not draw both the root label and selection pill"
    );
    let position = backend
        .texts
        .iter()
        .zip(backend.text_positions.iter())
        .find_map(|(text, position)| (text == "Frame").then_some(*position))
        .expect("selected root frame label should be painted");
    assert_eq!(
        position.x, 40.0,
        "selected root frame label should use the plain root-title position, not pill padding"
    );
}

#[test]
fn multi_selection_overlay_omits_count_label_and_paints_union_dimensions() {
    let mut state = EditorState::new();
    state.doc.children = vec![
        named_rect_node("n2", "Schedule Card 1"),
        named_rect_node("n3", "Schedule Card 2"),
    ];
    state.selection.set = vec![
        op_editor_core::NodeId::new("n2"),
        op_editor_core::NodeId::new("n3"),
    ];
    state.selection.anchor = op_editor_core::NodeId::new("n3");
    let scene = sample_scene();
    let viewport = CanvasViewport::from_editor(&state, &scene);
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 800.0, 600.0));
    }

    assert!(!backend
        .texts
        .iter()
        .any(|text| text == "Selected: 2 objects"));
    assert!(
        backend.texts.iter().all(|text| text != "Schedule Card 1"),
        "multi-select overlay must not paint individual selected node names"
    );
    assert!(
        backend.texts.iter().any(|text| text == "200 × 60"),
        "multi-select overlay should paint union dimensions; texts: {:?}",
        backend.texts
    );
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
    // >=3 fills (canvas bg, frame fill, button rect), >=2 strokes
    // (frame outline + selection overlay), and the two scene text runs
    // still draw even when overlay labels add their own text.
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
    assert!(backend.texts.iter().any(|text| text == "Title"));
    assert!(backend.texts.iter().any(|text| text == "Button"));
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
    // 900 < reveal-250: before the sparkle cursor's entry window, so no cursor strokes either.
    viewport.now_ms = 900;
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
fn single_selected_canvas_paint_reuses_paint_traversal_lookup() {
    let _guard = crate::agent_indicator_test_support::lock();
    op_editor_core::agent_indicators::clear();

    let mut selected = SceneNode::leaf("selected-ellipse", NodeKind::Ellipse);
    selected.bounds = Rect::xywh(24.0, 24.0, 40.0, 40.0);
    selected.arc_start_angle = Some(0.0);
    selected.arc_sweep_angle = Some(180.0);
    let mut node = selected;
    let depth = 7;
    for i in (0..depth).rev() {
        let mut frame = SceneNode::leaf(format!("wrap-{i}"), NodeKind::Frame);
        frame.bounds = Rect::xywh(0.0, 0.0, 120.0, 120.0);
        frame.children = vec![node];
        node = frame;
    }
    let scene = LayoutScene {
        pages: vec![ScenePage {
            id: "p".into(),
            name: "p".into(),
            children: vec![node],
        }],
        active_page_index: 0,
    };
    let mut state = EditorState::new();
    state.viewport.zoom = 1.0;
    state.viewport.pan_x = 0.0;
    state.viewport.pan_y = 0.0;
    state.set_single_selection(op_editor_core::NodeId::new("selected-ellipse"));
    state.tool = op_editor_core::Tool::Select;
    let viewport = CanvasViewport::from_editor(&state, &scene);

    crate::layout_scene::reset_find_visit_count();
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 300.0, 300.0));
    }

    assert_eq!(
        crate::layout_scene::find_visit_count(),
        0,
        "single-selected paint should use the scene node already seen during paint traversal"
    );
}

#[test]
fn pen_preview_canvas_paint_reuses_paint_traversal_lookup() {
    let _guard = crate::agent_indicator_test_support::lock();
    op_editor_core::agent_indicators::clear();

    let anchor = |x, y| SceneAnchor {
        pos: Point2D::new(x, y),
        handle_in: None,
        handle_out: None,
        point_type: ScenePointType::Corner,
    };
    let mut path = SceneNode::leaf("editing-path", NodeKind::Path);
    path.bounds = Rect::xywh(24.0, 24.0, 80.0, 40.0);
    path.points = vec![Point2D::new(24.0, 24.0), Point2D::new(104.0, 64.0)];
    path.path_anchors = vec![anchor(24.0, 24.0), anchor(104.0, 64.0)];
    let mut node = path;
    for i in (0..7).rev() {
        let mut frame = SceneNode::leaf(format!("wrap-{i}"), NodeKind::Frame);
        frame.bounds = Rect::xywh(0.0, 0.0, 140.0, 100.0);
        frame.children = vec![node];
        node = frame;
    }
    let scene = LayoutScene {
        pages: vec![ScenePage {
            id: "p".into(),
            name: "p".into(),
            children: vec![node],
        }],
        active_page_index: 0,
    };
    let state = EditorState::new();
    let mut viewport = CanvasViewport::from_editor(&state, &scene);
    viewport.pen_in_progress = Some("editing-path".into());
    viewport.pen_cursor_doc = Some(Point2D::new(120.0, 80.0));

    crate::layout_scene::reset_find_visit_count();
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 300.0, 300.0));
    }

    assert_eq!(
        crate::layout_scene::find_visit_count(),
        0,
        "pen preview paint should use the scene node already seen during paint traversal"
    );
    assert!(backend.strokes > 0, "pen preview should still paint");
}

#[test]
fn frame_label_paint_uses_top_level_scene_nodes() {
    let _guard = crate::agent_indicator_test_support::lock();
    op_editor_core::agent_indicators::clear();

    let mut roots = Vec::new();
    let mut labels = Vec::new();
    for i in 0..18 {
        let mut child = SceneNode::leaf(format!("child-{i}"), NodeKind::Rect);
        child.bounds = Rect::xywh(8.0, 8.0, 12.0, 12.0);
        let mut frame = SceneNode::leaf(format!("frame-{i}"), NodeKind::Frame);
        frame.bounds = Rect::xywh(i as f32 * 140.0, 0.0, 120.0, 80.0);
        frame.children = vec![child];
        labels.push(FrameLabel::new(
            format!("frame-{i}"),
            format!("Frame {i}"),
            Color {
                r: 0.6,
                g: 0.6,
                b: 0.6,
                a: 1.0,
            },
            false,
        ));
        roots.push(frame);
    }
    let scene = LayoutScene {
        pages: vec![ScenePage {
            id: "p".into(),
            name: "p".into(),
            children: roots,
        }],
        active_page_index: 0,
    };
    let mut state = EditorState::new();
    state.viewport.zoom = 1.0;
    let mut viewport = CanvasViewport::from_editor(&state, &scene);
    viewport.frame_labels = labels;

    crate::layout_scene::reset_find_visit_count();
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 2_800.0, 300.0));
    }

    assert_eq!(
        crate::layout_scene::find_visit_count(),
        0,
        "frame labels belong to top-level roots and should not deep-search the scene"
    );
}

#[test]
fn generating_frame_label_paints_vector_icon_and_keeps_full_hit_area() {
    let mut frame = SceneNode::leaf("n1", NodeKind::Frame);
    frame.bounds = Rect::xywh(40.0, 40.0, 120.0, 80.0);
    let color = Color {
        r: 0.0,
        g: 0.48,
        b: 1.0,
        a: 1.0,
    };
    let labels = vec![FrameLabel::new("n1", "Generating: Frame", color, true)];
    let mut state = EditorState::new();
    state.viewport.zoom = 1.0;
    let clip = Rect::xywh(0.0, 0.0, 400.0, 300.0);
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        super::super::canvas_frame_labels::paint_frame_labels(
            &mut cx,
            std::slice::from_ref(&frame),
            &labels,
            &[],
            Point2D::ZERO,
            &state.viewport,
            clip,
        );
    }

    assert_eq!(backend.texts, vec!["Generating: Frame"]);
    assert_eq!(
        backend.strokes,
        super::super::icons::Icon::Sparkles.paths().len()
    );
    assert_eq!(
        super::super::canvas_frame_labels::frame_label_at_point(
            std::slice::from_ref(&frame),
            &labels,
            Point2D::ZERO,
            &state.viewport,
            clip,
            Point2D::new(170.0, 20.0),
        ),
        Some("n1".into())
    );

    let idle_labels = vec![FrameLabel::new("n1", "Frame", color, false)];
    let mut idle_backend = RecordingBackend::default();
    let mut cx = PaintCx {
        backend: &mut idle_backend,
    };
    super::super::canvas_frame_labels::paint_frame_labels(
        &mut cx,
        &[frame],
        &idle_labels,
        &[],
        Point2D::ZERO,
        &state.viewport,
        clip,
    );
    assert_eq!(idle_backend.strokes, 0);
}

#[test]
fn selected_root_frame_label_uses_primary_active_color() {
    let scene = sample_scene();
    let mut state = EditorState::new();
    state.doc.children = vec![named_frame_node("n1", "Frame")];
    state.set_single_selection(op_editor_core::NodeId::new("n1"));
    let viewport = CanvasViewport::from_editor(&state, &scene);

    let color = viewport
        .frame_labels
        .iter()
        .find(|label| label.id == "n1")
        .map(|label| label.color)
        .expect("root frame label should be collected");

    assert_eq!(color, viewport.theme.primary);
}

#[test]
fn hovered_root_frame_label_uses_primary_active_color() {
    let _guard = crate::agent_indicator_test_support::lock();
    op_editor_core::agent_indicators::clear();
    let scene = sample_scene();
    let mut state = EditorState::new();
    state.doc.children = vec![named_frame_node("n1", "Frame")];
    state.editor_ui.canvas_hover_node = Some(op_editor_core::NodeId::new("n1"));
    let viewport = CanvasViewport::from_editor(&state, &scene);
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 800.0, 600.0));
    }

    let color = backend
        .texts
        .iter()
        .zip(backend.text_colors.iter())
        .find_map(|(text, color)| (text == "Frame").then_some(*color))
        .expect("root frame label should paint");

    assert_eq!(color, viewport.theme.primary.to_jian());
}

#[test]
fn frame_label_paint_matches_roots_linearly() {
    let _guard = crate::agent_indicator_test_support::lock();
    op_editor_core::agent_indicators::clear();

    let mut roots = Vec::new();
    let mut labels = Vec::new();
    for i in 0..32 {
        let mut frame = SceneNode::leaf(format!("frame-{i}"), NodeKind::Frame);
        frame.bounds = Rect::xywh(i as f32 * 90.0, 0.0, 80.0, 48.0);
        roots.push(frame);
        labels.push(FrameLabel::new(
            format!("frame-{i}"),
            format!("Frame {i}"),
            Color {
                r: 0.6,
                g: 0.6,
                b: 0.6,
                a: 1.0,
            },
            false,
        ));
    }
    let scene = LayoutScene {
        pages: vec![ScenePage {
            id: "p".into(),
            name: "p".into(),
            children: roots,
        }],
        active_page_index: 0,
    };
    let mut state = EditorState::new();
    state.viewport.zoom = 1.0;
    let mut viewport = CanvasViewport::from_editor(&state, &scene);
    viewport.frame_labels = labels;

    super::super::canvas_frame_labels::reset_label_match_count();
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 3_200.0, 240.0));
    }

    assert!(
        super::super::canvas_frame_labels::label_match_count() <= 32,
        "frame label matching should stay linear in the number of top-level roots"
    );
}

#[test]
fn multi_selection_overlay_finds_nodes_linearly() {
    let _guard = crate::agent_indicator_test_support::lock();
    op_editor_core::agent_indicators::clear();

    let roots: Vec<_> = (0..32)
        .map(|i| {
            let mut frame = SceneNode::leaf(format!("frame-{i}"), NodeKind::Frame);
            frame.bounds = Rect::xywh(i as f32 * 12.0, 0.0, 10.0, 10.0);
            frame
        })
        .collect();
    let scene = LayoutScene {
        pages: vec![ScenePage {
            id: "p".into(),
            name: "p".into(),
            children: roots,
        }],
        active_page_index: 0,
    };
    let state = EditorState::new();
    let mut viewport = CanvasViewport::from_editor(&state, &scene);
    viewport.selected_set = (0..32).map(|i| format!("frame-{i}")).collect();
    viewport.frame_labels.clear();

    crate::layout_scene::reset_find_visit_count();
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 800.0, 200.0));
    }

    assert!(
        crate::layout_scene::find_visit_count() <= 32,
        "multi-selection overlay should resolve selected nodes in one scene pass"
    );
}

#[test]
fn hover_outline_does_not_deep_search_after_scene_paint() {
    let _guard = crate::agent_indicator_test_support::lock();
    op_editor_core::agent_indicators::clear();

    let mut node = SceneNode::leaf("hovered-leaf", NodeKind::Rect);
    node.bounds = Rect::xywh(24.0, 24.0, 40.0, 40.0);
    let depth = 8;
    for i in (0..depth).rev() {
        let mut frame = SceneNode::leaf(format!("wrap-{i}"), NodeKind::Frame);
        frame.bounds = Rect::xywh(0.0, 0.0, 120.0, 120.0);
        frame.children = vec![node];
        node = frame;
    }
    let scene = LayoutScene {
        pages: vec![ScenePage {
            id: "p".into(),
            name: "p".into(),
            children: vec![node],
        }],
        active_page_index: 0,
    };
    let state = EditorState::new();
    let mut viewport = CanvasViewport::from_editor(&state, &scene);
    viewport.hovered = Some("hovered-leaf".into());

    crate::layout_scene::reset_find_visit_count();
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 300.0, 300.0));
    }

    assert_eq!(
        crate::layout_scene::find_visit_count(),
        0,
        "hover outline should paint during the existing scene walk instead of deep-searching after paint"
    );
    assert!(
        backend.strokes > 0,
        "hover outline should still paint dashed strokes"
    );
}

/// Mirror of the hover-outline stroke color in `canvas_viewport.rs`.
const HOVER_OUTLINE_COLOR: Color = Color {
    r: 0.231,
    g: 0.51,
    b: 0.965,
    a: 1.0,
};
const SELECTION_BLUE: Color = Color {
    r: 13.0 / 255.0,
    g: 153.0 / 255.0,
    b: 1.0,
    a: 1.0,
};

fn line_lies_on_rect_edge(from: Point2D, to: Point2D, rect: Rect) -> bool {
    const EPSILON: f32 = 0.01;
    let left = rect.origin.x;
    let right = rect.origin.x + rect.size.x;
    let top = rect.origin.y;
    let bottom = rect.origin.y + rect.size.y;
    let in_x = |x: f32| x >= left - EPSILON && x <= right + EPSILON;
    let in_y = |y: f32| y >= top - EPSILON && y <= bottom + EPSILON;
    let same = |a: f32, b: f32| (a - b).abs() <= EPSILON;
    ((same(from.y, top) && same(to.y, top)) || (same(from.y, bottom) && same(to.y, bottom)))
        && in_x(from.x)
        && in_x(to.x)
        || ((same(from.x, left) && same(to.x, left)) || (same(from.x, right) && same(to.x, right)))
            && in_y(from.y)
            && in_y(to.y)
}

#[test]
fn hover_focus_is_solid_and_only_direct_visible_children_are_dashed() {
    let _guard = crate::agent_indicator_test_support::lock();
    op_editor_core::agent_indicators::clear();
    let scene = hover_hierarchy_scene();
    let mut state = EditorState::new();
    state.editor_ui.canvas_hover_node = Some(op_editor_core::NodeId::new("focus"));
    let viewport = CanvasViewport::from_editor(&state, &scene);
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 300.0, 240.0));
    }

    let focus_solids: Vec<Rect> = backend
        .stroke_rects
        .iter()
        .filter_map(|(rect, color)| (*color == HOVER_OUTLINE_COLOR).then_some(*rect))
        .collect();
    assert_eq!(
        focus_solids,
        vec![Rect::xywh(10.0, 10.0, 220.0, 180.0)],
        "an unselected hierarchy focus should paint one solid outline"
    );

    let direct_a = Rect::xywh(30.0, 40.0, 48.0, 32.0);
    let direct_b = Rect::xywh(100.0, 40.0, 80.0, 100.0);
    let child_lines: Vec<(Point2D, Point2D)> = backend
        .stroke_lines
        .iter()
        .filter_map(|(from, to, color)| (*color == HOVER_OUTLINE_COLOR).then_some((*from, *to)))
        .collect();
    assert!(
        !child_lines.is_empty(),
        "direct children should paint dashed hints"
    );
    assert!(child_lines
        .iter()
        .any(|(from, to)| line_lies_on_rect_edge(*from, *to, direct_a)));
    assert!(child_lines
        .iter()
        .any(|(from, to)| line_lies_on_rect_edge(*from, *to, direct_b)));
    assert!(
        child_lines.iter().all(|(from, to)| {
            line_lies_on_rect_edge(*from, *to, direct_a)
                || line_lies_on_rect_edge(*from, *to, direct_b)
        }),
        "hidden direct children and visible grandchildren must not receive hierarchy hints"
    );
}

#[test]
fn selected_hover_focus_keeps_child_hints_selection_handles_and_dimensions() {
    let _guard = crate::agent_indicator_test_support::lock();
    op_editor_core::agent_indicators::clear();
    let scene = hover_hierarchy_scene();
    let mut state = EditorState::new();
    state.doc.children = vec![named_frame_node("focus", "Focus")];
    state.set_single_selection(op_editor_core::NodeId::new("focus"));
    state.editor_ui.canvas_hover_node = Some(op_editor_core::NodeId::new("focus"));
    let mut viewport = CanvasViewport::from_editor(&state, &scene);
    viewport.selection_label = Some("220 × 180".into());
    assert_eq!(
        viewport.hovered.as_deref(),
        Some("focus"),
        "selected nodes must remain eligible as hierarchy hover focus"
    );

    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 300.0, 240.0));
    }

    assert!(
        backend
            .stroke_rects
            .iter()
            .all(|(_, color)| *color != HOVER_OUTLINE_COLOR),
        "a selected focus should not duplicate its solid outline"
    );
    assert!(
        backend
            .stroke_lines
            .iter()
            .any(|(_, _, color)| *color == HOVER_OUTLINE_COLOR),
        "a selected focus should still expose dashed direct-child hints"
    );
    assert!(
        backend
            .stroke_colors
            .iter()
            .filter(|color| **color == viewport.theme.primary)
            .count()
            >= 8,
        "single selection handles should still paint above hierarchy hover"
    );
    assert!(
        backend.texts.iter().any(|text| text == "220 × 180"),
        "the selected dimensions capsule should remain visible"
    );
}

/// Replay the recorded op stream up to the first stroke painted in
/// `color`, tracking the save/restore transform stack, and return the
/// `(radians, pivot)` rotations active at that stroke. `None` when no
/// stroke of that color was painted.
fn active_rotations_at_first_stroke(
    backend: &RecordingBackend,
    color: Color,
) -> Option<Vec<(f32, Point2D)>> {
    let mut stroke_i = 0usize;
    let mut rot_i = 0usize;
    let mut stack: Vec<Vec<(f32, Point2D)>> = vec![Vec::new()];
    for op in &backend.ops {
        match op {
            Op::Save => {
                let top = stack.last().cloned().unwrap_or_default();
                stack.push(top);
            }
            Op::Restore => {
                stack.pop();
            }
            Op::Rotate => {
                stack
                    .last_mut()
                    .expect("unbalanced save/restore")
                    .push(backend.rotations[rot_i]);
                rot_i += 1;
            }
            Op::Stroke | Op::StrokeOval => {
                if backend.stroke_colors[stroke_i] == color {
                    return Some(stack.last().cloned().unwrap_or_default());
                }
                stroke_i += 1;
            }
            _ => {}
        }
    }
    None
}

fn active_rotations_at_first_oval_stroke(
    backend: &RecordingBackend,
    color: Color,
) -> Option<Vec<(f32, Point2D)>> {
    let mut stroke_i = 0usize;
    let mut rot_i = 0usize;
    let mut stack: Vec<Vec<(f32, Point2D)>> = vec![Vec::new()];
    for op in &backend.ops {
        match op {
            Op::Save => {
                let top = stack.last().cloned().unwrap_or_default();
                stack.push(top);
            }
            Op::Restore => {
                stack.pop();
            }
            Op::Rotate => {
                stack
                    .last_mut()
                    .expect("unbalanced save/restore")
                    .push(backend.rotations[rot_i]);
                rot_i += 1;
            }
            Op::Stroke | Op::StrokeOval => {
                let is_match =
                    matches!(op, Op::StrokeOval) && backend.stroke_colors[stroke_i] == color;
                if is_match {
                    return Some(stack.last().cloned().unwrap_or_default());
                }
                stroke_i += 1;
            }
            _ => {}
        }
    }
    None
}

#[derive(Clone, Default)]
struct ActiveTransforms {
    rotations: Vec<(f32, Point2D)>,
    scales: Vec<(Point2D, Point2D)>,
}

fn active_transforms_at_first_line_stroke(
    backend: &RecordingBackend,
    color: Color,
) -> Option<ActiveTransforms> {
    let mut stroke_i = 0usize;
    let mut rot_i = 0usize;
    let mut scale_i = 0usize;
    let mut stack = vec![ActiveTransforms::default()];
    for op in &backend.ops {
        match op {
            Op::Save => {
                let top = stack.last().cloned().unwrap_or_default();
                stack.push(top);
            }
            Op::Restore => {
                stack.pop();
            }
            Op::Scale => {
                stack
                    .last_mut()
                    .expect("unbalanced save/restore")
                    .scales
                    .push(backend.scales[scale_i]);
                scale_i += 1;
            }
            Op::Rotate => {
                stack
                    .last_mut()
                    .expect("unbalanced save/restore")
                    .rotations
                    .push(backend.rotations[rot_i]);
                rot_i += 1;
            }
            Op::Stroke | Op::StrokeOval => {
                let is_match =
                    backend.stroke_is_line[stroke_i] && backend.stroke_colors[stroke_i] == color;
                if is_match {
                    return stack.last().cloned();
                }
                stroke_i += 1;
            }
            _ => {}
        }
    }
    None
}

#[test]
fn hover_outline_rotates_with_rotated_frame() {
    let _guard = crate::agent_indicator_test_support::lock();
    op_editor_core::agent_indicators::clear();
    let mut scene = sample_scene();
    let rotation = 0.5_f32;
    scene.pages[0].children[0].rotation = rotation;
    let state = EditorState::new();
    let mut viewport = CanvasViewport::from_editor(&state, &scene);
    viewport.hovered = Some("n1".into());
    let mut backend = RecordingBackend::default();
    let rect = Rect::xywh(0.0, 0.0, 800.0, 600.0);
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, rect);
    }

    let rotations = active_rotations_at_first_stroke(&backend, HOVER_OUTLINE_COLOR)
        .expect("hover outline should paint dashed strokes");
    assert_eq!(
        rotations.len(),
        1,
        "hover outline should paint under the hovered frame's rotation"
    );
    // Frame n1 spans (40, 40, 320, 200) → doc-space center (200, 140).
    let vp = &viewport.viewport;
    let pivot = Point2D::new(
        rect.origin.x + vp.pan_x + 200.0 * vp.zoom,
        rect.origin.y + vp.pan_y + 140.0 * vp.zoom,
    );
    assert!((rotations[0].0 - rotation).abs() < 1e-4);
    assert!(
        (rotations[0].1.x - pivot.x).abs() < 0.5 && (rotations[0].1.y - pivot.y).abs() < 0.5,
        "hover outline must rotate about the frame's own center; got {:?}, want {:?}",
        rotations[0].1,
        pivot
    );
}

#[test]
fn hover_outline_on_child_applies_ancestor_rotation() {
    let _guard = crate::agent_indicator_test_support::lock();
    op_editor_core::agent_indicators::clear();
    let mut scene = sample_scene();
    let rotation = 0.5_f32;
    scene.pages[0].children[0].rotation = rotation;
    let state = EditorState::new();
    let mut viewport = CanvasViewport::from_editor(&state, &scene);
    // n2 is an unrotated child of the rotated frame n1.
    viewport.hovered = Some("n2".into());
    let mut backend = RecordingBackend::default();
    let rect = Rect::xywh(0.0, 0.0, 800.0, 600.0);
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, rect);
    }

    let rotations = active_rotations_at_first_stroke(&backend, HOVER_OUTLINE_COLOR)
        .expect("hover outline should paint dashed strokes");
    assert_eq!(
        rotations.len(),
        1,
        "child hover outline should inherit the rotated ancestor's transform"
    );
    // The pivot is the rotated PARENT's center, not the child's.
    let vp = &viewport.viewport;
    let pivot = Point2D::new(
        rect.origin.x + vp.pan_x + 200.0 * vp.zoom,
        rect.origin.y + vp.pan_y + 140.0 * vp.zoom,
    );
    assert!((rotations[0].0 - rotation).abs() < 1e-4);
    assert!(
        (rotations[0].1.x - pivot.x).abs() < 0.5 && (rotations[0].1.y - pivot.y).abs() < 0.5,
        "child hover outline must rotate about the ancestor's pivot; got {:?}, want {:?}",
        rotations[0].1,
        pivot
    );
}

#[test]
fn direct_child_hover_hint_replays_parent_and_child_rotation_flip_chain() {
    let _guard = crate::agent_indicator_test_support::lock();
    op_editor_core::agent_indicators::clear();
    let mut scene = hover_hierarchy_scene();
    let focus = &mut scene.pages[0].children[0];
    let rotation = 0.4_f32;
    focus.rotation = rotation;
    focus.flip_y = true;
    // Children paint in reverse order; the hidden child is skipped, so
    // direct-b supplies the first dashed hierarchy stroke.
    focus.children[1].flip_x = true;

    let mut state = EditorState::new();
    state.editor_ui.canvas_hover_node = Some(op_editor_core::NodeId::new("focus"));
    let viewport = CanvasViewport::from_editor(&state, &scene);
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 300.0, 240.0));
    }

    let transforms = active_transforms_at_first_line_stroke(&backend, HOVER_OUTLINE_COLOR)
        .expect("a direct child should paint dashed hierarchy strokes");
    assert_eq!(
        transforms.rotations.len(),
        1,
        "the child hint should inherit the focus rotation once"
    );
    assert!((transforms.rotations[0].0 - rotation).abs() < 1e-4);
    assert_eq!(
        transforms.scales.len(),
        2,
        "the child hint should replay both the focus and child flips"
    );
    assert_eq!(transforms.scales[0].0, Point2D::new(1.0, -1.0));
    assert_eq!(transforms.scales[1].0, Point2D::new(-1.0, 1.0));
}

#[test]
fn selection_overlay_on_child_applies_ancestor_rotation() {
    let _guard = crate::agent_indicator_test_support::lock();
    op_editor_core::agent_indicators::clear();
    let mut scene = sample_scene();
    let rotation = 0.5_f32;
    scene.pages[0].children[0].rotation = rotation;
    let mut state = EditorState::new();
    state.set_single_selection(op_editor_core::NodeId::new("n2"));
    let viewport = CanvasViewport::from_editor(&state, &scene);
    let mut backend = RecordingBackend::default();
    let rect = Rect::xywh(0.0, 0.0, 800.0, 600.0);
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, rect);
    }

    let rotations = active_rotations_at_first_stroke(&backend, viewport.theme.primary)
        .expect("selected child overlay should paint primary strokes");
    assert!(
        rotations.iter().any(|(r, _)| (*r - rotation).abs() < 1e-4),
        "selection overlay should inherit the rotated ancestor transform; got {rotations:?}"
    );
}

#[test]
fn multi_selection_overlay_on_children_applies_ancestor_rotation() {
    let _guard = crate::agent_indicator_test_support::lock();
    op_editor_core::agent_indicators::clear();
    let mut scene = sample_scene();
    let rotation = 0.5_f32;
    scene.pages[0].children[0].rotation = rotation;
    let mut state = EditorState::new();
    state.selection.set = vec![
        op_editor_core::NodeId::new("n2"),
        op_editor_core::NodeId::new("n3"),
    ];
    state.selection.anchor = op_editor_core::NodeId::new("n2");
    let viewport = CanvasViewport::from_editor(&state, &scene);
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 800.0, 600.0));
    }

    let rotations = active_rotations_at_first_stroke(&backend, viewport.theme.primary)
        .expect("multi-selection overlay should paint primary strokes");
    assert!(
        rotations.iter().any(|(r, _)| (*r - rotation).abs() < 1e-4),
        "multi-selection overlay should inherit the rotated ancestor transform; got {rotations:?}"
    );
}

#[test]
fn path_editor_overlay_on_child_applies_ancestor_rotation() {
    let _guard = crate::agent_indicator_test_support::lock();
    op_editor_core::agent_indicators::clear();
    let mut path = SceneNode::leaf("editing-path", NodeKind::Path);
    path.bounds = Rect::xywh(60.0, 80.0, 120.0, 40.0);
    path.points = vec![Point2D::new(60.0, 80.0), Point2D::new(180.0, 120.0)];
    path.path_anchors = vec![
        SceneAnchor {
            pos: Point2D::new(60.0, 80.0),
            handle_in: None,
            handle_out: None,
            point_type: ScenePointType::Corner,
        },
        SceneAnchor {
            pos: Point2D::new(180.0, 120.0),
            handle_in: None,
            handle_out: None,
            point_type: ScenePointType::Corner,
        },
    ];
    let rotation = 0.5_f32;
    let mut frame = SceneNode::leaf("rotated-frame", NodeKind::Frame);
    frame.bounds = Rect::xywh(40.0, 40.0, 320.0, 200.0);
    frame.rotation = rotation;
    frame.children = vec![path];
    let scene = LayoutScene {
        pages: vec![ScenePage {
            id: "p".into(),
            name: "p".into(),
            children: vec![frame],
        }],
        active_page_index: 0,
    };
    let mut state = EditorState::new();
    state.set_single_selection(op_editor_core::NodeId::new("editing-path"));
    let mut viewport = CanvasViewport::from_editor(&state, &scene);
    viewport.tool = op_editor_core::Tool::Select;
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 800.0, 600.0));
    }

    let rotations = active_rotations_at_first_oval_stroke(&backend, SELECTION_BLUE)
        .expect("path editor should paint selection-blue anchor strokes");
    assert!(
        rotations.iter().any(|(r, _)| (*r - rotation).abs() < 1e-4),
        "path editor overlay should inherit the rotated ancestor transform; got {rotations:?}"
    );
}

#[test]
fn arc_handles_on_child_apply_ancestor_rotation() {
    let _guard = crate::agent_indicator_test_support::lock();
    op_editor_core::agent_indicators::clear();
    let mut ellipse = SceneNode::leaf("arc-ellipse", NodeKind::Ellipse);
    ellipse.bounds = Rect::xywh(60.0, 80.0, 120.0, 80.0);
    ellipse.arc_start_angle = Some(0.0);
    ellipse.arc_sweep_angle = Some(90.0);
    ellipse.arc_inner_radius = Some(0.5);
    let rotation = 0.5_f32;
    let mut frame = SceneNode::leaf("rotated-frame", NodeKind::Frame);
    frame.bounds = Rect::xywh(40.0, 40.0, 320.0, 200.0);
    frame.rotation = rotation;
    frame.children = vec![ellipse];
    let scene = LayoutScene {
        pages: vec![ScenePage {
            id: "p".into(),
            name: "p".into(),
            children: vec![frame],
        }],
        active_page_index: 0,
    };
    let mut state = EditorState::new();
    state.set_single_selection(op_editor_core::NodeId::new("arc-ellipse"));
    let mut viewport = CanvasViewport::from_editor(&state, &scene);
    viewport.tool = op_editor_core::Tool::Select;
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 800.0, 600.0));
    }

    let rotations = active_rotations_at_first_oval_stroke(&backend, viewport.theme.background)
        .expect("arc handles should paint oval strokes");
    assert!(
        rotations.iter().any(|(r, _)| (*r - rotation).abs() < 1e-4),
        "arc handles should inherit the rotated ancestor transform; got {rotations:?}"
    );
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

#[test]
fn shader_fill_dispatches_through_the_shader_backend_method() {
    use crate::layout_scene::{SceneShader, SceneShaderUniform};
    let mut node = SceneNode::leaf("s1", NodeKind::Rect);
    node.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
    node.fill = Some(Color::WHITE); // fallback colour the builder bakes in
    node.fill_type = SceneFillType::Shader;
    node.shader = Some(SceneShader {
        sksl: "half4 main(float2 p){ return half4(1.0,0.0,0.0,1.0); }".to_string(),
        uniforms: vec![SceneShaderUniform {
            name: "u_mix".to_string(),
            values: vec![0.5],
        }],
        opacity: 1.0,
        fallback: Color::WHITE,
    });

    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        crate::widgets::canvas_viewport_overlay::paint_fill_then_stroke(
            &mut cx,
            &node,
            node.bounds,
            1.0,
            node.fill,
        );
    }
    assert_eq!(
        backend.shader_fills, 1,
        "shader body must route through fill_round_rect_shader, not the solid fill"
    );
    assert_eq!(
        backend.rects, 0,
        "no plain-fill fallback when a shader is present"
    );
}

#[test]
fn mesh_gradient_dispatches_through_the_mesh_backend_method() {
    use crate::layout_scene::SceneGradient;
    let mut node = SceneNode::leaf("m1", NodeKind::Rect);
    node.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
    node.fill = Some(Color::WHITE);
    node.fill_type = SceneFillType::MeshGradient;
    node.gradient = Some(SceneGradient::Mesh {
        rows: 2,
        cols: 2,
        colors: vec![Color::WHITE; 4],
        opacity: 1.0,
    });

    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        crate::widgets::canvas_viewport_overlay::paint_fill_then_stroke(
            &mut cx,
            &node,
            node.bounds,
            1.0,
            node.fill,
        );
    }
    assert_eq!(
        backend.mesh_fills, 1,
        "mesh body must route through fill_round_rect_mesh_gradient"
    );
    assert_eq!(
        backend.rects, 0,
        "no flat first-vertex fill at the dispatch layer"
    );
}
