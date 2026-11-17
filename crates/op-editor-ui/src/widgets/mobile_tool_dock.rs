//! Touch tool dock, shared by the pane painter and the native shell actions.

use super::{centered_icon_rect, dock_surface, paint_touch_icon};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::panel_control_metrics::mix;
use crate::widgets::toolbar::icon_for_shape;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, Theme};
use op_editor_core::{tool::Tool, EditorState};

/// Dock hit results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobileDockHit {
    /// Tool selection (Select / Pen / Text).
    Tool(Tool),
    /// Shape slot — opens the touch shape picker.
    ShapeSlot,
    /// More sheet.
    More,
}

/// The touch tool dock: Select / Shape / Pen / Text / More.
pub struct MobileDock {
    pub active: Tool,
    pub shape_tool: Tool,
    pub shape_picker_open: bool,
    pub theme: Theme,
    pub compact: bool,
}

impl MobileDock {
    pub fn for_editor(state: &EditorState) -> Self {
        Self {
            active: state.tool,
            shape_tool: state.editor_ui.shape_tool,
            shape_picker_open: state.editor_ui.shape_picker.open,
            theme: crate::widgets::editor_state_ext::theme_for(&state.editor_ui),
            compact: state.editor_ui.compact_layout(),
        }
    }

    pub fn slot_count(&self) -> usize {
        if self.compact {
            5
        } else {
            4
        }
    }

    pub fn slot_rect(rect: Rect, index: usize, slot_count: usize) -> Rect {
        let slot_w = rect.size.x / slot_count.max(1) as f32;
        Rect {
            origin: Point2D::new(rect.origin.x + slot_w * index as f32, rect.origin.y),
            size: Point2D::new(slot_w, rect.size.y),
        }
    }

    fn slot_icon(&self, index: usize) -> Icon {
        match index {
            0 => Icon::from_name("mouse-pointer-2").unwrap_or(Icon::Cursor),
            1 => icon_for_shape(self.shape_tool),
            2 => Icon::from_name("pen-tool").unwrap_or(Icon::PenTool),
            3 => Icon::from_name("type").unwrap_or(Icon::Type),
            _ => Icon::from_name("ellipsis").unwrap_or(Icon::MoreHorizontal),
        }
    }

    fn slot_active(&self, index: usize) -> bool {
        match index {
            0 => self.active == Tool::Select,
            1 => matches!(
                self.active,
                Tool::Rect | Tool::Ellipse | Tool::Polygon | Tool::Line
            ),
            2 => self.active == Tool::Pen,
            3 => self.active == Tool::Text,
            _ => false,
        }
    }

    pub fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        if self.compact {
            cx.backend.fill_rect(rect, dock_surface(self.theme));
            cx.backend.fill_rect(
                Rect {
                    origin: rect.origin,
                    size: Point2D::new(rect.size.x, 1.0),
                },
                self.theme.border,
            );
        } else {
            cx.backend
                .fill_round_rect(rect, 20.0, mix(self.theme.popover, Color::BLACK, 0.08));
            cx.backend
                .stroke_round_rect(rect, 20.0, self.theme.border, 1.0);
        }

        let slot_count = self.slot_count();
        for index in 0..slot_count {
            let slot = Self::slot_rect(rect, index, slot_count);
            let active = self.slot_active(index);
            let open = index == 1 && self.shape_picker_open;
            let emphasized = active || open;
            let icon = self.slot_icon(index);
            let active_rect = centered_icon_rect(slot, 40.0);
            if emphasized {
                cx.backend
                    .fill_round_rect(active_rect, 12.0, self.theme.row_selected_primary);
            }
            let color = if emphasized {
                self.theme.primary
            } else {
                self.theme.foreground.with_alpha(0.82)
            };
            if index == 1 {
                // Keep the shape glyph and its dropdown cue together inside
                // the 40pt feedback wash. The whole slot remains one touch
                // target; the chevron is a visual affordance, not a second
                // control.
                let (icon_rect, chevron_rect) = shape_slot_glyph_rects(slot);
                draw_icon(
                    cx.backend,
                    icon,
                    icon_rect.origin,
                    icon_rect.size.x,
                    color,
                    1.75,
                );
                draw_icon(
                    cx.backend,
                    if open {
                        Icon::ChevronUp
                    } else {
                        Icon::ChevronDown
                    },
                    chevron_rect.origin,
                    chevron_rect.size.x,
                    color,
                    1.6,
                );
            } else {
                paint_touch_icon(cx, slot, icon, 22.0, color);
            }
        }
    }

    pub fn hit_test(&self, rect: Rect, point: Point2D) -> Option<MobileDockHit> {
        if !rect.contains(point) {
            return None;
        }
        let slot_count = self.slot_count();
        let index = (((point.x - rect.origin.x) / (rect.size.x / slot_count as f32)).floor()
            as usize)
            .min(slot_count - 1);
        self.hit_for_slot(index)
    }

    pub fn hit_for_slot(&self, index: usize) -> Option<MobileDockHit> {
        match index {
            0 => Some(MobileDockHit::Tool(Tool::Select)),
            1 => Some(MobileDockHit::ShapeSlot),
            2 => Some(MobileDockHit::Tool(Tool::Pen)),
            3 => Some(MobileDockHit::Tool(Tool::Text)),
            4 if self.compact => Some(MobileDockHit::More),
            _ => None,
        }
    }
}

/// Lay out the shape glyph and its dropdown cue as one centered compound
/// mark. Both remain inside the dock's 40pt active/pressed feedback wash.
fn shape_slot_glyph_rects(slot: Rect) -> (Rect, Rect) {
    const ICON_SIZE: f32 = 20.0;
    const CHEVRON_SIZE: f32 = 9.0;
    const GAP: f32 = 3.0;
    let group_width = ICON_SIZE + GAP + CHEVRON_SIZE;
    let group_x = slot.origin.x + (slot.size.x - group_width) / 2.0;
    let center_y = slot.origin.y + slot.size.y / 2.0;
    (
        Rect {
            origin: Point2D::new(group_x, center_y - ICON_SIZE / 2.0),
            size: Point2D::new(ICON_SIZE, ICON_SIZE),
        },
        Rect {
            origin: Point2D::new(group_x + ICON_SIZE + GAP, center_y - CHEVRON_SIZE / 2.0),
            size: Point2D::new(CHEVRON_SIZE, CHEVRON_SIZE),
        },
    )
}
