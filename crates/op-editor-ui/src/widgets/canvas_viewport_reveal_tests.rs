use super::{paint_node_with_options, RevealSchedule};
use crate::layout_scene::{NodeKind, SceneNode};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use std::collections::HashMap;

#[derive(Default)]
struct RevealCaptureBackend {
    ops: Vec<String>,
    scales: usize,
    translations: Vec<Point2D>,
}

impl RenderBackend for RevealCaptureBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, rect: Rect, _: Color) {
        self.ops
            .push(format!("fill({},{})", rect.origin.x, rect.origin.y));
    }
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn save(&mut self) {
        self.ops.push("save".into());
    }
    fn restore(&mut self) {
        self.ops.push("restore".into());
    }
    fn translate(&mut self, delta: Point2D) {
        self.translations.push(delta);
    }
    fn scale(&mut self, factor: Point2D, _: Point2D) {
        let _ = factor;
        self.scales += 1;
        self.ops.push("scale".into());
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

fn frame_with_child() -> SceneNode {
    let mut child = SceneNode::leaf("c", NodeKind::Rect);
    child.bounds = Rect::xywh(10.0, 10.0, 500.0, 20.0);
    child.fill = Some(Color::RED);
    let mut frame = SceneNode::leaf("f", NodeKind::Frame);
    frame.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
    frame.fill = Some(Color::WHITE);
    frame.children = vec![child];
    frame
}

fn paint_with_reveals(
    node: &SceneNode,
    reveals: &HashMap<String, u64>,
    now_ms: u64,
) -> RevealCaptureBackend {
    let mut backend = RevealCaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    let _ = paint_node_with_options(
        &mut cx,
        node,
        Point2D::ZERO,
        1.0,
        None,
        Rect::xywh(0.0, 0.0, 4000.0, 4000.0),
        Some(RevealSchedule {
            starts: reveals,
            now_ms,
        }),
        None,
        None,
        None,
    );
    backend
}

#[test]
fn future_reveal_child_waits_before_painting() {
    let frame = frame_with_child();
    let reveals = HashMap::from([("c".to_string(), 1_200)]);

    assert_eq!(
        paint_with_reveals(&frame, &reveals, 1_000).ops,
        vec!["fill(0,0)".to_string()]
    );
    assert_eq!(
        paint_with_reveals(&frame, &reveals, 1_200).ops,
        vec!["fill(0,0)".to_string(), "fill(10,10)".to_string()]
    );
}

#[test]
fn active_reveal_paints_content_without_transform() {
    let mut node = SceneNode::leaf("c", NodeKind::Rect);
    node.bounds = Rect::xywh(10.0, 10.0, 50.0, 30.0);
    node.fill = Some(Color::RED);
    let reveals = HashMap::from([("c".to_string(), 1_000)]);

    let backend = paint_with_reveals(&node, &reveals, 1_120);

    assert_eq!(backend.scales, 0, "TS Skia reveal does not scale content");
    assert!(
        backend.translations.is_empty(),
        "TS Skia reveal does not lift content"
    );
    assert_eq!(backend.ops, vec!["fill(10,10)".to_string()]);
}

#[test]
fn active_reveal_starts_at_authored_position() {
    let mut node = SceneNode::leaf("c", NodeKind::Rect);
    node.bounds = Rect::xywh(10.0, 10.0, 50.0, 30.0);
    node.fill = Some(Color::RED);
    let reveals = HashMap::from([("c".to_string(), 1_000)]);

    let backend = paint_with_reveals(&node, &reveals, 1_000);

    assert_eq!(backend.scales, 0);
    assert!(backend.translations.is_empty());
    assert_eq!(backend.ops, vec!["fill(10,10)".to_string()]);
}

#[test]
fn opening_parent_reveal_prevents_nested_child_transform() {
    let frame = frame_with_child();
    let reveals = HashMap::from([("f".to_string(), 1_000), ("c".to_string(), 1_040)]);

    let backend = paint_with_reveals(&frame, &reveals, 1_040);

    assert_eq!(backend.scales, 0, "reveal should not transform content");
    assert!(
        backend.ops.contains(&"fill(10,10)".to_string()),
        "started child should still paint inside the parent reveal"
    );
}

#[test]
fn parent_reveal_suppresses_child_transform_through_opening_beat() {
    let frame = frame_with_child();
    let reveals = HashMap::from([("f".to_string(), 1_000), ("c".to_string(), 1_048)]);

    let backend = paint_with_reveals(&frame, &reveals, 1_056);

    assert_eq!(backend.scales, 0, "reveal should not transform content");
}

#[test]
fn child_reveal_paints_after_parent_opening_beat_without_transform() {
    let frame = frame_with_child();
    let reveals = HashMap::from([("f".to_string(), 1_000), ("c".to_string(), 1_080)]);

    let backend = paint_with_reveals(&frame, &reveals, 1_180);

    assert_eq!(backend.scales, 0, "reveal should not transform content");
    assert!(backend.ops.contains(&"fill(10,10)".to_string()));
}

#[test]
fn delayed_child_reveal_paints_after_parent_settles_without_transform() {
    let frame = frame_with_child();
    let reveals = HashMap::from([("f".to_string(), 1_000), ("c".to_string(), 1_420)]);

    let backend = paint_with_reveals(&frame, &reveals, 1_520);

    assert_eq!(backend.scales, 0, "reveal should not transform content");
    assert!(backend.ops.contains(&"fill(10,10)".to_string()));
}
