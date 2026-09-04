//! Density-aware LayerPanel metrics and shared action geometry.
//!
//! Paint, hit testing, scrolling, reveal and drag/drop all consume the same
//! metrics so touch rows cannot visually drift away from their hit targets.

use crate::{Point2D, Rect};
use op_editor_core::editor_ui_state::EditorUiState;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LayerPanelMetrics {
    pub section_header_height: f32,
    pub page_row_height: f32,
    pub layer_row_height: f32,
    pub row_pad_x: f32,
    pub section_gap: f32,
    pub header_font: f32,
    pub row_font: f32,
    pub glyph_size: f32,
    pub trailing_glyph_size: f32,
    pub action_target: f32,
    pub pages_max_rows: usize,
    pub touch: bool,
}

impl LayerPanelMetrics {
    pub const DESKTOP: Self = Self {
        section_header_height: 28.0,
        page_row_height: 32.0,
        layer_row_height: 28.0,
        row_pad_x: 12.0,
        section_gap: 8.0,
        header_font: 12.0,
        row_font: 13.0,
        glyph_size: 14.0,
        trailing_glyph_size: 12.0,
        action_target: 22.0,
        pages_max_rows: 6,
        touch: false,
    };

    pub fn for_ui(ui: &EditorUiState) -> Self {
        if !ui.touch_chrome() {
            return Self::DESKTOP;
        }
        Self {
            section_header_height: 44.0,
            page_row_height: 48.0,
            layer_row_height: 48.0,
            row_pad_x: 12.0,
            section_gap: 8.0,
            header_font: 14.0,
            row_font: 15.0,
            glyph_size: 18.0,
            trailing_glyph_size: 18.0,
            action_target: 44.0,
            pages_max_rows: if ui.size_class.is_compact() { 3 } else { 6 },
            touch: true,
        }
    }
}

fn square_centered_on(glyph: Rect, side: f32) -> Rect {
    Rect {
        origin: Point2D::new(
            glyph.origin.x - (side - glyph.size.x) / 2.0,
            glyph.origin.y - (side - glyph.size.y) / 2.0,
        ),
        size: Point2D::new(side, side),
    }
}

pub(crate) fn glyph_rect_in(target: Rect, size: f32) -> Rect {
    Rect {
        origin: Point2D::new(
            target.origin.x + (target.size.x - size) / 2.0,
            target.origin.y + (target.size.y - size) / 2.0,
        ),
        size: Point2D::new(size, size),
    }
}

pub(crate) fn add_page_target(rect: Rect, header_y: f32, m: LayerPanelMetrics) -> Rect {
    if m.touch {
        return Rect {
            origin: Point2D::new(rect.origin.x + rect.size.x - m.action_target, header_y),
            size: Point2D::new(m.action_target, m.action_target),
        };
    }
    let glyph = Rect {
        origin: Point2D::new(
            rect.origin.x + rect.size.x - m.row_pad_x - if m.touch { m.glyph_size } else { 12.0 },
            header_y + (m.section_header_height - m.glyph_size) / 2.0,
        ),
        size: Point2D::new(m.glyph_size, m.glyph_size),
    };
    square_centered_on(glyph, m.action_target)
}

pub(crate) fn delete_page_target(rect: Rect, row_y: f32, m: LayerPanelMetrics) -> Rect {
    if m.touch {
        return Rect {
            origin: Point2D::new(
                rect.origin.x + rect.size.x - m.action_target,
                row_y + (m.page_row_height - m.action_target) / 2.0,
            ),
            size: Point2D::new(m.action_target, m.action_target),
        };
    }
    let glyph = Rect {
        origin: Point2D::new(
            rect.origin.x + rect.size.x - m.row_pad_x - m.glyph_size,
            row_y + (m.page_row_height - m.glyph_size) / 2.0,
        ),
        size: Point2D::new(m.glyph_size, m.glyph_size),
    };
    square_centered_on(glyph, m.action_target)
}

pub(crate) fn layer_action_targets(row: Rect, m: LayerPanelMetrics) -> (Rect, Rect) {
    if m.touch {
        let lock = Rect {
            origin: Point2D::new(row.origin.x + row.size.x - m.action_target, row.origin.y),
            size: Point2D::new(m.action_target, row.size.y),
        };
        let eye = Rect {
            origin: Point2D::new(lock.origin.x - m.action_target, row.origin.y),
            size: Point2D::new(m.action_target, row.size.y),
        };
        return (eye, lock);
    }
    let trailing_right = row.origin.x + row.size.x - 8.0;
    let lock_x = trailing_right - 14.0;
    let eye_x = lock_x - 22.0;
    let y = row.origin.y + 7.0;
    let eye_glyph = Rect {
        origin: Point2D::new(eye_x, y),
        size: Point2D::new(m.trailing_glyph_size, m.trailing_glyph_size),
    };
    let lock_glyph = Rect {
        origin: Point2D::new(lock_x, y),
        size: Point2D::new(m.trailing_glyph_size, m.trailing_glyph_size),
    };
    (
        square_centered_on(eye_glyph, m.action_target),
        square_centered_on(lock_glyph, m.action_target),
    )
}

/// Dedicated touch reorder affordance immediately before visibility and lock.
/// Normal row drags remain available for list scrolling.
pub(crate) fn layer_drag_target(row: Rect, m: LayerPanelMetrics) -> Option<Rect> {
    if !m.touch {
        return None;
    }
    let (eye, _) = layer_action_targets(row, m);
    Some(Rect {
        origin: Point2D::new(eye.origin.x - m.action_target, row.origin.y),
        size: Point2D::new(m.action_target, row.size.y),
    })
}

pub(crate) fn collapse_target(
    row: Rect,
    indent: f32,
    horizontal_offset: f32,
    m: LayerPanelMetrics,
) -> Rect {
    if m.touch {
        return Rect {
            origin: Point2D::new(row.origin.x + indent - horizontal_offset, row.origin.y),
            size: Point2D::new(m.action_target, row.size.y),
        };
    }
    let glyph = Rect {
        origin: Point2D::new(
            row.origin.x + indent - horizontal_offset,
            row.origin.y + 6.0,
        ),
        size: Point2D::new(m.glyph_size, m.glyph_size),
    };
    square_centered_on(glyph, m.action_target)
}

pub(crate) fn layer_node_icon_x(row: Rect, indent: f32, m: LayerPanelMetrics) -> f32 {
    if m.touch {
        row.origin.x + indent + m.action_target + 4.0
    } else {
        row.origin.x + indent + 18.0
    }
}
