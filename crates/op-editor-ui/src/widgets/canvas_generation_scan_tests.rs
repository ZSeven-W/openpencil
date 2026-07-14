use super::canvas_generation_scan::{
    generating_paint_sets, is_pending_filled_section, is_placeholder_section,
    paint_generation_scan, paint_queued_skeleton, scan_phase, SKELETON_BLUE,
};
use crate::layout_scene::{NodeKind, SceneNode};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use op_editor_core::agent_indicators::{AgentIndicators, AgentTag};

fn visual_rect(id: &str) -> SceneNode {
    let mut node = SceneNode::leaf(id, NodeKind::Rect);
    node.bounds = Rect::xywh(0.0, 0.0, 120.0, 80.0);
    node.fill = Some(Color::BLACK);
    node
}

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
fn filled_shell_stays_visually_empty_until_its_first_child_reveal() {
    let mut shell = SceneNode::leaf("shell", NodeKind::Frame);
    shell.children.push(visual_rect("future-content"));
    let reveals = std::collections::HashMap::from([("future-content".into(), 1_350)]);

    assert!(is_pending_filled_section(&shell, &reveals, 1_000));
    assert!(
        !is_pending_filled_section(&shell, &reveals, 1_350),
        "the radar hands off exactly when the first child starts its reveal"
    );
}

#[test]
fn transparent_wrappers_can_lead_to_a_future_reveal() {
    let mut wrapper = SceneNode::leaf("wrapper", NodeKind::Group);
    wrapper.children.push(visual_rect("future-content"));
    let mut shell = SceneNode::leaf("shell", NodeKind::Frame);
    shell.children.push(wrapper);
    let reveals = std::collections::HashMap::from([
        ("wrapper".into(), 1_100),
        ("future-content".into(), 1_350),
    ]);

    assert!(
        is_pending_filled_section(&shell, &reveals, 1_200),
        "a zero-bounds wrapper cannot take the visual handoff from the radar"
    );
    assert!(!is_pending_filled_section(&shell, &reveals, 1_350));
}

#[test]
fn bounded_unscheduled_transparent_containers_recurse_to_pending_content() {
    for (index, kind) in [NodeKind::Frame, NodeKind::Group, NodeKind::Rect]
        .into_iter()
        .enumerate()
    {
        let mut wrapper = SceneNode::leaf(format!("wrapper-{index}"), kind);
        wrapper.bounds = Rect::xywh(0.0, 0.0, 120.0, 80.0);
        wrapper.children.push(visual_rect("future-content"));
        let mut shell = SceneNode::leaf("shell", NodeKind::Frame);
        shell.children.push(wrapper);
        let reveals = std::collections::HashMap::from([("future-content".into(), 1_350)]);

        assert!(
            is_pending_filled_section(&shell, &reveals, 1_000),
            "bounded transparent containers are layout, not visible content"
        );
    }
}

#[test]
fn empty_transparent_wrapper_keeps_its_parent_shell_visually_empty() {
    let mut shell = SceneNode::leaf("shell", NodeKind::Frame);
    let mut wrapper = SceneNode::leaf("empty-wrapper", NodeKind::Frame);
    wrapper.bounds = Rect::xywh(0.0, 0.0, 120.0, 80.0);
    shell.children.push(wrapper);

    assert!(is_pending_filled_section(
        &shell,
        &std::collections::HashMap::new(),
        1_000
    ));
}

#[test]
fn started_zero_bounds_wrapper_with_empty_subtree_keeps_the_radar() {
    let mut shell = SceneNode::leaf("shell", NodeKind::Frame);
    shell
        .children
        .push(SceneNode::leaf("empty-wrapper", NodeKind::Group));
    let reveals = std::collections::HashMap::from([("empty-wrapper".into(), 1_000)]);

    assert!(
        is_pending_filled_section(&shell, &reveals, 1_000),
        "a zero-bounds reveal has no wireframe or child pixels to take the handoff"
    );
}

#[test]
fn future_zero_bounds_wrapper_gates_an_existing_visible_descendant() {
    let mut wrapper = SceneNode::leaf("wrapper", NodeKind::Group);
    wrapper.children.push(visual_rect("already-visible"));
    let mut shell = SceneNode::leaf("shell", NodeKind::Frame);
    shell.children.push(wrapper);
    let reveals = std::collections::HashMap::from([("wrapper".into(), 1_350)]);

    assert!(
        is_pending_filled_section(&shell, &reveals, 1_000),
        "a future wrapper reveal hides its whole subtree"
    );
    assert!(
        !is_pending_filled_section(&shell, &reveals, 1_350),
        "a zero-bounds wrapper hands off to its visible child once it starts"
    );
}

#[test]
fn empty_transparent_branches_do_not_block_a_pending_visual_child() {
    let mut shell = SceneNode::leaf("shell", NodeKind::Frame);
    shell.children = vec![
        SceneNode::leaf("empty-wrapper", NodeKind::Group),
        visual_rect("future-content"),
    ];
    let reveals = std::collections::HashMap::from([("future-content".into(), 1_350)]);

    assert!(is_pending_filled_section(&shell, &reveals, 1_000));
}

#[test]
fn mixed_visible_and_pending_content_never_reactivates_the_shell_radar() {
    let mut shell = SceneNode::leaf("shell", NodeKind::Frame);
    shell.children = vec![
        visual_rect("already-visible"),
        visual_rect("future-content"),
    ];
    let reveals = std::collections::HashMap::from([("future-content".into(), 1_350)]);

    assert!(!is_pending_filled_section(&shell, &reveals, 1_000));
}

#[test]
fn pending_filled_shell_consumes_the_global_deck_until_handoff() {
    let mut shell = SceneNode::leaf("shell", NodeKind::Frame);
    shell.children.push(visual_rect("future-content"));
    let later = SceneNode::leaf("later-shell", NodeKind::Frame);
    let mut root = SceneNode::leaf("root", NodeKind::Frame);
    root.children = vec![shell, later];

    let mut indicators = AgentIndicators::default();
    indicators.run_active = true;
    indicators.frames.insert(
        "root".into(),
        AgentTag {
            color: "#4ECDC4".into(),
            name: "Mochi".into(),
        },
    );
    indicators.reveals.insert("future-content".into(), 1_350);

    let pending = generating_paint_sets(&[root.clone()], Some(&indicators), 1_000).unwrap();
    assert!(pending.scan.contains("shell"));
    assert!(pending.queued.contains("later-shell"));

    let handed_off = generating_paint_sets(&[root], Some(&indicators), 1_350).unwrap();
    assert!(handed_off.scan.contains("later-shell"));
    assert!(!handed_off.queued.contains("later-shell"));
}

#[test]
fn finishing_fast_batch_keeps_scan_until_its_first_future_reveal() {
    let mut shell = SceneNode::leaf("shell", NodeKind::Frame);
    shell.children.push(visual_rect("future-content"));
    let mut root = SceneNode::leaf("root", NodeKind::Frame);
    root.children.push(shell);

    let mut indicators = AgentIndicators::default();
    indicators.frames.insert(
        "root".into(),
        AgentTag {
            color: "#4ECDC4".into(),
            name: "Mochi".into(),
        },
    );
    indicators.reveals.insert("future-content".into(), 1_350);

    let pending = generating_paint_sets(&[root.clone()], Some(&indicators), 1_000)
        .expect("a graceful finish retains the existing reveal runway");
    assert!(pending.scan.contains("shell"));
    assert!(
        generating_paint_sets(&[root], Some(&indicators), 1_350).is_none(),
        "at the last reveal start the wireframe owns the handoff"
    );
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
    assert!(generating_paint_sets(&[root.clone()], Some(&indicators), 1_000).is_none());

    indicators.run_active = true;
    indicators.frames.insert(
        "root".into(),
        AgentTag {
            color: "#4ECDC4".into(),
            name: "Mochi".into(),
        },
    );
    let ids = generating_paint_sets(&[root], Some(&indicators), 1_000)
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
