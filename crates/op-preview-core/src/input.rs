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
//!
//! ## Track C-3: input gate during a screen transition
//!
//! `dispatch_pointer_phase` / `dispatch_wheel` DISCARD their event
//! outright while `transition_active()` — see that method's doc
//! (`crate::preview::transition`) for why discard, not queue.
//!
//! The explicit-timestamp pointer path synchronizes the session clock
//! BEFORE consulting that gate: `dispatch_pointer_phase_at` advances
//! `last_now_ms` + the jian runtime clock monotonically to the event's
//! host timestamp first, because the web host passes event host time
//! directly and never pushes `set_now_ms` — a stale session clock must
//! not keep a finished transition "active" and discard live input.

use super::{apply_widget_state, PreviewSession};

use jian_core::gesture::pointer::{Modifiers, PointerPhase};
use op_editor_ui::layout_scene::SceneNode;
use op_editor_ui::widgets::canvas_viewport_paint::tabs_active_index;
use op_editor_ui::{Point2D, Rect};

/// The synthetic pointer identity the legacy wrappers
/// ([`Self::dispatch_pointer_phase`] / [`Self::dispatch_pointer_phase_at`])
/// kept dispatching before R4 — desktop mouse panels keep using it.
const LEGACY_POINTER_ID: u32 = 1;

impl PreviewSession {
    /// Route a printable character into the focused widget. Returns
    /// `true` when the runtime consumed it (a focused editable widget
    /// accepted the text). `dispatch_text_input` now returns
    /// `CoreResult<bool>` — `Err(CoreError::Busy)` means the runtime is
    /// mid variant-swap and froze input for IME safety, which reads as
    /// "not consumed" here, same as any other declined dispatch.
    /// R8: store one deferred discrete input, replacing whatever was
    /// there. Never a queue — the user's latest intent is the only one
    /// worth honouring when the screen finally arrives.
    pub(crate) fn defer_discrete(&mut self, input: crate::transition::DeferredDiscreteInput) {
        self.deferred_discrete_input = Some(input);
    }

    /// R8: watch a press that began during a transition. `Down` opens the
    /// tracker, `Up` closes it and defers a Tap when the press stayed
    /// within jian's tap slop and duration; `Cancel` and anything else
    /// drop it. Move/Hover are ignored entirely — drift is judged once,
    /// at Up, against the Down point.
    pub(crate) fn track_transition_press(
        &mut self,
        pointer_id: u32,
        kind: jian_core::gesture::pointer::PointerKind,
        scene_x: f32,
        scene_y: f32,
        phase: PointerPhase,
        t_ms: u64,
    ) {
        match phase {
            PointerPhase::Down => {
                self.transition_tap = Some(crate::transition::TransitionTapTracker {
                    pointer_id,
                    kind,
                    down_x: scene_x,
                    down_y: scene_y,
                    down_ms: t_ms,
                });
            }
            PointerPhase::Up => {
                let completes = self
                    .transition_tap
                    .as_ref()
                    .is_some_and(|t| t.completes_tap(pointer_id, scene_x, scene_y, t_ms));
                if completes {
                    let activation = self.pending_activation;
                    let route_generation = self.route_generation;
                    self.defer_discrete(crate::transition::DeferredDiscreteInput::Tap {
                        scene_x,
                        scene_y,
                        pointer_id,
                        kind,
                        activation,
                        route_generation,
                    });
                }
                self.transition_tap = None;
            }
            PointerPhase::Cancel => {
                self.transition_tap = None;
            }
            _ => {}
        }
    }

    /// R8: replay the deferred input, if one survived, now that the
    /// transition has finished and the arriving screen's layout and hit
    /// mapping have settled.
    ///
    /// A stale input is dropped rather than replayed: a route change
    /// between capture and completion means the screen it was aimed at is
    /// no longer on screen, and its coordinates would land somewhere the
    /// user never pointed at.
    /// Returns whether an input was actually replayed — `false` when the
    /// slot was empty or the stored input had gone stale. Callers use it
    /// to tell "nothing was waiting" apart from "something was dropped".
    pub(crate) fn replay_deferred_input(&mut self) -> bool {
        if self.transition_active() {
            // Replaying into a live transition would route the input
            // straight back into the deferral path and store it again.
            // Replay belongs to the completion edge, and only there.
            return false;
        }
        let Some(input) = self.deferred_discrete_input.take() else {
            return false;
        };
        self.transition_tap = None;
        if input.route_generation() != self.route_generation {
            return false;
        }
        let restore = self.pending_activation;
        self.pending_activation = input.activation();
        match input {
            crate::transition::DeferredDiscreteInput::Tap {
                scene_x,
                scene_y,
                pointer_id,
                kind,
                ..
            } => {
                let now = self.last_now_ms;
                self.dispatch_pointer_for_id_at(
                    pointer_id,
                    kind,
                    scene_x,
                    scene_y,
                    PointerPhase::Down,
                    now,
                );
                self.dispatch_pointer_for_id_at(
                    pointer_id,
                    kind,
                    scene_x,
                    scene_y,
                    PointerPhase::Up,
                    now,
                );
            }
            crate::transition::DeferredDiscreteInput::Submit { key, modifiers, .. } => {
                self.dispatch_key(&key, modifiers);
            }
            crate::transition::DeferredDiscreteInput::Back { .. } => {
                self.dispatch_key("Escape", Modifiers::default());
            }
        }
        self.pending_activation = restore;
        true
    }

    pub fn dispatch_text(&mut self, text: &str) -> bool {
        if self.transition_active() {
            // R8: text and IME are never deferred. A commit replayed onto
            // the arriving screen would land in whatever field happens to
            // hold focus there — a different field, or none.
            return false;
        }
        self.runtime.dispatch_text_input(text).unwrap_or(false)
    }

    /// Route a named key (e.g. `"Backspace"`, `"ArrowLeft"`, `"Enter"`,
    /// `"Tab"`) into the runtime with the given modifier set. Returns
    /// `true` when the dispatch emitted any semantic event.
    pub fn dispatch_key(&mut self, key: &str, modifiers: Modifiers) -> bool {
        if self.transition_active() {
            // R8: Enter and Escape are discrete decisions that survive the
            // wait — everything else (arrows, editing keys) belongs to a
            // text session that will not exist on the arriving screen.
            match key {
                "Enter" => self.defer_discrete(crate::transition::DeferredDiscreteInput::Submit {
                    key: key.to_string(),
                    modifiers,
                    activation: self.pending_activation,
                    route_generation: self.route_generation,
                }),
                "Escape" => self.defer_discrete(crate::transition::DeferredDiscreteInput::Back {
                    activation: self.pending_activation,
                    route_generation: self.route_generation,
                }),
                _ => {}
            }
            return false;
        }
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
    ///
    /// Source-compatible timestamp wrapper: the event is stamped with
    /// the session's `last_now_ms` (the last value the host pushed via
    /// [`Self::set_now_ms`]), so a clock-pushing host gets real
    /// timestamps without changing call sites, and a host that never
    /// pushes a clock gets `0` exactly as before. Prefer
    /// [`Self::dispatch_pointer_phase_at`] when the host already has the
    /// event's own timestamp in hand.
    pub fn dispatch_pointer_phase(
        &mut self,
        scene_x: f32,
        scene_y: f32,
        phase: PointerPhase,
    ) -> bool {
        self.dispatch_pointer_phase_at(scene_x, scene_y, phase, self.last_now_ms)
    }

    /// [`Self::dispatch_pointer_phase`] with an explicit monotonic event
    /// timestamp `t_ms`: the mapping/capture semantics are identical
    /// (anchor on `Down`, reuse on held `Move`, take on `Up`/`Cancel`),
    /// and only the runtime `PointerEvent.t_ms` differs. Gesture
    /// recognizers that measure velocity (Swipe) and timer deadlines
    /// (LongPress) need real per-event timestamps, so every live
    /// preview pointer path in the hosts routes through this method
    /// with the host clock.
    ///
    /// The session/runtime monotonic clock is synchronized from `t_ms`
    /// BEFORE the transition gate and dispatch run, so an event that
    /// carries its own timestamp past a transition's end is never
    /// discarded because the host's last `set_now_ms` push still sits
    /// inside the window (a pointer event between frames, for example).
    /// The sync is monotonic: an out-of-order event (`t_ms` behind the
    /// last pushed clock) leaves the clock where it is, while the event
    /// itself always keeps `t_ms` as the runtime `PointerEvent.t_ms` —
    /// the caller's factual timestamp.
    ///
    /// Pointer identity/kind compatibility note (R4): the synthetic id
    /// (1), `PointerKind::Mouse` and the LEFT-button semantics are kept
    /// for now; full pointer identity/kind mapping remains the R4
    /// pointer-identity work.
    pub fn dispatch_pointer_phase_at(
        &mut self,
        scene_x: f32,
        scene_y: f32,
        phase: PointerPhase,
        t_ms: u64,
    ) -> bool {
        use jian_core::gesture::pointer::PointerKind;
        self.dispatch_pointer_for_id_at(
            LEGACY_POINTER_ID,
            PointerKind::Mouse,
            scene_x,
            scene_y,
            phase,
            t_ms,
        )
    }

    /// [`Self::dispatch_pointer_phase_at`] with an explicit POINTER
    /// IDENTITY (R4 Canonical PreviewInput, multi-pointer step): `id`
    /// keys the per-pointer capture anchor and reaches the Jian runtime
    /// unchanged so two concurrent pointers hold independent streams —
    /// which is what makes Scale/Rotate claims possible through the
    /// product preview path at all. `kind` rides along untouched; hosts
    /// that only speak mouse pass [`PointerKind::Mouse`] with id 1 via
    /// the legacy wrappers.
    pub fn dispatch_pointer_for_id_at(
        &mut self,
        pointer_id: u32,
        kind: jian_core::gesture::pointer::PointerKind,
        scene_x: f32,
        scene_y: f32,
        phase: PointerPhase,
        t_ms: u64,
    ) -> bool {
        use jian_core::gesture::pointer::{MouseButtons, PointerEvent};
        // Sync the session/runtime clock from the event's own timestamp
        // BEFORE `transition_active` consults `last_now_ms` — that gate
        // is only as fresh as the last host clock push, and an explicit
        // `t_ms` past the transition end must dispatch, not be
        // discarded on a stale push. Guarded so an out-of-order event
        // never moves the clock backwards.
        if t_ms > self.last_now_ms {
            self.set_now_ms(t_ms);
        }
        // Track C-3: discard pointer input while a screen-transition
        // animation plays — see `transition_active`'s doc for why discard
        // (not queue) is the right call here. The per-pointer anchors are
        // dropped with the transition's screen rebuild, so nothing stale
        // can resume under it either way.
        if self.transition_active() {
            // R8: raw phases never reach the runtime during a transition.
            // The tracker remembers just enough about the press to decide,
            // at Up, whether it was a Tap worth replaying.
            self.track_transition_press(pointer_id, kind, scene_x, scene_y, phase, t_ms);
            return false;
        }
        let hit = self
            .deepest_mapped_hit(scene_x, scene_y)
            .map(|(_, _, id)| id);
        self.track_interaction(pointer_id, kind, phase, hit.as_deref());
        let (rt_x, rt_y) = self.resolve_runtime_point(scene_x, scene_y, phase, pointer_id);
        use jian_core::geometry::point;
        let mut ev = PointerEvent::simple_at(pointer_id, phase, point(rt_x, rt_y), t_ms);
        ev.kind = kind;
        if matches!(phase, PointerPhase::Hover) {
            // Hover is definitionally unpressed regardless of kind.
            ev.buttons = MouseButtons::empty();
            ev.pressure = 0.0;
        }
        !self.runtime.dispatch_pointer(ev).is_empty()
    }

    /// The R4 Canonical PreviewInput POINTER path: take a FULL host
    /// [`PointerEvent`], transform ONLY its coordinates through the same
    /// per-pointer capture pipeline as the legacy wrappers, and pass
    /// every other fact (id, kind, pressure, buttons, modifiers, tilt,
    /// timestamp) into the runtime unchanged. Returns the semantic
    /// events it produced so [`super::input_event`]'s `dispatch_input`
    /// can report handler keys.
    ///
    /// The transition gate matches the legacy path (discard while a
    /// screen transition plays); R8 replaces that discard with the
    /// one-slot deferred discrete-input policy.
    pub(crate) fn dispatch_pointer_event(
        &mut self,
        mut event: jian_core::gesture::PointerEvent,
    ) -> Vec<jian_core::gesture::SemanticEvent> {
        use jian_core::gesture::pointer::MouseButtons;
        use jian_core::gesture::PointerPhase;
        if event.t_ms > self.last_now_ms {
            self.set_now_ms(event.t_ms);
        }
        if self.transition_active() {
            return Vec::new();
        }
        // Interaction-state tracking reads the SCENE-space hit; compute
        // it before the transform overwrites the position below.
        let hit_node = self
            .deepest_mapped_hit(event.position.x, event.position.y)
            .map(|(_, _, id)| id);
        self.track_interaction(event.id.0, event.kind, event.phase, hit_node.as_deref());
        let (rt_x, rt_y) =
            self.resolve_runtime_point(event.position.x, event.position.y, event.phase, event.id.0);
        event.position = jian_core::geometry::point(rt_x, rt_y);
        if matches!(event.phase, PointerPhase::Hover) {
            // Hover is definitionally unpressed regardless of kind.
            event.buttons = MouseButtons::empty();
            event.pressure = 0.0;
        }
        self.runtime.dispatch_pointer(event)
    }

    /// Update the R4 interaction state from one pointer phase: any
    /// kind's `Down` records the pressed node (Touch Down is the
    /// approved touch fallback's signal; mouse presses feed the same
    /// state), `Up`/`Cancel` clear it, and unpressed Mouse/Pen movement
    /// tracks hover — Touch never hovers, so its movement leaves hover
    /// untouched.
    pub(crate) fn track_interaction(
        &mut self,
        pointer_id: u32,
        kind: jian_core::gesture::pointer::PointerKind,
        phase: PointerPhase,
        hit_node: Option<&str>,
    ) {
        use jian_core::gesture::pointer::PointerKind;
        match phase {
            PointerPhase::Down => {
                if let Some(id) = hit_node {
                    self.interaction.set_pressed(pointer_id, id.to_owned());
                }
            }
            PointerPhase::Up | PointerPhase::Cancel => {
                self.interaction.clear_pressed(pointer_id);
            }
            PointerPhase::Hover => {
                if matches!(kind, PointerKind::Touch) {
                    return;
                }
                match hit_node {
                    Some(id) => self.interaction.set_hovered(id.to_owned()),
                    None => self.interaction.clear_hovered(),
                }
            }
            PointerPhase::Move => {}
        }
    }

    /// Cancel one pointer's live stream by id WITHOUT needing its last
    /// coordinates (teardown / suspend barriers): resolves through the
    /// same per-pointer pipeline as any phase, so that pointer's capture
    /// anchor is released and ITS arena timers (LongPress / touch
    /// ContextMenu) settle — a global single-id cancel would leave other
    /// pointers' deadlines armed. Returns `true` when the runtime emitted
    /// anything for it.
    pub fn cancel_pointer(&mut self, pointer_id: u32, t_ms: u64) -> bool {
        let kind = jian_core::gesture::pointer::PointerKind::Mouse;
        self.dispatch_pointer_for_id_at(pointer_id, kind, 0.0, 0.0, PointerPhase::Cancel, t_ms)
    }

    /// The scene→runtime point for one pointer phase, honoring THAT
    /// pointer's gesture anchor: `Down` resolves fresh and stores the
    /// mapping under its id, pressed `Move` reuses it, `Up` reuses then
    /// clears it, `Hover` (unpressed) always resolves fresh and never
    /// stores. Pointers whose `Down` hit no mapped node resolve fresh
    /// every event until an anchored `Down` replaces that state.
    pub(crate) fn resolve_runtime_point(
        &mut self,
        x: f32,
        y: f32,
        phase: PointerPhase,
        pointer_id: u32,
    ) -> (f32, f32) {
        let anchored = |session: &Self, mapping: Option<(Rect, Rect)>| match mapping {
            Some(m) => session.scene_to_runtime_via(x, y, Some(m)),
            // No anchor (the Down hit no mapped node, or a stray Move
            // without a Down): resolve fresh at the point.
            None => session.scene_to_runtime(x, y),
        };
        match phase {
            PointerPhase::Down => {
                let mapping = self.deepest_mapped_rects(x, y);
                match mapping {
                    Some(m) => {
                        self.gesture_mappings.insert(pointer_id, m);
                    }
                    None => {
                        self.gesture_mappings.remove(&pointer_id);
                    }
                }
                anchored(self, self.gesture_mappings.get(&pointer_id).copied())
            }
            PointerPhase::Move => anchored(self, self.gesture_mappings.get(&pointer_id).copied()),
            PointerPhase::Up | PointerPhase::Cancel => {
                let mapping = self.gesture_mappings.remove(&pointer_id);
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
        if self.transition_active() {
            return false;
        }
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
    pub(crate) fn scene_to_runtime(&self, x: f32, y: f32) -> (f32, f32) {
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
        self.deepest_mapped_hit(x, y)
            .map(|(scene, runtime, _)| (scene, runtime))
    }

    /// [`Self::deepest_mapped_rects`] plus the hit node's SCHEMA id —
    /// what R4 interaction-state tracking needs to record which node a
    /// pointer pressed or hovers.
    pub(crate) fn deepest_mapped_hit(&self, x: f32, y: f32) -> Option<(Rect, Rect, String)> {
        let scene = self.overlay_runtime_state(&self.scene);
        let page = scene.active_page()?;
        for node in page.children.iter().rev() {
            if let Some(hit) = self.deepest_mapped_in(node, x, y) {
                return Some(hit);
            }
        }
        None
    }

    fn deepest_mapped_in(&self, node: &SceneNode, x: f32, y: f32) -> Option<(Rect, Rect, String)> {
        if node.hidden {
            return None;
        }
        let b = node.bounds;
        let (x, y) = inverse_node_transform(node, x, y);
        if x < b.origin.x
            || x > b.origin.x + b.size.x
            || y < b.origin.y
            || y > b.origin.y + b.size.y
        {
            return None;
        }
        for child in self.mapped_children(node).iter().rev() {
            if let Some(hit) = self.deepest_mapped_in(child, x, y) {
                return Some(hit);
            }
        }
        if node.locked {
            return None;
        }
        self.runtime_rect(&node.id).map(|r| (b, r, node.id.clone()))
    }

    /// Match the design/preview painter's tabs rule when choosing a scene
    /// mapping anchor. Runtime state overlays the authored active value first,
    /// so switching tabs cannot leave an invisible panel hittable here.
    fn mapped_children<'a>(&self, node: &'a SceneNode) -> &'a [SceneNode] {
        let Some(authored) = node.widget.as_ref().filter(|widget| widget.kind == "tabs") else {
            return &node.children;
        };
        let mut effective = authored.clone();
        if let Some(state) = self.runtime.widget_states.get(&node.id) {
            apply_widget_state(&mut effective, state);
        }
        node.children
            .get(tabs_active_index(&effective))
            .map(std::slice::from_ref)
            .unwrap_or_default()
    }

    /// The runtime layout rect for the node with schema `id`, in the
    /// runtime's hit-test space, or `None` when the id has no live
    /// runtime node (e.g. a child a promotion dropped from the tree).
    /// `pub(crate)` so `mod.rs`'s test-only `node_rect`
    /// accessor can reach it from the parent module.
    ///
    /// Merge note (responsive-m1a into main): jian-core's `node_rect`
    /// now bakes a non-viewport-normalized root's own authored origin
    /// into every rect under it (see `op-pen-loader`'s `compute_layout`
    /// for the full mechanism) — jian's own convention calls this
    /// "absolute scene coordinates" and its own runtimes hit-test
    /// directly against it. OpenPencil's PreviewSession keeps a SECOND,
    /// separate coordinate frame ("runtime space", root-relative) for
    /// `Runtime::dispatch_pointer` and friends, which `scene_to_runtime`
    /// above maps into via this function. Subtract the root's authored
    /// origin back out so `runtime_rect` keeps returning root-relative
    /// space regardless of jian-core's own internal convention.
    pub(crate) fn runtime_rect(&self, id: &str) -> Option<Rect> {
        let doc = self.runtime.document.as_ref()?;
        let key = doc.tree.by_id.get(id).copied()?;
        let r = self.runtime.layout.node_rect(key)?;
        let (ox, oy) = self.root_authored_origin_of(doc, key);
        Some(Rect {
            origin: Point2D::new(r.origin.x - ox, r.origin.y - oy),
            size: Point2D::new(r.size.width, r.size.height),
        })
    }

    /// Walk `key` up to its tree root and return that root's authored
    /// `(x, y)`, or `(0, 0)` when the root is viewport-normalized (its
    /// origin is already baked out of `node_rect` by jian-core) or has
    /// no authored position.
    fn root_authored_origin_of(
        &self,
        doc: &jian_core::document::RuntimeDocument,
        key: jian_core::document::tree::NodeKey,
    ) -> (f32, f32) {
        let mut cur = key;
        while let Some(parent) = doc.tree.nodes.get(cur).and_then(|n| n.parent) {
            cur = parent;
        }
        if self.runtime.layout.is_origin_normalized(cur) {
            return (0.0, 0.0);
        }
        doc.tree
            .nodes
            .get(cur)
            .map(|root| op_pen_loader::root_authored_origin(&root.schema))
            .unwrap_or((0.0, 0.0))
    }

    /// Advance focus to the next focusable widget (Tab). `focus_next` now
    /// returns `CoreResult<()>` (declines during a variant-swap freeze,
    /// same as `dispatch_text_input`) — fire-and-forget here, same as the
    /// pre-Result behavior: a declined focus move just doesn't move.
    pub fn focus_next(&mut self) {
        let _ = self.runtime.focus_next();
        self.seed_focused_widget_state();
    }

    /// Advance focus to the previous focusable widget (Shift+Tab). See
    /// `focus_next`'s doc for the `CoreResult` note.
    pub fn focus_previous(&mut self) {
        let _ = self.runtime.focus_previous();
        self.seed_focused_widget_state();
    }

    /// Lazily seed the focused widget's runtime state so a freshly
    /// Tab-focused (but not-yet-typed) text input shows its caret right
    /// away — `Runtime::focus_next` only moves the focus pointer; it
    /// does not touch the widget-state store. A no-op for non-widget
    /// (or already-seeded) focus targets.
    pub(crate) fn seed_focused_widget_state(&mut self) {
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

    /// Test-only: the phase-aware point resolution the legacy single-
    /// pointer path uses (pid 1), exercising gesture-anchored capture.
    #[cfg(all(test, not(target_os = "windows")))]
    pub(crate) fn resolve_runtime_point_for_test(
        &mut self,
        x: f32,
        y: f32,
        phase: PointerPhase,
    ) -> (f32, f32) {
        self.resolve_runtime_point(x, y, phase, LEGACY_POINTER_ID)
    }

    /// Test-only: ids that currently hold a capture anchor.
    #[cfg(all(test, not(target_os = "windows")))]
    pub(crate) fn anchored_pointer_ids_for_test(&self) -> Vec<u32> {
        let mut ids: Vec<u32> = self.gesture_mappings.keys().copied().collect();
        ids.sort_unstable();
        ids
    }

    /// Test-only: install a gesture anchor for the LEGACY pointer id,
    /// simulating a Down on a node whose scene and runtime rects diverge
    /// (promotion hug drift, engine drift) without depending on a fixture
    /// that reproduces the divergence organically.
    #[cfg(all(test, not(target_os = "windows")))]
    pub(crate) fn set_gesture_mapping_for_test(&mut self, scene: Rect, runtime: Rect) {
        self.gesture_mappings
            .insert(LEGACY_POINTER_ID, (scene, runtime));
    }

    /// Test-only: ids the scene→runtime mapper can descend into for a
    /// container after applying live widget state.
    #[cfg(all(test, not(target_os = "windows")))]
    pub(crate) fn mapped_child_ids_for_test(&self, id: &str) -> Vec<String> {
        let Some(node) = self.scene.active_page().and_then(|page| page.find(id)) else {
            return Vec::new();
        };
        self.mapped_children(node)
            .iter()
            .map(|child| child.id.clone())
            .collect()
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
        // See `focus_next`'s doc for the `CoreResult` note.
        let _ = self.runtime.focus_request(key);
        self.seed_focused_widget_state();
        self.runtime.focus.current() == Some(key)
    }
}

fn inverse_node_transform(node: &SceneNode, x: f32, y: f32) -> (f32, f32) {
    let pivot_x = node.bounds.origin.x + node.bounds.size.x / 2.0;
    let pivot_y = node.bounds.origin.y + node.bounds.size.y / 2.0;
    let mut local_x = x;
    let mut local_y = y;
    if node.rotation.abs() > f32::EPSILON {
        let dx = local_x - pivot_x;
        let dy = local_y - pivot_y;
        let cosine = (-node.rotation).cos();
        let sine = (-node.rotation).sin();
        local_x = pivot_x + dx * cosine - dy * sine;
        local_y = pivot_y + dx * sine + dy * cosine;
    }
    if node.flip_x {
        local_x = 2.0 * pivot_x - local_x;
    }
    if node.flip_y {
        local_y = 2.0 * pivot_y - local_y;
    }
    (local_x, local_y)
}
