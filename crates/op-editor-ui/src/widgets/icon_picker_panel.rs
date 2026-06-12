//! Native Iconify picker used by the toolbar and icon property row.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::icon_catalog::{search_icons, IconCatalogEntry, IconRenderStyle};
use crate::widgets::{
    draw_icon, draw_icon_catalog_entry, draw_icon_data, Icon, IconPathData, PaintCx,
};
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::{EditorState, IconPickerRemoteIcon, Locale};

pub const ICON_PICKER_PANEL_W: f32 = 320.0;
pub const ICON_PICKER_PANEL_H: f32 = 420.0;
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
    hover: Option<op_editor_core::IconPickerButton>,
}

impl<'a> IconPickerPanel<'a> {
    pub fn for_editor(state: &'a EditorState) -> Option<IconPickerPanel<'a>> {
        if !state.editor_ui.icon_picker_open {
            return None;
        }
        Some(IconPickerPanel {
            state,
            theme: theme_for(&state.editor_ui),
            locale: state.editor_ui.locale,
            hover: state.editor_ui.icon_picker_hover,
        })
    }

    /// Resolve a pointer to a hoverable target (close / row / load-more).
    /// Mirrors [`Self::hit_test`]'s row math; rows are index-addressable
    /// so no string ids leak into the hover state.
    pub fn hover_at(
        &self,
        panel: Rect,
        point: Point2D,
    ) -> Option<op_editor_core::IconPickerButton> {
        use op_editor_core::IconPickerButton as B;
        if !(panel).contains(point) {
            return None;
        }
        if (Self::close_rect(panel)).contains(point) {
            return Some(B::Close);
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
                return Some(B::Row(row));
            }
            if has_more && row == items.len() && !self.state.editor_ui.icon_picker_remote.loading {
                return Some(B::LoadMore);
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
        let close_hovered = self.hover == Some(op_editor_core::IconPickerButton::Close);
        if close_hovered {
            cx.backend
                .fill_round_rect(close, 6.0, self.theme.button_hover);
        }
        draw_icon(
            cx.backend,
            Icon::Close,
            Point2D::new(close.origin.x + 5.0, close.origin.y + 5.0),
            14.0,
            if close_hovered {
                self.theme.foreground
            } else {
                self.theme.muted_foreground
            },
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
        // Ctrl/Cmd+A highlights the whole query (painted behind the text).
        if self.state.editor_ui.icon_picker_select_all && !raw_query.is_empty() {
            crate::widgets::text_selection::paint_single_line_selection(
                cx,
                &self.theme,
                raw_query,
                search.origin.x + 30.0,
                search.origin.y + 18.0,
                12.0,
                search.origin.x + search.size.x - 8.0,
            );
        }
        let layout = TextLayout::single_run(
            search_text,
            "system-ui",
            12.0,
            (color).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &layout,
            Point2D::new(search.origin.x + 30.0, search.origin.y + 18.0),
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
        if self.hover == Some(op_editor_core::IconPickerButton::Row(idx)) {
            cx.backend
                .fill_round_rect(row, 6.0, self.theme.button_hover);
        }
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
        if self.hover == Some(op_editor_core::IconPickerButton::LoadMore) {
            cx.backend
                .fill_round_rect(row, 6.0, self.theme.button_hover);
        }
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
