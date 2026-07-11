//! Per-message agent identity layout and paint.

use super::ai_chat_transcript::draw_line;
use crate::theme::Theme;
use crate::util::parse_hex_color;
use crate::widgets::PaintCx;
use crate::{Color, Rect};
use op_editor_core::ChatMessage;

const ROW_H: f32 = 18.0;
const ROW_GAP: f32 = 4.0;
const DOT_D: f32 = 5.0;
const DOT_NAME_GAP: f32 = 6.0;
const FONT_SIZE: f32 = 11.0;

pub(crate) struct AgentIdentityHeader {
    pub rect: Rect,
    name: String,
    color: Option<Color>,
}

pub(crate) fn layout_agent_identity(
    message: &ChatMessage,
    x: f32,
    y: f32,
    width: f32,
) -> (Option<AgentIdentityHeader>, f32) {
    let Some(name) = message.agent_name.as_ref() else {
        return (None, y);
    };
    let header = AgentIdentityHeader {
        rect: Rect::xywh(x, y, width, ROW_H),
        name: name.clone(),
        color: message.agent_color.as_deref().and_then(parse_hex_color),
    };
    (Some(header), y + ROW_H + ROW_GAP)
}

pub(crate) fn paint_agent_identity(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    header: &AgentIdentityHeader,
) {
    let dot = Rect::xywh(
        header.rect.origin.x,
        header.rect.origin.y + (header.rect.size.y - DOT_D) / 2.0,
        DOT_D,
        DOT_D,
    );
    cx.backend
        .fill_oval(dot, header.color.unwrap_or(theme.primary));
    draw_line(
        cx,
        &header.name,
        dot.origin.x + DOT_D + DOT_NAME_GAP,
        jian_widgets::centered_text_baseline_y(header.rect, FONT_SIZE),
        FONT_SIZE,
        theme.foreground,
    );
}
