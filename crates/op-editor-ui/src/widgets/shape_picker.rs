//! `ShapePicker` — dropdown panel listing the seven shape options
//! the Toolbar's shape slot exposes (mirrors
//! `apps/web/src/components/editor/shape-tool-dropdown.tsx`).
//!
//! Anchored to the right of the Toolbar shape slot when
//! `Document.ui.shape_picker_open == true`. Picking a row sets
//! `ui.shape_tool` (drives the toolbar slot's icon), sets
//! `doc.tool` (active tool), and closes the panel. Two of the
//! seven rows are one-shot actions rather than tool changes:
//! `Icon` opens an icon picker (Step N+ follow-up) and
//! `ImportImageOrSvg` opens a file dialog. Both are reported
//! verbatim by the host so it can dispatch to the right place.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::{doc_shape_choice, theme_for, translate};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::editor_ui_state::EditorUiState;
use op_editor_core::Tool;

pub const SHAPE_PICKER_WIDTH: f32 = 220.0;
const ROW_HEIGHT: f32 = 32.0;
const ROW_PAD_X: f32 = 12.0;
const ICON_SIZE: f32 = 16.0;

/// What the user picked from the dropdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeChoice {
    /// Pick a shape tool (Rect / Ellipse / Polygon / Line / Pen).
    Tool(Tool),
    /// Open the icon picker (host concern; this widget only
    /// reports the intent).
    OpenIconPicker,
    /// Open a file dialog to import an image / SVG.
    ImportImageOrSvg,
}

/// Localised row strings — looked up once at panel construction.
struct PickerLabels {
    rectangle: String,
    ellipse: String,
    polygon: String,
    line: String,
    icon: String,
    import_image: String,
    pen: String,
}

impl PickerLabels {
    fn for_editor_ui(ui: &EditorUiState) -> Self {
        let pick = |key: &'static str, fallback: &'static str| -> String {
            let translated = translate(ui, key);
            if translated == key {
                fallback.to_string()
            } else {
                translated.to_string()
            }
        };
        Self {
            rectangle: pick("shapes.rectangle", "Rectangle"),
            ellipse: pick("shapes.ellipse", "Ellipse"),
            polygon: pick("shapes.polygon", "Polygon"),
            line: pick("shapes.line", "Line"),
            icon: pick("shapes.icon", "Icon"),
            import_image: pick("shapes.importImageSvg", "Import Image or SVG\u{2026}"),
            pen: pick("shapes.pen", "Pen"),
        }
    }
}

struct PickerRow {
    icon: Icon,
    label: String,
    choice: ShapeChoice,
}

pub struct ShapePicker {
    pub id: WidgetId,
    pub theme: Theme,
    pub current_shape: Tool,
    /// Choice currently under the cursor — mirrors
    /// `Document.ui.shape_picker_hover` so paint can tint the row.
    pub hovered: Option<ShapeChoice>,
    rows: Vec<PickerRow>,
}

impl ShapePicker {
    pub fn for_editor_ui(ui: &EditorUiState) -> Self {
        let labels = PickerLabels::for_editor_ui(ui);
        let rows = vec![
            PickerRow {
                icon: Icon::Square,
                label: labels.rectangle,
                choice: ShapeChoice::Tool(Tool::Rect),
            },
            PickerRow {
                icon: Icon::Circle,
                label: labels.ellipse,
                choice: ShapeChoice::Tool(Tool::Ellipse),
            },
            PickerRow {
                icon: Icon::Triangle,
                label: labels.polygon,
                choice: ShapeChoice::Tool(Tool::Polygon),
            },
            PickerRow {
                icon: Icon::Minus,
                label: labels.line,
                choice: ShapeChoice::Tool(Tool::Line),
            },
            PickerRow {
                icon: Icon::Sparkles,
                label: labels.icon,
                choice: ShapeChoice::OpenIconPicker,
            },
            PickerRow {
                icon: Icon::ImagePlus,
                label: labels.import_image,
                choice: ShapeChoice::ImportImageOrSvg,
            },
            PickerRow {
                icon: Icon::PenTool,
                label: labels.pen,
                choice: ShapeChoice::Tool(Tool::Pen),
            },
        ];
        Self {
            id: WidgetId::new(5200),
            theme: theme_for(ui),
            current_shape: ui.shape_tool,
            hovered: ui.shape_picker_hover.map(doc_shape_choice),
            rows,
        }
    }

    /// Total panel height for all 7 rows + 6 px top/bottom pad.
    pub fn panel_height() -> f32 {
        ROW_HEIGHT * 7.0 + 12.0
    }

    /// Hit-test the panel — returns the choice on the row under
    /// `point`, or `None` if the cursor is in the chrome / outside.
    pub fn hit_test(&self, panel_rect: Rect, point: Point2D) -> Option<ShapeChoice> {
        if !rect_contains(panel_rect, point) {
            return None;
        }
        let inner_y = point.y - panel_rect.origin.y - 6.0;
        if inner_y < 0.0 {
            return None;
        }
        let idx = (inner_y / ROW_HEIGHT) as usize;
        self.rows.get(idx).map(|r| r.choice)
    }
}

fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.x <= r.origin.x + r.size.x
        && p.y >= r.origin.y
        && p.y <= r.origin.y + r.size.y
}

impl Widget for ShapePicker {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(SHAPE_PICKER_WIDTH, Self::panel_height()),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_round_rect(rect, 8.0, self.theme.popover);
        cx.backend
            .stroke_round_rect(rect, 8.0, self.theme.border, 1.0);

        for (idx, row) in self.rows.iter().enumerate() {
            let row_y = rect.origin.y + 6.0 + idx as f32 * ROW_HEIGHT;
            let row_rect = Rect {
                origin: Point2D::new(rect.origin.x + 4.0, row_y),
                size: Point2D::new(rect.size.x - 8.0, ROW_HEIGHT),
            };
            let active = matches!(row.choice, ShapeChoice::Tool(t) if t == self.current_shape);
            let hovered = !active && self.hovered == Some(row.choice);
            if active {
                cx.backend
                    .fill_round_rect(row_rect, 6.0, self.theme.row_selected_primary);
            } else if hovered {
                cx.backend.fill_round_rect(row_rect, 6.0, self.theme.muted);
            }
            let icon_color = if active {
                self.theme.primary
            } else {
                self.theme.foreground
            };
            draw_icon(
                cx.backend,
                row.icon,
                Point2D::new(
                    row_rect.origin.x + ROW_PAD_X,
                    row_y + (ROW_HEIGHT - ICON_SIZE) / 2.0,
                ),
                ICON_SIZE,
                icon_color,
                1.4,
            );
            let label = TextLayout::single_run(
                &row.label,
                "system-ui",
                13.0,
                to_jian_color(if active {
                    self.theme.primary
                } else {
                    self.theme.foreground
                }),
                Point2D::new(0.0, 0.0),
            );
            cx.backend.draw_text(
                &label,
                Point2D::new(
                    row_rect.origin.x + ROW_PAD_X + ICON_SIZE + 12.0,
                    row_y + 21.0,
                ),
            );
        }
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Menu);
        node.set_label("Shape tools");
        node
    }
}

fn to_jian_color(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_height_covers_seven_rows() {
        assert!(ShapePicker::panel_height() > ROW_HEIGHT * 7.0);
    }

    #[test]
    fn first_row_resolves_to_rectangle() {
        let ui = op_editor_core::editor_ui_state::EditorUiState::new();
        let picker = ShapePicker::for_editor_ui(&ui);
        let panel_rect = Rect {
            origin: Point2D::new(100.0, 50.0),
            size: Point2D::new(SHAPE_PICKER_WIDTH, ShapePicker::panel_height()),
        };
        let hit = picker.hit_test(panel_rect, Point2D::new(150.0, 50.0 + 10.0));
        assert_eq!(hit, Some(ShapeChoice::Tool(Tool::Rect)));
    }

    #[test]
    fn last_row_resolves_to_pen() {
        let ui = op_editor_core::editor_ui_state::EditorUiState::new();
        let picker = ShapePicker::for_editor_ui(&ui);
        let panel_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(SHAPE_PICKER_WIDTH, ShapePicker::panel_height()),
        };
        let last_y = 6.0 + (7.0 - 0.5) * ROW_HEIGHT;
        let hit = picker.hit_test(panel_rect, Point2D::new(50.0, last_y));
        assert_eq!(hit, Some(ShapeChoice::Tool(Tool::Pen)));
    }
}
