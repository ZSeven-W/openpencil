//! Fixed "Pencil it out" checklist for AI design progress.
//!
//! TS pins this process block between transcript and input instead
//! of letting step rows scroll away with assistant messages.

use super::ai_chat_panel::{to_jian_color, PAD};
use super::ai_chat_transcript_design::extract_design_json_blocks;
use super::ai_chat_transcript_steps::{
    extract_step_blocks, split_design_progress, ParsedStep, ParsedStepStatus,
};
use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::chat::{ChatMessage, ChatRole};

pub(crate) const PROGRESS_H: f32 = 2.0;
pub(crate) const HEADER_H: f32 = 32.0;
const ITEM_H: f32 = 22.0;
const ITEM_GAP: f32 = 1.0;
const DETAIL_GAP: f32 = 2.0;
const DETAIL_LINE_H: f32 = 14.0;
const BOTTOM_PAD: f32 = 8.0;
const MAX_LIST_H: f32 = 144.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChecklistItem {
    pub label: String,
    pub done: bool,
    pub active: bool,
    pub failed: bool,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DetailStatus {
    Done,
    Pending,
    Error,
}

#[derive(Debug, Clone, Copy)]
struct ChecklistProgress {
    streaming: bool,
    total: usize,
    has_explicit_status: bool,
    has_terminal_result: bool,
    json_block_count: usize,
    use_progress_position_fallback: bool,
}

pub(crate) fn fixed_checklist_items(messages: &[ChatMessage]) -> Vec<ChecklistItem> {
    let Some(message) = messages
        .iter()
        .rev()
        .find(|msg| msg.role == ChatRole::Assistant)
    else {
        return Vec::new();
    };

    let mut steps: Vec<ParsedStep> = extract_step_blocks(&message.content, message.streaming)
        .steps
        .into_iter()
        .filter(|step| !step.title.eq_ignore_ascii_case("Thinking"))
        .collect();
    let mut use_progress_position_fallback = false;
    if steps.is_empty() {
        steps = split_design_progress(&message.thinking)
            .0
            .into_iter()
            .filter(|step| !step.title.eq_ignore_ascii_case("Thinking"))
            .collect();
        use_progress_position_fallback = !steps.is_empty();
    }
    if steps.is_empty() {
        return Vec::new();
    }

    let json_block_count = extract_design_json_blocks(&message.content, message.streaming)
        .blocks
        .len();
    let is_applied = message.content.contains('✅')
        || message.content.contains("<!-- APPLIED -->")
        || message.content.contains("[done] Applied");
    let has_error = message.content.to_ascii_lowercase().contains("**error:**");
    let has_explicit_status = steps.iter().any(|step| step.status.is_some());
    let has_terminal_result =
        !message.streaming && !has_error && (is_applied || json_block_count > 0);
    let progress = ChecklistProgress {
        streaming: message.streaming,
        total: steps.len(),
        has_explicit_status,
        has_terminal_result,
        json_block_count,
        use_progress_position_fallback,
    };
    let items: Vec<ChecklistItem> = steps
        .iter()
        .enumerate()
        .map(|(index, step)| {
            let (done, active, failed) = item_state(step, index, progress);
            ChecklistItem {
                label: step.title.clone(),
                done,
                active,
                failed,
                details: step.details.clone(),
            }
        })
        .collect();

    let completed = items.iter().filter(|item| item.done).count();
    let failed = items.iter().any(|item| item.failed);
    if !message.streaming && completed == 0 && !failed {
        Vec::new()
    } else {
        items
    }
}

pub(crate) fn fixed_checklist_height(messages: &[ChatMessage], collapsed: bool) -> f32 {
    let items = fixed_checklist_items(messages);
    let count = items.len();
    if count == 0 {
        return 0.0;
    }
    if collapsed {
        return PROGRESS_H + HEADER_H;
    }
    let list_h = item_list_height(&items).min(MAX_LIST_H);
    PROGRESS_H + HEADER_H + list_h + BOTTOM_PAD
}

pub(crate) fn fixed_checklist_content_height(messages: &[ChatMessage]) -> f32 {
    let items = fixed_checklist_items(messages);
    item_list_height(&items)
}

pub(crate) fn fixed_checklist_max_scroll(messages: &[ChatMessage], collapsed: bool) -> f32 {
    if collapsed {
        return 0.0;
    }
    let content_h = fixed_checklist_content_height(messages);
    (content_h - content_h.min(MAX_LIST_H)).max(0.0)
}

pub(crate) fn fixed_checklist_rect(panel_rect: Rect, input_h: f32, height: f32) -> Rect {
    let bottom = panel_rect.origin.y + panel_rect.size.y - input_h;
    Rect::xywh(
        panel_rect.origin.x,
        bottom - height,
        panel_rect.size.x,
        height,
    )
}

pub(crate) fn fixed_checklist_list_rect(rect: Rect) -> Rect {
    let top = rect.origin.y + PROGRESS_H + HEADER_H;
    let bottom = rect.origin.y + rect.size.y - BOTTOM_PAD;
    Rect::xywh(rect.origin.x, top, rect.size.x, (bottom - top).max(0.0))
}

pub(crate) fn paint_fixed_checklist(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    messages: &[ChatMessage],
    collapsed: bool,
    scroll: f32,
) {
    let items = fixed_checklist_items(messages);
    if items.is_empty() {
        return;
    }

    cx.backend.save();
    cx.backend.clip_rect(rect);
    cx.backend.fill_rect(rect, with_alpha(theme.popover, 0.96));
    cx.backend.fill_rect(
        Rect::xywh(rect.origin.x, rect.origin.y, rect.size.x, PROGRESS_H),
        with_alpha(theme.muted, 0.55),
    );
    let completed = items.iter().filter(|item| item.done).count();
    let progress = completed as f32 / items.len() as f32;
    cx.backend.fill_rect(
        Rect::xywh(
            rect.origin.x,
            rect.origin.y,
            rect.size.x * progress,
            PROGRESS_H,
        ),
        theme.primary,
    );
    cx.backend.fill_rect(
        Rect::xywh(rect.origin.x, rect.origin.y, rect.size.x, 1.0),
        theme.border,
    );

    let header_y = rect.origin.y + PROGRESS_H;
    draw_icon(
        cx.backend,
        Icon::Pencil,
        Point2D::new(rect.origin.x + PAD, header_y + 9.0),
        13.0,
        theme.muted_foreground,
        1.5,
    );
    draw_label(
        cx,
        "Pencil it out",
        12.0,
        theme.foreground,
        rect.origin.x + PAD + 20.0,
        header_y + 20.0,
    );

    let counter = format!("{completed}/{}", items.len());
    let counter_w = cx.backend.measure_text(&counter, 10.0);
    draw_label(
        cx,
        &counter,
        10.0,
        theme.muted_foreground,
        rect.origin.x + rect.size.x - PAD - counter_w - 20.0,
        header_y + 19.0,
    );
    draw_icon(
        cx.backend,
        if collapsed {
            Icon::ChevronDown
        } else {
            Icon::ChevronUp
        },
        Point2D::new(rect.origin.x + rect.size.x - PAD - 13.0, header_y + 10.0),
        12.0,
        theme.muted_foreground,
        1.4,
    );

    if collapsed {
        cx.backend.restore();
        return;
    }

    let list_rect = fixed_checklist_list_rect(rect);
    let list_bottom = list_rect.origin.y + list_rect.size.y;
    let scroll = scroll.clamp(0.0, fixed_checklist_max_scroll(messages, collapsed));
    cx.backend.save();
    cx.backend.clip_rect(list_rect);
    let mut y = list_rect.origin.y - scroll;
    for item in &items {
        let height = item_height(item);
        if y + height >= list_rect.origin.y && y <= list_bottom {
            paint_item(
                cx,
                theme,
                item,
                rect.origin.x + PAD,
                y,
                rect.size.x - PAD * 2.0,
            );
        }
        y += height + ITEM_GAP;
    }
    cx.backend.restore();
    cx.backend.restore();
}

fn item_list_height(items: &[ChecklistItem]) -> f32 {
    if items.is_empty() {
        0.0
    } else {
        items.iter().map(item_height).sum::<f32>()
            + (items.len().saturating_sub(1) as f32 * ITEM_GAP)
    }
}

fn item_height(item: &ChecklistItem) -> f32 {
    if item.details.is_empty() {
        ITEM_H
    } else {
        ITEM_H + DETAIL_GAP + item.details.len() as f32 * DETAIL_LINE_H
    }
}

fn item_state(step: &ParsedStep, index: usize, progress: ChecklistProgress) -> (bool, bool, bool) {
    if progress.has_explicit_status {
        return match step.status {
            Some(ParsedStepStatus::Done) => (true, false, false),
            Some(ParsedStepStatus::Error) => (false, false, true),
            Some(ParsedStepStatus::Streaming) => (false, progress.streaming, false),
            Some(ParsedStepStatus::Pending) | None => (false, false, false),
        };
    }

    if progress.has_terminal_result {
        return (true, false, false);
    }

    if progress.use_progress_position_fallback {
        let done = !progress.streaming || index + 1 < progress.total;
        let active = progress.streaming && index + 1 == progress.total && !done;
        return (done, active, false);
    }

    let done = index < progress.json_block_count;
    let active =
        progress.streaming && !done && index == progress.json_block_count && index < progress.total;
    (done, active, false)
}

fn paint_item(cx: &mut PaintCx<'_>, theme: &Theme, item: &ChecklistItem, x: f32, y: f32, w: f32) {
    if item.active {
        cx.backend.fill_round_rect(
            Rect::xywh(x, y, w, item_height(item)),
            5.0,
            with_alpha(theme.primary, 0.08),
        );
    }
    let icon_x = x + 2.0;
    let icon_y = y + 5.0;
    if item.done {
        cx.backend.fill_oval(
            Rect::xywh(icon_x, icon_y, 12.0, 12.0),
            with_alpha(theme.primary, 0.18),
        );
        draw_icon(
            cx.backend,
            Icon::Check,
            Point2D::new(icon_x + 2.0, icon_y + 2.0),
            8.0,
            theme.primary,
            2.0,
        );
    } else if item.failed {
        draw_icon(
            cx.backend,
            Icon::AlertTriangle,
            Point2D::new(icon_x, icon_y),
            13.0,
            theme.destructive,
            1.7,
        );
    } else {
        let color = if item.active {
            theme.primary
        } else {
            with_alpha(theme.muted_foreground, 0.35)
        };
        let r = if item.active { 4.0 } else { 3.0 };
        cx.backend.fill_oval(
            Rect::xywh(icon_x + 6.0 - r, icon_y + 6.0 - r, r * 2.0, r * 2.0),
            color,
        );
    }

    let color = if item.done {
        theme.muted_foreground
    } else if item.failed {
        theme.destructive
    } else if item.active {
        theme.foreground
    } else {
        with_alpha(theme.muted_foreground, 0.65)
    };
    draw_label(cx, &item.label, 12.0, color, x + 24.0, y + 15.0);
    if !item.details.is_empty() {
        let mut baseline = y + ITEM_H + DETAIL_GAP + 10.0;
        for detail in &item.details {
            let (status, text) = parse_detail_status(detail);
            let text_x = if let Some(status) = status {
                paint_detail_status(cx, theme, status, x + 25.0, baseline - 8.5);
                x + 39.0
            } else {
                x + 24.0
            };
            draw_label(
                cx,
                text,
                10.0,
                with_alpha(theme.muted_foreground, 0.65),
                text_x,
                baseline,
            );
            baseline += DETAIL_LINE_H;
        }
    }
}

fn parse_detail_status(line: &str) -> (Option<DetailStatus>, &str) {
    for (prefix, status) in [
        ("[done]", DetailStatus::Done),
        ("[pending]", DetailStatus::Pending),
        ("[error]", DetailStatus::Error),
    ] {
        if let Some(rest) = line.strip_prefix(prefix) {
            return (Some(status), rest.trim_start());
        }
    }
    (None, line)
}

fn paint_detail_status(cx: &mut PaintCx<'_>, theme: &Theme, status: DetailStatus, x: f32, y: f32) {
    match status {
        DetailStatus::Done => {
            cx.backend.fill_oval(
                Rect::xywh(x, y, 10.0, 10.0),
                with_alpha(theme.primary, 0.16),
            );
            draw_icon(
                cx.backend,
                Icon::Check,
                Point2D::new(x + 2.0, y + 2.0),
                6.0,
                theme.primary,
                2.0,
            );
        }
        DetailStatus::Pending => {
            cx.backend.fill_oval(
                Rect::xywh(x + 3.0, y + 3.0, 4.0, 4.0),
                with_alpha(theme.primary, 0.7),
            );
        }
        DetailStatus::Error => {
            draw_icon(
                cx.backend,
                Icon::AlertTriangle,
                Point2D::new(x, y - 1.0),
                11.0,
                theme.destructive,
                1.5,
            );
        }
    }
}

fn draw_label(cx: &mut PaintCx<'_>, text: &str, size: f32, color: Color, x: f32, y: f32) {
    let label = TextLayout::single_run(
        text,
        "system-ui",
        size,
        to_jian_color(color),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&label, Point2D::new(x, y));
}

fn with_alpha(color: Color, a: f32) -> Color {
    Color { a, ..color }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_checklist_uses_latest_assistant_step_status() {
        let mut message = ChatMessage::assistant_streaming();
        message.content = r#"<step title="Plan" status="done"></step>
<step title="Draw" status="streaming"></step>"#
            .into();

        let items = fixed_checklist_items(std::slice::from_ref(&message));

        assert_eq!(items.len(), 2);
        assert!(items[0].done);
        assert!(items[1].active);
    }

    #[test]
    fn fixed_checklist_fallback_starts_at_first_step_until_design_json_streams() {
        let mut message = ChatMessage::assistant_streaming();
        message.content = r#"<step title="Plan"></step>
<step title="Draw"></step>"#
            .into();

        let items = fixed_checklist_items(std::slice::from_ref(&message));

        assert_eq!(items.len(), 2);
        assert!(
            items[0].active,
            "TS keeps step[0] active before any JSON block"
        );
        assert!(!items[0].done);
        assert!(!items[1].active);
        assert!(!items[1].done);
    }

    #[test]
    fn fixed_checklist_hides_terminal_plan_without_design_result_like_ts() {
        let message = ChatMessage::assistant(
            r#"<step title="Plan"></step>
<step title="Draw"></step>"#,
        );

        let items = fixed_checklist_items(std::slice::from_ref(&message));

        assert!(
            items.is_empty(),
            "TS hides the checklist after a non-streaming turn with no applied or JSON result"
        );
    }

    #[test]
    fn fixed_checklist_uses_design_session_thinking_progress() {
        let mut message = ChatMessage::assistant_streaming();
        message.thinking = "• Planning…\n• Subtask `hero` — Hero section".into();

        let items = fixed_checklist_items(std::slice::from_ref(&message));

        assert_eq!(items.len(), 2);
        assert!(items[0].done);
        assert!(items[1].active);
        assert_eq!(items[1].label, "Subtask `hero` — Hero section");
    }

    #[test]
    fn fixed_checklist_height_omits_step_rows_when_collapsed() {
        let mut message = ChatMessage::assistant_streaming();
        message.content = r#"<step title="Plan" status="done"></step>
<step title="Draw" status="streaming"></step>"#
            .into();

        let messages = [message];
        let expanded = fixed_checklist_height(&messages, false);
        let collapsed = fixed_checklist_height(&messages, true);

        assert!(expanded > collapsed);
        assert_eq!(collapsed, PROGRESS_H + HEADER_H);
    }

    #[test]
    fn fixed_checklist_keeps_step_detail_lines() {
        let mut message = ChatMessage::assistant_streaming();
        message.content = r#"<step title="Plan" status="done">
[done] Checked constraints
[pending] Choose layout
</step>"#
            .into();

        let items = fixed_checklist_items(std::slice::from_ref(&message));

        assert_eq!(
            items[0].details,
            vec![
                "[done] Checked constraints".to_string(),
                "[pending] Choose layout".to_string()
            ]
        );
    }

    #[test]
    fn fixed_checklist_height_grows_for_step_detail_lines() {
        let mut plain = ChatMessage::assistant_streaming();
        plain.content = r#"<step title="Plan" status="done"></step>"#.into();
        let mut detailed = ChatMessage::assistant_streaming();
        detailed.content = r#"<step title="Plan" status="done">
Checked constraints
Choose layout
</step>"#
            .into();

        let plain_h = fixed_checklist_height(&[plain], false);
        let detailed_h = fixed_checklist_height(&[detailed], false);

        assert!(detailed_h > plain_h);
    }

    #[test]
    fn fixed_checklist_reports_scroll_overflow_for_many_steps() {
        let mut message = ChatMessage::assistant_streaming();
        message.content = (0..11)
            .map(|idx| format!(r#"<step title="Task {idx}" status="done"></step>"#))
            .collect::<Vec<_>>()
            .join("\n");

        let messages = [message];

        assert!(fixed_checklist_max_scroll(&messages, false) > 0.0);
        assert_eq!(fixed_checklist_max_scroll(&messages, true), 0.0);
    }

    #[test]
    fn parse_detail_status_strips_ts_prefixes() {
        assert_eq!(
            parse_detail_status("[done] Checked constraints"),
            (Some(DetailStatus::Done), "Checked constraints")
        );
        assert_eq!(
            parse_detail_status("[pending] Choose layout"),
            (Some(DetailStatus::Pending), "Choose layout")
        );
        assert_eq!(parse_detail_status("Plain detail"), (None, "Plain detail"));
    }
}
