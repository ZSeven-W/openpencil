//! Canvas agent indicators for the design-agent tool-loop (Phase 2.3).
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
//! `agents_running > 0`, so this module is a no-op for them.

use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use jian_ops_schema::node::PenNode;
use op_editor_core::agent_indicators;
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::EditorState;
use op_orchestrator::agent_identity::assign_agent_identities;

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
        let identities = assign_agent_identities(1);
        let id = identities
            .into_iter()
            .next()
            .expect("assign_agent_identities(1) always yields one");
        let initial = collect_top_level_frame_ids(state);
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
    use op_editor_core::EditorState;

    fn make_state() -> EditorState {
        EditorState::new()
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
}
