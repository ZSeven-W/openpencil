//! Images tab of the settings modal.

use crate::theme::Theme;
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::agent_settings::{AgentSettings, ImageGenProfile};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImagesHit {
    ToggleAdvanced,
    TestSearch,
    AddGenConfig,
    SetActiveGenConfig(usize),
    RemoveGenConfig(usize),
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
        settings.image_gen_profiles.len() as f32 * PROFILE_ROW_H
            + settings.image_gen_profiles.len().saturating_sub(1) as f32 * PROFILE_ROW_GAP
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
    Rect {
        origin: Point2D::new(
            content.origin.x,
            top + index as f32 * (PROFILE_ROW_H + PROFILE_ROW_GAP),
        ),
        size: Point2D::new(content.size.x, PROFILE_ROW_H),
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

pub fn hit_test(content: Rect, settings: &AgentSettings, scrolled: Point2D) -> ImagesHit {
    if rect_contains(advanced_toggle_rect(content), scrolled) {
        return ImagesHit::ToggleAdvanced;
    }
    if settings.images_advanced_open && rect_contains(test_btn_rect(content, settings), scrolled) {
        return ImagesHit::TestSearch;
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
    }
    ImagesHit::None
}

pub(super) fn paint_images_tab(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    content: Rect,
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
        paint_input_row(
            cx,
            theme,
            t_settings(ui, "settings.images.clientId"),
            t_settings(ui, "settings.images.clientIdPlaceholder"),
            content.origin.x,
            y,
            content.size.x,
        );
        y += ROW_H + ROW_VGAP;
        paint_input_row(
            cx,
            theme,
            t_settings(ui, "settings.images.clientSecret"),
            t_settings(ui, "settings.images.clientSecretPlaceholder"),
            content.origin.x,
            y,
            content.size.x,
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
            paint_profile_row(cx, theme, settings, profile, row);
        }
    }
}

fn paint_profile_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    profile: &ImageGenProfile,
    row: Rect,
) {
    let active = settings.active_image_gen_profile_id.as_deref() == Some(profile.id.as_str());
    if active {
        cx.backend.fill_round_rect(row, 6.0, theme.muted);
        cx.backend.stroke_round_rect(row, 6.0, theme.primary, 1.0);
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
}

fn paint_input_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    label: &str,
    placeholder: &str,
    x: f32,
    y: f32,
    w: f32,
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
    cx.backend.stroke_round_rect(field, 6.0, theme.border, 1.0);
    let ph = TextLayout::single_run(
        placeholder,
        "system-ui",
        13.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &ph,
        Point2D::new(field.origin.x + 12.0, field.origin.y + ROW_H / 2.0 + 5.0),
    );
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
