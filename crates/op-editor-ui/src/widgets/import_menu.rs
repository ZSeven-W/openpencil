//! `ImportMenu` — the TopBar import-button dropdown. Two rows:
//! import a Figma export (`.fig`) or a saved web page (`.html`).
//!
//! Built on the shared `Select` component (same as `LocalePicker` and
//! the fill-type picker) so row metrics, hover wash, and clamping
//! behave identically across every dropdown in the chrome.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::text_metrics;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Point2D, Rect, TextLayout};
pub use jian_widgets::components::select::SelectHit;
use jian_widgets::components::select::{Select, SelectItem, SelectState};
use jian_widgets::Tokens;
use op_editor_core::editor_ui_state::{EditorUiState, Locale};

pub const IMPORT_MENU_WIDTH: f32 = 200.0;
const SHORTCUT_FONT_SIZE: f32 = 11.0;
const SHORTCUT_PAD_RIGHT: f32 = 12.0;

/// What the user picked in the dropdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportMenuChoice {
    /// Open the Figma import modal (`.fig`).
    Figma,
    /// Open a file dialog for a saved page / snapshot (`.html`).
    Html,
}

impl ImportMenuChoice {
    pub const ALL: [ImportMenuChoice; 2] = [ImportMenuChoice::Figma, ImportMenuChoice::Html];

    /// Locale key for the row label.
    fn label_key(self) -> &'static str {
        match self {
            ImportMenuChoice::Figma => "import.fromFigma",
            ImportMenuChoice::Html => "import.fromHtml",
        }
    }

    /// English fallback for locales whose table predates these keys —
    /// `translate` yields the key itself, which would paint as debug
    /// text in the menu.
    fn fallback_label(self) -> &'static str {
        match self {
            ImportMenuChoice::Figma => "Import from Figma",
            ImportMenuChoice::Html => "Import from HTML",
        }
    }

    pub fn label(self, locale: Locale) -> &'static str {
        let key = self.label_key();
        let translated = op_i18n::translate(locale, key);
        if translated == key {
            self.fallback_label()
        } else {
            translated
        }
    }

    /// Keyboard chord shown at the trailing edge of the launcher row.
    pub const fn shortcut_label(self) -> &'static str {
        match self {
            ImportMenuChoice::Figma => "⌘⇧F",
            ImportMenuChoice::Html => "⌘⇧H",
        }
    }
}

pub struct ImportMenu {
    pub id: WidgetId,
    pub theme: Theme,
    locale: Locale,
    state: SelectState,
}

impl ImportMenu {
    pub fn for_editor_ui(ui: &EditorUiState) -> Self {
        Self {
            id: WidgetId::new(5410),
            theme: theme_for(ui),
            locale: ui.effective_locale(),
            state: ui.import_menu.clone(),
        }
    }

    /// Row height the shared component actually paints with (density
    /// token), so hosts anchoring the popup agree with hit-testing.
    pub fn row_height(&self) -> f32 {
        tokens_from_theme(&self.theme).density.row_height()
    }

    pub fn panel_height(&self) -> f32 {
        self.row_height() * ImportMenuChoice::ALL.len() as f32
    }

    /// Popup bounds under `anchor`, clamped into `viewport` by the
    /// shared component (flips above when the bottom edge is close).
    pub fn popup_rect(&self, anchor: Rect, viewport: Rect) -> Rect {
        Select::popup_rect(
            anchor,
            viewport,
            ImportMenuChoice::ALL.len(),
            &tokens_from_theme(&self.theme),
        )
    }

    pub fn hit(&self, anchor: Rect, viewport: Rect, point: Point2D) -> SelectHit {
        Select::hit(
            &self.state,
            anchor,
            viewport,
            ImportMenuChoice::ALL.len(),
            point,
            &tokens_from_theme(&self.theme),
        )
    }

    /// Row under the cursor, or `None` for chrome / outside.
    pub fn choice_at(
        &self,
        anchor: Rect,
        viewport: Rect,
        point: Point2D,
    ) -> Option<ImportMenuChoice> {
        match self.hit(anchor, viewport, point) {
            SelectHit::Row(idx) => ImportMenuChoice::ALL.get(idx).copied(),
            SelectHit::Inside | SelectHit::Outside => None,
        }
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

        if !self.state.open {
            return;
        }
        let popup = self.popup_rect(anchor, viewport);
        let row_h = self.row_height();
        cx.backend.save();
        cx.backend.clip_rect(popup);
        for (index, choice) in ImportMenuChoice::ALL.iter().enumerate() {
            let shortcut = choice.shortcut_label();
            let width = text_metrics::measure_chrome(cx.backend, shortcut, SHORTCUT_FONT_SIZE);
            let layout = TextLayout::single_run(
                shortcut,
                "system-ui",
                SHORTCUT_FONT_SIZE,
                self.theme.muted_foreground.to_jian(),
                Point2D::ZERO,
            );
            cx.backend.draw_text(
                &layout,
                Point2D::new(
                    popup.origin.x + popup.size.x - SHORTCUT_PAD_RIGHT - width,
                    popup.origin.y + row_h * (index as f32 + 0.5) + 4.0,
                ),
            );
        }
        cx.backend.restore();
    }

    fn items(&self) -> Vec<SelectItem<'static>> {
        ImportMenuChoice::ALL
            .iter()
            .map(|choice| SelectItem {
                label: choice.label(self.locale),
                // No persistent selection: the menu is a launcher, not
                // a setting, so no row paints the check mark.
                selected: false,
                disabled: false,
            })
            .collect()
    }
}

fn tokens_from_theme(theme: &Theme) -> Tokens {
    crate::widgets::button::tokens_from_theme(theme)
}

impl Widget for ImportMenu {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(IMPORT_MENU_WIDTH, self.panel_height()),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        self.paint_select(cx, popup_anchor(rect), rect);
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::ListBox);
        node.set_label("Import menu");
        node
    }
}

fn popup_anchor(popup: Rect) -> Rect {
    Rect {
        origin: popup.origin,
        size: Point2D::new(popup.size.x, 0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_menu() -> ImportMenu {
        let ui = EditorUiState {
            import_menu_open: true,
            import_menu: jian_widgets::components::select::SelectState {
                open: true,
                ..Default::default()
            },
            ..Default::default()
        };
        ImportMenu::for_editor_ui(&ui)
    }

    fn anchor() -> Rect {
        Rect {
            origin: Point2D::new(100.0, 40.0),
            size: Point2D::new(IMPORT_MENU_WIDTH, 32.0),
        }
    }

    fn viewport() -> Rect {
        Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(1200.0, 800.0),
        }
    }

    #[test]
    fn both_rows_hit_test_to_their_choice() {
        let menu = open_menu();
        let panel = menu.popup_rect(anchor(), viewport());
        let row_h = panel.size.y / ImportMenuChoice::ALL.len() as f32;
        let row_mid = |idx: usize| {
            Point2D::new(
                panel.origin.x + panel.size.x / 2.0,
                panel.origin.y + row_h * idx as f32 + row_h / 2.0,
            )
        };

        assert_eq!(
            menu.choice_at(anchor(), viewport(), row_mid(0)),
            Some(ImportMenuChoice::Figma)
        );
        assert_eq!(
            menu.choice_at(anchor(), viewport(), row_mid(1)),
            Some(ImportMenuChoice::Html)
        );
    }

    #[test]
    fn a_press_beyond_the_panel_is_outside() {
        let menu = open_menu();
        assert_eq!(
            menu.hit(anchor(), viewport(), Point2D::new(10.0, 700.0)),
            SelectHit::Outside
        );
        assert_eq!(
            menu.choice_at(anchor(), viewport(), Point2D::new(10.0, 700.0)),
            None
        );
    }

    #[test]
    fn a_closed_menu_swallows_nothing() {
        let menu = ImportMenu::for_editor_ui(&EditorUiState::default());
        let panel = menu.popup_rect(anchor(), viewport());
        let inside = Point2D::new(
            panel.origin.x + panel.size.x / 2.0,
            panel.origin.y + panel.size.y / 2.0,
        );
        assert_eq!(menu.hit(anchor(), viewport(), inside), SelectHit::Outside);
    }

    #[test]
    fn labels_fall_back_to_english_when_a_locale_lacks_the_keys() {
        // Every row must paint prose, never the raw lookup key.
        for locale in Locale::ALL {
            for choice in ImportMenuChoice::ALL {
                let label = choice.label(locale);
                assert!(
                    !label.starts_with("import."),
                    "{locale:?} {choice:?} painted the raw key {label}"
                );
            }
        }
    }

    #[test]
    fn rows_expose_their_documented_shortcuts() {
        assert_eq!(ImportMenuChoice::Figma.shortcut_label(), "⌘⇧F");
        assert_eq!(ImportMenuChoice::Html.shortcut_label(), "⌘⇧H");
    }
}
