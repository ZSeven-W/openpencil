//! Paint for the File menu — the `Widget` impl and every row body.
//!
//! Split out of `file_menu.rs` at the 800-line cap. Pure code motion: the
//! row map, geometry and hit-test stayed in the spine, and everything
//! that puts pixels on the screen came here. It is a CHILD module of
//! `file_menu` rather than a widgets-level sibling, so it can still reach
//! the menu's private fields and row-index helpers without widening their
//! visibility.
//!
//! Paint order is the contract shared with the spine's `hit`: both walk
//! the same rows in the same sequence, so a row painted in one must be
//! hit-tested in the other. See `FileMenu::choice_for_row`.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::menu_paint;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};

use super::{
    t, FileMenu, RecentEntry, DIVIDER_GAP, FONT_FAMILY, HEADER_HEIGHT, ICON_SIZE, MENU_WIDTH,
    PAD_X, PAD_Y, ROW_HEIGHT, SHORTCUT_FONT,
};

// Shared menu-row paint bodies live in `menu_paint` (see its doc for the
// hover-tint colour rationale); this thin wrapper binds this menu's
// width/row constants.
fn paint_row_tint(cx: &mut PaintCx<'_>, theme: &Theme, x: f32, y: f32) {
    menu_paint::paint_row_tint(cx, theme, x, y, MENU_WIDTH, ROW_HEIGHT);
}

impl<'a> Widget for FileMenu<'a> {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(MENU_WIDTH, self.height()),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_round_rect(rect, 10.0, self.theme.card);
        cx.backend
            .stroke_round_rect(rect, 10.0, self.theme.border, 1.0);
        let h = |row: usize| self.menu.hover == Some(row);
        let mut y = rect.origin.y + PAD_Y;
        paint_row(
            cx,
            &self.theme,
            rect.origin.x,
            y,
            Icon::Plus,
            t(self.ui, "new"),
            "⌘N",
            h(0),
        );
        y += ROW_HEIGHT;
        paint_row(
            cx,
            &self.theme,
            rect.origin.x,
            y,
            Icon::LayoutDashboard,
            t(self.ui, "newFromTemplate"),
            "",
            h(1),
        );
        y += ROW_HEIGHT;
        paint_row(
            cx,
            &self.theme,
            rect.origin.x,
            y,
            Icon::FolderOpen,
            t(self.ui, "open"),
            "⌘O",
            h(2),
        );
        y += ROW_HEIGHT;
        y = paint_divider(cx, &self.theme, rect, y);
        paint_row(
            cx,
            &self.theme,
            rect.origin.x,
            y,
            Icon::Save,
            t(self.ui, "save"),
            "⌘S",
            h(3),
        );
        y += ROW_HEIGHT;
        paint_row(
            cx,
            &self.theme,
            rect.origin.x,
            y,
            Icon::Save,
            t(self.ui, "saveAs"),
            "⌘⇧S",
            h(4),
        );
        y += ROW_HEIGHT;
        if self.has_save_as_template_row() {
            paint_row(
                cx,
                &self.theme,
                rect.origin.x,
                y,
                Icon::Package,
                t(self.ui, "saveAsTemplate"),
                "",
                h(5),
            );
            y += ROW_HEIGHT;
        }
        y = paint_divider(cx, &self.theme, rect, y);
        let export_image_row = 5 + usize::from(self.has_save_as_template_row());
        paint_row(
            cx,
            &self.theme,
            rect.origin.x,
            y,
            Icon::Download,
            t(self.ui, "exportImage"),
            "⌘⇧P",
            h(export_image_row),
        );
        y += ROW_HEIGHT;
        if self.has_export_all_row() {
            let row = export_image_row + 1;
            paint_row(
                cx,
                &self.theme,
                rect.origin.x,
                y,
                Icon::LayoutGrid,
                &self.export_all_label(),
                "",
                h(row),
            );
            y += ROW_HEIGHT;
        }
        if self.has_deck_export_rows() {
            let row = self.deck_html_row();
            paint_row(
                cx,
                &self.theme,
                rect.origin.x,
                y,
                Icon::Play,
                t(self.ui, "exportSlideshowHtml"),
                "",
                h(row),
            );
            y += ROW_HEIGHT;
            paint_row(
                cx,
                &self.theme,
                rect.origin.x,
                y,
                Icon::FileText,
                t(self.ui, "exportPptx"),
                "",
                h(row + 1),
            );
            y += ROW_HEIGHT;
        }
        y = paint_divider(cx, &self.theme, rect, y);
        paint_header(cx, &self.theme, rect.origin.x, y, t(self.ui, "recentFiles"));
        y += HEADER_HEIGHT;
        if self.recent.is_empty() {
            paint_empty(
                cx,
                &self.theme,
                rect.origin.x,
                y,
                t(self.ui, "noRecentFiles"),
            );
            y += ROW_HEIGHT;
        } else {
            let recent_start = self.recent_row_start();
            for (i, entry) in self.recent.iter().enumerate() {
                paint_recent_row(
                    cx,
                    &self.theme,
                    rect.origin.x,
                    y,
                    entry,
                    h(recent_start + i),
                );
                y += ROW_HEIGHT;
            }
        }
        y = paint_divider(cx, &self.theme, rect, y);
        if self.recent.is_empty() {
            paint_row_disabled(
                cx,
                &self.theme,
                rect.origin.x,
                y,
                Icon::Trash,
                t(self.ui, "clearHistory"),
                "",
            );
        } else {
            paint_row(
                cx,
                &self.theme,
                rect.origin.x,
                y,
                Icon::Trash,
                t(self.ui, "clearHistory"),
                "",
                h(self.recent_row_start() + self.recent.len()),
            );
        }
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Menu);
        node.set_label(op_i18n::translate(
            self.ui.effective_locale(),
            "a11y.fileMenu",
        ));
        node
    }
}

// Paint-context + geometry args threaded through; a struct adds no gain.
#[allow(clippy::too_many_arguments)]
fn paint_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    x: f32,
    y: f32,
    icon: Icon,
    label: &str,
    shortcut: &str,
    hovered: bool,
) {
    if hovered {
        paint_row_tint(cx, theme, x, y);
    }
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(x + PAD_X, y + (ROW_HEIGHT - ICON_SIZE) / 2.0),
        ICON_SIZE,
        theme.muted_foreground,
        1.4,
    );
    let label_layout = TextLayout::single_run(
        label,
        FONT_FAMILY,
        13.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label_layout,
        Point2D::new(x + PAD_X + ICON_SIZE + 10.0, y + ROW_HEIGHT / 2.0 + 5.0),
    );
    if !shortcut.is_empty() {
        let sw = cx
            .backend
            .measure_text_family(shortcut, SHORTCUT_FONT, FONT_FAMILY);
        let sl = TextLayout::single_run(
            shortcut,
            FONT_FAMILY,
            SHORTCUT_FONT,
            (theme.muted_foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &sl,
            Point2D::new(x + MENU_WIDTH - PAD_X - sw, y + ROW_HEIGHT / 2.0 + 4.0),
        );
    }
}

/// Like `paint_row` but at ~50% opacity foreground colour so the
/// row reads as inactive. Used for menu items whose backend isn't
/// wired (Export image / Clear history). Pairs with `hit_test`
/// returning `None` for the same coordinates.
fn paint_row_disabled(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    x: f32,
    y: f32,
    icon: Icon,
    label: &str,
    shortcut: &str,
) {
    let dim = Color {
        r: theme.muted_foreground.r,
        g: theme.muted_foreground.g,
        b: theme.muted_foreground.b,
        a: theme.muted_foreground.a * 0.55,
    };
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(x + PAD_X, y + (ROW_HEIGHT - ICON_SIZE) / 2.0),
        ICON_SIZE,
        dim,
        1.4,
    );
    let label_layout = TextLayout::single_run(
        label,
        FONT_FAMILY,
        13.0,
        (dim).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label_layout,
        Point2D::new(x + PAD_X + ICON_SIZE + 10.0, y + ROW_HEIGHT / 2.0 + 5.0),
    );
    if !shortcut.is_empty() {
        let sw = cx
            .backend
            .measure_text_family(shortcut, SHORTCUT_FONT, FONT_FAMILY);
        let sl = TextLayout::single_run(
            shortcut,
            FONT_FAMILY,
            SHORTCUT_FONT,
            (dim).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &sl,
            Point2D::new(x + MENU_WIDTH - PAD_X - sw, y + ROW_HEIGHT / 2.0 + 4.0),
        );
    }
}

fn paint_recent_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    x: f32,
    y: f32,
    entry: &RecentEntry,
    hovered: bool,
) {
    if hovered {
        paint_row_tint(cx, theme, x, y);
    }
    draw_icon(
        cx.backend,
        Icon::FileText,
        Point2D::new(x + PAD_X, y + (ROW_HEIGHT - ICON_SIZE) / 2.0),
        ICON_SIZE,
        theme.muted_foreground,
        1.4,
    );
    // The age column keeps a stable right edge. The name is both measured
    // and clipped before that column, so even an unexpected platform-font
    // metric cannot paint over the age.
    let aw = cx
        .backend
        .measure_text_family(&entry.age, SHORTCUT_FONT, FONT_FAMILY);
    let (name_x, name_budget, age_x) = super::recent_row_columns(x, aw);
    let display_name = truncate_to_width(cx, &entry.name, 13.0, name_budget);
    let name_layout = TextLayout::single_run(
        &display_name,
        FONT_FAMILY,
        13.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    if name_budget > 0.0 {
        cx.backend.save();
        cx.backend.clip_rect(Rect {
            origin: Point2D::new(name_x, y),
            size: Point2D::new(name_budget, ROW_HEIGHT),
        });
        cx.backend.draw_text(
            &name_layout,
            Point2D::new(name_x, y + ROW_HEIGHT / 2.0 + 5.0),
        );
        cx.backend.restore();
    }
    let age_layout = TextLayout::single_run(
        &entry.age,
        FONT_FAMILY,
        SHORTCUT_FONT,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&age_layout, Point2D::new(age_x, y + ROW_HEIGHT / 2.0 + 4.0));
}

/// Truncate `s` so it fits `max_w` pixels at `font_size`, appending
/// `…` when characters are dropped. Uses the backend's measurer so
/// CJK glyph widths are honoured (a 13-px ASCII "pencil-demo.op" and
/// the equivalent CJK string have very different advances).
///
/// `pub(crate)` (not private) — `screen_switcher_pills.rs` reuses it to
/// ellipsize a screen name into a fixed-width pill.
pub(crate) fn truncate_to_width(
    cx: &mut PaintCx<'_>,
    s: &str,
    font_size: f32,
    max_w: f32,
) -> String {
    super::truncate_to_width_measured(s, max_w, |text| {
        cx.backend.measure_text_family(text, font_size, FONT_FAMILY)
    })
}

fn paint_empty(cx: &mut PaintCx<'_>, theme: &Theme, x: f32, y: f32, label: &str) {
    let lw = cx.backend.measure_text_family(label, 12.0, FONT_FAMILY);
    let lay = TextLayout::single_run(
        label,
        FONT_FAMILY,
        12.0,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &lay,
        Point2D::new(x + (MENU_WIDTH - lw) / 2.0, y + ROW_HEIGHT / 2.0 + 4.0),
    );
}

fn paint_header(cx: &mut PaintCx<'_>, theme: &Theme, x: f32, y: f32, label: &str) {
    let layout = TextLayout::single_run(
        label,
        FONT_FAMILY,
        11.0,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &layout,
        Point2D::new(x + PAD_X, y + HEADER_HEIGHT / 2.0 + 4.0),
    );
}

fn paint_divider(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, y: f32) -> f32 {
    menu_paint::paint_divider(cx, theme, rect, y, MENU_WIDTH, PAD_X, DIVIDER_GAP)
}
