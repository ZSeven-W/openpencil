//! Images tab of the settings modal.

use crate::theme::Theme;
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::agent_settings_images_parts::{
    ellipsize, paint_profile_field, paint_profile_test_button, paint_provider_field,
    paint_provider_menu, paint_search_input_row,
};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::agent_settings::{
    AgentSettings, ImageGenField, ImageGenProfile, ImageGenProvider, ImageSearchField,
    ImageTestStatus, SettingsFocus,
};
use op_editor_core::editor_ui_state::EditorUiState;
use op_editor_core::{AgentSettingsButton, ButtonPressTarget};

mod hit_test;
mod layout;
mod paint;

pub use hit_test::*;
pub(super) use layout::*;
pub(super) use paint::paint_images_tab;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImagesHit {
    ToggleAdvanced,
    FocusSearchField(ImageSearchField),
    OpenRegisterLink,
    TestSearch,
    AddGenConfig,
    ToggleGenConfigEditor(usize),
    SetActiveGenConfig(usize),
    RemoveGenConfig(usize),
    TestGenConfig(usize),
    ToggleGenProviderMenu(usize),
    SelectGenProvider {
        index: usize,
        provider: ImageGenProvider,
    },
    FocusGenConfig {
        index: usize,
        field: ImageGenField,
    },
    None,
}

pub(super) fn responsive_content_height(
    settings: &AgentSettings,
    content_w: f32,
    touch: bool,
) -> f32 {
    content_height_for_ui(settings, content_w, touch)
}

/// Unscrolled rect for the active Images-tab input. Kept at the module
/// boundary so callers do not depend on the private responsive layout spine.
pub(super) fn focused_input_rect_for_ui(
    content: Rect,
    settings: &AgentSettings,
    focus: SettingsFocus,
    touch: bool,
) -> Option<Rect> {
    match focus {
        SettingsFocus::ImageSearch(field) if settings.images_advanced_open => {
            let index = match field {
                ImageSearchField::ClientId => 0,
                ImageSearchField::ClientSecret => 1,
            };
            Some(search_field_rect_for_ui(content, index, touch))
        }
        SettingsFocus::ImageGenProfile { index, field } => {
            settings.image_gen_profiles.get(index)?;
            let row = profile_row_rect_for_ui(content, settings, index, touch);
            Some(profile_input_rect_for_ui(
                row, settings, index, field, touch,
            ))
        }
        _ => None,
    }
}

/// Click target for the "Register at Openverse" link (text + chevron).
/// A fixed `REGISTER_LINK_W` width covers every locale's link label
/// without reaching the right-aligned Test button.
#[cfg(test)]
pub(super) fn register_link_rect(content: Rect) -> Rect {
    register_link_rect_for_ui(content, false)
}

fn has_search_credentials(settings: &AgentSettings) -> bool {
    !settings.openverse_client_id.trim().is_empty()
        || !settings.openverse_client_secret.trim().is_empty()
}

fn search_test_enabled(settings: &AgentSettings) -> bool {
    has_search_credentials(settings)
        && settings.images_search_test_status != ImageTestStatus::Testing
}

fn profile_test_enabled(profile: &ImageGenProfile) -> bool {
    !profile.api_key.trim().is_empty() && profile.test_status != ImageTestStatus::Testing
}

fn profile_field_index(field: ImageGenField) -> usize {
    match field {
        ImageGenField::Name => 0,
        ImageGenField::ApiKey => 2,
        ImageGenField::Model => 3,
        ImageGenField::BaseUrl => 4,
    }
}

fn image_gen_fields() -> [ImageGenField; 4] {
    use ImageGenField::*;
    [Name, ApiKey, Model, BaseUrl]
}

fn is_editing_profile(settings: &AgentSettings, index: usize) -> bool {
    matches!(
        settings.focus,
        Some(SettingsFocus::ImageGenProfile { index: i, .. }) if i == index
    )
}

#[cfg(test)]
mod touch_tests {
    use super::*;

    fn settings_with_expanded_profile(menu_open: bool) -> AgentSettings {
        let mut settings = AgentSettings {
            images_advanced_open: true,
            openverse_client_id: "client-id".into(),
            ..AgentSettings::default()
        };
        settings.add_image_gen_profile();
        settings.focus = Some(SettingsFocus::ImageGenProfile {
            index: 0,
            field: ImageGenField::Name,
        });
        if menu_open {
            settings.image_gen_provider_menu_open = Some(0);
        }
        settings
    }

    fn center(rect: Rect) -> Point2D {
        Point2D::new(
            rect.origin.x + rect.size.x / 2.0,
            rect.origin.y + rect.size.y / 2.0,
        )
    }

    fn assert_touch_rect(rect: Rect, label: &str) {
        assert!(
            rect.size.x >= 44.0 && rect.size.y >= 44.0,
            "{label} must have a 44pt touch target, got {rect:?}"
        );
    }

    fn assert_above(a: Rect, b: Rect, label: &str) {
        assert!(
            a.origin.y + a.size.y <= b.origin.y + 0.01,
            "{label} must not overlap: {a:?} / {b:?}"
        );
    }

    #[test]
    fn compact_390_stacks_search_and_profile_actions_at_touch_size() {
        let settings = settings_with_expanded_profile(false);
        let content = Rect::xywh(16.0, 80.0, 358.0, 0.0);

        let advanced = advanced_toggle_rect_for_ui(content, true);
        let client_id = search_field_rect_for_ui(content, 0, true);
        let secret = search_field_rect_for_ui(content, 1, true);
        let register = register_link_rect_for_ui(content, true);
        let search_test = test_btn_rect_for_ui(content, &settings, true);
        let add = add_btn_rect_for_ui(content, &settings, true);
        for (rect, label) in [
            (advanced, "advanced"),
            (client_id, "client id"),
            (secret, "client secret"),
            (register, "register link"),
            (search_test, "search test"),
            (add, "add profile"),
        ] {
            assert_touch_rect(rect, label);
        }
        assert_above(client_id, secret, "search fields");
        assert_above(secret, register, "secret and register link");
        assert_above(register, search_test, "compact register and test rows");

        let row = profile_row_rect_for_ui(content, &settings, 0, true);
        let active = profile_active_target_rect(row, true);
        let remove = profile_remove_rect_for_ui(row, true);
        let header = profile_header_rect_for_ui(row, true);
        assert_touch_rect(active, "active profile");
        assert_touch_rect(remove, "remove profile");
        assert_touch_rect(header, "profile header");

        let provider = profile_provider_rect_for_ui(row, &settings, 0, true);
        let api = profile_input_rect_for_ui(row, &settings, 0, ImageGenField::ApiKey, true);
        let test = profile_test_btn_rect_for_ui(row, &settings, 0, true);
        let model = profile_input_rect_for_ui(row, &settings, 0, ImageGenField::Model, true);
        for (rect, label) in [
            (provider, "provider"),
            (api, "api key"),
            (test, "profile test"),
            (model, "model"),
        ] {
            assert_touch_rect(rect, label);
        }
        assert!(api.size.x > 280.0, "API input should retain compact width");
        assert_above(api, test, "API input and test action");
        assert_above(test, model, "test action and model input");

        assert_eq!(
            hit_test_for_ui(content, &settings, center(api), true),
            ImagesHit::FocusGenConfig {
                index: 0,
                field: ImageGenField::ApiKey
            }
        );
        assert_eq!(
            hit_test_for_ui(content, &settings, center(provider), true),
            ImagesHit::ToggleGenProviderMenu(0)
        );
    }

    #[test]
    fn medium_834_keeps_side_by_side_search_action_and_44pt_profile_controls() {
        let settings = settings_with_expanded_profile(false);
        let content = Rect::xywh(80.0, 180.0, 680.0, 0.0);
        let register = register_link_rect_for_ui(content, true);
        let test = test_btn_rect_for_ui(content, &settings, true);
        assert_touch_rect(register, "register link");
        assert_touch_rect(test, "search test");
        assert!(
            register.origin.x + register.size.x <= test.origin.x,
            "medium search actions should not overlap"
        );

        let row = profile_row_rect_for_ui(content, &settings, 0, true);
        for field in image_gen_fields() {
            assert_touch_rect(
                profile_input_rect_for_ui(row, &settings, 0, field, true),
                "profile input",
            );
        }
        assert_eq!(
            hit_test_for_ui(
                content,
                &settings,
                center(profile_remove_rect_for_ui(row, true)),
                true,
            ),
            ImagesHit::RemoveGenConfig(0)
        );
    }

    #[test]
    fn touch_provider_menu_rows_shift_following_fields_and_content_height() {
        let closed = settings_with_expanded_profile(false);
        let open = settings_with_expanded_profile(true);
        let content = Rect::xywh(16.0, 80.0, 358.0, 0.0);
        let closed_row = profile_row_rect_for_ui(content, &closed, 0, true);
        let open_row = profile_row_rect_for_ui(content, &open, 0, true);

        assert_eq!(open_row.size.y - closed_row.size.y, 176.0);
        assert_eq!(
            content_height_for_ui(&open, content.size.x, true)
                - content_height_for_ui(&closed, content.size.x, true),
            176.0
        );
        let api = profile_input_rect_for_ui(open_row, &open, 0, ImageGenField::ApiKey, true);
        for (option_index, expected) in ImageGenProvider::ALL.iter().enumerate() {
            let option =
                profile_provider_option_rect_for_ui(open_row, &open, 0, option_index, true);
            assert_touch_rect(option, "provider option");
            assert_eq!(
                hit_test_for_ui(content, &open, center(option), true),
                ImagesHit::SelectGenProvider {
                    index: 0,
                    provider: *expected
                }
            );
            assert_above(option, api, "provider menu and following API input");
        }
        let last = profile_provider_option_rect_for_ui(open_row, &open, 0, 3, true);
        assert_above(last, api, "complete provider menu and API input");
        let base_url = profile_input_rect_for_ui(open_row, &open, 0, ImageGenField::BaseUrl, true);
        assert!(
            base_url.origin.y + base_url.size.y <= open_row.origin.y + open_row.size.y,
            "expanded profile must contain its final field"
        );
    }

    #[test]
    fn desktop_geometry_keeps_legacy_pixel_density() {
        let settings = settings_with_expanded_profile(true);
        let content = Rect::xywh(40.0, 80.0, 760.0, 0.0);
        assert_eq!(advanced_toggle_rect_for_ui(content, false).size.y, 24.0);
        assert_eq!(search_field_rect_for_ui(content, 0, false).size.y, 36.0);
        assert_eq!(add_btn_rect_for_ui(content, &settings, false).size.y, 28.0);
        let row = profile_row_rect_for_ui(content, &settings, 0, false);
        assert_eq!(profile_header_rect_for_ui(row, false).size.y, 32.0);
        assert_eq!(
            profile_provider_option_rect_for_ui(row, &settings, 0, 0, false)
                .size
                .y,
            24.0
        );
        assert_eq!(
            profile_input_rect_for_ui(row, &settings, 0, ImageGenField::Name, false)
                .size
                .y,
            24.0
        );
    }
}
