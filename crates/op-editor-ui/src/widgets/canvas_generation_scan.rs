//! Radar-scan overlay for empty sections inside actively generating frames.

use super::canvas_viewport_paint::REVEAL_WIREFRAME_MS;
use crate::layout_scene::{Effect, NodeKind, SceneNode};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};
use op_editor_core::agent_indicators::AgentIndicators;
use std::collections::{HashMap, HashSet};

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

/// A batch can populate a formerly-empty shell before any of its children
/// are allowed to paint. Keep that shell visually "empty" for the existing
/// reveal runway so a fast batch still shows part of the radar sweep.
pub(super) fn is_pending_filled_section(
    node: &SceneNode,
    reveals: &HashMap<String, u64>,
    now_ms: u64,
) -> bool {
    if node.kind != NodeKind::Frame || node.children.is_empty() {
        return false;
    }
    // A newly-created shell that has not started its own reveal is itself
    // hidden by the reveal painter; its nearest already-visible ancestor owns
    // the placeholder treatment until this shell materialises.
    if reveals
        .get(&node.id)
        .is_some_and(|started_at| *started_at > now_ms)
    {
        return false;
    }

    children_have_no_visible_material(&node.children, reveals, now_ms)
}

/// Transparent wrappers are not always assigned their own reveal slot. A
/// subtree remains visually empty while every branch is either still pending
/// or paints no pixels of its own; the first genuinely visible branch prevents
/// the shell from washing over finished work.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SubtreeMaterialization {
    Pending,
    Visible,
    Empty,
}

fn children_have_no_visible_material(
    children: &[SceneNode],
    reveals: &HashMap<String, u64>,
    now_ms: u64,
) -> bool {
    for child in children.iter().filter(|child| !child.hidden) {
        match subtree_materialization(child, reveals, now_ms) {
            SubtreeMaterialization::Visible => return false,
            SubtreeMaterialization::Pending | SubtreeMaterialization::Empty => {}
        }
    }
    true
}

fn subtree_materialization(
    node: &SceneNode,
    reveals: &HashMap<String, u64>,
    now_ms: u64,
) -> SubtreeMaterialization {
    if node.hidden {
        return SubtreeMaterialization::Empty;
    }

    // A pending reveal gates the whole subtree in `paint_node_inner`, even
    // when this node is a zero-sized wrapper around otherwise-visible
    // descendants. Once the reveal starts, a bounded node's wireframe is the
    // visible handoff from the parent radar. Geometry alone is not enough
    // after that beat: transparent layout containers must keep recursing.
    if let Some(started_at) = reveals.get(&node.id) {
        if *started_at > now_ms {
            return SubtreeMaterialization::Pending;
        }
        let elapsed_ms = now_ms.saturating_sub(*started_at);
        if elapsed_ms < REVEAL_WIREFRAME_MS && node_has_extent(node) {
            return SubtreeMaterialization::Visible;
        }
    }

    if node_paints_own_visual(node) {
        return SubtreeMaterialization::Visible;
    }

    let mut saw_pending = false;
    for child in node.children.iter().filter(|child| !child.hidden) {
        match subtree_materialization(child, reveals, now_ms) {
            SubtreeMaterialization::Pending => saw_pending = true,
            SubtreeMaterialization::Visible => return SubtreeMaterialization::Visible,
            SubtreeMaterialization::Empty => {}
        }
    }
    if saw_pending {
        SubtreeMaterialization::Pending
    } else {
        SubtreeMaterialization::Empty
    }
}

fn node_has_extent(node: &SceneNode) -> bool {
    node.bounds.size.x > 0.0 && node.bounds.size.y > 0.0
}

/// Pure counterpart to the per-kind canvas painter's own-pixel branches.
/// Layout bounds do not imply pixels: Frames, Groups, and Rectangles are
/// frequently transparent wrappers whose only visible material is a child.
fn node_paints_own_visual(node: &SceneNode) -> bool {
    if !node_has_extent(node) {
        return false;
    }

    let styled_box = node.image_src.is_some()
        || node.fill.is_some()
        || node.gradient.is_some()
        || node.shader.is_some()
        || node.stroke.is_some();
    let casts_outer_shadow = node
        .effects
        .iter()
        .any(|effect| matches!(effect, Effect::DropShadow(shadow) if !shadow.inner));
    let paints_widget = node.widget.as_ref().is_some_and(|widget| {
        matches!(
            widget.kind.as_str(),
            "switch"
                | "checkbox"
                | "slider"
                | "progress"
                | "select"
                | "radio_group"
                | "text_input"
                | "text_area"
                | "number_input"
                | "tabs"
        )
    });

    match &node.kind {
        NodeKind::Frame | NodeKind::Rect => styled_box || casts_outer_shadow || paints_widget,
        NodeKind::Group => false,
        NodeKind::Ellipse => {
            node.image_src.is_some()
                || node.fill.is_some()
                || node.stroke.is_some()
                || casts_outer_shadow
        }
        NodeKind::Polygon => {
            node.image_src.is_some() || node.fill.is_some() || node.stroke.is_some()
        }
        // The painter supplies a default black stroke when no explicit line
        // paint is authored.
        NodeKind::Line => true,
        NodeKind::Path => {
            if node
                .svg_path
                .as_deref()
                .is_some_and(|path| !path.is_empty())
            {
                node.fill.is_some()
                    || node.gradient.is_some()
                    || node.stroke.is_some()
                    || node
                        .effects
                        .iter()
                        .any(|effect| matches!(effect, Effect::DropShadow(shadow) if shadow.inner))
            } else {
                node.points.len() >= 2
            }
        }
        NodeKind::Text => {
            paints_widget || node.text.as_deref().is_some_and(|text| !text.is_empty())
        }
        NodeKind::Other(tag) if tag == "icon_font" => node
            .text
            .as_deref()
            .is_some_and(|glyph_name| !glyph_name.is_empty()),
        NodeKind::Other(_) => false,
    }
}

/// The two generation states a visually empty shell can be in.
pub(super) struct GenerationPaintSets {
    /// The shell being worked RIGHT NOW (first empty in fill order) plus all
    /// worked content: the full radar treatment — wash + sweeping band.
    pub scan: HashSet<String>,
    /// Shells still QUEUED: they keep their slot and show the skeleton, but
    /// as a quiet static wireframe — no wash to read as a dark slab, no sweep
    /// to claim they are being worked. The skeleton stays visible everywhere
    /// (user: "骨架先行效果还是要的"); only ONE shell may look active.
    pub queued: HashSet<String>,
}

pub(super) fn generating_paint_sets(
    roots: &[SceneNode],
    indicators: Option<&AgentIndicators>,
    now_ms: u64,
) -> Option<GenerationPaintSets> {
    let indicators = indicators.filter(|value| {
        !value.frames.is_empty()
            && (value.run_active
                || value
                    .reveals
                    .values()
                    .any(|started_at| *started_at > now_ms))
    })?;
    let mut sets = GenerationPaintSets {
        scan: HashSet::new(),
        queued: HashSet::new(),
    };
    for root in roots {
        if indicators.frames.contains_key(&root.id) {
            // Per generating root (Team mode runs several concurrently).
            let mut deck_taken = false;
            collect_descendants(
                &root.children,
                &mut sets,
                &mut deck_taken,
                &indicators.reveals,
                now_ms,
            );
        }
    }
    Some(sets)
}

fn collect_descendants(
    nodes: &[SceneNode],
    sets: &mut GenerationPaintSets,
    deck_taken: &mut bool,
    reveals: &HashMap<String, u64>,
    now_ms: u64,
) {
    // Work-order gate: across the WHOLE generating root, the FIRST visually
    // empty shell in document (pre-order) position is "on deck" and gets the
    // active radar; every later shell still SHOWS its skeleton, but as a quiet
    // wireframe. Two earlier shapes were both wrong: a per-container gate lit
    // the root's trailing bottom-nav while the model was filling the header
    // nested in the content wrapper, and suppressing the queue entirely left
    // holes in the middle of the skeleton. Fill order is document order, so
    // the deck is global — and exactly one shell may look active.
    for node in nodes {
        let empty_frame = node.kind == NodeKind::Frame && node.children.is_empty();
        let pending_filled = is_pending_filled_section(node, reveals, now_ms);
        if empty_frame || pending_filled {
            if !*deck_taken {
                sets.scan.insert(node.id.clone());
            } else {
                sets.queued.insert(node.id.clone());
            }
            *deck_taken = true;
        } else {
            sets.scan.insert(node.id.clone());
        }
        collect_descendants(&node.children, sets, deck_taken, reveals, now_ms);
    }
}

/// A queued shell's skeleton: the wireframe outline and a whisper of wash —
/// present, clearly a placeholder, and unmistakably NOT the one being worked.
pub(super) fn paint_queued_skeleton(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    bounds: Rect,
    zoom: f32,
    accent: Color,
) {
    if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
        return;
    }
    let radius = node.corner_radius * zoom;
    if radius > 0.5 {
        cx.backend
            .fill_round_rect(bounds, radius, accent.with_alpha(0.035));
        cx.backend
            .stroke_round_rect(bounds, radius, accent.with_alpha(0.3), 1.0);
    } else {
        cx.backend.fill_rect(bounds, accent.with_alpha(0.035));
        cx.backend.stroke_rect(bounds, accent.with_alpha(0.3), 1.0);
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
