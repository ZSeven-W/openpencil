use super::ai_chat_transcript::build_transcript;
use crate::Rect;
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

fn rect_contains(r: Rect, x: f32, y: f32) -> bool {
    x >= r.origin.x && x <= r.origin.x + r.size.x && y >= r.origin.y && y <= r.origin.y + r.size.y
}

pub(crate) fn transcript_hit(
    messages: &[ChatMessage],
    body_rect: Rect,
    x: f32,
    y: f32,
    locale: op_editor_core::Locale,
) -> Option<TranscriptHit> {
    if !rect_contains(body_rect, x, y) {
        return None;
    }
    for item in build_transcript(messages, body_rect, locale) {
        if let Some(t) = &item.thinking {
            if rect_contains(t.header, x, y) {
                return Some(TranscriptHit::ToggleThinking(item.msg_index));
            }
        }
        if let Some(t) = &item.tools {
            if rect_contains(t.header, x, y) {
                return Some(TranscriptHit::ToggleToolCalls(item.msg_index));
            }
            for (tool_index, card) in t.cards.iter().enumerate() {
                if rect_contains(card.header, x, y) {
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
                if rect_contains(apply, x, y) {
                    return Some(TranscriptHit::ApplyDesignBlock(
                        item.msg_index,
                        block.code.clone(),
                    ));
                }
            }
            if rect_contains(block.copy, x, y) {
                return Some(TranscriptHit::CopyDesignBlock(block.code.clone()));
            }
            if rect_contains(block.header, x, y) {
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
) -> Option<(usize, usize)> {
    if !rect_contains(body_rect, x, y) {
        return None;
    }
    for item in build_transcript(messages, body_rect, locale) {
        for (block_index, block) in item.design_blocks.iter().enumerate() {
            if rect_contains(block.rect, x, y) {
                return Some((item.msg_index, block_index));
            }
        }
    }
    None
}
