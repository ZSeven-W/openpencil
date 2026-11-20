use super::{paint_node_with_options, paint_scene_nodes_with_options_hiding};
use crate::layout_scene::{BlurEffect, Effect, MaskType, NodeKind, SceneNode};
use crate::widgets::PaintCx;
use crate::{Color, ImageBlendMode, Point2D, Rect, RenderBackend, TextLayout};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Op {
    Save,
    Clip,
    Backdrop,
    Composite(f32, ImageBlendMode),
    Blur,
    MaskSource,
    Fill(f32, f32),
    Svg,
    Restore,
}

#[derive(Default)]
struct CaptureBackend {
    ops: Vec<Op>,
    composite_bounds: Vec<Rect>,
}

impl RenderBackend for CaptureBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.ops.push(Op::Fill(rect.origin.x, color.a));
    }
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {
        self.ops.push(Op::Clip);
    }
    fn clip_round_rect(&mut self, _: Rect, _: f32) {
        self.ops.push(Op::Clip);
    }
    fn save(&mut self) {
        self.ops.push(Op::Save);
    }
    fn restore(&mut self) {
        self.ops.push(Op::Restore);
    }
    fn translate(&mut self, _: Point2D) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, rect: Rect, _: f32, color: Color) {
        self.fill_rect(rect, color);
    }
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn fill_svg_path_in_rect(&mut self, _: &str, _: Rect, _: Color) {
        self.ops.push(Op::Svg);
    }
    fn push_backdrop_blur_layer(&mut self, _: f32) {
        self.ops.push(Op::Backdrop);
    }
    fn push_blur_layer(&mut self, _: f32) {
        self.ops.push(Op::Blur);
    }
    fn push_composite_layer(&mut self, bounds: Rect, opacity: f32, mode: ImageBlendMode) {
        self.composite_bounds.push(bounds);
        self.ops.push(Op::Composite(opacity, mode));
    }
    fn supports_pixel_masks(&self) -> bool {
        true
    }
    fn push_mask_source_layer(&mut self, _: bool) {
        self.ops.push(Op::MaskSource);
    }
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

fn rect(id: &str, x: f32) -> SceneNode {
    let mut node = SceneNode::leaf(id, NodeKind::Rect);
    node.bounds = Rect::xywh(x, 0.0, 10.0, 10.0);
    node.fill = Some(Color::RED);
    node
}

fn capture(node: &SceneNode, cull: Rect) -> CaptureBackend {
    let mut backend = CaptureBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        let _ = paint_node_with_options(
            &mut cx,
            node,
            Point2D::ZERO,
            1.0,
            None,
            cull,
            None,
            None,
            None,
            None,
        );
    }
    backend
}

fn paint(node: &SceneNode) -> Vec<Op> {
    capture(node, Rect::xywh(-100.0, -100.0, 1000.0, 1000.0)).ops
}

#[test]
fn inactive_composed_subtree_issues_no_draw_or_layer_operations() {
    let mut frame = rect("overlay", 0.0);
    frame.kind = NodeKind::Frame;
    frame.children = vec![rect("headline", 5.0)];
    frame.composite_opacity = 0.0;
    assert!(paint(&frame).is_empty());

    frame.composite_opacity = 0.25;
    assert!(paint(&frame).contains(&Op::Composite(0.25, ImageBlendMode::Normal)));
}

#[test]
fn container_blend_layer_wraps_own_paint_and_complete_subtree() {
    let mut frame = rect("frame", 0.0);
    frame.kind = NodeKind::Frame;
    frame.blend_mode = ImageBlendMode::Multiply;
    frame.children = vec![rect("front", 20.0), rect("back", 10.0)];
    assert_eq!(
        paint(&frame),
        vec![
            Op::Composite(1.0, ImageBlendMode::Multiply),
            Op::Fill(0.0, 1.0),
            Op::Fill(10.0, 1.0),
            Op::Fill(20.0, 1.0),
            Op::Restore,
        ]
    );
}

#[test]
fn node_blend_save_layer_uses_effect_aware_subtree_not_viewport() {
    let mut frame = rect("frame", 100.0);
    frame.kind = NodeKind::Frame;
    frame.bounds = Rect::xywh(100.0, 50.0, 20.0, 20.0);
    frame.blend_mode = ImageBlendMode::Multiply;
    let mut child = rect("overflow", 160.0);
    child.bounds = Rect::xywh(160.0, 40.0, 10.0, 10.0);
    frame.children = vec![child];

    let cull = Rect::xywh(-1_000.0, -1_000.0, 4_000.0, 4_000.0);
    let capture = capture(&frame, cull);
    assert_eq!(
        capture.composite_bounds,
        vec![Rect::xywh(99.0, 39.0, 72.0, 32.0)]
    );
    assert_ne!(capture.composite_bounds[0], cull);
}

#[test]
fn node_blend_save_layer_intersects_visible_cull() {
    let mut node = rect("edge", 90.0);
    node.bounds = Rect::xywh(90.0, 90.0, 30.0, 30.0);
    node.blend_mode = ImageBlendMode::Screen;
    let cull = Rect::xywh(100.0, 100.0, 10.0, 10.0);
    let capture = capture(&node, cull);
    assert_eq!(capture.composite_bounds, vec![cull]);
}

#[test]
fn svg_path_early_return_balances_the_node_blend_layer() {
    let mut path = SceneNode::leaf("path", NodeKind::Path);
    path.bounds = Rect::xywh(0.0, 0.0, 10.0, 10.0);
    path.fill = Some(Color::RED);
    path.svg_path = Some("M0 0H10V10H0Z".into());
    path.blend_mode = ImageBlendMode::Overlay;
    assert_eq!(
        paint(&path),
        vec![
            Op::Composite(1.0, ImageBlendMode::Overlay),
            Op::Svg,
            Op::Restore,
        ]
    );
}

#[test]
fn node_blend_layer_applies_local_opacity_once() {
    let mut node = rect("half", 0.0);
    node.composite_opacity = 0.5;
    node.blend_mode = ImageBlendMode::SoftLight;
    assert_eq!(
        paint(&node),
        vec![
            Op::Composite(0.5, ImageBlendMode::SoftLight),
            Op::Fill(0.0, 1.0),
            Op::Restore,
        ]
    );
}

#[test]
fn translucent_normal_leaf_keeps_the_direct_paint_fast_path() {
    let mut node = rect("half-leaf", 0.0);
    node.opacity = 0.5;
    node.fill = Some(Color::RED.with_alpha(0.5));
    assert_eq!(paint(&node), vec![Op::Fill(0.0, 0.5)]);
}

#[test]
fn overlapping_children_share_one_container_opacity_layer() {
    let mut frame = SceneNode::leaf("half-frame", NodeKind::Frame);
    frame.composite_opacity = 0.5;
    // Same x means these opaque children overlap completely. Both stay opaque
    // inside the layer; the frame alpha is applied once to the assembled result.
    frame.children = vec![rect("front", 10.0), rect("back", 10.0)];
    assert_eq!(
        paint(&frame),
        vec![
            Op::Composite(0.5, ImageBlendMode::Normal),
            Op::Fill(10.0, 1.0),
            Op::Fill(10.0, 1.0),
            Op::Restore,
        ]
    );
}

#[test]
fn layer_order_is_background_blur_then_node_blend_then_layer_blur() {
    let mut node = rect("glass", 0.0);
    node.blend_mode = ImageBlendMode::ColorDodge;
    node.effects = vec![
        Effect::BackgroundBlur { radius: 8.0 },
        Effect::Blur(BlurEffect { radius: 4.0 }),
    ];
    assert_eq!(
        paint(&node),
        vec![
            Op::Save,
            Op::Clip,
            Op::Backdrop,
            Op::Composite(1.0, ImageBlendMode::ColorDodge),
            Op::Blur,
            Op::Fill(0.0, 1.0),
            Op::Restore,
            Op::Restore,
            Op::Restore,
            Op::Restore,
        ]
    );
}

#[test]
fn deferred_mask_suppresses_only_root_blend_not_descendant_blend() {
    let content = rect("content", 20.0);
    let mut mask = rect("mask", 0.0);
    mask.mask_type = Some(MaskType::Alpha);
    mask.is_mask = true;
    mask.blend_mode = ImageBlendMode::Multiply;
    mask.composite_opacity = 0.5;
    let mut child = rect("mask-child", 10.0);
    child.blend_mode = ImageBlendMode::Screen;
    mask.children = vec![child];

    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    let _ = paint_scene_nodes_with_options_hiding(
        &mut cx,
        &[content, mask],
        Point2D::ZERO,
        1.0,
        None,
        Rect::xywh(-100.0, -100.0, 1000.0, 1000.0),
        None,
        None,
        None,
        None,
        None,
        0,
        None,
        None,
        None,
        false,
    );

    assert!(backend
        .ops
        .contains(&Op::Composite(1.0, ImageBlendMode::Screen)));
    assert!(backend
        .ops
        .contains(&Op::Composite(0.5, ImageBlendMode::Normal)));
    assert!(!backend
        .ops
        .contains(&Op::Composite(1.0, ImageBlendMode::Multiply)));
    assert_eq!(
        backend
            .ops
            .iter()
            .filter(|op| matches!(op, Op::MaskSource))
            .count(),
        1
    );
}
