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
}

impl AgentIndicators {
    /// Drop every indicator without touching the epoch.
    fn clear_maps(&mut self) {
        self.nodes.clear();
        self.frames.clear();
        self.previews.clear();
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
    !r.nodes.is_empty() || !r.frames.is_empty()
}

/// A clone of the current indicators for the paint pass to read.
pub fn snapshot() -> AgentIndicators {
    REGISTRY.lock().unwrap().clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    // One test owns the whole flow so it doesn't race the process-global
    // registry against a sibling test under the default parallel runner.
    #[test]
    fn epoch_scopes_registration_and_teardown() {
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
}
