//! Shared missing-font modal used by native and web hosts.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::{theme_for, translate};
use crate::widgets::property_panel_typography::{
    font_picker_action_in_layout, font_picker_entries, font_picker_hit_in_layout,
    font_picker_import_hover_in_layout, font_picker_layout_at, paint_font_picker_at,
    FontPickerAction, FontPickerEntry, FontPickerLayout,
};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Point2D, Rect, TextLayout};
use jian_widgets::centered_text_baseline_y;
use op_editor_core::missing_fonts::{MissingFontEntry, MissingFontsPrompt};
use op_editor_core::{EditorState, EditorUiState};

const PANEL_WIDTH: f32 = 480.0;
const BASE_HEIGHT: f32 = 140.0;
const VIEWPORT_MARGIN: f32 = 8.0;
pub(crate) const ROW_HEIGHT: f32 = 72.0;
const ROWS_TOP: f32 = 68.0;
const HORIZONTAL_PAD: f32 = 20.0;
const BUTTON_HEIGHT: f32 = 28.0;
const TEXT_BUTTON_GAP: f32 = 12.0;
const FAMILY_FONT_SIZE: f32 = 13.0;
const FAMILY_FONT_WEIGHT: u16 = 600;
const FAMILY_LINE_HEIGHT: f32 = 16.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingFontsHit {
    ChooseFont(usize),
    SelectFont(usize),
    ImportFont(usize),
    ClosePicker,
    PickerInside,
    Dismiss,
    Inside,
    Outside,
}

pub struct MissingFontsPanel<'a> {
    id: WidgetId,
    theme: Theme,
    prompt: &'a MissingFontsPrompt,
    ui: &'a EditorUiState,
}

impl<'a> MissingFontsPanel<'a> {
    pub fn for_editor(state: &'a EditorState) -> Option<Self> {
        let prompt = state.editor_ui.missing_fonts_prompt.as_ref()?;
        if !state.editor_ui.missing_fonts_modal_open || prompt.entries.is_empty() {
            return None;
        }
        Some(Self {
            id: WidgetId::new(5460),
            theme: theme_for(&state.editor_ui),
            prompt,
            ui: &state.editor_ui,
        })
    }

    pub fn rect(&self, viewport_w: f32, viewport_h: f32) -> Rect {
        let top = crate::widgets::TOP_BAR_HEIGHT + VIEWPORT_MARGIN;
        let max_height = (viewport_h - top - VIEWPORT_MARGIN).max(1.0);
        let natural_height = BASE_HEIGHT + ROW_HEIGHT * self.prompt.entries.len() as f32;
        let height = natural_height.min(max_height);
        let width = PANEL_WIDTH.min((viewport_w - VIEWPORT_MARGIN * 2.0).max(1.0));
        let x = ((viewport_w - width) / 2.0).max(VIEWPORT_MARGIN);
        let y = ((viewport_h - height) / 2.0).max(top);
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(width, height),
        }
    }

    /// Scrollable rows region; the header and footer remain fixed.
    pub fn rows_rect(&self, panel: Rect) -> Rect {
        Rect {
            origin: Point2D::new(panel.origin.x, panel.origin.y + ROWS_TOP),
            size: Point2D::new(panel.size.x, (panel.size.y - BASE_HEIGHT).max(0.0)),
        }
    }

    pub fn max_rows_scroll(&self, panel: Rect) -> f32 {
        (ROW_HEIGHT * self.prompt.entries.len() as f32 - self.rows_rect(panel).size.y).max(0.0)
    }

    fn rows_scroll(&self, panel: Rect) -> f32 {
        self.ui
            .missing_fonts_scroll
            .offset
            .clamp(0.0, self.max_rows_scroll(panel))
    }

    fn picker_row(&self) -> Option<usize> {
        match self.ui.font_picker_purpose {
            Some(op_editor_core::FontPickerPurpose::MissingFont {
                row,
                surface: op_editor_core::MissingFontSurface::Prompt,
            }) if self.ui.font_picker.open => Some(row),
            _ => None,
        }
    }

    pub fn picker_entries(&self) -> std::rc::Rc<Vec<FontPickerEntry>> {
        font_picker_entries(
            &self.ui.imported_font_families,
            &self.ui.bundled_font_families,
            &self.ui.system_font_families,
            &self.ui.font_picker_search,
        )
    }

    pub fn picker_layout(&self, panel: Rect, viewport: Rect) -> Option<FontPickerLayout> {
        let row = self.picker_row()?;
        let entry = self.prompt.entries.get(row)?;
        if entry.resolved {
            return None;
        }
        let entries = self.picker_entries();
        Some(font_picker_layout_at(
            self.row_button_rect(panel, row),
            300.0,
            inset_rect(viewport, 8.0),
            &entries,
            self.ui.font_import_supported,
            false,
            self.ui.font_picker.scroll.offset,
        ))
    }

    pub fn hit_test(&self, panel: Rect, viewport: Rect, point: Point2D) -> MissingFontsHit {
        if let Some(row) = self.picker_row() {
            if let Some(layout) = self.picker_layout(panel, viewport) {
                if let Some(action) = font_picker_action_in_layout(&layout, point) {
                    return match action {
                        FontPickerAction::Select(index) => MissingFontsHit::SelectFont(index),
                        FontPickerAction::Import => MissingFontsHit::ImportFont(row),
                        FontPickerAction::Remove(_) => MissingFontsHit::PickerInside,
                    };
                }
                return match font_picker_hit_in_layout(&layout, point) {
                    jian_widgets::components::select::SelectHit::Outside => {
                        MissingFontsHit::ClosePicker
                    }
                    _ => MissingFontsHit::PickerInside,
                };
            }
            return MissingFontsHit::ClosePicker;
        }
        if !panel.contains(point) {
            return MissingFontsHit::Outside;
        }

        let rows = self.rows_rect(panel);
        for (row, entry) in self.prompt.entries.iter().enumerate() {
            if rows.contains(point)
                && !entry.resolved
                && self.row_button_rect(panel, row).contains(point)
            {
                return MissingFontsHit::ChooseFont(row);
            }
        }

        if dismiss_rect(panel, self.ui).contains(point) {
            MissingFontsHit::Dismiss
        } else {
            MissingFontsHit::Inside
        }
    }

    pub(crate) fn row_button_rect(&self, panel: Rect, row: usize) -> Rect {
        row_button_rect(modal_row_rect(panel, row, self.rows_scroll(panel)), self.ui)
    }

    pub fn picker_hover(
        &self,
        panel: Rect,
        viewport: Rect,
        point: Point2D,
    ) -> (Option<usize>, bool) {
        let Some(layout) = self.picker_layout(panel, viewport) else {
            return (None, false);
        };
        let hover = match font_picker_hit_in_layout(&layout, point) {
            jian_widgets::components::select::SelectHit::Row(index) => Some(index),
            _ => None,
        };
        (hover, font_picker_import_hover_in_layout(&layout, point))
    }

    pub fn paint_picker(&self, cx: &mut PaintCx<'_>, panel: Rect, viewport: Rect, now_ms: u64) {
        let Some(row) = self.picker_row() else {
            return;
        };
        let Some(entry) = self.prompt.entries.get(row) else {
            return;
        };
        let entries = self.picker_entries();
        paint_font_picker_at(
            cx,
            &self.theme,
            self.row_button_rect(panel, row),
            300.0,
            inset_rect(viewport, 8.0),
            self.ui.effective_locale(),
            &entries,
            self.ui.font_import_supported,
            false,
            &self.ui.font_picker_search,
            &self.ui.font_picker,
            self.ui.font_picker_import_hover,
            &entry.family,
            now_ms,
        );
    }
}

fn inset_rect(rect: Rect, inset: f32) -> Rect {
    Rect {
        origin: Point2D::new(rect.origin.x + inset, rect.origin.y + inset),
        size: Point2D::new(
            (rect.size.x - inset * 2.0).max(1.0),
            (rect.size.y - inset * 2.0).max(1.0),
        ),
    }
}

fn modal_row_rect(panel: Rect, row: usize, scroll: f32) -> Rect {
    Rect {
        origin: Point2D::new(
            panel.origin.x + HORIZONTAL_PAD,
            panel.origin.y + ROWS_TOP + row as f32 * ROW_HEIGHT - scroll,
        ),
        size: Point2D::new(panel.size.x - HORIZONTAL_PAD * 2.0, ROW_HEIGHT),
    }
}

/// Shared choose-file geometry used by the modal and Settings Fonts tab.
/// Fit-content pill width: label width + 12px padding each side.
pub(crate) fn fit_button_width(label: &str, font_size: f32) -> f32 {
    // CJK glyphs are full-width (~1.0 x font size); a flat Latin
    // factor undershoots them and clips the label.
    let text: f32 = label
        .chars()
        .map(|c| {
            if (c as u32) > 0x2E7F {
                font_size
            } else {
                font_size * 0.55
            }
        })
        .sum();
    text + 24.0
}

pub(crate) fn row_button_rect(row: Rect, ui: &EditorUiState) -> Rect {
    let width = fit_button_width(translate(ui, "missingFonts.chooseFont"), 11.0);
    Rect {
        origin: Point2D::new(
            row.origin.x + row.size.x - width,
            row.origin.y + (row.size.y - BUTTON_HEIGHT) / 2.0,
        ),
        size: Point2D::new(width, BUTTON_HEIGHT),
    }
}

fn row_text_rect(row: Rect, ui: &EditorUiState) -> Rect {
    let button = row_button_rect(row, ui);
    Rect {
        origin: Point2D::new(row.origin.x, row.origin.y + 1.0),
        size: Point2D::new(
            (button.origin.x - row.origin.x - TEXT_BUTTON_GAP).max(1.0),
            (row.size.y - 2.0).max(1.0),
        ),
    }
}

/// Lay a font-family label out on at most two lines, preferring CSS stack commas.
/// Unbroken names use a character boundary; only the second line is ellipsized.
fn family_label_lines(
    text: &str,
    max_width: f32,
    mut measure: impl FnMut(&str) -> f32,
) -> Vec<String> {
    if measure(text) <= max_width {
        return vec![text.to_owned()];
    }

    let mut fitting_end = 0;
    for (start, ch) in text.char_indices() {
        let end = start + ch.len_utf8();
        if measure(&text[..end]) > max_width {
            break;
        }
        fitting_end = end;
    }
    if fitting_end == 0 {
        return vec!["…".to_owned()];
    }

    let fitting = &text[..fitting_end];
    let mut fitting_break = |separator: fn(char) -> bool| {
        fitting
            .char_indices()
            .filter(|(_, ch)| separator(*ch))
            .map(|(start, ch)| start + ch.len_utf8())
            .next_back()
            .filter(|end| measure(text[..*end].trim_end()) >= max_width * 0.45)
    };
    let preferred_end = fitting_break(|ch| ch == ',')
        .or_else(|| fitting_break(char::is_whitespace))
        .unwrap_or(fitting_end);
    let first = text[..preferred_end].trim_end().to_owned();
    let remaining = text[preferred_end..].trim_start();
    if remaining.is_empty() {
        return vec![first];
    }
    let second = crate::util::ellipsize_to_width(remaining, max_width, &mut measure);
    vec![first, second]
}

fn dismiss_rect(panel: Rect, ui: &EditorUiState) -> Rect {
    let width = fit_button_width(translate(ui, "missingFonts.dismiss"), 12.0);
    Rect {
        origin: Point2D::new(
            panel.origin.x + panel.size.x - HORIZONTAL_PAD - width,
            panel.origin.y + panel.size.y - 44.0,
        ),
        size: Point2D::new(width, BUTTON_HEIGHT),
    }
}

pub(crate) fn paint_text(
    cx: &mut PaintCx<'_>,
    text: &str,
    origin: Point2D,
    font_size: f32,
    weight: u16,
    color: crate::Color,
) {
    let layout = TextLayout::single_run(
        text,
        "system-ui",
        font_size,
        color.to_jian(),
        Point2D::new(0.0, 0.0),
    )
    .with_font_weight(weight);
    cx.backend.draw_text(&layout, origin);
}

/// Paint one missing-font row. Both settings and the one-shot modal call this
/// so status, mismatch, and choose-file affordances cannot drift apart.
pub(crate) fn paint_missing_font_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    entry: &MissingFontEntry,
    row_index: usize,
    row: Rect,
    divider: bool,
) {
    let choose_hovered = ui.missing_fonts_hover
        == Some(op_editor_core::missing_fonts::MissingFontsHover::ChooseFile(row_index));
    if divider {
        cx.backend.fill_rect(
            Rect {
                origin: row.origin,
                size: Point2D::new(row.size.x, 1.0),
            },
            theme.border,
        );
    }
    let text_rect = row_text_rect(row, ui);
    let family_lines = family_label_lines(&entry.family, text_rect.size.x, |text| {
        cx.backend
            .measure_text_weighted(text, FAMILY_FONT_SIZE, FAMILY_FONT_WEIGHT)
    });
    let usage = translate(ui, "missingFonts.usage").replace("{n}", &entry.run_count.to_string());
    let usage = crate::util::ellipsize_to_width(&usage, text_rect.size.x, |text| {
        cx.backend.measure_text_weighted(text, 11.0, 400)
    });
    let mismatch_note = entry.mismatch_note.as_deref().map(|note| {
        crate::util::ellipsize_to_width(note, text_rect.size.x, |text| {
            cx.backend.measure_text_weighted(text, 11.0, 400)
        })
    });

    // Guard backend shaping differences so glyphs cannot overlap the action.
    cx.backend.save();
    cx.backend.clip_rect(text_rect);
    let family_first_y = if family_lines.len() > 1 { 17.0 } else { 22.0 };
    for (line, text) in family_lines.iter().enumerate() {
        paint_text(
            cx,
            text,
            Point2D::new(
                row.origin.x,
                row.origin.y + family_first_y + line as f32 * FAMILY_LINE_HEIGHT,
            ),
            FAMILY_FONT_SIZE,
            FAMILY_FONT_WEIGHT,
            theme.foreground,
        );
    }
    paint_text(
        cx,
        &usage,
        Point2D::new(
            row.origin.x,
            row.origin.y + if family_lines.len() > 1 { 51.0 } else { 44.0 },
        ),
        11.0,
        400,
        theme.muted_foreground,
    );
    if let Some(note) = &mismatch_note {
        paint_text(
            cx,
            note,
            Point2D::new(row.origin.x, row.origin.y + 67.0),
            11.0,
            400,
            theme.destructive,
        );
    }
    cx.backend.restore();

    let action = row_button_rect(row, ui);
    if entry.resolved {
        let chip = Rect {
            origin: Point2D::new(action.origin.x + action.size.x - 76.0, action.origin.y),
            size: Point2D::new(76.0, action.size.y),
        };
        cx.backend
            .fill_round_rect(chip, 6.0, theme.row_selected_primary);
        paint_text(
            cx,
            translate(ui, "missingFonts.resolved"),
            Point2D::new(chip.origin.x + 12.0, centered_text_baseline_y(chip, 11.0)),
            11.0,
            500,
            theme.primary,
        );
    } else {
        // Use `border` because `accent` and `muted` are identical in both palettes.
        let bg = if choose_hovered {
            theme.border
        } else {
            theme.muted
        };
        cx.backend.fill_round_rect(action, 6.0, bg);
        paint_text(
            cx,
            translate(ui, "missingFonts.chooseFont"),
            Point2D::new(
                action.origin.x + 12.0,
                centered_text_baseline_y(action, 11.0),
            ),
            11.0,
            500,
            theme.foreground,
        );
    }
}

impl Widget for MissingFontsPanel<'_> {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(
                    PANEL_WIDTH,
                    BASE_HEIGHT + ROW_HEIGHT * self.prompt.entries.len() as f32,
                ),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        cx.backend.fill_round_rect(panel, 14.0, self.theme.card);
        cx.backend
            .stroke_round_rect(panel, 14.0, self.theme.border, 1.0);

        paint_text(
            cx,
            translate(self.ui, "missingFonts.title"),
            Point2D::new(panel.origin.x + HORIZONTAL_PAD, panel.origin.y + 28.0),
            16.0,
            600,
            self.theme.foreground,
        );
        paint_text(
            cx,
            translate(self.ui, "missingFonts.subtitle"),
            Point2D::new(panel.origin.x + HORIZONTAL_PAD, panel.origin.y + 50.0),
            11.0,
            400,
            self.theme.muted_foreground,
        );

        let rows = self.rows_rect(panel);
        let scroll = self.rows_scroll(panel);
        cx.backend.save();
        cx.backend.clip_rect(rows);
        for (row, entry) in self.prompt.entries.iter().enumerate() {
            let row_rect = modal_row_rect(panel, row, scroll);
            if row_rect.origin.y + row_rect.size.y <= rows.origin.y
                || row_rect.origin.y >= rows.origin.y + rows.size.y
            {
                continue;
            }
            paint_missing_font_row(cx, &self.theme, self.ui, entry, row, row_rect, row > 0);
        }
        cx.backend.restore();

        let dismiss = dismiss_rect(panel, self.ui);
        let dismiss_hovered = self.ui.missing_fonts_hover
            == Some(op_editor_core::missing_fonts::MissingFontsHover::Dismiss);
        if dismiss_hovered {
            cx.backend.fill_round_rect(dismiss, 6.0, self.theme.border);
        }
        paint_text(
            cx,
            translate(self.ui, "missingFonts.dismiss"),
            Point2D::new(
                dismiss.origin.x + 12.0,
                centered_text_baseline_y(dismiss, 12.0),
            ),
            12.0,
            500,
            if dismiss_hovered {
                self.theme.foreground
            } else {
                self.theme.muted_foreground
            },
        );
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Dialog);
        node.set_label(translate(self.ui, "missingFonts.title"));
        node
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::property_panel_typography::FontPickerRow;
    use op_editor_core::missing_fonts::MissingFontEntry;

    fn prompt_state(rows: usize) -> EditorState {
        let mut state = EditorState::new();
        state.editor_ui.missing_fonts_prompt = Some(MissingFontsPrompt {
            entries: (0..rows)
                .map(|row| MissingFontEntry {
                    family: format!("F{row}"),
                    run_count: 1,
                    mismatch_note: None,
                    resolved: false,
                })
                .collect(),
        });
        state.editor_ui.missing_fonts_modal_open = true;
        state
    }

    fn viewport() -> Rect {
        Rect::xywh(0.0, 0.0, 1200.0, 800.0)
    }

    #[test]
    fn long_css_stack_wraps_before_action_without_losing_text() {
        let state = prompt_state(1);
        let row = Rect::xywh(20.0, 30.0, 440.0, ROW_HEIGHT);
        let action = row_button_rect(row, &state.editor_ui);
        let text_rect = row_text_rect(row, &state.editor_ui);
        let family = "Inter,ui-sans-serif,system-ui,-apple-system,\"PingFang SC\",sans-serif";
        let measure = |text: &str| text.chars().count() as f32 * 7.0;
        let lines = family_label_lines(family, text_rect.size.x, measure);

        assert_eq!(lines.len(), 2, "the long stack should use both lines");
        assert_eq!(
            lines.concat(),
            family,
            "a two-line fit should stay complete"
        );
        assert!(lines.iter().all(|line| measure(line) <= text_rect.size.x));
        assert!(
            text_rect.origin.x + text_rect.size.x + TEXT_BUTTON_GAP <= action.origin.x,
            "text and action gutters must not overlap"
        );
    }

    #[test]
    fn exceptionally_long_family_ellipsizes_on_second_line() {
        let family = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let measure = |text: &str| text.chars().count() as f32 * 10.0;
        let lines = family_label_lines(family, 100.0, measure);

        assert_eq!(lines.len(), 2);
        assert!(lines[1].ends_with('…'));
        assert!(lines.iter().all(|line| measure(line) <= 100.0));
    }

    #[test]
    fn action_hit_box_is_vertically_centered_in_the_shared_row() {
        let state = prompt_state(1);
        let row = Rect::xywh(20.0, 30.0, 440.0, ROW_HEIGHT);
        let action = row_button_rect(row, &state.editor_ui);

        assert_eq!(
            action.origin.y,
            row.origin.y + (ROW_HEIGHT - action.size.y) / 2.0
        );
    }

    #[test]
    fn choose_file_button_hit_maps_to_row() {
        let state = prompt_state(2);
        let panel = MissingFontsPanel::for_editor(&state).expect("open");
        let rect = panel.rect(1200.0, 800.0);
        let button = panel.row_button_rect(rect, 1);
        let centre = Point2D::new(
            button.origin.x + button.size.x / 2.0,
            button.origin.y + button.size.y / 2.0,
        );

        assert_eq!(
            panel.hit_test(rect, viewport(), centre),
            MissingFontsHit::ChooseFont(1)
        );
    }

    #[test]
    fn outside_click_does_not_choose() {
        let state = prompt_state(1);
        let panel = MissingFontsPanel::for_editor(&state).expect("open");
        let rect = panel.rect(1200.0, 800.0);

        assert_eq!(
            panel.hit_test(rect, viewport(), Point2D::new(1.0, 1.0)),
            MissingFontsHit::Outside
        );
    }

    #[test]
    fn resolved_row_has_no_button() {
        let mut state = prompt_state(1);
        state
            .editor_ui
            .missing_fonts_prompt
            .as_mut()
            .expect("prompt")
            .entries[0]
            .resolved = true;
        let panel = MissingFontsPanel::for_editor(&state).expect("open");
        let rect = panel.rect(1200.0, 800.0);
        let button = panel.row_button_rect(rect, 0);
        let centre = Point2D::new(
            button.origin.x + button.size.x / 2.0,
            button.origin.y + button.size.y / 2.0,
        );

        assert_eq!(
            panel.hit_test(rect, viewport(), centre),
            MissingFontsHit::Inside
        );
    }

    #[test]
    fn dismiss_button_has_a_dedicated_hit() {
        let state = prompt_state(1);
        let panel = MissingFontsPanel::for_editor(&state).expect("open");
        let rect = panel.rect(1200.0, 800.0);
        let dismiss = dismiss_rect(rect, &state.editor_ui);
        let centre = Point2D::new(
            dismiss.origin.x + dismiss.size.x / 2.0,
            dismiss.origin.y + dismiss.size.y / 2.0,
        );

        assert_eq!(
            panel.hit_test(rect, viewport(), centre),
            MissingFontsHit::Dismiss
        );
    }

    #[test]
    fn long_prompt_is_viewport_bounded_with_fixed_dismiss_and_scrollable_rows() {
        let mut state = prompt_state(20);
        let (rect, max_scroll) = {
            let panel = MissingFontsPanel::for_editor(&state).expect("open");
            let rect = panel.rect(600.0, 400.0);
            assert!(rect.origin.y + rect.size.y <= 392.0);
            assert!(panel.max_rows_scroll(rect) > 0.0);
            (rect, panel.max_rows_scroll(rect))
        };

        state.editor_ui.missing_fonts_scroll.offset = max_scroll;
        let panel = MissingFontsPanel::for_editor(&state).expect("open");
        let last = panel.row_button_rect(rect, 19);
        let last_center = Point2D::new(
            last.origin.x + last.size.x / 2.0,
            last.origin.y + last.size.y / 2.0,
        );
        assert_eq!(
            panel.hit_test(rect, Rect::xywh(0.0, 0.0, 600.0, 400.0), last_center),
            MissingFontsHit::ChooseFont(19)
        );

        let dismiss = dismiss_rect(rect, &state.editor_ui);
        assert!(dismiss.origin.y + dismiss.size.y <= 400.0);
    }

    #[test]
    fn shared_picker_selects_a_system_font_and_exposes_import() {
        let mut state = prompt_state(1);
        state.editor_ui.system_font_families = std::sync::Arc::new(vec!["Arial".to_string()]);
        state.editor_ui.font_import_supported = true;
        state
            .editor_ui
            .open_missing_font_picker(0, op_editor_core::MissingFontSurface::Prompt);
        let panel = MissingFontsPanel::for_editor(&state).expect("open");
        let rect = panel.rect(1200.0, 800.0);
        let layout = panel.picker_layout(rect, viewport()).expect("picker");
        let entries = panel.picker_entries();
        let arial = entries
            .iter()
            .position(|entry| entry.family == "Arial")
            .expect("Arial entry");
        let arial_rect = layout
            .rows
            .iter()
            .find_map(|(row, rect)| {
                matches!(row, FontPickerRow::Entry(index) if *index == arial).then_some(*rect)
            })
            .expect("Arial row");
        let arial_point = Point2D::new(
            arial_rect.origin.x + 12.0,
            arial_rect.origin.y + arial_rect.size.y / 2.0,
        );
        assert_eq!(
            panel.hit_test(rect, viewport(), arial_point),
            MissingFontsHit::SelectFont(arial)
        );

        let import_rect = layout
            .rows
            .iter()
            .find_map(|(row, rect)| matches!(row, FontPickerRow::ImportAction).then_some(*rect))
            .expect("import row");
        let import_point = Point2D::new(
            import_rect.origin.x + 12.0,
            import_rect.origin.y + import_rect.size.y / 2.0,
        );
        assert_eq!(
            panel.hit_test(rect, viewport(), import_point),
            MissingFontsHit::ImportFont(0)
        );
    }

    #[test]
    fn click_outside_shared_picker_closes_only_the_picker() {
        let mut state = prompt_state(1);
        state
            .editor_ui
            .open_missing_font_picker(0, op_editor_core::MissingFontSurface::Prompt);
        let panel = MissingFontsPanel::for_editor(&state).expect("open");
        let rect = panel.rect(1200.0, 800.0);

        assert_eq!(
            panel.hit_test(rect, viewport(), Point2D::new(4.0, 4.0)),
            MissingFontsHit::ClosePicker
        );
    }

    #[test]
    fn closed_or_empty_prompt_builds_none() {
        let state = EditorState::new();
        assert!(MissingFontsPanel::for_editor(&state).is_none());

        let mut state = prompt_state(0);
        assert!(MissingFontsPanel::for_editor(&state).is_none());
        state.editor_ui.missing_fonts_prompt = Some(MissingFontsPrompt {
            entries: vec![MissingFontEntry {
                family: "Katibeh".into(),
                run_count: 1,
                mismatch_note: None,
                resolved: false,
            }],
        });
        state.editor_ui.missing_fonts_modal_open = false;
        assert!(MissingFontsPanel::for_editor(&state).is_none());
    }
}
