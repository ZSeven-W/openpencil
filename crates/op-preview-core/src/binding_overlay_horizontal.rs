//! Axis-aware implicit carousels and pointer ownership, without document edits.

use super::*;
use jian_core::gesture::pointer::{PointerEvent, PointerPhase};

pub(crate) fn is_horizontal_viewport(node: &PenNode) -> bool {
    serde_json::to_value(node)
        .ok()
        .is_some_and(|value| op_editor_core::scroll_viewport::is_horizontal_scroll_viewport(&value))
}

fn right(node: &SceneNode) -> f32 {
    node.children
        .iter()
        .map(right)
        .fold(node.bounds.origin.x + node.bounds.size.x, f32::max)
}

pub(super) fn max_offset(node: &SceneNode) -> f32 {
    let end = node
        .children
        .iter()
        .map(right)
        .fold(node.bounds.origin.x, f32::max);
    (end - node.bounds.origin.x - node.bounds.size.x).max(0.0)
}

#[derive(Clone)]
pub(super) struct Drag {
    node: String,
    start: (f32, f32),
    last_x: f32,
    claimed: bool,
}

impl BindingOverlay {
    pub(crate) fn is_horizontal(&self, node_id: &str) -> bool {
        self.inner.borrow().horizontal_scrollers.contains(node_id)
    }

    pub(crate) fn scroll_revision(&self) -> u64 {
        self.inner.borrow().scroll_revision
    }

    pub(crate) fn clear_horizontal_drags(&self) {
        let drags = std::mem::take(&mut self.inner.borrow_mut().horizontal_drags);
        for drag in drags.values().filter(|drag| drag.claimed) {
            self.update_scroll(Some(&drag.node), 0.0, None, ScrollPhase::Cancelled);
        }
    }

    pub(crate) fn horizontal_target(&self, scene: &LayoutScene, x: f32, y: f32) -> Option<String> {
        fn hit(nodes: &[SceneNode], ids: &BTreeSet<String>, x: f32, y: f32) -> Option<String> {
            for node in nodes.iter().rev() {
                let b = node.bounds;
                let inside = x >= b.origin.x
                    && x <= b.origin.x + b.size.x
                    && y >= b.origin.y
                    && y <= b.origin.y + b.size.y;
                if node.clip_content && !inside {
                    continue;
                }
                if let Some(id) = hit(&node.children, ids, x, y) {
                    return Some(id);
                }
                if inside && ids.contains(&node.id) && max_offset(node) > 1.0 {
                    return Some(node.id.clone());
                }
            }
            None
        }
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        hit(
            &scene.active_page()?.children,
            &self.inner.borrow().horizontal_scrollers,
            x,
            y,
        )
    }

    /// First horizontal claim cancels the runtime press; subsequent phases
    /// stay with that pointer until Up/Cancel. Vertical intent releases it.
    pub(crate) fn horizontal_pointer(
        &self,
        scene: &LayoutScene,
        base: &LayoutScene,
        event: &PointerEvent,
    ) -> Option<bool> {
        let id = event.id.0;
        let (x, y) = (event.position.x, event.position.y);
        if !x.is_finite() || !y.is_finite() {
            self.inner.borrow_mut().horizontal_drags.remove(&id);
            return None;
        }
        if event.phase == PointerPhase::Down {
            if let Some(node) = self.horizontal_target(scene, x, y) {
                self.inner.borrow_mut().horizontal_drags.insert(
                    id,
                    Drag {
                        node,
                        start: (x, y),
                        last_x: x,
                        claimed: false,
                    },
                );
            }
            return None;
        }
        let mut drag = self.inner.borrow_mut().horizontal_drags.remove(&id)?;
        if matches!(event.phase, PointerPhase::Cancel | PointerPhase::Up) {
            if drag.claimed {
                self.update_scroll(Some(&drag.node), 0.0, None, ScrollPhase::Ended);
                return Some(false);
            }
            return None;
        }
        if event.phase != PointerPhase::Move {
            self.inner.borrow_mut().horizontal_drags.insert(id, drag);
            return None;
        }
        let (dx, dy) = (x - drag.start.0, y - drag.start.1);
        if !drag.claimed && dy.abs() > 8.0 && dy.abs() >= dx.abs() {
            return None;
        }
        if !drag.claimed && (dx.abs() <= 8.0 || dx.abs() <= dy.abs()) {
            self.inner.borrow_mut().horizontal_drags.insert(id, drag);
            return None;
        }
        let first_claim = !drag.claimed;
        drag.claimed = true;
        self.update_scroll(
            Some(&drag.node),
            x - drag.last_x,
            Some(self.max_offset(base, &drag.node)),
            ScrollPhase::Changed,
        );
        drag.last_x = x;
        self.inner.borrow_mut().horizontal_drags.insert(id, drag);
        Some(first_claim)
    }
}

impl crate::session::PreviewSession {
    pub(crate) fn horizontal_scroll_target(&self, x: f32, y: f32) -> Option<String> {
        if self
            .binding_overlay
            .inner
            .borrow()
            .horizontal_scrollers
            .is_empty()
        {
            return None;
        }
        self.binding_overlay
            .horizontal_target(&self.overlay_runtime_state(&self.scene), x, y)
    }
    pub(crate) fn handle_horizontal_drag(&mut self, event: &PointerEvent) -> bool {
        if self
            .binding_overlay
            .inner
            .borrow()
            .horizontal_scrollers
            .is_empty()
        {
            return false;
        }
        let scene = self.overlay_runtime_state(&self.scene);
        let Some(first_claim) = self
            .binding_overlay
            .horizontal_pointer(&scene, &self.scene, event)
        else {
            return false;
        };
        if first_claim {
            let mut cancel = event.clone();
            cancel.phase = PointerPhase::Cancel;
            let (x, y) = self.resolve_runtime_point(
                event.position.x,
                event.position.y,
                PointerPhase::Cancel,
                event.id.0,
            );
            cancel.position = jian_core::geometry::point(x, y);
            self.runtime.dispatch_pointer(cancel);
            self.interaction.track_pointer(
                event.id.0,
                event.kind,
                PointerPhase::Cancel,
                (event.position.x, event.position.y),
                None,
            );
        }
        self.gesture_mappings.remove(&event.id.0);
        true
    }
}
