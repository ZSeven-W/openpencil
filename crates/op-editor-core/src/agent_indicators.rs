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
    /// node id → owning agent. Drives the dashed breathing node border.
    pub nodes: HashMap<String, AgentTag>,
    /// root-frame id → owning agent. Drives the glow + name badge.
    pub frames: HashMap<String, AgentTag>,
    /// node ids that are claimed but not yet drawn — drives the preview
    /// pulse fill.
    pub previews: HashSet<String>,
}

static REGISTRY: LazyLock<Mutex<AgentIndicators>> =
    LazyLock::new(|| Mutex::new(AgentIndicators::default()));

/// Tag a single node with its owning agent (dashed breathing border).
pub fn add_node(node_id: &str, color: &str, name: &str) {
    REGISTRY.lock().unwrap().nodes.insert(
        node_id.to_string(),
        AgentTag {
            color: color.to_string(),
            name: name.to_string(),
        },
    );
}

/// Tag a root frame with its owning agent (glow + badge).
pub fn add_frame(frame_id: &str, color: &str, name: &str) {
    REGISTRY.lock().unwrap().frames.insert(
        frame_id.to_string(),
        AgentTag {
            color: color.to_string(),
            name: name.to_string(),
        },
    );
}

/// Mark a node as a not-yet-materialised preview (pulse fill).
pub fn mark_preview(node_id: &str) {
    REGISTRY
        .lock()
        .unwrap()
        .previews
        .insert(node_id.to_string());
}

/// Drop a node from the preview set once it has real geometry.
pub fn clear_preview(node_id: &str) {
    REGISTRY.lock().unwrap().previews.remove(node_id);
}

/// Clear every indicator — called when a generation finishes / is reset.
pub fn clear() {
    let mut r = REGISTRY.lock().unwrap();
    r.nodes.clear();
    r.frames.clear();
    r.previews.clear();
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

    #[test]
    fn node_and_frame_tags_round_trip() {
        clear();
        add_node("n5", "#FF6B6B", "Nova");
        add_frame("n1", "#4ECDC4", "Mochi");
        mark_preview("n5");
        assert!(is_active());
        let snap = snapshot();
        assert_eq!(snap.nodes.get("n5").unwrap().color, "#FF6B6B");
        assert_eq!(snap.frames.get("n1").unwrap().name, "Mochi");
        assert!(snap.previews.contains("n5"));
        clear_preview("n5");
        assert!(!snapshot().previews.contains("n5"));
        clear();
        assert!(!is_active());
        assert!(snapshot().nodes.is_empty());
    }
}
