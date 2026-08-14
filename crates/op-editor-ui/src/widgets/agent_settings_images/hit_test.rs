//! Hit-testing + hover probes for the Images settings tab. Carved off
//! `agent_settings_images.rs` to keep every file under the 800-line cap;
//! all rect maths lives on the spine so paint and hit-test agree.

use super::*;

pub fn hit_test(content: Rect, settings: &AgentSettings, scrolled: Point2D) -> ImagesHit {
    hit_test_for_ui(content, settings, scrolled, false)
}

pub fn hit_test_for_ui(
    content: Rect,
    settings: &AgentSettings,
    scrolled: Point2D,
    touch: bool,
) -> ImagesHit {
    if (advanced_toggle_rect_for_ui(content, touch)).contains(scrolled) {
        return ImagesHit::ToggleAdvanced;
    }
    if settings.images_advanced_open {
        if (search_field_rect_for_ui(content, 0, touch)).contains(scrolled) {
            return ImagesHit::FocusSearchField(ImageSearchField::ClientId);
        }
        if (search_field_rect_for_ui(content, 1, touch)).contains(scrolled) {
            return ImagesHit::FocusSearchField(ImageSearchField::ClientSecret);
        }
        if (register_link_rect_for_ui(content, touch)).contains(scrolled) {
            return ImagesHit::OpenRegisterLink;
        }
        if search_test_enabled(settings)
            && (test_btn_rect_for_ui(content, settings, touch)).contains(scrolled)
        {
            return ImagesHit::TestSearch;
        }
    }
    if (add_btn_rect_for_ui(content, settings, touch)).contains(scrolled) {
        return ImagesHit::AddGenConfig;
    }
    for (index, profile) in settings.image_gen_profiles.iter().enumerate() {
        let row = profile_row_rect_for_ui(content, settings, index, touch);
        if settings.image_gen_provider_menu_open == Some(index) {
            for (option_index, provider) in ImageGenProvider::ALL.iter().enumerate() {
                if (profile_provider_option_rect_for_ui(row, settings, index, option_index, touch))
                    .contains(scrolled)
                {
                    return ImagesHit::SelectGenProvider {
                        index,
                        provider: *provider,
                    };
                }
            }
        }
        if (profile_active_target_rect(row, touch)).contains(scrolled) {
            return ImagesHit::SetActiveGenConfig(index);
        }
        if (profile_remove_rect_for_ui(row, touch)).contains(scrolled) {
            return ImagesHit::RemoveGenConfig(index);
        }
        if (profile_header_rect_for_ui(row, touch)).contains(scrolled) {
            return ImagesHit::ToggleGenConfigEditor(index);
        }
        if is_editing_profile(settings, index) {
            if (profile_test_btn_rect_for_ui(row, settings, index, touch)).contains(scrolled) {
                if profile_test_enabled(profile) {
                    return ImagesHit::TestGenConfig(index);
                }
                return ImagesHit::None;
            }
            if (profile_provider_rect_for_ui(row, settings, index, touch)).contains(scrolled) {
                return ImagesHit::ToggleGenProviderMenu(index);
            }
            for field in image_gen_fields() {
                if (profile_input_rect_for_ui(row, settings, index, field, touch))
                    .contains(scrolled)
                {
                    return ImagesHit::FocusGenConfig { index, field };
                }
            }
        }
        if (row).contains(scrolled) {
            return ImagesHit::FocusGenConfig {
                index,
                field: ImageGenField::Name,
            };
        }
    }
    ImagesHit::None
}

pub fn search_test_button_hover_at(
    content: Rect,
    settings: &AgentSettings,
    scrolled: Point2D,
) -> bool {
    search_test_button_hover_at_for_ui(content, settings, scrolled, false)
}

pub fn search_test_button_hover_at_for_ui(
    content: Rect,
    settings: &AgentSettings,
    scrolled: Point2D,
    touch: bool,
) -> bool {
    settings.images_advanced_open
        && (test_btn_rect_for_ui(content, settings, touch)).contains(scrolled)
}

pub fn add_gen_button_hover_at(content: Rect, settings: &AgentSettings, scrolled: Point2D) -> bool {
    add_gen_button_hover_at_for_ui(content, settings, scrolled, false)
}

pub fn add_gen_button_hover_at_for_ui(
    content: Rect,
    settings: &AgentSettings,
    scrolled: Point2D,
    touch: bool,
) -> bool {
    (add_btn_rect_for_ui(content, settings, touch)).contains(scrolled)
}

pub fn profile_test_button_hover_at(
    content: Rect,
    settings: &AgentSettings,
    scrolled: Point2D,
) -> Option<usize> {
    profile_test_button_hover_at_for_ui(content, settings, scrolled, false)
}

pub fn profile_test_button_hover_at_for_ui(
    content: Rect,
    settings: &AgentSettings,
    scrolled: Point2D,
    touch: bool,
) -> Option<usize> {
    (0..settings.image_gen_profiles.len()).find(|&index| {
        let row = profile_row_rect_for_ui(content, settings, index, touch);
        is_editing_profile(settings, index)
            && (profile_test_btn_rect_for_ui(row, settings, index, touch)).contains(scrolled)
    })
}
