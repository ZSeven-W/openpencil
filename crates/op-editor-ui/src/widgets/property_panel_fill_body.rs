//! Fill-section per-type body paints — extracted from
//! `property_panel_fill.rs` to honour the 800-line ceiling. Contains
//! the Solid hex-input body, the Linear / Radial gradient body
//! (angle row + stop rows), and the small hex / angle formatters they
//! share. `paint_fill_image_body` lives in
//! `property_panel_fill_image_body.rs`.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::NodeSnapshot;
use crate::widgets::property_panel_color_variables::paint_color_variable_button;
use crate::widgets::property_panel_inputs::{
    format_color_hex, COLOR_VARIABLE_BUTTON_W, COLOR_VARIABLE_GAP, INPUT_HEIGHT, INPUT_RADIUS,
    PAD_X, SECTION_HEADER_HEIGHT,
};
use crate::widgets::property_panel_sections::EditContext;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::PropertyFocus;

#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_fill_solid_body(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    edit: &EditContext<'_>,
    fill_index: usize,
    fill: Color,
    variable_ref: Option<&str>,
    show_variable_button: bool,
    x: f32,
    y: f32,
    width: f32,
) {
    let hex_focus = PropertyFocus::FillHex(fill_index);
    let usable_w = width - PAD_X * 2.0;
    let hex_owned = format_color_hex(fill);
    let hex_focused = edit.focus == Some(hex_focus);
    let variable_text = variable_ref.map(|name| format!("${name}"));
    let hex_text = variable_text
        .as_deref()
        .unwrap_or_else(|| edit.value_for(hex_focus, &hex_owned));
    let variable_w = if show_variable_button {
        COLOR_VARIABLE_BUTTON_W + COLOR_VARIABLE_GAP
    } else {
        0.0
    };
    let hex_rect = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(usable_w - variable_w, INPUT_HEIGHT),
    };
    cx.backend
        .fill_round_rect(hex_rect, INPUT_RADIUS, theme.muted);
    if hex_focused && variable_ref.is_none() {
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
    let hex_x = hex_rect.origin.x + 30.0;
    let painted_hex = variable_ref.is_none()
        && edit.paint_input_view_at(
            cx,
            theme,
            hex_focus,
            Rect {
                origin: Point2D::new(hex_x, hex_rect.origin.y),
                size: Point2D::new(
                    (hex_rect.origin.x + hex_rect.size.x - 8.0 - hex_x).max(0.0),
                    hex_rect.size.y,
                ),
            },
            12.0,
            0.0,
            hex_rect.origin.y + 19.0,
        );
    if !painted_hex {
        let hex_layout = TextLayout::single_run(
            hex_text,
            "system-ui",
            12.0,
            (theme.foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        if variable_ref.is_none() {
            edit.paint_selection_at(
                cx,
                theme,
                hex_focus,
                hex_text,
                hex_x,
                hex_rect.origin.y + 19.0,
                12.0,
                hex_rect.origin.x + hex_rect.size.x - 8.0,
            );
        }
        cx.backend
            .draw_text(&hex_layout, Point2D::new(hex_x, hex_rect.origin.y + 19.0));
        if variable_ref.is_none() {
            if let Some(pos) = edit.caret_at(hex_focus) {
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
    }
    if show_variable_button {
        paint_color_variable_button(
            cx,
            theme,
            Rect {
                origin: Point2D::new(hex_rect.origin.x + hex_rect.size.x + COLOR_VARIABLE_GAP, y),
                size: Point2D::new(COLOR_VARIABLE_BUTTON_W, INPUT_HEIGHT),
            },
            variable_ref.is_some(),
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_fill_gradient_body(
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
            (theme.muted_foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &prefix,
            Point2D::new(angle_rect.origin.x + 10.0, angle_rect.origin.y + 19.0),
        );
        let angle_owned = format_angle(snapshot.gradient_angle.unwrap_or(0.0));
        let value_text = edit.value_for(angle_focus, &angle_owned);
        let value_x = angle_rect.origin.x + 44.0;
        if !edit.paint_input_view_at(
            cx,
            theme,
            angle_focus,
            Rect {
                origin: Point2D::new(value_x, angle_rect.origin.y),
                size: Point2D::new(
                    (angle_rect.origin.x + angle_rect.size.x - 18.0 - value_x).max(0.0),
                    angle_rect.size.y,
                ),
            },
            12.0,
            0.0,
            angle_rect.origin.y + 19.0,
        ) {
            let value = TextLayout::single_run(
                value_text,
                "system-ui",
                12.0,
                (theme.foreground).to_jian(),
                Point2D::new(0.0, 0.0),
            );
            edit.paint_selection_at(
                cx,
                theme,
                angle_focus,
                value_text,
                value_x,
                angle_rect.origin.y + 19.0,
                12.0,
                angle_rect.origin.x + angle_rect.size.x - 8.0,
            );
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
        }
        let unit = TextLayout::single_run(
            "°",
            "system-ui",
            12.0,
            (theme.muted_foreground).to_jian(),
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
        (theme.foreground).to_jian(),
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
    let show_remove = snapshot.gradient_stops.len() > 2;
    let remove_w = if show_remove { 26.0 } else { 0.0 };
    let remove_gap = if show_remove { 6.0 } else { 0.0 };
    for (index, stop) in snapshot.gradient_stops.iter().enumerate() {
        let row_y = yy;
        let hex_w = usable_w - pct_w - 8.0 - remove_w - remove_gap;
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
        jian_widgets::components::swatch::Swatch {
            color: stop.color,
            radius: 3.0,
            border: false,
        }
        .paint(
            cx.backend,
            swatch,
            &crate::widgets::button::tokens_from_theme(theme),
        );
        // Display only `#RRGGBB` — the swatch on the left already
        // conveys per-stop transparency. Alpha is preserved at
        // commit time, so the user never types raw alpha digits.
        let hex_owned = stop_hex_rgb_only(&stop.hex);
        let hex_text = edit.value_for(hex_focus, &hex_owned);
        let hex_text_x = hex_rect.origin.x + 30.0;
        if !edit.paint_input_view_at(
            cx,
            theme,
            hex_focus,
            Rect {
                origin: Point2D::new(hex_text_x, hex_rect.origin.y),
                size: Point2D::new(
                    (hex_rect.origin.x + hex_rect.size.x - 8.0 - hex_text_x).max(0.0),
                    hex_rect.size.y,
                ),
            },
            12.0,
            0.0,
            hex_rect.origin.y + 19.0,
        ) {
            let hex_layout = TextLayout::single_run(
                hex_text,
                "system-ui",
                12.0,
                (theme.foreground).to_jian(),
                Point2D::new(0.0, 0.0),
            );
            edit.paint_selection_at(
                cx,
                theme,
                hex_focus,
                hex_text,
                hex_text_x,
                hex_rect.origin.y + 19.0,
                12.0,
                hex_rect.origin.x + hex_rect.size.x - 8.0,
            );
            cx.backend.draw_text(
                &hex_layout,
                Point2D::new(hex_text_x, hex_rect.origin.y + 19.0),
            );
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
        let pct_x = pct_rect.origin.x + 12.0;
        if !edit.paint_input_view_at(
            cx,
            theme,
            offset_focus,
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
            let pct_layout = TextLayout::single_run(
                pct_text,
                "system-ui",
                12.0,
                (theme.foreground).to_jian(),
                Point2D::new(0.0, 0.0),
            );
            edit.paint_selection_at(
                cx,
                theme,
                offset_focus,
                pct_text,
                pct_x,
                pct_rect.origin.y + 19.0,
                12.0,
                pct_rect.origin.x + pct_rect.size.x - 8.0,
            );
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
        if show_remove {
            draw_icon(
                cx.backend,
                Icon::Close,
                Point2D::new(
                    pct_rect.origin.x + pct_rect.size.x + 10.0,
                    row_y + (INPUT_HEIGHT - 14.0) / 2.0,
                ),
                14.0,
                theme.muted_foreground,
                1.4,
            );
        }
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
