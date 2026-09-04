//! File-menu dropdown anchored under TopBar's folder+chevron button.
//!
//! Mirrors `apps/web/src/components/editor/top-bar.tsx` file menu
//! verbatim: New / Open / Save / Save As / Export image, then a
//! "Recent files" header + entries, finally Clear history. Desktop hosts
//! can insert Save As Template after Save As. Two more rows can appear
//! under Export image, each gated on a host capability flag: a whole frame set at once
//! (`EditorUiState::batch_frame_export_supported`) and, on a deck
//! document, the self-contained slideshow page
//! (`EditorUiState::deck_html_export_supported`, which gates the whole
//! deck-export family — the slideshow page and the editable PowerPoint
//! deck have the same host requirements, a save picker plus the
//! offscreen rasteriser, so one flag answers for both). Everything after
//! the export section shifts with however many of them are present.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::export_menu_rows;
use crate::widgets::menu_paint;
use crate::widgets::WidgetId;
use crate::{Point2D, Rect};
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
        "saveAsTemplate" => "menu.saveAsTemplate",
        "exportImage" => "fileMenu.exportImage",
        "exportAllFrames" => "fileMenu.exportAllFrames",
        "exportSlideshowHtml" => "fileMenu.exportSlideshowHtml",
        "exportPptx" => "fileMenu.exportPptx",
        "recentFiles" => "fileMenu.recentFiles",
        "noRecentFiles" => "fileMenu.noRecentFiles",
        "clearHistory" => "fileMenu.clearHistory",
        _ => return "",
    };
    let translated = op_i18n::translate(ui.effective_locale(), full);
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

#[path = "file_menu_paint.rs"]
mod paint;

// The paint bodies moved to `file_menu_paint.rs` at the 800-line cap. The
// re-export keeps `file_menu::truncate_to_width` at the path its callers
// already use; the row-column geometry and the measured truncation core
// stayed here, next to the rest of the row map they belong to.
pub(crate) use paint::truncate_to_width;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMenuChoice {
    NewFile,
    /// Open the Scene Template Center to start from a finished document.
    NewFromTemplate,
    OpenFile,
    Save,
    SaveAs,
    /// Persist the current document into the desktop user's template library.
    SaveAsTemplate,
    ExportImage,
    /// Export every top-level frame of the active page (or just the
    /// selected frames) as one PNG each. Only offered by hosts that set
    /// `EditorUiState::batch_frame_export_supported`.
    ExportAllFrames,
    /// Export the deck as one self-contained slideshow `.html`. Offered
    /// only when the host sets `EditorUiState::deck_html_export_supported`
    /// AND the document is tagged as a deck.
    ExportSlideshowHtml,
    /// Export the deck as an editable PowerPoint `.pptx` — same gate as
    /// [`FileMenuChoice::ExportSlideshowHtml`], one row below it. The two
    /// answer different questions: the HTML page is for presenting as
    /// authored, the `.pptx` is for handing the deck to someone who will
    /// keep working on it.
    ExportPptx,
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

    /// Whether the batch frame-export row is offered. Both export
    /// surfaces read the same predicate — see `export_menu_rows`.
    fn has_export_all_row(&self) -> bool {
        export_menu_rows::batch_frame_export_available(self.ui)
    }

    fn has_save_as_template_row(&self) -> bool {
        self.ui.scene_template_center.save_current_supported
    }

    /// Whether the deck-export rows are offered — same predicate the
    /// TopBar quick menu gates its deck rows on.
    fn has_deck_export_rows(&self) -> bool {
        export_menu_rows::deck_export_available(self.ui)
    }

    /// Rows in the export section (Export image, plus the optional
    /// Export-all-frames row and the two deck-export rows below it).
    fn export_rows(&self) -> usize {
        1 + usize::from(self.has_export_all_row()) + 2 * usize::from(self.has_deck_export_rows())
    }

    /// Row index of the deck-slideshow row — directly under whichever
    /// export rows precede it. Only meaningful when
    /// [`FileMenu::has_deck_export_rows`] holds.
    fn deck_html_row(&self) -> usize {
        6 + usize::from(self.has_save_as_template_row()) + usize::from(self.has_export_all_row())
    }

    /// Row index of the PowerPoint row, directly under the slideshow one.
    fn deck_pptx_row(&self) -> usize {
        self.deck_html_row() + 1
    }

    /// Row index of the first recent-file entry. Everything after the
    /// export section shifts with [`FileMenu::export_rows`].
    fn recent_row_start(&self) -> usize {
        5 + usize::from(self.has_save_as_template_row()) + self.export_rows()
    }

    /// Label for the batch-export row: naming the selected frames when
    /// the selection would narrow the scope, else "all frames".
    fn export_all_label(&self) -> String {
        if self.selected_frames >= 2 {
            op_i18n::translate(self.ui.effective_locale(), "fileMenu.exportSelectedFrames")
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
        h += ROW_HEIGHT * (2 + usize::from(self.has_save_as_template_row())) as f32;
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
            5 if self.has_save_as_template_row() => Some(FileMenuChoice::SaveAsTemplate),
            row if row == 5 + usize::from(self.has_save_as_template_row()) => {
                Some(FileMenuChoice::ExportImage)
            }
            row if self.has_export_all_row()
                && row == 6 + usize::from(self.has_save_as_template_row()) =>
            {
                Some(FileMenuChoice::ExportAllFrames)
            }
            row if self.has_deck_export_rows() && row == self.deck_html_row() => {
                Some(FileMenuChoice::ExportSlideshowHtml)
            }
            row if self.has_deck_export_rows() && row == self.deck_pptx_row() => {
                Some(FileMenuChoice::ExportPptx)
            }
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
        for _ in 0..2 + usize::from(self.has_save_as_template_row()) {
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

// Shared menu-row geometry lives in `menu_paint`; this thin wrapper binds
// this menu's width/row constants. Its paint twin sits beside the rest of
// the paint code in `file_menu_paint.rs`.
fn row_hit(x: f32, y: f32, point: Point2D) -> bool {
    menu_paint::row_hit(x, y, point, MENU_WIDTH, ROW_HEIGHT)
}

fn recent_row_columns(x: f32, age_width: f32) -> (f32, f32, f32) {
    let name_x = x + PAD_X + ICON_SIZE + 10.0;
    let content_right = x + MENU_WIDTH - PAD_X;
    let name_right = content_right - age_width - RECENT_COLUMN_GAP;
    let name_budget = (name_right - name_x).max(0.0);
    let age_x = content_right - age_width;
    (name_x, name_budget, age_x)
}

/// `pub(crate)` so widgets that ellipsize into a fixed-width slot can
/// test their budget against an injected advance model — the backend
/// measurer is not reachable from a widget's pure tests.
pub(crate) fn truncate_to_width_measured(
    s: &str,
    max_w: f32,
    mut measure: impl FnMut(&str) -> f32,
) -> String {
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

fn file_name(path: &str) -> String {
    path.rsplit('/')
        .next()
        .map(|s| s.to_string())
        .unwrap_or_else(|| path.to_string())
}

fn format_age(ui: &EditorUiState, elapsed_secs: u64) -> String {
    let locale = ui.effective_locale();
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
