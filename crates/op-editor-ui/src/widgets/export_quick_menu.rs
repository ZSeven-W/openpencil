//! Export quick menu — the compact dropdown behind the TopBar download
//! button.
//!
//! Every row is a shortcut to an export the File menu already offers; the
//! difference is reach (one click instead of two) and order (a deck leads
//! with the formats a deck is delivered in). Row availability comes from
//! `export_menu_rows`, shared with the File menu, so a host capability can
//! never gate one surface and not the other.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::export_menu_rows;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::menu_paint;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::editor_ui_state::EditorUiState;
use op_editor_core::ExportQuickRow;

pub const MENU_WIDTH: f32 = 232.0;
const PAD_X: f32 = 12.0;
const PAD_Y: f32 = 6.0;
const ROW_HEIGHT: f32 = 30.0;
const ICON_SIZE: f32 = 15.0;
/// Gap between the anchoring button and the menu's top edge — matches the
/// File menu and the account dropdown.
const ANCHOR_GAP: f32 = 6.0;

/// Where a press inside the menu's bounds landed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportQuickMenuHit {
    Row(ExportQuickRow),
    /// Menu chrome (padding between rows / the rounded edge) — swallowed.
    Inside,
    Outside,
}

pub struct ExportQuickMenu<'a> {
    pub id: WidgetId,
    pub theme: Theme,
    ui: &'a EditorUiState,
    rows: Vec<ExportQuickRow>,
}

impl<'a> ExportQuickMenu<'a> {
    pub fn for_editor_ui(ui: &'a EditorUiState) -> Self {
        Self {
            id: WidgetId::new(5700),
            theme: theme_for(ui),
            ui,
            rows: export_menu_rows::quick_menu_rows(ui),
        }
    }

    pub fn rows(&self) -> &[ExportQuickRow] {
        &self.rows
    }

    pub fn height(&self) -> f32 {
        PAD_Y * 2.0 + ROW_HEIGHT * self.rows.len() as f32
    }

    /// Anchored under the download button and right-aligned to it — a
    /// dropdown hanging off a right-cluster button grows down-and-left.
    /// `viewport_w` clamps it back inside a narrow window.
    pub fn rect_at(&self, anchor: Rect, viewport_w: f32) -> Rect {
        let right = anchor.origin.x + anchor.size.x;
        let x = (right - MENU_WIDTH)
            .min((viewport_w - MENU_WIDTH - 8.0).max(8.0))
            .max(8.0);
        Rect {
            origin: Point2D::new(x, anchor.origin.y + anchor.size.y + ANCHOR_GAP),
            size: Point2D::new(MENU_WIDTH, self.height()),
        }
    }

    pub fn hit(&self, panel: Rect, point: Point2D) -> ExportQuickMenuHit {
        if !(panel).contains(point) {
            return ExportQuickMenuHit::Outside;
        }
        let mut y = panel.origin.y + PAD_Y;
        for row in &self.rows {
            if row_hit(panel.origin.x, y, point) {
                return ExportQuickMenuHit::Row(*row);
            }
            y += ROW_HEIGHT;
        }
        ExportQuickMenuHit::Inside
    }

    /// Convenience for hover dispatch — same row geometry as [`Self::hit`].
    pub fn hovered_at(&self, panel: Rect, point: Point2D) -> Option<ExportQuickRow> {
        match self.hit(panel, point) {
            ExportQuickMenuHit::Row(row) => Some(row),
            ExportQuickMenuHit::Inside | ExportQuickMenuHit::Outside => None,
        }
    }

    fn label(&self, row: ExportQuickRow) -> String {
        match row {
            ExportQuickRow::Pptx => {
                export_menu_rows::trimmed_label(self.ui, "fileMenu.exportPptx").to_string()
            }
            // No dedicated catalogue key: the format name is the row, and
            // `export.exportFormat` is already the localized frame for it.
            ExportQuickRow::Pdf => {
                op_i18n::translate(self.ui.effective_locale(), "export.exportFormat")
                    .replace("{{format}}", "PDF")
            }
            ExportQuickRow::SlideshowHtml => {
                export_menu_rows::trimmed_label(self.ui, "fileMenu.exportSlideshowHtml").to_string()
            }
            ExportQuickRow::Image => {
                export_menu_rows::trimmed_label(self.ui, "fileMenu.exportImage").to_string()
            }
            ExportQuickRow::AllFrames => {
                export_menu_rows::trimmed_label(self.ui, "fileMenu.exportAllFrames").to_string()
            }
        }
    }

    fn icon(row: ExportQuickRow) -> Icon {
        match row {
            // Same glyph the File menu gives each action, so the two
            // surfaces read as one feature.
            ExportQuickRow::Pptx => Icon::FileText,
            ExportQuickRow::Pdf => Icon::FileDown,
            ExportQuickRow::SlideshowHtml => Icon::Play,
            ExportQuickRow::Image => Icon::Image,
            ExportQuickRow::AllFrames => Icon::LayoutGrid,
        }
    }
}

fn row_hit(x: f32, y: f32, point: Point2D) -> bool {
    menu_paint::row_hit(x, y, point, MENU_WIDTH, ROW_HEIGHT)
}

impl<'a> Widget for ExportQuickMenu<'a> {
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
        let mut y = rect.origin.y + PAD_Y;
        for row in &self.rows {
            let hovered = self.ui.export_quick_menu_hover == Some(*row);
            if hovered {
                menu_paint::paint_row_tint(
                    cx,
                    &self.theme,
                    rect.origin.x,
                    y,
                    MENU_WIDTH,
                    ROW_HEIGHT,
                );
            }
            draw_icon(
                cx.backend,
                Self::icon(*row),
                Point2D::new(rect.origin.x + PAD_X, y + (ROW_HEIGHT - ICON_SIZE) / 2.0),
                ICON_SIZE,
                self.theme.muted_foreground,
                1.4,
            );
            let label = self.label(*row);
            let layout = TextLayout::single_run(
                &label,
                "system-ui",
                13.0,
                (self.theme.foreground).to_jian(),
                Point2D::new(0.0, 0.0),
            );
            cx.backend.draw_text(
                &layout,
                Point2D::new(
                    rect.origin.x + PAD_X + ICON_SIZE + 10.0,
                    y + ROW_HEIGHT / 2.0 + 5.0,
                ),
            );
            y += ROW_HEIGHT;
        }
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Menu);
        node.set_label("Export menu");
        node
    }
}

#[cfg(test)]
#[path = "export_quick_menu_tests.rs"]
mod tests;
