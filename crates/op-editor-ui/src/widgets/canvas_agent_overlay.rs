//! Per-agent "breathing" canvas indicators for the concurrent agent
//! team — a soft glow plus a crisp ring in each agent's colour, pulsing
//! around the root frame that agent is building while a multi-screen
//! generation runs. Reads the process-global
//! [`op_editor_core::agent_indicators`] registry every frame.

use crate::layout_scene::SceneNode;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};

/// One full breathe (0 → 1 → 0) per this many ms.
const GLOW_PERIOD_MS: u64 = 1200;

/// Paint a breathing border around every active-page root frame that an
/// agent currently owns. `roots` are the page's top-level scene nodes;
/// `viewport_origin` already folds in the pan offset.
pub(crate) fn paint_agent_frame_indicators(
    cx: &mut PaintCx<'_>,
    roots: &[SceneNode],
    viewport_origin: Point2D,
    zoom: f32,
    now_ms: u64,
) {
    let indicators = op_editor_core::agent_indicators::snapshot();
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
    }
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
