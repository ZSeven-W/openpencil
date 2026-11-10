//! Cursor swarm — opt-in showmanship around a running generation.
//!
//! Planning and each section are collected whole before anything lands, so
//! the canvas sits still for long stretches of a run. With the Settings >
//! System switch on, a handful of extra named cursors roam the board being
//! generated and the still-empty part of that board shows shimmering
//! skeleton blocks. Nothing here is real work: the decoys are presentation
//! only, never touch the document, and step aside as soon as a run has
//! several genuine agents (Team mode) so they cannot be mistaken for them.
//!
//! Like the real agent cursor, every pose is a pure function of the scene,
//! the indicator registry and `now_ms` — no swarm state is stored anywhere.

use super::canvas_agent_cursor::parse_hex;
use super::canvas_agent_cursor_motion::{parked_cursor_position, ParkedWindow};
use super::canvas_generation_scan::SKELETON_BLUE;
use crate::layout_scene::SceneNode;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};
use jian_core::motion::ease_in_out_cubic;
use op_editor_core::agent_indicators::AgentIndicators;
use std::collections::HashSet;

/// Decoy cursors shown next to the real one — enough to read as a crowd.
const DECOYS: usize = 12;
/// Candidate identities; the real agent's own name is skipped.
const ROSTER: [(&str, &str); 14] = [
    ("Pixel", "#FF6B6B"),
    ("Nova", "#4ECDC4"),
    ("Juno", "#FFB84D"),
    ("Echo", "#A78BFA"),
    ("Miso", "#F472B6"),
    ("Tofu", "#34D399"),
    ("Zest", "#60A5FA"),
    ("Luma", "#FB923C"),
    ("Orbit", "#22D3EE"),
    ("Pico", "#E879F9"),
    ("Volt", "#FACC15"),
    ("Rune", "#818CF8"),
    ("Dash", "#F87171"),
    ("Kite", "#2DD4BF"),
];
/// One hop (flight + dwell) of the first decoy; later ones run slightly
/// slower so the group never moves in step. Short on purpose: the swarm
/// should look busy, not contemplative.
const HOP_BASE_MS: u64 = 380;
const HOP_STEP_MS: u64 = 45;
/// Share of a hop spent flying; the rest is dwell with the idle sway.
const FLIGHT_FRACTION: f32 = 0.55;
/// Afterimages behind a flying decoy, and their spacing in time.
const TRAIL_DOTS: usize = 5;
const TRAIL_STEP_MS: u64 = 22;
/// Decoys closer than this share of the stage's longer side get linked.
const LINK_REACH_FRACTION: f32 = 0.3;
/// Landing ping: an expanding ring where a decoy touches down.
const PING_MS: u64 = 300;
/// Skeleton shimmer cycle.
const SHIMMER_MS: u64 = 1_600;
/// Most blocks laid out on one board.
const MAX_BLOCKS: usize = 24;
/// Smallest empty band (screen px) worth filling with skeleton blocks.
const MIN_SKELETON_BAND: f32 = 48.0;

/// One decoy cursor for this frame.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SwarmCursor {
    pub pos: Point2D,
    pub color: Color,
    pub name: &'static str,
    /// Recent positions, newest first — the motion streak.
    pub trail: [Point2D; TRAIL_DOTS],
    /// Landing ring: centre and 0..1 progress, right after touchdown.
    pub ping: Option<(Point2D, f32)>,
}

/// Everything the swarm paints this frame, in screen coordinates.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct SwarmFrame {
    /// The board (or stand-in area) the swarm works on.
    pub stage: Option<Rect>,
    /// Skeleton blocks in the board's still-empty band.
    pub skeleton: Vec<Rect>,
    pub cursors: Vec<SwarmCursor>,
}

/// Where the swarm may work, resolved from the scene.
struct Stage {
    rect: Rect,
    /// Screen y below which the board is still empty; `None` when the stage
    /// is only a stand-in (no board exists yet), which shows no skeleton.
    empty_from: Option<f32>,
}

/// Resolve the swarm for this frame. Empty unless the switch is on, a run is
/// live with a confirmed agent, and that run has a single real agent.
#[allow(clippy::too_many_arguments)]
pub(crate) fn swarm_frame(
    enabled: bool,
    roots: &[SceneNode],
    indicators: &AgentIndicators,
    viewport_origin: Point2D,
    zoom: f32,
    viewport: Rect,
    starter_ghost: Option<[f32; 4]>,
    now_ms: u64,
) -> SwarmFrame {
    let Some(primary) = indicators.cursor_agent.as_ref() else {
        return SwarmFrame::default();
    };
    if !enabled || !indicators.run_active || has_several_real_agents(indicators) {
        return SwarmFrame::default();
    }
    let to_screen = |r: Rect| Rect {
        origin: Point2D::new(
            viewport_origin.x + r.origin.x * zoom,
            viewport_origin.y + r.origin.y * zoom,
        ),
        size: Point2D::new(r.size.x * zoom, r.size.y * zoom),
    };
    let stage = match generating_root(roots, indicators, now_ms) {
        Some(root) => {
            let rect = to_screen(root.bounds);
            let content_bottom = root
                .children
                .iter()
                .filter(|child| is_visible(child, indicators, now_ms))
                .map(|child| {
                    let b = to_screen(child.bounds);
                    b.origin.y + b.size.y
                })
                .fold(rect.origin.y, f32::max);
            Stage {
                rect,
                empty_from: Some(content_bottom),
            }
        }
        None => match starter_ghost {
            Some([x, y, w, h]) => {
                let rect = to_screen(Rect::xywh(x, y, w, h));
                Stage {
                    rect,
                    empty_from: Some(rect.origin.y),
                }
            }
            None => Stage {
                rect: inset(viewport, 0.2),
                empty_from: None,
            },
        },
    };
    if stage.rect.size.x < 8.0 || stage.rect.size.y < 8.0 {
        return SwarmFrame::default();
    }

    let blocks = layout_blocks(stage.rect);
    let stage_bottom = stage.rect.origin.y + stage.rect.size.y;
    let skeleton = match stage.empty_from {
        Some(from) if stage_bottom - from >= MIN_SKELETON_BAND => blocks
            .iter()
            .copied()
            .filter(|b| b.origin.y >= from && b.size.x >= 6.0 && b.size.y >= 4.0)
            .collect(),
        _ => Vec::new(),
    };

    let cursors = ROSTER
        .iter()
        .filter(|(name, _)| !name.eq_ignore_ascii_case(&primary.name))
        .take(DECOYS)
        .enumerate()
        .map(|(index, (name, hex))| {
            let index = index as u64;
            let at = |ms: u64| decoy_position(index, &blocks, stage.rect, ms);
            SwarmCursor {
                pos: at(now_ms),
                color: parse_hex(hex).unwrap_or(SKELETON_BLUE),
                name,
                trail: std::array::from_fn(|k| {
                    at(now_ms.saturating_sub((k as u64 + 1) * TRAIL_STEP_MS))
                }),
                ping: landing_ping(index, &blocks, stage.rect, now_ms),
            }
        })
        .collect();
    SwarmFrame {
        stage: Some(stage.rect),
        skeleton,
        cursors,
    }
}

/// Team mode tags each screen group's frame with its own agent; two distinct
/// tags mean real concurrent agents are already on canvas.
fn has_several_real_agents(indicators: &AgentIndicators) -> bool {
    let distinct: HashSet<(&str, &str)> = indicators
        .frames
        .values()
        .map(|tag| (tag.color.as_str(), tag.name.as_str()))
        .collect();
    distinct.len() > 1
}

/// The board being generated: the root that owns the most recent started
/// reveal, else any root an agent has claimed.
fn generating_root<'a>(
    roots: &'a [SceneNode],
    indicators: &AgentIndicators,
    now_ms: u64,
) -> Option<&'a SceneNode> {
    let mut best: Option<(u64, &SceneNode)> = None;
    for root in roots {
        let claimed = indicators.frames.contains_key(&root.id);
        let latest = latest_reveal(root, indicators, now_ms);
        let score = match (latest, claimed) {
            (Some(started), _) => started.saturating_add(1),
            (None, true) => 0,
            (None, false) => continue,
        };
        if best.is_none_or(|(top, _)| score >= top) {
            best = Some((score, root));
        }
    }
    best.map(|(_, root)| root)
}

fn latest_reveal(node: &SceneNode, indicators: &AgentIndicators, now_ms: u64) -> Option<u64> {
    let own = indicators
        .reveals
        .get(&node.id)
        .copied()
        .filter(|started| *started <= now_ms);
    node.children
        .iter()
        .filter_map(|child| latest_reveal(child, indicators, now_ms))
        .chain(own)
        .max()
}

/// A child counts as landed once its reveal has started (or it never had one).
fn is_visible(node: &SceneNode, indicators: &AgentIndicators, now_ms: u64) -> bool {
    indicators
        .reveals
        .get(&node.id)
        .is_none_or(|started| *started <= now_ms)
}

fn inset(rect: Rect, fraction: f32) -> Rect {
    let (dx, dy) = (rect.size.x * fraction, rect.size.y * fraction);
    Rect {
        origin: Point2D::new(rect.origin.x + dx, rect.origin.y + dy),
        size: Point2D::new(rect.size.x - dx * 2.0, rect.size.y - dy * 2.0),
    }
}

/// A fixed wireframe over the whole stage: hero block, a row of three cards,
/// two text lines, repeated. It depends on the stage rect alone, so decoy
/// targets stay put while content lands; only the blocks below the content
/// are painted.
fn layout_blocks(stage: Rect) -> Vec<Rect> {
    let w = stage.size.x;
    let pad = (w * 0.06).clamp(6.0, 40.0);
    let gap = (w * 0.035).clamp(4.0, 24.0);
    let inner_w = w - pad * 2.0;
    let left = stage.origin.x + pad;
    let bottom = stage.origin.y + stage.size.y - pad;
    let mut y = stage.origin.y + pad;
    let mut blocks = Vec::new();
    let block = |x: f32, y: f32, w: f32, h: f32| Rect {
        origin: Point2D::new(x, y),
        size: Point2D::new(w, h),
    };
    for row in 0.. {
        let height = match row % 3 {
            0 => inner_w * 0.34,
            1 => inner_w * 0.22,
            _ => inner_w * 0.045,
        };
        if blocks.len() >= MAX_BLOCKS || y + height > bottom {
            break;
        }
        match row % 3 {
            0 => blocks.push(block(left, y, inner_w, height)),
            1 => {
                let card_w = (inner_w - gap * 2.0) / 3.0;
                for col in 0..3 {
                    blocks.push(block(left + col as f32 * (card_w + gap), y, card_w, height));
                }
            }
            _ => {
                blocks.push(block(left, y, inner_w * 0.7, height));
                y += height + gap * 0.6;
                if y + height <= bottom {
                    blocks.push(block(left, y, inner_w * 0.45, height));
                }
            }
        }
        y += height + gap;
    }
    blocks
}

/// SplitMix64 — a cheap, well-mixed deterministic hash.
fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// 0..1 from a hash.
fn unit(seed: u64) -> f32 {
    (mix(seed) >> 40) as f32 / (1u64 << 24) as f32
}

/// Where decoy `index` stops on hop `hop`: inside one of the wireframe
/// blocks, or anywhere on a stage too small to hold one.
fn hop_target(index: u64, hop: u64, blocks: &[Rect], stage: Rect) -> Point2D {
    let seed = index
        .wrapping_mul(0x1F3D)
        .wrapping_add(hop.wrapping_mul(0x9E3B));
    let area = if blocks.is_empty() {
        stage
    } else {
        blocks[(mix(seed) % blocks.len() as u64) as usize]
    };
    Point2D::new(
        area.origin.x + area.size.x * (0.2 + 0.6 * unit(seed ^ 0xA5)),
        area.origin.y + area.size.y * (0.2 + 0.6 * unit(seed ^ 0x5A)),
    )
}

/// `(period, clock, hop number, ms into the hop, flight ms)` for a decoy.
fn hop_clock(index: u64, now_ms: u64) -> (u64, u64, u64, u64, u64) {
    let period = HOP_BASE_MS + HOP_STEP_MS * index;
    let clock = now_ms + index * 137;
    let flight = (period as f32 * FLIGHT_FRACTION) as u64;
    (period, clock, clock / period, clock % period, flight)
}

/// The ring a decoy leaves where it has just landed.
fn landing_ping(index: u64, blocks: &[Rect], stage: Rect, now_ms: u64) -> Option<(Point2D, f32)> {
    let (_, _, hop, into, flight) = hop_clock(index, now_ms);
    let since = into.checked_sub(flight).filter(|ms| *ms < PING_MS)?;
    Some((
        hop_target(index, hop, blocks, stage),
        since as f32 / PING_MS as f32,
    ))
}

/// Decoy pose: fly to this hop's target, then dwell there with the same idle
/// sway the real cursor uses, enveloped to zero before the next departure.
fn decoy_position(index: u64, blocks: &[Rect], stage: Rect, now_ms: u64) -> Point2D {
    let (period, clock, hop, into, flight) = hop_clock(index, now_ms);
    let to = hop_target(index, hop, blocks, stage);
    if into < flight {
        let from = hop_target(index, hop.wrapping_sub(1), blocks, stage);
        let t = ease_in_out_cubic(into as f32 / flight.max(1) as f32);
        return Point2D::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t);
    }
    // Windows live on the decoy's own (offset) clock; the sway only reads
    // elapsed time, so any consistent clock works.
    let hop_start = clock - into;
    parked_cursor_position(
        to,
        ParkedWindow {
            since_ms: hop_start + flight,
            until_ms: Some(hop_start + period),
        },
        clock,
    )
}

/// Skeleton blocks a decoy has just landed on: `(block, colour, 1..0 fade)`.
/// The block lights up in that decoy's colour and settles back.
fn block_flashes(frame: &SwarmFrame) -> Vec<(Rect, Color, f32)> {
    frame
        .cursors
        .iter()
        .filter_map(|cursor| {
            let (centre, progress) = cursor.ping?;
            let block = frame.skeleton.iter().find(|b| b.contains(centre))?;
            Some((*block, cursor.color, (1.0 - progress).powi(2)))
        })
        .collect()
}

/// Constellation lines between decoys that are close to each other:
/// `(from, to, colour, 0..1 strength)`, stronger the closer the pair.
fn cursor_links(frame: &SwarmFrame) -> Vec<(Point2D, Point2D, Color, f32)> {
    let Some(stage) = frame.stage else {
        return Vec::new();
    };
    let reach = (stage.size.x.max(stage.size.y) * LINK_REACH_FRACTION).clamp(80.0, 420.0);
    let mut links = Vec::new();
    for (i, a) in frame.cursors.iter().enumerate() {
        for b in &frame.cursors[i + 1..] {
            let (dx, dy) = (a.pos.x - b.pos.x, a.pos.y - b.pos.y);
            let distance = (dx * dx + dy * dy).sqrt();
            if distance < reach {
                links.push((a.pos, b.pos, a.color, 1.0 - distance / reach));
            }
        }
    }
    links
}

/// Paint the skeleton blocks and decoy cursors. Called right before the real
/// agent cursor so that one stays on top.
pub(crate) fn paint_swarm(
    cx: &mut PaintCx<'_>,
    frame: &SwarmFrame,
    now_ms: u64,
    style: op_editor_core::PencilCursorStyle,
) {
    if let Some(stage) = frame.stage.filter(|_| !frame.skeleton.is_empty()) {
        cx.backend.save();
        cx.backend.clip_rect(stage);
        for (index, block) in frame.skeleton.iter().enumerate() {
            let phase = ((now_ms + index as u64 * 140) % SHIMMER_MS) as f32 / SHIMMER_MS as f32;
            let breath = 0.5 - 0.5 * (phase * std::f32::consts::TAU).cos();
            let radius = (block.size.y * 0.18).min(8.0);
            cx.backend.fill_round_rect(
                *block,
                radius,
                SKELETON_BLUE.with_alpha(0.05 + 0.07 * breath),
            );
            cx.backend
                .stroke_round_rect(*block, radius, SKELETON_BLUE.with_alpha(0.28), 1.0);
        }
        for (block, color, fade) in block_flashes(frame) {
            let radius = (block.size.y * 0.18).min(8.0);
            cx.backend
                .fill_round_rect(block, radius, color.with_alpha(0.38 * fade));
            cx.backend
                .stroke_round_rect(block, radius, color.with_alpha(0.9 * fade), 1.5);
        }
        cx.backend.restore();
    }
    for (from, to, color, strength) in cursor_links(frame) {
        cx.backend
            .stroke_line(from, to, color.with_alpha(0.42 * strength), 1.0);
    }
    let dot = |centre: Point2D, r: f32| Rect {
        origin: Point2D::new(centre.x - r, centre.y - r),
        size: Point2D::new(r * 2.0, r * 2.0),
    };
    for cursor in &frame.cursors {
        if let Some((centre, progress)) = cursor.ping {
            let r = 5.0 + 22.0 * progress;
            let fade = (1.0 - progress).powi(2);
            cx.backend
                .fill_round_rect(dot(centre, r), r, cursor.color.with_alpha(0.16 * fade));
            cx.backend.stroke_round_rect(
                dot(centre, r),
                r,
                cursor.color.with_alpha(0.8 * fade),
                1.5,
            );
        }
        for (k, point) in cursor.trail.iter().enumerate().rev() {
            let age = (k + 1) as f32 / (TRAIL_DOTS + 1) as f32;
            let r = 4.0 * (1.0 - age) + 1.0;
            cx.backend.fill_round_rect(
                dot(*point, r),
                r,
                cursor.color.with_alpha(0.5 * (1.0 - age)),
            );
        }
    }
    for cursor in &frame.cursors {
        super::canvas_agent_cursor::paint_presence_cursor(
            cx,
            cursor.pos,
            cursor.color,
            cursor.name,
            style,
        );
    }
}

#[cfg(test)]
#[path = "canvas_agent_swarm_tests.rs"]
mod tests;
