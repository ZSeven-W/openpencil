//! Agent settings modal press dispatcher for the web host.
//!
//! The state walk itself lives in
//! `op_editor_ui::widgets::agent_settings_press_flow` (shared with the
//! native host); this file keeps the browser platform arms — clipboard
//! through `host_copy_text`, `window.open` for the help link, the
//! `PendingAuthAction` sign-out queue, the async CanvasKit font refresh —
//! plus the daemon MCP-server start/stop request seam, which is
//! web-only.

use super::agent_settings_mcp_server::{mcp_server_request, request_mcp_server_update};
use super::WidgetHost;
use op_editor_core::agent_settings::ImageGenField;
use op_editor_core::host_settings_commit::SettingsCommitScope;
use op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel;
use op_editor_ui::widgets::agent_settings_press_flow::{
    self as settings_flow, SettingsPress, SettingsPressOutcome,
};
use op_editor_ui::Point2D;

impl WidgetHost {
    /// Returns true once the modal swallowed the press.
    pub(in crate::widget_host) fn dispatch_agent_settings_press(
        &mut self,
        x: f32,
        y: f32,
        vw: f32,
        vh: f32,
    ) -> bool {
        self.refresh_layout_scene();
        let before_mcp = {
            let mcp = self.editor_state.editor_ui.agent_settings.mcp_server;
            (mcp.running, mcp.port)
        };
        let panel = AgentSettingsPanel::for_web_editor(&self.editor_state);
        let panel_rect = panel.rect(vw, vh);
        let point = Point2D::new(x, y);
        let hit = panel.hit_test(panel_rect, point);
        let focus_press = settings_flow::is_settings_focus_press(hit);
        // Preserve the window painted for an already-focused input (model
        // line window / single-line scroll shift); the shared focus
        // transition reseeds the draft and moves its caret to the end.
        let caret_before = if focus_press {
            panel.focused_text_byte_offset_at(panel_rect, point)
        } else {
            None
        };
        drop(panel);
        let outcome = settings_flow::apply_agent_settings_hit(
            &mut self.editor_state,
            hit,
            // The browser owns its own credential snapshot — commits must
            // not re-id `web-credential:` entries or clear the Openverse
            // owner tag (that is the daemon operator's transfer).
            SettingsCommitScope::Browser,
            self.now_ms,
        );
        if focus_press {
            // A fresh focus has no pre-click mapping; resolve against the
            // just-seeded draft so the caret still lands at the tapped glyph.
            let offset = caret_before.or_else(|| {
                let panel = AgentSettingsPanel::for_web_editor(&self.editor_state);
                panel.focused_text_byte_offset_at(panel_rect, point)
            });
            if let Some(offset) = offset {
                self.editor_state
                    .editor_ui
                    .settings_input
                    .set_caret(offset, self.now_ms);
            }
        }
        self.finish_agent_settings_press(outcome);
        let after_mcp = {
            let mcp = self.editor_state.editor_ui.agent_settings.mcp_server;
            (mcp.running, mcp.port)
        };
        if let Some(request) =
            mcp_server_request(before_mcp.0, before_mcp.1, after_mcp.0, after_mcp.1)
        {
            request_mcp_server_update(request);
        }
        self.mark_dirty();
        true
    }

    /// Browser platform arms for a settings press.
    fn finish_agent_settings_press(&mut self, outcome: SettingsPressOutcome) {
        match outcome.effect {
            SettingsPress::Handled => {}
            SettingsPress::BlankPress => {
                self.blur_text_inputs_on_blank_press();
            }
            SettingsPress::CopyText(text) => {
                // Write directly (web has no pending_copy_text drain — that
                // queue is the DESKTOP host's seam); host_copy_text also
                // relays through the extension in the VS Code embed, where
                // the webview permissions chain rejects navigator.clipboard.
                self.host_copy_text(&text);
            }
            SettingsPress::OpenUrl(url) => {
                if let Some(w) = web_sys::window() {
                    let _ = w.open_with_url_and_target(url, "_blank");
                }
            }
            SettingsPress::SignOut => {
                self.pending_auth_actions
                    .push(crate::widget_host::PendingAuthAction::SignOut);
            }
        }
        if outcome.refresh_fonts {
            self.refresh_missing_fonts_for_settings();
        }
    }

    pub(in crate::widget_host) fn focus_image_gen_profile(
        &mut self,
        index: usize,
        field: ImageGenField,
    ) {
        settings_flow::focus_image_gen_profile(&mut self.editor_state, index, field, self.now_ms);
    }
}
