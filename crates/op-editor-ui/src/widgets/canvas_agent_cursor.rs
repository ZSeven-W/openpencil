//! Figma-style per-agent pointer cursor for AI design generation.
//!
//! While a generation streams nodes onto the canvas, each agent gets an
//! arrow cursor that flies between the nodes it is placing, arriving
//! exactly when a node's reveal starts — the same instant
//! `canvas_viewport_paint` stops hiding it and plays its scale-in pop.
//! The element the cursor is parked on carries a breathing border in
//! the agent's colour, and the cursor stays visible for the whole run:
//! the registry retains reveals until the run ends, so between streamed
//! chunks the cursor parks on the last placement instead of fading.
//! Everything here is a pure function of the reveal schedule in
//! [`op_editor_core::agent_indicators`] and `now_ms`; no cursor state
//! is stored anywhere, so a mid-run schedule rewrite (frame-gap
//! recovery) simply re-derives the path next frame.

use std::collections::HashMap;

use crate::layout_scene::SceneNode;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::agent_indicators::{AgentIndicators, AgentTag};

/// Longest single flight; longer waypoint gaps depart late, arrive on time.
const MAX_FLIGHT_MS: u64 = 350;
/// Minimum time between cursor stops; denser waypoints collapse to the
/// last placement in the dwell window.
const DWELL_MS: u64 = 280;
/// Fade-in slide duration before a queue's first waypoint.
const ENTRY_MS: u64 = 250;
/// Where the entry slide starts, relative to the first waypoint (screen px).
const ENTRY_OFFSET_X: f32 = -28.0;
const ENTRY_OFFSET_Y: f32 = -20.0;
/// Tiny standalone leaves still reveal, but the cursor does not chase
/// them. The threshold is in document-space square pixels.
const MIN_STANDALONE_WAYPOINT_AREA: f32 = 2_000.0;

/// Fallback for reveals not owned by any tagged agent (the same red the
/// retired dashed-reveal border used as its untagged default).
const FALLBACK_COLOR: Color = Color {
    r: 1.0,
    g: 0.419,
    b: 0.419,
    a: 1.0,
};

/// Pencil-silhouette pointer, TIP at the origin pointing up-left, body
/// extending down-right at 45°, in screen px (zoom-independent). The
/// brand cursor: the working agent literally draws with a pencil —
/// agent-colour body, white outline, white sharpened tip wedge (see
/// `PENCIL_TIP_POINTS`). Anatomy mirrors Pencil's multiplayer cursor
/// (solid tinted pointer + name pill) without copying its arrow.
const PENCIL_BODY_POINTS: [(f32, f32); 5] = [
    (0.0, 0.0),   // graphite tip (hotspot)
    (8.1, 2.1),   // right flank of the sharpened collar
    (21.1, 15.3), // body end, right corner
    (15.3, 21.1), // body end, left corner (flat eraser butt)
    (2.1, 8.1),   // left flank of the sharpened collar
];

/// White wedge over the tip — the sharpened-graphite highlight.
const PENCIL_TIP_POINTS: [(f32, f32); 3] = [(0.0, 0.0), (4.1, 1.1), (1.1, 4.1)];

/// Collar seam between the sharpened cone and the painted body.
const PENCIL_COLLAR: ((f32, f32), (f32, f32)) = ((2.1, 8.1), (8.1, 2.1));

/// A scheduled placement the cursor must reach: one generated node's
/// reveal start, at that node's centre, plus the node's screen rect for
/// the current-element breathing border.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Waypoint {
    pub start_ms: u64,
    pub pos: Point2D,
    pub rect: Rect,
}

/// Frame-local cursor pose derived from a waypoint queue.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Kinematics {
    pub pos: Point2D,
    /// 0..=1 opacity (entry fade-in only — a live run never fades out;
    /// the whole overlay disappears together when the run's indicators
    /// clear).
    pub alpha: f32,
    /// Index of the last waypoint whose start has passed — the element
    /// the cursor is currently working on. `None` during entry.
    pub current: Option<usize>,
}

/// Parse a `#RRGGBB` hex string into an opaque [`Color`]. Returns `None`
/// for malformed / non-ASCII input.
pub(crate) fn parse_hex(hex: &str) -> Option<Color> {
    let h = hex.strip_prefix('#').unwrap_or(hex);
    if !h.is_ascii() || h.len() < 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()? as f32 / 255.0;
    let g = u8::from_str_radix(&h[2..4], 16).ok()? as f32 / 255.0;
    let b = u8::from_str_radix(&h[4..6], 16).ok()? as f32 / 255.0;
    Some(Color { r, g, b, a: 1.0 })
}

pub(crate) fn ease_out_cubic(t: f32) -> f32 {
    let u = 1.0 - t.clamp(0.0, 1.0);
    1.0 - u * u * u
}

/// Smoothstep-style ease for flights: slow out of the park, fast
/// mid-flight, soft landing. Chained short hops read as deliberate
/// dart-dart-dart instead of a constant-speed crawl.
pub(crate) fn ease_in_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let u = -2.0 * t + 2.0;
        1.0 - u * u * u / 2.0
    }
}

fn lerp(a: Point2D, b: Point2D, t: f32) -> Point2D {
    Point2D::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

/// Derive the cursor pose for one agent's placement queue at `now_ms`.
/// `waypoints` must be sorted by `start_ms`. Equal-start placements
/// coalesce: at their shared start instant the cursor parks on the last
/// one in the caller's sort order (start_ms, then node id). `None` =
/// cursor not shown (before the entry window opens).
pub(crate) fn cursor_kinematics(waypoints: &[Waypoint], now_ms: u64) -> Option<Kinematics> {
    if waypoints.is_empty() {
        return None;
    }
    let next_idx = waypoints
        .iter()
        .position(|w| w.start_ms > now_ms)
        .unwrap_or(waypoints.len());
    if next_idx == 0 {
        // Entry: slide + fade toward the queue's first waypoint.
        let first = &waypoints[0];
        let entry_start = first.start_ms.saturating_sub(ENTRY_MS);
        if now_ms < entry_start {
            return None;
        }
        let t = ((now_ms - entry_start) as f32 / ENTRY_MS as f32).clamp(0.0, 1.0);
        let from = Point2D::new(first.pos.x + ENTRY_OFFSET_X, first.pos.y + ENTRY_OFFSET_Y);
        return Some(Kinematics {
            pos: lerp(from, first.pos, ease_out_cubic(t)),
            alpha: t,
            current: None,
        });
    }
    let current = Some(next_idx - 1);
    let prev = &waypoints[next_idx - 1];
    if next_idx == waypoints.len() {
        // Queue exhausted: park on the last placement until the next
        // streamed chunk schedules more work (or the run ends).
        return Some(Kinematics {
            pos: prev.pos,
            alpha: 1.0,
            current,
        });
    }
    let next = &waypoints[next_idx];
    // Flight occupies at most 70% of the gap so even densely-staggered
    // hops keep a micro-dwell before departure — pause, then an eased
    // dart, instead of continuous constant-speed crawling.
    let gap = next.start_ms.saturating_sub(prev.start_ms).max(1);
    let flight = MAX_FLIGHT_MS.min((gap as f64 * 0.7) as u64).max(1);
    let depart = prev.start_ms.max(next.start_ms.saturating_sub(flight));
    if now_ms < depart {
        // Parked, waiting for a distant slot.
        return Some(Kinematics {
            pos: prev.pos,
            alpha: 1.0,
            current,
        });
    }
    let window = next.start_ms.saturating_sub(depart).max(1);
    let t = ((now_ms - depart) as f32 / window as f32).clamp(0.0, 1.0);
    Some(Kinematics {
        pos: lerp(prev.pos, next.pos, ease_in_out_cubic(t)),
        alpha: 1.0,
        current,
    })
}

/// Placements grouped by owning agent (`None` key = untagged fallback);
/// each group carries its tag plus `(node id, waypoint)` pairs.
type AgentGroups = HashMap<Option<(String, String)>, (Option<AgentTag>, Vec<(String, Waypoint)>)>;

struct CollectWaypointCx<'a> {
    indicators: &'a AgentIndicators,
    viewport_origin: Point2D,
    zoom: f32,
}

/// One agent's cursor, fully resolved for painting this frame.
#[derive(Debug, Clone)]
pub(crate) struct CursorSprite {
    pub pos: Point2D,
    pub alpha: f32,
    pub color: Color,
    pub name: Option<String>,
    /// Screen rect of the element the cursor is currently working on —
    /// it gets the breathing border.
    // Production paint no longer draws the breathing border (skeleton owns
    // the working-area affordance), but the kinematics tests still assert
    // waypoint→current-element mapping through this field.
    #[allow(dead_code)]
    pub current_rect: Option<Rect>,
}

/// Collect `(owning agent, node id, waypoint)` for every scheduled reveal,
/// inheriting agent tags down the tree the same way the frame-indicator
/// overlay does. `remaining` early-stops the walk once every reveal in
/// the snapshot has been located.
///
/// Waypoints target the REVEALING NODE itself — the cursor eases from
/// element to element as they materialize (ancestor-window suppression
/// keeps children that pop WITH their parent from double-booking it, and
/// tiny standalone leaves are skipped so a 12px icon doesn't yank the
/// pointer).
fn collect_waypoints(
    node: &SceneNode,
    cx: &CollectWaypointCx<'_>,
    inherited: Option<&AgentTag>,
    revealed_ancestor_start_ms: Option<u64>,
    remaining: &mut usize,
    out: &mut Vec<(Option<AgentTag>, String, Waypoint)>,
) {
    if *remaining == 0 {
        return;
    }
    let tag = cx
        .indicators
        .nodes
        .get(&node.id)
        .or_else(|| cx.indicators.frames.get(&node.id))
        .or(inherited);
    let reveal_start_ms = cx.indicators.reveals.get(&node.id).copied();
    if let Some(start_ms) = reveal_start_ms {
        *remaining -= 1;
        // Suppress only when this node pops in WITH its ancestor — i.e. its
        // reveal lands inside the dwell window AFTER the ancestor's start.
        // (The reversed comparison parked the cursor on the page root for a
        // whole run: the root reveals first at scaffold time, so "ancestor
        // earlier than child" held for every descendant — measured.)
        let suppressed_by_ancestor = revealed_ancestor_start_ms
            .is_some_and(|ancestor_start| start_ms <= ancestor_start + DWELL_MS);
        let b = node.aggregate_bounds();
        let area = b.size.x * b.size.y;
        // A tiny leaf only suppresses when it would be its OWN waypoint;
        // inside a skeleton section the waypoint is the section, so even a
        // 12px icon reveal legitimately parks the cursor on that section.
        let standalone_tiny_leaf = node.children.is_empty()
            && revealed_ancestor_start_ms.is_none()
            && area < MIN_STANDALONE_WAYPOINT_AREA;
        if !suppressed_by_ancestor && !standalone_tiny_leaf && b.size.x > 0.0 && b.size.y > 0.0 {
            let rect = Rect::xywh(
                cx.viewport_origin.x + b.origin.x * cx.zoom,
                cx.viewport_origin.y + b.origin.y * cx.zoom,
                b.size.x * cx.zoom,
                b.size.y * cx.zoom,
            );
            let pos = Point2D::new(
                rect.origin.x + rect.size.x / 2.0,
                rect.origin.y + rect.size.y / 2.0,
            );
            out.push((
                tag.cloned(),
                node.id.clone(),
                Waypoint {
                    start_ms,
                    pos,
                    rect,
                },
            ));
        }
    }
    // Children compare against the NEAREST revealed ancestor: a section
    // revealed long after the root opens a fresh dwell window for its own
    // children (min-propagation would pin every window to the root's start).
    let child_revealed_ancestor_start_ms = reveal_start_ms.or(revealed_ancestor_start_ms);
    for child in &node.children {
        collect_waypoints(
            child,
            cx,
            tag,
            child_revealed_ancestor_start_ms,
            remaining,
            out,
        );
        if *remaining == 0 {
            break;
        }
    }
}

fn coalesce_dwell_waypoints(placements: Vec<(String, Waypoint)>) -> Vec<(String, Waypoint)> {
    let mut coalesced: Vec<(String, Waypoint)> = Vec::new();
    for placement in placements {
        if let Some((previous_id, previous)) = coalesced.last() {
            // Same skeleton section → one waypoint: keep the FIRST arrival
            // and dwell (the cursor stays parked while the section fills).
            if *previous_id == placement.0 {
                continue;
            }
            if placement.1.start_ms.saturating_sub(previous.start_ms) < DWELL_MS {
                *coalesced.last_mut().expect("last checked above") = placement;
                continue;
            }
        }
        coalesced.push(placement);
    }
    coalesced
}

/// Resolve every agent's cursor for this frame. Pure: same scene +
/// indicators + clock in, same sprites out.
pub(crate) fn cursor_sprites(
    roots: &[SceneNode],
    indicators: &AgentIndicators,
    viewport_origin: Point2D,
    zoom: f32,
    now_ms: u64,
) -> Vec<CursorSprite> {
    if indicators.reveals.is_empty() {
        return Vec::new();
    }
    let mut remaining = indicators.reveals.len();
    let mut tagged: Vec<(Option<AgentTag>, String, Waypoint)> = Vec::new();
    let collect_cx = CollectWaypointCx {
        indicators,
        viewport_origin,
        zoom,
    };
    for root in roots {
        collect_waypoints(root, &collect_cx, None, None, &mut remaining, &mut tagged);
        if remaining == 0 {
            break;
        }
    }
    // Group placements per owning agent; key `None` = untagged fallback.
    let mut groups: AgentGroups = AgentGroups::new();
    for (tag, id, wp) in tagged {
        let key = tag.as_ref().map(|t| (t.color.clone(), t.name.clone()));
        groups
            .entry(key)
            .or_insert_with(|| (tag.clone(), Vec::new()))
            .1
            .push((id, wp));
    }
    // Deterministic order for the sprite output: `None` (untagged) first,
    // then tagged groups sorted by their `(color, name)` key — sprite
    // order must not depend on HashMap iteration order.
    let mut ordered: Vec<_> = groups.into_iter().collect();
    ordered.sort_by(|(a, _), (b, _)| a.cmp(b));
    let mut sprites = Vec::new();
    for (_, (tag, mut placements)) in ordered {
        placements.sort_by(|a, b| a.1.start_ms.cmp(&b.1.start_ms).then_with(|| a.0.cmp(&b.0)));
        let waypoints: Vec<Waypoint> = coalesce_dwell_waypoints(placements)
            .into_iter()
            .map(|(_, wp)| wp)
            .collect();
        let Some(kin) = cursor_kinematics(&waypoints, now_ms) else {
            continue;
        };
        let color = tag
            .as_ref()
            .and_then(|t| parse_hex(&t.color))
            .unwrap_or(FALLBACK_COLOR);
        sprites.push(CursorSprite {
            pos: kin.pos,
            alpha: kin.alpha,
            color,
            name: tag.map(|t| t.name),
            current_rect: kin.current.map(|i| waypoints[i].rect),
        });
    }
    sprites
}

/// Paint every agent cursor for this frame. Called by `CanvasViewport`
/// right after the frame indicators so cursors sit on top of the
/// breathing glows and badges.
pub(crate) fn paint_agent_cursors(
    cx: &mut PaintCx<'_>,
    roots: &[SceneNode],
    viewport_origin: Point2D,
    zoom: f32,
    now_ms: u64,
    indicators: &AgentIndicators,
) {
    for sprite in cursor_sprites(roots, indicators, viewport_origin, zoom, now_ms) {
        paint_sprite(cx, &sprite, now_ms);
    }
}

/// Slow breathing cycle for the current-element border.
const BREATH_PERIOD_MS: u64 = 1_800;

fn paint_sprite(cx: &mut PaintCx<'_>, sprite: &CursorSprite, now_ms: u64) {
    if sprite.alpha <= 0.0 {
        return;
    }
    // Breathing border on the element being output right now — skeleton
    // blue (one generation language, not per-agent color), slow cycle so
    // it reads as "alive", never as an alert.
    if let Some(rect) = sprite.current_rect {
        if rect.size.x > 1.0 && rect.size.y > 1.0 {
            let phase = (now_ms % BREATH_PERIOD_MS) as f32 / BREATH_PERIOD_MS as f32;
            let breath = 0.5 - 0.5 * (phase * std::f32::consts::TAU).cos();
            let blue = super::canvas_generation_scan::SKELETON_BLUE;
            cx.backend.stroke_rect(
                rect,
                blue.with_alpha((0.25 + 0.45 * breath) * sprite.alpha),
                1.5,
            );
        }
    }
    let at = |points: &[(f32, f32)]| -> Vec<Point2D> {
        points
            .iter()
            .map(|(dx, dy)| Point2D::new(sprite.pos.x + dx, sprite.pos.y + dy))
            .collect()
    };
    let body = at(&PENCIL_BODY_POINTS);
    // Soft contact shadow first so the pencil lifts off the canvas.
    let shadow: Vec<Point2D> = body
        .iter()
        .map(|p| Point2D::new(p.x + 1.0, p.y + 1.5))
        .collect();
    cx.backend
        .fill_polygon(&shadow, Color::BLACK.with_alpha(0.18 * sprite.alpha));
    cx.backend
        .fill_polygon(&body, sprite.color.with_alpha(sprite.alpha));
    cx.backend
        .stroke_polygon(&body, Color::WHITE.with_alpha(0.9 * sprite.alpha), 1.5);
    // Sharpened tip: white graphite wedge + collar seam.
    cx.backend.fill_polygon(
        &at(&PENCIL_TIP_POINTS),
        Color::WHITE.with_alpha(0.95 * sprite.alpha),
    );
    let (c0, c1) = PENCIL_COLLAR;
    cx.backend.stroke_line(
        Point2D::new(sprite.pos.x + c0.0, sprite.pos.y + c0.1),
        Point2D::new(sprite.pos.x + c1.0, sprite.pos.y + c1.1),
        Color::WHITE.with_alpha(0.8 * sprite.alpha),
        1.2,
    );
    if let Some(name) = &sprite.name {
        paint_name_pill(cx, sprite, name);
    }
}

/// Agent-coloured capsule label hanging below-right of the pencil tail.
/// Pencil-parity metrics: tight capsule, text optically centred (the old
/// `y + 3 + FONT` baseline sat the label visibly LOW in the pill).
fn paint_name_pill(cx: &mut PaintCx<'_>, sprite: &CursorSprite, name: &str) {
    const FONT: f32 = 10.5;
    const PAD_X: f32 = 8.0;
    const PILL_H: f32 = 17.0;
    // Clear the pencil body's down-right diagonal (~22px) with a little air.
    const OFFSET_X: f32 = 18.0;
    const OFFSET_Y: f32 = 24.0;
    let name_w = cx.backend.measure_text(name, FONT);
    let pill = Rect::xywh(
        sprite.pos.x + OFFSET_X,
        sprite.pos.y + OFFSET_Y,
        name_w + PAD_X * 2.0,
        PILL_H,
    );
    cx.backend.fill_round_rect(
        pill,
        PILL_H / 2.0,
        sprite.color.with_alpha(0.95 * sprite.alpha),
    );
    let label = TextLayout::single_run(
        name,
        "system-ui",
        FONT,
        jian_core::scene::Color::rgba(255, 255, 255, (255.0 * sprite.alpha) as u8),
        Point2D::new(0.0, 0.0),
    );
    // Optical centring: baseline = centre + ~35% of the font size (cap
    // height ≈ 0.7em, so half of it below centre) — matches the label
    // centring the chrome buttons use.
    let baseline_y = pill.origin.y + PILL_H / 2.0 + FONT * 0.35;
    cx.backend
        .draw_text(&label, Point2D::new(pill.origin.x + PAD_X, baseline_y));
}
