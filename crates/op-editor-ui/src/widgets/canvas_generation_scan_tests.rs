use super::canvas_generation_scan::{
    generating_paint_sets, is_placeholder_section, paint_generation_scan, paint_queued_skeleton,
    scan_phase, SKELETON_BLUE,
};
use crate::layout_scene::{NodeKind, SceneNode};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use op_editor_core::agent_indicators::{AgentIndicators, AgentTag};

#[test]
fn scan_phase_wraps_and_advances_monotonically() {
    let start = scan_phase(0, 1_200);
    assert_eq!(start, scan_phase(1_200, 1_200));
    assert_eq!(start, 0.0);

    let samples = [0, 200, 600, 1_199].map(|now_ms| scan_phase(now_ms, 1_200));
    assert!(samples.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(samples.iter().all(|phase| (0.0..=1.0).contains(phase)));
}

#[test]
fn placeholder_section_requires_an_empty_scene_frame() {
    let empty_frame = SceneNode::leaf("empty", NodeKind::Frame);
    assert!(is_placeholder_section(&empty_frame));

    let mut populated_frame = SceneNode::leaf("populated", NodeKind::Frame);
    populated_frame
        .children
        .push(SceneNode::leaf("content", NodeKind::Rect));
    assert!(!is_placeholder_section(&populated_frame));
}

#[test]
fn non_frame_leaf_is_not_a_placeholder_section() {
    let empty_rect = SceneNode::leaf("rect", NodeKind::Rect);
    assert!(!is_placeholder_section(&empty_rect));
}

#[test]
fn generating_descendants_exclude_the_claimed_root_and_skip_idle_allocation() {
    let mut nested = SceneNode::leaf("section", NodeKind::Frame);
    nested
        .children
        .push(SceneNode::leaf("content", NodeKind::Rect));
    let mut root = SceneNode::leaf("root", NodeKind::Frame);
    root.children.push(nested);

    let mut indicators = AgentIndicators::default();
    assert!(generating_paint_sets(&[root.clone()], Some(&indicators)).is_none());

    indicators.run_active = true;
    indicators.frames.insert(
        "root".into(),
        AgentTag {
            color: "#4ECDC4".into(),
            name: "Mochi".into(),
        },
    );
    let ids = generating_paint_sets(&[root], Some(&indicators))
        .unwrap()
        .scan;
    assert!(!ids.contains("root"));
    assert!(ids.contains("section"));
    assert!(ids.contains("content"));
}

/// Counts what the two skeleton states actually paint. The ACTIVE shell
/// sweeps (many band segments); a QUEUED shell must show its wireframe with
/// no sweep at all — otherwise every queued shell looks like it is being
/// worked, which is the ordering confusion the deck gate exists to prevent.
#[derive(Default)]
struct SkeletonCountBackend {
    fills: usize,
    strokes: usize,
}

impl RenderBackend for SkeletonCountBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {
        self.fills += 1;
    }
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {
        self.strokes += 1;
    }
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {
        self.fills += 1;
    }
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {
        self.strokes += 1;
    }
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn fill_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: f32, _: Color) {}
    fn fill_oval(&mut self, _: Rect, _: Color) {}
    fn stroke_oval(&mut self, _: Rect, _: Color, _: f32) {}
    fn fill_polygon(&mut self, _: &[Point2D], _: Color) {}
    fn stroke_polygon(&mut self, _: &[Point2D], _: Color, _: f32) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

#[test]
fn a_queued_shell_shows_its_wireframe_without_the_sweep() {
    let shell = SceneNode::leaf("shell", NodeKind::Frame);
    let bounds = Rect::xywh(0.0, 0.0, 300.0, 200.0);

    let mut active = SkeletonCountBackend::default();
    paint_generation_scan(
        &mut PaintCx {
            backend: &mut active,
        },
        &shell,
        bounds,
        1.0,
        500,
        SKELETON_BLUE,
    );

    let mut queued = SkeletonCountBackend::default();
    paint_queued_skeleton(
        &mut PaintCx {
            backend: &mut queued,
        },
        &shell,
        bounds,
        1.0,
        SKELETON_BLUE,
    );

    assert_eq!(
        queued.strokes, 1,
        "the queued shell keeps its skeleton outline"
    );
    assert_eq!(queued.fills, 1, "one whisper of wash, no sweep band");
    assert!(
        active.fills > queued.fills + 8,
        "the on-deck shell sweeps a banded gradient ({} fills) while the queue \
         stays still ({} fills)",
        active.fills,
        queued.fills
    );
}
