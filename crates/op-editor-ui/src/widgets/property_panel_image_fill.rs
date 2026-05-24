//! TS-parity floating editor for image fills.
//!
//! The body row in the Fill section toggles this popover; the upload
//! well inside it triggers the native file picker.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::{NodeSnapshot, PropertyPanelAction};
use crate::widgets::property_panel_image_preview::paint_image_preview;
use crate::widgets::property_panel_inputs::to_jian_color;
use crate::widgets::property_panel_layout::{
    action_button_rects_with_fill_picker, VisibleSections,
};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};

const PANEL_W: f32 = 220.0;
const PANEL_GAP: f32 = 8.0;
const PANEL_PAD: f32 = 12.0;
const HEADER_H: f32 = 44.0;
const MODE_H: f32 = 30.0;
const UPLOAD_H: f32 = 112.0;
const ADJ_HEADER_H: f32 = 28.0;
const ADJ_ROW_H: f32 = 34.0;

fn panel_h() -> f32 {
    HEADER_H
        + MODE_H
        + 10.0
        + UPLOAD_H
        + 12.0
        + 1.0
        + ADJ_HEADER_H
        + op_editor_core::ImageAdjustmentField::ALL.len() as f32 * ADJ_ROW_H
        + PANEL_PAD
}

fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.x <= r.origin.x + r.size.x
        && p.y >= r.origin.y
        && p.y <= r.origin.y + r.size.y
}

fn image_body_rect(panel_rect: Rect, visible: VisibleSections) -> Option<Rect> {
    if !visible.image && (!visible.fill || visible.fill_type != op_editor_core::FillType::Image) {
        return None;
    }
    action_button_rects_with_fill_picker(panel_rect, visible, &[], false, false, false, false)
        .into_iter()
        .find_map(|(action, rect)| {
            matches!(action, PropertyPanelAction::ToggleImageFillPopover).then_some(rect)
        })
}

fn popover_rect(panel_rect: Rect, visible: VisibleSections) -> Option<Rect> {
    let anchor = image_body_rect(panel_rect, visible)?;
    let h = panel_h();
    let min_top = panel_rect.origin.y + 8.0;
    let max_top = (panel_rect.origin.y + panel_rect.size.y - h - 8.0).max(min_top);
    Some(Rect {
        origin: Point2D::new(
            anchor.origin.x - PANEL_W - PANEL_GAP,
            anchor.origin.y.clamp(min_top, max_top),
        ),
        size: Point2D::new(PANEL_W, h),
    })
}

pub fn image_fill_popover_action_rects(
    panel_rect: Rect,
    visible: VisibleSections,
    snapshot: &NodeSnapshot,
) -> Vec<(PropertyPanelAction, Rect)> {
    let Some(pop) = popover_rect(panel_rect, visible) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    out.push((
        PropertyPanelAction::CloseImageFillPopover,
        Rect {
            origin: Point2D::new(pop.origin.x + pop.size.x - 34.0, pop.origin.y + 8.0),
            size: Point2D::new(26.0, 26.0),
        },
    ));

    let mode_y = pop.origin.y + HEADER_H;
    let mode_rect = Rect {
        origin: Point2D::new(pop.origin.x + PANEL_PAD, mode_y),
        size: Point2D::new(pop.size.x - PANEL_PAD * 2.0, MODE_H),
    };
    let chip_w = mode_rect.size.x / op_editor_core::ImageFillMode::ALL.len() as f32;
    for (i, mode) in op_editor_core::ImageFillMode::ALL.iter().enumerate() {
        out.push((
            PropertyPanelAction::SetImageFillMode(*mode),
            Rect {
                origin: Point2D::new(mode_rect.origin.x + i as f32 * chip_w, mode_rect.origin.y),
                size: Point2D::new(chip_w, mode_rect.size.y),
            },
        ));
    }

    let upload = upload_rect(pop);
    out.push((PropertyPanelAction::PickFillImage, upload));

    if snapshot
        .image_fill
        .as_ref()
        .map(|summary| summary.has_adjustments())
        .unwrap_or(false)
    {
        out.push((
            PropertyPanelAction::ResetImageAdjustments,
            Rect {
                origin: Point2D::new(
                    pop.origin.x + pop.size.x - 62.0,
                    adjustments_header_y(pop) + 3.0,
                ),
                size: Point2D::new(50.0, 20.0),
            },
        ));
    }
    out
}

pub fn image_fill_popover_action_at(
    panel_rect: Rect,
    visible: VisibleSections,
    snapshot: &NodeSnapshot,
    point: Point2D,
) -> Option<PropertyPanelAction> {
    for (action, rect) in image_fill_popover_action_rects(panel_rect, visible, snapshot)
        .into_iter()
        .rev()
    {
        if rect_contains(rect, point) {
            return Some(action);
        }
    }
    let pop = popover_rect(panel_rect, visible)?;
    for (field, track) in adjustment_track_rects(pop) {
        if rect_contains(track, point) {
            let pct = ((point.x - track.origin.x) / track.size.x).clamp(0.0, 1.0);
            return Some(PropertyPanelAction::SetImageAdjustment {
                field,
                value: (pct * 200.0 - 100.0).round(),
            });
        }
    }
    None
}

pub fn image_fill_popover_contains(
    panel_rect: Rect,
    visible: VisibleSections,
    point: Point2D,
) -> bool {
    popover_rect(panel_rect, visible)
        .map(|pop| rect_contains(pop, point))
        .unwrap_or(false)
}

pub fn image_fill_popover_adjustment_action_for_drag(
    panel_rect: Rect,
    visible: VisibleSections,
    field: op_editor_core::ImageAdjustmentField,
    x: f32,
) -> Option<PropertyPanelAction> {
    let pop = popover_rect(panel_rect, visible)?;
    let track = adjustment_track_rects(pop)
        .into_iter()
        .find_map(|(candidate, rect)| (candidate == field).then_some(rect))?;
    let pct = ((x - track.origin.x) / track.size.x).clamp(0.0, 1.0);
    Some(PropertyPanelAction::SetImageAdjustment {
        field,
        value: (pct * 200.0 - 100.0).round(),
    })
}

pub fn paint_image_fill_popover(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    panel_rect: Rect,
    visible: VisibleSections,
    snapshot: &NodeSnapshot,
    locale: op_editor_core::Locale,
) {
    let Some(pop) = popover_rect(panel_rect, visible) else {
        return;
    };
    let summary = snapshot
        .image_fill
        .clone()
        .unwrap_or(op_editor_core::ImageFillSummary {
            mode: op_editor_core::ImageFillMode::Fill,
            has_image: false,
            image_url: None,
            exposure: 0.0,
            contrast: 0.0,
            saturation: 0.0,
            temperature: 0.0,
            tint: 0.0,
            highlights: 0.0,
            shadows: 0.0,
        });
    cx.backend.fill_round_rect(pop, 8.0, theme.popover);
    cx.backend.stroke_round_rect(pop, 8.0, theme.border, 1.0);

    paint_label(
        cx,
        theme,
        op_i18n::translate(locale, "image.title"),
        pop.origin.x + PANEL_PAD,
        pop.origin.y + 28.0,
        13.0,
        theme.foreground,
    );
    draw_icon(
        cx.backend,
        Icon::Close,
        Point2D::new(pop.origin.x + pop.size.x - 30.0, pop.origin.y + 13.0),
        18.0,
        theme.foreground,
        1.8,
    );
    paint_mode_control(cx, theme, pop, summary.mode, locale);
    paint_upload(cx, theme, pop, &summary, locale);
    paint_adjustments(cx, theme, pop, &summary, locale);
}

fn upload_rect(pop: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            pop.origin.x + PANEL_PAD,
            pop.origin.y + HEADER_H + MODE_H + 10.0,
        ),
        size: Point2D::new(pop.size.x - PANEL_PAD * 2.0, UPLOAD_H),
    }
}

fn adjustments_header_y(pop: Rect) -> f32 {
    upload_rect(pop).origin.y + UPLOAD_H + 13.0
}

fn adjustment_track_rects(pop: Rect) -> Vec<(op_editor_core::ImageAdjustmentField, Rect)> {
    let mut out = Vec::new();
    let start_y = adjustments_header_y(pop) + ADJ_HEADER_H;
    for (i, field) in op_editor_core::ImageAdjustmentField::ALL.iter().enumerate() {
        out.push((
            *field,
            Rect {
                origin: Point2D::new(pop.origin.x + 80.0, start_y + i as f32 * ADJ_ROW_H + 15.0),
                size: Point2D::new(86.0, 8.0),
            },
        ));
    }
    out
}

fn paint_mode_control(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    pop: Rect,
    active: op_editor_core::ImageFillMode,
    locale: op_editor_core::Locale,
) {
    let mode_y = pop.origin.y + HEADER_H;
    let rect = Rect {
        origin: Point2D::new(pop.origin.x + PANEL_PAD, mode_y),
        size: Point2D::new(pop.size.x - PANEL_PAD * 2.0, MODE_H),
    };
    cx.backend.fill_round_rect(rect, 6.0, theme.muted);
    let chip_w = rect.size.x / op_editor_core::ImageFillMode::ALL.len() as f32;
    for (i, mode) in op_editor_core::ImageFillMode::ALL.iter().enumerate() {
        let chip = Rect {
            origin: Point2D::new(rect.origin.x + i as f32 * chip_w + 2.0, rect.origin.y + 2.0),
            size: Point2D::new(chip_w - 4.0, rect.size.y - 4.0),
        };
        let is_active = *mode == active;
        if is_active {
            cx.backend.fill_round_rect(chip, 5.0, theme.primary);
        }
        paint_centered_label(
            cx,
            theme,
            op_i18n::translate(locale, mode.label_key()),
            chip,
            if is_active {
                theme.primary_foreground
            } else {
                theme.muted_foreground
            },
        );
    }
}

fn paint_upload(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    pop: Rect,
    summary: &op_editor_core::ImageFillSummary,
    locale: op_editor_core::Locale,
) {
    let rect = upload_rect(pop);
    cx.backend.fill_round_rect(rect, 6.0, theme.muted);
    if let Some(src) = summary.image_url.as_deref() {
        let preview = Rect {
            origin: Point2D::new(rect.origin.x + 6.0, rect.origin.y + 6.0),
            size: Point2D::new((rect.size.x - 12.0).max(0.0), (rect.size.y - 12.0).max(0.0)),
        };
        if paint_image_preview(cx, preview, src, summary) {
            cx.backend.stroke_round_rect(rect, 6.0, theme.border, 1.0);
            return;
        }
    }
    draw_icon(
        cx.backend,
        Icon::ImagePlus,
        Point2D::new(
            rect.origin.x + rect.size.x / 2.0 - 12.0,
            rect.origin.y + 35.0,
        ),
        24.0,
        theme.muted_foreground,
        1.6,
    );
    let label = if summary.has_image {
        op_i18n::translate(locale, "image.title")
    } else {
        op_i18n::translate(locale, "image.clickToUpload")
    };
    paint_centered_label(
        cx,
        theme,
        label,
        Rect {
            origin: Point2D::new(rect.origin.x, rect.origin.y + 64.0),
            size: Point2D::new(rect.size.x, 24.0),
        },
        theme.muted_foreground,
    );
    cx.backend.stroke_round_rect(rect, 6.0, theme.border, 1.0);
}

fn paint_adjustments(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    pop: Rect,
    summary: &op_editor_core::ImageFillSummary,
    locale: op_editor_core::Locale,
) {
    let divider_y = upload_rect(pop).origin.y + UPLOAD_H + 12.0;
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(pop.origin.x, divider_y),
            size: Point2D::new(pop.size.x, 1.0),
        },
        theme.border,
    );
    let header_y = adjustments_header_y(pop);
    paint_label(
        cx,
        theme,
        op_i18n::translate(locale, "image.adjustments"),
        pop.origin.x + PANEL_PAD,
        header_y + 18.0,
        11.0,
        theme.muted_foreground,
    );
    if summary.has_adjustments() {
        paint_label(
            cx,
            theme,
            op_i18n::translate(locale, "image.reset"),
            pop.origin.x + pop.size.x - 56.0,
            header_y + 18.0,
            10.0,
            theme.muted_foreground,
        );
    }
    let start_y = header_y + ADJ_HEADER_H;
    for (i, field) in op_editor_core::ImageAdjustmentField::ALL.iter().enumerate() {
        let row_y = start_y + i as f32 * ADJ_ROW_H;
        let value = summary.adjustment(*field).clamp(-100.0, 100.0);
        paint_label(
            cx,
            theme,
            op_i18n::translate(locale, field.label_key()),
            pop.origin.x + PANEL_PAD,
            row_y + 21.0,
            11.0,
            theme.muted_foreground,
        );
        let track = Rect {
            origin: Point2D::new(pop.origin.x + 80.0, row_y + 17.0),
            size: Point2D::new(86.0, 4.0),
        };
        cx.backend.fill_round_rect(track, 2.0, theme.border);
        let fill_w = ((value + 100.0) / 200.0 * track.size.x).clamp(0.0, track.size.x);
        cx.backend.fill_round_rect(
            Rect {
                origin: track.origin,
                size: Point2D::new(fill_w, track.size.y),
            },
            2.0,
            theme.primary,
        );
        let knob_x = track.origin.x + fill_w;
        cx.backend.fill_oval(
            Rect {
                origin: Point2D::new(knob_x - 6.0, track.origin.y - 4.0),
                size: Point2D::new(12.0, 12.0),
            },
            theme.foreground,
        );
        paint_label(
            cx,
            theme,
            &format!("{}", value.round() as i32),
            pop.origin.x + pop.size.x - 30.0,
            row_y + 21.0,
            11.0,
            theme.muted_foreground,
        );
    }
}

fn paint_centered_label(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    label: &str,
    rect: Rect,
    color: Color,
) {
    let w = cx.backend.measure_text(label, 11.0);
    paint_label(
        cx,
        theme,
        label,
        rect.origin.x + (rect.size.x - w) / 2.0,
        rect.origin.y + rect.size.y / 2.0 + 4.0,
        11.0,
        color,
    );
}

fn paint_label(
    cx: &mut PaintCx<'_>,
    _theme: &Theme,
    label: &str,
    x: f32,
    baseline_y: f32,
    size: f32,
    color: Color,
) {
    let layout = TextLayout::single_run(
        label,
        "system-ui",
        size,
        to_jian_color(color),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&layout, Point2D::new(x, baseline_y));
}
