//! R4 Canonical PreviewInput — the session's unified input entry.
//!
//! Before this module the session exposed only per-domain dispatchers
//! (`dispatch_pointer_phase[_at]`, `dispatch_wheel`, `dispatch_text`,
//! `dispatch_key`), which forced every host to re-implement the same
//! input arbitration (clock sync, transition gating) and gave tests no
//! single seam to drive mixed input through. [`PreviewSession::
//! dispatch_input`] is now the sole public input entry: one enum
//! covering pointers, wheel, keys, text, IME, focus, back, and
//! lifecycle, one [`PreviewDispatchOutcome`] reporting what the input
//! did.
//!
//! The legacy helpers stay as compatibility wrappers (they construct an
//! envelope-less dispatch directly); new hosts migrate onto
//! `dispatch_input` + [`PreviewSession::pump`], whose wake deadline
//! ([`PreviewSession::next_wake_deadline_ms`]) the host must schedule
//! and pump even when no new input arrives.

use jian_core::action::services::Router as _;
use jian_core::gesture::pointer::{Modifiers, WheelEvent};
use jian_core::gesture::{PointerEvent, SemanticEvent};
use op_preview_contracts::UserActivationId;

/// The canonical input envelope: one input plus the user activation the
/// host certified for it. The session stores the activation only for
/// the synchronous ActionList that input spawns (R3 will thread it into
/// the effect source) and expires it before delayed/async work.
#[derive(Debug, Clone)]
pub struct PreviewInputEnvelope {
    pub input: PreviewInput,
    pub activation: Option<UserActivationId>,
}

fn input_host_time(input: &PreviewInput) -> Option<u64> {
    match input {
        PreviewInput::Pointer(event) => Some(event.t_ms),
        PreviewInput::Wheel { event, .. } => Some(event.t_ms),
        _ => None,
    }
}

fn map_input_time(input: &mut PreviewInput, debug: &crate::debug_trace::PreviewDebugState) {
    match input {
        PreviewInput::Pointer(event) => event.t_ms = debug.logical_time(event.t_ms),
        PreviewInput::Wheel { event, .. } => event.t_ms = debug.logical_time(event.t_ms),
        _ => {}
    }
}

fn input_name(input: &PreviewInput) -> &'static str {
    match input {
        PreviewInput::Pointer(_) => "pointer",
        PreviewInput::Wheel { .. } => "wheel",
        PreviewInput::Key { .. } => "key",
        PreviewInput::Text(_) => "text",
        PreviewInput::ImePreedit { .. } => "ime_preedit",
        PreviewInput::ImeCommit { .. } => "ime_commit",
        PreviewInput::ImeCancel => "ime_cancel",
        PreviewInput::FocusNext => "focus_next",
        PreviewInput::FocusPrevious => "focus_previous",
        PreviewInput::Back { .. } => "back",
        PreviewInput::Lifecycle(_) => "lifecycle",
    }
}

fn input_trace_detail(envelope: &PreviewInputEnvelope) -> serde_json::Value {
    let activation = envelope.activation.is_some();
    match &envelope.input {
        PreviewInput::Pointer(event) => serde_json::json!({
            "phase": format!("{:?}", event.phase),
            "pointerId": event.id,
            "activation": activation,
        }),
        PreviewInput::Wheel { phase, .. } => serde_json::json!({
            "phase": format!("{phase:?}"),
            "activation": activation,
        }),
        PreviewInput::Key {
            key, code, repeat, ..
        } => serde_json::json!({
            "key": key,
            "code": code,
            "repeat": repeat,
            "activation": activation,
        }),
        PreviewInput::Text(_)
        | PreviewInput::ImePreedit { .. }
        | PreviewInput::ImeCommit { .. } => {
            serde_json::json!({ "text": "<redacted>", "activation": activation })
        }
        PreviewInput::ImeCancel
        | PreviewInput::FocusNext
        | PreviewInput::FocusPrevious
        | PreviewInput::Back { .. }
        | PreviewInput::Lifecycle(_) => {
            serde_json::json!({ "activation": activation })
        }
    }
}

impl PreviewInputEnvelope {
    /// An envelope without certified activation — most input carries
    /// none (only activation-gated system effects need one).
    pub fn new(input: PreviewInput) -> Self {
        Self {
            input,
            activation: None,
        }
    }
}

/// Every input kind a Preview host can deliver, in one enum (R4 Step 3).
#[derive(Debug, Clone)]
pub enum PreviewInput {
    /// A full pointer event. ONLY the position is transformed
    /// (scene→runtime through that pointer's capture anchor); id, kind,
    /// pressure, buttons, modifiers, tilt, and timestamp reach the jian
    /// runtime unchanged, so two concurrent pointers keep independent
    /// streams (Scale/Rotate co-winning through the product path).
    Pointer(PointerEvent),
    /// Wheel/scroll gesture at a scene point, with the host's scroll
    /// phase. The phase rides into the `onScroll` `$event` payload once
    /// scroll payload expansion lands (the offset/max producers ship
    /// with the `$scroll` namespace in R6; absent facts serialize as
    /// absent, never guessed).
    Wheel {
        event: WheelEvent,
        phase: ScrollPhase,
    },
    /// A named key press. `code`/`repeat` ride along for the R4 key
    /// payload expansion (`$event.code` / `$event.repeat`); the jian
    /// KeyDown semantic currently routes `key` + `modifiers` only.
    Key {
        key: String,
        code: String,
        repeat: bool,
        modifiers: Modifiers,
    },
    /// Printable text from a keypress or paste, routed to the focused
    /// editable widget.
    Text(String),
    /// IME composition update over the focused editable.
    ImePreedit {
        text: String,
        /// Byte-offset selection inside the composed text.
        selection: std::ops::Range<usize>,
    },
    /// IME committed text replacing the composition.
    ImeCommit { text: String },
    /// IME cancelled the active composition.
    ImeCancel,
    /// Move focus forward (Tab).
    FocusNext,
    /// Move focus backward (Shift+Tab).
    FocusPrevious,
    /// Platform back affordance (system back button, Escape key, or a
    /// host-mapped equivalent). Pops the route in APP MODE.
    Back { source: BackSource },
    /// App- or page-level lifecycle phase, dispatched to the authored
    /// lifecycle hooks through the jian task queue. Node mount/unmount
    /// is NOT host input — the route reconciliation drives it.
    Lifecycle(PreviewLifecycle),
}

/// The host-reported phase of a scroll gesture (R4 Step 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollPhase {
    Began,
    Changed,
    Momentum,
    Ended,
    Cancelled,
}

/// What produced a [`PreviewInput::Back`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackSource {
    /// The platform's back affordance (Android back, iOS swipe).
    Platform,
    /// The Escape key.
    Escape,
    /// A host-mapped custom affordance.
    Custom,
}

/// App- or page-level lifecycle phase (R4 lifecycle dispatch).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewLifecycle {
    App(AppLifecyclePhase),
    Page(PageLifecyclePhase),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppLifecyclePhase {
    Launch,
    Resume,
    Background,
    Terminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageLifecyclePhase {
    Enter,
    Leave,
    Foreground,
    Background,
}

impl AppLifecyclePhase {
    fn reason(self) -> &'static str {
        match self {
            AppLifecyclePhase::Launch => "launch",
            AppLifecyclePhase::Resume => "resume",
            AppLifecyclePhase::Background => "background",
            AppLifecyclePhase::Terminate => "terminate",
        }
    }

    fn hook(self) -> &'static str {
        match self {
            AppLifecyclePhase::Launch => "onLaunch",
            AppLifecyclePhase::Resume => "onResume",
            AppLifecyclePhase::Background => "onBackground",
            AppLifecyclePhase::Terminate => "onTerminate",
        }
    }
}

impl PageLifecyclePhase {
    fn reason(self) -> &'static str {
        match self {
            PageLifecyclePhase::Enter => "enter",
            PageLifecyclePhase::Leave => "leave",
            PageLifecyclePhase::Foreground => "foreground",
            PageLifecyclePhase::Background => "background",
        }
    }

    fn hook(self) -> &'static str {
        match self {
            PageLifecyclePhase::Enter => "onEnter",
            PageLifecyclePhase::Leave => "onLeave",
            PageLifecyclePhase::Foreground => "onForeground",
            PageLifecyclePhase::Background => "onBackground",
        }
    }
}

/// What one dispatched input did (R4 Step 3): the semantic handlers that
/// ran (deduplicated, in first-fire order), whether a repaint is needed,
/// and how many effects the input enqueued (0 until the R3 effect queue
/// wires in — an input can only enqueue through a spawned action).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreviewDispatchOutcome {
    pub semantic_handlers: Vec<&'static str>,
    pub needs_redraw: bool,
    pub effects_enqueued: usize,
}

impl PreviewDispatchOutcome {
    /// Collapse the semantic events one input produced into the outcome.
    fn from_events(events: &[SemanticEvent]) -> Self {
        let mut semantic_handlers: Vec<&'static str> = Vec::new();
        for event in events {
            let key = event.handler_key();
            if !semantic_handlers.contains(&key) {
                semantic_handlers.push(key);
            }
        }
        Self {
            needs_redraw: !events.is_empty(),
            semantic_handlers,
            effects_enqueued: 0,
        }
    }

    fn none() -> Self {
        Self::default()
    }
}

impl super::PreviewSession {
    /// The sole public input entry (R4 Step 3). Routes one envelope
    /// through the domain pipeline (clock sync, transition gate,
    /// per-pointer capture) and reports what it did — including how
    /// many effects the input's synchronous action chains enqueued.
    pub fn dispatch_input(&mut self, mut envelope: PreviewInputEnvelope) -> PreviewDispatchOutcome {
        let host_time =
            input_host_time(&envelope.input).unwrap_or_else(|| self.debug.last_host_time());
        self.debug.note_host_time(host_time);
        let logical_time = self.debug.logical_time(host_time);
        self.debug.trace.begin_input(
            input_name(&envelope.input),
            logical_time,
            input_trace_detail(&envelope),
        );
        if self.debug.is_paused() {
            self.debug.trace.record_diagnostic(
                "PausedInput",
                "input ignored while preview is paused",
                None,
                None,
                None,
                logical_time,
            );
            self.debug.trace.finish_input();
            return PreviewDispatchOutcome::none();
        }
        map_input_time(&mut envelope.input, &self.debug);
        let binding_before = self.binding_values();
        let enqueued_before = self.effects.total_enqueued();
        // R8: expose the certified activation for the duration of this
        // dispatch so a transition-deferred input captures it and can
        // replay under the same certification.
        let restore_activation = self.pending_activation;
        self.pending_activation = envelope.activation;
        // The other half of the certification: the engine's action chains
        // read the id through Runtime::take_activation, so the envelope's
        // certification must reach the runtime, not just this session's
        // deferral slot. Cleared after the dispatch — an id the input's
        // chain did not consume must not leak to a later uncertified one.
        self.runtime
            .set_activation(envelope.activation.map(|a| a.raw()));
        let mut outcome = self.dispatch_input_inner(envelope);
        self.runtime.set_activation(None);
        self.pending_activation = restore_activation;
        outcome.effects_enqueued = self.effects.total_enqueued() - enqueued_before;
        let animation_now = self.runtime.now_ms;
        outcome.needs_redraw |=
            self.admit_animation_requests(animation_now) != crate::InvalidationKind::None;
        outcome.needs_redraw |=
            self.finish_binding_update(&binding_before) != crate::InvalidationKind::None;
        self.debug
            .trace
            .record_semantics(&outcome.semantic_handlers, logical_time);
        self.debug.trace.finish_input();
        outcome
    }

    fn dispatch_input_inner(&mut self, envelope: PreviewInputEnvelope) -> PreviewDispatchOutcome {
        match envelope.input {
            PreviewInput::Pointer(event) => {
                PreviewDispatchOutcome::from_events(&self.dispatch_pointer_event(event))
            }
            PreviewInput::Wheel { event, phase } => self.dispatch_input_wheel(event, phase),
            PreviewInput::Key {
                key,
                code,
                repeat,
                modifiers,
            } => {
                // R8: the canonical path shares the legacy path's
                // transition policy — Enter/Escape defer, the rest drop.
                // Routing through `dispatch_key` keeps one implementation
                // of that rule instead of a second copy that can drift.
                if self.transition_active() {
                    self.dispatch_key(&key, modifiers);
                    return PreviewDispatchOutcome::default();
                }
                let events = self.runtime.dispatch_keyboard(key, code, repeat, modifiers);
                PreviewDispatchOutcome::from_events(&events)
            }
            PreviewInput::Text(text) => {
                // R8: text is never deferred, and never replayed.
                if self.transition_active() {
                    return PreviewDispatchOutcome::default();
                }
                let consumed = self.runtime.dispatch_text_input(&text).unwrap_or(false);
                Self::bool_outcome(consumed)
            }
            PreviewInput::ImePreedit { text, selection } => {
                // R8: an in-flight composition cannot survive the screen
                // it was being typed into.
                if self.transition_active() {
                    return PreviewDispatchOutcome::default();
                }
                let consumed =
                    self.runtime
                        .edit_set_composing_text(&text, selection.start, selection.end);
                Self::bool_outcome(consumed)
            }
            PreviewInput::ImeCommit { text } => {
                // Cursor lands after the committed text (the
                // `new_cursor_position` convention is 1-based within the
                // committed text).
                let consumed = self
                    .runtime
                    .edit_commit(&text, jian_core::render::utf16_len(&text) as i32);
                Self::bool_outcome(consumed)
            }
            PreviewInput::ImeCancel => Self::bool_outcome(self.runtime.edit_cancel()),
            PreviewInput::FocusNext => {
                let events = self.runtime.focus_next().unwrap_or_default();
                self.seed_focused_widget_state();
                PreviewDispatchOutcome::from_events(&events)
            }
            PreviewInput::FocusPrevious => {
                let events = self.runtime.focus_previous().unwrap_or_default();
                self.seed_focused_widget_state();
                PreviewDispatchOutcome::from_events(&events)
            }
            PreviewInput::Back { source: _ } => {
                // Platform back maps to the router's pop; workbench-mode
                // sessions have no route stack to pop.
                let popped = self
                    .app
                    .as_ref()
                    .is_some_and(|app| app.router.current().stack.len() > 1);
                if popped {
                    if let Some(app) = &self.app {
                        app.router.pop();
                    }
                }
                Self::bool_outcome(popped)
            }
            PreviewInput::Lifecycle(lifecycle) => self.dispatch_input_lifecycle(lifecycle),
        }
    }

    /// Advance the session by the clock: flush due gesture timers
    /// (deferred Taps, LongPress deadlines), poll action tasks, resolve
    /// image requests, and return what the frame needs. A host schedules
    /// [`Self::next_wake_deadline_ms`] and calls this even with no new
    /// input — a lone Tap's delayed `onTap` fires only through a pump.
    pub fn pump(&mut self, now_ms: u64) -> PreviewDispatchOutcome {
        self.debug.note_host_time(now_ms);
        if self.debug.is_paused() {
            return PreviewDispatchOutcome::none();
        }
        let now_ms = self.debug.logical_time(now_ms);
        let binding_before = self.binding_values();
        let directive = self.runtime.pump(now_ms);
        if now_ms > self.last_now_ms {
            self.last_now_ms = now_ms;
        }
        let animation_invalidation = self.tick_animation(now_ms);
        let mut outcome = PreviewDispatchOutcome {
            semantic_handlers: Vec::new(),
            needs_redraw: directive.needs_paint
                || animation_invalidation != crate::InvalidationKind::None,
            effects_enqueued: 0,
        };
        outcome.needs_redraw |=
            self.finish_binding_update(&binding_before) != crate::InvalidationKind::None;
        outcome
    }

    /// The minimum future deadline the runtime needs a wake for (R4
    /// Step 3): gesture timers (deferred Tap / LongPress), caret blink,
    /// parked IME swaps, scheduled action tasks. R7/R8 fold animation
    /// and transition deadlines into this minimum. `None` = idle.
    pub fn next_wake_deadline_ms(&self) -> Option<u64> {
        if self.debug.is_paused() {
            return None;
        }
        [
            self.runtime.next_wake_ms(),
            self.animation.next_deadline_ms(),
        ]
        .into_iter()
        .flatten()
        .min()
        .map(|deadline| self.debug.host_deadline(deadline))
    }

    /// Clear ALL input ownership ahead of a Background/Terminate barrier
    /// or an equivalent lifecycle exit (R4 Step 4): per-pointer capture
    /// anchors, each cancelled pointer's arena timers, gesture arenas,
    /// Pressed state, focus, and any live IME composition. P6 later
    /// composes this into comprehensive `cancel_all` (tasks / effects /
    /// animations / deferred input).
    pub fn cancel_input_ownership(&mut self, reason: &str) {
        tracing::debug!(reason = %reason, "preview: cancelling input ownership");
        // Cancel each anchored pointer through the same per-pointer
        // pipeline as any phase, so ITS arena timers (LongPress / touch
        // ContextMenu) settle — a global single-id cancel would leave
        // other pointers' deadlines armed.
        let mut ids: Vec<u32> = self.gesture_mappings.keys().copied().collect();
        ids.sort_unstable();
        for pointer_id in ids {
            self.cancel_pointer(pointer_id, self.last_now_ms);
        }
        self.gesture_mappings.clear();
        self.interaction.clear_all_pressed();
        // Drop any live IME composition, then blur focus (emits
        // `onBlur` for the previously-focused node, like any focus move).
        self.runtime.edit_cancel();
        let _ = self.runtime.focus_clear();
    }

    /// Wheel through the canonical path: same transition gate + scene→
    /// runtime transform as the legacy `dispatch_wheel`, returning an
    /// outcome instead of a bool.
    fn dispatch_input_wheel(
        &mut self,
        event: WheelEvent,
        phase: ScrollPhase,
    ) -> PreviewDispatchOutcome {
        use jian_core::gesture::SemanticEvent;
        if self.transition_active() {
            return PreviewDispatchOutcome::none();
        }
        let (rt_x, rt_y) = self.scene_to_runtime(event.position.x, event.position.y);
        let delta_y = event.delta.y;
        let mut ev = event;
        ev.position = jian_core::geometry::point(rt_x, rt_y);
        let events: Vec<SemanticEvent> = self.runtime.dispatch_wheel(ev);
        let scroll_node_id = events.iter().find_map(|semantic| {
            let SemanticEvent::Scroll { node, .. } = semantic else {
                return None;
            };
            self.runtime
                .document
                .as_ref()
                .and_then(|document| document.tree.nodes.get(*node))
                .map(|data| jian_core::document::tree::node_schema_id(&data.schema).to_owned())
        });
        let max_offset = scroll_node_id
            .as_deref()
            .map(|node_id| self.binding_overlay.max_offset(&self.scene, node_id));
        let scroll_changed = self.binding_overlay.update_scroll(
            scroll_node_id.as_deref(),
            delta_y,
            max_offset,
            phase,
        );
        let mut outcome = PreviewDispatchOutcome::from_events(&events);
        outcome.needs_redraw |= scroll_changed;
        outcome
    }

    /// Lifecycle input: resolve the target scope from the session's
    /// current app/page state and spawn the authored hook through jian's
    /// lifecycle dispatch (which honors `disabledEvents`).
    fn dispatch_input_lifecycle(&mut self, lifecycle: PreviewLifecycle) -> PreviewDispatchOutcome {
        match lifecycle {
            PreviewLifecycle::App(phase) => {
                let payload = serde_json::json!({ "reason": phase.reason() });
                let spawned = self.runtime.dispatch_lifecycle(
                    &jian_core::runtime::LifecycleScope::App,
                    phase.hook(),
                    payload,
                );
                Self::bool_outcome(spawned)
            }
            PreviewLifecycle::Page(phase) => {
                let Some(page_id) = self.current_page_schema_id() else {
                    return PreviewDispatchOutcome::none();
                };
                let payload = serde_json::json!({
                    "reason": phase.reason(),
                    "page": page_id,
                });
                let spawned = self.runtime.dispatch_lifecycle(
                    &jian_core::runtime::LifecycleScope::Page {
                        page_id: page_id.clone(),
                    },
                    phase.hook(),
                    payload,
                );
                Self::bool_outcome(spawned)
            }
        }
    }

    /// The schema id of the currently-mounted page, when the document
    /// still carries pages (APP MODE's normalized doc does; the
    /// workbench path projects the active page to the top level and
    /// clears `pages`, so page lifecycle has no scope there).
    pub(crate) fn current_page_schema_id(&self) -> Option<String> {
        let app = self.app.as_ref()?;
        app.promoted_doc
            .pages
            .as_ref()
            .and_then(|pages| pages.get(app.page_idx))
            .map(|page| page.id.clone())
    }

    fn bool_outcome(consumed: bool) -> PreviewDispatchOutcome {
        if consumed {
            PreviewDispatchOutcome {
                semantic_handlers: Vec::new(),
                needs_redraw: true,
                effects_enqueued: 0,
            }
        } else {
            PreviewDispatchOutcome::none()
        }
    }
}
