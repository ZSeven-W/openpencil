//! Native Iconify picker used by the toolbar and icon property row.

use crate::theme::Theme;
use crate::widgets::button::paint_button_feedback_wash;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::icon_catalog::{search_icons, IconCatalogEntry, IconRenderStyle};
use crate::widgets::{
    draw_icon, draw_icon_catalog_entry, draw_icon_data, Icon, IconPathData, PaintCx,
};
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::{EditorState, IconPickerRemoteIcon, Locale};

pub const ICON_PICKER_PANEL_W: f32 = 320.0;
pub const ICON_PICKER_PANEL_H: f32 = 420.0;
pub const ICON_PICKER_CLOSE_HOVER: usize = usize::MAX - 1;
pub const ICON_PICKER_LOAD_MORE_HOVER: usize = usize::MAX;
const PAD: f32 = 14.0;
const HEADER_H: f32 = 40.0;
const SEARCH_H: f32 = 42.0;
const CLOSE_BTN: f32 = 24.0;
const ROW_H: f32 = 34.0;
const ICON_SIZE: f32 = 17.0;
const ROW_PAD_X: f32 = 10.0;
const CHAR_W: f32 = 6.0;
const LOCAL_LIMIT: usize = 120;
pub const ICONIFY_LOAD_MORE_LIMIT: usize = 48;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IconPickerHit {
    Close,
    DragHeader,
    SelectIcon { collection: String, name: String },
    LoadMore,
    Inside,
}

enum IconRow<'a> {
    Local(&'static IconCatalogEntry),
    Remote(&'a IconPickerRemoteIcon),
}

impl IconRow<'_> {
    fn collection(&self) -> &str {
        match self {
            IconRow::Local(i) => &i.collection,
            IconRow::Remote(i) => &i.collection,
        }
    }

    fn name(&self) -> &str {
        match self {
            IconRow::Local(i) => &i.name,
            IconRow::Remote(i) => &i.name,
        }
    }
}

pub struct IconPickerPanel<'a> {
    state: &'a EditorState,
    theme: Theme,
    locale: Locale,
    /// Which target the cursor is over — drives the hover wash.
    hover: Option<usize>,
    /// Which target is actively pressed — touch-compatible feedback.
    pressed: Option<usize>,
    /// Frame clock for the search field's caret blink.
    now_ms: u64,
}

impl<'a> IconPickerPanel<'a> {
    pub fn for_editor(state: &'a EditorState) -> Option<IconPickerPanel<'a>> {
        Self::for_editor_at(state, 0)
    }

    pub fn for_editor_at(state: &'a EditorState, now_ms: u64) -> Option<IconPickerPanel<'a>> {
        if !state.editor_ui.icon_picker.open {
            return None;
        }
        Some(IconPickerPanel {
            state,
            theme: theme_for(&state.editor_ui),
            locale: state.editor_ui.locale,
            hover: state.editor_ui.icon_picker.hover,
            pressed: state.editor_ui.icon_picker.pressed,
            now_ms,
        })
    }

    /// Resolve a pointer to a hoverable target (close / row / load-more).
    /// Mirrors [`Self::hit_test`]'s row math; rows are index-addressable
    /// so no string ids leak into the hover state.
    pub fn hover_at(&self, panel: Rect, point: Point2D) -> Option<usize> {
        if !(panel).contains(point) {
            return None;
        }
        if (Self::close_rect(panel)).contains(point) {
            return Some(ICON_PICKER_CLOSE_HOVER);
        }
        if point.y <= panel.origin.y + HEADER_H {
            return None;
        }
        let list_top = Self::list_top(panel);
        if point.y >= list_top {
            let row = ((point.y - list_top) / ROW_H) as usize;
            let capacity = Self::visible_row_capacity(panel);
            let has_more = self.has_load_more();
            let item_cap = capacity.saturating_sub(usize::from(has_more));
            let items = self.rows(item_cap);
            if row < items.len() {
                return Some(row);
            }
            if has_more && row == items.len() && !self.state.editor_ui.icon_picker_remote.loading {
                return Some(ICON_PICKER_LOAD_MORE_HOVER);
            }
        }
        None
    }

    fn t(&self, key: &'static str) -> &'static str {
        crate::i18n::translate(self.locale, key)
    }

    fn query(&self) -> String {
        self.state
            .editor_ui
            .icon_picker_search
            .trim()
            .to_lowercase()
    }

    fn rows(&self, limit: usize) -> Vec<IconRow<'a>> {
        let query = self.query();
        let mut rows: Vec<IconRow<'a>> = search_icons(&query, LOCAL_LIMIT)
            .into_iter()
            .map(IconRow::Local)
            .collect();
        let remote = &self.state.editor_ui.icon_picker_remote;
        if !query.is_empty() && remote.query == query {
            for icon in &remote.icons {
                if !rows
                    .iter()
                    .any(|row| row.collection() == icon.collection && row.name() == icon.name)
                {
                    rows.push(IconRow::Remote(icon));
                }
            }
        }
        rows.truncate(limit);
        rows
    }

    fn has_load_more(&self) -> bool {
        let query = self.query();
        if query.is_empty() {
            return false;
        }
        let remote = &self.state.editor_ui.icon_picker_remote;
        if remote.loading {
            return true;
        }
        remote.query != query || remote.next_start < remote.total
    }

    fn close_rect(panel: Rect) -> Rect {
        let y = panel.origin.y + (HEADER_H - CLOSE_BTN) / 2.0;
        Rect {
            origin: Point2D::new(panel.origin.x + panel.size.x - PAD - CLOSE_BTN, y),
            size: Point2D::new(CLOSE_BTN, CLOSE_BTN),
        }
    }

    fn search_rect(panel: Rect) -> Rect {
        Rect {
            origin: Point2D::new(panel.origin.x + PAD, panel.origin.y + HEADER_H + 6.0),
            size: Point2D::new(panel.size.x - PAD * 2.0, 28.0),
        }
    }

    fn list_top(panel: Rect) -> f32 {
        panel.origin.y + HEADER_H + SEARCH_H
    }

    fn visible_row_capacity(panel: Rect) -> usize {
        ((panel.size.y - HEADER_H - SEARCH_H - PAD) / ROW_H)
            .floor()
            .max(0.0) as usize
    }

    pub fn hit_test(&self, panel: Rect, point: Point2D) -> Option<IconPickerHit> {
        if !(panel).contains(point) {
            return None;
        }
        if (Self::close_rect(panel)).contains(point) {
            return Some(IconPickerHit::Close);
        }
        if point.y <= panel.origin.y + HEADER_H {
            return Some(IconPickerHit::DragHeader);
        }
        let list_top = Self::list_top(panel);
        if point.y >= list_top {
            let row = ((point.y - list_top) / ROW_H) as usize;
            let capacity = Self::visible_row_capacity(panel);
            let has_more = self.has_load_more();
            let item_cap = capacity.saturating_sub(usize::from(has_more));
            let items = self.rows(item_cap);
            if row < items.len() {
                let item = &items[row];
                return Some(IconPickerHit::SelectIcon {
                    collection: item.collection().to_string(),
                    name: item.name().to_string(),
                });
            }
            if has_more && row == items.len() && !self.state.editor_ui.icon_picker_remote.loading {
                return Some(IconPickerHit::LoadMore);
            }
        }
        Some(IconPickerHit::Inside)
    }

    pub fn paint(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        cx.backend.fill_round_rect(panel, 8.0, self.theme.popover);
        cx.backend
            .stroke_round_rect(panel, 8.0, self.theme.border, 1.0);
        self.paint_header(cx, panel);
        self.paint_search(cx, panel);

        let rows = Self::visible_row_capacity(panel);
        let has_more = self.has_load_more();
        let item_cap = rows.saturating_sub(usize::from(has_more));
        let filtered = self.rows(item_cap);
        if filtered.is_empty() && !has_more {
            self.paint_empty(cx, panel);
            return;
        }
        for (idx, item) in filtered.iter().enumerate() {
            self.paint_row(cx, panel, idx, item);
        }
        if has_more && rows > 0 {
            self.paint_load_more(cx, panel, filtered.len());
        }
    }

    fn paint_header(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let title = TextLayout::single_run(
            self.t("icon.title"),
            "system-ui",
            13.0,
            (self.theme.foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &title,
            Point2D::new(panel.origin.x + PAD, panel.origin.y + 25.0),
        );
        let close = Self::close_rect(panel);
        let close_hovered = self.hover == Some(ICON_PICKER_CLOSE_HOVER);
        let close_pressed = self.pressed == Some(ICON_PICKER_CLOSE_HOVER);
        jian_widgets::components::icon_button::IconButton {
            icon_paths: Icon::Close.paths(),
            hovered: close_hovered,
            pressed: close_pressed,
            active: false,
            enabled: true,
            icon_size: 14.0,
            stroke_width: 1.4,
        }
        .paint(
            cx.backend,
            close,
            &crate::widgets::button::tokens_from_theme(&self.theme),
        );
    }

    fn paint_search(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let search = Self::search_rect(panel);
        cx.backend.fill_round_rect(search, 6.0, self.theme.muted);
        draw_icon(
            cx.backend,
            Icon::Search,
            Point2D::new(search.origin.x + 8.0, search.origin.y + 6.0),
            16.0,
            self.theme.muted_foreground,
            1.4,
        );
        // Value + placeholder + caret + select-all highlight render through the
        // unified jian TextInputView (family-aware caret, no hand-rolled drift).
        // The buffer is rebuilt from the filter String each frame; the picker
        // owns the keyboard while open, so it reads as focused.
        let placeholder = self.t("icon.searchIcons");
        let mut search_input = jian_core::text_input::TextInputState::with_text(
            self.state.editor_ui.icon_picker_search.clone(),
        );
        if self.state.editor_ui.icon_picker_select_all && !search_input.text().is_empty() {
            search_input.select_all();
        }
        crate::widgets::property_panel_text_input::paint_text_input_view(
            cx,
            &self.theme,
            &search_input,
            search,
            12.0,
            30.0,
            search.origin.y + 18.0,
            self.now_ms,
            placeholder,
            true,
        );
    }

    fn paint_empty(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let empty = TextLayout::single_run(
            self.t("icon.noIconsFound"),
            "system-ui",
            12.0,
            (self.theme.muted_foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &empty,
            Point2D::new(panel.origin.x + PAD, Self::list_top(panel) + 28.0),
        );
    }

    fn paint_row(&self, cx: &mut PaintCx<'_>, panel: Rect, idx: usize, item: &IconRow<'_>) {
        let y = Self::list_top(panel) + idx as f32 * ROW_H;
        let row = Rect {
            origin: Point2D::new(panel.origin.x + 6.0, y),
            size: Point2D::new(panel.size.x - 12.0, ROW_H),
        };
        cx.backend.fill_round_rect(row, 6.0, self.theme.popover);
        paint_button_feedback_wash(
            cx.backend,
            &self.theme,
            row,
            6.0,
            self.hover == Some(idx),
            self.pressed == Some(idx),
        );
        let icon_pos = Point2D::new(row.origin.x + ROW_PAD_X, y + (ROW_H - ICON_SIZE) / 2.0);
        match item {
            IconRow::Local(icon) => draw_icon_catalog_entry(
                cx.backend,
                icon,
                icon_pos,
                ICON_SIZE,
                self.theme.foreground,
                1.5,
            ),
            IconRow::Remote(icon) => draw_icon_data(
                cx.backend,
                IconPathData {
                    d: &icon.d,
                    style: remote_style(&icon.style),
                    viewbox: icon.width.max(icon.height),
                },
                icon_pos,
                ICON_SIZE,
                self.theme.foreground,
                1.5,
            ),
        }
        let label = truncate(
            &format!("{}:{}", item.collection(), item.name()),
            ((row.size.x - 54.0) / CHAR_W) as usize,
        );
        let text = TextLayout::single_run(
            &label,
            "system-ui",
            12.0,
            (self.theme.foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&text, Point2D::new(row.origin.x + 38.0, y + 21.0));
    }

    fn paint_load_more(&self, cx: &mut PaintCx<'_>, panel: Rect, idx: usize) {
        let y = Self::list_top(panel) + idx as f32 * ROW_H;
        let row = Rect {
            origin: Point2D::new(panel.origin.x + 6.0, y + 2.0),
            size: Point2D::new(panel.size.x - 12.0, ROW_H - 4.0),
        };
        cx.backend.fill_round_rect(row, 6.0, self.theme.muted);
        paint_button_feedback_wash(
            cx.backend,
            &self.theme,
            row,
            6.0,
            self.hover == Some(ICON_PICKER_LOAD_MORE_HOVER),
            self.pressed == Some(ICON_PICKER_LOAD_MORE_HOVER),
        );
        let label = if self.state.editor_ui.icon_picker_remote.loading {
            "..."
        } else {
            self.t("git.history.loadMore")
        };
        let text = TextLayout::single_run(
            label,
            "system-ui",
            12.0,
            (self.theme.foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &text,
            Point2D::new(row.origin.x + ROW_PAD_X, row.origin.y + 20.0),
        );
    }
}

fn remote_style(style: &str) -> IconRenderStyle {
    if style == "stroke" {
        IconRenderStyle::Stroke
    } else {
        IconRenderStyle::Fill
    }
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    out.push_str("...");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, RenderBackend};

    #[derive(Default)]
    struct CaptureBackend {
        round_fills: Vec<(Rect, f32, Color)>,
    }

    impl RenderBackend for CaptureBackend {
        fn begin_frame(&mut self) {}

        fn end_frame(&mut self) {}

        fn fill_rect(&mut self, _rect: Rect, _color: Color) {}

        fn stroke_rect(&mut self, _rect: Rect, _color: Color, _width: f32) {}

        fn draw_text(&mut self, _layout: &TextLayout, _origin: Point2D) {}

        fn clip_rect(&mut self, _rect: Rect) {}

        fn stroke_line(&mut self, _from: Point2D, _to: Point2D, _color: Color, _width: f32) {}

        fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
            self.round_fills.push((rect, radius, color));
        }

        fn stroke_round_rect(&mut self, _rect: Rect, _radius: f32, _color: Color, _width: f32) {}

        fn stroke_svg_path(
            &mut self,
            _d: &str,
            _top_left: Point2D,
            _size: f32,
            _color: Color,
            _width: f32,
        ) {
        }

        fn save(&mut self) {}

        fn restore(&mut self) {}

        fn translate(&mut self, _offset: Point2D) {}

        fn resize(&mut self, _width: u32, _height: u32) {}

        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    #[test]
    fn hover_at_uses_select_state_indices_for_close_and_rows() {
        let mut state = EditorState::default();
        state.editor_ui.open_icon_picker(false);
        let panel = IconPickerPanel::for_editor(&state).unwrap();
        let rect = Rect::xywh(0.0, 0.0, ICON_PICKER_PANEL_W, ICON_PICKER_PANEL_H);
        let close = IconPickerPanel::close_rect(rect);

        assert_eq!(
            panel.hover_at(
                rect,
                Point2D::new(close.origin.x + close.size.x / 2.0, close.origin.y + 4.0)
            ),
            Some(ICON_PICKER_CLOSE_HOVER)
        );
        assert_eq!(
            panel.hover_at(
                rect,
                Point2D::new(
                    rect.origin.x + 20.0,
                    IconPickerPanel::list_top(rect) + ROW_H / 2.0
                )
            ),
            Some(0)
        );
    }

    #[test]
    fn for_editor_picks_up_pressed_select_state() {
        let mut state = EditorState::default();
        state.editor_ui.open_icon_picker(false);
        state.editor_ui.icon_picker.pressed = Some(ICON_PICKER_CLOSE_HOVER);

        let panel = IconPickerPanel::for_editor(&state).unwrap();

        assert_eq!(panel.pressed, Some(ICON_PICKER_CLOSE_HOVER));
    }

    #[test]
    fn pressed_close_button_paints_pressed_feedback() {
        let mut state = EditorState::default();
        state.editor_ui.open_icon_picker(false);
        state.editor_ui.icon_picker.pressed = Some(ICON_PICKER_CLOSE_HOVER);
        let panel = IconPickerPanel::for_editor(&state).unwrap();
        let rect = Rect::xywh(0.0, 0.0, ICON_PICKER_PANEL_W, ICON_PICKER_PANEL_H);
        let close = IconPickerPanel::close_rect(rect);
        let expected = panel
            .theme
            .button_hover
            .with_alpha(panel.theme.button_hover.a * 1.8);
        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        panel.paint(&mut cx, rect);

        assert!(
            backend
                .round_fills
                .iter()
                .any(|(fill, radius, color)| *fill == close
                    && *radius == 6.0
                    && *color == expected),
            "pressed close button should paint shared pressed feedback"
        );
    }

    #[test]
    fn pressed_load_more_paints_pressed_feedback() {
        let mut state = EditorState::default();
        state.editor_ui.open_icon_picker(false);
        state.editor_ui.icon_picker_search = "unlikely-remote-only".to_string();
        state.editor_ui.icon_picker.pressed = Some(ICON_PICKER_LOAD_MORE_HOVER);
        let panel = IconPickerPanel::for_editor(&state).unwrap();
        let rect = Rect::xywh(0.0, 0.0, ICON_PICKER_PANEL_W, ICON_PICKER_PANEL_H);
        let rows = IconPickerPanel::visible_row_capacity(rect);
        let filtered = panel.rows(rows.saturating_sub(1));
        let y = IconPickerPanel::list_top(rect) + filtered.len() as f32 * ROW_H;
        let load_more = Rect {
            origin: Point2D::new(rect.origin.x + 6.0, y + 2.0),
            size: Point2D::new(rect.size.x - 12.0, ROW_H - 4.0),
        };
        let expected = panel
            .theme
            .button_hover
            .with_alpha(panel.theme.button_hover.a * 1.8);
        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        panel.paint(&mut cx, rect);

        assert!(
            backend
                .round_fills
                .iter()
                .any(|(fill, radius, color)| *fill == load_more
                    && *radius == 6.0
                    && *color == expected),
            "pressed load-more button should paint shared pressed feedback"
        );
    }
}
