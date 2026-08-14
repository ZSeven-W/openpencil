//! Fonts tab of the settings modal.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::translate;
use crate::widgets::missing_fonts_panel::{
    paint_missing_font_row, paint_text, row_button_rect, ROW_HEIGHT,
};
use crate::widgets::property_panel_typography::{
    font_picker_action_in_layout, font_picker_entries, font_picker_hit_in_layout,
    font_picker_layout_at_for_ui, paint_font_picker_at_for_ui, FontPickerAction, FontPickerLayout,
};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};
use jian_widgets::centered_text_baseline_y;
use op_editor_core::editor_ui_state::EditorUiState;
use op_editor_core::missing_fonts::MissingFontEntry;

const TOP_PAD: f32 = 12.0;
const SECTION_TITLE_HEIGHT: f32 = 36.0;
const EMPTY_BODY_HEIGHT: f32 = 44.0;
const SECTION_GAP: f32 = 28.0;
const BOTTOM_PAD: f32 = 24.0;
const REMOVE_HEIGHT: f32 = 28.0;
const IMPORTED_ROW_HEIGHT: f32 = 44.0;
const TOUCH_TOP_PAD: f32 = 16.0;
const TOUCH_SECTION_TITLE_HEIGHT: f32 = 44.0;
const TOUCH_EMPTY_BODY_HEIGHT: f32 = 52.0;
const TOUCH_SECTION_GAP: f32 = 32.0;
const TOUCH_BOTTOM_PAD: f32 = 32.0;
const TOUCH_MISSING_ROW_HEIGHT: f32 = 96.0;
const TOUCH_IMPORTED_ROW_HEIGHT: f32 = 64.0;
const TOUCH_ACTION_HEIGHT: f32 = 44.0;

#[derive(Clone, Copy)]
struct FontsDensity {
    top_pad: f32,
    section_title_height: f32,
    empty_body_height: f32,
    section_gap: f32,
    bottom_pad: f32,
    missing_row_height: f32,
    imported_row_height: f32,
    action_height: f32,
}

fn density(ui: &EditorUiState) -> FontsDensity {
    if ui.touch_chrome() {
        FontsDensity {
            top_pad: TOUCH_TOP_PAD,
            section_title_height: TOUCH_SECTION_TITLE_HEIGHT,
            empty_body_height: TOUCH_EMPTY_BODY_HEIGHT,
            section_gap: TOUCH_SECTION_GAP,
            bottom_pad: TOUCH_BOTTOM_PAD,
            missing_row_height: TOUCH_MISSING_ROW_HEIGHT,
            imported_row_height: TOUCH_IMPORTED_ROW_HEIGHT,
            action_height: TOUCH_ACTION_HEIGHT,
        }
    } else {
        FontsDensity {
            top_pad: TOP_PAD,
            section_title_height: SECTION_TITLE_HEIGHT,
            empty_body_height: EMPTY_BODY_HEIGHT,
            section_gap: SECTION_GAP,
            bottom_pad: BOTTOM_PAD,
            missing_row_height: ROW_HEIGHT,
            imported_row_height: IMPORTED_ROW_HEIGHT,
            action_height: REMOVE_HEIGHT,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontsHit {
    ChooseFont(usize),
    SelectFont(usize),
    ImportFont(usize),
    ClosePicker,
    PickerInside,
    RemoveImportedFont(usize),
    None,
}

fn missing_entries(ui: &EditorUiState) -> &[MissingFontEntry] {
    ui.missing_fonts_prompt
        .as_ref()
        .map(|prompt| prompt.entries.as_slice())
        .unwrap_or_default()
}

fn missing_body_height(ui: &EditorUiState) -> f32 {
    let metrics = density(ui);
    let rows = missing_entries(ui).len();
    if rows == 0 {
        metrics.empty_body_height
    } else {
        rows as f32 * metrics.missing_row_height
    }
}

fn imported_section_top(content: Rect, ui: &EditorUiState) -> f32 {
    let metrics = density(ui);
    content.origin.y
        + metrics.top_pad
        + metrics.section_title_height
        + missing_body_height(ui)
        + metrics.section_gap
}

pub(crate) fn missing_row_rect(content: Rect, ui: &EditorUiState, row: usize) -> Rect {
    let metrics = density(ui);
    Rect::xywh(
        content.origin.x,
        content.origin.y
            + metrics.top_pad
            + metrics.section_title_height
            + row as f32 * metrics.missing_row_height,
        content.size.x,
        metrics.missing_row_height,
    )
}

fn imported_row_rect(content: Rect, ui: &EditorUiState, row: usize) -> Rect {
    let row_height = density(ui).imported_row_height;
    Rect::xywh(
        content.origin.x,
        imported_section_top(content, ui)
            + density(ui).section_title_height
            + row as f32 * row_height,
        content.size.x,
        row_height,
    )
}

fn settings_row_button_rect(row: Rect, ui: &EditorUiState) -> Rect {
    if !ui.touch_chrome() {
        return row_button_rect(row, ui);
    }
    let height = TOUCH_ACTION_HEIGHT;
    let width = (super::missing_fonts_panel::fit_button_width(
        translate(ui, "missingFonts.chooseFont"),
        15.0,
    ) + 8.0)
        .max(height);
    Rect::xywh(
        row.origin.x + row.size.x - width,
        row.origin.y + (row.size.y - height) / 2.0,
        width,
        height,
    )
}

pub(crate) fn imported_remove_rect(content: Rect, ui: &EditorUiState, row: usize) -> Rect {
    let row = imported_row_rect(content, ui, row);
    let touch = ui.touch_chrome();
    let font_size = if touch { 15.0 } else { 11.0 };
    let height = density(ui).action_height;
    let width =
        (super::missing_fonts_panel::fit_button_width(translate(ui, "common.delete"), font_size)
            + if touch { 8.0 } else { 0.0 })
        .max(height);
    Rect::xywh(
        row.origin.x + row.size.x - width,
        row.origin.y + (row.size.y - height) / 2.0,
        width,
        height,
    )
}

pub(super) fn content_height(ui: &EditorUiState) -> f32 {
    let metrics = density(ui);
    metrics.top_pad
        + metrics.section_title_height
        + missing_body_height(ui)
        + metrics.section_gap
        + metrics.section_title_height
        + ui.imported_font_families.len() as f32 * metrics.imported_row_height
        + metrics.bottom_pad
}

fn picker_row(ui: &EditorUiState) -> Option<usize> {
    match ui.font_picker_purpose {
        Some(op_editor_core::FontPickerPurpose::MissingFont {
            row,
            surface: op_editor_core::MissingFontSurface::Settings,
        }) if ui.font_picker.open => Some(row),
        _ => None,
    }
}

pub(crate) fn picker_layout(
    panel: Rect,
    content: Rect,
    ui: &EditorUiState,
    scroll_y: f32,
) -> Option<FontPickerLayout> {
    let row = picker_row(ui)?;
    if missing_entries(ui).get(row)?.resolved {
        return None;
    }
    let entries = font_picker_entries(
        &ui.imported_font_families,
        &ui.bundled_font_families,
        &ui.system_font_families,
        &ui.font_picker_search,
    );
    let touch = ui.touch_chrome();
    let mut trigger = settings_row_button_rect(missing_row_rect(content, ui, row), ui);
    trigger.origin.y -= scroll_y;
    Some(font_picker_layout_at_for_ui(
        trigger,
        300.0,
        picker_bounds(panel, content),
        &entries,
        ui.font_import_supported,
        true,
        ui.font_picker.scroll.offset,
        touch,
    ))
}

fn picker_bounds(panel: Rect, content: Rect) -> Rect {
    let left = content.origin.x.max(panel.origin.x);
    let top = content.origin.y.max(panel.origin.y);
    let right = (content.origin.x + content.size.x).min(panel.origin.x + panel.size.x);
    let bottom = (content.origin.y + content.size.y).min(panel.origin.y + panel.size.y);
    Rect::xywh(left, top, (right - left).max(1.0), (bottom - top).max(1.0))
}

pub fn hit_test(
    panel: Rect,
    content: Rect,
    ui: &EditorUiState,
    point: Point2D,
    scroll_y: f32,
) -> FontsHit {
    if let Some(row) = picker_row(ui) {
        if let Some(layout) = picker_layout(panel, content, ui, scroll_y) {
            if let Some(action) = font_picker_action_in_layout(&layout, point) {
                return match action {
                    FontPickerAction::Select(index) => FontsHit::SelectFont(index),
                    FontPickerAction::Import => FontsHit::ImportFont(row),
                    FontPickerAction::Remove(index) => imported_index_for_picker_entry(ui, index)
                        .map_or(FontsHit::PickerInside, FontsHit::RemoveImportedFont),
                };
            }
            return match font_picker_hit_in_layout(&layout, point) {
                jian_widgets::components::select::SelectHit::Outside => FontsHit::ClosePicker,
                _ => FontsHit::PickerInside,
            };
        }
        return FontsHit::ClosePicker;
    }
    let scrolled = Point2D::new(point.x, point.y + scroll_y);
    for (row, entry) in missing_entries(ui).iter().enumerate() {
        if !entry.resolved
            && settings_row_button_rect(missing_row_rect(content, ui, row), ui).contains(scrolled)
        {
            return FontsHit::ChooseFont(row);
        }
    }
    for row in 0..ui.imported_font_families.len() {
        if imported_remove_rect(content, ui, row).contains(scrolled) {
            return FontsHit::RemoveImportedFont(row);
        }
    }
    FontsHit::None
}

pub(super) fn paint_picker(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    panel: Rect,
    content: Rect,
    ui: &EditorUiState,
    scroll_y: f32,
    now_ms: u64,
) {
    let Some(row) = picker_row(ui) else {
        return;
    };
    let Some(entry) = missing_entries(ui).get(row) else {
        return;
    };
    let entries = font_picker_entries(
        &ui.imported_font_families,
        &ui.bundled_font_families,
        &ui.system_font_families,
        &ui.font_picker_search,
    );
    let touch = ui.touch_chrome();
    let mut trigger = settings_row_button_rect(missing_row_rect(content, ui, row), ui);
    trigger.origin.y -= scroll_y;
    paint_font_picker_at_for_ui(
        cx,
        theme,
        trigger,
        300.0,
        picker_bounds(panel, content),
        ui.effective_locale(),
        &entries,
        ui.font_import_supported,
        true,
        &ui.font_picker_search,
        &ui.font_picker,
        ui.font_picker_import_hover,
        &entry.family,
        now_ms,
        touch,
    );
}

fn imported_index_for_picker_entry(ui: &EditorUiState, entry_index: usize) -> Option<usize> {
    let entries = font_picker_entries(
        &ui.imported_font_families,
        &ui.bundled_font_families,
        &ui.system_font_families,
        &ui.font_picker_search,
    );
    let entry = entries.get(entry_index)?;
    entry.imported.then(|| {
        ui.imported_font_families
            .iter()
            .position(|family| family == &entry.family)
    })?
}

pub(super) fn paint_fonts_tab(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    content: Rect,
) {
    let metrics = density(ui);
    let touch = ui.touch_chrome();
    paint_section_title(
        cx,
        theme,
        translate(ui, "missingFonts.title"),
        content.origin.x,
        content.origin.y + metrics.top_pad,
        touch,
    );

    let entries = missing_entries(ui);
    if entries.is_empty() {
        paint_text(
            cx,
            translate(ui, "missingFonts.noneMissing"),
            Point2D::new(
                content.origin.x,
                content.origin.y
                    + metrics.top_pad
                    + metrics.section_title_height
                    + if touch { 28.0 } else { 20.0 },
            ),
            if touch { 14.0 } else { 12.0 },
            400,
            theme.muted_foreground,
        );
    } else {
        for (row, entry) in entries.iter().enumerate() {
            paint_settings_missing_row(
                cx,
                theme,
                ui,
                entry,
                row,
                missing_row_rect(content, ui, row),
                row > 0,
            );
        }
    }

    let imported_top = imported_section_top(content, ui);
    paint_section_title(
        cx,
        theme,
        translate(ui, "missingFonts.importedSection"),
        content.origin.x,
        imported_top,
        touch,
    );
    for (row, family) in ui.imported_font_families.iter().enumerate() {
        let row_rect = imported_row_rect(content, ui, row);
        let remove = imported_remove_rect(content, ui, row);
        if row > 0 {
            cx.backend.fill_rect(
                Rect::xywh(row_rect.origin.x, row_rect.origin.y, row_rect.size.x, 1.0),
                theme.border,
            );
        }
        let family_font = if touch { 15.0 } else { 13.0 };
        let family_w = (remove.origin.x - row_rect.origin.x - 16.0).max(1.0);
        let family = crate::util::ellipsize_to_width(family, family_w, |text| {
            cx.backend
                .measure_text_family_styled(text, family_font, "system-ui", 500, false)
        });
        cx.backend.save();
        cx.backend.clip_rect(Rect::xywh(
            row_rect.origin.x,
            row_rect.origin.y + 1.0,
            family_w,
            (row_rect.size.y - 2.0).max(1.0),
        ));
        paint_text(
            cx,
            &family,
            Point2D::new(
                row_rect.origin.x,
                centered_text_baseline_y(row_rect, family_font),
            ),
            family_font,
            500,
            theme.foreground,
        );
        cx.backend.restore();
        let remove_hovered = ui.missing_fonts_hover
            == Some(op_editor_core::missing_fonts::MissingFontsHover::RemoveImported(row));
        let remove_bg = if remove_hovered {
            theme.border
        } else {
            theme.muted
        };
        cx.backend.fill_round_rect(remove, 6.0, remove_bg);
        paint_text(
            cx,
            translate(ui, "common.delete"),
            Point2D::new(
                remove.origin.x + 12.0,
                centered_text_baseline_y(remove, if touch { 15.0 } else { 11.0 }),
            ),
            if touch { 15.0 } else { 11.0 },
            500,
            theme.destructive,
        );
    }
}

fn paint_settings_missing_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    entry: &MissingFontEntry,
    row_index: usize,
    row: Rect,
    divider: bool,
) {
    if !ui.touch_chrome() {
        paint_missing_font_row(cx, theme, ui, entry, row_index, row, divider);
        return;
    }
    if divider {
        cx.backend.fill_rect(
            Rect::xywh(row.origin.x, row.origin.y, row.size.x, 1.0),
            theme.border,
        );
    }
    let action = settings_row_button_rect(row, ui);
    let text_w = (action.origin.x - row.origin.x - 16.0).max(1.0);
    let family = crate::util::ellipsize_to_width(&entry.family, text_w, |text| {
        cx.backend.measure_text_weighted(text, 15.0, 600)
    });
    let usage = translate(ui, "missingFonts.usage").replace("{n}", &entry.run_count.to_string());
    let usage = crate::util::ellipsize_to_width(&usage, text_w, |text| {
        cx.backend.measure_text_weighted(text, 14.0, 400)
    });
    cx.backend.save();
    cx.backend.clip_rect(Rect::xywh(
        row.origin.x,
        row.origin.y + 1.0,
        text_w,
        row.size.y - 2.0,
    ));
    paint_text(
        cx,
        &family,
        Point2D::new(row.origin.x, row.origin.y + 34.0),
        15.0,
        600,
        theme.foreground,
    );
    paint_text(
        cx,
        &usage,
        Point2D::new(row.origin.x, row.origin.y + 60.0),
        14.0,
        400,
        theme.muted_foreground,
    );
    if let Some(note) = entry.mismatch_note.as_deref() {
        let note = crate::util::ellipsize_to_width(note, text_w, |text| {
            cx.backend.measure_text_weighted(text, 14.0, 400)
        });
        paint_text(
            cx,
            &note,
            Point2D::new(row.origin.x, row.origin.y + 82.0),
            14.0,
            400,
            theme.destructive,
        );
    }
    cx.backend.restore();

    let hovered = ui.missing_fonts_hover
        == Some(op_editor_core::missing_fonts::MissingFontsHover::ChooseFile(row_index));
    cx.backend.fill_round_rect(
        action,
        8.0,
        if hovered { theme.border } else { theme.muted },
    );
    let label = if entry.resolved {
        translate(ui, "missingFonts.resolved")
    } else {
        translate(ui, "missingFonts.chooseFont")
    };
    paint_text(
        cx,
        label,
        Point2D::new(
            action.origin.x + 16.0,
            centered_text_baseline_y(action, 15.0),
        ),
        15.0,
        500,
        if entry.resolved {
            theme.primary
        } else {
            theme.foreground
        },
    );
}

fn paint_section_title(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    title: &str,
    x: f32,
    y: f32,
    touch: bool,
) {
    let font_size = if touch { 16.0 } else { 15.0 };
    paint_text(
        cx,
        title,
        Point2D::new(x, y + if touch { 28.0 } else { 20.0 }),
        font_size,
        500,
        theme.foreground,
    );
}

#[cfg(test)]
#[path = "agent_settings_fonts_tests.rs"]
mod touch_tests;
