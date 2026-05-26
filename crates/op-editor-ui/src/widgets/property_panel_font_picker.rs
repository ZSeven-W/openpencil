//! Scrollable font-family picker for the native text property section.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::PropertyPanelAction;
use crate::widgets::property_panel_action::BUILT_IN_FONT_FAMILIES;
use crate::widgets::property_panel_inputs::{
    to_jian_color, INPUT_HEIGHT, PAD_X, SECTION_GAP, SECTION_HEADER_HEIGHT, TAB_HEIGHT,
};
use crate::widgets::property_panel_layout::VisibleSections;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use std::collections::HashSet;

const BUTTON_H: f32 = 28.0;
const TEXT_LAYOUT_BLOCK_H: f32 = SECTION_HEADER_HEIGHT + BUTTON_H + 12.0;
const FONT_ROW_H: f32 = 28.0;
const FONT_PICKER_PAD_Y: f32 = 6.0;
const FONT_PICKER_MAX_H: f32 = 320.0;
const CJK_FONT_PRIORITY: [&str; 16] = [
    "pingfang sc",
    "hiragino sans gb",
    "songti sc",
    "heiti sc",
    "kaiti sc",
    "yuanti sc",
    "stheiti",
    "stsong",
    "stkaiti",
    "microsoft yahei",
    "simsun",
    "simhei",
    "noto sans cjk",
    "noto serif cjk",
    "source han",
    "wenquanyi",
];
const CJK_FONT_KEYWORDS: [&str; 25] = [
    "pingfang",
    "hiragino",
    "songti",
    "heiti",
    "kaiti",
    "yuanti",
    "stheiti",
    "stsong",
    "stkaiti",
    "stfangsong",
    "yahei",
    "simsun",
    "simhei",
    "fangsong",
    "mingliu",
    "lihei",
    "lisong",
    "apple li",
    "noto sans cjk",
    "noto serif cjk",
    "noto sans sc",
    "noto serif sc",
    "source han",
    "wenquanyi",
    "hanwang",
];

pub fn prepare_system_font_families(system_fonts: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for family in prioritized_system_fonts(&system_fonts) {
        push_font_family(&mut out, &mut seen, &family);
    }
    out
}

pub fn font_family_options(system_fonts: &[String], active_family: &str) -> Vec<String> {
    font_family_option_refs(system_fonts, active_family)
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn font_family_option_refs<'a>(system_fonts: &'a [String], active_family: &'a str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    push_font_family_ref(&mut out, &mut seen, display_font_family(active_family));
    for family in BUILT_IN_FONT_FAMILIES {
        push_font_family_ref(&mut out, &mut seen, family);
    }
    for family in system_fonts {
        push_font_family_ref(&mut out, &mut seen, family);
    }
    out
}

pub fn font_family_picker_rect(
    panel_rect: Rect,
    visible: VisibleSections,
    active_family: &str,
    system_fonts: &[String],
) -> Option<Rect> {
    let rows = font_family_option_refs(system_fonts, active_family).len();
    if rows == 0 {
        return None;
    }
    let x0 = panel_rect.origin.x;
    let usable_w = panel_rect.size.x - PAD_X * 2.0;
    let text_y = text_section_top(panel_rect, visible)?;
    let content_h = rows as f32 * FONT_ROW_H + FONT_PICKER_PAD_Y * 2.0;
    Some(Rect {
        origin: Point2D::new(
            x0 + PAD_X,
            text_y + TEXT_LAYOUT_BLOCK_H + SECTION_HEADER_HEIGHT + INPUT_HEIGHT - 2.0,
        ),
        size: Point2D::new(usable_w, content_h.min(FONT_PICKER_MAX_H)),
    })
}

pub fn font_family_picker_max_scroll(
    panel_rect: Rect,
    visible: VisibleSections,
    active_family: &str,
    system_fonts: &[String],
) -> f32 {
    let rows = font_family_option_refs(system_fonts, active_family).len();
    let Some(picker) = font_family_picker_rect(panel_rect, visible, active_family, system_fonts)
    else {
        return 0.0;
    };
    let content_h = rows as f32 * FONT_ROW_H + FONT_PICKER_PAD_Y * 2.0;
    (content_h - picker.size.y).max(0.0)
}

pub fn font_family_picker_row_rect(
    panel_rect: Rect,
    visible: VisibleSections,
    active_family: &str,
    system_fonts: &[String],
    scroll: f32,
    target_family: &str,
) -> Option<Rect> {
    let picker = font_family_picker_rect(panel_rect, visible, active_family, system_fonts)?;
    let options = font_family_option_refs(system_fonts, active_family);
    let index = options.iter().position(|family| {
        *family == target_family || display_font_family(family) == target_family
    })?;
    let max = font_family_picker_max_scroll(panel_rect, visible, active_family, system_fonts);
    let y =
        picker.origin.y + FONT_PICKER_PAD_Y + index as f32 * FONT_ROW_H - scroll.clamp(0.0, max);
    let row = Rect {
        origin: Point2D::new(picker.origin.x, y),
        size: Point2D::new(picker.size.x, FONT_ROW_H),
    };
    if row.origin.y + row.size.y < picker.origin.y || row.origin.y > picker.origin.y + picker.size.y
    {
        return None;
    }
    Some(row)
}

pub fn font_family_picker_action_at(
    panel_rect: Rect,
    visible: VisibleSections,
    active_family: &str,
    system_fonts: &[String],
    scroll: f32,
    point: Point2D,
) -> Option<PropertyPanelAction> {
    let picker = font_family_picker_rect(panel_rect, visible, active_family, system_fonts)?;
    if !rect_contains(picker, point) {
        return None;
    }
    let options = font_family_option_refs(system_fonts, active_family);
    let max = font_family_picker_max_scroll(panel_rect, visible, active_family, system_fonts);
    let local_y = point.y - picker.origin.y - FONT_PICKER_PAD_Y + scroll.clamp(0.0, max);
    if local_y < 0.0 {
        return None;
    }
    let index = (local_y / FONT_ROW_H).floor() as usize;
    options
        .get(index)
        .map(|family| (*family).to_string())
        .map(PropertyPanelAction::SetFontFamily)
}

pub fn paint_font_family_picker(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    panel_rect: Rect,
    visible: VisibleSections,
    active_family: &str,
    system_fonts: &[String],
    scroll: f32,
) {
    let Some(picker) = font_family_picker_rect(panel_rect, visible, active_family, system_fonts)
    else {
        return;
    };
    let options = font_family_option_refs(system_fonts, active_family);
    let max = font_family_picker_max_scroll(panel_rect, visible, active_family, system_fonts);
    let scroll = scroll.clamp(0.0, max);
    cx.backend.fill_round_rect(picker, 8.0, theme.popover);
    cx.backend.stroke_round_rect(picker, 8.0, theme.border, 1.0);
    cx.backend.save();
    cx.backend.clip_rect(picker);
    cx.backend.translate(Point2D::new(0.0, -scroll));
    let active = display_font_family(active_family);
    let start = (scroll / FONT_ROW_H).floor().max(0.0) as usize;
    let visible_count = (picker.size.y / FONT_ROW_H).ceil() as usize + 2;
    for (index, family) in options.iter().enumerate().skip(start).take(visible_count) {
        let row = Rect {
            origin: Point2D::new(
                picker.origin.x,
                picker.origin.y + FONT_PICKER_PAD_Y + index as f32 * FONT_ROW_H,
            ),
            size: Point2D::new(picker.size.x, FONT_ROW_H),
        };
        paint_font_row(
            cx,
            theme,
            row,
            family,
            display_font_family(family) == active,
        );
    }
    cx.backend.restore();
    paint_scrollbar(cx, theme, picker, options.len(), scroll);
}

fn paint_font_row(cx: &mut PaintCx<'_>, theme: &Theme, row: Rect, family: &str, is_active: bool) {
    if is_active {
        cx.backend
            .fill_round_rect(row, 6.0, theme.row_selected_primary);
    }
    let label = TextLayout::single_run(
        family,
        "system-ui",
        12.0,
        to_jian_color(if is_active {
            theme.primary
        } else {
            theme.foreground
        }),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label,
        Point2D::new(row.origin.x + 10.0, row.origin.y + 19.0),
    );
    if is_active {
        draw_icon(
            cx.backend,
            Icon::Check,
            Point2D::new(row.origin.x + row.size.x - 22.0, row.origin.y + 7.0),
            14.0,
            theme.primary,
            1.6,
        );
    }
}

fn paint_scrollbar(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    picker: Rect,
    row_count: usize,
    scroll: f32,
) {
    let content_h = row_count as f32 * FONT_ROW_H + FONT_PICKER_PAD_Y * 2.0;
    if content_h <= picker.size.y + 0.5 {
        return;
    }
    let track_h = picker.size.y - 12.0;
    let thumb_h = (track_h * picker.size.y / content_h).max(24.0);
    let max_scroll = (content_h - picker.size.y).max(0.0);
    let t = if max_scroll > 0.0 {
        (scroll / max_scroll).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let thumb = Rect {
        origin: Point2D::new(
            picker.origin.x + picker.size.x - 8.0,
            picker.origin.y + 6.0 + t * (track_h - thumb_h),
        ),
        size: Point2D::new(3.0, thumb_h),
    };
    cx.backend
        .fill_round_rect(thumb, 1.5, theme.muted_foreground);
}

fn prioritized_system_fonts(system_fonts: &[String]) -> Vec<String> {
    let mut cjk = Vec::new();
    let mut rest = Vec::new();
    for family in system_fonts {
        if is_cjk_family(family) {
            cjk.push(family.clone());
        } else {
            rest.push(family.clone());
        }
    }
    cjk.sort_by(|a, b| {
        cjk_rank(a)
            .cmp(&cjk_rank(b))
            .then_with(|| compare_family(a, b))
    });
    rest.sort_by(|a, b| compare_family(a, b));
    cjk.extend(rest);
    cjk
}

fn push_font_family(out: &mut Vec<String>, seen: &mut HashSet<String>, family: &str) {
    let family = family.trim();
    if family.is_empty() {
        return;
    }
    if seen.insert(family.to_lowercase()) {
        out.push(family.to_string());
    }
}

fn push_font_family_ref<'a>(out: &mut Vec<&'a str>, seen: &mut HashSet<String>, family: &'a str) {
    let family = family.trim();
    if family.is_empty() {
        return;
    }
    if seen.insert(family.to_lowercase()) {
        out.push(family);
    }
}

fn is_cjk_family(family: &str) -> bool {
    if !family.is_ascii() {
        return true;
    }
    let lower = family.to_lowercase();
    CJK_FONT_KEYWORDS
        .iter()
        .any(|keyword| lower.contains(keyword))
}

fn cjk_rank(family: &str) -> usize {
    let lower = family.to_lowercase();
    CJK_FONT_PRIORITY
        .iter()
        .position(|keyword| lower.contains(keyword))
        .unwrap_or(CJK_FONT_PRIORITY.len())
}

fn compare_family(a: &str, b: &str) -> std::cmp::Ordering {
    a.to_lowercase().cmp(&b.to_lowercase())
}

fn text_section_top(panel_rect: Rect, visible: VisibleSections) -> Option<f32> {
    if !visible.text {
        return None;
    }
    let mut y = panel_rect.origin.y;
    y += TAB_HEIGHT;
    y += crate::widgets::property_panel_inputs::HEADER_HEIGHT;
    if visible.create_component {
        y += 8.0 + 36.0 + 12.0;
    }
    y += SECTION_HEADER_HEIGHT;
    y += INPUT_HEIGHT + 6.0;
    y += INPUT_HEIGHT + 12.0;
    y += SECTION_GAP;
    if visible.flex_layout {
        y += crate::widgets::property_panel_flex::flex_section_height(visible.flex_layout_mode);
    }
    if visible.size_options {
        y += SECTION_HEADER_HEIGHT;
        y += INPUT_HEIGHT + 10.0;
        y += 22.0 * if visible.clip_content { 3.0 } else { 2.0 };
        y += 12.0 + SECTION_GAP;
    }
    if visible.icon {
        y += crate::widgets::property_panel_icon::icon_section_height();
    }
    Some(y)
}

fn display_font_family(value: &str) -> &str {
    value
        .split(',')
        .next()
        .unwrap_or(value)
        .trim()
        .trim_matches(['"', '\''])
}

fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.x <= r.origin.x + r.size.x
        && p.y >= r.origin.y
        && p.y <= r.origin.y + r.size.y
}
