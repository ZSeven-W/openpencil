//! `ShapePicker` — dropdown panel listing the seven shape options
//! the Toolbar's shape slot exposes (mirrors
//! `apps/web/src/components/editor/shape-tool-dropdown.tsx`).
//!
//! Anchored to the right of the Toolbar shape slot when
//! `Document.ui.shape_picker.open == true`. Picking a row sets
//! `ui.shape_tool` (drives the toolbar slot's icon), sets
//! `doc.tool` (active tool), and closes the panel. Two of the
//! seven rows are one-shot actions rather than tool changes:
//! `Icon` opens an icon picker (Step N+ follow-up) and
//! `ImportImageOrSvg` opens a file dialog. Both are reported
//! verbatim by the host so it can dispatch to the right place.

use crate::theme::Theme;
use crate::widgets::button::paint_button_feedback_wash;
use crate::widgets::editor_state_ext::{theme_for, translate};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Point2D, Rect, TextLayout};
pub use jian_widgets::components::select::SelectHit;
use jian_widgets::components::select::{Select, SelectState};
use jian_widgets::{Density, Tokens};
use op_editor_core::editor_ui_state::EditorUiState;
use op_editor_core::Tool;

pub const SHAPE_PICKER_WIDTH: f32 = 220.0;
const ROW_HEIGHT: f32 = 30.0;
const ROW_PAD_X: f32 = 12.0;
const ICON_SIZE: f32 = 16.0;
const ROW_COUNT: usize = 7;

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
    pub state: SelectState,
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
            state: ui.shape_picker.clone(),
            rows,
        }
    }

    /// Total panel height from the shared Select row metric.
    pub fn panel_height() -> f32 {
        ROW_HEIGHT * ROW_COUNT as f32
    }

    pub fn hit_popup(&self, panel_rect: Rect, point: Point2D) -> SelectHit {
        let anchor = popup_anchor(panel_rect);
        Select::hit(
            &self.state,
            anchor,
            panel_rect,
            self.rows.len(),
            point,
            &tokens_from_theme(&self.theme),
        )
    }

    pub fn choice_at(&self, idx: usize) -> Option<ShapeChoice> {
        self.rows.get(idx).map(|r| r.choice)
    }

    /// Hit-test the panel — returns the choice on the row under
    /// `point`, or `None` if the cursor is in the chrome / outside.
    pub fn hit_test(&self, panel_rect: Rect, point: Point2D) -> Option<ShapeChoice> {
        match self.hit_popup(panel_rect, point) {
            SelectHit::Row(idx) => self.choice_at(idx),
            SelectHit::Inside | SelectHit::Outside => None,
        }
    }
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
            let row_y = rect.origin.y + idx as f32 * ROW_HEIGHT;
            let row_rect = Rect {
                origin: Point2D::new(rect.origin.x + 4.0, row_y),
                size: Point2D::new(rect.size.x - 8.0, ROW_HEIGHT),
            };
            let active = matches!(row.choice, ShapeChoice::Tool(t) if t == self.current_shape);
            let hovered = !active && self.state.hover == Some(idx);
            let pressed = !active && self.state.pressed == Some(idx);
            if active {
                cx.backend
                    .fill_round_rect(row_rect, 6.0, self.theme.row_selected_primary);
            } else if hovered || pressed {
                paint_button_feedback_wash(
                    cx.backend,
                    &self.theme,
                    row_rect,
                    6.0,
                    hovered,
                    pressed,
                );
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
                (if active {
                    self.theme.primary
                } else {
                    self.theme.foreground
                })
                .to_jian(),
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

fn popup_anchor(popup: Rect) -> Rect {
    Rect {
        origin: popup.origin,
        size: Point2D::new(popup.size.x, 0.0),
    }
}

fn tokens_from_theme(theme: &Theme) -> Tokens {
    Tokens {
        background: theme.background,
        foreground: theme.foreground,
        card: theme.card,
        card_foreground: theme.card_foreground,
        popover: theme.popover,
        popover_foreground: theme.popover_foreground,
        primary: theme.primary,
        primary_foreground: theme.primary_foreground,
        muted: theme.muted,
        muted_foreground: theme.muted_foreground,
        border: theme.border,
        accent: theme.accent,
        accent_foreground: theme.accent_foreground,
        destructive: theme.destructive,
        button_hover: theme.button_hover,
        row_selected: theme.row_selected,
        row_selected_primary: theme.row_selected_primary,
        density: Density::Desktop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, RenderBackend};

    #[derive(Default)]
    struct RoundFillBackend {
        fills: Vec<(Rect, f32, Color)>,
    }

    impl RenderBackend for RoundFillBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
            self.fills.push((rect, radius, color));
        }
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    fn color_close(a: Color, b: Color) -> bool {
        (a.r - b.r).abs() < 1e-6
            && (a.g - b.g).abs() < 1e-6
            && (a.b - b.b).abs() < 1e-6
            && (a.a - b.a).abs() < 1e-6
    }

    #[test]
    fn panel_height_covers_seven_rows() {
        assert_eq!(ShapePicker::panel_height(), ROW_HEIGHT * ROW_COUNT as f32);
    }

    #[test]
    fn hit_select_uses_shared_outside_protocol() {
        let mut ui = op_editor_core::editor_ui_state::EditorUiState::new();
        ui.shape_picker.open = true;
        let picker = ShapePicker::for_editor_ui(&ui);
        let panel_rect = Rect {
            origin: Point2D::new(100.0, 50.0),
            size: Point2D::new(SHAPE_PICKER_WIDTH, ShapePicker::panel_height()),
        };

        assert_eq!(
            picker.hit_popup(panel_rect, Point2D::new(150.0, 50.0 + 10.0)),
            SelectHit::Row(0)
        );
        assert_eq!(
            picker.hit_popup(panel_rect, Point2D::new(99.0, 50.0)),
            SelectHit::Outside
        );
    }

    #[test]
    fn first_row_resolves_to_rectangle() {
        let mut ui = op_editor_core::editor_ui_state::EditorUiState::new();
        ui.shape_picker.open = true;
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
        let mut ui = op_editor_core::editor_ui_state::EditorUiState::new();
        ui.shape_picker.open = true;
        let picker = ShapePicker::for_editor_ui(&ui);
        let panel_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(SHAPE_PICKER_WIDTH, ShapePicker::panel_height()),
        };
        let last_y = (ROW_COUNT as f32 - 0.5) * ROW_HEIGHT;
        let hit = picker.hit_test(panel_rect, Point2D::new(50.0, last_y));
        assert_eq!(hit, Some(ShapeChoice::Tool(Tool::Pen)));
    }

    #[test]
    fn pressed_row_uses_shared_select_feedback() {
        let mut ui = op_editor_core::editor_ui_state::EditorUiState::new();
        ui.shape_picker.open = true;
        ui.shape_picker.pressed = Some(1);
        let picker = ShapePicker::for_editor_ui(&ui);
        let panel_rect = Rect {
            origin: Point2D::new(100.0, 50.0),
            size: Point2D::new(SHAPE_PICKER_WIDTH, ShapePicker::panel_height()),
        };
        let pressed_row = Rect {
            origin: Point2D::new(panel_rect.origin.x + 4.0, panel_rect.origin.y + ROW_HEIGHT),
            size: Point2D::new(panel_rect.size.x - 8.0, ROW_HEIGHT),
        };
        let expected = picker
            .theme
            .button_hover
            .with_alpha(picker.theme.button_hover.a * 1.8);
        let mut backend = RoundFillBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        picker.paint(&mut cx, panel_rect);

        assert!(
            backend.fills.iter().any(|(fill, radius, color)| {
                *fill == pressed_row
                    && (*radius - 6.0).abs() < 0.01
                    && color_close(*color, expected)
            }),
            "pressed shape picker row should paint the shared pressed feedback token"
        );
    }
}
