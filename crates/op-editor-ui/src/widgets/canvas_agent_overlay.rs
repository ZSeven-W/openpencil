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

/// One full breathe (0 → 1 → 0) per this many ms.
const GLOW_PERIOD_MS: u64 = 1200;
const REVEAL_ACCENT: Color = Color {
    r: 0.231,
    g: 0.510,
    b: 0.965,
    a: 1.0,
};

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
    paint_node_reveal_indicators(
        cx,
        roots,
        viewport_origin,
        zoom,
        now_ms,
        &indicators.reveals,
    );
    if indicators.frames.is_empty() {
        return;
    }
    // Bell curve 0 → 1 → 0 across the period (matches the TS glow breath).
    let phase = (now_ms % GLOW_PERIOD_MS) as f32 / GLOW_PERIOD_MS as f32;
    let breath = (phase * std::f32::consts::PI).sin();
    for (frame_id, tag) in &indicators.frames {
        let Some(node) = roots.iter().find(|n| n.id == *frame_id) else {
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

fn paint_node_reveal_indicators(
    cx: &mut PaintCx<'_>,
    roots: &[SceneNode],
    viewport_origin: Point2D,
    zoom: f32,
    now_ms: u64,
    reveals: &HashMap<String, u64>,
) {
    for root in roots {
        paint_node_reveal_indicator(cx, root, viewport_origin, zoom, now_ms, reveals, false);
    }
}

fn paint_node_reveal_indicator(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    viewport_origin: Point2D,
    zoom: f32,
    now_ms: u64,
    reveals: &HashMap<String, u64>,
    ancestor_revealing: bool,
) {
    let reveal = reveals
        .get(&node.id)
        .and_then(|started_at| reveal_phase(*started_at, now_ms));
    if let Some((t, ease)) = reveal {
        if !ancestor_revealing {
            paint_reveal_sweep(cx, node, viewport_origin, zoom, t, ease);
        }
    }
    let child_ancestor_revealing = ancestor_revealing
        || reveal.is_some_and(|(t, _)| {
            t < op_editor_core::agent_indicators::REVEAL_CHILD_SUPPRESS_FRACTION
        });
    for child in &node.children {
        paint_node_reveal_indicator(
            cx,
            child,
            viewport_origin,
            zoom,
            now_ms,
            reveals,
            child_ancestor_revealing,
        );
    }
}

fn reveal_phase(started_at: u64, now_ms: u64) -> Option<(f32, f32)> {
    if now_ms < started_at {
        return None;
    }
    let elapsed = now_ms.saturating_sub(started_at);
    if elapsed > op_editor_core::agent_indicators::REVEAL_DURATION_MS {
        return None;
    }
    let t = (elapsed as f32 / op_editor_core::agent_indicators::REVEAL_DURATION_MS as f32)
        .clamp(0.0, 1.0);
    Some((t, ease_in_out_sine(t)))
}

fn paint_reveal_sweep(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    viewport_origin: Point2D,
    zoom: f32,
    t: f32,
    ease: f32,
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
    let animated = lifted_scaled_rect(screen, ease);
    let tail = (1.0 - t * t).clamp(0.0, 1.0);
    let radius = 8.0_f32.min(animated.size.y / 2.0);
    cx.backend
        .fill_round_rect(animated, radius, REVEAL_ACCENT.with_alpha(0.075 * tail));
    cx.backend.stroke_round_rect(
        animated,
        radius,
        REVEAL_ACCENT.with_alpha(0.42 * tail),
        1.25,
    );
    let sweep_w = (animated.size.x * 0.18).clamp(16.0, 64.0);
    let sweep = Rect {
        origin: Point2D::new(
            animated.origin.x - sweep_w + (animated.size.x + sweep_w * 2.0) * ease,
            animated.origin.y - 3.0,
        ),
        size: Point2D::new(sweep_w, animated.size.y + 6.0),
    };
    cx.backend.save();
    cx.backend.clip_rect(animated);
    cx.backend
        .fill_round_rect(sweep, sweep_w / 2.0, Color::WHITE.with_alpha(0.24 * tail));
    cx.backend.restore();
}

fn lifted_scaled_rect(rect: Rect, ease: f32) -> Rect {
    let settle = 1.0 - ease;
    let scale = 0.986 + ease * 0.014;
    let lift = settle * 5.0;
    let w = rect.size.x * scale;
    let h = rect.size.y * scale;
    Rect {
        origin: Point2D::new(
            rect.origin.x + (rect.size.x - w) / 2.0,
            rect.origin.y + (rect.size.y - h) / 2.0 + lift,
        ),
        size: Point2D::new(w, h),
    }
}

fn ease_in_out_sine(t: f32) -> f32 {
    -(std::f32::consts::PI * t).cos() / 2.0 + 0.5
}

/// A small pill above the frame's top-left: agent colour background, a
/// pulsing white status dot, and the agent's name — so the user can tell
/// which agent owns which frame.
fn paint_agent_badge(cx: &mut PaintCx<'_>, frame: Rect, color: Color, name: &str, breath: f32) {
    const BADGE_H: f32 = 18.0;
    const PAD: f32 = 7.0;
    const DOT: f32 = 6.0;
    let font = 11.0;
    let name_w = cx.backend.measure_text(name, font);
    let badge = Rect {
        origin: Point2D::new(frame.origin.x, frame.origin.y - BADGE_H - 4.0),
        size: Point2D::new(PAD + DOT + 5.0 + name_w + PAD, BADGE_H),
    };
    // Opaque fill: a translucent badge would composite over the (theme-
    // dependent) canvas, shifting the real background luminance away from
    // the value the foreground-contrast pick below is computed against.
    cx.backend.fill_round_rect(badge, BADGE_H / 2.0, color);
    // Contrast-aware foreground via WCAG relative luminance — dark glyphs
    // on light agent colours (coral / yellow / mint / teal / orange),
    // white only on the genuinely dark one (purple), so the name reads on
    // every palette entry. 0.179 is the standard black-vs-white crossover.
    let fg = if relative_luminance(color) > 0.179 {
        0.12_f32
    } else {
        1.0_f32
    };
    let fg_u8 = (fg * 255.0) as u8;
    // Pulsing status dot in the foreground colour.
    cx.backend.fill_round_rect(
        Rect {
            origin: Point2D::new(badge.origin.x + PAD, badge.origin.y + (BADGE_H - DOT) / 2.0),
            size: Point2D::new(DOT, DOT),
        },
        DOT / 2.0,
        Color {
            r: fg,
            g: fg,
            b: fg,
            a: 0.55 + breath * 0.45,
        },
    );
    let label = TextLayout::single_run(
        name,
        "system-ui",
        font,
        jian_core::scene::Color::rgba(fg_u8, fg_u8, fg_u8, 255),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label,
        Point2D::new(badge.origin.x + PAD + DOT + 5.0, badge.origin.y + 13.0),
    );
}

/// WCAG relative luminance of an opaque colour (gamma-corrected). Used
/// to pick a readable badge foreground.
fn relative_luminance(c: Color) -> f32 {
    fn lin(v: f32) -> f32 {
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b)
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
        round_fills: Vec<Rect>,
        round_strokes: Vec<Rect>,
        clips: usize,
        saves: usize,
        restores: usize,
    }

    impl RenderBackend for RevealCaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
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
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, rect: Rect, _: f32, _: Color) {
            self.round_fills.push(rect);
        }
        fn stroke_round_rect(&mut self, rect: Rect, _: f32, _: Color, _: f32) {
            self.round_strokes.push(rect);
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

    #[test]
    fn reveal_overlay_paints_lifted_highlight_sweep_and_prunes_after_window() {
        let _guard = crate::agent_indicator_test_support::lock();
        let epoch = op_editor_core::agent_indicators::begin();
        op_editor_core::agent_indicators::add_reveal(epoch, "new-node", 1_000);
        let roots = reveal_root();
        let mut backend = RevealCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        paint_agent_frame_indicators(&mut cx, &roots, Point2D::new(100.0, 50.0), 2.0, 1_160);

        assert!(
            backend.round_fills.len() >= 2,
            "reveal paints a wash plus a clipped sweep"
        );
        assert!(
            !backend.round_strokes.is_empty(),
            "reveal paints a crisp outline"
        );
        assert!(
            backend.saves > 0 && backend.clips > 0 && backend.restores > 0,
            "sweep should be clipped to the animated node rect"
        );
        let base = Rect::xywh(120.0, 90.0, 240.0, 96.0);
        let animated = backend.round_fills[0];
        assert!(
            animated.origin.y > base.origin.y && animated.size.x < base.size.x,
            "reveal overlay should start slightly low and small before settling"
        );

        let mut expired_backend = RevealCaptureBackend::default();
        let mut expired_cx = PaintCx {
            backend: &mut expired_backend,
        };
        paint_agent_frame_indicators(
            &mut expired_cx,
            &roots,
            Point2D::new(100.0, 50.0),
            2.0,
            2_600,
        );
        assert!(expired_backend.round_fills.is_empty());
        assert!(expired_backend.round_strokes.is_empty());
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
        assert!(pending_backend.round_strokes.is_empty());

        let mut started_backend = RevealCaptureBackend::default();
        let mut started_cx = PaintCx {
            backend: &mut started_backend,
        };
        paint_agent_frame_indicators(
            &mut started_cx,
            &roots,
            Point2D::new(100.0, 50.0),
            2.0,
            1_500,
        );
        assert!(
            !started_backend.round_fills.is_empty(),
            "reveal should paint once its scheduled start arrives"
        );
        op_editor_core::agent_indicators::end_if_epoch(epoch);
    }

    #[test]
    fn opening_parent_reveal_suppresses_nested_child_sweep() {
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
        paint_agent_frame_indicators(&mut cx, &roots, Point2D::new(100.0, 50.0), 2.0, 1_040);

        assert_eq!(
            backend.round_strokes.len(),
            1,
            "nested child reveal should not add another sweep during the parent's opening beat"
        );
        op_editor_core::agent_indicators::end_if_epoch(epoch);
    }

    #[test]
    fn delayed_child_reveal_paints_its_sweep_after_parent_settles() {
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

        let mut backend = RevealCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        paint_agent_frame_indicators(&mut cx, &roots, Point2D::new(100.0, 50.0), 2.0, 1_520);

        assert_eq!(
            backend.round_strokes.len(),
            2,
            "a delayed child should keep its own sweep once the parent reveal has settled"
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

    #[test]
    fn badge_foreground_is_legible_on_every_palette_color() {
        // `true` = the badge would use dark glyphs (light background).
        let dark = |hex: &str| relative_luminance(parse_hex(hex).unwrap()) > 0.179;
        assert!(dark("#FF6B6B"), "coral");
        assert!(dark("#4ECDC4"), "teal");
        assert!(dark("#FFD93D"), "yellow");
        assert!(dark("#A8E6CF"), "mint");
        assert!(dark("#FF8A5C"), "orange");
        assert!(!dark("#6C5CE7"), "purple is dark enough for white text");
    }
}
