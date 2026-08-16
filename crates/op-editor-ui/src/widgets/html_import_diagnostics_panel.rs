//! Post-import HTML diagnostics overlay shared by the native and web hosts.
//!
//! A non-modal card in the bottom-right corner: it reports how many
//! degradations the last HTML import recorded, expands into the per-warning
//! rows, and dismisses. Unlike the missing-font modal it paints no scrim and
//! swallows no press outside its own bounds — the canvas stays usable while
//! the report is up.
//!
//! Row copy is looked up per warning code (`htmlImport.warn.<code>`) and
//! interpolated with the importer's structured fields; a code with no locale
//! entry yet falls back to the importer's own English sentence, so a new
//! `ImportWarning` variant is never rendered as a bare key.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::{theme_for, translate};
use crate::widgets::text_metrics;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Point2D, Rect, TextLayout};
use jian_widgets::centered_text_baseline_y;
use op_editor_core::html_import_diagnostics::{
    HtmlImportDiagnostic, HtmlImportDiagnosticsHover, HtmlImportDiagnosticsSummary,
};
use op_editor_core::{EditorState, EditorUiState};

const PANEL_WIDTH: f32 = 420.0;
const HEADER_HEIGHT: f32 = 88.0;
const ROWS_MAX_HEIGHT: f32 = 230.0;
/// Uniform row height: two wrapped message lines plus the code line.
///
/// A row's sentence is a full localized clause and does not fit the ~388 px
/// of text column at 11 px, so it wraps (see [`wrap_row_lines`]). Keeping the
/// height uniform — rather than per-row measured — is what keeps
/// [`HtmlImportDiagnosticsPanel::max_rows_scroll`] and the paint loop's
/// `index * ROW_HEIGHT` arithmetic a single multiplication.
const ROW_HEIGHT: f32 = 46.0;
/// Message lines per row; the last one ellipsizes when the text overflows.
const MAX_ROW_LINES: usize = 2;
const ROW_FONT_SIZE: f32 = 11.0;
const ROW_CODE_FONT_SIZE: f32 = 10.0;
const HORIZONTAL_PAD: f32 = 16.0;
const BUTTON_HEIGHT: f32 = 26.0;
const VIEWPORT_MARGIN: f32 = 16.0;
/// Clearance for the host's bottom status strip.
const BOTTOM_INSET: f32 = 44.0;

/// What a press landed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HtmlImportDiagnosticsHit {
    /// The show-details / hide-details toggle.
    Toggle,
    /// The dismiss button.
    Dismiss,
    /// Inside the card but on no control — consumed so the press does not
    /// reach the canvas underneath.
    Inside,
    /// Outside the card — NOT consumed; the overlay is non-modal.
    Outside,
}

pub struct HtmlImportDiagnosticsPanel<'a> {
    id: WidgetId,
    theme: Theme,
    ui: &'a EditorUiState,
}

impl<'a> HtmlImportDiagnosticsPanel<'a> {
    /// `None` whenever the overlay must not paint: dismissed, or the last
    /// import degraded nothing.
    pub fn for_editor(state: &'a EditorState) -> Option<Self> {
        let ui = &state.editor_ui;
        if !ui.html_import_diagnostics_open || ui.html_import_diagnostics.is_empty() {
            return None;
        }
        Some(Self {
            id: WidgetId::new(5470),
            theme: theme_for(ui),
            ui,
        })
    }

    fn rows(&self) -> &'a [HtmlImportDiagnostic] {
        &self.ui.html_import_diagnostics
    }

    fn expanded(&self) -> bool {
        self.ui.html_import_diagnostics_expanded
    }

    /// Degradations the `MAX_DIAGNOSTIC_ROWS` cap dropped, i.e. the `N` of the
    /// trailing `+N more` row. Routed through
    /// [`HtmlImportDiagnosticsSummary::hidden`] so the cap arithmetic has one
    /// definition shared with `cap_rows`.
    pub fn hidden_rows(&self) -> usize {
        HtmlImportDiagnosticsSummary {
            shown: self.rows().len(),
            total: self.ui.html_import_diagnostics_total,
        }
        .hidden()
    }

    /// Painted rows: one per retained diagnostic, plus the `+N more` footer
    /// when the cap dropped some. Without the footer a capped report claims a
    /// total in its header that the expanded list silently contradicts.
    fn row_count(&self) -> usize {
        self.rows().len() + usize::from(self.hidden_rows() > 0)
    }

    /// Height of the scrollable rows viewport (zero while collapsed).
    fn rows_viewport_height(&self) -> f32 {
        if !self.expanded() {
            return 0.0;
        }
        let content = self.row_count() as f32 * ROW_HEIGHT;
        content.min(ROWS_MAX_HEIGHT)
    }

    fn height(&self) -> f32 {
        HEADER_HEIGHT + self.rows_viewport_height()
    }

    /// Bottom-right placement, clamped so a short viewport still shows the
    /// header instead of pushing the card off-screen.
    pub fn rect(&self, viewport_w: f32, viewport_h: f32) -> Rect {
        let height = self.height();
        let x = (viewport_w - PANEL_WIDTH - VIEWPORT_MARGIN).max(VIEWPORT_MARGIN);
        let y = (viewport_h - BOTTOM_INSET - height)
            .max(crate::widgets::TOP_BAR_HEIGHT + VIEWPORT_MARGIN);
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(PANEL_WIDTH, height),
        }
    }

    /// The clipped viewport the rows scroll inside.
    pub fn rows_rect(&self, panel: Rect) -> Rect {
        Rect {
            origin: Point2D::new(panel.origin.x, panel.origin.y + HEADER_HEIGHT),
            size: Point2D::new(panel.size.x, self.rows_viewport_height()),
        }
    }

    /// Largest scroll offset that still keeps content in view.
    pub fn max_rows_scroll(&self, _panel: Rect) -> f32 {
        let content = self.row_count() as f32 * ROW_HEIGHT;
        (content - self.rows_viewport_height()).max(0.0)
    }

    fn rows_scroll(&self, panel: Rect) -> f32 {
        self.ui
            .html_import_diagnostics_scroll
            .offset
            .clamp(0.0, self.max_rows_scroll(panel))
    }

    pub fn toggle_rect(&self, panel: Rect) -> Rect {
        let label = translate(self.ui, self.toggle_key());
        let width = crate::widgets::missing_fonts_panel::fit_button_width(label, 12.0);
        Rect {
            origin: Point2D::new(
                panel.origin.x + HORIZONTAL_PAD,
                panel.origin.y + HEADER_HEIGHT - 38.0,
            ),
            size: Point2D::new(width, BUTTON_HEIGHT),
        }
    }

    pub fn dismiss_rect(&self, panel: Rect) -> Rect {
        let label = translate(self.ui, "htmlImport.diagnostics.dismiss");
        let width = crate::widgets::missing_fonts_panel::fit_button_width(label, 12.0);
        Rect {
            origin: Point2D::new(
                panel.origin.x + panel.size.x - HORIZONTAL_PAD - width,
                panel.origin.y + HEADER_HEIGHT - 38.0,
            ),
            size: Point2D::new(width, BUTTON_HEIGHT),
        }
    }

    fn toggle_key(&self) -> &'static str {
        if self.expanded() {
            "htmlImport.diagnostics.collapse"
        } else {
            "htmlImport.diagnostics.expand"
        }
    }

    pub fn hit_test(&self, panel: Rect, point: Point2D) -> HtmlImportDiagnosticsHit {
        if self.dismiss_rect(panel).contains(point) {
            return HtmlImportDiagnosticsHit::Dismiss;
        }
        if self.toggle_rect(panel).contains(point) {
            return HtmlImportDiagnosticsHit::Toggle;
        }
        if panel.contains(point) {
            return HtmlImportDiagnosticsHit::Inside;
        }
        HtmlImportDiagnosticsHit::Outside
    }

    /// Which control (if any) a cursor at `point` is over.
    pub fn hover_at(&self, panel: Rect, point: Point2D) -> Option<HtmlImportDiagnosticsHover> {
        match self.hit_test(panel, point) {
            HtmlImportDiagnosticsHit::Toggle => Some(HtmlImportDiagnosticsHover::Toggle),
            HtmlImportDiagnosticsHit::Dismiss => Some(HtmlImportDiagnosticsHover::Dismiss),
            _ => None,
        }
    }

    /// Localized one-line summary, e.g. `3 items were degraded`.
    pub fn summary_text(&self) -> String {
        let total = self
            .ui
            .html_import_diagnostics_total
            .max(self.rows().len())
            .to_string();
        op_i18n::translate_with(
            self.ui.effective_locale(),
            "htmlImport.diagnostics.summary",
            &[("count", total.as_str())],
        )
    }
}

/// Localized text for one warning row, falling back to the importer's own
/// English sentence when the code has no locale entry.
pub fn row_text(ui: &EditorUiState, diagnostic: &HtmlImportDiagnostic) -> String {
    match op_i18n::translate_dynamic(ui.effective_locale(), &diagnostic.i18n_key) {
        Some(template) => op_i18n::interpolate(template, &diagnostic.arg_pairs()),
        None => diagnostic.message.clone(),
    }
}

/// Break `text` into at most [`MAX_ROW_LINES`] lines no wider than
/// `max_width`, ellipsizing the last one when the text still overflows.
///
/// Row copy is a full localized sentence and the text column is ~388 px, so
/// without this the tail of almost every row was hard-clipped by the panel's
/// `clip_rect` — the user saw a truncated clause with no ellipsis to say so.
/// Breaking prefers whitespace; a run with no break opportunity (a long URL,
/// or CJK copy, which carries no spaces at all) falls back to per-character
/// breaking so it still fills the line instead of overflowing it.
fn wrap_row_lines(cx: &mut PaintCx<'_>, text: &str, font_size: f32, max_width: f32) -> Vec<String> {
    if max_width <= 0.0 || text.is_empty() {
        return vec![text.to_string()];
    }
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut last_break: Option<usize> = None;
    for ch in text.chars() {
        let mut candidate = current.clone();
        candidate.push(ch);
        if text_metrics::measure_chrome(cx.backend, &candidate, font_size) <= max_width {
            if ch.is_whitespace() {
                last_break = Some(candidate.len());
            }
            current = candidate;
            continue;
        }
        if lines.len() + 1 >= MAX_ROW_LINES {
            // No room for another line — ellipsize what is left.
            return finish_with_ellipsis(cx, lines, current, font_size, max_width);
        }
        match last_break.filter(|at| *at > 0 && *at < current.len()) {
            Some(at) => {
                let rest = current[at..].to_string();
                current.truncate(at);
                lines.push(current.trim_end().to_string());
                current = rest;
                current.push(ch);
            }
            None => {
                lines.push(current.clone());
                current = ch.to_string();
            }
        }
        last_break = None;
    }
    lines.push(current);
    lines
}

/// Tail of [`wrap_row_lines`]: the remaining text does not fit the last
/// permitted line, so trim it back one character at a time until the line plus
/// an ellipsis fits.
fn finish_with_ellipsis(
    cx: &mut PaintCx<'_>,
    mut lines: Vec<String>,
    mut current: String,
    font_size: f32,
    max_width: f32,
) -> Vec<String> {
    while !current.is_empty() {
        let candidate = format!("{}…", current.trim_end());
        if text_metrics::measure_chrome(cx.backend, &candidate, font_size) <= max_width {
            lines.push(candidate);
            return lines;
        }
        current.pop();
    }
    lines.push("…".to_string());
    lines
}

fn paint_text(
    cx: &mut PaintCx<'_>,
    text: &str,
    origin: Point2D,
    font_size: f32,
    weight: u16,
    color: crate::Color,
) {
    let layout = TextLayout::single_run(
        text,
        "system-ui",
        font_size,
        color.to_jian(),
        Point2D::new(0.0, 0.0),
    )
    .with_font_weight(weight);
    cx.backend.draw_text(&layout, origin);
}

fn paint_button(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, label: &str, hovered: bool) {
    if hovered {
        cx.backend.fill_round_rect(rect, 6.0, theme.border);
    }
    paint_text(
        cx,
        label,
        Point2D::new(rect.origin.x + 12.0, centered_text_baseline_y(rect, 12.0)),
        12.0,
        500,
        if hovered {
            theme.foreground
        } else {
            theme.muted_foreground
        },
    );
}

impl Widget for HtmlImportDiagnosticsPanel<'_> {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(PANEL_WIDTH, self.height()),
            },
        }
    }

    fn access_node(&self) -> accesskit::Node {
        // `Alert`, not `AlertDialog`: in accesskit (following ARIA) an
        // `AlertDialog` is a modal that traps focus and must be acknowledged,
        // which would make a screen reader promise interaction semantics this
        // overlay does not have. It is a non-modal status report — the canvas
        // stays live behind it and an outside press falls straight through —
        // so `Alert` is the role that matches.
        let mut node = accesskit::Node::new(accesskit::Role::Alert);
        node.set_label(format!(
            "{} — {}",
            translate(self.ui, "htmlImport.diagnostics.title"),
            self.summary_text()
        ));
        node
    }

    fn paint(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        cx.backend.fill_round_rect(panel, 12.0, self.theme.card);
        cx.backend
            .stroke_round_rect(panel, 12.0, self.theme.border, 1.0);

        paint_text(
            cx,
            translate(self.ui, "htmlImport.diagnostics.title"),
            Point2D::new(panel.origin.x + HORIZONTAL_PAD, panel.origin.y + 26.0),
            14.0,
            600,
            self.theme.foreground,
        );
        paint_text(
            cx,
            &self.summary_text(),
            Point2D::new(panel.origin.x + HORIZONTAL_PAD, panel.origin.y + 45.0),
            11.0,
            400,
            self.theme.muted_foreground,
        );

        let hover = self.ui.html_import_diagnostics_hover;
        let toggle = self.toggle_rect(panel);
        paint_button(
            cx,
            &self.theme,
            toggle,
            translate(self.ui, self.toggle_key()),
            hover == Some(HtmlImportDiagnosticsHover::Toggle),
        );
        let dismiss = self.dismiss_rect(panel);
        paint_button(
            cx,
            &self.theme,
            dismiss,
            translate(self.ui, "htmlImport.diagnostics.dismiss"),
            hover == Some(HtmlImportDiagnosticsHover::Dismiss),
        );

        if !self.expanded() {
            return;
        }
        let rows = self.rows_rect(panel);
        let scroll = self.rows_scroll(panel);
        let text_width = (rows.size.x - HORIZONTAL_PAD * 2.0).max(0.0);
        let hidden = self.hidden_rows();
        cx.backend.save();
        cx.backend.clip_rect(rows);
        for index in 0..self.row_count() {
            let top = rows.origin.y + index as f32 * ROW_HEIGHT - scroll;
            if top + ROW_HEIGHT <= rows.origin.y || top >= rows.origin.y + rows.size.y {
                continue;
            }
            let row = Rect {
                origin: Point2D::new(rows.origin.x, top),
                size: Point2D::new(rows.size.x, ROW_HEIGHT),
            };
            if index > 0 {
                cx.backend.fill_rect(
                    Rect {
                        origin: Point2D::new(row.origin.x + HORIZONTAL_PAD, row.origin.y),
                        size: Point2D::new(row.size.x - HORIZONTAL_PAD * 2.0, 1.0),
                    },
                    self.theme.border,
                );
            }
            let Some(diagnostic) = self.rows().get(index) else {
                // Trailing `+N more`: the rows the cap dropped. It carries no
                // code line, so it centres in the row instead.
                let label = op_i18n::translate_with(
                    self.ui.effective_locale(),
                    "htmlImport.diagnostics.more",
                    &[("count", hidden.to_string().as_str())],
                );
                paint_text(
                    cx,
                    &label,
                    Point2D::new(
                        row.origin.x + HORIZONTAL_PAD,
                        centered_text_baseline_y(row, ROW_FONT_SIZE),
                    ),
                    ROW_FONT_SIZE,
                    500,
                    self.theme.muted_foreground,
                );
                continue;
            };
            let lines = wrap_row_lines(
                cx,
                &row_text(self.ui, diagnostic),
                ROW_FONT_SIZE,
                text_width,
            );
            for (line_index, line) in lines.iter().enumerate() {
                paint_text(
                    cx,
                    line,
                    Point2D::new(
                        row.origin.x + HORIZONTAL_PAD,
                        row.origin.y + 15.0 + line_index as f32 * 13.0,
                    ),
                    ROW_FONT_SIZE,
                    400,
                    self.theme.foreground,
                );
            }
            paint_text(
                cx,
                &diagnostic.code,
                Point2D::new(
                    row.origin.x + HORIZONTAL_PAD,
                    row.origin.y + ROW_HEIGHT - 8.0,
                ),
                ROW_CODE_FONT_SIZE,
                400,
                self.theme.muted_foreground,
            );
        }
        cx.backend.restore();
    }
}
