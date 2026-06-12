//! Paint for the image-node Search / Generate popovers — split out
//! of `property_panel_image_assets.rs` (geometry + hit-testing) to
//! honor the 800-line cap. Strings are literal English to match the
//! TS components verbatim (they hardcode them, no i18n keys).

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel_image_assets::{
    generate_popover_layout, search_popover_layout, GeneratePopoverView, ImageGenProfileView,
    POPOVER_PAD,
};
use crate::widgets::property_panel_inputs::to_jian_color;
use crate::widgets::property_panel_layout::VisibleSections;
use crate::widgets::PaintCx;
use crate::{Color, ImageAdjustments, ImageDrawMode, Point2D, Rect, TextLayout};
use op_editor_core::image_panel_state::{ImageGeneratePhase, ImagePanelState, ImageSearchSource};

// --- Paint ------------------------------------------------------------

fn hex_color(rgb: u32, a: f32) -> Color {
    Color {
        r: ((rgb >> 16) & 0xff) as f32 / 255.0,
        g: ((rgb >> 8) & 0xff) as f32 / 255.0,
        b: (rgb & 0xff) as f32 / 255.0,
        a,
    }
}

fn paint_popup_chrome(cx: &mut PaintCx<'_>, theme: &Theme, popup: Rect) {
    cx.backend.fill_round_rect(popup, 8.0, theme.popover);
    cx.backend.stroke_round_rect(popup, 8.0, theme.border, 1.0);
}

fn paint_spinner(cx: &mut PaintCx<'_>, theme: &Theme, centre: Point2D, size: f32, now_ms: u64) {
    let angle = (now_ms % 1000) as f32 / 1000.0 * std::f32::consts::TAU;
    cx.backend.save();
    cx.backend.rotate(angle, centre);
    draw_icon(
        cx.backend,
        Icon::Loader,
        Point2D::new(centre.x - size / 2.0, centre.y - size / 2.0),
        size,
        theme.muted_foreground,
        1.6,
    );
    cx.backend.restore();
}

fn paint_centered_label(
    cx: &mut PaintCx<'_>,
    color: Color,
    text: &str,
    size: f32,
    centre_x: f32,
    baseline: f32,
) {
    let layout = TextLayout::single_run(
        text,
        "system-ui",
        size,
        to_jian_color(color),
        Point2D::new(0.0, 0.0),
    );
    let w = cx.backend.measure_text(text, size);
    cx.backend
        .draw_text(&layout, Point2D::new(centre_x - w / 2.0, baseline));
}

fn paint_data_url_image(cx: &mut PaintCx<'_>, rect: Rect, src: &str, radius: f32) {
    let Some(bytes) = crate::widgets::property_panel_image_preview::data_url_bytes(src) else {
        return;
    };
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut h);
    cx.backend.save();
    cx.backend.clip_round_rect(rect, radius);
    cx.backend.draw_image_with_options(
        rect,
        h.finish(),
        &bytes,
        ImageDrawMode::Crop,
        ImageAdjustments::default(),
        1.0,
        0.0,
    );
    cx.backend.restore();
}

/// Paint the search popover (late overlay).
#[allow(clippy::too_many_arguments)]
pub fn paint_search_popover(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    panel_rect: Rect,
    visible: VisibleSections,
    state: &ImagePanelState,
    now_ms: u64,
) {
    let Some(layout) = search_popover_layout(panel_rect, visible, state) else {
        return;
    };
    paint_popup_chrome(cx, theme, layout.popup);

    // Search input — bordered box, draft or muted placeholder + caret.
    cx.backend.fill_round_rect(layout.input, 5.0, theme.card);
    cx.backend
        .stroke_round_rect(layout.input, 5.0, theme.border, 1.0);
    let text_x = layout.input.origin.x + 8.0;
    let baseline = layout.input.origin.y + layout.input.size.y / 2.0 + 4.0;
    if state.search_query.is_empty() {
        let placeholder = TextLayout::single_run(
            "Search images...",
            "system-ui",
            11.0,
            to_jian_color(theme.muted_foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&placeholder, Point2D::new(text_x, baseline));
    } else {
        let draft = TextLayout::single_run(
            &state.search_query,
            "system-ui",
            11.0,
            to_jian_color(theme.foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(&draft, Point2D::new(text_x, baseline));
    }
    let caret_x = text_x + cx.backend.measure_text(&state.search_query, 11.0) + 1.0;
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(caret_x, layout.input.origin.y + 7.0),
            size: Point2D::new(1.0, layout.input.size.y - 14.0),
        },
        theme.foreground,
    );

    // Submit icon-button (disabled wash while loading / empty query).
    let disabled = state.search_loading || state.search_query.trim().is_empty();
    cx.backend.fill_round_rect(layout.submit, 5.0, theme.card);
    cx.backend
        .stroke_round_rect(layout.submit, 5.0, theme.border, 1.0);
    draw_icon(
        cx.backend,
        Icon::Search,
        Point2D::new(layout.submit.origin.x + 7.0, layout.submit.origin.y + 7.0),
        14.0,
        if disabled {
            theme.muted_foreground
        } else {
            theme.foreground
        },
        1.5,
    );

    let centre_x = layout.body.origin.x + layout.body.size.x / 2.0;
    if state.search_loading {
        paint_spinner(
            cx,
            theme,
            Point2D::new(centre_x, layout.body.origin.y + 30.0),
            20.0,
            now_ms,
        );
        paint_centered_label(
            cx,
            theme.muted_foreground,
            "Searching...",
            11.0,
            centre_x,
            layout.body.origin.y + 58.0,
        );
    } else if state.search_results.is_empty() {
        draw_icon(
            cx.backend,
            Icon::Image,
            Point2D::new(centre_x - 12.0, layout.body.origin.y + 16.0),
            24.0,
            theme.muted_foreground,
            1.4,
        );
        paint_centered_label(
            cx,
            theme.muted_foreground,
            if state.search_has_searched {
                "No results found"
            } else {
                "Search for images"
            },
            11.0,
            centre_x,
            layout.body.origin.y + 58.0,
        );
    } else {
        for (cell, hit) in layout.cells.iter().zip(state.search_results.iter()) {
            cx.backend.fill_round_rect(*cell, 5.0, theme.muted);
            cx.backend.stroke_round_rect(*cell, 5.0, theme.border, 1.0);
            paint_data_url_image(cx, *cell, &hit.thumb_data_url, 5.0);
        }
        if let (Some(footer), Some(source)) = (layout.footer, state.search_source) {
            cx.backend.fill_rect(
                Rect {
                    origin: Point2D::new(footer.origin.x, footer.origin.y - 4.0),
                    size: Point2D::new(footer.size.x, 1.0),
                },
                theme.border,
            );
            let label = format!(
                "Images from {}. Freely licensed — verify license before use.",
                match source {
                    ImageSearchSource::Openverse => "Openverse",
                    ImageSearchSource::Wikimedia => "Wikimedia Commons",
                }
            );
            let layout_text = TextLayout::single_run(
                &label,
                "system-ui",
                9.0,
                to_jian_color(theme.muted_foreground),
                Point2D::new(0.0, 0.0),
            );
            cx.backend.draw_text(
                &layout_text,
                Point2D::new(footer.origin.x, footer.origin.y + 12.0),
            );
        }
    }
}

/// Paint the generate popover (late overlay).
#[allow(clippy::too_many_arguments)]
pub fn paint_generate_popover(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    panel_rect: Rect,
    visible: VisibleSections,
    state: &ImagePanelState,
    profile: Option<&ImageGenProfileView>,
    now_ms: u64,
) {
    let Some(layout) = generate_popover_layout(panel_rect, visible, state, profile) else {
        return;
    };
    paint_popup_chrome(cx, theme, layout.popup);
    let centre_x = layout.popup.origin.x + layout.popup.size.x / 2.0;
    match layout.view {
        GeneratePopoverView::NotConfigured => {
            draw_icon(
                cx.backend,
                Icon::Settings,
                Point2D::new(centre_x - 16.0, layout.popup.origin.y + POPOVER_PAD),
                32.0,
                theme.muted_foreground,
                1.4,
            );
            paint_centered_label(
                cx,
                theme.muted_foreground,
                "Image generation not configured",
                11.0,
                centre_x,
                layout.popup.origin.y + POPOVER_PAD + 32.0 + 10.0 + 12.0,
            );
            if let Some(btn) = layout.primary {
                cx.backend.fill_round_rect(btn, 6.0, theme.card);
                cx.backend.stroke_round_rect(btn, 6.0, theme.border, 1.0);
                paint_centered_label(
                    cx,
                    theme.foreground,
                    "Open Settings",
                    11.0,
                    btn.origin.x + btn.size.x / 2.0,
                    btn.origin.y + btn.size.y / 2.0 + 4.0,
                );
            }
        }
        GeneratePopoverView::Loading => {
            paint_spinner(
                cx,
                theme,
                Point2D::new(centre_x, layout.popup.origin.y + POPOVER_PAD + 14.0),
                24.0,
                now_ms,
            );
            paint_centered_label(
                cx,
                theme.muted_foreground,
                "Generating...",
                11.0,
                centre_x,
                layout.popup.origin.y + POPOVER_PAD + 48.0,
            );
        }
        GeneratePopoverView::Preview => {
            if let (Some(rect), Some(url)) = (layout.preview, state.generate_preview.as_ref()) {
                cx.backend.fill_round_rect(rect, 6.0, theme.muted);
                cx.backend.stroke_round_rect(rect, 6.0, theme.border, 1.0);
                paint_data_url_image(cx, rect, url, 6.0);
            }
            if let Some(btn) = layout.primary {
                cx.backend.fill_round_rect(btn, 6.0, theme.primary);
                paint_centered_label(
                    cx,
                    theme.primary_foreground,
                    "Apply",
                    11.0,
                    btn.origin.x + btn.size.x / 2.0,
                    btn.origin.y + btn.size.y / 2.0 + 4.0,
                );
            }
            if let Some(btn) = layout.secondary {
                cx.backend.fill_round_rect(btn, 6.0, theme.card);
                cx.backend.stroke_round_rect(btn, 6.0, theme.border, 1.0);
                paint_centered_label(
                    cx,
                    theme.foreground,
                    "Retry",
                    11.0,
                    btn.origin.x + btn.size.x / 2.0,
                    btn.origin.y + btn.size.y / 2.0 + 4.0,
                );
            }
        }
        GeneratePopoverView::Idle => {
            if let Some(ta) = layout.textarea {
                cx.backend.fill_round_rect(ta, 6.0, theme.card);
                cx.backend.stroke_round_rect(ta, 6.0, theme.border, 1.0);
                let text_x = ta.origin.x + 10.0;
                if state.generate_prompt.is_empty() {
                    let placeholder = TextLayout::single_run(
                        "Describe the image...",
                        "system-ui",
                        11.0,
                        to_jian_color(theme.muted_foreground),
                        Point2D::new(0.0, 0.0),
                    );
                    cx.backend
                        .draw_text(&placeholder, Point2D::new(text_x, ta.origin.y + 18.0));
                } else {
                    // Single-line tail view of the prompt (textarea
                    // wrapping is a follow-up; the host caps drafts).
                    let prompt = TextLayout::single_run(
                        &state.generate_prompt,
                        "system-ui",
                        11.0,
                        to_jian_color(theme.foreground),
                        Point2D::new(0.0, 0.0),
                    );
                    cx.backend
                        .draw_text(&prompt, Point2D::new(text_x, ta.origin.y + 18.0));
                }
                let caret_x = text_x + cx.backend.measure_text(&state.generate_prompt, 11.0) + 1.0;
                cx.backend.fill_rect(
                    Rect {
                        origin: Point2D::new(caret_x, ta.origin.y + 8.0),
                        size: Point2D::new(1.0, 14.0),
                    },
                    theme.foreground,
                );
            }
            if state.generate_phase == ImageGeneratePhase::Error {
                let err = TextLayout::single_run(
                    &state.generate_error,
                    "system-ui",
                    10.0,
                    to_jian_color(hex_color(0xef4444, 1.0)), // destructive
                    Point2D::new(0.0, 0.0),
                );
                cx.backend.draw_text(
                    &err,
                    Point2D::new(
                        layout.popup.origin.x + POPOVER_PAD,
                        layout.popup.origin.y + POPOVER_PAD + 48.0 + 20.0,
                    ),
                );
            }
            if let Some(btn) = layout.primary {
                let enabled = !state.generate_prompt.trim().is_empty();
                cx.backend.fill_round_rect(
                    btn,
                    6.0,
                    if enabled { theme.primary } else { theme.muted },
                );
                let fg = if enabled {
                    theme.primary_foreground
                } else {
                    theme.muted_foreground
                };
                let label_w = cx.backend.measure_text("Generate", 11.0);
                let start_x = btn.origin.x + (btn.size.x - label_w - 18.0) / 2.0;
                draw_icon(
                    cx.backend,
                    Icon::Sparkles,
                    Point2D::new(start_x, btn.origin.y + 7.0),
                    14.0,
                    fg,
                    1.5,
                );
                let label = TextLayout::single_run(
                    "Generate",
                    "system-ui",
                    11.0,
                    to_jian_color(fg),
                    Point2D::new(0.0, 0.0),
                );
                cx.backend.draw_text(
                    &label,
                    Point2D::new(start_x + 18.0, btn.origin.y + btn.size.y / 2.0 + 4.0),
                );
                // Footer: "profile · provider · model" (TS bottom line).
                if let Some(p) = profile {
                    let footer = format!(
                        "{} · {} · {}",
                        p.name,
                        p.provider,
                        if p.model.is_empty() {
                            "default"
                        } else {
                            p.model.as_str()
                        }
                    );
                    paint_centered_label(
                        cx,
                        theme.muted_foreground,
                        &footer,
                        9.0,
                        centre_x,
                        btn.origin.y + btn.size.y + 16.0,
                    );
                }
            }
        }
    }
}

/// Warning-row palette (TS hardcodes Tailwind orange).
pub(crate) fn warning_colors() -> (Color, Color, Color, Color) {
    (
        hex_color(0xf97316, 0.4), // border orange-500/40
        hex_color(0xf97316, 0.1), // bg orange-500/10
        hex_color(0xfb923c, 1.0), // icon orange-400
        hex_color(0xfed7aa, 1.0), // text orange-200
    )
}
