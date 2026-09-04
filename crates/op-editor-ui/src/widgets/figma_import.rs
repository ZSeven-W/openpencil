//! Shared Figma / HTML import modal. The source selects its copy and
//! glyph while both modes share the same drop-zone interaction. The
//! host accepts direct file drops and routes drop-zone clicks through
//! the platform file picker.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::text_metrics;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Point2D, Rect, TextLayout};
use jian_widgets::components::button::{Button, ButtonVariant};
use jian_widgets::components::select::{
    Select, SelectHit, SelectItem, SelectState, MAX_VISIBLE_ROWS,
};
use op_editor_core::editor_ui_state::Locale;
use op_editor_core::EditorState;

pub const MODAL_WIDTH: f32 = 460.0;
pub const MODAL_HEIGHT: f32 = 260.0;
pub const PAGE_MODAL_HEIGHT: f32 = 400.0;
const PAD: f32 = 18.0;
const PAGE_LIST_TOP: f32 = 78.0;
const IMPORT_ALL_HEIGHT: f32 = 34.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FigmaImportHit {
    Close,
    DropZone,
    Page(usize),
    ImportAll,
    Outside,
    Inside,
}

pub struct FigmaImportModal {
    pub id: WidgetId,
    pub theme: Theme,
    /// Which source is being imported — selects the copy only; every
    /// rect and interaction is identical for both.
    source: op_editor_core::figma_import_state::ImportSource,
    /// Active UI locale — drives the modal's `t` copy lookup.
    locale: Locale,
    /// Which target the cursor is over — drives the hover wash.
    hover: Option<op_editor_core::FigmaImportButton>,
    /// Which target is currently pressed by the primary pointer.
    pressed: Option<op_editor_core::FigmaImportButton>,
    pages: Vec<op_editor_core::FigmaImportPage>,
    page_select: SelectState,
}

impl FigmaImportModal {
    pub fn for_editor(state: &EditorState) -> Self {
        let hover = state.editor_ui.figma_import_hover;
        let pressed = match state.editor_ui.pressed_button {
            Some(op_editor_core::ButtonPressTarget::FigmaImport(button)) => Some(button),
            _ => None,
        };
        let mut page_select = state.editor_ui.figma_import_page_select.clone();
        page_select.open = true;
        page_select.hover = match hover {
            Some(op_editor_core::FigmaImportButton::Page(index)) => Some(index),
            _ => None,
        };
        page_select.pressed = match pressed {
            Some(op_editor_core::FigmaImportButton::Page(index)) => Some(index),
            _ => None,
        };
        Self {
            id: WidgetId::new(5400),
            theme: theme_for(&state.editor_ui),
            source: state.editor_ui.import_source,
            locale: state.editor_ui.effective_locale(),
            hover,
            pressed,
            pages: state.editor_ui.figma_import_pages.clone(),
            page_select,
        }
    }

    pub fn page_selection_active(&self) -> bool {
        self.source == op_editor_core::figma_import_state::ImportSource::Figma
            && self.pages.len() > 1
    }

    pub fn rect(&self, viewport_w: f32, viewport_h: f32) -> Rect {
        let x = ((viewport_w - MODAL_WIDTH) / 2.0).max(16.0);
        let height = if self.page_selection_active() {
            PAGE_MODAL_HEIGHT
        } else {
            MODAL_HEIGHT
        };
        let y = ((viewport_h - height) / 2.0).max(crate::widgets::TOP_BAR_HEIGHT + 16.0);
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(MODAL_WIDTH, height),
        }
    }

    pub fn hit_test(&self, panel: Rect, point: Point2D) -> FigmaImportHit {
        if !(panel).contains(point) {
            return FigmaImportHit::Outside;
        }
        if (close_rect(panel)).contains(point) {
            return FigmaImportHit::Close;
        }
        if self.page_selection_active() {
            let tokens = crate::widgets::button::tokens_from_theme(&self.theme);
            return match Select::hit(
                &self.page_select,
                page_list_anchor(panel),
                page_list_viewport(panel, tokens.density.row_height()),
                self.pages.len(),
                point,
                &tokens,
            ) {
                SelectHit::Row(index) => FigmaImportHit::Page(index),
                SelectHit::Inside => FigmaImportHit::Inside,
                SelectHit::Outside
                    if import_all_rect(panel, tokens.density.row_height()).contains(point) =>
                {
                    FigmaImportHit::ImportAll
                }
                SelectHit::Outside => FigmaImportHit::Inside,
            };
        }
        if (drop_zone_rect(panel)).contains(point) {
            return FigmaImportHit::DropZone;
        }
        FigmaImportHit::Inside
    }

    pub fn page_list_rect(&self, panel: Rect) -> Option<Rect> {
        self.page_selection_active().then(|| {
            let tokens = crate::widgets::button::tokens_from_theme(&self.theme);
            Select::popup_rect(
                page_list_anchor(panel),
                page_list_viewport(panel, tokens.density.row_height()),
                self.pages.len(),
                &tokens,
            )
        })
    }

    pub fn max_page_scroll(&self) -> f32 {
        let row_height = crate::widgets::button::tokens_from_theme(&self.theme)
            .density
            .row_height();
        self.pages.len().saturating_sub(MAX_VISIBLE_ROWS) as f32 * row_height
    }
}

fn close_rect(panel: Rect) -> Rect {
    let s = 14.0;
    Rect {
        origin: Point2D::new(
            panel.origin.x + panel.size.x - 14.0 - s,
            panel.origin.y + 14.0,
        ),
        size: Point2D::new(s, s),
    }
}

fn drop_zone_rect(panel: Rect) -> Rect {
    let top = panel.origin.y + 44.0;
    let bottom = panel.origin.y + panel.size.y - 44.0;
    Rect {
        origin: Point2D::new(panel.origin.x + PAD, top),
        size: Point2D::new(panel.size.x - PAD * 2.0, bottom - top),
    }
}

fn page_list_anchor(panel: Rect) -> Rect {
    Rect::xywh(
        panel.origin.x + PAD,
        panel.origin.y + PAGE_LIST_TOP,
        panel.size.x - PAD * 2.0,
        0.0,
    )
}

fn page_list_viewport(panel: Rect, row_height: f32) -> Rect {
    Rect::xywh(
        panel.origin.x + PAD,
        panel.origin.y + PAGE_LIST_TOP,
        panel.size.x - PAD * 2.0,
        row_height * MAX_VISIBLE_ROWS as f32,
    )
}

fn import_all_rect(panel: Rect, row_height: f32) -> Rect {
    Rect::xywh(
        panel.origin.x + PAD,
        panel.origin.y + PAGE_LIST_TOP + row_height * MAX_VISIBLE_ROWS as f32 + 14.0,
        panel.size.x - PAD * 2.0,
        IMPORT_ALL_HEIGHT,
    )
}

fn t(
    locale: Locale,
    source: op_editor_core::figma_import_state::ImportSource,
    key: &str,
) -> &'static str {
    use op_editor_core::figma_import_state::ImportSource;
    let (title, drop, browse, footer) = match source {
        ImportSource::Figma => (
            "figma.title",
            "figma.dropFile",
            "figma.orBrowse",
            "figma.exportTip",
        ),
        ImportSource::Html => (
            "html.title",
            "html.dropFile",
            "html.orBrowse",
            "html.saveTip",
        ),
    };
    let lookup = match key {
        "title" => title,
        "drop" => drop,
        "browse" => browse,
        "footer" => footer,
        _ => return "",
    };
    let translated = op_i18n::translate(locale, lookup);
    if translated != lookup {
        return translated;
    }
    // A locale table without the HTML keys must never paint a raw key.
    match (source, key) {
        (ImportSource::Html, "title") => "Import HTML or web project",
        (ImportSource::Html, "drop") => "Drop an .html / .htm file or .zip project here",
        (ImportSource::Html, "browse") => "or click to choose a file",
        (ImportSource::Html, "footer") => ".zip packages can bundle CSS, images, and page assets",
        _ => translated,
    }
}

impl Widget for FigmaImportModal {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(MODAL_WIDTH, MODAL_HEIGHT),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_round_rect(rect, 12.0, self.theme.card);
        cx.backend
            .stroke_round_rect(rect, 12.0, self.theme.border, 1.0);

        let title = TextLayout::single_run(
            t(self.locale, self.source, "title"),
            "system-ui",
            14.0,
            (self.theme.foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &title,
            Point2D::new(rect.origin.x + PAD, rect.origin.y + 26.0),
        );

        // Close X — smaller stroke, tighter. Hover wash + foreground.
        let close = close_rect(rect);
        let close_hovered = self.hover == Some(op_editor_core::FigmaImportButton::Close);
        let close_pressed = self.pressed == Some(op_editor_core::FigmaImportButton::Close);
        // Pad the wash out a little so it reads as a button around
        // the 14 px glyph rather than hugging it.
        let pad = 5.0;
        let bg = Rect {
            origin: Point2D::new(close.origin.x - pad, close.origin.y - pad),
            size: Point2D::new(close.size.x + pad * 2.0, close.size.y + pad * 2.0),
        };
        jian_widgets::components::icon_button::IconButton {
            icon_paths: Icon::Close.paths(),
            hovered: close_hovered,
            pressed: close_pressed,
            active: false,
            enabled: true,
            icon_size: close.size.x,
            stroke_width: 1.6,
        }
        .paint(
            cx.backend,
            bg,
            &crate::widgets::button::tokens_from_theme(&self.theme),
        );

        if self.page_selection_active() {
            self.paint_page_selector(cx, rect);
            return;
        }

        // Compact source-specific import panel.
        let drop = drop_zone_rect(rect);
        cx.backend.fill_round_rect(drop, 10.0, self.theme.muted);
        // Brighten the browse drop-zone on hover (it's clickable).
        crate::widgets::button::paint_ghost_button_feedback(
            cx.backend,
            &self.theme,
            drop,
            self.hover == Some(op_editor_core::FigmaImportButton::DropZone),
            self.pressed == Some(op_editor_core::FigmaImportButton::DropZone),
        );
        cx.backend
            .stroke_round_rect(drop, 10.0, self.theme.border, 1.0);

        // Keep Figma branded while HTML and archive imports use a
        // neutral file glyph.
        let glyph_size = 24.0;
        let glyph_origin = Point2D::new(
            drop.origin.x + drop.size.x / 2.0 - glyph_size / 2.0,
            drop.origin.y + drop.size.y / 2.0 - glyph_size - 16.0,
        );
        match self.source {
            op_editor_core::figma_import_state::ImportSource::Figma => {
                crate::widgets::brand_icons::paint_figma_logo(
                    cx.backend,
                    glyph_origin,
                    glyph_size,
                    self.theme.muted_foreground,
                );
            }
            op_editor_core::figma_import_state::ImportSource::Html => draw_icon(
                cx.backend,
                Icon::FileText,
                glyph_origin,
                glyph_size,
                self.theme.muted_foreground,
                1.8,
            ),
        }

        let headline = t(self.locale, self.source, "drop");
        let head_w = text_metrics::measure_chrome(cx.backend, headline, 13.0);
        let head_layout = TextLayout::single_run(
            headline,
            "system-ui",
            13.0,
            (self.theme.foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &head_layout,
            Point2D::new(
                drop.origin.x + (drop.size.x - head_w) / 2.0,
                drop.origin.y + drop.size.y / 2.0 + 12.0,
            ),
        );

        let sub = t(self.locale, self.source, "browse");
        let sub_w = text_metrics::measure_chrome(cx.backend, sub, 11.0);
        let sub_layout = TextLayout::single_run(
            sub,
            "system-ui",
            11.0,
            (self.theme.muted_foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &sub_layout,
            Point2D::new(
                drop.origin.x + (drop.size.x - sub_w) / 2.0,
                drop.origin.y + drop.size.y / 2.0 + 30.0,
            ),
        );

        let footer = TextLayout::single_run(
            t(self.locale, self.source, "footer"),
            "system-ui",
            11.0,
            (self.theme.muted_foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &footer,
            Point2D::new(rect.origin.x + PAD, rect.origin.y + rect.size.y - 16.0),
        );
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Dialog);
        node.set_label(t(self.locale, self.source, "title"));
        node
    }
}

impl FigmaImportModal {
    fn paint_page_selector(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        let instruction = op_i18n::translate(self.locale, "figma.selectPage")
            .replace("{{count}}", &self.pages.len().to_string());
        let instruction_layout = TextLayout::single_run(
            &instruction,
            "system-ui",
            11.0,
            self.theme.muted_foreground.to_jian(),
            Point2D::ZERO,
        );
        cx.backend.draw_text(
            &instruction_layout,
            Point2D::new(rect.origin.x + PAD, rect.origin.y + 58.0),
        );

        let labels: Vec<String> = self
            .pages
            .iter()
            .map(|page| {
                let layers = op_i18n::translate(self.locale, "figma.layers")
                    .replace("{{count}}", &page.layer_count.to_string());
                format!("{}  ·  {layers}", page.name)
            })
            .collect();
        let items: Vec<SelectItem<'_>> = labels
            .iter()
            .map(|label| SelectItem {
                label,
                selected: false,
                disabled: false,
            })
            .collect();
        let tokens = crate::widgets::button::tokens_from_theme(&self.theme);
        Select {
            state: &self.page_select,
            items: &items,
        }
        .paint(
            cx.backend,
            page_list_anchor(rect),
            page_list_viewport(rect, tokens.density.row_height()),
            &tokens,
        );

        Button {
            label: op_i18n::translate(self.locale, "figma.importAll"),
            icon_paths: None,
            variant: ButtonVariant::Primary,
            enabled: true,
            hovered: self.hover == Some(op_editor_core::FigmaImportButton::ImportAll),
            pressed: self.pressed == Some(op_editor_core::FigmaImportButton::ImportAll),
            font_size: 12.0,
        }
        .paint(
            cx.backend,
            import_all_rect(rect, tokens.density.row_height()),
            &tokens,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, RenderBackend};

    #[derive(Default)]
    struct CaptureBackend {
        round_fills: Vec<(Rect, f32, Color)>,
        filled_svg_paths: usize,
        stroked_svg_paths: usize,
    }

    impl RenderBackend for CaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
            self.round_fills.push((rect, radius, color));
        }
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {
            self.stroked_svg_paths += 1;
        }
        fn fill_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: f32, _: Color) {
            self.filled_svg_paths += 1;
        }
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

    fn page_selector(page_count: usize, scroll: f32) -> FigmaImportModal {
        let mut state = EditorState::new();
        state.editor_ui.figma_import_open = true;
        state.editor_ui.figma_import_pages = (0..page_count)
            .map(|index| op_editor_core::FigmaImportPage {
                name: format!("Page {}", index + 1),
                layer_count: index,
            })
            .collect();
        state.editor_ui.figma_import_page_select.open = true;
        state.editor_ui.figma_import_page_select.scroll.offset = scroll;
        FigmaImportModal::for_editor(&state)
    }

    #[test]
    fn clicking_drop_zone_requests_figma_import() {
        let modal = FigmaImportModal::for_editor(&EditorState::new());
        let panel = modal.rect(800.0, 600.0);
        let drop = drop_zone_rect(panel);
        let point = Point2D::new(
            drop.origin.x + drop.size.x / 2.0,
            drop.origin.y + drop.size.y / 2.0,
        );

        assert_eq!(modal.hit_test(panel, point), FigmaImportHit::DropZone);
    }

    #[test]
    fn prepared_page_rows_and_import_all_have_distinct_hits() {
        let modal = page_selector(70, 0.0);
        let panel = modal.rect(900.0, 700.0);
        let list = modal.page_list_rect(panel).expect("page list");
        let first_row = Point2D::new(list.origin.x + 20.0, list.origin.y + 15.0);
        let row_height = crate::widgets::button::tokens_from_theme(&modal.theme)
            .density
            .row_height();
        let all = import_all_rect(panel, row_height);

        assert_eq!(modal.hit_test(panel, first_row), FigmaImportHit::Page(0));
        assert_eq!(
            modal.hit_test(
                panel,
                Point2D::new(all.origin.x + 20.0, all.origin.y + all.size.y / 2.0)
            ),
            FigmaImportHit::ImportAll
        );
    }

    #[test]
    fn page_hit_respects_select_scroll_offset() {
        let modal = page_selector(70, 300.0);
        let panel = modal.rect(900.0, 700.0);
        let list = modal.page_list_rect(panel).expect("page list");
        let first_visible = Point2D::new(list.origin.x + 20.0, list.origin.y + 15.0);

        assert_eq!(
            modal.hit_test(panel, first_visible),
            FigmaImportHit::Page(10)
        );
        assert_eq!(modal.max_page_scroll(), 62.0 * 30.0);
    }

    #[test]
    fn pressed_close_uses_shared_button_feedback() {
        let mut state = EditorState::new();
        state.editor_ui.pressed_button = Some(op_editor_core::ButtonPressTarget::FigmaImport(
            op_editor_core::FigmaImportButton::Close,
        ));
        let modal = FigmaImportModal::for_editor(&state);
        let panel = modal.rect(800.0, 600.0);
        let close = close_rect(panel);
        let pad = 5.0;
        let bg = Rect {
            origin: Point2D::new(close.origin.x - pad, close.origin.y - pad),
            size: Point2D::new(close.size.x + pad * 2.0, close.size.y + pad * 2.0),
        };
        let expected = modal
            .theme
            .button_hover
            .with_alpha(modal.theme.button_hover.a * 1.8);
        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        modal.paint(&mut cx, panel);

        assert!(
            backend.round_fills.iter().any(|(rect, radius, color)| {
                *rect == bg && (*radius - 6.0).abs() < 0.01 && color_close(*color, expected)
            }),
            "pressed close button should paint the shared pressed feedback token"
        );
    }

    #[test]
    fn html_modal_uses_a_neutral_file_icon_instead_of_figma_brand_paths() {
        let mut state = EditorState::new();
        state.editor_ui.import_source = op_editor_core::figma_import_state::ImportSource::Html;
        let modal = FigmaImportModal::for_editor(&state);
        let panel = modal.rect(800.0, 600.0);
        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        modal.paint(&mut cx, panel);

        assert_eq!(backend.filled_svg_paths, 0);
        assert!(
            backend.stroked_svg_paths > Icon::Close.paths().len(),
            "HTML modal should paint a neutral file glyph in addition to the close icon"
        );
    }

    #[test]
    fn figma_modal_keeps_its_brand_paths() {
        let modal = FigmaImportModal::for_editor(&EditorState::new());
        let panel = modal.rect(800.0, 600.0);
        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        modal.paint(&mut cx, panel);

        assert_eq!(backend.filled_svg_paths, 5);
    }
}
