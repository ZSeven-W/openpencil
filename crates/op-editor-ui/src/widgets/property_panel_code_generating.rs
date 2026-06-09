use super::code_i18n::CodePanelStrings;
use super::{action_hovered, code_neutral_hover_color, paint_full_button, FullButtonStyle};
use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel_inputs::{to_jian_color, INPUT_HEIGHT, PAD_X};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::codegen::{ChunkStatus, CodegenHover, CodegenState};

const CARD_RADIUS: f32 = 12.0;
const CARD_PAD_Y: f32 = 14.0;
const HEADER_H: f32 = 34.0;
const STEP_H: f32 = 28.0;
const CANCEL_GAP: f32 = 12.0;
const CARD_BOTTOM_PAD: f32 = 14.0;

pub(super) fn generating_cancel_y(state: &CodegenState, y: f32) -> f32 {
    y + CARD_PAD_Y + HEADER_H + step_count(state) as f32 * STEP_H + CANCEL_GAP
}

pub(super) fn paint_generating_body(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &CodegenState,
    strings: CodePanelStrings,
    x: f32,
    y: f32,
    w: f32,
    now_ms: u64,
) -> f32 {
    let card = progress_card_rect(state, x, y, w);
    cx.backend.fill_round_rect(card, CARD_RADIUS, theme.muted);
    cx.backend
        .stroke_round_rect(card, CARD_RADIUS, theme.border, 1.0);
    paint_header(cx, theme, state, strings, card);

    let mut row_y = card.origin.y + CARD_PAD_Y + HEADER_H;
    paint_step(
        cx,
        theme,
        StepPaint {
            rect: step_rect(card, row_y),
            label: strings.planning(),
            status: phase_status(state.progress.planning_done),
            first: true,
            last: step_count(state) == 1,
            now_ms,
            strings,
        },
    );
    row_y += STEP_H;

    for chunk in &state.progress.chunks {
        paint_step(
            cx,
            theme,
            StepPaint {
                rect: step_rect(card, row_y),
                label: &chunk.name,
                status: chunk_status(chunk.status),
                first: false,
                last: false,
                now_ms,
                strings,
            },
        );
        row_y += STEP_H;
    }

    paint_step(
        cx,
        theme,
        StepPaint {
            rect: step_rect(card, row_y),
            label: strings.assembly(),
            status: phase_status(state.progress.assembly_done),
            first: false,
            last: true,
            now_ms,
            strings,
        },
    );

    paint_full_button(
        cx,
        theme,
        strings.cancel(),
        x,
        generating_cancel_y(state, y),
        w,
        FullButtonStyle {
            filled: false,
            hovered: action_hovered(state, CodegenHover::Cancel),
        },
    );
    card.origin.y + card.size.y
}

fn progress_card_rect(state: &CodegenState, x: f32, y: f32, w: f32) -> Rect {
    let cancel_y = generating_cancel_y(state, y);
    Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(
            w - PAD_X * 2.0,
            cancel_y - y + INPUT_HEIGHT + CARD_BOTTOM_PAD,
        ),
    }
}

fn step_count(state: &CodegenState) -> usize {
    2 + state.progress.chunks.len()
}

fn step_rect(card: Rect, y: f32) -> Rect {
    Rect {
        origin: Point2D::new(card.origin.x + 10.0, y),
        size: Point2D::new(card.size.x - 20.0, STEP_H),
    }
}

fn paint_header(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &CodegenState,
    strings: CodePanelStrings,
    card: Rect,
) {
    let badge = Rect {
        origin: Point2D::new(card.origin.x + 12.0, card.origin.y + 12.0),
        size: Point2D::new(24.0, 24.0),
    };
    cx.backend
        .fill_round_rect(badge, 8.0, code_neutral_hover_color(theme));
    draw_icon(
        cx.backend,
        Icon::Sparkles,
        Point2D::new(badge.origin.x + 6.0, badge.origin.y + 6.0),
        12.0,
        theme.primary,
        1.7,
    );
    draw_text(
        cx,
        &strings.generating_framework(state.framework),
        12.0,
        theme.foreground,
        card.origin.x + 44.0,
        card.origin.y + 20.0,
    );
    draw_text(
        cx,
        strings.preparing_production(),
        10.0,
        theme.muted_foreground,
        card.origin.x + 44.0,
        card.origin.y + 35.0,
    );
}

#[derive(Debug, Clone, Copy)]
struct StepPaint<'a> {
    rect: Rect,
    label: &'a str,
    status: StepStatus,
    first: bool,
    last: bool,
    now_ms: u64,
    strings: CodePanelStrings,
}

#[derive(Debug, Clone, Copy)]
enum StepStatus {
    Waiting,
    Running,
    Done,
    Failed,
    Skipped,
}

fn phase_status(done: Option<bool>) -> StepStatus {
    match done {
        None => StepStatus::Waiting,
        Some(false) => StepStatus::Running,
        Some(true) => StepStatus::Done,
    }
}

fn chunk_status(status: ChunkStatus) -> StepStatus {
    match status {
        ChunkStatus::Pending => StepStatus::Waiting,
        ChunkStatus::Running => StepStatus::Running,
        ChunkStatus::Done => StepStatus::Done,
        ChunkStatus::Degraded | ChunkStatus::Failed => StepStatus::Failed,
        ChunkStatus::Skipped => StepStatus::Skipped,
    }
}

fn paint_step(cx: &mut PaintCx<'_>, theme: &Theme, step: StepPaint<'_>) {
    let center_x = step.rect.origin.x + 9.0;
    let center_y = step.rect.origin.y + STEP_H / 2.0;
    if !step.first {
        cx.backend.stroke_line(
            Point2D::new(center_x, step.rect.origin.y),
            Point2D::new(center_x, center_y - 8.0),
            theme.border,
            1.0,
        );
    }
    if !step.last {
        cx.backend.stroke_line(
            Point2D::new(center_x, center_y + 8.0),
            Point2D::new(center_x, step.rect.origin.y + STEP_H),
            theme.border,
            1.0,
        );
    }

    let (icon, icon_color, label_color, status_label) =
        step_style(theme, step.status, step.strings);
    paint_step_icon(cx, icon, icon_color, Point2D::new(center_x, center_y), step);
    draw_text(
        cx,
        step.label,
        12.0,
        label_color,
        step.rect.origin.x + 30.0,
        center_y + 4.0,
    );
    let status_w = cx.backend.measure_text(status_label, 10.0);
    draw_text(
        cx,
        status_label,
        10.0,
        theme.muted_foreground,
        step.rect.origin.x + step.rect.size.x - status_w,
        center_y + 3.0,
    );
}

fn paint_step_icon(
    cx: &mut PaintCx<'_>,
    icon: Icon,
    color: Color,
    center: Point2D,
    step: StepPaint<'_>,
) {
    let top_left = Point2D::new(center.x - 7.0, center.y - 7.0);
    if matches!(step.status, StepStatus::Running) {
        cx.backend.save();
        cx.backend
            .rotate(loader_rotation_radians(step.now_ms), center);
        draw_icon(cx.backend, icon, top_left, 14.0, color, 1.8);
        cx.backend.restore();
    } else {
        draw_icon(cx.backend, icon, top_left, 14.0, color, 1.8);
    }
}

fn loader_rotation_radians(now_ms: u64) -> f32 {
    const PERIOD_MS: f32 = 900.0;
    (now_ms as f32 % PERIOD_MS) / PERIOD_MS * std::f32::consts::TAU
}

fn step_style(
    theme: &Theme,
    status: StepStatus,
    strings: CodePanelStrings,
) -> (Icon, Color, Color, &'static str) {
    match status {
        StepStatus::Waiting => (
            Icon::Minus,
            theme.muted_foreground,
            theme.muted_foreground,
            strings.waiting(),
        ),
        StepStatus::Running => (
            Icon::Loader,
            theme.primary,
            theme.foreground,
            strings.running(),
        ),
        StepStatus::Done => (Icon::Check, theme.primary, theme.foreground, strings.done()),
        StepStatus::Failed => (
            Icon::AlertTriangle,
            theme.destructive,
            theme.destructive,
            strings.issue(),
        ),
        StepStatus::Skipped => (
            Icon::Minus,
            theme.muted_foreground,
            theme.muted_foreground,
            strings.skipped(),
        ),
    }
}

fn draw_text(cx: &mut PaintCx<'_>, text: &str, size: f32, color: Color, x: f32, baseline_y: f32) {
    let layout = TextLayout::single_run(
        text,
        "system-ui",
        size,
        to_jian_color(color),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&layout, Point2D::new(x, baseline_y));
}
