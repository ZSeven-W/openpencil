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
//! - B2 `$pointer` chooses the smallest currently-Down pointer id as
//!   primary, falling back to the current non-Touch hover sample. With
//!   no pointer it reports `inside=false` and retains the last published
//!   primary scene coordinate (initially 0,0). A synthetic Cancel never
//!   overwrites that retained coordinate with its compatibility 0,0.

use jian_core::gesture::pointer::{PointerKind, PointerPhase};
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct ScenePoint {
    x_bits: u32,
    y_bits: u32,
}

impl ScenePoint {
    fn new((x, y): (f32, f32)) -> Self {
        Self {
            x_bits: x.to_bits(),
            y_bits: y.to_bits(),
        }
    }

    fn get(self) -> (f32, f32) {
        (f32::from_bits(self.x_bits), f32::from_bits(self.y_bits))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PressedPointer {
    node: Option<String>,
    scene: ScenePoint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HoveredPointer {
    id: u32,
    node: Option<String>,
    scene: ScenePoint,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PointerSample {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) present: bool,
}

/// Per-pointer pressed + hover tracking, keyed by node schema id.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct InteractionState {
    /// Pointer id → active Down record. BTree order defines B2 primary.
    pressed: BTreeMap<u32, PressedPointer>,
    /// Current non-Touch hover record when no pointer is Down.
    hovered: Option<HoveredPointer>,
    /// Last primary coordinate, retained after the active set empties.
    last_primary: ScenePoint,
}

impl InteractionState {
    /// The node `pointer_id` is currently pressing, by schema id.
    pub fn pressed_node(&self, pointer_id: u32) -> Option<&str> {
        self.pressed
            .get(&pointer_id)
            .and_then(|pointer| pointer.node.as_deref())
    }

    /// Every currently-pressed node's schema id (deduplicated, stable
    /// ascending order) — what a paint pass needs to derive pressed
    /// widget states.
    pub fn pressed_nodes(&self) -> Vec<&str> {
        let mut nodes: Vec<&str> = self
            .pressed
            .values()
            .filter_map(|pointer| pointer.node.as_deref())
            .collect();
        nodes.sort_unstable();
        nodes.dedup();
        nodes
    }

    /// The hovered node's schema id, when a mouse/pen pointer is over
    /// one without pressing.
    pub fn hovered_node(&self) -> Option<&str> {
        self.hovered
            .as_ref()
            .and_then(|pointer| pointer.node.as_deref())
    }

    /// Track one factual scene-space pointer phase in the existing R4
    /// interaction store. Down records remain active through Move until
    /// Up/Cancel; non-Touch Hover is the fallback when nothing is Down.
    pub(crate) fn track_pointer(
        &mut self,
        pointer_id: u32,
        kind: PointerKind,
        phase: PointerPhase,
        scene: (f32, f32),
        hit_node: Option<&str>,
    ) {
        let scene = ScenePoint::new(scene);
        match phase {
            PointerPhase::Down => {
                if self
                    .hovered
                    .as_ref()
                    .is_some_and(|hovered| hovered.id == pointer_id)
                {
                    self.hovered = None;
                }
                self.pressed.insert(
                    pointer_id,
                    PressedPointer {
                        node: hit_node.map(str::to_owned),
                        scene,
                    },
                );
            }
            PointerPhase::Move => {
                if let Some(pointer) = self.pressed.get_mut(&pointer_id) {
                    pointer.scene = scene;
                }
            }
            PointerPhase::Up => {
                if let Some(pointer) = self.pressed.get_mut(&pointer_id) {
                    pointer.scene = scene;
                    self.publish_primary();
                }
                self.pressed.remove(&pointer_id);
                if !matches!(kind, PointerKind::Touch) {
                    self.hovered = Some(HoveredPointer {
                        id: pointer_id,
                        node: hit_node.map(str::to_owned),
                        scene,
                    });
                }
            }
            PointerPhase::Cancel => {
                self.pressed.remove(&pointer_id);
                if self
                    .hovered
                    .as_ref()
                    .is_some_and(|hovered| hovered.id == pointer_id)
                {
                    self.hovered = None;
                }
            }
            PointerPhase::Hover => {
                if matches!(kind, PointerKind::Touch) {
                    return;
                }
                self.hovered = Some(HoveredPointer {
                    id: pointer_id,
                    node: hit_node.map(str::to_owned),
                    scene,
                });
            }
        }
        self.publish_primary();
    }

    pub(crate) fn pointer_sample(&self) -> PointerSample {
        let current = self.primary_position();
        let (x, y) = current.unwrap_or(self.last_primary).get();
        PointerSample {
            x,
            y,
            present: current.is_some(),
        }
    }

    fn primary_position(&self) -> Option<ScenePoint> {
        self.pressed
            .first_key_value()
            .map(|(_, pointer)| pointer.scene)
            .or_else(|| self.hovered.as_ref().map(|pointer| pointer.scene))
    }

    fn publish_primary(&mut self) {
        if let Some(position) = self.primary_position() {
            self.last_primary = position;
        }
    }

    /// Clear every press — transition, lifecycle exit, or ownership
    /// cancel. Hover is intentionally kept: a lifecycle exit does not
    /// move the (unpressed) mouse.
    pub(crate) fn clear_all_pressed(&mut self) {
        self.pressed.clear();
        self.publish_primary();
    }
}
