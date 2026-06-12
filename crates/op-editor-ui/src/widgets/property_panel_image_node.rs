//! Image-node specific section for the native property panel.
//!
//! Port of `apps/web/src/components/panels/image-section.tsx`:
//! thumbnail + fit-mode row (opens the image editor popover), the
//! local-asset warning row with Relink, and the Search / Generate
//! buttons whose popovers live in `property_panel_image_assets.rs`.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::{NodeSnapshot, PropertyPanelAction};
use crate::widgets::property_panel_image_assets::{
    ImageAssetWarning, IMAGE_BUTTON_H, IMAGE_ROW_GAP, IMAGE_WARNING_H,
};
use crate::widgets::property_panel_image_popovers::warning_colors;
use crate::widgets::property_panel_image_preview::paint_image_preview;
use crate::widgets::property_panel_inputs::{
    paint_section_divider, paint_section_label, to_jian_color, INPUT_HEIGHT, INPUT_RADIUS, PAD_X,
    SECTION_GAP,
};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};

/// Total height the image section consumes BEFORE the trailing
/// `SECTION_GAP` the walkers add — header + thumb row + optional
/// warning + buttons row + the legacy 34 px divider tail.
pub fn image_section_height(has_warning: bool) -> f32 {
    let warning = if has_warning {
        IMAGE_ROW_GAP + IMAGE_WARNING_H
    } else {
        0.0
    };
    crate::widgets::property_panel_inputs::SECTION_HEADER_HEIGHT
        + INPUT_HEIGHT
        + warning
        + IMAGE_ROW_GAP
        + IMAGE_BUTTON_H
        + 34.0
}

/// Push the section's clickable rects onto the walker output. `y` is
/// the section top (matches `paint_image_node_section`'s `y`).
pub fn push_image_action_rects(
    out: &mut Vec<(PropertyPanelAction, Rect)>,
    x0: f32,
    y: f32,
    w: f32,
    has_warning: bool,
) {
    let usable_w = w - PAD_X * 2.0;
    let mut y = y + crate::widgets::property_panel_inputs::SECTION_HEADER_HEIGHT;
    out.push((
        PropertyPanelAction::ToggleImageFillPopover,
        Rect {
            origin: Point2D::new(x0 + PAD_X, y),
            size: Point2D::new(usable_w, INPUT_HEIGHT),
        },
    ));
    y += INPUT_HEIGHT;
    if has_warning {
        y += IMAGE_ROW_GAP;
        // Relink button on the warning row's right edge (TS h-6 px-2
        // outline button). The warning row only paints when a host
        // asset check flagged the src, which implies a host that can
        // pop a relink dialog.
        out.push((
            PropertyPanelAction::RelinkImage,
            Rect {
                origin: Point2D::new(x0 + PAD_X + usable_w - 50.0, y + 8.0),
                size: Point2D::new(44.0, 24.0),
            },
        ));
        y += IMAGE_WARNING_H;
    }
    y += IMAGE_ROW_GAP;
    let half_w = (usable_w - 4.0) / 2.0;
    out.push((
        PropertyPanelAction::ToggleImageSearchPopover,
        Rect {
            origin: Point2D::new(x0 + PAD_X, y),
            size: Point2D::new(half_w, IMAGE_BUTTON_H),
        },
    ));
    out.push((
        PropertyPanelAction::ToggleImageGeneratePopover,
        Rect {
            origin: Point2D::new(x0 + PAD_X + half_w + 4.0, y),
            size: Point2D::new(half_w, IMAGE_BUTTON_H),
        },
    ));
}

#[allow(clippy::too_many_arguments)]
pub fn paint_image_node_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    warning: Option<&ImageAssetWarning>,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut y = paint_section_label(
        cx,
        theme,
        op_i18n::translate(locale, "image.title"),
        x,
        y,
        width,
    );
    let usable_w = width - PAD_X * 2.0;
    let row = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(usable_w, INPUT_HEIGHT),
    };
    cx.backend.fill_round_rect(row, INPUT_RADIUS, theme.muted);
    cx.backend
        .stroke_round_rect(row, INPUT_RADIUS, theme.border, 1.0);

    let summary = snapshot.image_fill.as_ref();
    let thumb = Rect {
        origin: Point2D::new(row.origin.x + 6.0, row.origin.y + 5.0),
        size: Point2D::new(20.0, 20.0),
    };
    let painted = summary
        .and_then(|summary| {
            summary
                .image_url
                .as_deref()
                .map(|src| paint_image_preview(cx, thumb, src, summary))
        })
        .unwrap_or(false);
    if !painted {
        draw_icon(
            cx.backend,
            Icon::ImagePlus,
            Point2D::new(thumb.origin.x, thumb.origin.y),
            18.0,
            theme.muted_foreground,
            1.4,
        );
    }
    let label_key = summary
        .map(|summary| summary.mode.label_key())
        .unwrap_or("image.fill");
    let label = TextLayout::single_run(
        op_i18n::translate(locale, label_key),
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label,
        Point2D::new(row.origin.x + 32.0, row.origin.y + 19.0),
    );
    y += INPUT_HEIGHT;

    if let Some(warning) = warning {
        y += IMAGE_ROW_GAP;
        y = paint_warning_row(cx, theme, warning, x, y, width);
    }

    // Search / Generate buttons (TS outline buttons, flex-1 h-7).
    y += IMAGE_ROW_GAP;
    let half_w = (usable_w - 4.0) / 2.0;
    paint_outline_button(
        cx,
        theme,
        Rect {
            origin: Point2D::new(x + PAD_X, y),
            size: Point2D::new(half_w, IMAGE_BUTTON_H),
        },
        Icon::Search,
        "Search",
    );
    paint_outline_button(
        cx,
        theme,
        Rect {
            origin: Point2D::new(x + PAD_X + half_w + 4.0, y),
            size: Point2D::new(half_w, IMAGE_BUTTON_H),
        },
        Icon::Sparkles,
        "Generate",
    );
    y += IMAGE_BUTTON_H + 34.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

fn paint_warning_row(
    cx: &mut PaintCx<'_>,
    _theme: &Theme,
    warning: &ImageAssetWarning,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let usable_w = width - PAD_X * 2.0;
    let row = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(usable_w, IMAGE_WARNING_H),
    };
    let (border, bg, icon, text) = warning_colors();
    cx.backend.fill_round_rect(row, 6.0, bg);
    cx.backend.stroke_round_rect(row, 6.0, border, 1.0);
    draw_icon(
        cx.backend,
        Icon::AlertTriangle,
        Point2D::new(row.origin.x + 8.0, row.origin.y + 8.0),
        14.0,
        icon,
        1.5,
    );
    let message = TextLayout::single_run(
        warning.message,
        "system-ui",
        10.0,
        to_jian_color(text),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &message,
        Point2D::new(row.origin.x + 28.0, row.origin.y + 16.0),
    );
    // Asset path line — dimmer, truncated to the row (TS break-all
    // wraps; we keep one line, see module divergence note).
    let mut path_color = text;
    path_color.a = 0.8;
    let path = TextLayout::single_run(
        &warning.asset_path,
        "system-ui",
        9.0,
        to_jian_color(path_color),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.save();
    cx.backend.clip_rect(Rect {
        origin: Point2D::new(row.origin.x + 28.0, row.origin.y + 20.0),
        size: Point2D::new(usable_w - 28.0 - 56.0, 16.0),
    });
    cx.backend.draw_text(
        &path,
        Point2D::new(row.origin.x + 28.0, row.origin.y + 31.0),
    );
    cx.backend.restore();
    // Relink button — right edge (rect mirrors push_image_action_rects).
    let btn = Rect {
        origin: Point2D::new(row.origin.x + usable_w - 50.0, row.origin.y + 8.0),
        size: Point2D::new(44.0, 24.0),
    };
    cx.backend.stroke_round_rect(btn, 5.0, border, 1.0);
    let relink = TextLayout::single_run(
        "Relink",
        "system-ui",
        9.0,
        to_jian_color(text),
        Point2D::new(0.0, 0.0),
    );
    let w = cx.backend.measure_text("Relink", 9.0);
    cx.backend.draw_text(
        &relink,
        Point2D::new(
            btn.origin.x + (btn.size.x - w) / 2.0,
            btn.origin.y + btn.size.y / 2.0 + 3.0,
        ),
    );
    y + IMAGE_WARNING_H
}

fn paint_outline_button(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, icon: Icon, label: &str) {
    cx.backend.fill_round_rect(rect, 6.0, theme.card);
    cx.backend.stroke_round_rect(rect, 6.0, theme.border, 1.0);
    let label_w = cx.backend.measure_text(label, 11.0);
    let start_x = rect.origin.x + (rect.size.x - label_w - 17.0) / 2.0;
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(start_x, rect.origin.y + 7.0),
        13.0,
        theme.foreground,
        1.5,
    );
    let layout = TextLayout::single_run(
        label,
        "system-ui",
        11.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &layout,
        Point2D::new(start_x + 17.0, rect.origin.y + rect.size.y / 2.0 + 4.0),
    );
}
