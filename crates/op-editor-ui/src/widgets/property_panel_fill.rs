//! Fill-section paint code split out of
//! `property_panel_sections.rs` to honour the 800-line ceiling.
//! Contains: `fill_type_label`, `paint_fill_type_picker`,
//! `paint_fill_section`, and the three per-type body paints
//! (`paint_fill_solid_body`, `paint_fill_gradient_body`,
//! `paint_fill_image_body`).

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::NodeSnapshot;
use crate::widgets::property_panel_inputs::{
    format_color_hex, paint_section_divider, paint_section_label_with_add, to_jian_color,
    HEADER_HEIGHT, INPUT_HEIGHT, INPUT_RADIUS, PAD_X, SECTION_GAP, SECTION_HEADER_HEIGHT,
    TAB_HEIGHT,
};
use crate::widgets::property_panel_layout::{fill_body_height, VisibleSections};
use crate::widgets::property_panel_sections::{EditContext, PropertyLabels};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
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
pub fn paint_fill_type_picker(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    panel_rect: Rect,
    visible: VisibleSections,
    active: op_editor_core::FillType,
    locale: op_editor_core::Locale,
) {
    use op_editor_core::FillType;
    let x0 = panel_rect.origin.x;
    let w = panel_rect.size.x;
    let usable_w = w - PAD_X * 2.0;

    // Replicate the y walk down to the Fill section's dropdown row
    // so the picker anchors directly under it.
    let mut y = panel_rect.origin.y;
    y += TAB_HEIGHT;
    y += HEADER_HEIGHT;
    y += 8.0 + 36.0 + 12.0;
    // Position section.
    y += SECTION_HEADER_HEIGHT;
    y += INPUT_HEIGHT + 6.0;
    y += INPUT_HEIGHT + 12.0;
    y += SECTION_GAP;
    if visible.flex_layout {
        y += SECTION_HEADER_HEIGHT;
        y += 32.0 + 12.0;
        y += SECTION_GAP;
    }
    if visible.size_options {
        y += SECTION_HEADER_HEIGHT;
        y += INPUT_HEIGHT + 10.0;
        y += 22.0 * 3.0;
        y += 12.0 + SECTION_GAP;
    }
    if visible.opacity {
        y += SECTION_HEADER_HEIGHT;
        y += INPUT_HEIGHT + 12.0 + SECTION_GAP;
    }
    // Fill section starts here.
    y += SECTION_HEADER_HEIGHT;
    let dropdown_x = x0 + PAD_X + 22.0 + 6.0;
    let dropdown_w = usable_w - 22.0 - 6.0 - 50.0 - 22.0 - 12.0;
    let panel_y = y + INPUT_HEIGHT + 4.0;
    let row_h = 32.0;
    let panel_h = row_h * 4.0 + 12.0;
    let pop_rect = Rect {
        origin: Point2D::new(dropdown_x, panel_y),
        size: Point2D::new(dropdown_w, panel_h),
    };
    cx.backend.fill_round_rect(pop_rect, 8.0, theme.popover);
    cx.backend
        .stroke_round_rect(pop_rect, 8.0, theme.border, 1.0);
    let types = [
        FillType::Solid,
        FillType::LinearGradient,
        FillType::RadialGradient,
        FillType::Image,
    ];
    for (i, t) in types.iter().enumerate() {
        let row_y = panel_y + 6.0 + i as f32 * row_h;
        let row_rect = Rect {
            origin: Point2D::new(pop_rect.origin.x + 4.0, row_y),
            size: Point2D::new(pop_rect.size.x - 8.0, row_h),
        };
        let is_active = *t == active;
        if is_active {
            cx.backend
                .fill_round_rect(row_rect, 6.0, theme.row_selected_primary);
        }
        let lbl_color = if is_active {
            theme.primary
        } else {
            theme.foreground
        };
        let lbl = TextLayout::single_run(
            fill_type_label(locale, *t),
            "system-ui",
            13.0,
            to_jian_color(lbl_color),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&lbl, Point2D::new(row_rect.origin.x + 14.0, row_y + 21.0));
        if is_active {
            draw_icon(
                cx.backend,
                Icon::Check,
                Point2D::new(row_rect.origin.x + row_rect.size.x - 24.0, row_y + 8.0),
                16.0,
                theme.primary,
                1.6,
            );
        }
    }
}
// ── Fill section ──────────────────────────────────────────────────

// Paint-context + geometry args threaded through; a struct adds no gain.
#[allow(clippy::too_many_arguments)]
pub fn paint_fill_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    edit: &EditContext<'_>,
    labels: &PropertyLabels,
    fill_type: op_editor_core::FillType,
    _fill_picker_open: bool,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut y = paint_section_label_with_add(cx, theme, labels.fill, x, y, width);
    let usable_w = width - PAD_X * 2.0;
    let fill = snapshot.fill.unwrap_or(Color::WHITE);
    let swatch_rect = Rect {
        origin: Point2D::new(x + PAD_X, y + 2.0),
        size: Point2D::new(22.0, 22.0),
    };
    // Swatch icon depends on the fill type so the head row reads
    // as a small preview of what's rendered below.
    use op_editor_core::FillType;
    match fill_type {
        FillType::Solid => {
            cx.backend.fill_round_rect(swatch_rect, 4.0, fill);
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
    cx.backend
        .fill_round_rect(dropdown_rect, INPUT_RADIUS, theme.muted);
    let type_label = fill_type_label(locale, fill_type);
    let label = TextLayout::single_run(
        type_label,
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label,
        Point2D::new(dropdown_rect.origin.x + 10.0, dropdown_rect.origin.y + 19.0),
    );
    draw_icon(
        cx.backend,
        Icon::ChevronDown,
        Point2D::new(
            dropdown_rect.origin.x + dropdown_rect.size.x - 22.0,
            dropdown_rect.origin.y + 5.0,
        ),
        16.0,
        theme.muted_foreground,
        1.4,
    );
    let pct_rect = Rect {
        origin: Point2D::new(dropdown_rect.origin.x + dropdown_rect.size.x + 6.0, y),
        size: Point2D::new(50.0, INPUT_HEIGHT),
    };
    cx.backend
        .fill_round_rect(pct_rect, INPUT_RADIUS, theme.muted);
    let opacity_focused = edit.focus == Some(PropertyFocus::FillOpacity);
    if opacity_focused {
        cx.backend
            .stroke_round_rect(pct_rect, INPUT_RADIUS, theme.primary, 1.5);
    }
    let opacity_owned = ((snapshot.fill_opacity * 100.0).round() as i32).to_string();
    let pct_text = edit.value_for(PropertyFocus::FillOpacity, &opacity_owned);
    let pct = TextLayout::single_run(
        pct_text,
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    let pct_x = pct_rect.origin.x + 10.0;
    cx.backend
        .draw_text(&pct, Point2D::new(pct_x, pct_rect.origin.y + 19.0));
    if let Some(pos) = edit.caret_at(PropertyFocus::FillOpacity) {
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
    let pct_unit = TextLayout::single_run(
        "%",
        "system-ui",
        12.0,
        to_jian_color(theme.muted_foreground),
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
            paint_fill_solid_body(cx, theme, edit, fill, x, y, width);
        }
        FillType::LinearGradient => {
            paint_fill_gradient_body(cx, theme, edit, snapshot, locale, x, y, width, true);
        }
        FillType::RadialGradient => {
            paint_fill_gradient_body(cx, theme, edit, snapshot, locale, x, y, width, false);
        }
        FillType::Image => {
            paint_fill_image_body(cx, theme, locale, x, y, width);
        }
    }
    y += fill_body_height(fill_type) - 6.0 + 12.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

fn paint_fill_solid_body(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    edit: &EditContext<'_>,
    fill: Color,
    x: f32,
    y: f32,
    width: f32,
) {
    let usable_w = width - PAD_X * 2.0;
    let hex_owned = format_color_hex(fill);
    let hex_focused = edit.focus == Some(PropertyFocus::FillHex);
    let hex_text = edit.value_for(PropertyFocus::FillHex, &hex_owned);
    let hex_rect = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(usable_w, INPUT_HEIGHT),
    };
    cx.backend
        .fill_round_rect(hex_rect, INPUT_RADIUS, theme.muted);
    if hex_focused {
        cx.backend
            .stroke_round_rect(hex_rect, INPUT_RADIUS, theme.primary, 1.5);
    }
    cx.backend.fill_round_rect(
        Rect {
            // Vertically centre the 16-tall swatch in the 30-tall row.
            origin: Point2D::new(hex_rect.origin.x + 6.0, hex_rect.origin.y + 7.0),
            size: Point2D::new(16.0, 16.0),
        },
        3.0,
        fill,
    );
    let hex_layout = TextLayout::single_run(
        hex_text,
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    let hex_x = hex_rect.origin.x + 30.0;
    cx.backend
        .draw_text(&hex_layout, Point2D::new(hex_x, hex_rect.origin.y + 19.0));
    if let Some(pos) = edit.caret_at(PropertyFocus::FillHex) {
        let w = cx
            .backend
            .measure_text(&hex_text[..pos.min(hex_text.len())], 12.0);
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(hex_x + w, hex_rect.origin.y + 6.0),
                size: Point2D::new(1.5, hex_rect.size.y - 12.0),
            },
            theme.foreground,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_fill_gradient_body(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    edit: &EditContext<'_>,
    snapshot: &NodeSnapshot,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
    show_angle: bool,
) {
    let usable_w = width - PAD_X * 2.0;
    let mut yy = y;
    if show_angle {
        let angle_rect = Rect {
            origin: Point2D::new(x + PAD_X, yy),
            size: Point2D::new(usable_w, INPUT_HEIGHT),
        };
        let angle_focus = PropertyFocus::GradientAngle;
        let angle_focused = edit.focus == Some(angle_focus);
        cx.backend
            .fill_round_rect(angle_rect, INPUT_RADIUS, theme.muted);
        if angle_focused {
            cx.backend
                .stroke_round_rect(angle_rect, INPUT_RADIUS, theme.primary, 1.5);
        }
        let prefix = TextLayout::single_run(
            op_i18n::translate(locale, "fill.angle"),
            "system-ui",
            12.0,
            to_jian_color(theme.muted_foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &prefix,
            Point2D::new(angle_rect.origin.x + 10.0, angle_rect.origin.y + 19.0),
        );
        let angle_owned = format_angle(snapshot.gradient_angle.unwrap_or(0.0));
        let value_text = edit.value_for(angle_focus, &angle_owned);
        let value = TextLayout::single_run(
            value_text,
            "system-ui",
            12.0,
            to_jian_color(theme.foreground),
            Point2D::new(0.0, 0.0),
        );
        let value_x = angle_rect.origin.x + 44.0;
        cx.backend
            .draw_text(&value, Point2D::new(value_x, angle_rect.origin.y + 19.0));
        if let Some(pos) = edit.caret_at(angle_focus) {
            let w = cx
                .backend
                .measure_text(&value_text[..pos.min(value_text.len())], 12.0);
            cx.backend.fill_rect(
                Rect {
                    origin: Point2D::new(value_x + w, angle_rect.origin.y + 6.0),
                    size: Point2D::new(1.5, angle_rect.size.y - 12.0),
                },
                theme.foreground,
            );
        }
        let unit = TextLayout::single_run(
            "°",
            "system-ui",
            12.0,
            to_jian_color(theme.muted_foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &unit,
            Point2D::new(
                angle_rect.origin.x + angle_rect.size.x - 14.0,
                angle_rect.origin.y + 19.0,
            ),
        );
        yy += INPUT_HEIGHT + 6.0;
    }
    // Color stops header + plus.
    let header = TextLayout::single_run(
        op_i18n::translate(locale, "fill.stops"),
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&header, Point2D::new(x + PAD_X, yy + 16.0));
    draw_icon(
        cx.backend,
        Icon::Plus,
        Point2D::new(x + width - PAD_X - 14.0, yy + 6.0),
        14.0,
        theme.muted_foreground,
        1.4,
    );
    yy += SECTION_HEADER_HEIGHT;
    let pct_w = 56.0;
    for (index, stop) in snapshot.gradient_stops.iter().enumerate() {
        let row_y = yy;
        let hex_w = usable_w - pct_w - 8.0;
        let hex_rect = Rect {
            origin: Point2D::new(x + PAD_X, row_y),
            size: Point2D::new(hex_w, INPUT_HEIGHT),
        };
        let hex_focus = PropertyFocus::GradientStopHex(index);
        let hex_focused = edit.focus == Some(hex_focus);
        cx.backend
            .fill_round_rect(hex_rect, INPUT_RADIUS, theme.muted);
        if hex_focused {
            cx.backend
                .stroke_round_rect(hex_rect, INPUT_RADIUS, theme.primary, 1.5);
        }
        let swatch = Rect {
            origin: Point2D::new(hex_rect.origin.x + 6.0, hex_rect.origin.y + 5.0),
            size: Point2D::new(16.0, 16.0),
        };
        paint_alpha_checker(cx, swatch, 3.0);
        cx.backend.fill_round_rect(swatch, 3.0, stop.color);
        // Display only `#RRGGBB` — the swatch on the left already
        // conveys per-stop transparency. Alpha is preserved at
        // commit time, so the user never types raw alpha digits.
        let hex_owned = stop_hex_rgb_only(&stop.hex);
        let hex_text = edit.value_for(hex_focus, &hex_owned);
        let hex_layout = TextLayout::single_run(
            hex_text,
            "system-ui",
            12.0,
            to_jian_color(theme.foreground),
            Point2D::new(0.0, 0.0),
        );
        let hex_text_x = hex_rect.origin.x + 30.0;
        cx.backend
            .draw_text(&hex_layout, Point2D::new(hex_text_x, hex_rect.origin.y + 19.0));
        if let Some(pos) = edit.caret_at(hex_focus) {
            let w = cx
                .backend
                .measure_text(&hex_text[..pos.min(hex_text.len())], 12.0);
            cx.backend.fill_rect(
                Rect {
                    origin: Point2D::new(hex_text_x + w, hex_rect.origin.y + 6.0),
                    size: Point2D::new(1.5, hex_rect.size.y - 12.0),
                },
                theme.foreground,
            );
        }
        let pct_rect = Rect {
            origin: Point2D::new(x + PAD_X + hex_w + 8.0, row_y),
            size: Point2D::new(pct_w, INPUT_HEIGHT),
        };
        let offset_focus = PropertyFocus::GradientStopOffset(index);
        let offset_focused = edit.focus == Some(offset_focus);
        cx.backend
            .fill_round_rect(pct_rect, INPUT_RADIUS, theme.muted);
        if offset_focused {
            cx.backend
                .stroke_round_rect(pct_rect, INPUT_RADIUS, theme.primary, 1.5);
        }
        let pct_owned = ((stop.offset * 100.0).round() as i32).to_string();
        let pct_text = edit.value_for(offset_focus, &pct_owned);
        let pct_layout = TextLayout::single_run(
            pct_text,
            "system-ui",
            12.0,
            to_jian_color(theme.foreground),
            Point2D::new(0.0, 0.0),
        );
        let pct_x = pct_rect.origin.x + 12.0;
        cx.backend
            .draw_text(&pct_layout, Point2D::new(pct_x, pct_rect.origin.y + 19.0));
        if let Some(pos) = edit.caret_at(offset_focus) {
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
        let pct_unit = TextLayout::single_run(
            "%",
            "system-ui",
            12.0,
            to_jian_color(theme.muted_foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &pct_unit,
            Point2D::new(
                pct_rect.origin.x + pct_rect.size.x - 14.0,
                pct_rect.origin.y + 19.0,
            ),
        );
        yy += INPUT_HEIGHT + 4.0;
    }
    let _ = yy;
}

/// Format the gradient angle for the panel input — drop the
/// fractional part when the angle is whole-degree so 0/45/90/180
/// don't paint as "0.0", "45.0", etc.
fn format_angle(angle: f32) -> String {
    if angle.fract() == 0.0 {
        format!("{}", angle as i32)
    } else {
        format!("{}", angle)
    }
}

/// Strip alpha from a canonical-schema stop hex (`#RRGGBB` or
/// `#RRGGBBAA`) for the panel input. The swatch on the left already
/// paints with alpha so the input stays at 6 hex chars regardless
/// of authored transparency.
pub fn stop_hex_rgb_only(hex: &str) -> String {
    let trimmed = hex.trim();
    let stripped = trimmed.trim_start_matches('#');
    let body = if stripped.len() >= 6 {
        &stripped[..6]
    } else {
        stripped
    };
    format!("#{}", body.to_uppercase())
}

/// Paint a 2×2 light/dark checker behind a colour swatch so a
/// partially or fully transparent fill reads as "transparent"
/// instead of looking like the input pill's background. The
/// caller paints the (possibly translucent) stop colour on top.
fn paint_alpha_checker(cx: &mut PaintCx<'_>, rect: Rect, radius: f32) {
    let light = Color {
        r: 0.95,
        g: 0.95,
        b: 0.95,
        a: 1.0,
    };
    let dark = Color {
        r: 0.78,
        g: 0.78,
        b: 0.78,
        a: 1.0,
    };
    cx.backend.fill_round_rect(rect, radius, light);
    let hx = rect.size.x / 2.0;
    let hy = rect.size.y / 2.0;
    // Skia has no clipped round-rect for sub-rect fills, so paint
    // the two darker quadrants as plain rects — the surrounding
    // light fill already supplies the rounded silhouette.
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(rect.origin.x + hx, rect.origin.y),
            size: Point2D::new(hx, hy),
        },
        dark,
    );
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(rect.origin.x, rect.origin.y + hy),
            size: Point2D::new(hx, hy),
        },
        dark,
    );
}

fn paint_fill_image_body(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
) {
    let usable_w = width - PAD_X * 2.0;
    let row = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(usable_w, INPUT_HEIGHT),
    };
    cx.backend.fill_round_rect(row, INPUT_RADIUS, theme.muted);
    cx.backend
        .stroke_round_rect(row, INPUT_RADIUS, theme.border, 1.0);
    draw_icon(
        cx.backend,
        Icon::ImagePlus,
        Point2D::new(row.origin.x + 6.0, row.origin.y + 5.0),
        18.0,
        theme.muted_foreground,
        1.4,
    );
    let label = TextLayout::single_run(
        op_i18n::translate(locale, "fill.title"),
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label,
        Point2D::new(row.origin.x + 30.0, row.origin.y + 19.0),
    );
}
