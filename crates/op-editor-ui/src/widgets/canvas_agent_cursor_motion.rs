//! Pure waypoint and idle-motion derivation for the generation cursor.

use crate::{Point2D, Rect};

/// Longest single flight; longer waypoint gaps depart late, arrive on time.
const MAX_FLIGHT_MS: u64 = 350;
/// Fade-in slide duration before a queue's first waypoint.
const ENTRY_MS: u64 = 250;
/// Where the entry slide starts, relative to the first waypoint (screen px).
const ENTRY_OFFSET_X: f32 = -28.0;
const ENTRY_OFFSET_Y: f32 = -20.0;

/// The parked cursor waits briefly before beginning its subtle idle sway.
const IDLE_SETTLE_MS: u64 = 220;
/// Motion and rest share one slow cycle so the cursor reads as breathing.
const IDLE_PERIOD_MS: u64 = 1_800;
/// Fade the sway in and back out before a scheduled flight.
const IDLE_ENVELOPE_MS: u64 = 220;
/// Maximum screen-pixel displacement. This is deliberately independent of zoom.
const IDLE_SWAY_X: f32 = 3.0;
const IDLE_SWAY_Y: f32 = 1.5;

/// A scheduled placement the cursor must reach: one generated node's
/// reveal start, at that node's centre, plus the node's screen rect for
/// the current-element breathing border.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Waypoint {
    pub start_ms: u64,
    pub pos: Point2D,
    pub rect: Rect,
}

/// A period in which the cursor is anchored and may show idle motion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ParkedWindow {
    pub since_ms: u64,
    pub until_ms: Option<u64>,
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
    /// Present only while waiting at an already-reached waypoint.
    pub parked: Option<ParkedWindow>,
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

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Add a tiny figure-eight sway while a live generation cursor is parked.
/// The envelope is zero on arrival and at a scheduled departure, so the
/// real waypoint flight never jumps or inherits the idle displacement.
pub(crate) fn parked_cursor_position(
    anchor: Point2D,
    parked: ParkedWindow,
    now_ms: u64,
) -> Point2D {
    let elapsed = now_ms.saturating_sub(parked.since_ms);
    if elapsed <= IDLE_SETTLE_MS {
        return anchor;
    }

    let motion_elapsed = elapsed - IDLE_SETTLE_MS;
    let enter = smoothstep(motion_elapsed as f32 / IDLE_ENVELOPE_MS as f32);
    let exit = parked.until_ms.map_or(1.0, |until_ms| {
        smoothstep(until_ms.saturating_sub(now_ms) as f32 / IDLE_ENVELOPE_MS as f32)
    });
    let envelope = enter * exit;
    if envelope <= f32::EPSILON {
        return anchor;
    }

    let phase =
        (motion_elapsed % IDLE_PERIOD_MS) as f32 / IDLE_PERIOD_MS as f32 * std::f32::consts::TAU;
    Point2D::new(
        anchor.x + phase.sin() * IDLE_SWAY_X * envelope,
        anchor.y + (phase * 2.0).sin() * IDLE_SWAY_Y * envelope,
    )
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
            parked: None,
        });
    }

    let current = Some(next_idx - 1);
    let prev = &waypoints[next_idx - 1];
    if next_idx == waypoints.len() {
        return Some(Kinematics {
            pos: prev.pos,
            alpha: 1.0,
            current,
            parked: Some(ParkedWindow {
                since_ms: prev.start_ms,
                until_ms: None,
            }),
        });
    }

    let next = &waypoints[next_idx];
    let gap = next.start_ms.saturating_sub(prev.start_ms).max(1);
    let flight = MAX_FLIGHT_MS.min((gap as f64 * 0.7) as u64).max(1);
    let depart = prev.start_ms.max(next.start_ms.saturating_sub(flight));
    if now_ms < depart {
        return Some(Kinematics {
            pos: prev.pos,
            alpha: 1.0,
            current,
            parked: Some(ParkedWindow {
                since_ms: prev.start_ms,
                until_ms: Some(depart),
            }),
        });
    }

    let window = next.start_ms.saturating_sub(depart).max(1);
    let t = ((now_ms - depart) as f32 / window as f32).clamp(0.0, 1.0);
    Some(Kinematics {
        pos: lerp(prev.pos, next.pos, ease_in_out_cubic(t)),
        alpha: 1.0,
        current,
        parked: None,
    })
}
