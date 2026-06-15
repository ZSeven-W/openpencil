//! Widget section (Phase D3) — per-kind editable props for a form
//! widget node (TextInput / TextArea / NumberInput / Select /
//! RadioGroup / Switch / Checkbox / Slider / Progress / Tabs).
//!
//! Visibility is gated on `VisibleSections.widget` (`Some(kind)`),
//! which the panel derives from `snapshot.widget`. The same kind is
//! threaded through paint AND both layout walkers so the rows, input
//! hit-rects, and the `checked` toggle action-rect line up — and so
//! sections after Widget shift correctly when it is hidden.
//!
//! Scope: placeholder / value / label / checked / min / max / step
//! are editable. The `options` (Select / RadioGroup) and `tabs` (Tabs)
//! lists are surfaced read-only as a count; the add/remove row editor
//! is a deferred follow-up. The `states` overrides panel is explicitly
//! out of scope (JSON-only).

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::{NodeSnapshot, PropertyPanelAction, WidgetKind};
use crate::widgets::property_panel_inputs::{
    paint_input_with_prefix_focused_state, paint_section_divider, paint_section_label,
    INPUT_HEIGHT, INPUT_RADIUS, PAD_X, SECTION_GAP, SECTION_HEADER_HEIGHT,
};
use crate::widgets::property_panel_sections::EditContext;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::PropertyFocus;

/// One editable row's vertical advance (input + gap).
const ROW_ADVANCE: f32 = INPUT_HEIGHT + 6.0;

/// The ordered list of text-input rows a widget kind exposes, each
/// paired with its `PropertyFocus`. Drives paint + the input-rect
/// walker so they can never drift. (placeholder → value → label →
/// leading icon → trailing icon → bind-key).
fn text_rows(kind: WidgetKind) -> Vec<(PropertyFocus, &'static str)> {
    let mut rows = Vec::new();
    if kind.has_placeholder() {
        rows.push((PropertyFocus::WidgetPlaceholder, "Placeholder"));
    }
    if kind.has_text_value() {
        rows.push((PropertyFocus::WidgetValue, "Value"));
    }
    if kind.has_label() {
        rows.push((PropertyFocus::WidgetLabel, "Label"));
    }
    if kind.has_icons() {
        rows.push((PropertyFocus::WidgetLeadingIcon, "Leading icon"));
        rows.push((PropertyFocus::WidgetTrailingIcon, "Trailing icon"));
    }
    if kind.has_bind_value() {
        rows.push((PropertyFocus::WidgetBindKey, "Bind value"));
    }
    rows
}

/// Whether the kind paints a read-only `options` / `tabs` count row.
fn list_row_label(kind: WidgetKind) -> Option<&'static str> {
    match kind {
        WidgetKind::Select | WidgetKind::RadioGroup => Some("Options"),
        WidgetKind::Tabs => Some("Tabs"),
        _ => None,
    }
}

/// Total height the Widget section consumes for `kind`: header +
/// text rows + (checked row?) + (range row?) + (list row?) + divider
/// gap. Paint, both walkers, and the content-height clamp all read
/// this so the y-math stays in lockstep.
pub fn widget_section_height(kind: WidgetKind) -> f32 {
    let mut h = SECTION_HEADER_HEIGHT;
    h += text_rows(kind).len() as f32 * ROW_ADVANCE;
    if kind.has_checked() {
        h += ROW_ADVANCE;
    }
    if kind.has_range() {
        // min / max / step share one row.
        h += ROW_ADVANCE;
    }
    if list_row_label(kind).is_some() {
        h += ROW_ADVANCE;
    }
    h += 12.0;
    h
}

/// Push every editable text/numeric input hit-rect for the Widget
/// section, advancing `y` past the whole section. Mirrors
/// `paint_widget_section`'s row walk exactly.
pub fn push_widget_input_rects(
    rects: &mut Vec<(PropertyFocus, Rect)>,
    kind: WidgetKind,
    x0: f32,
    y: f32,
    usable_w: f32,
) {
    let mut y = y + SECTION_HEADER_HEIGHT;
    for (focus, _) in text_rows(kind) {
        rects.push((
            focus,
            Rect {
                origin: Point2D::new(x0 + PAD_X, y),
                size: Point2D::new(usable_w, INPUT_HEIGHT),
            },
        ));
        y += ROW_ADVANCE;
    }
    if kind.has_checked() {
        // The checked toggle is an action rect, not an input — it is
        // emitted by `push_widget_action_rects`. Still advance past
        // its row so range/list inputs below line up.
        y += ROW_ADVANCE;
    }
    if kind.has_range() {
        let col_w = (usable_w - 12.0) / 3.0;
        for (i, focus) in [
            PropertyFocus::WidgetMin,
            PropertyFocus::WidgetMax,
            PropertyFocus::WidgetStep,
        ]
        .into_iter()
        .enumerate()
        {
            rects.push((
                focus,
                Rect {
                    origin: Point2D::new(x0 + PAD_X + i as f32 * (col_w + 6.0), y),
                    size: Point2D::new(col_w, INPUT_HEIGHT),
                },
            ));
        }
    }
    // List (options / tabs) row is read-only — no input rect.
}

/// Push the Widget section's action rects (the `checked` toggle only,
/// today). `y` is the section's top; the walk matches paint. `checked`
/// is the current literal value — the emitted toggle action carries
/// the NEXT value (`!checked`).
pub fn push_widget_action_rects(
    out: &mut Vec<(PropertyPanelAction, Rect)>,
    kind: WidgetKind,
    checked: bool,
    x0: f32,
    y: f32,
    usable_w: f32,
) {
    let mut y = y + SECTION_HEADER_HEIGHT;
    y += text_rows(kind).len() as f32 * ROW_ADVANCE;
    if kind.has_checked() {
        // Whole row toggles the value (box + label).
        out.push((
            PropertyPanelAction::ToggleWidgetChecked(!checked),
            Rect {
                origin: Point2D::new(x0 + PAD_X, y),
                size: Point2D::new(usable_w, INPUT_HEIGHT),
            },
        ));
    }
}

/// Paint the Widget section. Returns the y below the section divider.
#[allow(clippy::too_many_arguments)]
pub fn paint_widget_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    edit: &EditContext<'_>,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let Some(summary) = snapshot.widget.as_ref() else {
        return y;
    };
    let kind = summary.kind;
    let usable_w = width - PAD_X * 2.0;
    let mut y = paint_section_label(
        cx,
        theme,
        translate(locale, "widget.title", "Widget"),
        x,
        y,
        width,
    );
    // Text-input rows (placeholder / value / label).
    for (focus, prefix) in text_rows(kind) {
        let rect = Rect {
            origin: Point2D::new(x + PAD_X, y),
            size: Point2D::new(usable_w, INPUT_HEIGHT),
        };
        let fallback = match focus {
            PropertyFocus::WidgetPlaceholder => summary.placeholder.as_str(),
            PropertyFocus::WidgetValue => summary.value.as_str(),
            PropertyFocus::WidgetLabel => summary.label.as_str(),
            PropertyFocus::WidgetLeadingIcon => summary.leading_icon.as_str(),
            PropertyFocus::WidgetTrailingIcon => summary.trailing_icon.as_str(),
            PropertyFocus::WidgetBindKey => summary.bind_key.as_str(),
            _ => "",
        };
        let value = edit.value_for(focus, fallback);
        paint_input_with_prefix_focused_state(
            cx,
            theme,
            rect,
            prefix,
            value,
            edit.focus == Some(focus),
            edit.caret_at(focus),
            edit.select_all_at(focus),
            edit.input_at(focus),
            edit.now_ms,
        );
        y += ROW_ADVANCE;
    }
    // Checked toggle row.
    if kind.has_checked() {
        paint_check_row(
            cx,
            theme,
            x + PAD_X,
            y,
            translate(locale, "widget.checked", "Checked"),
            summary.checked,
        );
        y += ROW_ADVANCE;
    }
    // Min / max / step numeric row (Slider / NumberInput).
    if kind.has_range() {
        let col_w = (usable_w - 12.0) / 3.0;
        let cols = [
            (PropertyFocus::WidgetMin, "Min", summary.min.as_str()),
            (PropertyFocus::WidgetMax, "Max", summary.max.as_str()),
            (PropertyFocus::WidgetStep, "Step", summary.step.as_str()),
        ];
        for (i, (focus, prefix, fallback)) in cols.into_iter().enumerate() {
            let rect = Rect {
                origin: Point2D::new(x + PAD_X + i as f32 * (col_w + 6.0), y),
                size: Point2D::new(col_w, INPUT_HEIGHT),
            };
            let value = edit.value_for(focus, fallback);
            paint_input_with_prefix_focused_state(
                cx,
                theme,
                rect,
                prefix,
                value,
                edit.focus == Some(focus),
                edit.caret_at(focus),
                edit.select_all_at(focus),
                edit.input_at(focus),
                edit.now_ms,
            );
        }
        y += ROW_ADVANCE;
    }
    // Read-only list count (options / tabs) — the row editor is a
    // deferred follow-up; the count tells the user what is authored.
    if let Some(label) = list_row_label(kind) {
        let count = match kind {
            WidgetKind::Tabs => summary.tab_count,
            _ => summary.option_count,
        };
        paint_list_count_row(cx, theme, x + PAD_X, y, usable_w, label, count);
        y += ROW_ADVANCE;
    }
    y += 12.0;
    paint_section_divider(cx, theme, x, y, width);
    // Section gap is added by the caller (mirrors the text-section
    // convention: `widget_section_height` excludes the gap; callers
    // `+= SECTION_GAP` after this section's height/return).
    y + SECTION_GAP
}

/// Paint a full-width checkbox row (box + label) for the `checked`
/// toggle. Mirrors the size-section check rows' visual style.
fn paint_check_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    x: f32,
    y: f32,
    label: &str,
    checked: bool,
) {
    let box_rect = Rect {
        origin: Point2D::new(x, y + 7.0),
        size: Point2D::new(16.0, 16.0),
    };
    if checked {
        cx.backend.fill_round_rect(box_rect, 4.0, theme.primary);
        draw_icon(
            cx.backend,
            Icon::Check,
            Point2D::new(box_rect.origin.x + 1.0, box_rect.origin.y + 1.0),
            14.0,
            theme.primary_foreground,
            1.8,
        );
    } else {
        cx.backend
            .stroke_round_rect(box_rect, 4.0, theme.border, 1.0);
    }
    let lbl = TextLayout::single_run(
        label,
        "system-ui",
        12.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&lbl, Point2D::new(x + 22.0, y + INPUT_HEIGHT / 2.0 + 4.0));
}

/// Paint the read-only options / tabs count pill (`Options    3`).
fn paint_list_count_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    x: f32,
    y: f32,
    usable_w: f32,
    label: &str,
    count: usize,
) {
    let rect = Rect {
        origin: Point2D::new(x, y),
        size: Point2D::new(usable_w, INPUT_HEIGHT),
    };
    cx.backend.fill_round_rect(rect, INPUT_RADIUS, theme.muted);
    let baseline_y = rect.origin.y + rect.size.y / 2.0 + 4.0;
    let label_layout = TextLayout::single_run(
        label,
        "system-ui",
        12.0,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label_layout,
        Point2D::new(rect.origin.x + 10.0, baseline_y),
    );
    let count_text = count.to_string();
    let count_layout = TextLayout::single_run(
        &count_text,
        "system-ui",
        12.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    let count_w = cx.backend.measure_text(&count_text, 12.0);
    cx.backend.draw_text(
        &count_layout,
        Point2D::new(rect.origin.x + rect.size.x - 12.0 - count_w, baseline_y),
    );
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
