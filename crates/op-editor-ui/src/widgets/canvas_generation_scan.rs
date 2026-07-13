//! Radar-scan overlay for empty sections inside actively generating frames.

use crate::layout_scene::{NodeKind, SceneNode};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};
use op_editor_core::agent_indicators::AgentIndicators;
use std::collections::HashSet;

// Calm sweep — Pencil's placeholder band reads unhurried; a fast strobe
// draws the eye away from the content that IS landing.
const SCAN_PERIOD_MS: u64 = 2_200;
// Tall band — Pencil's wash fills roughly half the placeholder as it
// sweeps (measured from the zoomed hero frames, 2026-07-12).
const BAND_HEIGHT_FRACTION: f32 = 0.55;
// Enough segments that the band reads as one smooth gradient — at 8 the
// steps were visible as venetian-blind stripes (user screenshot 2026-07-11).
const BAND_SEGMENTS: usize = 24;
const EDGE_FADE_FRACTION: f32 = 0.15;

/// Pencil's skeleton periwinkle — the generation visuals keep ONE fixed blue
/// regardless of the design's palette or the editor theme (measured from the
/// 1760px hero frames: the rocket design is orange/green yet every wireframe,
/// "Generating" label, and placeholder wash stays this blue). Painting with
/// the theme accent made the scan grey-on-grey on dark designs.
pub(super) const SKELETON_BLUE: Color = Color::rgb_u8(0x6c, 0x7c, 0xf0);

pub(super) fn scan_phase(now_ms: u64, period_ms: u64) -> f32 {
    if period_ms == 0 {
        return 0.0;
    }
    (now_ms % period_ms) as f32 / period_ms as f32
}

pub(super) fn is_placeholder_section(node: &SceneNode) -> bool {
    node.kind == NodeKind::Frame && node.children.is_empty()
}

/// Scan-eligible ids + QUEUED shells to suppress entirely.
pub(super) struct GenerationPaintSets {
    /// Nodes allowed to paint the placeholder wash (the on-deck shells and
    /// all worked content).
    pub scan: HashSet<String>,
    /// Empty shells whose turn has NOT come: they keep their layout slot
    /// but paint NOTHING — Pencil shows plain canvas where work has not
    /// reached, not a stack of dark author-filled slabs (user report
    /// 2026-07-12: "下面的黑块是什么？先隐藏？").
    pub suppressed: HashSet<String>,
}

pub(super) fn generating_paint_sets(
    roots: &[SceneNode],
    indicators: Option<&AgentIndicators>,
) -> Option<GenerationPaintSets> {
    let indicators = indicators.filter(|value| value.run_active && !value.frames.is_empty())?;
    let mut sets = GenerationPaintSets {
        scan: HashSet::new(),
        suppressed: HashSet::new(),
    };
    for root in roots {
        if indicators.frames.contains_key(&root.id) {
            // Per generating root (Team mode runs several concurrently).
            let mut deck_taken = false;
            collect_descendants(&root.children, &mut sets, &mut deck_taken);
        }
    }
    Some(sets)
}

fn collect_descendants(nodes: &[SceneNode], sets: &mut GenerationPaintSets, deck_taken: &mut bool) {
    // Work-order gate: across the WHOLE generating root only the FIRST
    // empty shell in document (pre-order) position is "on deck" and washes;
    // every other empty shell waits its turn INVISIBLY (layout slot kept,
    // paint suppressed). The gate was per-container at first, which lit the
    // root's trailing bottom-nav shell while the model was narrating "fill
    // the Header" — the fill order is document order, so the deck must be
    // global to the root (user report 2026-07-12). Pencil never lights a
    // section work has not reached, and a region must light up the moment
    // it becomes the next target.
    for node in nodes {
        let empty_frame = node.kind == NodeKind::Frame && node.children.is_empty();
        if empty_frame {
            if !*deck_taken {
                sets.scan.insert(node.id.clone());
            } else {
                sets.suppressed.insert(node.id.clone());
            }
            *deck_taken = true;
        } else {
            sets.scan.insert(node.id.clone());
        }
        collect_descendants(&node.children, sets, deck_taken);
    }
}

pub(super) fn paint_generation_scan(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    bounds: Rect,
    zoom: f32,
    now_ms: u64,
    accent: Color,
) {
    if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
        return;
    }

    let phase = scan_phase(now_ms, SCAN_PERIOD_MS);
    let band_height = bounds.size.y * BAND_HEIGHT_FRACTION;
    let travel = bounds.size.y + band_height;
    let band_top = bounds.origin.y - band_height + travel * phase;
    let travel_alpha = edge_fade(phase);
    let segment_height = band_height / BAND_SEGMENTS as f32;

    // Pencil parity: the placeholder is a soft translucent accent PANEL,
    // not a hole — without this base fill an empty frame shows the raw
    // canvas background through the design (reads as a black slab on
    // light pages). Base wash + hairline border + slow sweep band.
    let radius = node.corner_radius * zoom;
    if radius > 0.5 {
        cx.backend
            .fill_round_rect(bounds, radius, accent.with_alpha(0.09));
    } else {
        cx.backend.fill_rect(bounds, accent.with_alpha(0.09));
    }

    cx.backend.save();
    cx.backend.clip_rect(bounds);
    for segment in 0..BAND_SEGMENTS {
        let t = (segment as f32 + 0.5) / BAND_SEGMENTS as f32;
        let ramp = 1.0 - (t * 2.0 - 1.0).abs();
        let rect = Rect {
            origin: Point2D::new(bounds.origin.x, band_top + segment as f32 * segment_height),
            size: Point2D::new(bounds.size.x, segment_height + 0.5),
        };
        cx.backend
            .fill_rect(rect, accent.with_alpha(0.14 * ramp * travel_alpha));
    }
    cx.backend.restore();

    // Vivid outline — Pencil's skeleton wireframe reads as a clear 1px
    // periwinkle line, not a hint.
    let border = accent.with_alpha(0.7);
    if radius > 0.5 {
        cx.backend.stroke_round_rect(bounds, radius, border, 1.0);
    } else {
        cx.backend.stroke_rect(bounds, border, 1.0);
    }
}

fn edge_fade(phase: f32) -> f32 {
    let fade_in = (phase / EDGE_FADE_FRACTION).clamp(0.0, 1.0);
    let fade_out = ((1.0 - phase) / EDGE_FADE_FRACTION).clamp(0.0, 1.0);
    fade_in.min(fade_out)
}
