//! Images tab of the settings modal.

use crate::document::{AgentSettings, Document};
use crate::theme::Theme;
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImagesHit {
    ToggleAdvanced,
    TestSearch,
    AddGenConfig,
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
    h + SECTION_GAP + SECTION_TITLE_H + 80.0 + 24.0
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
        return Rect { origin: Point2D::new(0.0, 0.0), size: Point2D::new(0.0, 0.0) };
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
    ImagesHit::None
}

pub(super) fn paint_images_tab(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    doc: &Document,
    content: Rect,
) {
    let title_str = t_settings(doc, "settings.images.search");
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
        Color { r: 0.34, g: 0.78, b: 0.45, a: 1.0 }
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
        t_settings(doc, "settings.images.ready")
    } else {
        t_settings(doc, "settings.images.notConfigured")
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
        Point2D::new(toggle.origin.x, toggle.origin.y + (ADVANCED_ROW_H - 14.0) / 2.0),
        14.0,
        theme.muted_foreground,
        1.8,
    );
    let advanced_label = TextLayout::single_run(
        t_settings(doc, "settings.images.advanced"),
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
            t_settings(doc, "settings.images.oauthLabel"),
            "system-ui",
            12.0,
            to_jian(theme.muted_foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(&sub, Point2D::new(content.origin.x, y + 14.0));
        y += SUBTITLE_H;
        paint_input_row(
            cx,
            theme,
            t_settings(doc, "settings.images.clientId"),
            t_settings(doc, "settings.images.clientIdPlaceholder"),
            content.origin.x,
            y,
            content.size.x,
        );
        y += ROW_H + ROW_VGAP;
        paint_input_row(
            cx,
            theme,
            t_settings(doc, "settings.images.clientSecret"),
            t_settings(doc, "settings.images.clientSecretPlaceholder"),
            content.origin.x,
            y,
            content.size.x,
        );
        y += ROW_H + BODY_GAP;
        let link_text = t_settings(doc, "settings.images.registerLink");
        let link = TextLayout::single_run(
            link_text,
            "system-ui",
            12.0,
            to_jian(theme.primary),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(&link, Point2D::new(content.origin.x, y + 22.0));
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
        cx.backend.stroke_round_rect(test_btn, 6.0, theme.border, 1.0);
        let test_label = t_settings(doc, "settings.images.test");
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
        t_settings(doc, "settings.images.generation"),
        "system-ui",
        15.0,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&gen_title, Point2D::new(content.origin.x, gen_top + 20.0));
    let add_btn = add_btn_rect(content, settings);
    cx.backend.fill_round_rect(add_btn, 6.0, theme.muted);
    cx.backend.stroke_round_rect(add_btn, 6.0, theme.border, 1.0);
    let add_label = t_settings(doc, "settings.images.add");
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

    let hint = t_settings(doc, "settings.images.empty");
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
