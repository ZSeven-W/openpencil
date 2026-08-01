//! File-menu dropdown anchored under TopBar's folder+chevron button.
//!
//! Mirrors `apps/web/src/components/editor/top-bar.tsx` file menu
//! verbatim: New / Open / Save / Save As / Export image, then a
//! "Recent files" header + entries, finally Clear history. Hosts that
//! can write a whole frame set at once get one extra row under Export
//! image (see `EditorUiState::batch_frame_export_supported`).

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::menu_paint;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};
pub use jian_widgets::components::menu::MenuHit;
use op_editor_core::editor_ui_state::EditorUiState;

/// Resolve a file-menu row label via `op-i18n`. The Rust file menu
/// deliberately omits the trailing ellipsis some `fileMenu.*` values
/// carry (a Mac-convention "..."), so the result is trimmed.
fn t(ui: &EditorUiState, key: &str) -> &'static str {
    let full = match key {
        "new" => "fileMenu.newFile",
        "newFromTemplate" => "fileMenu.newFromTemplate",
        "open" => "fileMenu.openFile",
        "save" => "fileMenu.save",
        "saveAs" => "fileMenu.saveAs",
        "exportImage" => "fileMenu.exportImage",
        "exportAllFrames" => "fileMenu.exportAllFrames",
        "recentFiles" => "fileMenu.recentFiles",
        "noRecentFiles" => "fileMenu.noRecentFiles",
        "clearHistory" => "fileMenu.clearHistory",
        _ => return "",
    };
    let translated = op_i18n::translate(ui.locale, full);
    if translated == full {
        // A key that is not in the catalogue yet must not surface as
        // "fileMenu.newFromTemplate" in the menu.
        return match key {
            "newFromTemplate" => "从模板新建",
            _ => "",
        };
    }
    translated.trim_end_matches(['.', '…'])
}

pub const MENU_WIDTH: f32 = 300.0;
const PAD_X: f32 = 12.0;
const PAD_Y: f32 = 6.0;
const ROW_HEIGHT: f32 = 30.0;
const HEADER_HEIGHT: f32 = 22.0;
const DIVIDER_GAP: f32 = 4.0;
const ICON_SIZE: f32 = 16.0;
const SHORTCUT_FONT: f32 = 11.0;
const FONT_FAMILY: &str = "system-ui";
const RECENT_COLUMN_GAP: f32 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMenuChoice {
    NewFile,
    /// Open the Scene Template Center to start from a finished document.
    NewFromTemplate,
    OpenFile,
    Save,
    SaveAs,
    ExportImage,
    /// Export every top-level frame of the active page (or just the
    /// selected frames) as one PNG each. Only offered by hosts that set
    /// `EditorUiState::batch_frame_export_supported`.
    ExportAllFrames,
    OpenRecent(usize),
    ClearRecent,
}

pub struct FileMenu<'a> {
    pub id: WidgetId,
    pub theme: Theme,
    ui: &'a EditorUiState,
    pub recent: Vec<RecentEntry>,
    /// Top-level frames in the current selection. Cosmetic only — it
    /// picks between the row's "all frames" and "N frames" labels; the
    /// exporter re-derives the scope itself, and row geometry is the
    /// same either way, so hit-test call sites can leave it at 0.
    selected_frames: usize,
    /// Shared interaction state populated by the host on cursor-move.
    /// `hover` stores an actionable row index.
    pub menu: jian_widgets::components::menu::MenuState,
}

#[derive(Debug, Clone)]
pub struct RecentEntry {
    pub name: String,
    pub age: String,
}

impl<'a> FileMenu<'a> {
    pub fn for_editor_ui(ui: &'a EditorUiState, recent: Vec<RecentEntry>) -> Self {
        Self {
            id: WidgetId::new(5300),
            theme: theme_for(ui),
            ui,
            recent,
            selected_frames: 0,
            menu: ui.file_menu.clone(),
        }
    }

    /// Tell the menu how many top-level frames are selected so the
    /// batch-export row can name them. Paint-time only — see
    /// [`FileMenu::selected_frames`].
    pub fn with_selected_frames(mut self, count: usize) -> Self {
        self.selected_frames = count;
        self
    }

    /// Convenience: build a `FileMenu` whose recents are derived
    /// from `ui.recent_files` formatted at `now_secs`. Paint + host
    /// dispatch both reach this entry point.
    pub fn from_editor_ui(ui: &'a EditorUiState, now_secs: u64) -> Self {
        let recent = ui
            .recent_files
            .iter()
            .map(|r| RecentEntry {
                name: file_name(&r.path),
                age: format_age(ui, now_secs.saturating_sub(r.modified_at)),
            })
            .collect();
        Self::for_editor_ui(ui, recent)
    }

    /// Whether the batch frame-export row is offered — hosts without a
    /// directory picker + offscreen exporter leave it out entirely
    /// rather than paint a dead row.
    fn has_export_all_row(&self) -> bool {
        self.ui.batch_frame_export_supported
    }

    /// Rows in the export section (Export image, plus the optional
    /// Export-all-frames row below it).
    fn export_rows(&self) -> usize {
        1 + usize::from(self.has_export_all_row())
    }

    /// Row index of the first recent-file entry. Everything after the
    /// export section shifts with [`FileMenu::export_rows`].
    fn recent_row_start(&self) -> usize {
        5 + self.export_rows()
    }

    /// Label for the batch-export row: naming the selected frames when
    /// the selection would narrow the scope, else "all frames".
    fn export_all_label(&self) -> String {
        if self.selected_frames >= 2 {
            op_i18n::translate(self.ui.locale, "fileMenu.exportSelectedFrames")
                .replace("{{count}}", &self.selected_frames.to_string())
                .trim_end_matches(['.', '…'])
                .to_string()
        } else {
            t(self.ui, "exportAllFrames").to_string()
        }
    }

    /// Total height = action rows + recent header + recent rows (or
    /// empty hint) + clear row + section paddings.
    pub fn height(&self) -> f32 {
        let mut h = PAD_Y;
        h += ROW_HEIGHT * 3.0; // New + New from template + Open
        h += DIVIDER_GAP * 2.0 + 1.0; // divider
        h += ROW_HEIGHT * 2.0; // Save + Save As
        h += DIVIDER_GAP * 2.0 + 1.0;
        h += ROW_HEIGHT * self.export_rows() as f32; // Export image (+ all frames)
        h += DIVIDER_GAP * 2.0 + 1.0;
        h += HEADER_HEIGHT; // Recent files header
        let recent_len = self.recent.len().max(1);
        h += ROW_HEIGHT * recent_len as f32;
        h += DIVIDER_GAP * 2.0 + 1.0;
        h += ROW_HEIGHT; // Clear history
        h += PAD_Y;
        h
    }

    pub fn rect_at(&self, anchor: Rect) -> Rect {
        let x = anchor.origin.x;
        let y = anchor.origin.y + anchor.size.y + 6.0;
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(MENU_WIDTH, self.height()),
        }
    }

    /// Convenience alias: `hit_test` is reused for hover dispatch
    /// (same row geometry, no separate code path needed).
    pub fn hovered_at(&self, panel: Rect, point: Point2D) -> Option<usize> {
        match self.hit(panel, point) {
            MenuHit::Row(idx) => Some(idx),
            MenuHit::Inside | MenuHit::Outside => None,
        }
    }

    pub fn choice_for_row(&self, row: usize) -> Option<FileMenuChoice> {
        let recent_start = self.recent_row_start();
        match row {
            0 => Some(FileMenuChoice::NewFile),
            1 => Some(FileMenuChoice::NewFromTemplate),
            2 => Some(FileMenuChoice::OpenFile),
            3 => Some(FileMenuChoice::Save),
            4 => Some(FileMenuChoice::SaveAs),
            5 => Some(FileMenuChoice::ExportImage),
            6 if self.has_export_all_row() => Some(FileMenuChoice::ExportAllFrames),
            row if row >= recent_start && row < recent_start + self.recent.len() => {
                Some(FileMenuChoice::OpenRecent(row - recent_start))
            }
            row if !self.recent.is_empty() && row == recent_start + self.recent.len() => {
                Some(FileMenuChoice::ClearRecent)
            }
            _ => None,
        }
    }

    pub fn hit(&self, panel: Rect, point: Point2D) -> MenuHit {
        if !(panel).contains(point) {
            return MenuHit::Outside;
        }
        let mut row = 0usize;
        let mut y = panel.origin.y + PAD_Y;
        for _ in 0..3 {
            if row_hit(panel.origin.x, y, point) {
                return MenuHit::Row(row);
            }
            y += ROW_HEIGHT;
            row += 1;
        }
        y += DIVIDER_GAP * 2.0 + 1.0;
        for _ in 0..2 {
            if row_hit(panel.origin.x, y, point) {
                return MenuHit::Row(row);
            }
            y += ROW_HEIGHT;
            row += 1;
        }
        y += DIVIDER_GAP * 2.0 + 1.0;
        for _ in 0..self.export_rows() {
            if row_hit(panel.origin.x, y, point) {
                return MenuHit::Row(row);
            }
            y += ROW_HEIGHT;
            row += 1;
        }
        y += DIVIDER_GAP * 2.0 + 1.0;
        y += HEADER_HEIGHT;
        for _ in self.recent.iter() {
            if row_hit(panel.origin.x, y, point) {
                return MenuHit::Row(row);
            }
            y += ROW_HEIGHT;
            row += 1;
        }
        if self.recent.is_empty() {
            y += ROW_HEIGHT;
        }
        y += DIVIDER_GAP * 2.0 + 1.0;
        if !self.recent.is_empty() && row_hit(panel.origin.x, y, point) {
            return MenuHit::Row(row);
        }
        MenuHit::Inside
    }

    /// `point` is in screen space; return the activated row, or None
    /// for clicks on dividers / headers / outside the menu.
    pub fn hit_test(&self, panel: Rect, point: Point2D) -> Option<FileMenuChoice> {
        match self.hit(panel, point) {
            MenuHit::Row(idx) => self.choice_for_row(idx),
            MenuHit::Inside | MenuHit::Outside => None,
        }
    }
}

// Shared menu-row geometry/paint bodies live in `menu_paint` (see its
// doc for the hover-tint colour rationale); these thin wrappers bind
// this menu's width/row/pad constants.
fn paint_row_tint(cx: &mut PaintCx<'_>, theme: &Theme, x: f32, y: f32) {
    menu_paint::paint_row_tint(cx, theme, x, y, MENU_WIDTH, ROW_HEIGHT);
}

fn row_hit(x: f32, y: f32, point: Point2D) -> bool {
    menu_paint::row_hit(x, y, point, MENU_WIDTH, ROW_HEIGHT)
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
        y = paint_divider(cx, &self.theme, rect, y);
        paint_row(
            cx,
            &self.theme,
            rect.origin.x,
            y,
            Icon::Download,
            t(self.ui, "exportImage"),
            "⌘⇧P",
            h(5),
        );
        y += ROW_HEIGHT;
        if self.has_export_all_row() {
            paint_row(
                cx,
                &self.theme,
                rect.origin.x,
                y,
                Icon::LayoutGrid,
                &self.export_all_label(),
                "",
                h(6),
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
        node.set_label(op_i18n::translate(self.ui.locale, "a11y.fileMenu"));
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
    let (name_x, name_budget, age_x) = recent_row_columns(x, aw);
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

fn recent_row_columns(x: f32, age_width: f32) -> (f32, f32, f32) {
    let name_x = x + PAD_X + ICON_SIZE + 10.0;
    let content_right = x + MENU_WIDTH - PAD_X;
    let name_right = content_right - age_width - RECENT_COLUMN_GAP;
    let name_budget = (name_right - name_x).max(0.0);
    let age_x = content_right - age_width;
    (name_x, name_budget, age_x)
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
    truncate_to_width_measured(s, max_w, |text| {
        cx.backend.measure_text_family(text, font_size, FONT_FAMILY)
    })
}

fn truncate_to_width_measured(s: &str, max_w: f32, mut measure: impl FnMut(&str) -> f32) -> String {
    if max_w <= 0.0 {
        return String::new();
    }
    if measure(s) <= max_w {
        return s.to_string();
    }
    let ellipsis_w = measure("…");
    if ellipsis_w > max_w {
        return String::new();
    }
    let budget = (max_w - ellipsis_w).max(0.0);
    let mut kept = String::new();
    for ch in s.chars() {
        let mut probe = kept.clone();
        probe.push(ch);
        let w = measure(&probe);
        if w > budget {
            break;
        }
        kept = probe;
    }
    if kept.is_empty() {
        "…".to_string()
    } else {
        format!("{}…", kept)
    }
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

fn file_name(path: &str) -> String {
    path.rsplit('/')
        .next()
        .map(|s| s.to_string())
        .unwrap_or_else(|| path.to_string())
}

fn format_age(ui: &EditorUiState, elapsed_secs: u64) -> String {
    let locale = ui.locale;
    if elapsed_secs < 60 {
        op_i18n::translate(locale, "fileMenu.justNow").to_string()
    } else if elapsed_secs < 3600 {
        op_i18n::translate(locale, "fileMenu.minutesAgo")
            .replace("{{count}}", &(elapsed_secs / 60).to_string())
    } else if elapsed_secs < 86400 {
        op_i18n::translate(locale, "fileMenu.hoursAgo")
            .replace("{{count}}", &(elapsed_secs / 3600).to_string())
    } else {
        op_i18n::translate(locale, "fileMenu.daysAgo")
            .replace("{{count}}", &(elapsed_secs / 86400).to_string())
    }
}

#[cfg(test)]
#[path = "file_menu_tests.rs"]
mod tests;
