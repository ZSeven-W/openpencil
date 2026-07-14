use super::{paint_node_with_options, paint_node_with_options_hiding, RevealSchedule};
use crate::layout_scene::{NodeKind, SceneNode};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
struct RevealCaptureBackend {
    ops: Vec<String>,
    scales: usize,
    strokes: usize,
    translations: Vec<Point2D>,
}

impl RenderBackend for RevealCaptureBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, rect: Rect, _: Color) {
        self.ops
            .push(format!("fill({},{})", rect.origin.x, rect.origin.y));
    }
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {
        self.strokes += 1;
    }
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
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {
        self.strokes += 1;
    }
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

fn paint_with_generation_scan(
    node: &SceneNode,
    reveals: &HashMap<String, u64>,
    now_ms: u64,
    scan: &HashSet<String>,
) -> RevealCaptureBackend {
    let mut backend = RevealCaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    let _ = paint_node_with_options_hiding(
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
        None,
        now_ms,
        Some(scan),
        Some(super::super::canvas_generation_scan::SKELETON_BLUE),
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
    // 1_700: past c's 260ms wireframe + 180ms scale-pop windows, so this
    // checks the settled paint ops, not either in-flight treatment.
    assert_eq!(
        paint_with_reveals(&frame, &reveals, 1_700).ops,
        vec!["fill(0,0)".to_string(), "fill(10,10)".to_string()]
    );
}

#[test]
fn pending_child_keeps_parent_radar_until_reveal_handoff() {
    let frame = frame_with_child();
    let reveals = HashMap::from([("c".to_string(), 1_350)]);
    let scan = HashSet::from(["f".to_string()]);

    let pending = paint_with_generation_scan(&frame, &reveals, 1_000, &scan);
    assert!(
        pending
            .ops
            .iter()
            .filter(|op| op.starts_with("fill"))
            .count()
            > 20,
        "the existing shell keeps a visible radar band during the reveal runway"
    );
    assert!(pending.ops.contains(&"save".to_string()));

    let handed_off = paint_with_generation_scan(&frame, &reveals, 1_350, &scan);
    assert_eq!(
        handed_off.ops,
        vec!["fill(0,0)".to_string(), "fill(10,10)".to_string()],
        "at the child's start the parent radar stops and the child wireframe begins"
    );
    assert_eq!(handed_off.strokes, 1, "the child is still a wireframe");
}

#[test]
fn active_reveal_paints_content_without_transform() {
    let mut node = SceneNode::leaf("c", NodeKind::Rect);
    node.bounds = Rect::xywh(10.0, 10.0, 50.0, 30.0);
    node.fill = Some(Color::RED);
    let reveals = HashMap::from([("c".to_string(), 1_000)]);

    // 1_600: past the wireframe-ghost beat (260ms) AND the 180ms scale-pop
    // window, so the pop has settled and steady-state paint has no transform.
    let backend = paint_with_reveals(&node, &reveals, 1_600);

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

    let backend = paint_with_reveals(&node, &reveals, 1_260);

    // elapsed_ms == REVEAL_WIREFRAME_MS is exactly when the scale-pop
    // begins (the wireframe ghost hands off to the pop at this instant),
    // so a save/scale now legitimately wraps the fill; the authored
    // (10,10) position painted underneath is still unshifted.
    assert_eq!(backend.scales, 1);
    assert!(backend.translations.is_empty());
    assert_eq!(
        backend.ops,
        vec![
            "save".to_string(),
            "scale".to_string(),
            "fill(10,10)".to_string(),
            "restore".to_string(),
        ]
    );
}

#[test]
fn opening_parent_and_child_reveals_each_pop_independently() {
    let frame = frame_with_child();
    let reveals = HashMap::from([("f".to_string(), 1_000), ("c".to_string(), 1_040)]);

    let backend = paint_with_reveals(&frame, &reveals, 1_300);

    // Parent (elapsed 300ms) and child (elapsed 260ms) are each past their
    // wireframe-ghost beat and independently inside their own 180ms
    // scale-pop window, so both legitimately apply save/scale/restore —
    // nested pop-stacking is the per-node design, not something the paint
    // layer suppresses.
    assert_eq!(backend.scales, 2);
    assert!(
        backend.ops.contains(&"fill(10,10)".to_string()),
        "started child should still paint inside the parent reveal"
    );
}

#[test]
fn overlapping_parent_and_child_reveals_pop_through_opening_beat() {
    let frame = frame_with_child();
    let reveals = HashMap::from([("f".to_string(), 1_000), ("c".to_string(), 1_048)]);

    let backend = paint_with_reveals(&frame, &reveals, 1_316);

    // Parent (elapsed 316ms) and child (elapsed 268ms) are both past the
    // ghost beat and still inside their own 180ms scale-pop windows — see
    // opening_parent_and_child_reveals_each_pop_independently above.
    assert_eq!(
        backend.scales, 2,
        "each in-window reveal applies its own pop"
    );
}

#[test]
fn child_pops_alone_after_parent_opening_beat() {
    let frame = frame_with_child();
    let reveals = HashMap::from([("f".to_string(), 1_000), ("c".to_string(), 1_080)]);

    let backend = paint_with_reveals(&frame, &reveals, 1_460);

    // Parent's ghost+pop have settled (elapsed 460ms >= 260+180), but the
    // child (elapsed 380ms) is legitimately inside ITS own pop window —
    // one scale, from the child alone.
    assert_eq!(
        backend.scales, 1,
        "child pops on its own independent window"
    );
    assert!(backend.ops.contains(&"fill(10,10)".to_string()));
}

#[test]
fn delayed_child_pops_alone_after_parent_settles() {
    let frame = frame_with_child();
    let reveals = HashMap::from([("f".to_string(), 1_000), ("c".to_string(), 1_420)]);

    let backend = paint_with_reveals(&frame, &reveals, 1_780);

    // Parent settled long ago (elapsed 780ms); the child (elapsed 360ms)
    // is past its ghost beat and pops on its own independent window.
    assert_eq!(
        backend.scales, 1,
        "child pops on its own independent window"
    );
    assert!(backend.ops.contains(&"fill(10,10)".to_string()));
}
