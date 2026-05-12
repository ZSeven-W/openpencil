//! PropertyPanel action + commit dispatch, split out of `input.rs`
//! to stay under the 800-line cap.

use super::helpers::parse_hex_color;
use super::WidgetHostNative;
use openpencil_shell_core::document::PropertyFocus;

impl WidgetHostNative {
    pub(in crate::widget_host) fn apply_property_action(
        &mut self,
        action: openpencil_shell_core::widgets::PropertyPanelAction,
    ) {
        use openpencil_shell_core::widgets::PropertyPanelAction as A;
        match action {
            A::SetFlexLayout(mode) => self.document.ui.flex_layout = mode,
            A::ToggleSizeFillWidth => {
                self.document.ui.size_fill_width = !self.document.ui.size_fill_width;
            }
            A::ToggleSizeFillHeight => {
                self.document.ui.size_fill_height = !self.document.ui.size_fill_height;
            }
            A::ToggleSizeHugWidth => {
                self.document.ui.size_hug_width = !self.document.ui.size_hug_width;
            }
            A::ToggleSizeHugHeight => {
                self.document.ui.size_hug_height = !self.document.ui.size_hug_height;
            }
            A::ToggleSizeClipContent => {
                self.document.ui.size_clip_content = !self.document.ui.size_clip_content;
            }
            A::ToggleFillTypePicker => {
                self.document.ui.fill_type_picker_open = !self.document.ui.fill_type_picker_open;
            }
            A::SetFillType(t) => {
                self.document.set_selected_fill_type(t);
                self.document.ui.fill_type_picker_open = false;
            }
            A::OpenColorPicker(target) => {
                // Fallback anchor when called outside the press
                // path (no click y available); the press handler
                // calls `open_color_picker` directly with the real
                // click y so the picker centers on the swatch.
                let _ = self.document.open_color_picker(target, 0.0);
            }
        }
    }

    /// Commit any focused settings-modal input (currently only the
    /// MCP port). Parses the draft, clamps to a valid port range,
    /// writes it back, and clears focus + draft. No-op when nothing
    /// is focused.
    pub(in crate::widget_host) fn commit_settings_focus_if_any(&mut self) {
        use openpencil_shell_core::document::SettingsFocus;
        let Some(focus) = self.document.ui.agent_settings.focus.take() else {
            return;
        };
        let draft = std::mem::take(&mut self.document.ui.settings_input_draft);
        match focus {
            SettingsFocus::McpPort => {
                if let Ok(port) = draft.trim().parse::<u16>() {
                    // Keep ports above 1024 to avoid root-only ranges;
                    // anything below silently falls back to 1024 so
                    // the user still gets a usable value.
                    self.document.ui.agent_settings.mcp_server.port = port.max(1024);
                }
            }
        }
    }

    pub(in crate::widget_host) fn commit_property_focus_if_any(&mut self) {
        let Some(focus) = self.document.ui.property_focus.take() else {
            return;
        };
        self.document.ui.property_draft_select_all = false;
        let draft = std::mem::take(&mut self.document.ui.property_input_draft);
        match focus {
            PropertyFocus::FillHex => {
                let stripped = draft.trim().trim_start_matches('#');
                if !stripped.is_empty() {
                    if let Some(color) = parse_hex_color(draft.trim()) {
                        let _ = self.document.set_selected_color(true, color);
                    }
                }
            }
            PropertyFocus::StrokeHex => {
                let stripped = draft.trim().trim_start_matches('#');
                if !stripped.is_empty() {
                    if let Some(color) = parse_hex_color(draft.trim()) {
                        let _ = self.document.set_selected_color(false, color);
                    }
                }
            }
            _ => {
                if let Ok(value) = draft.trim().parse::<f32>() {
                    let _ = self.document.commit_property_edit(focus, value);
                }
            }
        }
    }
}
