//! Input dispatch for [`super::PreviewSession`] — keyboard, focus,
//! and the pointer pipeline with its scene→runtime coordinate mapping.
//!
//! Split out of `preview/mod.rs` to honor the repo's 800-line-per-file
//! cap (same pattern as `app_mode.rs` / `scene_helpers.rs`: an inherent
//! `impl` block in a child module reaching the session's plain-private
//! fields, which Rust's default privacy already exposes to descendant
//! modules).
//!
//! ## Scene→runtime mapping + pointer capture
//!
//! The scene paints DESIGN-canvas geometry (authored rects for Figma
//! Preserve imports; the unpromoted flex solve otherwise), while the
//! runtime hit-tests its OWN layout (the promoted tree, always
//! flex-solved). The two can disagree per node, so a tap maps through
//! the pair of rects of the deepest painted node it hit. A pointer
//! gesture anchors that pair at `Down` and reuses it for every held
//! `Move` and the `Up` (pointer capture), so a drag never remaps
//! through a neighbour mid-gesture.

use super::PreviewSession;

use jian_core::gesture::pointer::{Modifiers, PointerPhase};
use op_editor_ui::layout_scene::SceneNode;
use op_editor_ui::{Point2D, Rect};

impl PreviewSession {
    /// Route a printable character into the focused widget. Returns
    /// `true` when the runtime consumed it (a focused editable widget
    /// accepted the text).
    pub fn dispatch_text(&mut self, text: &str) -> bool {
        self.runtime.dispatch_text_input(text)
    }

    /// Route a named key (e.g. `"Backspace"`, `"ArrowLeft"`, `"Enter"`,
    /// `"Tab"`) into the runtime with the given modifier set. Returns
    /// `true` when the dispatch emitted any semantic event.
    pub fn dispatch_key(&mut self, key: &str, modifiers: Modifiers) -> bool {
        !self
            .runtime
            .dispatch_keyboard(key.to_string(), modifiers)
            .is_empty()
    }

    /// Dispatch a tap (Down then Up) at a SCENE-space point into the
    /// runtime so clicks land on switches / buttons / and place caret /
    /// focus in text inputs. The host converts the screen press to scene
    /// (document) space via the editor viewport; here we translate it
    /// into the runtime's root-relative space (subtract the containing
    /// root's authored origin) so the hit-test matches where the widget
    /// paints. Returns `true` when the runtime emitted any semantic
    /// event.
    pub fn dispatch_tap(&mut self, scene_x: f32, scene_y: f32) -> bool {
        let down = self.dispatch_pointer_phase(scene_x, scene_y, PointerPhase::Down);
        let up = self.dispatch_pointer_phase(scene_x, scene_y, PointerPhase::Up);
        down || up
    }

    /// Dispatch one pointer phase at a SCENE-space point. `Down`/`Up`/
    /// `Move` carry mouse-left button semantics (a held drag — slider
    /// knobs); `Hover` is an unpressed move (fires `onHoverEnter` /
    /// `onHoverLeave` actions). Returns `true` when the runtime emitted
    /// any semantic event.
    ///
    /// Pointer-capture semantics: `Down` anchors the scene→runtime
    /// mapping on the node it hit, and the held `Move`s + the `Up`
    /// reuse THAT anchor. Re-resolving per event would remap a drag
    /// through whatever node the pointer crosses (a slider drag past
    /// its own edge would jump into a neighbour's coordinate space and
    /// could activate a widget the pointer isn't visually over).
    pub fn dispatch_pointer_phase(
        &mut self,
        scene_x: f32,
        scene_y: f32,
        phase: PointerPhase,
    ) -> bool {
        use jian_core::geometry::point;
        use jian_core::gesture::pointer::{MouseButtons, PointerEvent, PointerKind};
        let (rt_x, rt_y) = self.resolve_runtime_point(scene_x, scene_y, phase);
        let mut ev = PointerEvent::simple(1, phase, point(rt_x, rt_y));
        ev.kind = PointerKind::Mouse;
        if matches!(phase, PointerPhase::Hover) {
            ev.buttons = MouseButtons::empty();
            ev.pressure = 0.0;
        }
        !self.runtime.dispatch_pointer(ev).is_empty()
    }

    /// The scene→runtime point for one pointer phase, honoring the
    /// gesture anchor: `Down` resolves fresh and stores the mapping,
    /// pressed `Move` reuses it, `Up` reuses then clears it, `Hover`
    /// (unpressed) always resolves fresh and never stores.
    fn resolve_runtime_point(&mut self, x: f32, y: f32, phase: PointerPhase) -> (f32, f32) {
        let anchored = |session: &Self, mapping: Option<(Rect, Rect)>| match mapping {
            Some(m) => session.scene_to_runtime_via(x, y, Some(m)),
            // No anchor (the Down hit no mapped node, or a stray Move
            // without a Down): resolve fresh at the point.
            None => session.scene_to_runtime(x, y),
        };
        match phase {
            PointerPhase::Down => {
                self.gesture_mapping = self.deepest_mapped_rects(x, y);
                anchored(self, self.gesture_mapping)
            }
            PointerPhase::Move => anchored(self, self.gesture_mapping),
            PointerPhase::Up | PointerPhase::Cancel => {
                let mapping = self.gesture_mapping.take();
                anchored(self, mapping)
            }
            PointerPhase::Hover => self.scene_to_runtime(x, y),
        }
    }

    /// Route a wheel at a SCENE-space point into the runtime. Returns
    /// `true` only when a node carrying `events.onScroll` consumed it —
    /// the host falls back to canvas pan/zoom otherwise. `dx`/`dy` are
    /// screen-pixel deltas (same magnitude the design canvas pans by).
    pub fn dispatch_wheel(&mut self, scene_x: f32, scene_y: f32, dx: f32, dy: f32) -> bool {
        use jian_core::geometry::point;
        use jian_core::gesture::pointer::WheelEvent;
        let (rt_x, rt_y) = self.scene_to_runtime(scene_x, scene_y);
        let ev = WheelEvent::simple(point(rt_x, rt_y), point(dx, dy));
        !self.runtime.dispatch_wheel(ev).is_empty()
    }

    /// Translate a scene-space point into the runtime's hit-test space.
    ///
    /// The scene paints DESIGN-canvas geometry (authored rects for
    /// Figma Preserve imports; the unpromoted flex solve otherwise),
    /// while the runtime hit-tests its OWN layout (the promoted tree,
    /// always flex-solved). The two can disagree per node, so a plain
    /// root-origin subtraction would land taps on the wrong element.
    /// Instead: find the deepest painted node containing the point that
    /// also exists in the runtime layout, and map the point through the
    /// pair of rects (offset + proportional scale), so a tap lands at
    /// the same relative spot inside the runtime's copy of the node —
    /// keeping caret placement and slider-knob drags accurate.
    ///
    /// Falls back to the root-origin translation when the point is
    /// outside every mapped node (empty canvas — nothing to hit).
    fn scene_to_runtime(&self, x: f32, y: f32) -> (f32, f32) {
        let mapping = self.deepest_mapped_rects(x, y);
        self.scene_to_runtime_via(x, y, mapping)
    }

    /// Map a scene point through a given (scene rect, runtime rect)
    /// anchor — same relative position inside the runtime rect,
    /// linearly extrapolated when the point is outside the scene rect
    /// (a held drag past the node's edge). `None` resolves fresh at the
    /// point, falling back to the root-origin translation.
    fn scene_to_runtime_via(&self, x: f32, y: f32, mapping: Option<(Rect, Rect)>) -> (f32, f32) {
        if let Some((s, r)) = mapping {
            let fx = if s.size.x > f32::EPSILON {
                (x - s.origin.x) / s.size.x
            } else {
                0.0
            };
            let fy = if s.size.y > f32::EPSILON {
                (y - s.origin.y) / s.size.y
            } else {
                0.0
            };
            return (r.origin.x + fx * r.size.x, r.origin.y + fy * r.size.y);
        }
        for frame in &self.root_frames {
            let rect = frame.scene_rect;
            if x >= rect.origin.x
                && x <= rect.origin.x + rect.size.x
                && y >= rect.origin.y
                && y <= rect.origin.y + rect.size.y
            {
                return (x - frame.offset.0, y - frame.offset.1);
            }
        }
        (x, y)
    }

    /// The (scene rect, runtime rect) pair of the deepest visible scene
    /// node containing the point that also has a runtime layout rect.
    /// Children win over parents; later siblings (painted on top) win
    /// over earlier ones.
    fn deepest_mapped_rects(&self, x: f32, y: f32) -> Option<(Rect, Rect)> {
        let page = self.scene.active_page()?;
        for node in page.children.iter().rev() {
            if let Some(hit) = self.deepest_mapped_in(node, x, y) {
                return Some(hit);
            }
        }
        None
    }

    fn deepest_mapped_in(&self, node: &SceneNode, x: f32, y: f32) -> Option<(Rect, Rect)> {
        if node.hidden {
            return None;
        }
        let b = node.bounds;
        if x < b.origin.x
            || x > b.origin.x + b.size.x
            || y < b.origin.y
            || y > b.origin.y + b.size.y
        {
            return None;
        }
        for child in node.children.iter().rev() {
            if let Some(hit) = self.deepest_mapped_in(child, x, y) {
                return Some(hit);
            }
        }
        self.runtime_rect(&node.id).map(|r| (b, r))
    }

    /// The runtime layout rect for the node with schema `id`, in the
    /// runtime's hit-test space, or `None` when the id has no live
    /// runtime node (e.g. a child a promotion dropped from the tree).
    /// `pub(in crate::preview)` so `mod.rs`'s test-only `node_rect`
    /// accessor can reach it from the parent module.
    pub(in crate::preview) fn runtime_rect(&self, id: &str) -> Option<Rect> {
        let doc = self.runtime.document.as_ref()?;
        let key = doc.tree.by_id.get(id).copied()?;
        let r = self.runtime.layout.node_rect(key)?;
        Some(Rect {
            origin: Point2D::new(r.origin.x, r.origin.y),
            size: Point2D::new(r.size.width, r.size.height),
        })
    }

    /// Advance focus to the next focusable widget (Tab).
    pub fn focus_next(&mut self) {
        self.runtime.focus_next();
        self.seed_focused_widget_state();
    }

    /// Advance focus to the previous focusable widget (Shift+Tab).
    pub fn focus_previous(&mut self) {
        self.runtime.focus_previous();
        self.seed_focused_widget_state();
    }

    /// Lazily seed the focused widget's runtime state so a freshly
    /// Tab-focused (but not-yet-typed) text input shows its caret right
    /// away — `Runtime::focus_next` only moves the focus pointer; it
    /// does not touch the widget-state store. A no-op for non-widget
    /// (or already-seeded) focus targets.
    fn seed_focused_widget_state(&mut self) {
        let Some(key) = self.runtime.focus.current() else {
            return;
        };
        // Clone the focused node's schema so the `&PenNode` borrow of
        // `runtime.document` is released before `get_or_init` takes
        // `runtime.widget_states` mutably (focus changes are rare, so
        // the clone is cheap relative to the interaction it serves).
        let schema = self
            .runtime
            .document
            .as_ref()
            .and_then(|d| d.tree.nodes.get(key))
            .map(|n| n.schema.clone());
        if let Some(schema) = schema {
            self.runtime
                .widget_states
                .get_or_init(&schema, &self.runtime.state);
        }
    }

    /// Test-only: translate a scene-space point into the runtime's
    /// root-relative space (exercises the tap coordinate fix).
    #[cfg(all(test, not(target_os = "windows")))]
    pub(crate) fn scene_to_runtime_for_test(&self, x: f32, y: f32) -> (f32, f32) {
        self.scene_to_runtime(x, y)
    }

    /// Test-only: the phase-aware point resolution `dispatch_pointer_phase`
    /// uses (exercises the gesture-anchored pointer-capture mapping).
    #[cfg(all(test, not(target_os = "windows")))]
    pub(crate) fn resolve_runtime_point_for_test(
        &mut self,
        x: f32,
        y: f32,
        phase: PointerPhase,
    ) -> (f32, f32) {
        self.resolve_runtime_point(x, y, phase)
    }

    /// Test-only: install a gesture anchor directly, simulating a Down
    /// on a node whose scene and runtime rects diverge (promotion hug
    /// drift, engine drift) without depending on a fixture that
    /// reproduces the divergence organically.
    #[cfg(all(test, not(target_os = "windows")))]
    pub(crate) fn set_gesture_mapping_for_test(&mut self, scene: Rect, runtime: Rect) {
        self.gesture_mapping = Some((scene, runtime));
    }

    /// Test-only: focus a node by schema `id` directly (skips the
    /// Tab-ring walk `focus_next`/`focus_previous` use), then seed its
    /// widget runtime state the same way those two do. Returns `true`
    /// when the id resolved to a live node AND that node is in the
    /// focus chain (`FocusManager::request` rejects ids outside it).
    #[cfg(all(test, not(target_os = "windows")))]
    pub(crate) fn focus_node_for_test(&mut self, id: &str) -> bool {
        let Some(key) = self
            .runtime
            .document
            .as_ref()
            .and_then(|d| d.tree.by_id.get(id).copied())
        else {
            return false;
        };
        self.runtime.focus_request(key);
        self.seed_focused_widget_state();
        self.runtime.focus.current() == Some(key)
    }
}
