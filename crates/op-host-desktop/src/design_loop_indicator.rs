//! Canvas agent indicators for the design-agent tool-loop (Phase 2.3) AND
//! the single-shot design orchestrator (`current_design: DesignSession`).
//!
//! Both drivers share the same `DesignLoopIndicator` epoch/color/name/
//! initial-frame-set shape and the same `register_new_frames` /
//! `collect_top_level_frame_ids` core — only the "is a turn running" signal
//! and the epoch-ownership semantics differ:
//!
//! ## Chat-loop driver — [`pump_indicator`]
//!
//! When `OPENPENCIL_DESIGN_AGENT_LOOP` is active and a design turn is
//! running, this module:
//! 1. Calls `op_editor_core::agent_indicators::begin()` to start an epoch.
//! 2. Assigns the single-agent identity via
//!    `op_orchestrator::agent_identity::assign_agent_identities`.
//! 3. Each pump, registers any *new* top-level Frame nodes (not present when
//!    the turn started) with `agent_indicators::add_frame` so the canvas
//!    painter draws a colour glow + name badge around them.
//! 4. When the chat session ends (turn done), calls
//!    `agent_indicators::finish_if_epoch` — queued reveals drain
//!    gracefully, then the overlay clears itself — and clears
//!    `state.chat.agents_running`. (A user stop elsewhere calls
//!    `end_if_epoch` for an immediate teardown.)
//!
//! Additive only — CRUD chat and orchestrator paths never set
//! `agents_running > 0`, so this driver is a no-op for them.
//!
//! ## Design-session (orchestrator) driver — [`pump_design_session_indicator`]
//!
//! `chat_session_launch.rs::launch_cli_standard_turn` and
//! `op_host_services::design_session::start` (the builtin-provider design
//! path) both already call `agent_indicators::begin()` themselves before
//! constructing the `DesignSession`, and `DesignSession::drop` already
//! retires that epoch (`finish_if_epoch` on natural completion,
//! `end_if_epoch` on abort). So this driver only *reads* the active epoch
//! via `active_epoch()` and registers new top-level frames while
//! `current_design.is_some()` — it never begins or ends an epoch itself,
//! which keeps epoch ownership single-writer even though both drivers can
//! run against the same process (never against the same turn — see the
//! function doc for why).

use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use jian_ops_schema::node::PenNode;
use op_editor_core::agent_indicators;
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::EditorState;
use op_orchestrator::agent_identity::assign_agent_identities_seeded;

/// Per-run randomness for the identity pool — nanos are plenty to meet a
/// different face each run without an RNG dependency.
pub(crate) fn identity_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0)
}

/// Runtime state of one active design-loop indicator epoch.
pub(crate) struct DesignLoopIndicator {
    /// Epoch handle returned by `agent_indicators::begin()`.
    pub epoch: u64,
    /// Hex colour assigned to the single agent, e.g. `"#FF6B6B"`.
    pub color: String,
    /// Display name assigned to the single agent, e.g. `"Kiki"`.
    pub name: String,
    /// Top-level Frame ids that existed BEFORE the turn started.
    /// Frames added during the turn are the ones we tag.
    pub initial_frame_ids: HashSet<String>,
}

/// Collect the ids of every top-level `Frame` node on the active page.
pub(crate) fn collect_top_level_frame_ids(state: &EditorState) -> HashSet<String> {
    state
        .active_children()
        .iter()
        .filter_map(|node| {
            if matches!(node, PenNode::Frame(_)) {
                Some(node.id_str().to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Register any Frame nodes that appeared since the turn started.
/// Called every pump while the chat session is still alive.
pub(crate) fn register_new_frames(indicator: &DesignLoopIndicator, state: &EditorState) {
    for node in state.active_children() {
        if let PenNode::Frame(_) = node {
            let id = node.id_str();
            if !indicator.initial_frame_ids.contains(id) {
                agent_indicators::add_frame(indicator.epoch, id, &indicator.color, &indicator.name);
            }
        }
    }
}

/// Called every frame from `app_handler`'s `RedrawRequested` branch,
/// right after `chat_session::pump`.
///
/// Lifecycle:
/// - When `state.chat.agents_running.0 > 0` and no indicator exists yet
///   → creates one (begins the epoch, snapshots initial frames).
/// - While the indicator exists and `current_chat.is_some()` → registers
///   newly-added frames.
/// - When the indicator exists but `current_chat` has gone (turn done /
///   stopped) → tears down: retires the epoch, clears `agents_running`.
pub(super) fn pump_indicator(
    indicator: &mut Option<DesignLoopIndicator>,
    current_chat: &Option<op_editor_host_core::chat::ChatSession>,
    state: &mut EditorState,
) {
    // Lazy creation when the design loop starts a turn.
    if state.chat.agents_running.0 > 0 && indicator.is_none() {
        let epoch = agent_indicators::active_epoch().unwrap_or_else(agent_indicators::begin);
        let identities = assign_agent_identities_seeded(1, identity_seed());
        let id = identities
            .into_iter()
            .next()
            .expect("assign_agent_identities(1) always yields one");
        let initial = collect_top_level_frame_ids(state);
        // Persona parity with the canvas: the streaming assistant bubble
        // wears the run's agent identity (name + colour), not the backend
        // label — Pencil's transcript reads "• Cosmo", never "Codex CLI".
        if let Some(msg) = state
            .chat
            .messages
            .iter_mut()
            .rev()
            .find(|msg| msg.role == op_editor_core::ChatRole::Assistant && msg.streaming)
        {
            msg.agent_name = Some(id.name.clone());
            msg.agent_color = Some(id.color.clone());
        }
        *indicator = Some(DesignLoopIndicator {
            epoch,
            color: id.color,
            name: id.name,
            initial_frame_ids: initial,
        });
    }

    if let Some(ind) = indicator.as_ref() {
        if current_chat.is_some() {
            // Turn still in flight — tag any frames that appeared.
            register_new_frames(ind, state);
        } else {
            // Turn ended (session dropped) — finish gracefully so the
            // queued reveals play out before the overlay clears itself.
            let epoch = ind.epoch;
            agent_indicators::finish_if_epoch(epoch);
            state.chat.agents_running = (0, 0);
            *indicator = None;
        }
    }
}

/// Called every frame from `app_handler`'s `RedrawRequested` branch, right
/// after `design_session::pump_commands` / `pump_progress` have drained this
/// frame's deltas — mirrors [`pump_indicator`] running after
/// `chat_session::pump` so a same-frame session-finish is observed here
/// instead of lagging a full frame behind.
///
/// Lifecycle (deliberately asymmetric with [`pump_indicator`] — see the
/// module doc):
/// - Lazy creation the first time `current_design.is_some()` with no local
///   indicator yet: reads the epoch the launch site already began via
///   [`op_editor_core::agent_indicators::active_epoch`] (never calls
///   `begin()` — that would either steal a fresh epoch out from under the
///   in-flight `DesignSession`'s `indicator_epoch`, or silently no-op if
///   one is already active, neither of which this driver should decide).
///   `None` means no epoch is active for this turn (e.g. a test harness
///   using `DesignSession::from_channels` without an epoch) — the driver
///   stays dormant rather than fabricating one.
/// - While the indicator exists and `current_design` stays `Some` →
///   registers newly-added top-level frames, same as the chat-loop driver.
/// - When the indicator exists but `current_design` has gone back to `None`
///   (turn finished or aborted) → drops ONLY the local indicator handle.
///   The epoch itself was already retired by `DesignSession::drop` at the
///   moment `current_design` was cleared (`design_session::pump_progress`
///   sets `*current = None` on `poll.finished`, which drops the session in
///   place) — ending it again here would race a fresh epoch a subsequent
///   turn may have already begun.
///
/// The chat-loop and design-session drivers never observe the same turn:
/// `launch_cli_standard_turn` parks both a `ChatSession` and a
/// `DesignSession` up front while an async classifier picks the route, but
/// it never sets `state.chat.agents_running`, so [`pump_indicator`]'s lazy
/// creation gate (`agents_running.0 > 0`) stays closed for that path —
/// only a `OPENPENCIL_DESIGN_AGENT_LOOP` turn sets `agents_running`, and
/// that turn never populates `current_design`.
pub(super) fn pump_design_session_indicator(
    indicator: &mut Option<DesignLoopIndicator>,
    current_design: &Option<crate::design_session::DesignSession>,
    state: &EditorState,
) {
    if current_design.is_some() && indicator.is_none() {
        if let Some(epoch) = agent_indicators::active_epoch() {
            let identities = assign_agent_identities_seeded(1, identity_seed());
            let id = identities
                .into_iter()
                .next()
                .expect("assign_agent_identities(1) always yields one");
            // The transcript is the source of truth for a single-agent
            // design turn. CLI turns are stamped by `ChatState::begin_send`
            // (for example "Claude Code"), while builtin turns use their
            // provider display name. Reusing that label keeps the canvas
            // cursor and chat bubble from presenting two different agents.
            // A persona remains the fallback for harnesses that do not seed
            // a streaming assistant message; its colour also supplies the
            // canvas accent when the transcript has no orchestrated colour.
            let (name, color) = state
                .chat
                .messages
                .iter()
                .rev()
                .find(|message| {
                    message.role == op_editor_core::ChatRole::Assistant && message.streaming
                })
                .map(|message| {
                    (
                        message
                            .agent_name
                            .clone()
                            .unwrap_or_else(|| id.name.clone()),
                        message
                            .agent_color
                            .clone()
                            .unwrap_or_else(|| id.color.clone()),
                    )
                })
                .unwrap_or((id.name, id.color));
            let initial = collect_top_level_frame_ids(state);
            *indicator = Some(DesignLoopIndicator {
                epoch,
                color,
                name,
                initial_frame_ids: initial,
            });
        }
    }

    if let Some(ind) = indicator.as_ref() {
        if current_design.is_some() {
            register_new_frames(ind, state);
        } else {
            // The epoch was already retired by `DesignSession::drop` when
            // `current_design` was cleared — only the local handle is ours
            // to drop.
            *indicator = None;
        }
    }
}

/// True when the active design-loop epoch still has scheduled reveals that
/// should finish before the loop-end structural finalizer rewrites the tree.
pub(crate) fn reveal_drain_pending_for_active_epoch() -> bool {
    let Some(epoch) = agent_indicators::active_epoch() else {
        return false;
    };
    let Some(end) = agent_indicators::latest_reveal_end_ms(epoch) else {
        return false;
    };
    reveal_now_millis() < end
}

fn reveal_now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design_session::DesignSession;
    use jian_ops_schema::node::base::PenNodeBase;
    use jian_ops_schema::node::{ContainerProps, FrameNode};
    use op_editor_core::EditorState;
    use std::sync::mpsc;

    fn make_state() -> EditorState {
        EditorState::new()
    }

    fn frame_node(id: &str) -> PenNode {
        PenNode::Frame(FrameNode {
            base: PenNodeBase {
                id: id.to_string(),
                name: Some("Frame".into()),
                ..Default::default()
            },
            container: ContainerProps::default(),
            children: Some(Vec::new()),
            image_search_query: None,
            reusable: None,
            screen: None,
            slot: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        })
    }

    /// Builds a live `DesignSession` (real mpsc channels, dropped receivers
    /// so nothing blocks) bound to a fresh `agent_indicators` epoch — the
    /// same shape `chat_session_launch.rs::launch_cli_standard_turn` and
    /// `op_host_services::design_session::start` hand to `current_design`.
    fn design_session_with_epoch() -> (DesignSession, u64) {
        let (_delta_tx, delta_rx) = mpsc::channel();
        let (_cmd_tx, cmd_rx) = mpsc::channel();
        let epoch = agent_indicators::begin();
        (
            DesignSession::from_channels_with_epoch(delta_rx, cmd_rx, epoch),
            epoch,
        )
    }

    #[test]
    fn collect_top_level_frame_ids_does_not_panic_on_fresh_doc() {
        let state = make_state();
        // A fresh blank document may or may not have frames — just verify
        // the function runs without panicking.
        let _ids = collect_top_level_frame_ids(&state);
    }

    #[test]
    fn pump_indicator_noop_when_agents_running_zero() {
        let mut state = make_state();
        let mut indicator: Option<DesignLoopIndicator> = None;
        // agents_running = (0,0), no session → stays idle.
        pump_indicator(&mut indicator, &None, &mut state);
        assert!(indicator.is_none());
        assert_eq!(state.chat.agents_running, (0, 0));
    }

    #[test]
    fn pump_indicator_teardown_clears_indicator_and_agents_running() {
        // Touches the process-global `agent_indicators` registry — guard
        // against the new `design_session_indicator_*` tests below racing
        // this one under the default parallel test runner.
        let _guard = lock_agent_indicators();
        let mut state = make_state();
        // Manually plant an indicator as if a turn had been launched.
        let epoch = op_editor_core::agent_indicators::begin();
        let mut indicator: Option<DesignLoopIndicator> = Some(DesignLoopIndicator {
            epoch,
            color: "#FF6B6B".to_string(),
            name: "Kiki".to_string(),
            initial_frame_ids: HashSet::new(),
        });
        state.chat.agents_running = (1, 1);
        // Session gone (None) → teardown path.
        pump_indicator(&mut indicator, &None, &mut state);
        assert!(indicator.is_none(), "teardown must clear the indicator");
        assert_eq!(
            state.chat.agents_running,
            (0, 0),
            "teardown must clear agents_running"
        );
    }

    // ── pump_design_session_indicator: current_design-driven pump ──────────

    /// Guards every test below against the process-global `agent_indicators`
    /// registry racing a concurrently-running test in this same binary
    /// (`main.rs::agent_indicator_test_lock`, the established pattern also
    /// used by `sub_agent_session_tests.rs` / `chat_intent_host_tests.rs`).
    fn lock_agent_indicators() -> std::sync::MutexGuard<'static, ()> {
        crate::agent_indicator_test_lock::LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn design_session_indicator_noop_when_current_design_none() {
        let _guard = lock_agent_indicators();
        agent_indicators::clear();
        let state = make_state();
        let mut indicator: Option<DesignLoopIndicator> = None;
        pump_design_session_indicator(&mut indicator, &None, &state);
        assert!(indicator.is_none());
    }

    #[test]
    fn design_session_indicator_stays_dormant_without_an_active_epoch() {
        // Contract: this driver only READS the epoch the launch site began;
        // it never calls `begin()` itself. With no epoch active, a live
        // `current_design` must not spin one up out of thin air.
        let _guard = lock_agent_indicators();
        agent_indicators::clear();
        let state = make_state();
        let (session, _epoch) = design_session_with_epoch();
        // Retire the epoch immediately so none is active by the time the
        // pump runs, mirroring "epoch already ended elsewhere".
        agent_indicators::clear();
        let current = Some(session);
        let mut indicator: Option<DesignLoopIndicator> = None;
        pump_design_session_indicator(&mut indicator, &current, &state);
        assert!(
            indicator.is_none(),
            "no active epoch → driver must stay dormant, not fabricate one"
        );
    }

    #[test]
    fn design_session_indicator_registers_frames_that_appear_after_the_turn_starts() {
        let _guard = lock_agent_indicators();
        agent_indicators::clear();
        let mut state = make_state();
        let (session, epoch) = design_session_with_epoch();
        let current = Some(session);
        let mut indicator: Option<DesignLoopIndicator> = None;

        // First pump: no frames on the canvas yet — snapshots the (empty)
        // initial set and assigns the agent identity.
        pump_design_session_indicator(&mut indicator, &current, &state);
        assert!(indicator.is_some(), "live current_design must create one");
        assert_eq!(indicator.as_ref().unwrap().epoch, epoch);
        assert!(!agent_indicators::is_frame_generating("frame-1"));

        // The orchestrator inserts a top-level frame mid-turn.
        state.active_children_mut().push(frame_node("frame-1"));
        pump_design_session_indicator(&mut indicator, &current, &state);

        assert!(
            agent_indicators::is_frame_generating("frame-1"),
            "a frame added during the turn must be tagged as generating"
        );
    }

    #[test]
    fn design_session_indicator_reuses_streaming_transcript_identity() {
        let _guard = lock_agent_indicators();
        agent_indicators::clear();
        let mut state = make_state();
        let mut message = op_editor_core::ChatMessage::assistant_streaming();
        message.agent_name = Some("Claude Code".into());
        state.chat.messages.push(message);
        let (session, _epoch) = design_session_with_epoch();
        let current = Some(session);
        let mut indicator: Option<DesignLoopIndicator> = None;

        pump_design_session_indicator(&mut indicator, &current, &state);

        assert_eq!(
            indicator.as_ref().map(|value| value.name.as_str()),
            Some("Claude Code")
        );
    }

    #[test]
    fn design_session_indicator_teardown_clears_local_handle_only() {
        let _guard = lock_agent_indicators();
        agent_indicators::clear();
        let mut state = make_state();
        let (session, _epoch) = design_session_with_epoch();
        let mut current = Some(session);
        let mut indicator: Option<DesignLoopIndicator> = None;

        pump_design_session_indicator(&mut indicator, &current, &state);
        state.active_children_mut().push(frame_node("frame-1"));
        pump_design_session_indicator(&mut indicator, &current, &state);
        assert!(agent_indicators::is_frame_generating("frame-1"));

        // Turn finished: `design_session::pump_progress` already dropped the
        // session (which itself retired the epoch via `Drop`) before this
        // driver observes `current_design == None`.
        current = None;
        pump_design_session_indicator(&mut indicator, &current, &state);

        assert!(indicator.is_none(), "teardown must clear the local handle");
        assert!(
            !agent_indicators::is_frame_generating("frame-1"),
            "DesignSession::drop already retired the epoch — frames stop registering as generating"
        );
        // The epoch is retired, not reused — a fresh turn always gets a new one.
        assert_eq!(agent_indicators::active_epoch(), None);
    }

    #[test]
    fn chat_loop_and_design_session_drivers_do_not_interfere() {
        // `launch_cli_standard_turn` parks a ChatSession AND a DesignSession
        // together while classification resolves, but never sets
        // `agents_running` — so the chat-loop driver's lazy-creation gate
        // must stay closed even though `current_chat` is conceptually "live"
        // (represented here just by the agents_running invariant it reads).
        let _guard = lock_agent_indicators();
        agent_indicators::clear();
        let mut state = make_state();
        let (session, epoch) = design_session_with_epoch();
        let current_design = Some(session);
        let mut chat_indicator: Option<DesignLoopIndicator> = None;
        let mut design_indicator: Option<DesignLoopIndicator> = None;

        // Chat-loop driver: agents_running stays (0, 0) for this route.
        pump_indicator(&mut chat_indicator, &None, &mut state);
        assert!(chat_indicator.is_none());

        // Design-session driver: drives off the same epoch independently.
        pump_design_session_indicator(&mut design_indicator, &current_design, &state);
        assert!(design_indicator.is_some());
        assert_eq!(design_indicator.as_ref().unwrap().epoch, epoch);
    }
}
