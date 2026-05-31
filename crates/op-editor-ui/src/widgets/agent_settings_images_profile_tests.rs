use crate::widgets::agent_settings_panel::{AgentSettingsHit, AgentSettingsPanel};
use crate::widgets::{PaintCx, Widget};
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use op_editor_core::agent_settings::{
    AgentSettingsTab, ImageGenField, ImageTestStatus, SettingsFocus,
};
use op_editor_core::EditorState;

#[derive(Default)]
struct CaptureBackend {
    fills: Vec<(Rect, Color)>,
}

impl RenderBackend for CaptureBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.fills.push((rect, color));
    }
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

fn color_eq(a: Color, b: Color) -> bool {
    (a.r - b.r).abs() < 1e-6
        && (a.g - b.g).abs() < 1e-6
        && (a.b - b.b).abs() < 1e-6
        && (a.a - b.a).abs() < 1e-6
}

fn caret_fills(fills: &[(Rect, Color)], color: Color) -> Vec<Rect> {
    fills
        .iter()
        .filter_map(|(rect, fill)| {
            (color_eq(*fill, color)
                && (rect.size.x - 1.5).abs() < 0.01
                && (rect.size.y - 15.0).abs() < 0.01)
                .then_some(*rect)
        })
        .collect()
}

#[test]
fn images_tab_content_height_includes_profile_rows() {
    let mut empty = EditorState::default();
    empty.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    let empty_h = AgentSettingsPanel::for_editor(&empty).content_total_height();

    let mut with_profiles = EditorState::default();
    with_profiles.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    with_profiles
        .editor_ui
        .agent_settings
        .add_image_gen_profile();
    with_profiles
        .editor_ui
        .agent_settings
        .add_image_gen_profile();
    with_profiles
        .editor_ui
        .agent_settings
        .add_image_gen_profile();
    let profiles_h = AgentSettingsPanel::for_editor(&with_profiles).content_total_height();

    assert!(
        profiles_h > empty_h,
        "configured image generation profiles should replace the TS empty state with rows"
    );
}

#[test]
fn focused_image_gen_field_paints_visible_caret_at_blink_on_phase() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    state.editor_ui.agent_settings.add_image_gen_profile();
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::ImageGenProfile {
        index: 0,
        field: ImageGenField::BaseUrl,
    });
    state.editor_ui.settings_input_draft = "https://api.example.com/v1".into();

    let panel = AgentSettingsPanel::for_editor_at(&state, 100);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert_eq!(caret_fills(&backend.fills, panel.theme.foreground).len(), 1);
}

#[test]
fn focused_image_gen_field_hides_caret_at_blink_off_phase() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    state.editor_ui.agent_settings.add_image_gen_profile();
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::ImageGenProfile {
        index: 0,
        field: ImageGenField::BaseUrl,
    });
    state.editor_ui.settings_input_draft = "https://api.example.com/v1".into();

    let panel = AgentSettingsPanel::for_editor_at(&state, 500);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(caret_fills(&backend.fills, panel.theme.foreground).is_empty());
}

#[test]
fn images_tab_expanded_profile_fields_are_focusable() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    state.editor_ui.agent_settings.add_image_gen_profile();
    state.editor_ui.agent_settings.image_gen_profiles[0].api_key = "sk-test".into();
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::ImageGenProfile {
        index: 0,
        field: ImageGenField::Name,
    });
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;

    let gen_top = content_y + 36.0 + 24.0 + 28.0;
    let row_y = gen_top + 36.0;
    let api_field_y = row_y + 32.0 + 8.0 + 36.0 * 2.0;

    assert_eq!(
        panel.hit_test(
            rect,
            crate::Point2D::new(content_x + 110.0 + 20.0, api_field_y + 12.0)
        ),
        AgentSettingsHit::FocusGenConfig {
            index: 0,
            field: ImageGenField::ApiKey,
        }
    );
    assert_eq!(
        panel.hit_test(
            rect,
            crate::Point2D::new(content_x + 430.0, api_field_y + 12.0)
        ),
        AgentSettingsHit::TestGenConfig(0)
    );
    assert!(
        panel.content_total_height() > 180.0,
        "focused image profile should expand to show editable fields"
    );
}

#[test]
fn images_tab_profile_test_is_disabled_while_testing_like_ts() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    state.editor_ui.agent_settings.add_image_gen_profile();
    state.editor_ui.agent_settings.image_gen_profiles[0].api_key = "sk-test".into();
    state.editor_ui.agent_settings.image_gen_profiles[0].test_status = ImageTestStatus::Testing;
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::ImageGenProfile {
        index: 0,
        field: ImageGenField::Name,
    });
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;

    let gen_top = content_y + 36.0 + 24.0 + 28.0;
    let row_y = gen_top + 36.0;
    let api_field_y = row_y + 32.0 + 8.0 + 36.0 * 2.0;

    assert_eq!(
        panel.hit_test(
            rect,
            crate::Point2D::new(content_x + 430.0, api_field_y + 12.0)
        ),
        AgentSettingsHit::Inside
    );
}

#[test]
fn images_tab_expanded_profile_provider_row_is_clickable() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    state.editor_ui.agent_settings.add_image_gen_profile();
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::ImageGenProfile {
        index: 0,
        field: ImageGenField::Name,
    });
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let gen_top = content_y + 36.0 + 24.0 + 28.0;
    let row_y = gen_top + 36.0;
    let provider_y = row_y + 32.0 + 8.0 + 36.0;

    assert_eq!(
        panel.hit_test(
            rect,
            crate::Point2D::new(content_x + 110.0 + 20.0, provider_y + 12.0)
        ),
        AgentSettingsHit::ToggleGenProviderMenu(0)
    );
}
