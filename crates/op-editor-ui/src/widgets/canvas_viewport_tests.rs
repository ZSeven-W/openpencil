//! Sibling test file for `canvas_viewport.rs` (800-line cap convention).
//!
//! This spine holds the shared test fixtures — the recording
//! `RenderBackend`, the scene/state builders and the transform-replay
//! helpers — and declares the per-behaviour test modules that were
//! split out into the sibling `canvas_viewport_tests/` directory.

use super::*;
use crate::layout_scene::{
    LayoutScene, NodeKind, SceneAnchor, SceneFillType, SceneNode, ScenePage, ScenePointType,
    SceneStroke, SceneStrokeAlign,
};
use crate::{Color, Point2D, Rect, TextLayout};
use jian_ops_schema::node::{ContainerProps, FrameNode, PenNode, PenNodeBase, RectangleNode};
use jian_ops_schema::page::PenPage;
use jian_ops_schema::sizing::SizingBehavior;
use op_editor_core::EditorState;
use std::collections::HashMap;

#[path = "canvas_viewport_tests/frame_labels.rs"]
mod frame_labels;

#[path = "canvas_viewport_tests/selection_overlay.rs"]
mod selection_overlay;

#[path = "canvas_viewport_tests/scene_paint.rs"]
mod scene_paint;

#[path = "canvas_viewport_tests/normal_focus.rs"]
mod normal_focus;

#[path = "canvas_viewport_tests/lookup_traversal.rs"]
mod lookup_traversal;

#[path = "canvas_viewport_tests/hover_hierarchy.rs"]
mod hover_hierarchy;

#[path = "canvas_viewport_tests/transforms.rs"]
mod transforms;

#[path = "canvas_viewport_tests/hit_tests.rs"]
mod hit_tests;

#[path = "canvas_viewport_tests/backend_dispatch.rs"]
mod backend_dispatch;

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
    fill_colors: Vec<Color>,
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
    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.fill_rects.push(rect);
        self.fill_colors.push(color);
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
        breakpoint: None,
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
