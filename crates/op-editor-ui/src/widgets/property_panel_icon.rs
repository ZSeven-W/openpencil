//! Icon section for path/icon_font selections.

use crate::theme::Theme;
use crate::widgets::icon_catalog::lookup_icon;
use crate::widgets::icons::{draw_icon, draw_icon_catalog_entry, Icon};
use crate::widgets::property_panel::PropertyPanelAction;
use crate::widgets::property_panel_inputs::{
    paint_dropdown, paint_section_divider, paint_section_label, to_jian_color, INPUT_HEIGHT, PAD_X,
    SECTION_GAP, SECTION_HEADER_HEIGHT,
};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};

pub fn icon_section_height() -> f32 {
    SECTION_HEADER_HEIGHT + INPUT_HEIGHT * 2.0 + 6.0 + 12.0 + SECTION_GAP
}

pub fn push_icon_action_rects(
    out: &mut Vec<(PropertyPanelAction, Rect)>,
    x: f32,
    y: f32,
    width: f32,
) {
    let usable_w = width - PAD_X * 2.0;
    let row_y = y + SECTION_HEADER_HEIGHT;
    for offset in [0.0, INPUT_HEIGHT + 6.0] {
        out.push((
            PropertyPanelAction::OpenSelectedIconPicker,
            Rect {
                origin: Point2D::new(x + PAD_X, row_y + offset),
                size: Point2D::new(usable_w, INPUT_HEIGHT),
            },
        ));
    }
}

pub fn paint_icon_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &crate::widgets::property_panel::NodeSnapshot,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let Some(icon) = snapshot.icon.as_ref() else {
        return y;
    };
    let mut y = paint_section_label(
        cx,
        theme,
        translate(locale, "icon.title", "Icon"),
        x,
        y,
        width,
    );
    let row = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(width - PAD_X * 2.0, INPUT_HEIGHT),
    };
    paint_icon_name_row(cx, theme, row, icon.family.as_str(), icon.name.as_str());
    y += INPUT_HEIGHT + 6.0;
    let library = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(width - PAD_X * 2.0, INPUT_HEIGHT),
    };
    paint_dropdown(cx, theme, library, display_family(icon.family.as_str()));
    y += INPUT_HEIGHT + 12.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

fn paint_icon_name_row(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, family: &str, name: &str) {
    cx.backend.fill_round_rect(
        rect,
        crate::widgets::property_panel_inputs::INPUT_RADIUS,
        theme.muted,
    );
    if let Some(icon) = lookup_icon(family, name) {
        draw_icon_catalog_entry(
            cx.backend,
            icon,
            Point2D::new(rect.origin.x + 9.0, rect.origin.y + 7.0),
            16.0,
            theme.muted_foreground,
            1.5,
        );
    } else if let Some(icon) = Icon::from_name(name) {
        draw_icon(
            cx.backend,
            icon,
            Point2D::new(rect.origin.x + 9.0, rect.origin.y + 7.0),
            16.0,
            theme.muted_foreground,
            1.5,
        );
    } else {
        draw_icon(
            cx.backend,
            Icon::ImagePlus,
            Point2D::new(rect.origin.x + 9.0, rect.origin.y + 7.0),
            16.0,
            theme.muted_foreground,
            1.5,
        );
    }
    let label = TextLayout::single_run(
        name,
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label,
        Point2D::new(rect.origin.x + 34.0, rect.origin.y + 19.0),
    );
    draw_icon(
        cx.backend,
        Icon::ChevronDown,
        Point2D::new(rect.origin.x + rect.size.x - 22.0, rect.origin.y + 5.0),
        16.0,
        theme.muted_foreground,
        1.4,
    );
}

fn display_family(family: &str) -> &str {
    if family.eq_ignore_ascii_case("lucide") {
        "Lucide"
    } else {
        family
    }
}

fn translate(
    locale: op_editor_core::Locale,
    key: &'static str,
    fallback: &'static str,
) -> &'static str {
    let translated = crate::i18n::translate(locale, key);
    if translated == key {
        fallback
    } else {
        translated
    }
}
