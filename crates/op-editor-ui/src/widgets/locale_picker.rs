//! `LocalePicker` — dropdown panel listing the 15 native-script
//! UI locales. Anchored under the TopBar Globe button when
//! `Document.ui.locale_picker_open == true`.
//!
//! Mirrors the TS app's i18n switcher: vertical list of locale
//! names, currently-selected row marked with a check icon and
//! primary tint.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::editor_ui_state::EditorUiState;
use op_i18n::Locale;

pub const LOCALE_PICKER_WIDTH: f32 = 200.0;
const ROW_HEIGHT: f32 = 32.0;
const ROW_PAD_X: f32 = 12.0;
const ICON_SIZE: f32 = 16.0;

/// Locale rows + selected marker — built fresh from the document
/// for each paint.
pub struct LocalePicker {
    pub id: WidgetId,
    pub theme: Theme,
    pub selected: Locale,
    /// Row currently under the cursor — drives the per-row tint.
    /// Mirrors `Document.ui.locale_picker_hover`, populated by the
    /// host on cursor-move while `locale_picker_open == true`.
    pub hovered: Option<Locale>,
}

impl LocalePicker {
    pub fn for_editor_ui(ui: &EditorUiState) -> Self {
        Self {
            id: WidgetId::new(5100),
            theme: theme_for(ui),
            selected: ui.locale,
            hovered: ui.locale_picker_hover,
        }
    }

    /// Total panel height for the 15 locale rows + 6px top/bottom pad.
    pub fn panel_height() -> f32 {
        ROW_HEIGHT * Locale::ALL.len() as f32 + 12.0
    }

    /// Hit-test the panel at `point`. Returns the locale whose row
    /// contains the cursor, or `None` for the chrome / outside.
    /// `panel_rect` is the already-computed panel bounds the host
    /// would paint into.
    pub fn hit_test(&self, panel_rect: Rect, point: Point2D) -> Option<Locale> {
        if !(panel_rect).contains(point) {
            return None;
        }
        let inner_y = point.y - panel_rect.origin.y - 6.0;
        if inner_y < 0.0 {
            return None;
        }
        let idx = (inner_y / ROW_HEIGHT) as usize;
        Locale::ALL.get(idx).copied()
    }
}

impl Widget for LocalePicker {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(LOCALE_PICKER_WIDTH, Self::panel_height()),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        // Panel body.
        cx.backend.fill_round_rect(rect, 8.0, self.theme.popover);
        cx.backend
            .stroke_round_rect(rect, 8.0, self.theme.border, 1.0);

        // Rows.
        for (idx, &locale) in Locale::ALL.iter().enumerate() {
            let row_y = rect.origin.y + 6.0 + idx as f32 * ROW_HEIGHT;
            let row_rect = Rect {
                origin: Point2D::new(rect.origin.x + 4.0, row_y),
                size: Point2D::new(rect.size.x - 8.0, ROW_HEIGHT),
            };
            let is_selected = locale == self.selected;
            let is_hovered = !is_selected && self.hovered == Some(locale);
            if is_selected {
                cx.backend
                    .fill_round_rect(row_rect, 6.0, self.theme.row_selected_primary);
            } else if is_hovered {
                cx.backend.fill_round_rect(row_rect, 6.0, self.theme.muted);
            }
            let label_color = if is_selected {
                self.theme.primary
            } else {
                self.theme.foreground
            };
            let label = TextLayout::single_run(
                locale.display_name(),
                "system-ui",
                13.0,
                (label_color).to_jian(),
                Point2D::new(0.0, 0.0),
            );
            cx.backend.draw_text(
                &label,
                Point2D::new(row_rect.origin.x + ROW_PAD_X, row_y + 21.0),
            );
            if is_selected {
                draw_icon(
                    cx.backend,
                    Icon::Check,
                    Point2D::new(
                        row_rect.origin.x + row_rect.size.x - ICON_SIZE - 10.0,
                        row_y + (ROW_HEIGHT - ICON_SIZE) / 2.0,
                    ),
                    ICON_SIZE,
                    self.theme.primary,
                    1.6,
                );
            }
        }
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::ListBox);
        node.set_label("Language picker");
        node
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_height_covers_15_rows() {
        let h = LocalePicker::panel_height();
        assert!(h > ROW_HEIGHT * 15.0);
    }

    #[test]
    fn hit_test_resolves_first_row_to_first_locale() {
        let ui = op_editor_core::editor_ui_state::EditorUiState::new();
        let picker = LocalePicker::for_editor_ui(&ui);
        let panel_rect = Rect {
            origin: Point2D::new(100.0, 50.0),
            size: Point2D::new(LOCALE_PICKER_WIDTH, LocalePicker::panel_height()),
        };
        let hit = picker.hit_test(panel_rect, Point2D::new(150.0, 50.0 + 10.0));
        assert_eq!(hit, Some(Locale::ALL[0]));
    }

    #[test]
    fn hit_test_resolves_last_row() {
        let ui = op_editor_core::editor_ui_state::EditorUiState::new();
        let picker = LocalePicker::for_editor_ui(&ui);
        let panel_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(LOCALE_PICKER_WIDTH, LocalePicker::panel_height()),
        };
        let last_y = 6.0 + (Locale::ALL.len() as f32 - 0.5) * ROW_HEIGHT;
        let hit = picker.hit_test(panel_rect, Point2D::new(50.0, last_y));
        assert_eq!(hit, Some(Locale::ALL[Locale::ALL.len() - 1]));
    }

    #[test]
    fn hit_test_outside_returns_none() {
        let ui = op_editor_core::editor_ui_state::EditorUiState::new();
        let picker = LocalePicker::for_editor_ui(&ui);
        let panel_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(LOCALE_PICKER_WIDTH, LocalePicker::panel_height()),
        };
        assert!(picker
            .hit_test(panel_rect, Point2D::new(-5.0, 50.0))
            .is_none());
        assert!(picker
            .hit_test(panel_rect, Point2D::new(50.0, -5.0))
            .is_none());
    }
}
