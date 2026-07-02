//! Process-global registry of active "agent team" canvas indicators.
//!
//! While a concurrent design generation is running, each parallel
//! sub-agent claims the nodes / root frame it is building and tags them
//! with its colour + name. The canvas reads this registry every frame
//! to paint a breathing border per node, a glow + badge per agent root
//! frame, and a faint pulse over not-yet-materialised preview nodes.
//!
//! A *process-global* (rather than a field on `EditorState`) mirrors the
//! TS `agent-indicator.ts` `globalThis` registry: the design worker that
//! streams nodes in and the paint pass that draws them live in different
//! places, and threading a registry through every command applier would
//! be far more invasive. Access is serialized through a `Mutex`; on the
//! single-threaded UI / wasm targets the lock is always uncontended.

use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};

/// Window after a reveal starts during which the entrance animation
/// (scale-pop) plays and the reveal counts as "animating" for redraw
/// scheduling. While a run is active reveals are retained past this
/// window (they anchor the cursor + current-element border); they are
/// only dropped wholesale when the run ends.
pub const REVEAL_DURATION_MS: u64 = 1_000;
/// Delay between generated nodes in the placement queue. Deliberately
/// slow (user-tuned): each element gets a readable fly-in → pop beat
/// instead of a burst.
pub const REVEAL_STAGGER_MS: u64 = 160;
/// Minimum delay before descendants of a newly revealed container begin
/// their own entrances. This leaves the parent opening beat readable
/// without making nested content feel stalled.
pub const REVEAL_CHILD_RUNWAY_MS: u64 = 72;
/// Extra delay for nested generated nodes. Visual traversal order
/// already places children after parents; keeping this at zero avoids
/// depth changes compressing adjacent stream slots into the same frame.
pub const REVEAL_DEPTH_STAGGER_MS: u64 = 0;
/// Parent reveals suppress nested child borders only during their first
/// frame beat, avoiding stacked outlines when a generated container and
/// its first children become ready together.
pub const REVEAL_CHILD_SUPPRESS_FRACTION: f32 = 0.04;
const REVEAL_MAX_NEW_STARTS_PER_SNAPSHOT: usize = 1;
pub(crate) const REVEAL_BURST_RECOVERY_STAGGER_MS: u64 = REVEAL_STAGGER_MS;
const CLOCK_REBASE_THRESHOLD_MS: u64 = 60_000;
pub(crate) const REVEAL_FRAME_MS: u64 = 16;

/// A node's / frame's owning agent — colour hex (e.g. `"#FF6B6B"`) + name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTag {
    pub color: String,
    pub name: String,
}

/// Snapshot of all active indicators, returned to the paint pass.
#[derive(Debug, Clone, Default)]
pub struct AgentIndicators {
    /// Monotonic run epoch — bumped by [`begin`], checked by
    /// [`clear_if_epoch`]. Kept *inside* the registry so the
    /// check-and-clear happens under one lock and a stale run can't wipe
    /// the run that replaced it (the check + clear must be atomic — a
    /// separate atomic counter would reintroduce a check-then-act race).
    epoch: u64,
    /// node id → owning agent. Drives the dashed breathing node border.
    pub nodes: HashMap<String, AgentTag>,
    /// root-frame id → owning agent. Drives the glow + name badge.
    pub frames: HashMap<String, AgentTag>,
    /// node ids that are claimed but not yet drawn — drives the preview
    /// pulse fill.
    pub previews: HashSet<String>,
    /// node id → reveal start timestamp. Drives the new-node entrance
    /// animation (scale-pop), the agent cursor's placement queue, and
    /// the current-element breathing border. Retained for the whole
    /// run (see [`REVEAL_DURATION_MS`]).
    pub reveals: HashMap<String, u64>,
    /// `true` from [`begin`] until the run's clear/end — keeps the
    /// cursor + current-element border alive between streamed chunks
    /// even when no reveal is inside its animation window.
    pub run_active: bool,
    /// `true` once a finished run is draining ([`finish_if_epoch`]):
    /// retention is off and the overlay clears itself as soon as the
    /// already-queued reveals finish playing out.
    finishing: bool,
    /// The drain just cleared the overlay, but the frame on screen was
    /// painted BEFORE the clear — one more repaint is needed to erase
    /// the cursor. Sticky until the paint path consumes it (the
    /// clear happens inside the host's "anything animating?" probe, so
    /// without this the probe answers "no" at the exact moment a final
    /// erase frame is required and the cursor lingers until the next
    /// user interaction).
    needs_final_frame: bool,
    last_reveal_snapshot_ms: Option<u64>,
}

impl AgentIndicators {
    /// Drop every indicator without touching the epoch.
    fn clear_maps(&mut self) {
        self.nodes.clear();
        self.frames.clear();
        self.previews.clear();
        self.reveals.clear();
        self.run_active = false;
        self.finishing = false;
        self.needs_final_frame = false;
        self.last_reveal_snapshot_ms = None;
    }
}

static REGISTRY: LazyLock<Mutex<AgentIndicators>> =
    LazyLock::new(|| Mutex::new(AgentIndicators::default()));

/// Tag a single node with its owning agent (dashed breathing border).
///
/// Scoped to `epoch`: a registration whose epoch is no longer the active
/// run is dropped on the floor, so a stale / cancelled run streaming
/// nodes in late can't pollute the run that replaced it. The epoch check
/// and the insert share the registry lock, so a `begin` can't slip in
/// between them.
pub fn add_node(epoch: u64, node_id: &str, color: &str, name: &str) {
    let mut r = REGISTRY.lock().unwrap();
    if r.epoch != epoch {
        return;
    }
    r.nodes.insert(
        node_id.to_string(),
        AgentTag {
            color: color.to_string(),
            name: name.to_string(),
        },
    );
}

/// Tag a root frame with its owning agent (glow + badge). Epoch-scoped
/// like [`add_node`] — a stale run's registration is dropped.
pub fn add_frame(epoch: u64, frame_id: &str, color: &str, name: &str) {
    let mut r = REGISTRY.lock().unwrap();
    if r.epoch != epoch {
        return;
    }
    r.frames.insert(
        frame_id.to_string(),
        AgentTag {
            color: color.to_string(),
            name: name.to_string(),
        },
    );
}

/// Mark a node as a not-yet-materialised preview (pulse fill).
/// Epoch-scoped like [`add_node`].
pub fn mark_preview(epoch: u64, node_id: &str) {
    let mut r = REGISTRY.lock().unwrap();
    if r.epoch != epoch {
        return;
    }
    r.previews.insert(node_id.to_string());
}

/// Drop a node from the preview set once it has real geometry.
/// Epoch-scoped like [`add_node`].
pub fn clear_preview(epoch: u64, node_id: &str) {
    let mut r = REGISTRY.lock().unwrap();
    if r.epoch != epoch {
        return;
    }
    r.previews.remove(node_id);
}

/// Start a short entrance animation for a node that has just been
/// materialised by an AI generation command.
pub fn add_reveal(epoch: u64, node_id: &str, started_at_ms: u64) {
    let mut r = REGISTRY.lock().unwrap();
    if r.epoch != epoch {
        return;
    }
    r.reveals.insert(node_id.to_string(), started_at_ms);
}

/// Start offset for a generated node inside one applied batch.
///
/// The queue follows the visual traversal order of the newly applied
/// subtree rather than a per-parent sibling index. It is a UNIFORM
/// queue: every placement gets the same [`REVEAL_STAGGER_MS`] beat, so
/// dense batches stream out one readable element at a time instead of
/// compressing into a burst.
pub fn reveal_offset_ms(depth: u64, stream_index: u64) -> u64 {
    depth
        .saturating_mul(REVEAL_DEPTH_STAGGER_MS)
        .saturating_add(stream_index.saturating_mul(REVEAL_STAGGER_MS))
}

/// Clear every indicator — called when a generation finishes / is reset.
pub fn clear() {
    REGISTRY.lock().unwrap().clear_maps();
}

/// Begin a new run: bump the epoch, clear any prior indicators, and
/// return the new epoch for the caller to hold and pass to
/// [`clear_if_epoch`] on teardown.
pub fn begin() -> u64 {
    let mut r = REGISTRY.lock().unwrap();
    r.epoch += 1;
    r.clear_maps();
    r.run_active = true;
    r.epoch
}

/// Clear indicators only if `epoch` is still the active run. A newer
/// [`begin`] makes this a no-op, so a stale / cancelled run finishing
/// late can't wipe the indicators of the run that replaced it. The
/// epoch compare and the clear share the registry lock, so a `begin`
/// racing in between can't slip its fresh indicators through.
pub fn clear_if_epoch(epoch: u64) {
    let mut r = REGISTRY.lock().unwrap();
    if r.epoch == epoch {
        r.clear_maps();
    }
}

/// Gracefully end the run identified by `epoch` after its work is done:
/// stop run-long reveal retention and let the already-queued reveals
/// play out at the queue cadence; once the last one leaves its
/// animation window the whole overlay (glow, badges, cursor, borders)
/// clears itself and the epoch is retired. A no-op if a newer run took
/// over. Use for NATURAL completion — a user stop wants
/// [`end_if_epoch`]'s immediate clear so a cancelled generation doesn't
/// keep animating.
pub fn finish_if_epoch(epoch: u64) {
    let mut r = REGISTRY.lock().unwrap();
    if r.epoch != epoch {
        return;
    }
    r.run_active = false;
    r.finishing = true;
    drain_finished_run(&mut r);
}

/// Once a finishing run's reveal queue has fully played out (or was
/// empty to begin with), clear the whole overlay and retire the epoch.
/// Flags one final erase frame so the host repaints the (now empty)
/// overlay instead of leaving the last-painted cursor on screen.
fn drain_finished_run(r: &mut AgentIndicators) {
    if r.finishing && r.reveals.is_empty() {
        r.clear_maps();
        r.epoch += 1;
        r.needs_final_frame = true;
    }
}

/// End the run identified by `epoch`: if it's still the active run, clear
/// its indicators *and retire the epoch* (bump it), so any registration
/// still in flight from that run's worker no-ops instead of re-populating
/// the set we just cleared. A no-op if a newer run already took over.
///
/// This is what the host calls when a turn is stopped: `clear_if_epoch`
/// alone would clear, but a worker mid `add_frame` loop (scaffold just
/// built, first cancel-detecting channel op not yet reached) would re-add
/// under the unchanged epoch. Retiring the epoch closes that window.
pub fn end_if_epoch(epoch: u64) {
    let mut r = REGISTRY.lock().unwrap();
    if r.epoch == epoch {
        r.clear_maps();
        r.epoch += 1;
    }
}

/// `true` while any node / frame indicator is active — the paint loop
/// uses this to keep requesting redraws so the breathing animates.
pub fn is_active() -> bool {
    let r = REGISTRY.lock().unwrap();
    has_active_indicators(&r)
}

/// A clone of the current indicators for the paint pass to read.
pub fn snapshot() -> AgentIndicators {
    REGISTRY.lock().unwrap().clone()
}

/// A clone of the current indicators with reveal animations expired at
/// `now_ms` pruned. The paint pass calls this every frame so completed
/// reveal animations stop requesting redraws.
pub fn snapshot_at(now_ms: u64) -> AgentIndicators {
    snapshot_at_if_active(now_ms).unwrap_or_default()
}

/// A clone of the current indicators after reveal maintenance, or
/// `None` when there is nothing active for paint to consume.
pub fn snapshot_at_if_active(now_ms: u64) -> Option<AgentIndicators> {
    let mut r = REGISTRY.lock().unwrap();
    if !has_active_indicators(&r) {
        // This paint IS the post-drain erase frame — consume the flag.
        r.needs_final_frame = false;
        return None;
    }
    rebase_external_clock_reveals(&mut r, now_ms);
    // While the run is live, settled reveals are retained — they anchor
    // the cursor's parked position and the current-element border
    // between streamed chunks. Only a dead run's stragglers prune.
    if !r.run_active {
        r.reveals
            .retain(|_, started| now_ms.saturating_sub(*started) <= REVEAL_DURATION_MS);
    }
    drain_finished_run(&mut r);
    smooth_overdue_reveal_burst(&mut r, now_ms);
    if has_active_indicators(&r) {
        Some(r.clone())
    } else {
        // The drain fired inside THIS paint's snapshot — the caller is
        // about to paint the empty overlay, which erases the cursor.
        r.needs_final_frame = false;
        None
    }
}

/// Next host-clock millisecond needed for generated-node reveal animation.
pub fn next_reveal_deadline_ms(now_ms: u64) -> Option<u64> {
    let mut r = REGISTRY.lock().unwrap();
    rebase_external_clock_reveals(&mut r, now_ms);
    if !r.run_active {
        r.reveals
            .retain(|_, started| now_ms.saturating_sub(*started) <= REVEAL_DURATION_MS);
    }
    drain_finished_run(&mut r);
    let next_start = r
        .reveals
        .values()
        .filter(|started| **started > now_ms)
        .min()
        .copied();
    let active = r
        .reveals
        .values()
        .any(|started| *started <= now_ms && now_ms.saturating_sub(*started) <= REVEAL_DURATION_MS);
    match (next_start, active) {
        (Some(start), true) => Some(start.min(now_ms.saturating_add(REVEAL_FRAME_MS))),
        (Some(start), false) => Some(start),
        (None, true) => Some(now_ms.saturating_add(REVEAL_FRAME_MS)),
        // Nothing animating — but if the drain just cleared the overlay,
        // keep asking for one more frame until the paint path consumes
        // the erase (otherwise the last-painted cursor stays on screen).
        (None, false) => r
            .needs_final_frame
            .then(|| now_ms.saturating_add(REVEAL_FRAME_MS)),
    }
}

fn rebase_external_clock_reveals(r: &mut AgentIndicators, now_ms: u64) {
    let future_floor = now_ms.saturating_add(CLOCK_REBASE_THRESHOLD_MS);
    let mut external: Vec<(String, u64)> = r
        .reveals
        .iter()
        .filter_map(|(id, started)| {
            if *started > future_floor {
                Some((id.clone(), *started))
            } else {
                None
            }
        })
        .collect();
    if external.is_empty() {
        return;
    }
    external.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    let batch_start = external[0].1;
    let local_tail = r
        .reveals
        .values()
        .filter(|started| **started <= future_floor)
        .max()
        .copied();
    let mut next_slot = local_tail
        .map(|tail| tail.saturating_add(REVEAL_STAGGER_MS))
        .unwrap_or(now_ms)
        .max(now_ms);
    for (id, raw_started) in external {
        let offset_start = now_ms.saturating_add(raw_started.saturating_sub(batch_start));
        let started_at = offset_start.max(next_slot);
        if let Some(slot) = r.reveals.get_mut(&id) {
            *slot = started_at;
        }
        next_slot = started_at.saturating_add(REVEAL_STAGGER_MS);
    }
}

fn smooth_overdue_reveal_burst(r: &mut AgentIndicators, now_ms: u64) {
    let prev_ms = r.last_reveal_snapshot_ms.replace(now_ms);
    if prev_ms.is_some_and(|prev| now_ms <= prev) {
        return;
    }
    let mut ordered: Vec<(String, u64)> = r
        .reveals
        .iter()
        .map(|(id, started)| (id.clone(), *started))
        .collect();
    ordered.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    let newly_due = ordered
        .iter()
        .filter(|(_, started)| reveal_became_due(prev_ms, *started, now_ms))
        .count();
    if newly_due <= REVEAL_MAX_NEW_STARTS_PER_SNAPSHOT {
        return;
    }
    let mut newly_due_seen = 0;
    let mut reschedule_tail = false;
    let mut next_slot = now_ms;
    for (id, original_start) in ordered {
        if reveal_became_due(prev_ms, original_start, now_ms) {
            newly_due_seen += 1;
            if newly_due_seen <= REVEAL_MAX_NEW_STARTS_PER_SNAPSHOT {
                if let Some(started_at) = r.reveals.get_mut(&id) {
                    *started_at = next_slot;
                }
                next_slot = next_slot.saturating_add(REVEAL_BURST_RECOVERY_STAGGER_MS);
                reschedule_tail = true;
                continue;
            } else {
                reschedule_tail = true;
            }
        }
        if !reschedule_tail {
            continue;
        }
        let scheduled_start = original_start.max(next_slot);
        if let Some(started_at) = r.reveals.get_mut(&id) {
            *started_at = scheduled_start;
        }
        next_slot = scheduled_start.saturating_add(REVEAL_BURST_RECOVERY_STAGGER_MS);
    }
}

fn reveal_became_due(prev_ms: Option<u64>, started_at: u64, now_ms: u64) -> bool {
    // A reveal that settled past its animation window is a parked anchor
    // for the cursor / border (run-long retention) — never replay it.
    if now_ms.saturating_sub(started_at) > REVEAL_DURATION_MS {
        return false;
    }
    match prev_ms {
        Some(prev) => started_at > prev && started_at <= now_ms,
        None => started_at <= now_ms,
    }
}

fn has_active_indicators(r: &AgentIndicators) -> bool {
    r.run_active || !r.nodes.is_empty() || !r.frames.is_empty() || !r.reveals.is_empty()
}
