//! Press-handler helpers split out of `press.rs` to honor the
//! 800-line cap — node spawn for the active tool + the
//! agent-settings modal press dispatcher.
//!
//! The agent-settings state walk itself lives in
//! `op_editor_ui::widgets::agent_settings_press_flow` (shared with the
//! web host); what stays here is the desktop platform arms — the
//! clipboard queue, the OS URL launcher, `op-auth-bridge` session
//! revoke, synchronous font enumeration.

use super::WidgetHostNative;
use op_editor_core::agent_settings::ImageGenField;
use op_editor_core::host_settings_commit::SettingsCommitScope;
use op_editor_ui::widgets::agent_settings_fonts::FontsHit;
use op_editor_ui::widgets::agent_settings_panel::AgentSettingsHit;
use op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel;
use op_editor_ui::widgets::agent_settings_press_flow::{
    self as settings_flow, SettingsPress, SettingsPressOutcome,
};
use op_editor_ui::Point2D;

fn create_initial_size_for_tool(tool: op_editor_core::Tool) -> (f64, f64) {
    if matches!(tool, op_editor_core::Tool::Text) {
        (
            f64::from(op_editor_core::DEFAULT_TEXT_NODE_WIDTH),
            f64::from(op_editor_core::DEFAULT_TEXT_NODE_HEIGHT),
        )
    } else {
        (1.0, 1.0)
    }
}

impl WidgetHostNative {
    /// Spawn a fresh node for the active shape / frame / text tool at
    /// `doc_point`. Returns the new node's id when the tool maps to a
    /// creatable kind; `None` for `Select` / `Hand`.
    ///
    /// Delegates to `EditorState::create_node_for_tool` — the host
    /// never builds canonical nodes itself.
    pub(in crate::widget_host) fn create_node_for_active_tool(
        &mut self,
        doc_point: Point2D,
    ) -> Option<op_editor_core::NodeId> {
        let tool = self.editor_state.tool;
        let allowed = if matches!(tool, op_editor_core::Tool::Pen) {
            self.collab_allows_pen_path_mutation()
        } else {
            self.collab_allows_document_mutation(
                op_editor_core::CollabDocumentMutation::BasicNodeInsert,
            )
        };
        if !allowed {
            return None;
        }
        // Click-create default size: Text needs room for its
        // placeholder glyphs; shape tools start 1×1 so a drag
        // immediately sizes the node to the cursor.
        let (init_w, init_h) = create_initial_size_for_tool(tool);
        let result = if let Some(allocator) = self.collab_id_allocator.as_mut() {
            self.editor_state.create_node_for_tool_with_allocator(
                tool,
                allocator,
                doc_point.x as f64,
                doc_point.y as f64,
                init_w,
                init_h,
            )
        } else {
            Ok(self.editor_state.create_node_for_tool(
                tool,
                &mut self.next_node_id,
                doc_point.x as f64,
                doc_point.y as f64,
                init_w,
                init_h,
            ))
        };
        let id = match result {
            Ok(id) => id,
            Err(error) => {
                self.show_collab_id_error(error);
                return None;
            }
        };
        if id.is_some() {
            self.mark_dirty();
        }
        id
    }

    /// Agent-settings modal press dispatcher. Returns true when the
    /// click was consumed by the modal.
    pub(in crate::widget_host) fn dispatch_agent_settings_press(
        &mut self,
        x: f32,
        y: f32,
        vw: f32,
        vh: f32,
    ) -> bool {
        self.refresh_layout_scene();
        let panel = AgentSettingsPanel::for_editor(&self.editor_state);
        let panel_rect = panel.rect(vw, vh);
        let hit = panel.hit_test(panel_rect, Point2D::new(x, y));
        if matches!(hit, AgentSettingsHit::Fonts(FontsHit::SelectFont(_)))
            && !self.collab_allows_document_mutation(
                op_editor_core::CollabDocumentMutation::Unsupported(
                    op_editor_core::CollabUnsupportedFeature::Typography,
                ),
            )
        {
            return true;
        }
        let outcome = settings_flow::apply_agent_settings_hit(
            &mut self.editor_state,
            hit,
            // The desktop operator owns the settings file: editing an
            // entry that arrived as a browser credential snapshot
            // transfers ownership to a local id.
            SettingsCommitScope::Operator,
            self.now_ms,
        );
        self.finish_agent_settings_press(outcome);
        self.mark_dirty();
        true
    }

    /// Desktop platform arms for a settings press.
    fn finish_agent_settings_press(&mut self, outcome: SettingsPressOutcome) {
        match outcome.effect {
            SettingsPress::Handled => {}
            SettingsPress::BlankPress => {
                self.blur_text_inputs_on_blank_press();
            }
            SettingsPress::CopyText(text) => {
                self.editor_state.chat.queue_copy_text(text);
            }
            SettingsPress::OpenUrl(url) => open_external_url(url),
            SettingsPress::SignOut => {
                // Revoke the device session (inert no-op in stub builds).
                op_auth_bridge::sign_out();
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

// Draft seeding for a freshly-focused property row and the panel
// `ColorTarget` -> `ui_draft::ColorTarget` mapping are pure over the
// panel snapshot, so both live in the wasm-clean UI crate next to the
// commit path they must agree with. Re-exported under the historical
// `press_helpers` paths so `press.rs` / `property_popovers.rs` call
// sites stay unchanged.
pub(in crate::widget_host) use op_editor_ui::widgets::property_panel_commit::property_focus_initial;
pub(in crate::widget_host) use op_editor_ui::widgets::property_panel_dispatch_support::color_target;

/// Open `url` in the user's default browser. Spawns the platform's
/// URL launcher detached and ignores any error — opening a help link
/// must never block or panic the editor. Used by the agent-settings
/// "Register at Openverse" link.
fn open_external_url(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut c = std::process::Command::new("cmd");
        c.raw_arg(windows_start_args(url));
        c.creation_flags(CREATE_NO_WINDOW);
        let _ = c.spawn();
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}

/// Args for `cmd /C start "" "<url>"` with the URL double-quoted so
/// cmd doesn't split it at `&` (query-string URLs like the Openverse
/// OAuth registration link). `"` is illegal in URLs — stripped
/// defensively.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn windows_start_args(url: &str) -> String {
    format!("/C start \"\" \"{}\"", url.replace('"', "%22"))
}

#[cfg(test)]
mod tests {
    use super::create_initial_size_for_tool;
    use op_editor_core::Tool;

    #[test]
    fn text_tool_initial_size_uses_compact_text_bounds() {
        let (w, h) = create_initial_size_for_tool(Tool::Text);

        assert_eq!(w, f64::from(op_editor_core::DEFAULT_TEXT_NODE_WIDTH));
        assert_eq!(h, f64::from(op_editor_core::DEFAULT_TEXT_NODE_HEIGHT));
    }
}
