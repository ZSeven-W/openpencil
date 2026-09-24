//! Immediate-mode paint pass for the phone works reader's chrome: the
//! header (← Home, title + subtitle, 普通 / 专业), the pager, the status
//! line and the fixed bottom bar. The real canvas inside the stage is
//! painted by the host before this pass, and the chat sheet after it.

use super::{ReaderLayout, WorksReader};
use crate::widgets::home_surface::StudioPalette;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::workspace_surface::{family_label, phase_key};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};
use op_editor_core::{ReaderHit, WorkspacePhase};

const SANS: &str = "system-ui";

fn text_weighted(
    cx: &mut PaintCx<'_>,
    content: &str,
    origin: Point2D,
    size: f32,
    color: Color,
    weight: u16,
) {
    let layout = crate::TextLayout::single_run(content, SANS, size, color.to_jian(), Point2D::ZERO)
        .with_font_weight(weight);
    cx.backend.draw_text(&layout, origin);
}

fn fade(color: Color, factor: f32) -> Color {
    Color {
        a: color.a * factor,
        ..color
    }
}

/// Cut `content` (by whole chars, with an ellipsis) until it fits `max_w`.
fn fit_text(cx: &mut PaintCx<'_>, content: &str, size: f32, max_w: f32) -> String {
    if cx.backend.measure_text_family(content, size, SANS) <= max_w {
        return content.to_string();
    }
    let mut chars: Vec<char> = content.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let candidate: String = chars.iter().collect::<String>() + "…";
        if cx.backend.measure_text_family(&candidate, size, SANS) <= max_w {
            return candidate;
        }
    }
    String::new()
}

/// A label centred in `rect` on its baseline.
fn centered_label(
    cx: &mut PaintCx<'_>,
    label: &str,
    rect: Rect,
    size: f32,
    color: Color,
    weight: u16,
) {
    let w = cx.backend.measure_text_family(label, size, SANS);
    text_weighted(
        cx,
        label,
        Point2D::new(
            rect.origin.x + (rect.size.x - w).max(0.0) / 2.0,
            jian_widgets::centered_text_baseline_y(rect, size),
        ),
        size,
        color,
        weight,
    );
}

pub(super) fn paint_reader(reader: &WorksReader<'_>, cx: &mut PaintCx<'_>, rect: Rect) {
    let layout = reader.layout(rect.size.x, rect.size.y);
    let palette = StudioPalette::for_mode(reader.ui.effective_theme_mode());
    if let Some(board) = reader.board_screen {
        paint_stage_mask(cx, layout.stage, board, palette);
    }
    paint_header(reader, cx, &layout, palette);
    if reader.boards.is_empty() {
        paint_empty_stage(reader, cx, &layout, palette);
    }
    paint_pager(reader, cx, &layout, palette);
    paint_status(reader, cx, &layout, palette);
    paint_bottom_bar(reader, cx, &layout, palette);
}

fn paint_header(
    reader: &WorksReader<'_>,
    cx: &mut PaintCx<'_>,
    layout: &ReaderLayout,
    palette: StudioPalette,
) {
    let locale = reader.ui.locale;
    let header = layout.header;
    cx.backend.fill_rect(header, palette.topbar);
    cx.backend.stroke_line(
        Point2D::new(0.0, header.size.y),
        Point2D::new(header.size.x, header.size.y),
        palette.topbar_line,
        1.0,
    );
    let pressed = reader.state.reader_pressed;
    if pressed == Some(ReaderHit::Back) {
        cx.backend
            .fill_round_rect(layout.back, 12.0, palette.button_hover);
    }
    draw_icon(
        cx.backend,
        Icon::ArrowLeft,
        Point2D::new(
            layout.back.origin.x + (layout.back.size.x - 20.0) / 2.0,
            layout.back.origin.y + (layout.back.size.y - 20.0) / 2.0,
        ),
        20.0,
        palette.ink,
        1.8,
    );

    // Title over "family · phase".
    let title = fit_text(cx, &reader.title, 15.0, layout.title.size.x);
    text_weighted(
        cx,
        &title,
        Point2D::new(layout.title.origin.x, layout.title.origin.y + 19.0),
        15.0,
        palette.ink,
        650,
    );
    let subtitle = format!(
        "{} · {}",
        family_label(locale, reader.state.family),
        op_i18n::translate(locale, phase_key(reader.state.phase))
    );
    let subtitle = fit_text(cx, &subtitle, 11.0, layout.title.size.x);
    text_weighted(
        cx,
        &subtitle,
        Point2D::new(layout.title.origin.x, layout.title.origin.y + 36.0),
        11.0,
        palette.muted,
        500,
    );

    // 普通 / 专业 — the same segmented control the compact Home carries,
    // with 普通 selected: the reader IS the normal view.
    let switch = layout.mode_switch;
    cx.backend.fill_round_rect(switch, 10.0, palette.segment_bg);
    cx.backend
        .stroke_round_rect(switch, 10.0, palette.segment_line, 1.0);
    let normal = inset(layout.mode_normal, 3.0);
    cx.backend.fill_round_rect(normal, 7.0, palette.panel);
    centered_label(
        cx,
        op_i18n::translate(locale, "home.mode.normal"),
        layout.mode_normal,
        12.0,
        palette.blue,
        600,
    );
    let professional_color = if pressed == Some(ReaderHit::ModeProfessional) {
        fade(palette.ink, 0.85)
    } else {
        fade(palette.ink, 0.6)
    };
    centered_label(
        cx,
        op_i18n::translate(locale, "home.mode.professional"),
        layout.mode_professional,
        12.0,
        professional_color,
        500,
    );
}

/// Height kept above the board for its frame label (the page's name).
const BOARD_LABEL_BAND: f32 = 28.0;

/// Cover the stage outside the current board with the canvas backdrop:
/// the canvas paints every board, the reader shows one page.
fn paint_stage_mask(cx: &mut PaintCx<'_>, stage: Rect, board: Rect, palette: StudioPalette) {
    let keep_top = (board.origin.y - BOARD_LABEL_BAND).max(stage.origin.y);
    let keep_bottom = (board.origin.y + board.size.y).min(stage.origin.y + stage.size.y);
    let keep_left = board.origin.x.max(stage.origin.x);
    let keep_right = (board.origin.x + board.size.x).min(stage.origin.x + stage.size.x);
    let stage_bottom = stage.origin.y + stage.size.y;
    let stage_right = stage.origin.x + stage.size.x;
    if keep_top >= keep_bottom || keep_left >= keep_right {
        cx.backend.fill_rect(stage, palette.canvas);
        return;
    }
    for rect in [
        Rect::xywh(
            stage.origin.x,
            stage.origin.y,
            stage.size.x,
            keep_top - stage.origin.y,
        ),
        Rect::xywh(
            stage.origin.x,
            keep_bottom,
            stage.size.x,
            stage_bottom - keep_bottom,
        ),
        Rect::xywh(
            stage.origin.x,
            keep_top,
            keep_left - stage.origin.x,
            keep_bottom - keep_top,
        ),
        Rect::xywh(
            keep_right,
            keep_top,
            stage_right - keep_right,
            keep_bottom - keep_top,
        ),
    ] {
        if rect.size.x > 0.0 && rect.size.y > 0.0 {
            cx.backend.fill_rect(rect, palette.canvas);
        }
    }
}

fn inset(rect: Rect, by: f32) -> Rect {
    Rect::xywh(
        rect.origin.x + by,
        rect.origin.y + by,
        (rect.size.x - by * 2.0).max(0.0),
        (rect.size.y - by * 2.0).max(0.0),
    )
}

/// Nothing has landed yet: a quiet note in the middle of the stage, so
/// the first seconds of a run read as "on its way", not as a blank page.
fn paint_empty_stage(
    reader: &WorksReader<'_>,
    cx: &mut PaintCx<'_>,
    layout: &ReaderLayout,
    palette: StudioPalette,
) {
    if reader.state.phase != WorkspacePhase::Generating {
        return;
    }
    let stage = layout.stage;
    let note = op_i18n::translate(reader.ui.locale, "reader.emptyStage");
    let band = Rect::xywh(
        stage.origin.x + 24.0,
        stage.origin.y + stage.size.y / 2.0 - 22.0,
        (stage.size.x - 48.0).max(0.0),
        44.0,
    );
    cx.backend.fill_round_rect(band, 14.0, palette.panel);
    cx.backend.stroke_round_rect(band, 14.0, palette.line, 1.0);
    let fitted = fit_text(cx, note, 13.0, band.size.x - 24.0);
    centered_label(cx, &fitted, band, 13.0, palette.muted, 500);
}

fn paint_pager(
    reader: &WorksReader<'_>,
    cx: &mut PaintCx<'_>,
    layout: &ReaderLayout,
    palette: StudioPalette,
) {
    let Some(row) = layout.pager else {
        return;
    };
    cx.backend.fill_rect(row, palette.page);
    let count = reader.boards.len();
    let current = reader.current_index();
    let label = if count == 0 {
        "– / –".to_string()
    } else {
        format!("{} / {}", current + 1, count)
    };
    if let Some(label_rect) = layout.page_label {
        centered_label(cx, &label, label_rect, 13.0, fade(palette.ink, 0.75), 500);
    }
    for (rect, icon, enabled, hit) in [
        (layout.prev, Icon::ChevronLeft, current > 0, ReaderHit::Prev),
        (
            layout.next,
            Icon::ChevronRight,
            current + 1 < count,
            ReaderHit::Next,
        ),
    ] {
        let Some(rect) = rect else {
            continue;
        };
        if enabled && reader.state.reader_pressed == Some(hit) {
            cx.backend
                .fill_round_rect(inset(rect, 4.0), 10.0, palette.button_hover);
        }
        draw_icon(
            cx.backend,
            icon,
            Point2D::new(
                rect.origin.x + (rect.size.x - 20.0) / 2.0,
                rect.origin.y + (rect.size.y - 20.0) / 2.0,
            ),
            20.0,
            if enabled {
                palette.ink
            } else {
                fade(palette.ink, 0.25)
            },
            1.8,
        );
    }
}

fn paint_status(
    reader: &WorksReader<'_>,
    cx: &mut PaintCx<'_>,
    layout: &ReaderLayout,
    palette: StudioPalette,
) {
    let locale = reader.ui.locale;
    let status = layout.status;
    cx.backend.fill_rect(status, palette.page);
    let phase = reader.state.phase;
    let (icon, color) = match phase {
        WorkspacePhase::Generating => (Icon::Loader, palette.blue),
        WorkspacePhase::Done => (Icon::CheckCircle, palette.status_green),
        WorkspacePhase::Stopped => (Icon::AlertCircle, palette.muted),
        WorkspacePhase::Failed => (Icon::AlertTriangle, Color::rgb_u8(0xE5, 0x48, 0x4D)),
    };
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(16.0, status.origin.y + (status.size.y - 16.0) / 2.0),
        16.0,
        color,
        1.8,
    );
    // A running page edit says which page it is changing; everything
    // else reads the phase.
    let line = match (&reader.state.page_edit_running, phase) {
        (Some(target), WorkspacePhase::Generating) => {
            op_i18n::translate(locale, "reader.editingPage")
                .replace("{{page}}", &target.page_number().to_string())
        }
        _ => op_i18n::translate(locale, phase_key(phase)).to_string(),
    };
    let right = layout
        .status_action
        .map_or(status.size.x - 16.0, |rect| rect.origin.x - 8.0);
    let line = fit_text(cx, &line, 13.0, (right - 40.0).max(0.0));
    text_weighted(
        cx,
        &line,
        Point2D::new(40.0, jian_widgets::centered_text_baseline_y(status, 13.0)),
        13.0,
        fade(palette.ink, 0.8),
        500,
    );
    if let (Some(rect), Some(label), Some(hit)) = (
        layout.status_action,
        reader.status_action_label(),
        reader.status_action(),
    ) {
        let pill = Rect::xywh(rect.origin.x, rect.origin.y + 6.0, rect.size.x, 32.0);
        let pressed = reader.state.reader_pressed == Some(hit);
        cx.backend.fill_round_rect(
            pill,
            16.0,
            if pressed {
                palette.button_hover
            } else {
                palette.raised
            },
        );
        cx.backend
            .stroke_round_rect(pill, 16.0, palette.raised_line, 1.0);
        centered_label(cx, label, pill, 13.0, palette.ink, 600);
    }
}

fn paint_bottom_bar(
    reader: &WorksReader<'_>,
    cx: &mut PaintCx<'_>,
    layout: &ReaderLayout,
    palette: StudioPalette,
) {
    let locale = reader.ui.locale;
    let bar = layout.bottom_bar;
    cx.backend.fill_rect(bar, palette.topbar);
    cx.backend.stroke_line(
        Point2D::new(0.0, bar.origin.y),
        Point2D::new(bar.size.x, bar.origin.y),
        palette.topbar_line,
        1.0,
    );
    let pressed = reader.state.reader_pressed;

    // 继续对话 — the secondary, outlined button with the sparkle mark.
    let chat = layout.continue_chat;
    cx.backend.fill_round_rect(
        chat,
        14.0,
        if pressed == Some(ReaderHit::ContinueChat) {
            palette.button_hover
        } else {
            palette.raised
        },
    );
    cx.backend
        .stroke_round_rect(chat, 14.0, palette.raised_line, 1.0);
    let chat_label = op_i18n::translate(locale, "reader.continueChat");
    let label_w = cx.backend.measure_text_family(chat_label, 14.0, SANS);
    let icon_x = chat.origin.x + (chat.size.x - label_w - 18.0 - 6.0).max(0.0) / 2.0;
    draw_icon(
        cx.backend,
        Icon::Sparkles,
        Point2D::new(icon_x, chat.origin.y + (chat.size.y - 18.0) / 2.0),
        18.0,
        palette.ink,
        1.6,
    );
    text_weighted(
        cx,
        chat_label,
        Point2D::new(
            icon_x + 24.0,
            jian_widgets::centered_text_baseline_y(chat, 14.0),
        ),
        14.0,
        palette.ink,
        600,
    );

    // 改这一页 — the primary action, disabled while a run is live.
    let edit = layout.edit_page;
    let enabled = reader.edit_enabled();
    let fill = if !enabled {
        palette.disabled_primary
    } else if pressed == Some(ReaderHit::EditPage) {
        palette.blue_hover
    } else {
        palette.blue
    };
    cx.backend.fill_round_rect(edit, 14.0, fill);
    let ink = if enabled {
        Color::rgb_u8(0xFF, 0xFF, 0xFF)
    } else {
        palette.disabled_primary_ink
    };
    centered_label(
        cx,
        op_i18n::translate(locale, "reader.editPage"),
        edit,
        14.0,
        ink,
        650,
    );
}
