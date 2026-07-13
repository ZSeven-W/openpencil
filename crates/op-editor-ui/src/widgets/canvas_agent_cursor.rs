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

const CLASSIC_BODY: [(f32, f32); 5] = [
    (0.00, 0.00),
    (8.10, 2.10),
    (21.10, 15.30),
    (15.30, 21.10),
    (2.10, 8.10),
];

const CLASSIC_TIP: [(f32, f32); 3] = [(0.00, 0.00), (4.10, 1.10), (1.10, 4.10)];

const CLASSIC_COLLAR: [(f32, f32); 2] = [(2.10, 8.10), (8.10, 2.10)];

const CHUBBY_BODY: [(f32, f32); 21] = [
    (0.00, 0.00),
    (9.50, 3.20),
    (10.07, 3.44),
    (10.57, 3.78),
    (11.00, 4.20),
    (18.40, 11.60),
    (19.10, 12.50),
    (19.46, 13.46),
    (19.46, 14.46),
    (19.10, 15.50),
    (18.40, 16.60),
    (16.60, 18.40),
    (15.50, 19.10),
    (14.46, 19.46),
    (13.46, 19.46),
    (12.50, 19.10),
    (11.60, 18.40),
    (4.20, 11.00),
    (3.78, 10.57),
    (3.44, 10.07),
    (3.20, 9.50),
];

const CHUBBY_TIP: [(f32, f32); 6] = [
    (0.00, 0.00),
    (5.20, 1.70),
    (4.16, 2.41),
    (3.22, 3.23),
    (2.41, 4.16),
    (1.70, 5.20),
];

const CHUBBY_COLLAR: [(f32, f32); 3] = [(3.20, 9.50), (6.30, 6.30), (9.50, 3.20)];

const CHUBBY_ERASER: [(f32, f32); 13] = [
    (15.20, 15.20),
    (18.40, 11.60),
    (19.10, 12.50),
    (19.46, 13.46),
    (19.46, 14.46),
    (19.10, 15.50),
    (18.40, 16.60),
    (16.60, 18.40),
    (15.50, 19.10),
    (14.46, 19.46),
    (13.46, 19.46),
    (12.50, 19.10),
    (11.60, 18.40),
];

const CRAYON_BODY: [(f32, f32); 20] = [
    (0.00, 0.00),
    (2.01, -0.01),
    (3.72, 0.28),
    (5.16, 0.84),
    (6.30, 1.70),
    (19.00, 14.40),
    (19.68, 15.31),
    (19.90, 16.23),
    (19.68, 17.16),
    (19.00, 18.10),
    (18.10, 19.00),
    (17.16, 19.68),
    (16.23, 19.90),
    (15.31, 19.68),
    (14.40, 19.00),
    (1.70, 6.30),
    (0.84, 5.16),
    (0.28, 3.72),
    (-0.01, 2.01),
    (0.00, 0.00),
];

const CRAYON_TIP: [(f32, f32); 11] = [
    (0.00, 0.00),
    (2.07, 0.04),
    (3.73, 0.44),
    (5.00, 1.20),
    (3.86, 1.74),
    (2.85, 2.55),
    (1.96, 3.64),
    (1.20, 5.00),
    (0.44, 3.73),
    (0.04, 2.07),
    (0.00, 0.00),
];

const MARKER_BODY: [(f32, f32); 18] = [
    (4.60, 0.90),
    (19.60, 13.40),
    (20.40, 14.33),
    (20.70, 15.30),
    (20.50, 16.32),
    (19.80, 17.40),
    (17.40, 19.80),
    (16.32, 20.50),
    (15.30, 20.70),
    (14.32, 20.40),
    (13.40, 19.60),
    (0.90, 4.60),
    (0.50, 3.80),
    (0.50, 3.00),
    (0.90, 2.20),
    (2.18, 1.00),
    (3.40, 0.00),
    (4.60, 0.90),
];

/// One selectable cursor look: baked point sets plus its two style quirks.
pub(crate) struct Silhouette {
    body: &'static [(f32, f32)],
    tip: &'static [(f32, f32)],
    collar: &'static [(f32, f32)],
    /// Chubby only: the pink eraser butt overlay.
    eraser: Option<&'static [(f32, f32)]>,
    /// Marker only: the tip is a white DOT, not a wedge.
    round_tip_dot: Option<(f32, f32, f32)>,
}

pub(crate) fn silhouette_for(style: op_editor_core::PencilCursorStyle) -> Silhouette {
    use op_editor_core::PencilCursorStyle as S;
    match style {
        S::Classic => Silhouette {
            body: &CLASSIC_BODY,
            tip: &CLASSIC_TIP,
            collar: &CLASSIC_COLLAR,
            eraser: None,
            round_tip_dot: None,
        },
        S::Rounded => Silhouette {
            body: &ROUNDED_BODY,
            tip: &ROUNDED_TIP,
            collar: &ROUNDED_COLLAR,
            eraser: None,
            round_tip_dot: None,
        },
        S::Chubby => Silhouette {
            body: &CHUBBY_BODY,
            tip: &CHUBBY_TIP,
            collar: &CHUBBY_COLLAR,
            eraser: Some(&CHUBBY_ERASER),
            round_tip_dot: None,
        },
        S::Crayon => Silhouette {
            body: &CRAYON_BODY,
            tip: &CRAYON_TIP,
            collar: &[],
            eraser: None,
            round_tip_dot: None,
        },
        S::Marker => Silhouette {
            body: &MARKER_BODY,
            tip: &[],
            collar: &[],
            eraser: None,
            round_tip_dot: Some((2.7, 2.7, 2.1)),
        },
    }
}

/// Fallback for reveals not owned by any tagged agent (the same red the
/// retired dashed-reveal border used as its untagged default).
/// O4 outline halo — near-black slate, drawn wide and translucent
/// OUTSIDE the white rim.
const PENCIL_HALO: Color = Color {
    r: 0.067,
    g: 0.094,
    b: 0.153,
    a: 1.0,
};

/// Chubby variant's eraser butt.
const PENCIL_ERASER_PINK: Color = Color {
    r: 1.0,
    g: 0.62,
    b: 0.70,
    a: 1.0,
};

const FALLBACK_COLOR: Color = Color {
    r: 1.0,
    g: 0.419,
    b: 0.419,
    a: 1.0,
};

/// Pencil-silhouette pointer, TIP at the origin pointing up-left, body
/// extending down-right at 45°, in screen px (zoom-independent). The
/// brand cursor: the working agent literally draws with a pencil —
/// agent-colour body, white outline, white rounded tip wedge (see
/// `PENCIL_TIP_POINTS`). Anatomy mirrors Pencil's multiplayer cursor
/// (solid tinted pointer + name pill) without copying its arrow.
///
/// ROUNDED variant (user-picked "B", then fattened 1.28x across the
/// pencil axis with the tip hotspot fixed - "加胖更可爱", 2026-07-12): every corner of the
/// straight-edged silhouette is a quadratic arc — round shoulders at the
/// eraser butt, a curved collar, a soft tip wedge. The arcs are baked
/// as dense polygon samples so the existing polygon painters render
/// them smoothly with no new backend primitive.
const ROUNDED_BODY: [(f32, f32); 19] = [
    (0.00, 0.00),
    (8.38, 1.22),
    (8.94, 1.35),
    (9.44, 1.58),
    (9.87, 1.93),
    (20.87, 12.93),
    (21.44, 13.79),
    (21.54, 14.76),
    (21.18, 15.85),
    (20.36, 17.04),
    (17.04, 20.36),
    (15.85, 21.18),
    (14.76, 21.54),
    (13.78, 21.44),
    (12.93, 20.87),
    (1.93, 9.87),
    (1.58, 9.44),
    (1.35, 8.94),
    (1.22, 8.38),
];

/// White wedge over the tip — the sharpened-graphite highlight, with a
/// curved base matching the rounded collar.
const ROUNDED_TIP: [(f32, f32); 6] = [
    (0.00, 0.00),
    (4.85, 0.75),
    (3.76, 1.69),
    (2.70, 2.70),
    (1.69, 3.76),
    (0.75, 4.85),
];

/// Collar seam between the sharpened cone and the painted body — a soft
/// arc sampled as a short polyline.
const ROUNDED_COLLAR: [(f32, f32); 5] = [
    (1.22, 8.38),
    (3.11, 6.49),
    (4.93, 4.67),
    (6.68, 2.91),
    (8.38, 1.22),
];

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
    style: op_editor_core::PencilCursorStyle,
) {
    let silhouette = silhouette_for(style);
    for sprite in cursor_sprites(roots, indicators, viewport_origin, zoom, now_ms) {
        paint_sprite(cx, &sprite, now_ms, &silhouette);
    }
}

/// Slow breathing cycle for the current-element border.
const BREATH_PERIOD_MS: u64 = 1_800;

/// Draw one style's silhouette at settings-swatch scale (unanimated,
/// theme-primary body) — the Settings > System picker's preview.
pub(crate) fn paint_cursor_swatch(
    cx: &mut PaintCx<'_>,
    style: op_editor_core::PencilCursorStyle,
    origin: Point2D,
    color: Color,
) {
    let silhouette = silhouette_for(style);
    let at = |points: &[(f32, f32)]| -> Vec<Point2D> {
        points
            .iter()
            .map(|(dx, dy)| Point2D::new(origin.x + dx, origin.y + dy))
            .collect()
    };
    let body = at(silhouette.body);
    paint_soft_halo(cx, &body, 1.0);
    paint_rim(cx, &body, 0.95);
    cx.backend.fill_polygon(&body, color);
    if let Some(eraser) = silhouette.eraser {
        cx.backend.fill_polygon(&at(eraser), PENCIL_ERASER_PINK);
    }
    if let Some((dx, dy, r)) = silhouette.round_tip_dot {
        cx.backend.fill_oval(
            Rect::xywh(origin.x + dx - r, origin.y + dy - r, r * 2.0, r * 2.0),
            Color::WHITE.with_alpha(0.95),
        );
    }
    if !silhouette.tip.is_empty() {
        cx.backend
            .fill_polygon(&at(silhouette.tip), Color::WHITE.with_alpha(0.95));
    }
}

/// Uniformly outset a silhouette by `offset` px about its centroid. Used for
/// both the halo layers and the white rim: a filled outset paints the rim as
/// GEOMETRY, so its width is exact everywhere and no stroke joins can notch it
/// (the trait's fallback polygon stroke drew each edge as its own capped
/// segment — every vertex of the densely-sampled arc showed a jaggy).
fn outset(body: &[Point2D], offset: f32, drop: f32) -> Vec<Point2D> {
    let n = body.len() as f32;
    let (mut sum_x, mut sum_y) = (0.0f32, 0.0f32);
    for p in body {
        sum_x += p.x;
        sum_y += p.y;
    }
    let (cx, cy) = (sum_x / n, sum_y / n);
    let radius = body
        .iter()
        .map(|p| ((p.x - cx).powi(2) + (p.y - cy).powi(2)).sqrt())
        .fold(1.0f32, f32::max);
    let k = 1.0 + offset / radius;
    body.iter()
        .map(|p| Point2D::new(cx + (p.x - cx) * k, cy + (p.y - cy) * k + drop))
        .collect()
}

/// Width of the white rim (px of outset beyond the body silhouette).
const RIM: f32 = 1.6;

/// Soft halo as concentric FILLED expansions of the silhouette, largest
/// first with per-layer alpha stacking smoothly toward the body. Filled
/// polygons have no stroke joins, so sharp silhouette corners cannot spike
/// (three concentric STROKES read as a dirty banded ring - user feedback
/// 2026-07-12, twice). The slight downward bias doubles as the contact
/// shadow, replacing the old hard-edged offset copy.
fn paint_soft_halo(cx: &mut PaintCx<'_>, body: &[Point2D], alpha_scale: f32) {
    // The halo sits OUTSIDE the white rim, so every layer clears it.
    for (offset, alpha) in [
        (RIM + 3.2, 0.030),
        (RIM + 2.4, 0.040),
        (RIM + 1.6, 0.050),
        (RIM + 0.8, 0.060),
    ] {
        let ring = outset(body, offset, 0.7);
        cx.backend
            .fill_polygon(&ring, PENCIL_HALO.with_alpha(alpha * alpha_scale));
    }
}

/// The white rim, painted as a filled outset the body then covers — a solid
/// ring of exactly `RIM` px with no stroke joins to notch it.
fn paint_rim(cx: &mut PaintCx<'_>, body: &[Point2D], alpha: f32) {
    cx.backend
        .fill_polygon(&outset(body, RIM, 0.0), Color::WHITE.with_alpha(alpha));
}

fn paint_sprite(cx: &mut PaintCx<'_>, sprite: &CursorSprite, now_ms: u64, silhouette: &Silhouette) {
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
    let body = at(silhouette.body);
    // Dark soft halo OUTSIDE the white rim (user-picked "O4", the macOS
    // pointer treatment) - filled-expansion feather, see paint_soft_halo.
    paint_soft_halo(cx, &body, sprite.alpha);
    paint_rim(cx, &body, 0.95 * sprite.alpha);
    cx.backend
        .fill_polygon(&body, sprite.color.with_alpha(sprite.alpha));
    // Style quirks first (they paint OVER the body):
    // Chubby's pink eraser butt, Marker's white nib dot.
    if let Some(eraser) = silhouette.eraser {
        cx.backend
            .fill_polygon(&at(eraser), PENCIL_ERASER_PINK.with_alpha(sprite.alpha));
        cx.backend.stroke_polygon(
            &at(eraser),
            Color::WHITE.with_alpha(0.9 * sprite.alpha),
            1.2,
        );
    }
    if let Some((dx, dy, r)) = silhouette.round_tip_dot {
        cx.backend.fill_oval(
            Rect::xywh(
                sprite.pos.x + dx - r,
                sprite.pos.y + dy - r,
                r * 2.0,
                r * 2.0,
            ),
            Color::WHITE.with_alpha(0.95 * sprite.alpha),
        );
    }
    // Sharpened tip: white graphite wedge + collar seam (skipped by
    // styles whose point sets are empty).
    if !silhouette.tip.is_empty() {
        cx.backend.fill_polygon(
            &at(silhouette.tip),
            Color::WHITE.with_alpha(0.95 * sprite.alpha),
        );
    }
    let collar = at(silhouette.collar);
    for seam in collar.windows(2) {
        cx.backend.stroke_line(
            seam[0],
            seam[1],
            Color::WHITE.with_alpha(0.8 * sprite.alpha),
            1.2,
        );
    }
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
