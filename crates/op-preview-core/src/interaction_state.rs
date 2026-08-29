//! Runtime interaction state (R4): which nodes each pointer is pressing
//! and which node is hovered, tracked by the session's own dispatch
//! paths so Preview paint can produce the approved touch fallback from
//! authored/derived widget states.
//!
//! Rules (from the R4 step):
//! - Touch `Down` sets Pressed on the hit node and NEVER dispatches or
//!   sets Hover; the press clears back to Focused/Idle on `Up`.
//! - Mouse/Pen unpressed movement (`Hover`) may set hover.
//! - Arena loss, `Cancel`, a screen transition, or a lifecycle exit
//!   clears Pressed — `cancel_input_ownership` / the transition gate
//!   call [`InteractionState::clear_all_pressed`].

use std::collections::HashMap;

/// Per-pointer pressed + hover tracking, keyed by node schema id.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct InteractionState {
    /// Pointer id → schema id of the node that pointer is pressing.
    pressed: HashMap<u32, String>,
    /// Schema id of the hovered node (mouse/pen hover only).
    hovered: Option<String>,
}

impl InteractionState {
    /// The node `pointer_id` is currently pressing, by schema id.
    pub fn pressed_node(&self, pointer_id: u32) -> Option<&str> {
        self.pressed.get(&pointer_id).map(String::as_str)
    }

    /// Every currently-pressed node's schema id (deduplicated, stable
    /// ascending order) — what a paint pass needs to derive pressed
    /// widget states.
    pub fn pressed_nodes(&self) -> Vec<&str> {
        let mut nodes: Vec<&str> = self.pressed.values().map(String::as_str).collect();
        nodes.sort_unstable();
        nodes.dedup();
        nodes
    }

    /// The hovered node's schema id, when a mouse/pen pointer is over
    /// one without pressing.
    pub fn hovered_node(&self) -> Option<&str> {
        self.hovered.as_deref()
    }

    /// Record that `pointer_id` pressed `node_id` (the hit node at
    /// `Down`).
    pub(crate) fn set_pressed(&mut self, pointer_id: u32, node_id: String) {
        self.pressed.insert(pointer_id, node_id);
    }

    /// End `pointer_id`'s press (`Up`/`Cancel`).
    pub(crate) fn clear_pressed(&mut self, pointer_id: u32) {
        self.pressed.remove(&pointer_id);
    }

    /// Record the hovered node for an unpressed mouse/pen pointer.
    pub(crate) fn set_hovered(&mut self, node_id: String) {
        self.hovered = Some(node_id);
    }

    /// Clear hover (pressed movement never hovers; leaving clears).
    pub(crate) fn clear_hovered(&mut self) {
        self.hovered = None;
    }

    /// Clear every press — transition, lifecycle exit, or ownership
    /// cancel. Hover is intentionally kept: a lifecycle exit does not
    /// move the (unpressed) mouse.
    pub(crate) fn clear_all_pressed(&mut self) {
        self.pressed.clear();
    }
}
