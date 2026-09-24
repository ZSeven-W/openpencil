//! Geometry + hit-testing for the generation workspace's quality report:
//! the header's `质检：修复 N 处 · 待关注 M 处` chip and the panel it
//! expands into.
//!
//! Pure functions of the report and the chrome layout, so paint (in
//! `workspace_quality_paint.rs`), hit-test and the host all walk the SAME
//! rows. Text widths the geometry depends on are estimated from the string
//! rather than measured — hit-testing has no backend, and a chip whose
//! press area disagreed with its paint is the bug this shape prevents.

use crate::{Point2D, Rect};
use op_editor_core::{QualityReport, WorkspaceHit};

/// Chip font size; the paint uses the same value.
pub const QUALITY_CHIP_FONT: f32 = 12.0;
/// Leading status dot diameter inside the chip.
pub const QUALITY_CHIP_DOT: f32 = 8.0;
/// Horizontal padding inside the chip.
pub const QUALITY_CHIP_PAD_X: f32 = 12.0;
const CHIP_GAP: f32 = 10.0;
const CHIP_MIN_W: f32 = 120.0;
const CHIP_MAX_W: f32 = 320.0;

/// Panel width.
pub const QUALITY_PANEL_W: f32 = 380.0;
/// Inner padding of the panel.
pub const QUALITY_PANEL_PAD: f32 = 14.0;
/// Title + summary block at the top of the panel.
pub const QUALITY_PANEL_HEAD_H: f32 = 56.0;
const PANEL_MARGIN: f32 = 12.0;
const TOPIC_H: f32 = 30.0;
const ITEM_H: f32 = 24.0;
const BOARDS_HEADER_H: f32 = 30.0;
/// Remaining issues listed per topic before the rest fold into a
/// "N more" row.
const REMAINING_CAP: usize = 4;
/// Fixed items listed per topic — they are the reassurance, the remaining
/// issues are the work, so fewer are shown.
const FIXED_CAP: usize = 2;

/// Deterministic advance estimate for `text` at `size`: CJK and other
/// wide glyphs at one em, ASCII at a little over half. Slightly generous so
/// the painted label always fits the rect the estimate produced.
pub fn estimate_text_width(text: &str, size: f32) -> f32 {
    text.chars()
        .map(|c| if c.is_ascii() { 0.6 } else { 1.0 })
        .sum::<f32>()
        * size
}

/// The chip, right-aligned just left of the header's Export button.
pub fn quality_chip_rect(export: Rect, label: &str) -> Rect {
    let width = (QUALITY_CHIP_PAD_X * 2.0
        + QUALITY_CHIP_DOT
        + 8.0
        + estimate_text_width(label, QUALITY_CHIP_FONT))
    .clamp(CHIP_MIN_W, CHIP_MAX_W);
    Rect::xywh(
        export.origin.x - CHIP_GAP - width,
        export.origin.y,
        width,
        export.size.y,
    )
}

/// What one panel row shows. Indices point into `QualityReport::topics`
/// (and that topic's lists) or `QualityReport::boards`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityRowKind {
    /// A topic header: label + its counts (or "passed").
    Topic { topic: usize },
    /// An issue that still needs attention — clickable when it names a node.
    Remaining { topic: usize, item: usize },
    /// A fix that was applied.
    Fixed { topic: usize, item: usize },
    /// Fixes that were counted but not itemized.
    CountOnly { topic: usize },
    /// `count` further items folded away.
    More { count: usize },
    /// The per-board section heading.
    BoardsHeader,
    /// One board's totals.
    Board { index: usize },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QualityRow {
    pub kind: QualityRowKind,
    pub rect: Rect,
}

/// The expanded panel: its frame, the head block, and every row that fits.
#[derive(Debug, Clone, PartialEq)]
pub struct QualityPanelLayout {
    pub panel: Rect,
    pub head: Rect,
    pub rows: Vec<QualityRow>,
    /// Rows were cut at the panel's maximum height.
    pub truncated: bool,
}

/// Lay the panel out under `chip`, clamped into the viewport.
pub fn quality_panel_layout(
    chip: Rect,
    viewport_w: f32,
    viewport_h: f32,
    report: &QualityReport,
) -> QualityPanelLayout {
    let max_x = (viewport_w - QUALITY_PANEL_W - PANEL_MARGIN).max(PANEL_MARGIN);
    let x = (chip.origin.x + chip.size.x - QUALITY_PANEL_W).clamp(PANEL_MARGIN, max_x);
    let y = chip.origin.y + chip.size.y + 8.0;
    let bottom_limit = (viewport_h - PANEL_MARGIN).max(y + QUALITY_PANEL_HEAD_H);
    let inner_x = x + QUALITY_PANEL_PAD;
    let inner_w = QUALITY_PANEL_W - QUALITY_PANEL_PAD * 2.0;

    let mut rows = Vec::new();
    let mut cursor = y + QUALITY_PANEL_HEAD_H;
    let mut truncated = false;
    let mut push = |kind: QualityRowKind, height: f32| -> bool {
        if truncated || cursor + height + QUALITY_PANEL_PAD > bottom_limit {
            truncated = true;
            return false;
        }
        rows.push(QualityRow {
            kind,
            rect: Rect::xywh(inner_x, cursor, inner_w, height),
        });
        cursor += height;
        true
    };

    for (topic_index, topic) in report.topics.iter().enumerate() {
        if !topic.checked && topic.found_count() == 0 {
            continue;
        }
        push(QualityRowKind::Topic { topic: topic_index }, TOPIC_H);
        for item in 0..topic.remaining.len().min(REMAINING_CAP) {
            push(
                QualityRowKind::Remaining {
                    topic: topic_index,
                    item,
                },
                ITEM_H,
            );
        }
        if topic.remaining.len() > REMAINING_CAP {
            push(
                QualityRowKind::More {
                    count: topic.remaining.len() - REMAINING_CAP,
                },
                ITEM_H,
            );
        }
        for item in 0..topic.fixed.len().min(FIXED_CAP) {
            push(
                QualityRowKind::Fixed {
                    topic: topic_index,
                    item,
                },
                ITEM_H,
            );
        }
        if topic.fixed.len() > FIXED_CAP {
            push(
                QualityRowKind::More {
                    count: topic.fixed.len() - FIXED_CAP,
                },
                ITEM_H,
            );
        }
        if topic.fixed_without_detail > 0 {
            push(QualityRowKind::CountOnly { topic: topic_index }, ITEM_H);
        }
    }
    if !report.boards.is_empty() {
        push(QualityRowKind::BoardsHeader, BOARDS_HEADER_H);
        for index in 0..report.boards.len() {
            push(QualityRowKind::Board { index }, ITEM_H);
        }
    }

    let height = cursor - y + QUALITY_PANEL_PAD;
    QualityPanelLayout {
        panel: Rect::xywh(x, y, QUALITY_PANEL_W, height),
        head: Rect::xywh(inner_x, y, inner_w, QUALITY_PANEL_HEAD_H),
        rows,
        truncated,
    }
}

impl QualityPanelLayout {
    /// Hit-test the open panel. A remaining-issue row answers with its
    /// indices; anything else on the panel is swallowed as
    /// [`WorkspaceHit::QualityPanel`] so the canvas below never sees it.
    pub fn hit_test(&self, point: Point2D) -> Option<WorkspaceHit> {
        if !self.panel.contains(point) {
            return None;
        }
        for row in &self.rows {
            if let QualityRowKind::Remaining { topic, item } = row.kind {
                if row.rect.contains(point) {
                    return Some(WorkspaceHit::QualityItem { topic, item });
                }
            }
        }
        Some(WorkspaceHit::QualityPanel)
    }
}

#[cfg(test)]
#[path = "workspace_quality_tests.rs"]
mod tests;
