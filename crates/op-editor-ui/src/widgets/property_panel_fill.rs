//! Fill-section paint code split out of
//! `property_panel_sections.rs` to honour the 800-line ceiling.
//! Contains: `fill_type_label`, `paint_fill_type_picker`,
//! `paint_fill_section`, the multi-fill loop (`paint_one_fill`) and the
//! per-fill height helpers. The per-type body paints live in the
//! sibling `property_panel_fill_body` module.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::NodeSnapshot;
// Per-type body paints live in the sibling `property_panel_fill_body`
// module (split off for the 800-line cap). `stop_hex_rgb_only` is
// re-exported so existing `property_panel_fill::stop_hex_rgb_only`
// paths keep resolving.
use crate::widgets::property_panel_fill_body::{paint_fill_gradient_body, paint_fill_solid_body};
pub use crate::widgets::property_panel_fill_body::stop_hex_rgb_only;
use crate::widgets::property_panel_fill_image_body::paint_fill_image_body;
pub use crate::widgets::property_panel_fill_picker::{
    fill_type_at, fill_type_picker_hit, fill_type_picker_rect,
};
use crate::widgets::property_panel_fill_picker::{popup_anchor, tokens_from_theme, FILL_TYPES};
use crate::widgets::property_panel_inputs::{
    paint_section_divider, paint_section_label_with_add, INPUT_HEIGHT, INPUT_RADIUS, PAD_X,
    SECTION_GAP,
};
use crate::widgets::property_panel_layout::{fill_body_height_with_stops, VisibleSections};
use crate::widgets::property_panel_sections::{EditContext, PropertyLabels};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
pub use jian_widgets::components::select::SelectHit;
use jian_widgets::components::select::{Select, SelectItem, SelectState};
use op_editor_core::PropertyFocus;

/// Display label for a fill-type variant (Solid / Gradient /
/// Image), localised against `locale` via the `fill.*` keys.
pub fn fill_type_label(
    locale: op_editor_core::Locale,
    t: op_editor_core::FillType,
) -> &'static str {
    use op_editor_core::FillType;
    let key = match t {
        FillType::Solid => "fill.solid",
        FillType::LinearGradient => "fill.linear",
        FillType::RadialGradient => "fill.radial",
        FillType::Image => "fill.image",
    };
    op_i18n::translate(locale, key)
}

/// Paint the fill-type picker overlay (4 rows). Called by
/// `PropertyPanel::paint` AFTER all other sections have painted
/// so the dropdown overlays them.
#[allow(clippy::too_many_arguments)]
pub fn paint_fill_type_picker(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    panel_rect: Rect,
    visible: VisibleSections,
    state: &SelectState,
    active: op_editor_core::FillType,
    row_offset_y: f32,
    locale: op_editor_core::Locale,
) {
    let picker_rect = fill_type_picker_rect(panel_rect, visible, row_offset_y);
    let items: Vec<SelectItem<'static>> = FILL_TYPES
        .iter()
        .map(|t| SelectItem {
            label: fill_type_label(locale, *t),
            selected: *t == active,
            disabled: false,
        })
        .collect();
    let select = Select {
        state,
        items: &items,
    };
    select.paint(
        cx.backend,
        popup_anchor(picker_rect),
        picker_rect,
        &tokens_from_theme(theme),
    );
}
// ── Fill section ──────────────────────────────────────────────────

/// Total height the body that follows fill `index`'s head row
/// consumes. The primary fill (index 0) paints its full editable body
/// (solid hex / gradient stops / image). A non-primary gradient /
/// image fill renders head-row only (the gradient-stop / image-fill
/// editors key off the primary fill), so it contributes no body. A
/// Solid fill at any index always paints its hex row.
pub fn fill_row_body_height(
    fill_type: op_editor_core::FillType,
    is_primary: bool,
    primary_stop_count: usize,
) -> f32 {
    use op_editor_core::FillType;
    match fill_type {
        FillType::Solid => INPUT_HEIGHT + 6.0,
        FillType::LinearGradient | FillType::RadialGradient | FillType::Image => {
            if is_primary {
                fill_body_height_with_stops(fill_type, primary_stop_count)
            } else {
                0.0
            }
        }
    }
}

/// Total height (px) one fill row consumes — its head row plus its
/// body. Mirrors the y-walk in `paint_one_fill`.
pub fn fill_row_height(
    fill_type: op_editor_core::FillType,
    is_primary: bool,
    primary_stop_count: usize,
) -> f32 {
    INPUT_HEIGHT + 6.0 + fill_row_body_height(fill_type, is_primary, primary_stop_count)
}

/// Vertical offset (px) of fill `index`'s head row relative to the
/// first fill's head row — the sum of every prior fill row's height.
/// `0.0` for `index == 0`. Used to anchor a fill row's type-picker
/// overlay below its own dropdown.
pub fn fill_row_offset(
    fills: &[crate::widgets::property_panel_snapshot::FillSummary],
    index: usize,
    primary_stop_count: usize,
) -> f32 {
    fills
        .iter()
        .take(index)
        .enumerate()
        .map(|(i, f)| fill_row_height(f.fill_type, i == 0, primary_stop_count))
        .sum()
}

/// Paint-context + geometry args threaded through; a struct adds no gain.
#[allow(clippy::too_many_arguments)]
pub fn paint_fill_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    edit: &EditContext<'_>,
    labels: &PropertyLabels,
    _fill_type: op_editor_core::FillType,
    _fill_picker_open: bool,
    _fill_variable_ref: Option<&str>,
    show_variable_button: bool,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    // Header row + "+" (appends a fill). One editable row per fill.
    let mut y = paint_section_label_with_add(cx, theme, labels.fill, x, y, width);
    let primary_stop_count = snapshot.gradient_stops.len();
    for (index, fill) in snapshot.fills.iter().enumerate() {
        y = paint_one_fill(
            cx,
            theme,
            snapshot,
            edit,
            index,
            fill,
            index == 0,
            // The colour-variable button only shows on the primary fill
            // (the variable subsystem keys off `fills[0]`).
            if index == 0 {
                fill.variable_ref.as_deref()
            } else {
                None
            },
            index == 0 && show_variable_button,
            locale,
            x,
            y,
            width,
            primary_stop_count,
        );
    }
    y += 12.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

/// Paint one fill row: head row (swatch + type dropdown + opacity% +
/// ×) followed by its body (solid hex / gradient stops / image).
/// Returns the y just past the body. Geometry mirrors
/// `fill_row_height` / the layout walkers so paint + hit-test agree.
#[allow(clippy::too_many_arguments)]
fn paint_one_fill(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    edit: &EditContext<'_>,
    fill_index: usize,
    fill: &crate::widgets::property_panel_snapshot::FillSummary,
    is_primary: bool,
    fill_variable_ref: Option<&str>,
    show_variable_button: bool,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
    primary_stop_count: usize,
) -> f32 {
    let mut y = y;
    let fill_type = fill.fill_type;
    let usable_w = width - PAD_X * 2.0;
    let fill_color = fill.color;
    let swatch_rect = Rect {
        origin: Point2D::new(x + PAD_X, y + 2.0),
        size: Point2D::new(22.0, 22.0),
    };
    // Swatch icon depends on the fill type so the head row reads
    // as a small preview of what's rendered below.
    use op_editor_core::FillType;
    match fill_type {
        FillType::Solid => {
            cx.backend.fill_round_rect(swatch_rect, 4.0, fill_color);
            cx.backend
                .stroke_round_rect(swatch_rect, 4.0, theme.border, 1.0);
        }
        FillType::LinearGradient => {
            // Visual stand-in for a horizontal black→white gradient
            // by stacking two halves (skia gradients not yet on
            // RenderBackend). Good enough as a thumbnail.
            let half_w = swatch_rect.size.x / 2.0;
            cx.backend.fill_round_rect(
                Rect {
                    origin: swatch_rect.origin,
                    size: Point2D::new(half_w, swatch_rect.size.y),
                },
                4.0,
                Color::BLACK,
            );
            cx.backend.fill_round_rect(
                Rect {
                    origin: Point2D::new(swatch_rect.origin.x + half_w, swatch_rect.origin.y),
                    size: Point2D::new(swatch_rect.size.x - half_w, swatch_rect.size.y),
                },
                4.0,
                Color::WHITE,
            );
            cx.backend
                .stroke_round_rect(swatch_rect, 4.0, theme.border, 1.0);
        }
        FillType::RadialGradient => {
            // Outer black ring + inner white oval — reads as a
            // radial preview.
            cx.backend.fill_round_rect(swatch_rect, 4.0, Color::BLACK);
            let inset = 4.0;
            cx.backend.fill_oval(
                Rect {
                    origin: Point2D::new(
                        swatch_rect.origin.x + inset,
                        swatch_rect.origin.y + inset,
                    ),
                    size: Point2D::new(
                        swatch_rect.size.x - inset * 2.0,
                        swatch_rect.size.y - inset * 2.0,
                    ),
                },
                Color::WHITE,
            );
            cx.backend
                .stroke_round_rect(swatch_rect, 4.0, theme.border, 1.0);
        }
        FillType::Image => {
            cx.backend.fill_round_rect(swatch_rect, 4.0, theme.muted);
            cx.backend
                .stroke_round_rect(swatch_rect, 4.0, theme.border, 1.0);
            draw_icon(
                cx.backend,
                Icon::ImagePlus,
                Point2D::new(swatch_rect.origin.x + 3.0, swatch_rect.origin.y + 3.0),
                16.0,
                theme.muted_foreground,
                1.4,
            );
        }
    }
    let dropdown_rect = Rect {
        origin: Point2D::new(swatch_rect.origin.x + swatch_rect.size.x + 6.0, y),
        size: Point2D::new(usable_w - 22.0 - 6.0 - 50.0 - 22.0 - 12.0, INPUT_HEIGHT),
    };
    jian_widgets::components::select_trigger::SelectTrigger {
        icon_paths: None,
        label: fill_type_label(locale, fill_type),
        placeholder: "",
        hovered: false,
        pressed: false,
        enabled: true,
        font_size: 12.0,
        bordered: true,
    }
    .paint(
        cx.backend,
        dropdown_rect,
        &crate::widgets::button::tokens_from_theme(theme),
    );
    let opacity_focus = PropertyFocus::FillOpacity(fill_index);
    let pct_rect = Rect {
        origin: Point2D::new(dropdown_rect.origin.x + dropdown_rect.size.x + 6.0, y),
        size: Point2D::new(50.0, INPUT_HEIGHT),
    };
    cx.backend
        .fill_round_rect(pct_rect, INPUT_RADIUS, theme.muted);
    let opacity_focused = edit.focus == Some(opacity_focus);
    if opacity_focused {
        cx.backend
            .stroke_round_rect(pct_rect, INPUT_RADIUS, theme.primary, 1.5);
    }
    let opacity_owned = ((fill.opacity * 100.0).round() as i32).to_string();
    let pct_text = edit.value_for(opacity_focus, &opacity_owned);
    let pct = TextLayout::single_run(
        pct_text,
        "system-ui",
        12.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    let pct_x = pct_rect.origin.x + 10.0;
    if !edit.paint_input_view_at(
        cx,
        theme,
        opacity_focus,
        Rect {
            origin: Point2D::new(pct_x, pct_rect.origin.y),
            size: Point2D::new(
                (pct_rect.origin.x + pct_rect.size.x - 18.0 - pct_x).max(0.0),
                pct_rect.size.y,
            ),
        },
        12.0,
        0.0,
        pct_rect.origin.y + 19.0,
    ) {
        edit.paint_selection_at(
            cx,
            theme,
            opacity_focus,
            pct_text,
            pct_x,
            pct_rect.origin.y + 19.0,
            12.0,
            pct_rect.origin.x + pct_rect.size.x - 8.0,
        );
        cx.backend
            .draw_text(&pct, Point2D::new(pct_x, pct_rect.origin.y + 19.0));
        if let Some(pos) = edit.caret_at(opacity_focus) {
            let w = cx
                .backend
                .measure_text(&pct_text[..pos.min(pct_text.len())], 12.0);
            cx.backend.fill_rect(
                Rect {
                    origin: Point2D::new(pct_x + w, pct_rect.origin.y + 6.0),
                    size: Point2D::new(1.5, pct_rect.size.y - 12.0),
                },
                theme.foreground,
            );
        }
    }
    let pct_unit = TextLayout::single_run(
        "%",
        "system-ui",
        12.0,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &pct_unit,
        Point2D::new(
            pct_rect.origin.x + pct_rect.size.x - 14.0,
            pct_rect.origin.y + 19.0,
        ),
    );
    draw_icon(
        cx.backend,
        Icon::Close,
        Point2D::new(
            pct_rect.origin.x + pct_rect.size.x + 8.0,
            y + (INPUT_HEIGHT - 14.0) / 2.0,
        ),
        14.0,
        theme.muted_foreground,
        1.4,
    );
    y += INPUT_HEIGHT + 6.0;
    match fill_type {
        FillType::Solid => {
            paint_fill_solid_body(
                cx,
                theme,
                edit,
                fill_index,
                fill_color,
                fill_variable_ref,
                show_variable_button,
                x,
                y,
                width,
            );
        }
        // Gradient / image bodies key off the primary fill (the
        // gradient-stop + image-fill editors operate on `fills[0]`), so
        // only the primary fill paints them; a non-primary gradient /
        // image fill shows head-row only until retyped to Solid.
        FillType::LinearGradient if is_primary => {
            paint_fill_gradient_body(cx, theme, edit, snapshot, locale, x, y, width, true);
        }
        FillType::RadialGradient if is_primary => {
            paint_fill_gradient_body(cx, theme, edit, snapshot, locale, x, y, width, false);
        }
        FillType::Image if is_primary => {
            paint_fill_image_body(cx, theme, snapshot, locale, x, y, width);
        }
        FillType::LinearGradient | FillType::RadialGradient | FillType::Image => {}
    }
    y += fill_row_body_height(fill_type, is_primary, primary_stop_count);
    y
}

