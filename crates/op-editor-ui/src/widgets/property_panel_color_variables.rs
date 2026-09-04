//! Colour-variable affordances for the property panel.
//!
//! Kept separate from the fill / stroke paint modules so the variable
//! picker can be shared without growing the already-large section
//! files. The picker is a floating overlay with its own capped,
//! scrolling list — it never grows the inspector's own content height.

use crate::theme::Theme;
use crate::util::ellipsize_to_width;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::ColorVariableOption;
use crate::widgets::property_panel_inputs::INPUT_RADIUS;
use crate::widgets::property_panel_layout::{
    COLOR_VARIABLE_MENU_PAD_Y, COLOR_VARIABLE_MENU_ROW_H, COLOR_VARIABLE_MENU_W,
};
use crate::widgets::property_panel_snapshot::color_from_hex;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};

/// Tallest the popup may grow before its list scrolls. A document can
/// carry dozens of `--color-*` variables; without this cap the list ran
/// past the bottom of the screen and buried the whole inspector.
pub const COLOR_VARIABLE_MENU_MAX_H: f32 = 320.0;
/// Gap between the `{}` trigger and the popup edge.
const MENU_GAP: f32 = 4.0;
const MENU_RADIUS: f32 = 8.0;
/// Horizontal inset of a row's highlight from the popup edge.
const ROW_INSET: f32 = 4.0;
const SWATCH_SIZE: f32 = 16.0;
const SWATCH_X: f32 = 8.0;
const SWATCH_RADIUS: f32 = 4.0;
/// Name column start, measured from the row highlight's left edge.
const NAME_X: f32 = 30.0;
/// Trailing inset of the right-aligned hex column.
const HEX_PAD_RIGHT: f32 = 10.0;
/// Minimum blank gutter kept between the name and the hex column.
const NAME_HEX_GAP: f32 = 8.0;
const ROW_FONT: f32 = 12.0;
const ROW_RADIUS: f32 = 6.0;
const MONO_FAMILY: &str = "ui-monospace";
const UI_FAMILY: &str = "system-ui";
const SCROLLBAR_W: f32 = 2.0;
const SCROLLBAR_INSET: f32 = 5.0;
const SCROLLBAR_MIN_LEN: f32 = 24.0;

pub fn paint_color_variable_button(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, active: bool) {
    if active {
        cx.backend
            .fill_round_rect(rect, INPUT_RADIUS, theme.row_selected_primary);
    }
    draw_icon(
        cx.backend,
        Icon::Braces,
        Point2D::new(rect.origin.x + 6.0, rect.origin.y + 7.0),
        16.0,
        if active {
            theme.primary
        } else {
            theme.muted_foreground
        },
        1.6,
    );
}

/// A clickable row of the picker. `Variable(i)` indexes the
/// `ColorVariableOption` slice the panel painted from, so the host's
/// `BindColorVariable { index }` semantics are unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorVariableRow {
    /// The leading "unbind" row — only present while a variable is bound.
    Unbind,
    Variable(usize),
}

/// Resolved popup geometry. Row rects already carry the list scroll, so
/// paint and hit-test must both cull against `viewport`.
#[derive(Debug, Clone)]
pub struct ColorVariablePickerLayout {
    /// Popup chrome (background + border).
    pub popup: Rect,
    /// Clip band the scrolling rows live in.
    pub viewport: Rect,
    /// Unclipped height of every row plus the vertical padding.
    pub content_height: f32,
    pub max_scroll: f32,
    /// Applied scroll, already clamped to `max_scroll`.
    pub scroll: f32,
    pub rows: Vec<(ColorVariableRow, Rect)>,
}

impl ColorVariablePickerLayout {
    pub(crate) fn scaled_about(mut self, origin: Point2D, scale: f32) -> Self {
        let map = |rect: Rect| Rect {
            origin: Point2D::new(
                origin.x + (rect.origin.x - origin.x) * scale,
                origin.y + (rect.origin.y - origin.y) * scale,
            ),
            size: Point2D::new(rect.size.x * scale, rect.size.y * scale),
        };
        self.popup = map(self.popup);
        self.viewport = map(self.viewport);
        self.content_height *= scale;
        self.max_scroll *= scale;
        self.scroll *= scale;
        for (_, rect) in &mut self.rows {
            *rect = map(*rect);
        }
        self
    }

    /// Row under `point`, or `None` when the point is outside the list
    /// or over a row scrolled out of view.
    pub fn row_at(&self, point: Point2D) -> Option<ColorVariableRow> {
        self.row_slot_at(point).map(|slot| self.rows[slot].0)
    }

    /// Index into [`Self::rows`] of the row under `point` — the key the
    /// hosts retain as hover state, so paint and hit-test agree on which
    /// row is lit without re-deriving the leading unbind offset.
    pub fn row_slot_at(&self, point: Point2D) -> Option<usize> {
        if !self.viewport.contains(point) {
            return None;
        }
        self.rows.iter().position(|(_, rect)| rect.contains(point))
    }

    /// Whether `point` is anywhere on the popup — the host swallows
    /// such presses instead of dismissing the picker.
    pub fn contains(&self, point: Point2D) -> bool {
        self.popup.contains(point)
    }
}

/// Lay the popup out against its `{}` trigger. `anchor` is in the
/// inspector's scrolled space (so the popup tracks the row it belongs
/// to); `bounds` is the unscrolled visible rail band the popup is
/// clamped into. Returns `None` when there is nothing to list.
pub fn color_variable_picker_layout(
    anchor: Rect,
    bounds: Rect,
    count: usize,
    bound: bool,
    scroll: f32,
) -> Option<ColorVariablePickerLayout> {
    let row_count = count + usize::from(bound);
    if row_count == 0 {
        return None;
    }
    let content_height =
        COLOR_VARIABLE_MENU_PAD_Y * 2.0 + row_count as f32 * COLOR_VARIABLE_MENU_ROW_H;
    let popup_h = content_height
        .min(COLOR_VARIABLE_MENU_MAX_H)
        .min(bounds.size.y.max(0.0));
    let max_scroll = (content_height - popup_h).max(0.0);
    let scroll = scroll.clamp(0.0, max_scroll);

    let popup_w = COLOR_VARIABLE_MENU_W.min(bounds.size.x.max(1.0));
    let min_x = bounds.origin.x;
    let max_x = (bounds.origin.x + bounds.size.x - popup_w).max(min_x);
    // Right-align the popup with the trigger, the way the fill-type and
    // font dropdowns do, then clamp so it can never leave the rail.
    let popup_x = (anchor.origin.x + anchor.size.x - popup_w).clamp(min_x, max_x);

    let bounds_bottom = bounds.origin.y + bounds.size.y;
    let below = anchor.origin.y + anchor.size.y + MENU_GAP;
    let above = anchor.origin.y - MENU_GAP - popup_h;
    let preferred_y = if below + popup_h <= bounds_bottom || above < bounds.origin.y {
        below
    } else {
        above
    };
    let max_y = (bounds_bottom - popup_h).max(bounds.origin.y);
    let popup_y = preferred_y.clamp(bounds.origin.y, max_y);

    let popup = Rect {
        origin: Point2D::new(popup_x, popup_y),
        size: Point2D::new(popup_w, popup_h),
    };
    let list_top = popup_y + COLOR_VARIABLE_MENU_PAD_Y - scroll;
    let row_rect = |slot: usize| Rect {
        origin: Point2D::new(popup_x, list_top + slot as f32 * COLOR_VARIABLE_MENU_ROW_H),
        size: Point2D::new(popup_w, COLOR_VARIABLE_MENU_ROW_H),
    };
    let mut rows = Vec::with_capacity(row_count);
    let leading = usize::from(bound);
    if bound {
        rows.push((ColorVariableRow::Unbind, row_rect(0)));
    }
    for index in 0..count {
        rows.push((ColorVariableRow::Variable(index), row_rect(leading + index)));
    }
    Some(ColorVariablePickerLayout {
        popup,
        viewport: popup,
        content_height,
        max_scroll,
        scroll,
        rows,
    })
}

/// Paint the popup for an already-resolved layout. `current` is the
/// bound variable name for the picker's target, if any; `hover` is the
/// row slot under the cursor (see [`ColorVariablePickerLayout::row_slot_at`]).
pub fn paint_color_variable_picker(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    layout: &ColorVariablePickerLayout,
    variables: &[ColorVariableOption],
    current: Option<&str>,
    hover: Option<usize>,
    locale: op_editor_core::Locale,
) {
    cx.backend
        .fill_round_rect(layout.popup, MENU_RADIUS, theme.popover);
    cx.backend
        .stroke_round_rect(layout.popup, MENU_RADIUS, theme.border, 1.0);

    cx.backend.save();
    cx.backend.clip_rect(layout.viewport);
    let viewport_bottom = layout.viewport.origin.y + layout.viewport.size.y;
    for (slot, (row, rect)) in layout.rows.iter().enumerate() {
        if rect.origin.y + rect.size.y < layout.viewport.origin.y || rect.origin.y > viewport_bottom
        {
            continue;
        }
        let hovered = hover == Some(slot);
        match row {
            ColorVariableRow::Unbind => paint_unbind_row(cx, theme, locale, hovered, *rect),
            ColorVariableRow::Variable(index) => {
                let Some(variable) = variables.get(*index) else {
                    continue;
                };
                let active = current == Some(variable.name.as_str());
                paint_variable_row(cx, theme, variable, active, hovered, *rect);
            }
        }
    }
    cx.backend.restore();
    paint_scrollbar(cx, theme, layout);
}

fn paint_scrollbar(cx: &mut PaintCx<'_>, theme: &Theme, layout: &ColorVariablePickerLayout) {
    let track = (layout.viewport.size.y - COLOR_VARIABLE_MENU_PAD_Y * 2.0).max(0.0);
    let state = jian_core::scroll::ScrollState {
        offset: layout.scroll,
    };
    let Some(thumb) = state.thumb(
        track,
        layout.content_height,
        layout.viewport.size.y,
        SCROLLBAR_MIN_LEN,
    ) else {
        return;
    };
    let rect = Rect {
        origin: Point2D::new(
            layout.viewport.origin.x + layout.viewport.size.x - SCROLLBAR_INSET,
            layout.viewport.origin.y + COLOR_VARIABLE_MENU_PAD_Y + thumb.offset,
        ),
        size: Point2D::new(SCROLLBAR_W, thumb.len),
    };
    cx.backend.fill_round_rect(
        rect,
        SCROLLBAR_W / 2.0,
        Color {
            a: 0.55,
            ..theme.muted_foreground
        },
    );
}

/// The highlight / content box of a row, inset from the popup edge.
fn row_inner(row: Rect) -> Rect {
    Rect {
        origin: Point2D::new(row.origin.x + ROW_INSET, row.origin.y),
        size: Point2D::new((row.size.x - ROW_INSET * 2.0).max(0.0), row.size.y),
    }
}

fn paint_unbind_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    locale: op_editor_core::Locale,
    hovered: bool,
    row: Rect,
) {
    let inner = row_inner(row);
    if hovered {
        cx.backend.fill_round_rect(inner, ROW_RADIUS, theme.muted);
    }
    draw_icon(
        cx.backend,
        Icon::Close,
        Point2D::new(
            inner.origin.x + SWATCH_X,
            inner.origin.y + (inner.size.y - SWATCH_SIZE) / 2.0,
        ),
        SWATCH_SIZE,
        theme.destructive,
        1.6,
    );
    let text = op_i18n::translate(locale, "variablePicker.unbind");
    let baseline = jian_widgets::centered_text_baseline_y(inner, ROW_FONT);
    let max_w = (inner.size.x - NAME_X - HEX_PAD_RIGHT).max(0.0);
    let shown = ellipsize_to_width(text, max_w, |probe| {
        cx.backend.measure_text_family(probe, ROW_FONT, UI_FAMILY)
    });
    let layout = TextLayout::single_run(
        &shown,
        UI_FAMILY,
        ROW_FONT,
        (theme.destructive).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&layout, Point2D::new(inner.origin.x + NAME_X, baseline));
}

fn paint_variable_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    variable: &ColorVariableOption,
    active: bool,
    hovered: bool,
    row: Rect,
) {
    let inner = row_inner(row);
    // The bound row's tint wins over the hover wash, the way the export
    // and effect-add popups resolve the same overlap.
    if active {
        cx.backend
            .fill_round_rect(inner, ROW_RADIUS, theme.row_selected_primary);
    } else if hovered {
        cx.backend.fill_round_rect(inner, ROW_RADIUS, theme.muted);
    }
    let swatch = Rect {
        origin: Point2D::new(
            inner.origin.x + SWATCH_X,
            inner.origin.y + (inner.size.y - SWATCH_SIZE) / 2.0,
        ),
        size: Point2D::new(SWATCH_SIZE, SWATCH_SIZE),
    };
    let color = variable
        .resolved_hex
        .as_deref()
        .and_then(color_from_hex)
        .unwrap_or(Color::BLACK);
    cx.backend.fill_round_rect(swatch, SWATCH_RADIUS, color);
    cx.backend
        .stroke_round_rect(swatch, SWATCH_RADIUS, theme.border, 1.0);

    let baseline = jian_widgets::centered_text_baseline_y(inner, ROW_FONT);
    // The hex column is a fixed right-aligned slot; the name gets
    // whatever is left and is ellipsized into it, so a long variable
    // name can never run under the value.
    let hex = variable.resolved_hex.as_deref();
    let hex_w = hex
        .map(|value| cx.backend.measure_text_family(value, ROW_FONT, MONO_FAMILY))
        .unwrap_or(0.0);
    let hex_rect = row_hex_rect(row, hex_w);
    let name_max = row_name_budget(row, hex_w);
    let name_rect = row_name_rect(row, name_max);
    let name = format!("--{}", variable.name);
    let shown = ellipsize_to_width(&name, name_max, |probe| {
        cx.backend.measure_text_family(probe, ROW_FONT, MONO_FAMILY)
    });
    let name_layout = TextLayout::single_run(
        &shown,
        MONO_FAMILY,
        ROW_FONT,
        (if active {
            theme.primary
        } else {
            theme.foreground
        })
        .to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&name_layout, Point2D::new(name_rect.origin.x, baseline));

    if let Some(value) = hex {
        let hex_layout = TextLayout::single_run(
            value,
            MONO_FAMILY,
            ROW_FONT,
            (theme.muted_foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&hex_layout, Point2D::new(hex_rect.origin.x, baseline));
    }
}

/// Name column of a row, in the same coordinates the paint pass uses.
/// Tests assert it never intersects [`row_hex_rect`].
pub fn row_name_rect(row: Rect, name_w: f32) -> Rect {
    let inner = row_inner(row);
    Rect {
        origin: Point2D::new(inner.origin.x + NAME_X, inner.origin.y),
        size: Point2D::new(name_w, inner.size.y),
    }
}

/// Left edge of the right-aligned hex column. Never moves left of the
/// name column's start, so a pathologically wide value can only squeeze
/// the name to nothing — it can never reopen the overlap.
fn row_hex_x(inner: Rect, hex_w: f32) -> f32 {
    (inner.origin.x + inner.size.x - HEX_PAD_RIGHT - hex_w).max(inner.origin.x + NAME_X)
}

/// Right-aligned hex column of a row, given the measured value width.
pub fn row_hex_rect(row: Rect, hex_w: f32) -> Rect {
    let inner = row_inner(row);
    Rect {
        origin: Point2D::new(row_hex_x(inner, hex_w), inner.origin.y),
        size: Point2D::new(hex_w, inner.size.y),
    }
}

/// Width the name column may occupy before it would touch the hex
/// column — the budget the paint pass ellipsizes into.
pub fn row_name_budget(row: Rect, hex_w: f32) -> f32 {
    let inner = row_inner(row);
    (row_hex_x(inner, hex_w) - NAME_HEX_GAP - (inner.origin.x + NAME_X)).max(0.0)
}
