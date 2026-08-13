//! Font-family picker paint pass — split out of
//! `property_panel_typography.rs` to keep that module under the
//! 800-line ceiling. Re-exported from the parent as
//! `property_panel_typography::paint_font_picker`.

use super::{
    display_font_family, font_picker_layout, font_picker_layout_at, FontPickerEntry,
    FontPickerLayout, FontPickerRow, PAD_X,
};
use crate::theme::Theme;
use crate::widgets::button::paint_button_feedback_wash;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel_layout::VisibleSections;
use crate::widgets::text_metrics;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use jian_widgets::components::select::SelectState;

/// Paint the dropdown (call as a late overlay, after the sections).
#[allow(clippy::too_many_arguments)]
pub fn paint_font_picker(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    panel_rect: Rect,
    visible: VisibleSections,
    locale: op_editor_core::Locale,
    entries: &[FontPickerEntry],
    allow_import: bool,
    search: &str,
    state: &SelectState,
    import_hover: bool,
    active_family: &str,
    now_ms: u64,
) {
    if !state.open {
        return;
    }
    let Some(layout) = font_picker_layout(
        panel_rect,
        visible,
        entries,
        allow_import,
        state.scroll.offset,
    ) else {
        return;
    };
    paint_font_picker_layout(
        cx,
        theme,
        locale,
        entries,
        search,
        state,
        import_hover,
        active_family,
        now_ms,
        &layout,
    );
}

/// Paint the same picker against an arbitrary row trigger. Missing-font
/// surfaces use this without depending on PropertyPanel layout internals.
#[allow(clippy::too_many_arguments)]
pub fn paint_font_picker_at(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    trigger: Rect,
    popup_width: f32,
    bounds: Rect,
    locale: op_editor_core::Locale,
    entries: &[FontPickerEntry],
    allow_import: bool,
    allow_remove: bool,
    search: &str,
    state: &SelectState,
    import_hover: bool,
    active_family: &str,
    now_ms: u64,
) {
    if !state.open {
        return;
    }
    let layout = font_picker_layout_at(
        trigger,
        popup_width,
        bounds,
        entries,
        allow_import,
        allow_remove,
        state.scroll.offset,
    );
    paint_font_picker_layout(
        cx,
        theme,
        locale,
        entries,
        search,
        state,
        import_hover,
        active_family,
        now_ms,
        &layout,
    );
}

#[allow(clippy::too_many_arguments)]
fn paint_font_picker_layout(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    locale: op_editor_core::Locale,
    entries: &[FontPickerEntry],
    search: &str,
    state: &SelectState,
    import_hover: bool,
    active_family: &str,
    now_ms: u64,
    layout: &FontPickerLayout,
) {
    cx.backend.fill_round_rect(layout.popup, 8.0, theme.popover);
    cx.backend
        .stroke_round_rect(layout.popup, 8.0, theme.border, 1.0);

    // Search row — Search glyph + draft (or muted placeholder) +
    // steady caret, separated by a hairline (TS border-b).
    let s = layout.search;
    draw_icon(
        cx.backend,
        Icon::Search,
        Point2D::new(s.origin.x + 8.0, s.origin.y + (s.size.y - 12.0) / 2.0),
        12.0,
        theme.muted_foreground,
        1.5,
    );
    let text_x = s.origin.x + 26.0;
    let baseline = s.origin.y + s.size.y / 2.0 + 4.0;
    // Draft + placeholder + caret render through the unified jian TextInputView
    // (family-aware caret, no hand-rolled drift). The buffer is rebuilt from the
    // search String each frame; the open picker reads as focused.
    let mut search_input = jian_core::text_input::TextInputState::with_text(search.to_string());
    // Anchor the blink to this frame so the caret stays visible while the
    // picker is open (the buffer is rebuilt each frame).
    search_input.touch(now_ms);
    crate::widgets::property_panel_text_input::paint_text_input_view(
        cx,
        theme,
        &search_input,
        s,
        11.0,
        text_x - s.origin.x,
        baseline,
        now_ms,
        op_i18n::translate(locale, "text.font.search"),
        true,
    );
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(s.origin.x, s.origin.y + s.size.y - 1.0),
            size: Point2D::new(s.size.x, 1.0),
        },
        theme.border,
    );

    // Scrolling list — clipped to the viewport.
    cx.backend.save();
    cx.backend.clip_rect(layout.viewport);
    let active = display_font_family(active_family);
    for (row, rect) in &layout.rows {
        // Cull rows fully outside the viewport.
        if rect.origin.y + rect.size.y < layout.viewport.origin.y
            || rect.origin.y > layout.viewport.origin.y + layout.viewport.size.y
        {
            continue;
        }
        match row {
            FontPickerRow::GroupImported
            | FontPickerRow::GroupBundled
            | FontPickerRow::GroupSystem => {
                let label_str = match row {
                    FontPickerRow::GroupImported => {
                        op_i18n::translate(locale, "text.font.imported")
                    }
                    FontPickerRow::GroupBundled => op_i18n::translate(locale, "text.font.bundled"),
                    _ => op_i18n::translate(locale, "text.font.system"),
                };
                let label = TextLayout::single_run(
                    label_str,
                    "system-ui",
                    9.0,
                    (theme.muted_foreground).to_jian(),
                    Point2D::new(0.0, 0.0),
                );
                cx.backend.draw_text(
                    &label,
                    Point2D::new(rect.origin.x + 10.0, rect.origin.y + rect.size.y - 4.0),
                );
            }
            FontPickerRow::NoResults => {
                let label_str = op_i18n::translate(locale, "text.font.noResults");
                let label = TextLayout::single_run(
                    label_str,
                    "system-ui",
                    11.0,
                    (theme.muted_foreground).to_jian(),
                    Point2D::new(0.0, 0.0),
                );
                let w = text_metrics::measure_chrome(cx.backend, label_str, 11.0);
                cx.backend.draw_text(
                    &label,
                    Point2D::new(
                        rect.origin.x + (rect.size.x - w) / 2.0,
                        rect.origin.y + rect.size.y / 2.0 + 4.0,
                    ),
                );
            }
            FontPickerRow::RemoveEntry(_) => {
                // Small muted "x" centred in its square hit-rect.
                draw_icon(
                    cx.backend,
                    Icon::Close,
                    Point2D::new(
                        rect.origin.x + (rect.size.x - 12.0) / 2.0,
                        rect.origin.y + (rect.size.y - 12.0) / 2.0,
                    ),
                    12.0,
                    theme.muted_foreground,
                    1.5,
                );
            }
            FontPickerRow::ImportAction => {
                // Hover wash inset like the Entry arm so the row reads
                // identically to entry hover (painted BELOW the hairline
                // + icon + label).
                if import_hover {
                    let row_rect = Rect {
                        origin: Point2D::new(rect.origin.x + 2.0, rect.origin.y + 2.0),
                        size: Point2D::new(rect.size.x - 4.0, rect.size.y - 4.0),
                    };
                    paint_button_feedback_wash(cx.backend, theme, row_rect, 5.0, true, false);
                }
                // Top hairline separating the action from the list.
                cx.backend.fill_rect(
                    Rect {
                        origin: Point2D::new(rect.origin.x, rect.origin.y),
                        size: Point2D::new(rect.size.x, 1.0),
                    },
                    theme.border,
                );
                draw_icon(
                    cx.backend,
                    Icon::Plus,
                    Point2D::new(
                        rect.origin.x + PAD_X,
                        rect.origin.y + (rect.size.y - 12.0) / 2.0,
                    ),
                    12.0,
                    theme.foreground,
                    1.6,
                );
                let label = TextLayout::single_run(
                    op_i18n::translate(locale, "text.font.importAction"),
                    "system-ui",
                    11.0,
                    (theme.foreground).to_jian(),
                    Point2D::new(0.0, 0.0),
                );
                cx.backend.draw_text(
                    &label,
                    Point2D::new(
                        rect.origin.x + PAD_X + 18.0,
                        rect.origin.y + rect.size.y / 2.0 + 4.0,
                    ),
                );
            }
            FontPickerRow::Entry(i) => {
                let Some(entry) = entries.get(*i) else {
                    continue;
                };
                let is_active = entry.family.eq_ignore_ascii_case(active);
                let row_rect = Rect {
                    origin: Point2D::new(rect.origin.x + 2.0, rect.origin.y),
                    size: Point2D::new(rect.size.x - 4.0, rect.size.y),
                };
                if is_active {
                    cx.backend
                        .fill_round_rect(row_rect, 5.0, theme.row_selected_primary);
                } else if state.hover == Some(*i) || state.pressed == Some(*i) {
                    paint_button_feedback_wash(
                        cx.backend,
                        theme,
                        row_rect,
                        5.0,
                        state.hover == Some(*i),
                        state.pressed == Some(*i),
                    );
                }
                // Each row renders in its own family (TS style
                // fontFamily: font.family).
                let label = TextLayout::single_run(
                    &entry.family,
                    &entry.family,
                    11.0,
                    (if is_active {
                        theme.primary
                    } else {
                        theme.foreground
                    })
                    .to_jian(),
                    Point2D::new(0.0, 0.0),
                );
                cx.backend.draw_text(
                    &label,
                    Point2D::new(
                        rect.origin.x + 10.0,
                        rect.origin.y + rect.size.y / 2.0 + 4.0,
                    ),
                );
                if is_active {
                    draw_icon(
                        cx.backend,
                        Icon::Check,
                        Point2D::new(
                            rect.origin.x + rect.size.x - 20.0,
                            rect.origin.y + (rect.size.y - 12.0) / 2.0,
                        ),
                        12.0,
                        theme.primary,
                        1.6,
                    );
                }
            }
        }
    }
    cx.backend.restore();
}
