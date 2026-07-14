use super::ai_chat_transcript::ACTION_STEP_H;
use super::ai_chat_transcript_cache::CanonicalTranscript;
use crate::{Point2D, Rect};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriptHit {
    ToggleThinking(usize),
    ToggleToolCalls(usize),
    SetToolCallCardExpanded(usize, usize, bool),
    SetDesignBlockExpanded(usize, usize, bool),
    SetActionStepExpanded(usize, usize, bool),
    CopyDesignBlock(String),
    ApplyDesignBlock(usize, String),
}

pub(crate) fn transcript_hit(
    canonical: &CanonicalTranscript,
    body_rect: Rect,
    x: f32,
    y: f32,
    scroll_offset: f32,
) -> Option<TranscriptHit> {
    if !(body_rect).contains(Point2D::new(x, y)) {
        return None;
    }
    // The canonical layout is scroll 0; the caller resolved it once and passes
    // it in, so shift the query point down by the scroll instead of rebuilding
    // (or re-fingerprinting) a scroll-applied transcript.
    let p = Point2D::new(x, y + scroll_offset);
    for item in &canonical.items {
        if let Some(t) = &item.thinking {
            if (t.header).contains(p) {
                return Some(TranscriptHit::ToggleThinking(item.msg_index));
            }
        }
        if let Some(t) = &item.tools {
            if (t.header).contains(p) {
                return Some(TranscriptHit::ToggleToolCalls(item.msg_index));
            }
            for card in &t.cards {
                if (card.header).contains(p) {
                    return Some(TranscriptHit::SetToolCallCardExpanded(
                        item.msg_index,
                        card.index,
                        !card.expanded,
                    ));
                }
            }
        }
        // Interleaved flow panels are headerless — only their cards toggle,
        // via the ORIGINAL tool index each card carries.
        for panel in &item.flow_panels {
            for card in &panel.cards {
                if (card.header).contains(p) {
                    return Some(TranscriptHit::SetToolCallCardExpanded(
                        item.msg_index,
                        card.index,
                        !card.expanded,
                    ));
                }
            }
        }
        for step in &item.steps {
            // Only the header band toggles — clicking a detail line must not
            // collapse. Detail-less structured activities are status rows, not
            // disclosure controls, so they deliberately have no hit target.
            if step.details.is_empty() {
                continue;
            }
            let header = Rect::xywh(
                step.rect.origin.x,
                step.rect.origin.y,
                step.rect.size.x,
                ACTION_STEP_H,
            );
            if (header).contains(p) {
                return Some(TranscriptHit::SetActionStepExpanded(
                    item.msg_index,
                    step.source_index,
                    !step.expanded,
                ));
            }
        }
        for (block_index, block) in item.design_blocks.iter().enumerate() {
            if let Some(apply) = block.apply {
                if (apply).contains(p) {
                    return Some(TranscriptHit::ApplyDesignBlock(
                        item.msg_index,
                        block.code.clone(),
                    ));
                }
            }
            if (block.copy).contains(p) {
                return Some(TranscriptHit::CopyDesignBlock(block.code.clone()));
            }
            if (block.header).contains(p) {
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
    canonical: &CanonicalTranscript,
    body_rect: Rect,
    x: f32,
    y: f32,
    scroll_offset: f32,
) -> Option<(usize, usize)> {
    if !(body_rect).contains(Point2D::new(x, y)) {
        return None;
    }
    let p = Point2D::new(x, y + scroll_offset);
    for item in &canonical.items {
        for (block_index, block) in item.design_blocks.iter().enumerate() {
            if (block.rect).contains(p) {
                return Some((item.msg_index, block_index));
            }
        }
    }
    None
}
