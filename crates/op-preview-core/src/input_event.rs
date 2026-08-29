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
    /// per-pointer capture) and reports what it did.
    pub fn dispatch_input(&mut self, envelope: PreviewInputEnvelope) -> PreviewDispatchOutcome {
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
                let _ = (code, repeat);
                let events = self.runtime.dispatch_keyboard(key, modifiers);
                PreviewDispatchOutcome::from_events(&events)
            }
            PreviewInput::Text(text) => {
                let consumed = self.runtime.dispatch_text_input(&text).unwrap_or(false);
                Self::bool_outcome(consumed)
            }
            PreviewInput::ImePreedit { text, selection } => {
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
        let directive = self.runtime.pump(now_ms);
        if now_ms > self.last_now_ms {
            self.last_now_ms = now_ms;
        }
        PreviewDispatchOutcome {
            semantic_handlers: Vec::new(),
            needs_redraw: directive.needs_paint,
            effects_enqueued: 0,
        }
    }

    /// The minimum future deadline the runtime needs a wake for (R4
    /// Step 3): gesture timers (deferred Tap / LongPress), caret blink,
    /// parked IME swaps, scheduled action tasks. R7/R8 fold animation
    /// and transition deadlines into this minimum. `None` = idle.
    pub fn next_wake_deadline_ms(&self) -> Option<u64> {
        self.runtime.next_wake_ms()
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
        let ev = WheelEvent::simple(jian_core::geometry::point(rt_x, rt_y), event.delta);
        let events: Vec<SemanticEvent> = self.runtime.dispatch_wheel(ev);
        let outcome = PreviewDispatchOutcome::from_events(&events);
        // The phase is host-reported fact for the (upcoming) scroll
        // payload expansion; consuming it here keeps the envelope's
        // contract honest until then.
        let _ = phase;
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
