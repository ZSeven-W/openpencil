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
/// Fade-in slide duration before a queue's first waypoint.
const ENTRY_MS: u64 = 250;
/// Where the entry slide starts, relative to the first waypoint (screen px).
const ENTRY_OFFSET_X: f32 = -28.0;
const ENTRY_OFFSET_Y: f32 = -20.0;
/// One full breathe (0 → 1 → 0) of the current-element border per this
/// many ms — same cadence as the agent frame glow.
const BORDER_BREATH_PERIOD_MS: u64 = 1_200;
/// Fallback for reveals not owned by any tagged agent (the same red the
/// retired dashed-reveal border used as its untagged default).
const FALLBACK_COLOR: Color = Color {
    r: 1.0,
    g: 0.419,
    b: 0.419,
    a: 1.0,
};

/// Classic arrow-pointer silhouette, tip at the origin pointing up-left,
/// in screen px (zoom-independent). Filled with the agent colour and
/// outlined white, like a multiplayer cursor.
const ARROW_POINTS: [(f32, f32); 7] = [
    (0.0, 0.0),
    (0.0, 14.5),
    (3.6, 11.4),
    (6.1, 16.6),
    (8.7, 15.4),
    (6.2, 10.3),
    (10.9, 10.3),
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
    let depart = prev
        .start_ms
        .max(next.start_ms.saturating_sub(MAX_FLIGHT_MS));
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
        pos: lerp(prev.pos, next.pos, ease_out_cubic(t)),
        alpha: 1.0,
        current,
    })
}

/// Placements grouped by owning agent (`None` key = untagged fallback);
/// each group carries its tag plus `(node id, waypoint)` pairs.
type AgentGroups = HashMap<Option<(String, String)>, (Option<AgentTag>, Vec<(String, Waypoint)>)>;

/// One agent's cursor, fully resolved for painting this frame.
#[derive(Debug, Clone)]
pub(crate) struct CursorSprite {
    pub pos: Point2D,
    pub alpha: f32,
    pub color: Color,
    pub name: Option<String>,
    /// Screen rect of the element the cursor is currently working on —
    /// it gets the breathing border.
    pub current_rect: Option<Rect>,
}

/// Collect `(owning agent, node id, waypoint)` for every scheduled reveal,
/// inheriting agent tags down the tree the same way the frame-indicator
/// overlay does. `remaining` early-stops the walk once every reveal in
/// the snapshot has been located.
fn collect_waypoints(
    node: &SceneNode,
    indicators: &AgentIndicators,
    inherited: Option<&AgentTag>,
    viewport_origin: Point2D,
    zoom: f32,
    remaining: &mut usize,
    out: &mut Vec<(Option<AgentTag>, String, Waypoint)>,
) {
    if *remaining == 0 {
        return;
    }
    let tag = indicators
        .nodes
        .get(&node.id)
        .or_else(|| indicators.frames.get(&node.id))
        .or(inherited);
    if let Some(start_ms) = indicators.reveals.get(&node.id).copied() {
        *remaining -= 1;
        let b = node.aggregate_bounds();
        if b.size.x > 0.0 && b.size.y > 0.0 {
            let rect = Rect::xywh(
                viewport_origin.x + b.origin.x * zoom,
                viewport_origin.y + b.origin.y * zoom,
                b.size.x * zoom,
                b.size.y * zoom,
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
    for child in &node.children {
        collect_waypoints(
            child,
            indicators,
            tag,
            viewport_origin,
            zoom,
            remaining,
            out,
        );
        if *remaining == 0 {
            break;
        }
    }
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
    for root in roots {
        collect_waypoints(
            root,
            indicators,
            None,
            viewport_origin,
            zoom,
            &mut remaining,
            &mut tagged,
        );
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
        let waypoints: Vec<Waypoint> = placements.into_iter().map(|(_, wp)| wp).collect();
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

/// Bell curve 0 → 1 → 0 across the breath period.
fn border_breath(now_ms: u64) -> f32 {
    let phase = (now_ms % BORDER_BREATH_PERIOD_MS) as f32 / BORDER_BREATH_PERIOD_MS as f32;
    (phase * std::f32::consts::PI).sin()
}

fn paint_sprite(cx: &mut PaintCx<'_>, sprite: &CursorSprite, now_ms: u64) {
    if sprite.alpha <= 0.0 {
        return;
    }
    // Breathing border on the element currently being placed: a soft
    // outer wash plus a crisp ring that never fully disappears, so the
    // "current element" stays identifiable through the whole beat.
    if let Some(rect) = sprite.current_rect {
        let breath = border_breath(now_ms);
        cx.backend.stroke_round_rect(
            rect,
            6.0,
            sprite.color.with_alpha(breath * 0.35 * sprite.alpha),
            3.0,
        );
        cx.backend.stroke_round_rect(
            rect,
            6.0,
            sprite
                .color
                .with_alpha((0.35 + 0.65 * breath) * sprite.alpha),
            1.5,
        );
    }
    let pts: Vec<Point2D> = ARROW_POINTS
        .iter()
        .map(|(dx, dy)| Point2D::new(sprite.pos.x + dx, sprite.pos.y + dy))
        .collect();
    cx.backend
        .fill_polygon(&pts, sprite.color.with_alpha(sprite.alpha));
    cx.backend
        .stroke_polygon(&pts, Color::WHITE.with_alpha(0.9 * sprite.alpha), 1.5);
    if let Some(name) = &sprite.name {
        paint_name_pill(cx, sprite, name);
    }
}

/// Agent-coloured capsule label hanging below-right of the pointer.
fn paint_name_pill(cx: &mut PaintCx<'_>, sprite: &CursorSprite, name: &str) {
    const FONT: f32 = 11.0;
    const PAD_X: f32 = 7.0;
    const OFFSET_X: f32 = 12.0;
    const OFFSET_Y: f32 = 20.0;
    let name_w = cx.backend.measure_text(name, FONT);
    let h = FONT + 6.0;
    let pill = Rect::xywh(
        sprite.pos.x + OFFSET_X,
        sprite.pos.y + OFFSET_Y,
        name_w + PAD_X * 2.0,
        h,
    );
    cx.backend
        .fill_round_rect(pill, h / 2.0, sprite.color.with_alpha(0.92 * sprite.alpha));
    let label = TextLayout::single_run(
        name,
        "system-ui",
        FONT,
        jian_core::scene::Color::rgba(255, 255, 255, (255.0 * sprite.alpha) as u8),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label,
        Point2D::new(pill.origin.x + PAD_X, pill.origin.y + 3.0 + FONT),
    );
}
