//! Per-agent "breathing" canvas indicators for the concurrent agent
//! team — a soft glow plus a crisp ring in each agent's colour, pulsing
//! around the root frame that agent is building while a multi-screen
//! generation runs. Reads the process-global
//! [`op_editor_core::agent_indicators`] registry every frame.

use crate::layout_scene::SceneNode;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::agent_indicators::AgentIndicators;
use std::collections::HashMap;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

/// One full breathe (0 → 1 → 0) per this many ms.
const GLOW_PERIOD_MS: u64 = 1200;
const DEFAULT_AGENT_COLOR: Color = Color {
    r: 1.0,
    g: 0.419,
    b: 0.419,
    a: 1.0,
};

#[cfg(test)]
static REVEAL_WALK_VISITS: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static FRAME_LOOKUP_VISITS: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
fn reset_reveal_walk_visits() {
    REVEAL_WALK_VISITS.store(0, Ordering::Relaxed);
}

#[cfg(test)]
fn reveal_walk_visits() -> usize {
    REVEAL_WALK_VISITS.load(Ordering::Relaxed)
}

#[cfg(test)]
fn record_reveal_walk_visit() {
    REVEAL_WALK_VISITS.fetch_add(1, Ordering::Relaxed);
}

#[cfg(test)]
fn reset_frame_lookup_visits() {
    FRAME_LOOKUP_VISITS.store(0, Ordering::Relaxed);
}

#[cfg(test)]
fn frame_lookup_visits() -> usize {
    FRAME_LOOKUP_VISITS.load(Ordering::Relaxed)
}

#[cfg(test)]
fn record_frame_lookup_visit() {
    FRAME_LOOKUP_VISITS.fetch_add(1, Ordering::Relaxed);
}

/// Paint a breathing border around every active-page root frame that an
/// agent currently owns. `roots` are the page's top-level scene nodes;
/// `viewport_origin` already folds in the pan offset.
#[cfg(test)]
pub(crate) fn paint_agent_frame_indicators(
    cx: &mut PaintCx<'_>,
    roots: &[SceneNode],
    viewport_origin: Point2D,
    zoom: f32,
    now_ms: u64,
) {
    let indicators = op_editor_core::agent_indicators::snapshot_at(now_ms);
    paint_agent_frame_indicators_with_snapshot(
        cx,
        roots,
        viewport_origin,
        zoom,
        now_ms,
        &indicators,
    );
}

pub(crate) fn paint_agent_frame_indicators_with_snapshot(
    cx: &mut PaintCx<'_>,
    roots: &[SceneNode],
    viewport_origin: Point2D,
    zoom: f32,
    now_ms: u64,
    indicators: &AgentIndicators,
) {
    if !indicators.reveals.is_empty() {
        paint_node_reveal_indicators(
            cx,
            roots,
            viewport_origin,
            zoom,
            now_ms,
            &indicators.reveals,
            &indicators.nodes,
            &indicators.frames,
        );
    }
    if indicators.frames.is_empty() {
        return;
    }
    // Bell curve 0 → 1 → 0 across the period (matches the TS glow breath).
    let phase = (now_ms % GLOW_PERIOD_MS) as f32 / GLOW_PERIOD_MS as f32;
    let breath = (phase * std::f32::consts::PI).sin();
    for node in roots {
        #[cfg(test)]
        record_frame_lookup_visit();

        let Some(tag) = indicators.frames.get(&node.id) else {
            continue;
        };
        let Some(color) = parse_hex(&tag.color) else {
            continue;
        };
        let b = node.bounds;
        let screen = Rect {
            origin: Point2D::new(
                viewport_origin.x + b.origin.x * zoom,
                viewport_origin.y + b.origin.y * zoom,
            ),
            size: Point2D::new(b.size.x * zoom, b.size.y * zoom),
        };
        // Outer soft glow + inner crisp ring, both breathing.
        cx.backend.stroke_round_rect(
            screen,
            8.0,
            Color {
                a: breath * 0.4,
                ..color
            },
            3.0,
        );
        cx.backend
            .stroke_round_rect(screen, 8.0, Color { a: breath, ..color }, 1.5);
        paint_agent_badge(cx, screen, color, &tag.name, breath);
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_node_reveal_indicators(
    cx: &mut PaintCx<'_>,
    roots: &[SceneNode],
    viewport_origin: Point2D,
    zoom: f32,
    now_ms: u64,
    reveals: &HashMap<String, u64>,
    node_tags: &HashMap<String, op_editor_core::agent_indicators::AgentTag>,
    frame_tags: &HashMap<String, op_editor_core::agent_indicators::AgentTag>,
) {
    if reveals.is_empty() {
        return;
    }
    let mut remaining = reveals.len();
    for root in roots {
        if remaining == 0 {
            break;
        }
        paint_node_reveal_indicator(
            cx,
            root,
            viewport_origin,
            zoom,
            now_ms,
            reveals,
            node_tags,
            frame_tags,
            None,
            false,
            &mut remaining,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_node_reveal_indicator(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    viewport_origin: Point2D,
    zoom: f32,
    now_ms: u64,
    reveals: &HashMap<String, u64>,
    node_tags: &HashMap<String, op_editor_core::agent_indicators::AgentTag>,
    frame_tags: &HashMap<String, op_editor_core::agent_indicators::AgentTag>,
    inherited_agent_color: Option<Color>,
    ancestor_revealing: bool,
    remaining: &mut usize,
) {
    if *remaining == 0 {
        return;
    }

    #[cfg(test)]
    record_reveal_walk_visit();

    let agent_color = node_tags
        .get(&node.id)
        .or_else(|| frame_tags.get(&node.id))
        .and_then(|tag| parse_hex(&tag.color))
        .or(inherited_agent_color);
    let reveal_start = reveals.get(&node.id).copied();
    if reveal_start.is_some() {
        *remaining = remaining.saturating_sub(1);
    }
    let reveal = reveal_start.and_then(|started_at| reveal_breath(started_at, now_ms));
    if let Some(breath) = reveal {
        if !ancestor_revealing {
            let color = agent_color.unwrap_or(DEFAULT_AGENT_COLOR);
            paint_reveal_border(cx, node, viewport_origin, zoom, color, breath);
        }
    }
    let child_ancestor_revealing = ancestor_revealing
        || reveal_start
            .and_then(|started_at| reveal_fraction(started_at, now_ms))
            .is_some_and(|t| t <= op_editor_core::agent_indicators::REVEAL_CHILD_SUPPRESS_FRACTION);
    for child in &node.children {
        paint_node_reveal_indicator(
            cx,
            child,
            viewport_origin,
            zoom,
            now_ms,
            reveals,
            node_tags,
            frame_tags,
            agent_color,
            child_ancestor_revealing,
            remaining,
        );
        if *remaining == 0 {
            break;
        }
    }
}

fn reveal_fraction(started_at: u64, now_ms: u64) -> Option<f32> {
    if now_ms < started_at {
        return None;
    }
    let elapsed = now_ms.saturating_sub(started_at);
    if elapsed > op_editor_core::agent_indicators::REVEAL_DURATION_MS {
        return None;
    }
    let t = (elapsed as f32 / op_editor_core::agent_indicators::REVEAL_DURATION_MS as f32)
        .clamp(0.0, 1.0);
    Some(t)
}

fn reveal_breath(started_at: u64, now_ms: u64) -> Option<f32> {
    reveal_fraction(started_at, now_ms).map(|t| (t * std::f32::consts::PI).sin())
}

fn paint_reveal_border(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    viewport_origin: Point2D,
    zoom: f32,
    color: Color,
    breath: f32,
) {
    let bounds = node.aggregate_bounds();
    if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
        return;
    }
    let screen = Rect {
        origin: Point2D::new(
            viewport_origin.x + bounds.origin.x * zoom,
            viewport_origin.y + bounds.origin.y * zoom,
        ),
        size: Point2D::new(bounds.size.x * zoom, bounds.size.y * zoom),
    };
    super::canvas_viewport::paint_dashed_rect(cx, screen, color.with_alpha(breath * 0.7), 1.5);
}

/// A small pill above the frame's top-left: agent colour background, a
/// pulsing white status dot, and the agent's name — so the user can tell
/// which agent owns which frame.
fn paint_agent_badge(cx: &mut PaintCx<'_>, frame: Rect, color: Color, name: &str, breath: f32) {
    const PAD_X: f32 = 6.0;
    const PAD_Y: f32 = 3.0;
    const DOT_R: f32 = 3.0;
    const DOT_SPACE: f32 = DOT_R * 2.0 + 4.0;
    const LABEL_OFFSET_Y: f32 = 6.0;
    let font = 11.0;
    let name_w = cx.backend.measure_text(name, font);
    let badge_h = font + PAD_Y * 2.0;
    let badge_w = DOT_SPACE + name_w + PAD_X * 2.0;
    let badge = Rect {
        origin: Point2D::new(
            frame.origin.x + frame.size.x - badge_w,
            frame.origin.y - LABEL_OFFSET_Y - badge_h,
        ),
        size: Point2D::new(badge_w, badge_h),
    };
    cx.backend
        .fill_round_rect(badge, 4.0, color.with_alpha(0.9));
    let angle = (breath * std::f32::consts::PI * 2.0).max(0.0);
    let dot_center = Point2D::new(
        badge.origin.x + PAD_X + DOT_R,
        badge.origin.y + badge_h / 2.0,
    );
    for i in 0..3 {
        let trail = angle - i as f32 * 0.6;
        let dx = trail.cos() * DOT_R * 0.7;
        let dy = trail.sin() * DOT_R * 0.7;
        let alpha = 0.4 - i as f32 * 0.12;
        let radius = DOT_R * 0.8 * (1.0 - i as f32 * 0.2);
        cx.backend.fill_round_rect(
            Rect {
                origin: Point2D::new(dot_center.x + dx - radius, dot_center.y + dy - radius),
                size: Point2D::new(radius * 2.0, radius * 2.0),
            },
            radius,
            Color::WHITE.with_alpha(alpha),
        );
    }
    let main_dx = angle.cos() * DOT_R * 0.7;
    let main_dy = angle.sin() * DOT_R * 0.7;
    cx.backend.fill_round_rect(
        Rect {
            origin: Point2D::new(
                dot_center.x + main_dx - DOT_R * 0.6,
                dot_center.y + main_dy - DOT_R * 0.6,
            ),
            size: Point2D::new(DOT_R * 1.2, DOT_R * 1.2),
        },
        DOT_R * 0.6,
        Color::WHITE.with_alpha(0.95),
    );
    let label = TextLayout::single_run(
        name,
        "system-ui",
        font,
        jian_core::scene::Color::rgba(255, 255, 255, 255),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label,
        Point2D::new(
            badge.origin.x + PAD_X + DOT_SPACE,
            badge.origin.y + PAD_Y + font,
        ),
    );
}

/// Parse a `#RRGGBB` hex string into an opaque [`Color`]. Returns `None`
/// for malformed / non-ASCII input.
fn parse_hex(hex: &str) -> Option<Color> {
    let h = hex.strip_prefix('#').unwrap_or(hex);
    if !h.is_ascii() || h.len() < 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()? as f32 / 255.0;
    let g = u8::from_str_radix(&h[2..4], 16).ok()? as f32 / 255.0;
    let b = u8::from_str_radix(&h[4..6], 16).ok()? as f32 / 255.0;
    Some(Color { r, g, b, a: 1.0 })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout_scene::NodeKind;
    use crate::{RenderBackend, TextLayout};

    #[derive(Default)]
    struct RevealCaptureBackend {
        round_fills: Vec<(Rect, Color)>,
        round_strokes: Vec<(Rect, Color, f32)>,
        rect_strokes: Vec<(Rect, Color, f32)>,
        line_strokes: Vec<(Point2D, Point2D, Color, f32)>,
        clips: usize,
        saves: usize,
        restores: usize,
    }

    impl RenderBackend for RevealCaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, rect: Rect, color: Color, width: f32) {
            self.rect_strokes.push((rect, color, width));
        }
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {
            self.clips += 1;
        }
        fn save(&mut self) {
            self.saves += 1;
        }
        fn restore(&mut self) {
            self.restores += 1;
        }
        fn translate(&mut self, _: Point2D) {}
        fn stroke_line(&mut self, from: Point2D, to: Point2D, color: Color, width: f32) {
            self.line_strokes.push((from, to, color, width));
        }
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
    }

    fn reveal_root() -> Vec<SceneNode> {
        let mut node = SceneNode::leaf("new-node", NodeKind::Rect);
        node.bounds = Rect::xywh(10.0, 20.0, 120.0, 48.0);
        vec![node]
    }

    fn agent_root_with_child() -> Vec<SceneNode> {
        let mut child = SceneNode::leaf("child", NodeKind::Rect);
        child.bounds = Rect::xywh(20.0, 34.0, 96.0, 24.0);
        let mut root = SceneNode::leaf("root", NodeKind::Frame);
        root.bounds = Rect::xywh(10.0, 20.0, 120.0, 48.0);
        root.children = vec![child];
        vec![root]
    }

    fn reveal_tree_with_many_nodes(count: usize) -> Vec<SceneNode> {
        let mut root = SceneNode::leaf("root", NodeKind::Frame);
        root.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
        root.children = (0..count)
            .map(|i| {
                let mut child = SceneNode::leaf(format!("child-{i}"), NodeKind::Rect);
                child.bounds = Rect::xywh(i as f32, 0.0, 10.0, 10.0);
                child
            })
            .collect();
        vec![root]
    }

    fn many_root_frames(count: usize) -> Vec<SceneNode> {
        (0..count)
            .map(|i| {
                let mut root = SceneNode::leaf(format!("frame-{i}"), NodeKind::Frame);
                root.bounds = Rect::xywh(i as f32, 0.0, 100.0, 100.0);
                root
            })
            .collect()
    }

    #[test]
    fn empty_reveal_snapshot_skips_reveal_tree_walk() {
        let _guard = crate::agent_indicator_test_support::lock();
        let roots = reveal_tree_with_many_nodes(128);
        let indicators = AgentIndicators::default();
        let mut backend = RevealCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        reset_reveal_walk_visits();
        paint_agent_frame_indicators_with_snapshot(
            &mut cx,
            &roots,
            Point2D::ZERO,
            1.0,
            1_000,
            &indicators,
        );

        assert_eq!(
            reveal_walk_visits(),
            0,
            "empty reveal snapshots should not walk the scene tree"
        );
    }

    #[test]
    fn reveal_overlay_stops_after_all_reveal_targets_are_found() {
        let _guard = crate::agent_indicator_test_support::lock();
        let roots = reveal_tree_with_many_nodes(128);
        let mut indicators = AgentIndicators::default();
        indicators.reveals.insert("child-0".to_string(), 1_000);
        let mut backend = RevealCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        reset_reveal_walk_visits();
        paint_agent_frame_indicators_with_snapshot(
            &mut cx,
            &roots,
            Point2D::ZERO,
            1.0,
            1_000,
            &indicators,
        );

        assert!(
            reveal_walk_visits() <= 2,
            "a single early reveal should visit only the root and target, not every sibling (visited {})",
            reveal_walk_visits()
        );
        assert!(
            !backend.line_strokes.is_empty(),
            "the target reveal should still paint"
        );
    }

    #[test]
    fn frame_indicators_match_root_frames_linearly() {
        let _guard = crate::agent_indicator_test_support::lock();
        let roots = many_root_frames(32);
        let mut indicators = AgentIndicators::default();
        for i in 0..32 {
            indicators.frames.insert(
                format!("frame-{i}"),
                op_editor_core::agent_indicators::AgentTag {
                    color: "#4ECDC4".to_string(),
                    name: format!("Agent {i}"),
                },
            );
        }
        let mut backend = RevealCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        reset_frame_lookup_visits();
        paint_agent_frame_indicators_with_snapshot(
            &mut cx,
            &roots,
            Point2D::ZERO,
            1.0,
            1_000,
            &indicators,
        );

        assert!(
            frame_lookup_visits() <= 32,
            "frame indicators should match roots in one linear pass, not one root scan per agent frame"
        );
        assert_eq!(backend.round_strokes.len(), 64);
    }

    #[test]
    fn agent_reveal_inherits_frame_agent_for_ts_dashed_border() {
        let _guard = crate::agent_indicator_test_support::lock();
        let roots = agent_root_with_child();
        let mut indicators = AgentIndicators::default();
        indicators.frames.insert(
            "root".to_string(),
            op_editor_core::agent_indicators::AgentTag {
                color: "#FF6B6B".to_string(),
                name: "Sage".to_string(),
            },
        );
        indicators.reveals.insert("child".to_string(), 1_000);
        let mut backend = RevealCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        paint_agent_frame_indicators_with_snapshot(
            &mut cx,
            &roots,
            Point2D::new(100.0, 50.0),
            2.0,
            1_500,
            &indicators,
        );

        assert!(
            backend.rect_strokes.is_empty(),
            "TS reveal uses dashed line segments, not a solid stroke_rect"
        );
        assert!(
            backend.line_strokes.len() > 4,
            "TS reveal paints a dashed node border"
        );
        let (_, _, color, width) = backend.line_strokes[0];
        assert!(
            (color.r - 1.0).abs() < 0.01
                && (color.g - 0.419).abs() < 0.01
                && (color.b - 0.419).abs() < 0.01
                && (color.a - 0.7).abs() < 0.01,
            "node reveal should inherit the owning frame agent color at midpoint alpha"
        );
        assert!(
            (width - 1.5).abs() < 0.01,
            "TS node reveal border is 1.5 screen px"
        );
        let badge = backend
            .round_fills
            .first()
            .map(|(rect, _)| *rect)
            .expect("agent badge fill");
        let frame = Rect::xywh(120.0, 90.0, 240.0, 96.0);
        assert!(
            badge.origin.x > frame.origin.x + frame.size.x - 64.0,
            "badge should be right-aligned to the agent frame like TS"
        );
    }

    #[test]
    fn reveal_overlay_matches_ts_fade_border_without_sweep_or_lift() {
        let _guard = crate::agent_indicator_test_support::lock();
        let epoch = op_editor_core::agent_indicators::begin();
        op_editor_core::agent_indicators::add_reveal(epoch, "new-node", 1_000);
        let roots = reveal_root();
        let mut backend = RevealCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        paint_agent_frame_indicators(&mut cx, &roots, Point2D::new(100.0, 50.0), 2.0, 1_500);

        assert!(
            backend.round_fills.is_empty(),
            "TS Skia node reveal only fades the border; no wash or sweep"
        );
        assert!(
            !backend.line_strokes.is_empty(),
            "reveal paints a fading outline"
        );
        let base = Rect::xywh(120.0, 90.0, 240.0, 96.0);
        let (from, _, color, width) = backend.line_strokes[0];
        assert!(
            (from.x - base.origin.x).abs() < 0.01 && (from.y - base.origin.y).abs() < 0.01,
            "TS Skia node border is drawn on the authored rect without lift or scale"
        );
        assert!(
            (color.a - 0.7).abs() < 0.01,
            "TS Skia node border uses sin(t*pi) * 0.7 alpha at the midpoint"
        );
        assert!(
            (width - 1.5).abs() < 0.01,
            "TS Skia node border is 1.5 screen px"
        );
        assert_eq!(backend.saves, 0);
        assert_eq!(backend.clips, 0);
        assert_eq!(backend.restores, 0);

        let mut expired_backend = RevealCaptureBackend::default();
        let mut expired_cx = PaintCx {
            backend: &mut expired_backend,
        };
        paint_agent_frame_indicators(
            &mut expired_cx,
            &roots,
            Point2D::new(100.0, 50.0),
            2.0,
            2_001,
        );
        assert!(expired_backend.round_fills.is_empty());
        assert!(expired_backend.line_strokes.is_empty());
        op_editor_core::agent_indicators::clear();
    }

    #[test]
    fn future_reveal_overlay_waits_for_start_time() {
        let _guard = crate::agent_indicator_test_support::lock();
        let epoch = op_editor_core::agent_indicators::begin();
        op_editor_core::agent_indicators::add_reveal(epoch, "new-node", 1_500);
        let roots = reveal_root();

        let mut pending_backend = RevealCaptureBackend::default();
        let mut pending_cx = PaintCx {
            backend: &mut pending_backend,
        };
        paint_agent_frame_indicators(
            &mut pending_cx,
            &roots,
            Point2D::new(100.0, 50.0),
            2.0,
            1_000,
        );
        assert!(pending_backend.round_fills.is_empty());
        assert!(pending_backend.line_strokes.is_empty());

        let mut started_backend = RevealCaptureBackend::default();
        let mut started_cx = PaintCx {
            backend: &mut started_backend,
        };
        paint_agent_frame_indicators(
            &mut started_cx,
            &roots,
            Point2D::new(100.0, 50.0),
            2.0,
            2_000,
        );
        assert!(
            !started_backend.line_strokes.is_empty(),
            "reveal should paint once its scheduled start arrives"
        );
        op_editor_core::agent_indicators::end_if_epoch(epoch);
    }

    #[test]
    fn opening_parent_reveal_suppresses_nested_child_border() {
        let _guard = crate::agent_indicator_test_support::lock();
        let epoch = op_editor_core::agent_indicators::begin();
        op_editor_core::agent_indicators::add_reveal(epoch, "parent", 1_000);
        op_editor_core::agent_indicators::add_reveal(epoch, "child", 1_000);
        let mut child = SceneNode::leaf("child", NodeKind::Rect);
        child.bounds = Rect::xywh(16.0, 16.0, 48.0, 24.0);
        let mut parent = SceneNode::leaf("parent", NodeKind::Frame);
        parent.bounds = Rect::xywh(10.0, 20.0, 120.0, 48.0);
        parent.children = vec![child];
        let roots = vec![parent];

        let mut backend = RevealCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        paint_agent_frame_indicators(&mut cx, &roots, Point2D::new(100.0, 50.0), 2.0, 1_039);

        assert!(
            !backend.line_strokes.is_empty(),
            "opening parent reveal should still paint one coherent dashed border"
        );
        op_editor_core::agent_indicators::end_if_epoch(epoch);
    }

    #[test]
    fn delayed_child_reveal_paints_its_border_after_parent_settles() {
        let _guard = crate::agent_indicator_test_support::lock();
        let epoch = op_editor_core::agent_indicators::begin();
        op_editor_core::agent_indicators::add_reveal(epoch, "parent", 1_000);
        op_editor_core::agent_indicators::add_reveal(epoch, "child", 1_420);
        let mut child = SceneNode::leaf("child", NodeKind::Rect);
        child.bounds = Rect::xywh(16.0, 16.0, 48.0, 24.0);
        let mut parent = SceneNode::leaf("parent", NodeKind::Frame);
        parent.bounds = Rect::xywh(10.0, 20.0, 120.0, 48.0);
        parent.children = vec![child];
        let roots = vec![parent];

        let mut first_backend = RevealCaptureBackend::default();
        let mut first_cx = PaintCx {
            backend: &mut first_backend,
        };
        paint_agent_frame_indicators(&mut first_cx, &roots, Point2D::new(100.0, 50.0), 2.0, 1_000);

        let mut backend = RevealCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        paint_agent_frame_indicators(&mut cx, &roots, Point2D::new(100.0, 50.0), 2.0, 1_520);

        assert!(
            backend.line_strokes.iter().any(|(from, _, _, _)| {
                (from.x - 132.0).abs() < 0.01 && (from.y - 82.0).abs() < 0.01
            }),
            "a delayed child should keep its own border once the parent reveal has settled"
        );
        op_editor_core::agent_indicators::end_if_epoch(epoch);
    }

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
