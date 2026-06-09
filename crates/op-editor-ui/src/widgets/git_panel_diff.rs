//! Diff-view rendering + scroll metrics for [`GitPanel`].
//!
//! Split out of `git_panel.rs` to keep that file under the repo's
//! 800-line cap. The diff view replaces the panel's status / action
//! body while [`op_editor_core::GitPanelState::diff`] is set; the
//! header carries ◀ / ▶ / ▲ / ▼ / ✕ controls and the body paints a
//! per-line-coloured unified diff with intra-line change highlight.

use crate::widgets::git_panel::{
    truncate, GitPanel, GitPanelHit, DIFF_VIEW_HEIGHT, FOOTER_H, GIT_DIFF_PANEL_WIDTH,
    HEADER_BASELINE, PAD,
};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};
use op_editor_core::{GitButton, GitDiffView};

/// Diff body line height.
const DIFF_LINE_H: f32 = 14.0;
/// Baseline of the first diff body line, from the panel top.
const DIFF_BODY_TOP: f32 = 56.0;
/// Diff-header button side length + gap between buttons.
const DIFF_BTN: f32 = 22.0;
const DIFF_BTN_GAP: f32 = 6.0;
/// Approximate character advance — keep in sync with the `max_chars`
/// width divisor so hit/paint columns agree.
const CHAR_W: f32 = 6.0;
/// Columns a single ◀/▶ press scrolls.
const DIFF_H_STEP: usize = 16;
/// Width of a per-hunk "Stage" button.
const HUNK_BTN_W: f32 = 46.0;

impl GitPanel<'_> {
    /// The diff header's `[◀, ▶, ▲, ▼, ✕]` button rects, right-aligned.
    pub(super) fn diff_header_buttons(panel: Rect) -> [Rect; 5] {
        let y = panel.origin.y + 12.0;
        let right = panel.origin.x + panel.size.x - PAD;
        // `from_right` 0 = ✕ (rightmost) … 4 = ◀ (leftmost).
        let slot = |from_right: f32| Rect {
            origin: Point2D::new(
                right - (from_right + 1.0) * DIFF_BTN - from_right * DIFF_BTN_GAP,
                y,
            ),
            size: Point2D::new(DIFF_BTN, DIFF_BTN),
        };
        [slot(4.0), slot(3.0), slot(2.0), slot(1.0), slot(0.0)]
    }

    /// Number of diff lines visible in the body at once.
    fn diff_visible_lines() -> usize {
        let body_h = DIFF_VIEW_HEIGHT - DIFF_BODY_TOP - FOOTER_H - PAD;
        ((body_h / DIFF_LINE_H).floor() as usize).max(1)
    }

    /// Number of characters visible across the diff body at once.
    fn diff_visible_cols() -> usize {
        (((GIT_DIFF_PANEL_WIDTH - PAD * 2.0) / CHAR_W).floor() as usize).max(1)
    }

    /// Lines a single ▲/▼ press scrolls — one body-page less a small
    /// overlap so context carries across the jump.
    pub fn diff_page_step() -> usize {
        Self::diff_visible_lines().saturating_sub(2).max(1)
    }

    /// Columns a single ◀/▶ press scrolls.
    pub fn diff_h_step() -> usize {
        DIFF_H_STEP
    }

    /// Largest valid vertical scroll offset for the open diff.
    pub fn diff_max_scroll(&self) -> usize {
        match &self.state.diff {
            Some(view) => view.lines.len().saturating_sub(Self::diff_visible_lines()),
            None => 0,
        }
    }

    /// Largest valid horizontal scroll offset — the longest line's
    /// length past the visible column count.
    pub fn diff_max_h_scroll(&self) -> usize {
        match &self.state.diff {
            Some(view) => {
                let longest = view
                    .lines
                    .iter()
                    .map(|l| l.chars().count())
                    .max()
                    .unwrap_or(0);
                longest.saturating_sub(Self::diff_visible_cols())
            }
            None => 0,
        }
    }

    /// One `(hunk index, button rect)` per `@@` hunk header visible
    /// in the diff body — only when the diff is a single working-tree
    /// file (`stage_path` set), which is what per-hunk staging needs.
    pub(super) fn diff_hunk_buttons(&self, panel: Rect) -> Vec<(usize, Rect)> {
        let Some(view) = &self.state.diff else {
            return Vec::new();
        };
        if view.stage_path.is_none() {
            return Vec::new();
        }
        let visible = Self::diff_visible_lines();
        let scroll = view.scroll.min(self.diff_max_scroll());
        let mut out = Vec::new();
        for row in 0..visible.min(view.lines.len().saturating_sub(scroll)) {
            let abs = scroll + row;
            if !view.lines[abs].starts_with("@@") {
                continue;
            }
            // The hunk index is the count of `@@` headers before it.
            let hunk = view.lines[..abs]
                .iter()
                .filter(|l| l.starts_with("@@"))
                .count();
            let baseline = panel.origin.y + DIFF_BODY_TOP + row as f32 * DIFF_LINE_H;
            out.push((
                hunk,
                Rect {
                    origin: Point2D::new(
                        panel.origin.x + panel.size.x - PAD - HUNK_BTN_W,
                        baseline - DIFF_LINE_H + 1.0,
                    ),
                    size: Point2D::new(HUNK_BTN_W, DIFF_LINE_H),
                },
            ));
        }
        out
    }

    /// Paint the scrollable unified-diff view that replaces the
    /// status / action body while `GitPanelState::diff` is set.
    pub(super) fn paint_diff(&self, cx: &mut PaintCx<'_>, rect: Rect, view: &GitDiffView) {
        let left = rect.origin.x + PAD;
        let top = rect.origin.y;

        // Header — title + ◀ / ▶ / ▲ / ▼ / ✕ buttons.
        let title = truncate(
            &self
                .t("git.panel.diffTitle")
                .replace("{{title}}", &view.title),
            56,
        );
        self.text(
            cx,
            &title,
            left,
            top + HEADER_BASELINE,
            13.0,
            self.theme.foreground,
        );
        let [bl, br, up, down, close] = Self::diff_header_buttons(rect);
        let h = self.state.button_hover;
        self.paint_glyph_button(cx, bl, "◀", h == Some(GitButton::DiffScrollLeft));
        self.paint_glyph_button(cx, br, "▶", h == Some(GitButton::DiffScrollRight));
        self.paint_glyph_button(cx, up, "▲", h == Some(GitButton::DiffScrollUp));
        self.paint_glyph_button(cx, down, "▼", h == Some(GitButton::DiffScrollDown));
        self.paint_glyph_button(cx, close, "✕", h == Some(GitButton::CloseDiff));
        self.divider(cx, left, top + 42.0, rect.size.x);

        // Body — a visible window of diff lines, per-line coloured,
        // with intra-line highlight on the changed span.
        let visible = Self::diff_visible_lines();
        let scroll = view.scroll.min(self.diff_max_scroll());
        let h_scroll = view.h_scroll.min(self.diff_max_h_scroll());
        let max_chars = Self::diff_visible_cols();
        if view.lines.is_empty() {
            self.text(
                cx,
                self.t("git.panel.diffNoChanges"),
                left,
                top + DIFF_BODY_TOP,
                12.0,
                self.theme.muted_foreground,
            );
        }
        for row in 0..visible.min(view.lines.len().saturating_sub(scroll)) {
            let abs = scroll + row;
            let line = &view.lines[abs];
            let baseline = top + DIFF_BODY_TOP + row as f32 * DIFF_LINE_H;
            // Intra-line highlight — tint the changed span of a +/-
            // line against its adjacent opposite-sign partner.
            if let Some(span) = self.diff_change_span(&view.lines, abs) {
                self.paint_diff_change_tint(
                    cx,
                    line,
                    span,
                    Point2D::new(left, baseline),
                    h_scroll,
                    max_chars,
                );
            }
            let shown: String = line.chars().skip(h_scroll).take(max_chars).collect();
            self.text(cx, &shown, left, baseline, 11.0, self.diff_line_color(line));
        }

        // Per-hunk "Stage" buttons — only for a single working-tree
        // file's diff (`@@` headers in view).
        for (hunk, btn) in self.diff_hunk_buttons(rect) {
            self.paint_button_with_hit(
                cx,
                btn,
                self.t("git.panel.stage"),
                true,
                false,
                Some(GitPanelHit::StageHunk(hunk)),
            );
        }

        // Footer — scroll position + close hint.
        let total = view.lines.len();
        let shown_end = (scroll + visible).min(total);
        let start = if total == 0 { 0 } else { scroll + 1 };
        let col = if h_scroll == 0 {
            String::new()
        } else {
            self.t("git.panel.diffCol")
                .replace("{{n}}", &(h_scroll + 1).to_string())
        };
        self.text(
            cx,
            &self
                .t("git.panel.diffFooter")
                .replace("{{start}}", &start.to_string())
                .replace("{{end}}", &shown_end.to_string())
                .replace("{{total}}", &total.to_string())
                .replace("{{col}}", &col),
            left,
            top + self.height() - PAD,
            10.0,
            self.theme.muted_foreground,
        );
    }

    /// The changed character span `[start, end)` of the diff line at
    /// `index` — `None` unless it is an added / removed line paired
    /// with an adjacent opposite-sign line. The span is in full-line
    /// char coordinates (the leading +/- marker included).
    fn diff_change_span(&self, lines: &[String], index: usize) -> Option<(usize, usize)> {
        let line = lines.get(index)?;
        // Skip file-header `+++` / `---` lines — only true content.
        if line.starts_with("+++") || line.starts_with("---") {
            return None;
        }
        let partner = if line.starts_with('+') {
            lines
                .get(index.wrapping_sub(1))
                .filter(|p| p.starts_with('-'))
        } else if line.starts_with('-') {
            lines.get(index + 1).filter(|p| p.starts_with('+'))
        } else {
            None
        }?;
        let a: Vec<char> = line.chars().skip(1).collect();
        let b: Vec<char> = partner.chars().skip(1).collect();
        let prefix = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
        let suffix = a
            .iter()
            .rev()
            .zip(b.iter().rev())
            .take_while(|(x, y)| x == y)
            .count()
            .min(a.len().saturating_sub(prefix));
        let end = a.len() - suffix;
        if end <= prefix {
            return None;
        }
        // +1 shifts past the marker into full-line coordinates.
        Some((prefix + 1, end + 1))
    }

    /// Tint the visible portion of a changed span behind the text.
    /// `origin` is the line's `(left, baseline)`.
    fn paint_diff_change_tint(
        &self,
        cx: &mut PaintCx<'_>,
        line: &str,
        span: (usize, usize),
        origin: Point2D,
        h_scroll: usize,
        max_chars: usize,
    ) {
        let (span_start, span_end) = span;
        let view_end = h_scroll + max_chars;
        let vis_start = span_start.max(h_scroll);
        let vis_end = span_end.min(view_end);
        if vis_end <= vis_start {
            return;
        }
        let x0 = origin.x + (vis_start - h_scroll) as f32 * CHAR_W;
        let w = (vis_end - vis_start) as f32 * CHAR_W;
        let tint = if line.starts_with('+') {
            Color {
                r: 0.32,
                g: 0.73,
                b: 0.42,
                a: 0.22,
            }
        } else {
            Color {
                r: 0.92,
                g: 0.38,
                b: 0.38,
                a: 0.22,
            }
        };
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(x0, origin.y - DIFF_LINE_H + 3.0),
                size: Point2D::new(w, DIFF_LINE_H),
            },
            tint,
        );
    }

    /// Per-line colour for a unified-diff line.
    fn diff_line_color(&self, line: &str) -> Color {
        let green = Color {
            r: 0.32,
            g: 0.73,
            b: 0.42,
            a: 1.0,
        };
        let red = Color {
            r: 0.92,
            g: 0.38,
            b: 0.38,
            a: 1.0,
        };
        let blue = Color {
            r: 0.38,
            g: 0.63,
            b: 0.93,
            a: 1.0,
        };
        if line.starts_with("@@") {
            blue
        } else if line.starts_with("+++")
            || line.starts_with("---")
            || line.starts_with("diff ")
            || line.starts_with("index ")
            || line.starts_with("new file")
            || line.starts_with("deleted file")
            || line.starts_with("rename ")
            || line.starts_with("similarity ")
            || line.starts_with("commit ")
            || line.starts_with("Author:")
            || line.starts_with("Date:")
        {
            self.theme.muted_foreground
        } else if line.starts_with('+') {
            green
        } else if line.starts_with('-') {
            red
        } else {
            self.theme.foreground
        }
    }

    /// Paint one small square glyph button — the diff-view controls
    /// and the branch-row "merge into current" button.
    pub(super) fn paint_glyph_button(
        &self,
        cx: &mut PaintCx<'_>,
        rect: Rect,
        glyph: &str,
        hovered: bool,
    ) {
        cx.backend.fill_round_rect(rect, 5.0, self.theme.muted);
        if hovered {
            cx.backend
                .fill_round_rect(rect, 5.0, self.theme.button_hover);
        }
        cx.backend
            .stroke_round_rect(rect, 5.0, self.theme.border, 1.0);
        let baseline = rect.origin.y + rect.size.y / 2.0 + 4.0;
        self.text(
            cx,
            glyph,
            rect.origin.x + rect.size.x / 2.0 - 4.0,
            baseline,
            11.0,
            self.theme.foreground,
        );
    }
}
