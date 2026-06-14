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

/// Duration of the short generated-node entrance animation.
pub const REVEAL_DURATION_MS: u64 = 1_360;
/// Delay between the first generated nodes in one applied batch.
pub const REVEAL_STAGGER_MS: u64 = 48;
/// Extra delay for nested generated nodes. Keeps child content trailing
/// its parent without making the stream feel sluggish.
pub const REVEAL_DEPTH_STAGGER_MS: u64 = 14;
/// Parent reveals suppress child transforms only during their opening
/// beat. Once the parent has begun settling, delayed children animate
/// independently so streamed content does not pop in abruptly.
pub const REVEAL_CHILD_SUPPRESS_FRACTION: f32 = 0.11;
const REVEAL_FULL_STAGGER_SIBLINGS: u64 = 12;
const REVEAL_MID_STAGGER_SIBLINGS: u64 = 32;
const REVEAL_COMPRESSED_STAGGER_MS: u64 = 22;
const REVEAL_TAIL_STAGGER_MS: u64 = 8;
const CLOCK_REBASE_THRESHOLD_MS: u64 = 60_000;
const REVEAL_FRAME_MS: u64 = 16;

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
    /// node id → reveal start timestamp. Drives the short new-node
    /// entrance animation after AI applies generated nodes.
    pub reveals: HashMap<String, u64>,
}

impl AgentIndicators {
    /// Drop every indicator without touching the epoch.
    fn clear_maps(&mut self) {
        self.nodes.clear();
        self.frames.clear();
        self.previews.clear();
        self.reveals.clear();
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
/// subtree rather than a per-parent sibling index. That keeps nested
/// content from sharing the same start frame across different parents
/// while still compressing long tails enough to stay responsive.
pub fn reveal_offset_ms(depth: u64, stream_index: u64) -> u64 {
    let full = stream_index.min(REVEAL_FULL_STAGGER_SIBLINGS) * REVEAL_STAGGER_MS;
    let mid = stream_index
        .min(REVEAL_MID_STAGGER_SIBLINGS)
        .saturating_sub(REVEAL_FULL_STAGGER_SIBLINGS)
        .saturating_mul(REVEAL_COMPRESSED_STAGGER_MS);
    let tail = stream_index
        .saturating_sub(REVEAL_MID_STAGGER_SIBLINGS)
        .saturating_mul(REVEAL_TAIL_STAGGER_MS);
    depth
        .saturating_mul(REVEAL_DEPTH_STAGGER_MS)
        .saturating_add(full)
        .saturating_add(mid)
        .saturating_add(tail)
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
    !r.nodes.is_empty() || !r.frames.is_empty() || !r.reveals.is_empty()
}

/// A clone of the current indicators for the paint pass to read.
pub fn snapshot() -> AgentIndicators {
    REGISTRY.lock().unwrap().clone()
}

/// A clone of the current indicators with reveal animations expired at
/// `now_ms` pruned. The paint pass calls this every frame so completed
/// reveal animations stop requesting redraws.
pub fn snapshot_at(now_ms: u64) -> AgentIndicators {
    let mut r = REGISTRY.lock().unwrap();
    rebase_external_clock_reveals(&mut r, now_ms);
    r.reveals
        .retain(|_, started| now_ms.saturating_sub(*started) <= REVEAL_DURATION_MS);
    r.clone()
}

/// Next host-clock millisecond needed for generated-node reveal animation.
pub fn next_reveal_deadline_ms(now_ms: u64) -> Option<u64> {
    let mut r = REGISTRY.lock().unwrap();
    rebase_external_clock_reveals(&mut r, now_ms);
    r.reveals
        .retain(|_, started| now_ms.saturating_sub(*started) <= REVEAL_DURATION_MS);
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
        (None, false) => None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{LazyLock, Mutex};

    static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    // One test owns the whole flow so it doesn't race the process-global
    // registry against a sibling test under the default parallel runner.
    #[test]
    fn epoch_scopes_registration_and_teardown() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // Round-trip under a live epoch.
        let e1 = begin();
        add_node(e1, "n5", "#FF6B6B", "Nova");
        add_frame(e1, "n1", "#4ECDC4", "Mochi");
        mark_preview(e1, "n5");
        assert!(is_active());
        let snap = snapshot();
        assert_eq!(snap.nodes.get("n5").unwrap().color, "#FF6B6B");
        assert_eq!(snap.frames.get("n1").unwrap().name, "Mochi");
        assert!(snap.previews.contains("n5"));
        clear_preview(e1, "n5");
        assert!(!snapshot().previews.contains("n5"));

        // A newer run takes over: its begin() clears e1 and claims a fresh
        // epoch.
        let e2 = begin();
        assert!(e2 > e1, "begin bumps the epoch");
        assert!(snapshot().nodes.is_empty(), "begin clears the prior run");

        // The stale run (e1) keeps registering as it tears down — every
        // such call must be dropped, not folded into the live run.
        add_frame(e1, "stale", "#FF6B6B", "Nova");
        add_node(e1, "stale", "#FF6B6B", "Nova");
        mark_preview(e1, "stale");
        let snap = snapshot();
        assert!(snap.frames.is_empty(), "stale frame registration rejected");
        assert!(snap.nodes.is_empty(), "stale node registration rejected");
        assert!(
            snap.previews.is_empty(),
            "stale preview registration rejected"
        );

        // The live run registers fine under its own epoch.
        add_frame(e2, "live", "#4ECDC4", "Mochi");
        assert!(snapshot().frames.contains_key("live"));

        // The stale run's late teardown must not wipe the live run.
        clear_if_epoch(e1);
        assert!(
            snapshot().frames.contains_key("live"),
            "stale teardown is a no-op"
        );

        // The live run's own teardown clears.
        clear_if_epoch(e2);
        assert!(snapshot().frames.is_empty());
        assert!(!is_active());

        // end_if_epoch retires the epoch: after the host ends a run, a
        // worker still mid-registration under it can't re-populate.
        let e3 = begin();
        add_frame(e3, "f3", "#FFD93D", "Pixel");
        end_if_epoch(e3);
        assert!(snapshot().frames.is_empty(), "end_if_epoch clears");
        add_frame(e3, "late", "#FFD93D", "Pixel"); // in-flight registration
        assert!(
            snapshot().frames.is_empty(),
            "registration under a retired epoch no-ops"
        );

        // end_if_epoch on a stale epoch must not touch the live run.
        let e4 = begin();
        add_frame(e4, "f4", "#6C5CE7", "Echo");
        end_if_epoch(e3);
        assert!(
            snapshot().frames.contains_key("f4"),
            "end_if_epoch ignores a stale epoch"
        );
        clear();
    }

    #[test]
    fn reveal_registration_prunes_after_animation_window() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let epoch = begin();
        add_reveal(epoch, "n7", 1_000);
        assert!(is_active(), "reveal should keep the paint loop active");
        assert_eq!(snapshot_at(1_250).reveals.get("n7"), Some(&1_000));
        assert!(
            snapshot_at(2_400).reveals.is_empty(),
            "expired reveal should be pruned"
        );
        assert!(!is_active(), "expired reveal should not keep animating");
        clear();
    }

    #[test]
    fn snapshot_rebases_external_clock_reveals_to_paint_clock() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let epoch = begin();
        add_reveal(epoch, "a", 1_700_000_000_000);
        add_reveal(epoch, "b", 1_700_000_000_080);

        let snap = snapshot_at(1_000);

        assert_eq!(snap.reveals.get("a"), Some(&1_000));
        assert_eq!(snap.reveals.get("b"), Some(&1_080));
        clear();
    }

    #[test]
    fn snapshot_queues_external_reveals_after_active_local_tail() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let epoch = begin();
        add_reveal(epoch, "local-a", 1_000);
        add_reveal(epoch, "local-b", 1_080);
        add_reveal(epoch, "external-a", 1_700_000_000_000);
        add_reveal(epoch, "external-b", 1_700_000_000_005);

        let snap = snapshot_at(1_040);

        assert_eq!(snap.reveals.get("external-a"), Some(&1_128));
        assert_eq!(snap.reveals.get("external-b"), Some(&1_176));
        clear();
    }

    #[test]
    fn active_reveal_deadline_ticks_at_animation_frame_rate() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let epoch = begin();
        add_reveal(epoch, "active", 1_000);

        assert_eq!(next_reveal_deadline_ms(1_100), Some(1_116));
        end_if_epoch(epoch);
    }

    #[test]
    fn reveal_offsets_stream_dense_batches_without_shared_frames() {
        let offsets: Vec<u64> = (0..40).map(|i| reveal_offset_ms(1, i)).collect();

        assert_eq!(offsets[0], REVEAL_DEPTH_STAGGER_MS);
        assert_eq!(offsets[1] - offsets[0], REVEAL_STAGGER_MS);
        assert!(
            offsets.windows(2).all(|pair| pair[1] > pair[0]),
            "stream items should not share an entrance start frame"
        );
        assert!(
            offsets[39] - offsets[0] < 1_200,
            "dense generated batches should stay responsive instead of waiting several seconds"
        );
    }
}
