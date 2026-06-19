//! `LocalePicker` — dropdown panel listing the 15 native-script
//! UI locales. Anchored under the TopBar Globe button when
//! `Document.ui.locale_picker.open == true`.
//!
//! Mirrors the TS app's i18n switcher: vertical list of locale
//! names, currently-selected row marked with a check icon and
//! primary tint.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Point2D, Rect};
pub use jian_widgets::components::select::SelectHit;
use jian_widgets::components::select::{Select, SelectItem, SelectState, MAX_VISIBLE_ROWS};
use jian_widgets::Tokens;
use op_editor_core::editor_ui_state::EditorUiState;
use op_i18n::Locale;

pub const LOCALE_PICKER_WIDTH: f32 = 200.0;
const ROW_HEIGHT: f32 = 30.0;

/// Locale rows + selected marker — built fresh from the document
/// for each paint.
pub struct LocalePicker {
    pub id: WidgetId,
    pub theme: Theme,
    pub selected: Locale,
    pub state: SelectState,
}

impl LocalePicker {
    pub fn for_editor_ui(ui: &EditorUiState) -> Self {
        Self {
            id: WidgetId::new(5100),
            theme: theme_for(ui),
            selected: ui.locale,
            state: ui.locale_picker.clone(),
        }
    }

    /// Max panel height from the shared Select component.
    pub fn panel_height() -> f32 {
        ROW_HEIGHT * MAX_VISIBLE_ROWS as f32
    }

    pub fn row_height() -> f32 {
        ROW_HEIGHT
    }

    pub fn max_scroll() -> f32 {
        Locale::ALL.len().saturating_sub(MAX_VISIBLE_ROWS) as f32 * ROW_HEIGHT
    }

    pub fn popup_rect(&self, anchor: Rect, viewport: Rect) -> Rect {
        Select::popup_rect(
            anchor,
            viewport,
            Locale::ALL.len(),
            &tokens_from_theme(&self.theme),
        )
    }

    pub fn hit_select(&self, anchor: Rect, viewport: Rect, point: Point2D) -> SelectHit {
        Select::hit(
            &self.state,
            anchor,
            viewport,
            Locale::ALL.len(),
            point,
            &tokens_from_theme(&self.theme),
        )
    }

    pub fn paint_select(&self, cx: &mut PaintCx<'_>, anchor: Rect, viewport: Rect) {
        let items = self.items();
        let select = Select {
            state: &self.state,
            items: &items,
        };
        select.paint(
            cx.backend,
            anchor,
            viewport,
            &tokens_from_theme(&self.theme),
        );
    }

    /// Hit-test the panel at `point`. Returns the locale whose row
    /// contains the cursor, or `None` for the chrome / outside.
    /// `panel_rect` is the already-computed panel bounds the host
    /// would paint into.
    pub fn hit_test(&self, panel_rect: Rect, point: Point2D) -> Option<Locale> {
        match self.hit_popup(panel_rect, point) {
            SelectHit::Row(idx) => Self::locale_at(idx),
            SelectHit::Inside | SelectHit::Outside => None,
        }
    }

    pub fn hit_popup(&self, panel_rect: Rect, point: Point2D) -> SelectHit {
        let anchor = popup_anchor(panel_rect);
        self.hit_select(anchor, panel_rect, point)
    }

    pub fn locale_at(idx: usize) -> Option<Locale> {
        Locale::ALL.get(idx).copied()
    }

    fn items(&self) -> Vec<SelectItem<'static>> {
        Locale::ALL
            .iter()
            .map(|locale| SelectItem {
                label: locale.display_name(),
                selected: *locale == self.selected,
                disabled: false,
            })
            .collect()
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
        self.paint_select(cx, popup_anchor(rect), rect);
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::ListBox);
        node.set_label("Language picker");
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
    crate::widgets::button::tokens_from_theme(theme)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jian_widgets::components::select::SelectHit;

    #[test]
    fn panel_height_is_capped_to_select_visible_rows() {
        let h = LocalePicker::panel_height();
        assert_eq!(h, ROW_HEIGHT * 8.0);
    }

    #[test]
    fn hit_select_uses_shared_outside_protocol() {
        let mut ui = op_editor_core::editor_ui_state::EditorUiState::new();
        ui.locale_picker.open = true;
        let picker = LocalePicker::for_editor_ui(&ui);
        let anchor = Rect {
            origin: Point2D::new(100.0, 40.0),
            size: Point2D::new(LOCALE_PICKER_WIDTH, 28.0),
        };
        let viewport = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(400.0, 400.0),
        };
        let popup = picker.popup_rect(anchor, viewport);

        assert_eq!(
            picker.hit_select(
                anchor,
                viewport,
                Point2D::new(popup.origin.x + 10.0, popup.origin.y + ROW_HEIGHT * 0.5)
            ),
            SelectHit::Row(0)
        );
        assert_eq!(
            picker.hit_select(
                anchor,
                viewport,
                Point2D::new(popup.origin.x - 1.0, popup.origin.y)
            ),
            SelectHit::Outside
        );
    }

    #[test]
    fn hit_test_resolves_first_row_to_first_locale() {
        let mut ui = op_editor_core::editor_ui_state::EditorUiState::new();
        ui.locale_picker.open = true;
        let picker = LocalePicker::for_editor_ui(&ui);
        let panel_rect = Rect {
            origin: Point2D::new(100.0, 50.0),
            size: Point2D::new(LOCALE_PICKER_WIDTH, LocalePicker::panel_height()),
        };
        let hit = picker.hit_test(panel_rect, Point2D::new(150.0, 50.0 + 10.0));
        assert_eq!(hit, Some(Locale::ALL[0]));
    }

    #[test]
    fn hit_test_resolves_last_visible_row() {
        let mut ui = op_editor_core::editor_ui_state::EditorUiState::new();
        ui.locale_picker.open = true;
        let picker = LocalePicker::for_editor_ui(&ui);
        let panel_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(LOCALE_PICKER_WIDTH, LocalePicker::panel_height()),
        };
        let last_y = (MAX_VISIBLE_ROWS as f32 - 0.5) * ROW_HEIGHT;
        let hit = picker.hit_test(panel_rect, Point2D::new(50.0, last_y));
        assert_eq!(hit, Some(Locale::ALL[MAX_VISIBLE_ROWS - 1]));
    }

    #[test]
    fn hit_test_resolves_scrolled_last_row() {
        let mut ui = op_editor_core::editor_ui_state::EditorUiState::new();
        ui.locale_picker.open = true;
        ui.locale_picker.scroll.offset = LocalePicker::max_scroll();
        let picker = LocalePicker::for_editor_ui(&ui);
        let panel_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(LOCALE_PICKER_WIDTH, LocalePicker::panel_height()),
        };
        let last_y = (MAX_VISIBLE_ROWS as f32 - 0.5) * ROW_HEIGHT;
        let hit = picker.hit_test(panel_rect, Point2D::new(50.0, last_y));
        assert_eq!(hit, Locale::ALL.last().copied());
    }

    #[test]
    fn hit_test_outside_returns_none() {
        let mut ui = op_editor_core::editor_ui_state::EditorUiState::new();
        ui.locale_picker.open = true;
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
