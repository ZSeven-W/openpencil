//! Native Iconify picker used by the toolbar and icon property row.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::icon_catalog::{search_icons, IconCatalogEntry, IconRenderStyle};
use crate::widgets::{
    draw_icon, draw_icon_catalog_entry, draw_icon_data, Icon, IconPathData, PaintCx,
};
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::{EditorState, IconPickerRemoteIcon, Locale};

pub const ICON_PICKER_PANEL_W: f32 = 320.0;
pub const ICON_PICKER_PANEL_H: f32 = 420.0;
const PAD: f32 = 14.0;
const HEADER_H: f32 = 40.0;
const SEARCH_H: f32 = 42.0;
const CLOSE_BTN: f32 = 24.0;
const GRID_COLS: usize = 6;
const GRID_CELL: f32 = 40.0;
const GRID_GAP: f32 = 8.0;
const GRID_PITCH: f32 = GRID_CELL + GRID_GAP;
const GRID_ICON: f32 = 18.0;
const LOAD_MORE_H: f32 = 32.0;
const LOAD_MORE_GAP: f32 = 8.0;
const LOAD_MORE_INSET: f32 = 10.0;
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
    now_ms: u64,
}

impl<'a> IconPickerPanel<'a> {
    pub fn for_editor(state: &'a EditorState) -> Option<IconPickerPanel<'a>> {
        Self::for_editor_at(state, 0)
    }

    pub fn for_editor_at(state: &'a EditorState, now_ms: u64) -> Option<IconPickerPanel<'a>> {
        if !state.editor_ui.icon_picker_open {
            return None;
        }
        Some(IconPickerPanel {
            state,
            theme: theme_for(&state.editor_ui),
            locale: state.editor_ui.locale,
            now_ms,
        })
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

    fn all_rows(&self) -> Vec<IconRow<'a>> {
        self.rows(usize::MAX)
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

    fn footer_visible(&self) -> bool {
        self.state.editor_ui.icon_picker_search.trim().is_empty() || self.has_load_more()
    }

    fn footer_can_load_more(&self) -> bool {
        !self.state.editor_ui.icon_picker_search.trim().is_empty()
            && self.has_load_more()
            && !self.state.editor_ui.icon_picker_remote.loading
    }

    fn search_caret_visible(&self) -> bool {
        jian_core::anim::blink_visible(
            self.now_ms,
            self.state.editor_ui.icon_picker_caret_anchor_ms,
            500,
        )
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

    fn list_rect(panel: Rect) -> Rect {
        Self::list_rect_for(panel, false)
    }

    fn list_rect_for(panel: Rect, footer_visible: bool) -> Rect {
        let top = Self::list_top(panel);
        let footer_h = if footer_visible {
            LOAD_MORE_H + LOAD_MORE_GAP
        } else {
            0.0
        };
        Rect {
            origin: Point2D::new(panel.origin.x + 6.0, top),
            size: Point2D::new(
                panel.size.x - 12.0,
                (panel.origin.y + panel.size.y - PAD - footer_h - top).max(0.0),
            ),
        }
    }

    fn grid_origin(panel: Rect) -> Point2D {
        let list = Self::list_rect(panel);
        let grid_w = GRID_COLS as f32 * GRID_CELL + (GRID_COLS.saturating_sub(1)) as f32 * GRID_GAP;
        Point2D::new(
            list.origin.x + (list.size.x - grid_w).max(0.0) / 2.0,
            list.origin.y,
        )
    }

    fn grid_rows(item_count: usize) -> usize {
        item_count.div_ceil(GRID_COLS)
    }

    fn cell_rect(panel: Rect, index: usize) -> Rect {
        let origin = Self::grid_origin(panel);
        let col = index % GRID_COLS;
        let row = index / GRID_COLS;
        Rect {
            origin: Point2D::new(
                origin.x + col as f32 * GRID_PITCH,
                origin.y + row as f32 * GRID_PITCH,
            ),
            size: Point2D::new(GRID_CELL, GRID_CELL),
        }
    }

    fn load_more_rect(panel: Rect) -> Rect {
        Rect {
            origin: Point2D::new(
                panel.origin.x + PAD + LOAD_MORE_INSET,
                panel.origin.y + panel.size.y - PAD - LOAD_MORE_H,
            ),
            size: Point2D::new(panel.size.x - (PAD + LOAD_MORE_INSET) * 2.0, LOAD_MORE_H),
        }
    }

    fn content_height(item_count: usize) -> f32 {
        Self::grid_rows(item_count) as f32 * GRID_PITCH
    }

    fn max_scroll_for(panel: Rect, item_count: usize, footer_visible: bool) -> f32 {
        (Self::content_height(item_count) - Self::list_rect_for(panel, footer_visible).size.y)
            .max(0.0)
    }

    #[cfg(test)]
    fn visible_index_range(panel: Rect, item_count: usize, scroll: f32) -> std::ops::Range<usize> {
        Self::visible_index_range_for_list(Self::list_rect(panel), item_count, scroll)
    }

    fn visible_index_range_for_list(
        list: Rect,
        item_count: usize,
        scroll: f32,
    ) -> std::ops::Range<usize> {
        if item_count == 0 {
            return 0..0;
        }
        let start_row = (scroll / GRID_PITCH).floor().max(0.0) as usize;
        let row_count = (list.size.y / GRID_PITCH).ceil() as usize + 2;
        let start = (start_row * GRID_COLS).min(item_count);
        let end = ((start_row + row_count) * GRID_COLS).min(item_count);
        start..end
    }

    pub fn max_scroll(&self, panel: Rect) -> f32 {
        let items = self.all_rows().len();
        Self::max_scroll_for(panel, items, self.footer_visible())
    }

    fn scroll_for(&self, panel: Rect, item_count: usize, footer_visible: bool) -> f32 {
        self.state
            .editor_ui
            .icon_picker_scroll
            .clamp(0.0, Self::max_scroll_for(panel, item_count, footer_visible))
    }

    pub fn hit_test(&self, panel: Rect, point: Point2D) -> Option<IconPickerHit> {
        if !rect_contains(panel, point) {
            return None;
        }
        if rect_contains(Self::close_rect(panel), point) {
            return Some(IconPickerHit::Close);
        }
        if point.y <= panel.origin.y + HEADER_H {
            return Some(IconPickerHit::DragHeader);
        }
        let footer_visible = self.footer_visible();
        if footer_visible && rect_contains(Self::load_more_rect(panel), point) {
            return if self.footer_can_load_more() {
                Some(IconPickerHit::LoadMore)
            } else {
                Some(IconPickerHit::Inside)
            };
        }
        let list = Self::list_rect_for(panel, footer_visible);
        if rect_contains(list, point) {
            let items = self.all_rows();
            let scrolled = Point2D::new(
                point.x,
                point.y + self.scroll_for(panel, items.len(), footer_visible),
            );
            if let Some(index) = Self::cell_index_at(panel, scrolled, items.len()) {
                let item = &items[index];
                return Some(IconPickerHit::SelectIcon {
                    collection: item.collection().to_string(),
                    name: item.name().to_string(),
                });
            }
        }
        Some(IconPickerHit::Inside)
    }

    fn cell_index_at(panel: Rect, point: Point2D, item_count: usize) -> Option<usize> {
        let origin = Self::grid_origin(panel);
        let local_x = point.x - origin.x;
        let local_y = point.y - origin.y;
        if local_x < 0.0 || local_y < 0.0 {
            return None;
        }
        let col = (local_x / GRID_PITCH).floor() as usize;
        let row = (local_y / GRID_PITCH).floor() as usize;
        if col >= GRID_COLS {
            return None;
        }
        let index = row * GRID_COLS + col;
        if index >= item_count || !rect_contains(Self::cell_rect(panel, index), point) {
            return None;
        }
        Some(index)
    }

    pub fn paint(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        cx.backend.fill_round_rect(panel, 8.0, self.theme.popover);
        cx.backend
            .stroke_round_rect(panel, 8.0, self.theme.border, 1.0);
        self.paint_header(cx, panel);
        self.paint_search(cx, panel);

        let footer_visible = self.footer_visible();
        let filtered = self.all_rows();
        if filtered.is_empty() && !footer_visible {
            self.paint_empty(cx, panel);
            return;
        }
        let list = Self::list_rect_for(panel, footer_visible);
        let scroll = self.scroll_for(panel, filtered.len(), footer_visible);
        cx.backend.save();
        cx.backend.clip_rect(list);
        cx.backend.translate(Point2D::new(0.0, -scroll));
        for idx in Self::visible_index_range_for_list(list, filtered.len(), scroll) {
            self.paint_cell(cx, panel, idx, &filtered[idx]);
        }
        cx.backend.restore();
        if footer_visible {
            self.paint_load_more(cx, panel);
        }
        self.paint_scrollbar(cx, panel, filtered.len(), footer_visible, scroll);
    }

    fn paint_header(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let title = TextLayout::single_run(
            self.t("icon.title"),
            "system-ui",
            13.0,
            to_jian(self.theme.foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &title,
            Point2D::new(panel.origin.x + PAD, panel.origin.y + 25.0),
        );
        let close = Self::close_rect(panel);
        draw_icon(
            cx.backend,
            Icon::Close,
            Point2D::new(close.origin.x + 5.0, close.origin.y + 5.0),
            14.0,
            self.theme.muted_foreground,
            1.4,
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
        let raw_query = self.state.editor_ui.icon_picker_search.trim();
        let search_text = if raw_query.is_empty() {
            self.t("icon.searchIcons")
        } else {
            raw_query
        };
        let color = if raw_query.is_empty() {
            self.theme.muted_foreground
        } else {
            self.theme.foreground
        };
        let layout = TextLayout::single_run(
            search_text,
            "system-ui",
            12.0,
            to_jian(color),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &layout,
            Point2D::new(search.origin.x + 30.0, search.origin.y + 18.0),
        );
        if self.search_caret_visible() {
            let text_x = search.origin.x + 30.0;
            let caret_x = text_x + cx.backend.measure_text(raw_query, 12.0);
            let caret = Rect {
                origin: Point2D::new(caret_x + 1.0, search.origin.y + 7.0),
                size: Point2D::new(1.5, 15.0),
            };
            cx.backend.fill_rect(caret, self.theme.foreground);
        }
    }

    fn paint_empty(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let empty = TextLayout::single_run(
            self.t("icon.noIconsFound"),
            "system-ui",
            12.0,
            to_jian(self.theme.muted_foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &empty,
            Point2D::new(panel.origin.x + PAD, Self::list_top(panel) + 28.0),
        );
    }

    fn paint_cell(&self, cx: &mut PaintCx<'_>, panel: Rect, idx: usize, item: &IconRow<'_>) {
        let cell = Self::cell_rect(panel, idx);
        cx.backend.fill_round_rect(cell, 6.0, self.theme.popover);
        let icon_pos = Point2D::new(
            cell.origin.x + (cell.size.x - GRID_ICON) / 2.0,
            cell.origin.y + (cell.size.y - GRID_ICON) / 2.0,
        );
        match item {
            IconRow::Local(icon) => draw_icon_catalog_entry(
                cx.backend,
                icon,
                icon_pos,
                GRID_ICON,
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
                GRID_ICON,
                self.theme.foreground,
                1.5,
            ),
        }
    }

    fn paint_load_more(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let button = Self::load_more_rect(panel);
        let divider_y = button.origin.y - LOAD_MORE_GAP / 2.0;
        cx.backend.stroke_line(
            Point2D::new(panel.origin.x + PAD, divider_y),
            Point2D::new(panel.origin.x + panel.size.x - PAD, divider_y),
            with_alpha(self.theme.border, 0.85),
            1.0,
        );
        let raw_query = self.state.editor_ui.icon_picker_search.trim();
        let (label, color, fill, border, icon) = if raw_query.is_empty() {
            (
                self.t("icon.typeToSearch"),
                self.theme.muted_foreground,
                with_alpha(self.theme.muted, 0.55),
                with_alpha(self.theme.border, 0.65),
                Icon::Search,
            )
        } else if self.state.editor_ui.icon_picker_remote.loading {
            (
                "...",
                self.theme.primary,
                self.theme.row_selected_primary,
                with_alpha(self.theme.primary, 0.35),
                Icon::Loader,
            )
        } else {
            (
                self.t("git.history.loadMore"),
                self.theme.primary,
                self.theme.row_selected_primary,
                with_alpha(self.theme.primary, 0.35),
                Icon::Plus,
            )
        };
        cx.backend.fill_round_rect(button, 8.0, fill);
        cx.backend.stroke_round_rect(button, 8.0, border, 1.0);
        let text_size = 12.0;
        let icon_size = 13.0;
        let gap = 7.0;
        let text_w = cx.backend.measure_text(label, text_size);
        let content_w = icon_size + gap + text_w;
        let content_x = button.origin.x + (button.size.x - content_w).max(0.0) / 2.0;
        draw_icon(
            cx.backend,
            icon,
            Point2D::new(
                content_x,
                button.origin.y + (button.size.y - icon_size) / 2.0,
            ),
            icon_size,
            color,
            1.5,
        );
        let text = TextLayout::single_run(
            label,
            "system-ui",
            text_size,
            to_jian(color),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &text,
            Point2D::new(content_x + icon_size + gap, button.origin.y + 20.0),
        );
    }

    fn paint_scrollbar(
        &self,
        cx: &mut PaintCx<'_>,
        panel: Rect,
        item_count: usize,
        footer_visible: bool,
        scroll: f32,
    ) {
        let list = Self::list_rect_for(panel, footer_visible);
        let content_h = Self::content_height(item_count);
        if content_h <= list.size.y + 0.5 {
            return;
        }
        let track_h = list.size.y - 8.0;
        let thumb_h = (track_h * list.size.y / content_h).max(24.0);
        let max_scroll = (content_h - list.size.y).max(0.0);
        let t = if max_scroll > 0.0 {
            (scroll / max_scroll).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let thumb = Rect {
            origin: Point2D::new(
                panel.origin.x + panel.size.x - 8.0,
                list.origin.y + 4.0 + t * (track_h - thumb_h),
            ),
            size: Point2D::new(3.0, thumb_h),
        };
        cx.backend
            .fill_round_rect(thumb, 1.5, self.theme.muted_foreground);
    }
}

fn remote_style(style: &str) -> IconRenderStyle {
    if style == "stroke" {
        IconRenderStyle::Stroke
    } else {
        IconRenderStyle::Fill
    }
}

fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.x <= r.origin.x + r.size.x
        && p.y >= r.origin.y
        && p.y <= r.origin.y + r.size.y
}

fn with_alpha(mut c: Color, alpha: f32) -> Color {
    c.a *= alpha.clamp(0.0, 1.0);
    c
}

fn to_jian(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_state(scroll: f32) -> EditorState {
        let mut state = EditorState::starter();
        state.editor_ui.icon_picker_open = true;
        state.editor_ui.icon_picker_scroll = scroll;
        state
    }

    fn selected_name(hit: IconPickerHit) -> String {
        match hit {
            IconPickerHit::SelectIcon { name, .. } => name,
            other => panic!("expected selectable icon row, got {other:?}"),
        }
    }

    #[test]
    fn hit_test_honors_icon_picker_scroll() {
        let panel_rect = Rect {
            origin: Point2D::new(40.0, 40.0),
            size: Point2D::new(ICON_PICKER_PANEL_W, ICON_PICKER_PANEL_H),
        };
        let first_cell = IconPickerPanel::cell_rect(panel_rect, 0);
        let point = Point2D::new(
            first_cell.origin.x + first_cell.size.x / 2.0,
            first_cell.origin.y + first_cell.size.y / 2.0,
        );

        let state = open_state(0.0);
        let panel = IconPickerPanel::for_editor(&state).expect("picker open");
        assert!(panel.max_scroll(panel_rect) > GRID_PITCH * 2.0);
        let first = selected_name(panel.hit_test(panel_rect, point).expect("first row"));

        let state = open_state(GRID_PITCH * 3.0);
        let panel = IconPickerPanel::for_editor(&state).expect("picker open");
        let scrolled = selected_name(panel.hit_test(panel_rect, point).expect("scrolled row"));

        assert_ne!(first, scrolled);
    }

    #[test]
    fn visible_index_range_is_limited_to_visible_grid_rows() {
        let panel_rect = Rect {
            origin: Point2D::new(40.0, 40.0),
            size: Point2D::new(ICON_PICKER_PANEL_W, ICON_PICKER_PANEL_H),
        };

        let range = IconPickerPanel::visible_index_range(panel_rect, 120, GRID_PITCH * 3.0);

        assert!(range.start > 0);
        assert!(range.end < 120);
        assert!(range.end - range.start <= GRID_COLS * 9);
    }

    #[test]
    fn load_more_footer_is_clickable_without_scrolling_to_grid_end() {
        let panel_rect = Rect {
            origin: Point2D::new(40.0, 40.0),
            size: Point2D::new(ICON_PICKER_PANEL_W, ICON_PICKER_PANEL_H),
        };
        let mut state = open_state(0.0);
        state.editor_ui.icon_picker_search = "home".to_string();
        let panel = IconPickerPanel::for_editor(&state).expect("picker open");
        let row = IconPickerPanel::load_more_rect(panel_rect);
        let point = Point2D::new(
            row.origin.x + row.size.x / 2.0,
            row.origin.y + row.size.y / 2.0,
        );

        assert_eq!(
            panel.hit_test(panel_rect, point),
            Some(IconPickerHit::LoadMore)
        );
    }

    #[test]
    fn empty_query_footer_is_visible_but_disabled() {
        let panel_rect = Rect {
            origin: Point2D::new(40.0, 40.0),
            size: Point2D::new(ICON_PICKER_PANEL_W, ICON_PICKER_PANEL_H),
        };
        let state = open_state(0.0);
        let panel = IconPickerPanel::for_editor(&state).expect("picker open");
        let row = IconPickerPanel::load_more_rect(panel_rect);
        let point = Point2D::new(
            row.origin.x + row.size.x / 2.0,
            row.origin.y + row.size.y / 2.0,
        );

        assert_eq!(
            panel.hit_test(panel_rect, point),
            Some(IconPickerHit::Inside)
        );
        assert!(
            IconPickerPanel::list_rect_for(panel_rect, true).size.y
                < IconPickerPanel::list_rect(panel_rect).size.y
        );
        let expected = IconPickerPanel::max_scroll_for(panel_rect, panel.all_rows().len(), true);
        assert!((panel.max_scroll(panel_rect) - expected).abs() < 0.01);
    }

    #[test]
    fn search_caret_is_visible_at_blink_anchor() {
        let mut state = open_state(0.0);
        state.editor_ui.icon_picker_caret_anchor_ms = 1200;
        let panel = IconPickerPanel::for_editor_at(&state, 1200).expect("picker open");

        assert!(panel.search_caret_visible());
    }
}
