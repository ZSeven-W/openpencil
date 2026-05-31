//! Images tab of the settings modal.

use crate::theme::Theme;
use crate::widgets::agent_settings_caret::paint_caret;
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::agent_settings::{
    AgentSettings, ImageGenField, ImageGenProfile, ImageSearchField, SettingsFocus,
};
use op_editor_core::editor_ui_state::EditorUiState;

const TITLE_H: f32 = 36.0;
const ADVANCED_ROW_H: f32 = 24.0;
const SECTION_GAP: f32 = 28.0;
const SECTION_TITLE_H: f32 = 36.0;
const SUBTITLE_H: f32 = 22.0;
const ROW_H: f32 = 36.0;
const ROW_VGAP: f32 = 10.0;
const LABEL_W: f32 = 110.0;
const TEST_BTN_W: f32 = 56.0;
const ADD_BTN_W: f32 = 72.0;
const BTN_H: f32 = 28.0;
const BODY_GAP: f32 = 14.0;
const REGISTER_ROW_H: f32 = 36.0;
const PROFILE_ROW_H: f32 = 32.0;
const PROFILE_ROW_GAP: f32 = 6.0;
const ACTIVE_DOT: f32 = 14.0;
const DELETE_W: f32 = 24.0;
const PROFILE_FORM_TOP: f32 = 40.0;
const PROFILE_FIELD_H: f32 = 24.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImagesHit {
    ToggleAdvanced,
    FocusSearchField(ImageSearchField),
    TestSearch,
    AddGenConfig,
    SetActiveGenConfig(usize),
    RemoveGenConfig(usize),
    CycleGenProvider(usize),
    FocusGenConfig { index: usize, field: ImageGenField },
    None,
}

fn advanced_body_h() -> f32 {
    SUBTITLE_H + ROW_H + ROW_VGAP + ROW_H + BODY_GAP + REGISTER_ROW_H
}

fn image_gen_section_top(content: Rect, settings: &AgentSettings) -> f32 {
    let mut y = content.origin.y + TITLE_H + ADVANCED_ROW_H;
    if settings.images_advanced_open {
        y += advanced_body_h();
    }
    y + SECTION_GAP
}

pub(super) fn content_height(settings: &AgentSettings) -> f32 {
    let mut h = TITLE_H + ADVANCED_ROW_H;
    if settings.images_advanced_open {
        h += advanced_body_h();
    }
    h + SECTION_GAP + SECTION_TITLE_H + profile_list_h(settings) + 24.0
}

fn profile_list_h(settings: &AgentSettings) -> f32 {
    if settings.image_gen_profiles.is_empty() {
        80.0
    } else {
        settings
            .image_gen_profiles
            .iter()
            .enumerate()
            .map(|(index, _)| profile_row_h(settings, index))
            .sum::<f32>()
            + settings.image_gen_profiles.len().saturating_sub(1) as f32 * PROFILE_ROW_GAP
    }
}

fn profile_row_h(settings: &AgentSettings, index: usize) -> f32 {
    if is_editing_profile(settings, index) {
        PROFILE_ROW_H + 8.0 + 5.0 * ROW_H
    } else {
        PROFILE_ROW_H
    }
}

fn advanced_toggle_rect(content: Rect) -> Rect {
    Rect {
        origin: Point2D::new(content.origin.x, content.origin.y + TITLE_H),
        size: Point2D::new(140.0, ADVANCED_ROW_H),
    }
}

fn register_link_y(content: Rect) -> f32 {
    content.origin.y + TITLE_H + ADVANCED_ROW_H + SUBTITLE_H + ROW_H + ROW_VGAP + ROW_H + BODY_GAP
}

fn search_field_rect(content: Rect, index: usize) -> Rect {
    let y = content.origin.y
        + TITLE_H
        + ADVANCED_ROW_H
        + SUBTITLE_H
        + if index == 0 { 0.0 } else { ROW_H + ROW_VGAP };
    Rect {
        origin: Point2D::new(content.origin.x + LABEL_W, y),
        size: Point2D::new(content.size.x - LABEL_W, ROW_H),
    }
}

fn has_search_credentials(settings: &AgentSettings) -> bool {
    !settings.openverse_client_id.trim().is_empty()
        || !settings.openverse_client_secret.trim().is_empty()
}

fn test_btn_rect(content: Rect, settings: &AgentSettings) -> Rect {
    if !settings.images_advanced_open {
        return Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(0.0, 0.0),
        };
    }
    let y = register_link_y(content) + (REGISTER_ROW_H - BTN_H) / 2.0;
    Rect {
        origin: Point2D::new(content.origin.x + content.size.x - TEST_BTN_W, y),
        size: Point2D::new(TEST_BTN_W, BTN_H),
    }
}

fn add_btn_rect(content: Rect, settings: &AgentSettings) -> Rect {
    let top = image_gen_section_top(content, settings);
    Rect {
        origin: Point2D::new(
            content.origin.x + content.size.x - ADD_BTN_W,
            top + (SECTION_TITLE_H - BTN_H) / 2.0,
        ),
        size: Point2D::new(ADD_BTN_W, BTN_H),
    }
}

fn profile_row_rect(content: Rect, settings: &AgentSettings, index: usize) -> Rect {
    let top = image_gen_section_top(content, settings) + SECTION_TITLE_H;
    let y = settings
        .image_gen_profiles
        .iter()
        .enumerate()
        .take(index)
        .fold(top, |acc, (i, _)| {
            acc + profile_row_h(settings, i) + PROFILE_ROW_GAP
        });
    Rect {
        origin: Point2D::new(content.origin.x, y),
        size: Point2D::new(content.size.x, profile_row_h(settings, index)),
    }
}

fn profile_active_rect(row: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            row.origin.x + 8.0,
            row.origin.y + (PROFILE_ROW_H - ACTIVE_DOT) / 2.0,
        ),
        size: Point2D::new(ACTIVE_DOT, ACTIVE_DOT),
    }
}

fn profile_remove_rect(row: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            row.origin.x + row.size.x - DELETE_W,
            row.origin.y + (PROFILE_ROW_H - DELETE_W) / 2.0,
        ),
        size: Point2D::new(DELETE_W, DELETE_W),
    }
}

fn profile_field_rect(row: Rect, field_index: usize) -> Rect {
    Rect {
        origin: Point2D::new(
            row.origin.x + LABEL_W,
            row.origin.y + PROFILE_FORM_TOP + field_index as f32 * ROW_H,
        ),
        size: Point2D::new(row.size.x - LABEL_W - 12.0, PROFILE_FIELD_H),
    }
}

fn profile_provider_rect(row: Rect) -> Rect {
    profile_field_rect(row, 1)
}

fn profile_field_index(field: ImageGenField) -> usize {
    match field {
        ImageGenField::Name => 0,
        ImageGenField::ApiKey => 2,
        ImageGenField::Model => 3,
        ImageGenField::BaseUrl => 4,
    }
}

pub fn hit_test(content: Rect, settings: &AgentSettings, scrolled: Point2D) -> ImagesHit {
    if rect_contains(advanced_toggle_rect(content), scrolled) {
        return ImagesHit::ToggleAdvanced;
    }
    if settings.images_advanced_open {
        if rect_contains(search_field_rect(content, 0), scrolled) {
            return ImagesHit::FocusSearchField(ImageSearchField::ClientId);
        }
        if rect_contains(search_field_rect(content, 1), scrolled) {
            return ImagesHit::FocusSearchField(ImageSearchField::ClientSecret);
        }
        if has_search_credentials(settings)
            && rect_contains(test_btn_rect(content, settings), scrolled)
        {
            return ImagesHit::TestSearch;
        }
    }
    if rect_contains(add_btn_rect(content, settings), scrolled) {
        return ImagesHit::AddGenConfig;
    }
    for index in 0..settings.image_gen_profiles.len() {
        let row = profile_row_rect(content, settings, index);
        if rect_contains(profile_active_rect(row), scrolled) {
            return ImagesHit::SetActiveGenConfig(index);
        }
        if rect_contains(profile_remove_rect(row), scrolled) {
            return ImagesHit::RemoveGenConfig(index);
        }
        if is_editing_profile(settings, index) {
            if rect_contains(profile_provider_rect(row), scrolled) {
                return ImagesHit::CycleGenProvider(index);
            }
            for field in image_gen_fields() {
                if rect_contains(
                    profile_field_rect(row, profile_field_index(field)),
                    scrolled,
                ) {
                    return ImagesHit::FocusGenConfig { index, field };
                }
            }
        }
        if rect_contains(row, scrolled) {
            return ImagesHit::FocusGenConfig {
                index,
                field: ImageGenField::Name,
            };
        }
    }
    ImagesHit::None
}

pub(super) fn paint_images_tab(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    content: Rect,
    now_ms: u64,
) {
    let title_str = t_settings(ui, "settings.images.search");
    let title = TextLayout::single_run(
        title_str,
        "system-ui",
        15.0,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &title,
        Point2D::new(content.origin.x, content.origin.y + 20.0),
    );
    let title_w = cx.backend.measure_text(title_str, 15.0);
    let ready = settings.images_search_ready;
    let dot_color = if ready {
        Color {
            r: 0.34,
            g: 0.78,
            b: 0.45,
            a: 1.0,
        }
    } else {
        theme.muted_foreground
    };
    // Dot vertically aligned with the status text optical centre,
    // not the title baseline — keeps "● Ready" reading as one
    // horizontal pill instead of the dot drifting downward.
    let dot = Rect {
        origin: Point2D::new(content.origin.x + title_w + 14.0, content.origin.y + 11.0),
        size: Point2D::new(8.0, 8.0),
    };
    cx.backend.fill_oval(dot, dot_color);
    let status_text = if ready {
        t_settings(ui, "settings.images.ready")
    } else {
        t_settings(ui, "settings.images.notConfigured")
    };
    let status = TextLayout::single_run(
        status_text,
        "system-ui",
        12.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &status,
        Point2D::new(content.origin.x + title_w + 30.0, content.origin.y + 20.0),
    );

    // Advanced collapsible row.
    let toggle = advanced_toggle_rect(content);
    let chev_icon = if settings.images_advanced_open {
        Icon::ChevronDown
    } else {
        Icon::ChevronRight
    };
    draw_icon(
        cx.backend,
        chev_icon,
        Point2D::new(
            toggle.origin.x,
            toggle.origin.y + (ADVANCED_ROW_H - 14.0) / 2.0,
        ),
        14.0,
        theme.muted_foreground,
        1.8,
    );
    let advanced_label = TextLayout::single_run(
        t_settings(ui, "settings.images.advanced"),
        "system-ui",
        13.0,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &advanced_label,
        Point2D::new(toggle.origin.x + 22.0, toggle.origin.y + 17.0),
    );

    if settings.images_advanced_open {
        let mut y = toggle.origin.y + ADVANCED_ROW_H;
        let sub = TextLayout::single_run(
            t_settings(ui, "settings.images.oauthLabel"),
            "system-ui",
            12.0,
            to_jian(theme.muted_foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&sub, Point2D::new(content.origin.x, y + 14.0));
        y += SUBTITLE_H;
        paint_search_input_row(
            cx,
            theme,
            settings,
            ui,
            ImageSearchField::ClientId,
            t_settings(ui, "settings.images.clientId"),
            t_settings(ui, "settings.images.clientIdPlaceholder"),
            content.origin.x,
            y,
            content.size.x,
            now_ms,
        );
        y += ROW_H + ROW_VGAP;
        paint_search_input_row(
            cx,
            theme,
            settings,
            ui,
            ImageSearchField::ClientSecret,
            t_settings(ui, "settings.images.clientSecret"),
            t_settings(ui, "settings.images.clientSecretPlaceholder"),
            content.origin.x,
            y,
            content.size.x,
            now_ms,
        );
        y += ROW_H + BODY_GAP;
        let link_text = t_settings(ui, "settings.images.registerLink");
        let link = TextLayout::single_run(
            link_text,
            "system-ui",
            12.0,
            to_jian(theme.primary),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&link, Point2D::new(content.origin.x, y + 22.0));
        let link_w = cx.backend.measure_text(link_text, 12.0);
        draw_icon(
            cx.backend,
            Icon::ArrowUpRight,
            Point2D::new(content.origin.x + link_w + 6.0, y + 10.0),
            14.0,
            theme.primary,
            1.6,
        );
        let test_btn = test_btn_rect(content, settings);
        cx.backend.fill_round_rect(test_btn, 6.0, theme.muted);
        cx.backend
            .stroke_round_rect(test_btn, 6.0, theme.border, 1.0);
        let test_label = t_settings(ui, "settings.images.test");
        let test_w = cx.backend.measure_text(test_label, 13.0);
        let test_lay = TextLayout::single_run(
            test_label,
            "system-ui",
            13.0,
            to_jian(theme.foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &test_lay,
            Point2D::new(
                test_btn.origin.x + (TEST_BTN_W - test_w) / 2.0,
                test_btn.origin.y + BTN_H / 2.0 + 5.0,
            ),
        );
    }

    // Image Generation section.
    let gen_top = image_gen_section_top(content, settings);
    let gen_title = TextLayout::single_run(
        t_settings(ui, "settings.images.generation"),
        "system-ui",
        15.0,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&gen_title, Point2D::new(content.origin.x, gen_top + 20.0));
    let add_btn = add_btn_rect(content, settings);
    cx.backend.fill_round_rect(add_btn, 6.0, theme.muted);
    cx.backend
        .stroke_round_rect(add_btn, 6.0, theme.border, 1.0);
    let add_label = t_settings(ui, "settings.images.add");
    let add_w = cx.backend.measure_text(add_label, 13.0);
    let add_lay = TextLayout::single_run(
        add_label,
        "system-ui",
        13.0,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &add_lay,
        Point2D::new(
            add_btn.origin.x + (ADD_BTN_W - add_w) / 2.0,
            add_btn.origin.y + BTN_H / 2.0 + 5.0,
        ),
    );

    if settings.image_gen_profiles.is_empty() {
        let hint = t_settings(ui, "settings.images.empty");
        let hint_w = cx.backend.measure_text(hint, 13.0);
        let hint_lay = TextLayout::single_run(
            hint,
            "system-ui",
            13.0,
            to_jian(theme.muted_foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &hint_lay,
            Point2D::new(
                content.origin.x + (content.size.x - hint_w) / 2.0,
                gen_top + SECTION_TITLE_H + 48.0,
            ),
        );
    } else {
        for (index, profile) in settings.image_gen_profiles.iter().enumerate() {
            let row = profile_row_rect(content, settings, index);
            paint_profile_row(cx, theme, settings, ui, profile, index, row, now_ms);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_profile_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    profile: &ImageGenProfile,
    index: usize,
    row: Rect,
    now_ms: u64,
) {
    let active = settings.active_image_gen_profile_id.as_deref() == Some(profile.id.as_str());
    let editing = is_editing_profile(settings, index);
    if active || editing {
        cx.backend.fill_round_rect(row, 6.0, theme.muted);
        cx.backend.stroke_round_rect(
            row,
            6.0,
            if active { theme.primary } else { theme.border },
            1.0,
        );
    } else {
        cx.backend.stroke_round_rect(row, 6.0, theme.border, 1.0);
    }
    let dot = profile_active_rect(row);
    if active {
        cx.backend.fill_oval(dot, theme.primary);
        draw_icon(
            cx.backend,
            Icon::Check,
            Point2D::new(dot.origin.x + 3.0, dot.origin.y + 3.0),
            8.0,
            theme.primary_foreground,
            2.0,
        );
    } else {
        cx.backend.stroke_oval(dot, theme.muted_foreground, 1.5);
    }

    let name = if profile.name.trim().is_empty() {
        profile.provider.label()
    } else {
        profile.name.as_str()
    };
    let name = ellipsize(cx, name, row.size.x - 180.0, 12.0);
    let name_lay = TextLayout::single_run(
        &name,
        "system-ui",
        12.0,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &name_lay,
        Point2D::new(row.origin.x + 32.0, row.origin.y + 20.0),
    );

    let provider = profile.provider.label();
    let provider_w = cx.backend.measure_text(provider, 10.0);
    let provider_lay = TextLayout::single_run(
        provider,
        "system-ui",
        10.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &provider_lay,
        Point2D::new(
            row.origin.x + row.size.x - DELETE_W - 12.0 - provider_w,
            row.origin.y + 20.0,
        ),
    );

    draw_icon(
        cx.backend,
        Icon::Trash,
        Point2D::new(
            profile_remove_rect(row).origin.x + 6.0,
            profile_remove_rect(row).origin.y + 6.0,
        ),
        12.0,
        theme.muted_foreground,
        1.5,
    );

    if editing {
        for field in image_gen_fields() {
            paint_profile_field(cx, theme, settings, ui, profile, index, field, row, now_ms);
        }
        paint_provider_field(cx, theme, profile, row);
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_profile_field(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    profile: &ImageGenProfile,
    index: usize,
    field: ImageGenField,
    row: Rect,
    now_ms: u64,
) {
    let focused = settings.focus == Some(SettingsFocus::ImageGenProfile { index, field });
    let value = if focused {
        ui.settings_input_draft.as_str()
    } else {
        match field {
            ImageGenField::Name => profile.name.as_str(),
            ImageGenField::ApiKey if !profile.api_key.is_empty() => "********",
            ImageGenField::ApiKey => "",
            ImageGenField::Model => profile.model.as_str(),
            ImageGenField::BaseUrl => profile.base_url.as_deref().unwrap_or(""),
        }
    };
    let label = match field {
        ImageGenField::Name => "Name",
        ImageGenField::ApiKey => "API Key",
        ImageGenField::Model => "Model",
        ImageGenField::BaseUrl => "Base URL",
    };
    let input = profile_field_rect(row, profile_field_index(field));
    let label_lay = TextLayout::single_run(
        label,
        "system-ui",
        11.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label_lay,
        Point2D::new(row.origin.x + 12.0, input.origin.y + 16.0),
    );
    cx.backend.fill_round_rect(
        input,
        6.0,
        if focused {
            theme.background
        } else {
            theme.card
        },
    );
    cx.backend.stroke_round_rect(
        input,
        6.0,
        if focused { theme.primary } else { theme.border },
        1.0,
    );
    let value = ellipsize(cx, value, input.size.x - 12.0, 11.0);
    let text_x = input.origin.x + 6.0;
    let value_lay = TextLayout::single_run(
        &value,
        "system-ui",
        11.0,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&value_lay, Point2D::new(text_x, input.origin.y + 16.0));
    if focused {
        let caret_x = (text_x + cx.backend.measure_text(&value, 11.0) + 1.0)
            .min(input.origin.x + input.size.x - 8.0);
        let caret_y = input.origin.y + 4.5;
        let anchor = ui.settings_input_caret_anchor_ms;
        paint_caret(cx, theme, now_ms, anchor, caret_x, caret_y);
    }
}

fn paint_provider_field(cx: &mut PaintCx<'_>, theme: &Theme, profile: &ImageGenProfile, row: Rect) {
    let input = profile_provider_rect(row);
    let label_lay = TextLayout::single_run(
        "Provider",
        "system-ui",
        11.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label_lay,
        Point2D::new(row.origin.x + 12.0, input.origin.y + 16.0),
    );
    cx.backend.fill_round_rect(input, 6.0, theme.card);
    cx.backend.stroke_round_rect(input, 6.0, theme.border, 1.0);
    let value = ellipsize(cx, profile.provider.label(), input.size.x - 28.0, 11.0);
    let value_lay = TextLayout::single_run(
        &value,
        "system-ui",
        11.0,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &value_lay,
        Point2D::new(input.origin.x + 6.0, input.origin.y + 16.0),
    );
    draw_icon(
        cx.backend,
        Icon::ChevronDown,
        Point2D::new(input.origin.x + input.size.x - 18.0, input.origin.y + 5.0),
        14.0,
        theme.muted_foreground,
        1.5,
    );
}

#[allow(clippy::too_many_arguments)]
fn paint_search_input_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    field_kind: ImageSearchField,
    label: &str,
    placeholder: &str,
    x: f32,
    y: f32,
    w: f32,
    now_ms: u64,
) {
    let label_lay = TextLayout::single_run(
        label,
        "system-ui",
        13.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&label_lay, Point2D::new(x, y + ROW_H / 2.0 + 5.0));
    let field = Rect {
        origin: Point2D::new(x + LABEL_W, y),
        size: Point2D::new(w - LABEL_W, ROW_H),
    };
    cx.backend.fill_round_rect(field, 6.0, theme.background);
    let focused = settings.focus == Some(SettingsFocus::ImageSearch(field_kind));
    cx.backend.stroke_round_rect(
        field,
        6.0,
        if focused { theme.primary } else { theme.border },
        1.0,
    );
    let stored = match field_kind {
        ImageSearchField::ClientId => settings.openverse_client_id.as_str(),
        ImageSearchField::ClientSecret => settings.openverse_client_secret.as_str(),
    };
    let text = if focused {
        ui.settings_input_draft.as_str()
    } else if matches!(field_kind, ImageSearchField::ClientSecret) && !stored.is_empty() {
        "********"
    } else {
        stored
    };
    let showing_placeholder = text.is_empty();
    let value = if showing_placeholder {
        placeholder
    } else {
        text
    };
    let value = ellipsize(cx, value, field.size.x - 24.0, 13.0);
    let text_x = field.origin.x + 12.0;
    let lay = TextLayout::single_run(
        &value,
        "system-ui",
        13.0,
        to_jian(if showing_placeholder {
            theme.muted_foreground
        } else {
            theme.foreground
        }),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &lay,
        Point2D::new(text_x, field.origin.y + ROW_H / 2.0 + 5.0),
    );
    if focused {
        let caret_text = ellipsize(cx, text, field.size.x - 24.0, 13.0);
        let caret_x = (text_x + cx.backend.measure_text(&caret_text, 13.0) + 1.0)
            .min(field.origin.x + field.size.x - 12.0);
        let caret_y = field.origin.y + (ROW_H - 15.0) / 2.0;
        let anchor = ui.settings_input_caret_anchor_ms;
        paint_caret(cx, theme, now_ms, anchor, caret_x, caret_y);
    }
}

fn ellipsize(cx: &mut PaintCx<'_>, value: &str, max_w: f32, size: f32) -> String {
    if cx.backend.measure_text(value, size) <= max_w {
        return value.to_string();
    }
    let mut out = value.to_string();
    while !out.is_empty() && cx.backend.measure_text(&format!("{out}..."), size) > max_w {
        out.pop();
    }
    format!("{out}...")
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

fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.y >= r.origin.y
        && p.x <= r.origin.x + r.size.x
        && p.y <= r.origin.y + r.size.y
}

fn to_jian(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}
