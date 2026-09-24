//! The Studio Home composer and preview panels.

use super::super::copy::{self, SANS};
use super::super::fade;
use super::super::paint::art::paint_app_art;
use super::super::paint::art_phase;
use super::super::paint::cards::{
    paint_sticker, paint_template_paper, preview_template_for, text, text_weighted,
};
use super::super::{HomeLayout, HomeSurface, StudioPalette};
use crate::widgets::brand_icons::paint_figma_logo;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, RenderBackend};
use jian_widgets::components::text_area::TextArea;
use jian_widgets::Tokens;
use op_editor_core::{HomeFamily, HomeHit, InfoKind, SlideRatio};

/// Input text metrics (prototype 14 px / 1.75 line-height; jian's text
/// area uses 1.35 — the composer honors the component's own advance).
const INPUT_FONT: f32 = 14.0;

fn studio_tokens(palette: StudioPalette) -> Tokens {
    let mut tokens = Tokens::light();
    tokens.foreground = palette.ink;
    tokens.muted_foreground = palette.placeholder;
    tokens.primary = palette.blue;
    tokens
}

pub(super) fn paint_composer(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &HomeLayout,
    rise: f32,
    alpha: f32,
) {
    let palette = StudioPalette::for_mode(surface.ui.effective_theme_mode());
    let palette = fade_palette(palette, alpha);
    let locale = surface.ui.locale;
    let composer = shift(layout.composer, rise);
    cx.backend.fill_drop_shadow(
        Rect::xywh(
            composer.origin.x + 6.0,
            composer.origin.y + 10.0,
            composer.size.x - 12.0,
            composer.size.y - 8.0,
        ),
        12.0,
        14.0,
        fade(palette.ink, 0.08),
    );
    cx.backend.fill_round_rect(composer, 14.0, palette.panel);
    cx.backend
        .stroke_round_rect(composer, 14.0, palette.line, 1.0);

    // ── label row + segmented control ─────────────────────────────────
    let task_copy = copy::task_copy(locale, surface.state.task, surface.state.task_draft());
    let label_row = shift(layout.label_row, rise);
    text_weighted(
        cx,
        task_copy.label,
        Point2D::new(
            label_row.origin.x,
            jian_widgets::centered_text_baseline_y(label_row, 13.0),
        ),
        13.0,
        palette.ink,
        600,
    );
    let labels = copy::segment_labels(locale, surface.state.task);
    if !labels.is_empty() {
        let selected = copy::segment_index(surface.state.task, surface.state.task_draft());
        let segment = shift(layout.segment, rise);
        cx.backend.fill_round_rect(segment, 7.0, palette.segment_bg);
        cx.backend
            .stroke_round_rect(segment, 7.0, palette.segment_line, 1.0);
        for (index, label) in labels.iter().enumerate() {
            let option = shift(layout.segment_options[index], rise);
            let on = index as u8 == selected;
            if on {
                cx.backend.fill_round_rect(option, 5.0, palette.blue_soft);
            } else if surface.state.hover == Some(HomeHit::Segment(index as u8)) {
                cx.backend
                    .fill_round_rect(option, 5.0, fade(palette.blue_soft, 0.45));
            }
            let label_w = cx.backend.measure_text_family(label, 12.0, SANS);
            text_weighted(
                cx,
                label,
                Point2D::new(
                    option.origin.x + (option.size.x - label_w) / 2.0,
                    jian_widgets::centered_text_baseline_y(option, 12.0),
                ),
                12.0,
                if on {
                    palette.blue
                } else {
                    fade(palette.ink, 0.58)
                },
                if on { 600 } else { 400 },
            );
        }
    }

    // ── the input box ─────────────────────────────────────────────────
    let input_box = shift(layout.input_box, rise);
    let engaged = !surface.state.draft.is_empty()
        || matches!(
            surface.state.hover,
            Some(HomeHit::Sheet) | Some(HomeHit::Send)
        );
    cx.backend
        .fill_round_rect(input_box, 10.0, palette.surface_input);
    if engaged {
        cx.backend.fill_round_rect(
            Rect::xywh(
                input_box.origin.x - 3.0,
                input_box.origin.y - 3.0,
                input_box.size.x + 6.0,
                input_box.size.y + 6.0,
            ),
            13.0,
            fade(palette.blue, 0.05),
        );
    }
    cx.backend.stroke_round_rect(
        input_box,
        10.0,
        if engaged {
            Color::rgb_u8(0x74, 0xA7, 0xFF)
        } else {
            palette.input_line
        },
        1.0,
    );
    // jian's TextArea pads y by 4 and x by its pad_x; shift the rect so
    // the text lands at the prototype's 13/14 padding.
    let text_rect = Rect::xywh(
        input_box.origin.x + 1.0,
        input_box.origin.y + 9.0,
        input_box.size.x - 2.0,
        input_box.size.y - 18.0,
    );
    let text_area = TextArea {
        state: &surface.state.input,
        placeholder: task_copy.placeholder,
        focused: surface.state.visible,
        font_size: INPUT_FONT,
        now_ms: surface.now_ms,
        pad_x: 13.0,
        max_visible_lines: 8,
    };
    // `draw_text` is baseline-relative but jian's TextArea positions glyph
    // runs from the line top, so every draw shifts down by the ascent —
    // the same wrapper the chat composer's input paints through.
    let mut backend = crate::widgets::text_input_backend::BaselineAdjustingBackend {
        inner: cx.backend,
        baseline_delta_y: 14.0,
    };
    backend.save();
    backend.clip_round_rect(input_box, 10.0);
    text_area.paint(&mut backend, text_rect, &studio_tokens(palette));
    backend.restore();

    // Attachment chips (the chat composer's staged list is the truth).
    let names = &surface.attachment_names;
    if !names.is_empty() {
        let chips_h = names.len().min(2) as f32 * 34.0 + 8.0;
        let chips_area = Rect::xywh(
            input_box.origin.x + 10.0,
            input_box.origin.y + input_box.size.y - chips_h,
            input_box.size.x - 20.0,
            chips_h,
        );
        let mut chip_x = chips_area.origin.x;
        for name in names.iter().take(3) {
            let label: String = name.chars().take(8).collect();
            let chip_w = cx.backend.measure_text_family(&label, 11.0, SANS) + 26.0 + 34.0;
            let chip = Rect::xywh(chip_x, chips_area.origin.y + 6.0, chip_w, 26.0);
            cx.backend.fill_round_rect(chip, 6.0, palette.chip_bg);
            cx.backend
                .stroke_round_rect(chip, 6.0, palette.chip_line, 1.0);
            cx.backend.fill_round_rect(
                Rect::xywh(chip.origin.x + 4.0, chip.origin.y + 3.0, 20.0, 20.0),
                3.0,
                fade(palette.blue, 0.25),
            );
            text(
                cx,
                &label,
                Point2D::new(
                    chip.origin.x + 30.0,
                    jian_widgets::centered_text_baseline_y(chip, 11.0),
                ),
                11.0,
                fade(palette.ink, 0.75),
            );
            chip_x += chip_w + 6.0;
        }
    }

    // The inline replace-confirm strip rides above the input's bottom.
    if surface.state.replace_pending {
        let strip = shift(layout.replace_strip, rise);
        cx.backend.fill_round_rect(strip, 8.0, palette.raised);
        cx.backend
            .stroke_round_rect(strip, 8.0, fade(palette.blue, 0.4), 1.0);
        text(
            cx,
            copy::home_str(locale, "home.replace.title"),
            Point2D::new(
                strip.origin.x + 12.0,
                jian_widgets::centered_text_baseline_y(strip, 12.0),
            ),
            12.0,
            palette.ink,
        );
        for (rect, hit, label_key, primary) in [
            (
                layout.replace_keep,
                HomeHit::ReplaceKeep,
                "home.replace.keep",
                false,
            ),
            (
                layout.replace_use,
                HomeHit::ReplaceConfirm,
                "home.replace.use",
                true,
            ),
        ] {
            let button = shift(rect, rise);
            let label = copy::home_str(locale, label_key);
            let hovered = surface.state.hover == Some(hit);
            cx.backend.fill_round_rect(
                button,
                7.0,
                if primary {
                    if hovered {
                        palette.blue_hover
                    } else {
                        palette.blue
                    }
                } else if hovered {
                    palette.button_hover
                } else {
                    palette.raised
                },
            );
            if !primary {
                cx.backend
                    .stroke_round_rect(button, 7.0, palette.raised_line, 1.0);
            }
            let label_w = cx.backend.measure_text_family(label, 12.0, SANS);
            text(
                cx,
                label,
                Point2D::new(
                    button.origin.x + (button.size.x - label_w) / 2.0,
                    jian_widgets::centered_text_baseline_y(button, 12.0),
                ),
                12.0,
                if primary { Color::WHITE } else { palette.ink },
            );
        }
    }

    // ── tools row ────────────────────────────────────────────────────
    for (rect, hit, label, icon) in [
        (
            layout.screenshot,
            HomeHit::Attachment,
            copy::home_str(locale, "home.tools.screenshot"),
            Icon::Image,
        ),
        (
            layout.reference_link,
            HomeHit::ReferenceLink,
            copy::home_str(locale, "home.tools.link"),
            Icon::Link,
        ),
    ] {
        paint_tool_button(surface, cx, shift(rect, rise), hit, label, icon, palette);
    }
    let figma = shift(layout.figma, rise);
    let figma_hover = surface.state.hover == Some(HomeHit::Figma);
    let figma_color = if figma_hover {
        fade(palette.ink, 0.5)
    } else {
        fade(palette.ink, 0.72)
    };
    paint_figma_logo(
        cx.backend,
        Point2D::new(figma.origin.x, figma.origin.y + 3.0),
        20.0,
        figma_color,
    );
    text(
        cx,
        copy::home_str(locale, "home.tools.figma"),
        Point2D::new(
            figma.origin.x + 26.0,
            jian_widgets::centered_text_baseline_y(figma, 14.0),
        ),
        14.0,
        figma_color,
    );
    if figma_hover {
        paint_soon_tooltip(surface, cx, figma, palette);
    }

    // ── submit row ───────────────────────────────────────────────────
    super::super::model::paint_model_chip(surface, cx, shift(layout.model_chip, rise), palette);
    let send = shift(layout.send, rise);
    let empty = surface.state.draft.trim().is_empty();
    let connect_mode = !surface.usable_agent;
    let send_hover = surface.state.hover == Some(HomeHit::Send);
    let fill = if connect_mode {
        palette.blue
    } else if empty {
        palette.disabled_primary
    } else if send_hover {
        palette.blue_hover
    } else {
        palette.blue
    };
    let send_pressed = surface.state.pressed == Some(HomeHit::Send);
    let send_rect = if send_pressed {
        Rect::xywh(send.origin.x, send.origin.y + 1.0, send.size.x, send.size.y)
    } else {
        send
    };
    cx.backend.fill_round_rect(send_rect, 9.0, fill);
    if !connect_mode && !empty {
        cx.backend.fill_drop_shadow(
            Rect::xywh(
                send_rect.origin.x + 4.0,
                send_rect.origin.y + 4.0,
                send_rect.size.x - 8.0,
                send_rect.size.y - 4.0,
            ),
            8.0,
            8.0,
            fade(palette.blue, 0.16),
        );
    }
    let send_label = copy::home_str(
        locale,
        if connect_mode {
            "home.submit.connect"
        } else {
            "home.submit.start"
        },
    );
    let label_w = cx.backend.measure_text_family(send_label, 15.0, SANS);
    // White reads on the live blue; on the disabled slab it does not,
    // so the label follows the fill instead of being hardcoded.
    let send_ink = if !connect_mode && empty {
        palette.disabled_primary_ink
    } else {
        Color::WHITE
    };
    text_weighted(
        cx,
        send_label,
        Point2D::new(
            send_rect.origin.x + (send_rect.size.x - label_w - 21.0 - 10.0) / 2.0,
            jian_widgets::centered_text_baseline_y(send_rect, 15.0),
        ),
        15.0,
        send_ink,
        550,
    );
    draw_icon(
        cx.backend,
        Icon::ArrowRight,
        Point2D::new(
            send_rect.origin.x + (send_rect.size.x - label_w - 21.0 - 10.0) / 2.0 + label_w + 10.0,
            send_rect.origin.y + (send_rect.size.y - 21.0) / 2.0,
        ),
        21.0,
        send_ink,
        2.0,
    );
}

#[allow(clippy::too_many_arguments)]
fn paint_tool_button(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    rect: Rect,
    hit: HomeHit,
    label: &str,
    icon: Icon,
    palette: StudioPalette,
) {
    let hovered = surface.state.hover == Some(hit);
    let disabled = matches!(hit, HomeHit::ReferenceLink | HomeHit::Figma);
    let color = if disabled && hovered {
        fade(palette.ink, 0.5)
    } else if hovered {
        palette.blue
    } else {
        fade(palette.ink, 0.72)
    };
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(rect.origin.x, rect.origin.y + 5.0),
        18.0,
        color,
        1.6,
    );
    text(
        cx,
        label,
        Point2D::new(
            rect.origin.x + 26.0,
            jian_widgets::centered_text_baseline_y(rect, 14.0),
        ),
        14.0,
        color,
    );
    if disabled && hovered {
        paint_soon_tooltip(surface, cx, rect, palette);
    }
}

fn paint_soon_tooltip(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    anchor: Rect,
    palette: StudioPalette,
) {
    let label = copy::home_str(surface.ui.locale, "home.tools.soon");
    let w = cx.backend.measure_text_family(label, 11.0, SANS) + 20.0;
    let tooltip = Rect::xywh(anchor.origin.x, anchor.origin.y - 28.0, w, 22.0);
    cx.backend
        .fill_round_rect(tooltip, 7.0, palette.tooltip_fill);
    text(
        cx,
        label,
        Point2D::new(
            tooltip.origin.x + 10.0,
            jian_widgets::centered_text_baseline_y(tooltip, 11.0),
        ),
        11.0,
        palette.tooltip_ink,
    );
}

pub(super) fn paint_preview(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &HomeLayout,
    rise: f32,
    alpha: f32,
) {
    let palette = fade_palette(
        StudioPalette::for_mode(surface.ui.effective_theme_mode()),
        alpha,
    );
    let locale = surface.ui.locale;
    let panel = shift(layout.preview, rise);
    cx.backend.fill_round_rect(panel, 14.0, palette.preview);
    cx.backend
        .stroke_round_rect(panel, 14.0, palette.preview_line, 1.0);

    let heading = shift(layout.preview_heading, rise);
    let task_copy = copy::task_copy(locale, surface.state.task, surface.state.task_draft());
    // Eyebrow with letter-spacing (10 px, 0.7 tracking).
    let eyebrow = copy::home_str(locale, "home.preview.eyebrow");
    let mut x = heading.origin.x;
    for character in eyebrow.chars() {
        let glyph = character.to_string();
        text(
            cx,
            &glyph,
            Point2D::new(x, heading.origin.y + 11.0),
            10.0,
            palette.eyebrow,
        );
        x += cx.backend.measure_text_family(&glyph, 10.0, SANS) + 0.7;
    }
    text_weighted(
        cx,
        task_copy.example_title,
        Point2D::new(heading.origin.x, heading.origin.y + 33.0),
        17.0,
        palette.ink,
        650,
    );
    text(
        cx,
        task_copy.example_desc,
        Point2D::new(heading.origin.x, heading.origin.y + 54.0),
        11.0,
        palette.preview_desc,
    );
    // The 示例 sticker sits at the heading's top-right.
    paint_sticker(
        cx,
        Point2D::new(heading.origin.x + heading.size.x, heading.origin.y + 2.0),
        copy::home_str(locale, "home.preview.sticker"),
        palette,
    );

    // Art area with the 300 ms art-in on task switch (prototype:
    // opacity .2→1, rise 8 px, scale .985→1). The App path owns its
    // motion inside `paint_app_art`; the template paths share the
    // wrapper below.
    let art = shift(layout.preview_art, rise);
    let phase = art_phase(
        surface.ui.motion_stamp(surface.state.art_switched_at_ms),
        surface.now_ms,
    );
    let draft = surface.state.task_draft();
    match surface.state.task {
        HomeFamily::AppUi => paint_app_art(cx, art, palette, draft.device, phase),
        HomeFamily::Presentation => {
            let aspect = match draft.ratio {
                SlideRatio::Wide169 => Some(16.0 / 9.0),
                SlideRatio::Classic43 => Some(4.0 / 3.0),
            };
            paint_template_switched(
                cx,
                art,
                preview_template_for(HomeFamily::Presentation, InfoKind::Data),
                palette,
                aspect,
                phase,
            );
        }
        family => paint_template_switched(
            cx,
            art,
            preview_template_for(family, draft.info_kind),
            palette,
            None,
            phase,
        ),
    }

    // Footer: pages text left, 使用这个示例 / 回到工作区 link right.
    let footer = shift(layout.preview_footer, rise);
    text(
        cx,
        task_copy.example_pages,
        Point2D::new(
            footer.origin.x,
            jian_widgets::centered_text_baseline_y(footer, 12.0),
        ),
        12.0,
        palette.preview_footer,
    );
    // While a workspace is active, the link returns to it — the same
    // rect, a different intent (HomeHit::BackToWorkspace).
    let back_to_workspace = surface.ui.workspace.active;
    let use_hit = surface.state.hover
        == Some(if back_to_workspace {
            HomeHit::BackToWorkspace
        } else {
            HomeHit::UseExample
        });
    let use_label = if back_to_workspace {
        copy::home_str(locale, "workspace.backToWorkspace")
    } else {
        copy::home_str(locale, "home.preview.use")
    };
    let use_rect = shift(layout.use_example, rise);
    let label_w = cx.backend.measure_text_family(use_label, 12.0, SANS);
    text(
        cx,
        use_label,
        Point2D::new(
            use_rect.origin.x + use_rect.size.x - label_w - 15.0,
            jian_widgets::centered_text_baseline_y(use_rect, 12.0),
        ),
        12.0,
        if use_hit {
            palette.blue_hover
        } else {
            palette.blue
        },
    );
    draw_icon(
        cx.backend,
        Icon::ArrowUpRight,
        Point2D::new(
            use_rect.origin.x + use_rect.size.x - 13.0,
            use_rect.origin.y + 5.0,
        ),
        14.0,
        if use_hit {
            palette.blue_hover
        } else {
            palette.blue
        },
        1.6,
    );
    if use_hit {
        cx.backend.stroke_line(
            Point2D::new(
                use_rect.origin.x + use_rect.size.x - label_w - 15.0,
                footer.origin.y + 20.0,
            ),
            Point2D::new(
                use_rect.origin.x + use_rect.size.x - 15.0 + 13.0,
                footer.origin.y + 20.0,
            ),
            palette.blue_hover,
            1.0,
        );
    }
}

fn shift(rect: Rect, dy: f32) -> Rect {
    Rect::xywh(rect.origin.x, rect.origin.y + dy, rect.size.x, rect.size.y)
}

/// A template preview under the art-in switch motion: rise 8 px and
/// scale .985→1 around the art centre, image opacity .2→1.
fn paint_template_switched(
    cx: &mut PaintCx<'_>,
    art: Rect,
    template_id: &str,
    palette: StudioPalette,
    crop_aspect: Option<f32>,
    phase: f32,
) {
    let art = shift(art, (1.0 - phase) * 8.0);
    let alpha = 0.2 + 0.8 * phase;
    let scale = 0.985 + 0.015 * phase;
    cx.backend.save();
    cx.backend.scale(
        Point2D::new(scale, scale),
        Point2D::new(
            art.origin.x + art.size.x / 2.0,
            art.origin.y + art.size.y / 2.0,
        ),
    );
    paint_template_paper(cx, art, template_id, palette, crop_aspect, alpha);
    cx.backend.restore();
}

fn fade_palette(palette: StudioPalette, factor: f32) -> StudioPalette {
    palette.faded(factor)
}
