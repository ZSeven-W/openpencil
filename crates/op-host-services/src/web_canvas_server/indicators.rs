//! The `GET /api/mcp/indicators` relay surface on `WebCanvasState` —
//! split out of `web_canvas_server.rs` to keep the spine under the
//! 800-line cap. The write-side semantics (which nodes a registered
//! epoch focuses, who the badge names, when the relay grace retires)
//! live in `mcp_live::client_write`; this is only the endpoint's
//! serialize-and-drain step.

use super::{online_policy, WebCanvasState};

impl WebCanvasState {
    /// The `GET /api/mcp/indicators` body. Retires a write epoch whose
    /// relay grace has elapsed BEFORE serializing — `apply_remote` on the
    /// browser only mirrors active runs, so the poll cadence itself is
    /// what keeps a fresh epoch `run_active` long enough to be seen, then
    /// drains it. See `mcp_live::client_write`.
    ///
    /// The registry is process-global, so a shared deployment relays the
    /// empty projection instead of showing one account the shape of
    /// another account's design run.
    pub(crate) fn indicator_relay_body(&mut self) -> String {
        let now_ms = crate::design_agent_tools::reveal_now_millis();
        self.mcp_write_indicators.finish_expired_relay(now_ms);
        // The same reveal maintenance the paint path runs (`snapshot_at_if_active`
        // / `next_reveal_deadline_ms`) — a headless daemon has no paint pass, so
        // without this a finished run's stragglers serialize stale forever.
        let _ = op_editor_core::agent_indicators::next_reveal_deadline_ms(now_ms);
        if self.mode.allows_agent_indicator_relay() {
            self.mcp_write_indicators.note_relay_served();
            op_editor_core::agent_indicators::relay_json()
        } else {
            online_policy::EMPTY_INDICATOR_RELAY.to_string()
        }
    }
}
