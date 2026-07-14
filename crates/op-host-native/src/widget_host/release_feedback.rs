//! Shared pressed-state release commits for native pointer release.

use super::WidgetHostNative;
use op_editor_core::{agent_settings::ImageGenField, AgentSettingsButton, ButtonPressTarget};
use op_editor_ui::widgets::{FontWeightChoice, PropertyPanelAction};

impl WidgetHostNative {
    pub(in crate::widget_host) fn release_pressed_feedback(&mut self) -> bool {
        let pressed_button = self.editor_state.editor_ui.pressed_button.take();
        let button_released = pressed_button.is_some();
        let chat_model_pressed = self.editor_state.editor_ui.chat_model_picker.pressed.take();
        let chat_model_released = chat_model_pressed.is_some();
        let tracked_picker_pressed = self
            .editor_state
            .editor_ui
            .git_panel
            .tracked_picker
            .pressed
            .take();
        let tracked_picker_released = tracked_picker_pressed.is_some();
        let icon_picker_released = self
            .editor_state
            .editor_ui
            .icon_picker
            .pressed
            .take()
            .is_some();

        self.commit_deferred_pressed_button(pressed_button);
        self.commit_deferred_chat_model(chat_model_pressed);
        self.commit_deferred_tracked_picker(tracked_picker_pressed);

        let released = button_released
            || chat_model_released
            || tracked_picker_released
            || icon_picker_released;
        if released {
            self.mark_dirty();
        }
        released
    }

    fn commit_deferred_pressed_button(&mut self, pressed: Option<ButtonPressTarget>) {
        match pressed {
            Some(ButtonPressTarget::FontWeightPicker(index)) => {
                if let Some(choice) = FontWeightChoice::ALL.get(index).copied() {
                    self.apply_property_action(PropertyPanelAction::SetFontWeight(choice));
                }
            }
            Some(ButtonPressTarget::AgentSettings(AgentSettingsButton::ImageProviderOption {
                index,
                provider,
            })) => {
                {
                    let settings = &mut self.editor_state.editor_ui.agent_settings;
                    settings.take_over_browser_image_profile(index);
                    if let Some(profile) = settings.image_gen_profiles.get_mut(index) {
                        if profile.provider != provider {
                            profile.provider = provider;
                            profile.model.clear();
                        }
                    }
                    settings.image_gen_provider_menu_open = None;
                }
                self.focus_image_gen_profile(index, ImageGenField::Name);
            }
            _ => {}
        }
    }

    fn commit_deferred_chat_model(&mut self, pressed: Option<usize>) {
        if let Some(index) = pressed {
            self.editor_state.select_chat_model(index);
        }
    }

    fn commit_deferred_tracked_picker(&mut self, pressed: Option<usize>) {
        let Some(index) = pressed else {
            return;
        };
        let panel = &mut self.editor_state.editor_ui.git_panel;
        if index < panel.candidate_files.len() {
            panel.tracked_picker_selected = Some(index);
            panel.tracked_picker.hover = Some(index);
        }
    }
}
