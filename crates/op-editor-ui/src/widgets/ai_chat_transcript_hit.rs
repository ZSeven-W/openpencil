use super::ai_chat_transcript::build_transcript_with_design_hover;
use crate::{Point2D, Rect};
use op_editor_core::chat::ChatMessage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriptHit {
    ToggleThinking(usize),
    ToggleToolCalls(usize),
    SetToolCallCardExpanded(usize, usize, bool),
    SetDesignBlockExpanded(usize, usize, bool),
    CopyDesignBlock(String),
    ApplyDesignBlock(usize, String),
}

pub(crate) fn transcript_hit(
    messages: &[ChatMessage],
    body_rect: Rect,
    x: f32,
    y: f32,
    locale: op_editor_core::Locale,
    scroll_offset: f32,
) -> Option<TranscriptHit> {
    if !(body_rect).contains(Point2D::new(x, y)) {
        return None;
    }
    for item in build_transcript_with_design_hover(messages, body_rect, locale, None, scroll_offset)
    {
        if let Some(t) = &item.thinking {
            if (t.header).contains(Point2D::new(x, y)) {
                return Some(TranscriptHit::ToggleThinking(item.msg_index));
            }
        }
        if let Some(t) = &item.tools {
            if (t.header).contains(Point2D::new(x, y)) {
                return Some(TranscriptHit::ToggleToolCalls(item.msg_index));
            }
            for (tool_index, card) in t.cards.iter().enumerate() {
                if (card.header).contains(Point2D::new(x, y)) {
                    return Some(TranscriptHit::SetToolCallCardExpanded(
                        item.msg_index,
                        tool_index,
                        !card.expanded,
                    ));
                }
            }
        }
        for (block_index, block) in item.design_blocks.iter().enumerate() {
            if let Some(apply) = block.apply {
                if (apply).contains(Point2D::new(x, y)) {
                    return Some(TranscriptHit::ApplyDesignBlock(
                        item.msg_index,
                        block.code.clone(),
                    ));
                }
            }
            if (block.copy).contains(Point2D::new(x, y)) {
                return Some(TranscriptHit::CopyDesignBlock(block.code.clone()));
            }
            if (block.header).contains(Point2D::new(x, y)) {
                return Some(TranscriptHit::SetDesignBlockExpanded(
                    item.msg_index,
                    block_index,
                    !block.expanded,
                ));
            }
        }
    }
    None
}

pub(crate) fn design_block_at(
    messages: &[ChatMessage],
    body_rect: Rect,
    x: f32,
    y: f32,
    locale: op_editor_core::Locale,
    scroll_offset: f32,
) -> Option<(usize, usize)> {
    if !(body_rect).contains(Point2D::new(x, y)) {
        return None;
    }
    for item in build_transcript_with_design_hover(messages, body_rect, locale, None, scroll_offset)
    {
        for (block_index, block) in item.design_blocks.iter().enumerate() {
            if (block.rect).contains(Point2D::new(x, y)) {
                return Some((item.msg_index, block_index));
            }
        }
    }
    None
}
