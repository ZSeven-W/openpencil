//! Chrome-state transitions shared verbatim by the native and web
//! widget hosts (their `widget_host/` twins used to carry these as
//! copy-pasted impl blocks). Hosts stay thin: they map widget-facade
//! types to core types, call these, and `mark_dirty()` when a
//! transition reports a change.

use jian_ops_schema::node::PenNode;

use crate::agent_settings::SettingsFocus;
use crate::editor_ui_state::{CompositingPickerTarget, EditorUiState, ExportFormat};
use crate::scene_template_catalog::TemplateScene;
use crate::state::EditorState;
use crate::{EditorCommand, NodeId};

impl EditorUiState {
    /// Close the PropertyPanel compositing picker and clear its
    /// transient hover/press state.
    pub fn close_compositing_picker(&mut self) {
        self.compositing_picker.open = false;
        self.compositing_picker.hover = None;
        self.compositing_picker.pressed = None;
        self.compositing_picker_target = None;
    }

    /// Toggle the compositing picker for `target`, closing every other
    /// property-panel popover when it opens.
    pub fn toggle_compositing_picker(&mut self, target: CompositingPickerTarget) {
        let opening = !self.compositing_picker.open
            || self.compositing_picker_target.as_ref() != Some(&target);
        self.compositing_picker.open = opening;
        self.compositing_picker.hover = None;
        self.compositing_picker.pressed = None;
        self.compositing_picker.scroll.offset = 0.0;
        self.compositing_picker_target = opening.then_some(target);
        if opening {
            self.close_fill_type_picker();
            self.image_fill_popover_open = false;
            self.close_font_picker();
            self.font_weight_picker_open = false;
            self.padding_mode_popover_open = false;
            self.stroke_mode_popover_open = false;
            self.export_scale_picker_open = false;
            self.export_format_picker_open = false;
            self.close_color_variable_picker();
        }
    }

    /// Toggle the variables panel (closes the Design.md panel — the two
    /// share the left rail slot).
    pub fn toggle_variables_panel(&mut self) {
        self.variables_panel_open = !self.variables_panel_open;
        if !self.variables_panel_open {
            self.variables_panel_hover = None;
            self.axis_dropdown_open = None;
        }
        self.design_md_panel.open = false;
        self.design_md_panel.hover = None;
    }

    /// Toggle the Design.md panel.
    pub fn toggle_design_md_panel(&mut self) {
        self.design_md_panel.open = !self.design_md_panel.open;
        if !self.design_md_panel.open {
            self.design_md_panel.hover = None;
        }
    }

    /// Open the File ▸ Export dialog, preset to the format this
    /// document's scenario is delivered in.
    ///
    /// Every host routes its Export entry point through here so the
    /// preset cannot exist on one platform and not the other. The
    /// scenario only picks the starting format — the picker still offers
    /// all of them — but a deck whose deliverable is a PDF should not
    /// make the presenter re-pick PDF on every export.
    pub fn open_export_dialog(&mut self) {
        self.image_panel.close_popovers();
        if let Some(format) = scenario_default_export_format(self.scenario) {
            self.export_format = format;
        }
        self.export_dialog_open = true;
    }
}

/// The format a scenario's documents are delivered in, or `None` when the
/// scenario has no opinion and the user's own last choice stands.
pub fn scenario_default_export_format(scenario: Option<TemplateScene>) -> Option<ExportFormat> {
    match scenario {
        // A deck is delivered as a slide-per-page PDF — see
        // `op_host_services::export_pdf::export_deck_pdf`.
        Some(TemplateScene::Slides) => Some(ExportFormat::Pdf),
        // A card is delivered as a PNG, but so is a tutorial and a carousel,
        // a web page is one scrolling surface with no single deliverable at
        // all, and the user's own last choice (WebP for a lighter upload,
        // say) is as valid. Presetting here would silently overwrite that on
        // every export; the deck is the only scenario with a format its
        // delivery actually depends on.
        Some(TemplateScene::Tutorial)
        | Some(TemplateScene::Comparison)
        | Some(TemplateScene::Carousel)
        | Some(TemplateScene::Card)
        | Some(TemplateScene::Web)
        | Some(TemplateScene::Infographic)
        | None => None,
    }
}

// ─── Chat model-picker filter input ────────────────────────────────────

/// Insert `c` into the chat model-picker filter. Returns `true` when the
/// keystroke was consumed (picker open, printable char).
pub fn chat_model_picker_text(ui: &mut EditorUiState, c: char, now_ms: u64) -> bool {
    if !ui.chat_model_picker.open || c.is_control() {
        return false;
    }
    let mut s = [0u8; 4];
    ui.chat_model_picker_input
        .insert_str(c.encode_utf8(&mut s), now_ms);
    ui.chat_model_picker.scroll.offset = 0.0;
    ui.chat_model_picker.hover = None;
    ui.chat_model_picker.pressed = None;
    true
}

/// Backspace in the chat model-picker filter.
pub fn chat_model_picker_backspace(ui: &mut EditorUiState, now_ms: u64) -> bool {
    if !ui.chat_model_picker.open {
        return false;
    }
    ui.chat_model_picker_input.backspace(now_ms);
    ui.chat_model_picker_input.touch(now_ms);
    ui.chat_model_picker.scroll.offset = 0.0;
    ui.chat_model_picker.hover = None;
    ui.chat_model_picker.pressed = None;
    true
}

/// Move the chat model-picker caret left/right.
pub fn chat_model_picker_caret(ui: &mut EditorUiState, forward: bool, now_ms: u64) -> bool {
    if !ui.chat_model_picker.open {
        return false;
    }
    if forward {
        ui.chat_model_picker_input.move_right(false, now_ms);
    } else {
        ui.chat_model_picker_input.move_left(false, now_ms);
    }
    true
}

// ─── Settings-modal focused input ──────────────────────────────────────

/// Seed the settings input with `text` (focus/edit handoff).
pub fn set_settings_input_text(ui: &mut EditorUiState, text: String, now_ms: u64) {
    ui.settings_input.set_text(text);
    ui.settings_input.touch(now_ms);
}

/// Insert `c` into the focused settings input, honoring the per-focus
/// accept rules (digits-only + length cap for the port field, etc.).
pub fn settings_text(ui: &mut EditorUiState, c: char, now_ms: u64) -> bool {
    let Some(focus) = ui.agent_settings.focus else {
        return false;
    };
    if !settings_accepts(focus, &ui.settings_input, c) {
        return false;
    }
    let mut buf = [0; 4];
    ui.settings_input
        .insert_str(c.encode_utf8(&mut buf), now_ms);
    true
}

/// Insert a multi-character payload into the focused settings input.
///
/// Built-in provider Model fields are the sole multiline settings surface:
/// preserve their line structure while normalizing every platform newline
/// spelling (`CRLF` / bare `CR`) to `LF`. All other settings fields retain
/// their single-line contract and discard control characters before applying
/// their usual per-focus validation.
pub fn settings_text_payload(ui: &mut EditorUiState, text: &str, now_ms: u64) -> bool {
    let Some(focus) = ui.agent_settings.focus else {
        return false;
    };
    let multiline_model = matches!(
        focus,
        SettingsFocus::BuiltinAgent {
            field: crate::agent_settings::BuiltinAgentField::Model,
            ..
        } | SettingsFocus::BuiltinAgentDraft(crate::agent_settings::BuiltinAgentField::Model)
    );
    let mut inserted = false;
    let mut previous_was_cr = false;
    for c in text.chars() {
        if multiline_model {
            if c == '\n' && previous_was_cr {
                previous_was_cr = false;
                continue;
            }
            previous_was_cr = c == '\r';
            let normalized = if c == '\r' { '\n' } else { c };
            if settings_text(ui, normalized, now_ms) {
                inserted = true;
            }
        } else {
            previous_was_cr = false;
            if !c.is_control() && settings_text(ui, c, now_ms) {
                inserted = true;
            }
        }
    }
    inserted
}

/// Insert a newline only when the built-in provider Model list owns focus.
/// Hosts route Enter here before their generic "commit settings" branch.
pub fn settings_model_newline(ui: &mut EditorUiState, now_ms: u64) -> bool {
    if !matches!(
        ui.agent_settings.focus,
        Some(SettingsFocus::BuiltinAgent {
            field: crate::agent_settings::BuiltinAgentField::Model,
            ..
        }) | Some(SettingsFocus::BuiltinAgentDraft(
            crate::agent_settings::BuiltinAgentField::Model,
        ))
    ) {
        return false;
    }
    settings_text(ui, '\n', now_ms)
}

/// Backspace in the focused settings input.
pub fn settings_backspace(ui: &mut EditorUiState, now_ms: u64) -> bool {
    if ui.agent_settings.focus.is_none() {
        return false;
    }
    ui.settings_input.backspace(now_ms);
    true
}

/// Forward Delete in the focused settings input — removes the pending
/// selection or the character after the caret. Consumes the key whenever
/// a settings field owns the keyboard so it can never fall through to
/// canvas node deletion behind the modal.
pub fn settings_delete_forward(ui: &mut EditorUiState, now_ms: u64) -> bool {
    if ui.agent_settings.focus.is_none() {
        return false;
    }
    ui.settings_input.delete_forward(now_ms);
    true
}

/// Move the focused settings-input caret left/right.
pub fn settings_caret(ui: &mut EditorUiState, forward: bool, now_ms: u64) -> bool {
    if ui.agent_settings.focus.is_none() {
        return false;
    }
    if forward {
        ui.settings_input.move_right(false, now_ms);
    } else {
        ui.settings_input.move_left(false, now_ms);
    }
    true
}

fn settings_accepts(
    focus: SettingsFocus,
    input: &jian_core::text_input::TextInputState,
    c: char,
) -> bool {
    match focus {
        SettingsFocus::McpPort => {
            c.is_ascii_digit() && (input.is_select_all() || input.text().len() < 5)
        }
        SettingsFocus::BuiltinAgent {
            field: crate::agent_settings::BuiltinAgentField::Model,
            ..
        }
        | SettingsFocus::BuiltinAgentDraft(crate::agent_settings::BuiltinAgentField::Model) => {
            (c == '\n' || !c.is_control()) && (input.is_select_all() || input.text().len() < 65_536)
        }
        SettingsFocus::ImageSearch(_)
        | SettingsFocus::BuiltinAgent { .. }
        | SettingsFocus::BuiltinAgentDraft(_)
        | SettingsFocus::AcpAgent { .. }
        | SettingsFocus::AcpAgentDraft(_)
        | SettingsFocus::ImageGenProfile { .. } => {
            !c.is_control() && (input.is_select_all() || input.text().len() < 512)
        }
    }
}

// ─── Chat design-block apply ───────────────────────────────────────────

/// Insert a chat design block's parsed nodes onto the active page and
/// mark the source message applied. Returns `true` when the document
/// changed (the host should mark dirty).
pub fn apply_chat_design_block(
    state: &mut EditorState,
    message_index: usize,
    nodes: Vec<PenNode>,
) -> bool {
    if state.apply(EditorCommand::InsertSubtree {
        nodes,
        parent_id: NodeId::NONE,
        page_id: None,
    }) {
        state.chat.mark_message_design_applied(message_index);
        return true;
    }
    false
}

#[cfg(test)]
mod export_dialog_tests {
    use super::*;

    #[test]
    fn opening_a_deck_export_dialog_presets_pdf() {
        let mut ui = EditorUiState {
            scenario: Some(TemplateScene::Slides),
            export_format: ExportFormat::Png,
            ..Default::default()
        };

        ui.open_export_dialog();

        assert!(ui.export_dialog_open);
        assert_eq!(ui.export_format, ExportFormat::Pdf);
    }

    #[test]
    fn opening_a_non_deck_export_dialog_keeps_the_users_format() {
        for scenario in [
            None,
            Some(TemplateScene::Tutorial),
            Some(TemplateScene::Comparison),
            Some(TemplateScene::Carousel),
        ] {
            let mut ui = EditorUiState {
                scenario,
                export_format: ExportFormat::Webp,
                ..Default::default()
            };

            ui.open_export_dialog();

            assert!(ui.export_dialog_open);
            assert_eq!(
                ui.export_format,
                ExportFormat::Webp,
                "scenario {scenario:?} must not overrule the picked format"
            );
        }
    }
}

#[cfg(test)]
mod settings_text_payload_tests {
    use super::*;
    use crate::agent_settings::{AcpAgentField, BuiltinAgentField};

    #[test]
    fn model_payload_preserves_lines_and_normalizes_platform_newlines() {
        let mut ui = EditorUiState::default();
        ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::Model));
        ui.settings_input.set_text("old");
        ui.settings_input.select_all();

        assert!(settings_text_payload(
            &mut ui,
            "model-a\r\nmodel-b\rmodel-c\nmodel-d",
            42,
        ));
        assert_eq!(
            ui.settings_input.text(),
            "model-a\nmodel-b\nmodel-c\nmodel-d"
        );
    }

    #[test]
    fn non_model_settings_payload_remains_single_line() {
        let mut ui = EditorUiState::default();
        ui.agent_settings.focus = Some(SettingsFocus::AcpAgentDraft(AcpAgentField::Command));

        assert!(settings_text_payload(
            &mut ui,
            "node\r\nserver\t--stdio\u{7}",
            42,
        ));
        assert_eq!(ui.settings_input.text(), "nodeserver--stdio");
    }
}

#[cfg(test)]
mod settings_delete_forward_tests {
    use super::*;
    use crate::agent_settings::BuiltinAgentField;

    #[test]
    fn removes_the_character_after_the_caret_while_focused() {
        let mut ui = EditorUiState::default();
        ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::ApiKey));
        ui.settings_input.set_text("sk-old");
        ui.settings_input.set_caret(0, 0);

        assert!(settings_delete_forward(&mut ui, 42));
        assert_eq!(ui.settings_input.text(), "k-old");
        // At the buffer end the key stays owned (no-op edit).
        ui.settings_input.set_caret("k-old".len(), 0);
        assert!(settings_delete_forward(&mut ui, 43));
        assert_eq!(ui.settings_input.text(), "k-old");
    }

    #[test]
    fn ignored_without_a_settings_focus() {
        let mut ui = EditorUiState::default();
        ui.settings_input.set_text("sk-old");

        assert!(!settings_delete_forward(&mut ui, 42));
        assert_eq!(ui.settings_input.text(), "sk-old");
    }

    #[test]
    fn chrome_delete_guard_covers_the_settings_focus() {
        let mut state = crate::EditorState::starter();
        assert!(!crate::host_keyboard_transitions::delete_owned_by_chrome_input(&state));
        state.editor_ui.agent_settings.focus =
            Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::ApiKey));
        assert!(crate::host_keyboard_transitions::delete_owned_by_chrome_input(&state));
    }
}
