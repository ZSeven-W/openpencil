//! Diff-view rendering + scroll metrics for [`GitPanel`].
//!
//! Split out of `git_panel.rs` to keep that file under the repo's
//! 800-line cap. The diff view replaces the panel's status / action
//! body while [`op_editor_core::GitPanelState::diff`] is set; the
//! header carries ▲ / ▼ / ✕ controls and the body paints a
//! per-line-coloured unified diff.

use crate::widgets::git_panel::{
    truncate, GitPanel, DIFF_VIEW_HEIGHT, FOOTER_H, HEADER_BASELINE, PAD,
};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};
use op_editor_core::GitDiffView;

/// Diff body line height.
const DIFF_LINE_H: f32 = 14.0;
/// Baseline of the first diff body line, from the panel top.
const DIFF_BODY_TOP: f32 = 56.0;
/// Diff-header button side length + gap between buttons.
const DIFF_BTN: f32 = 22.0;
const DIFF_BTN_GAP: f32 = 6.0;

impl GitPanel<'_> {
    /// The diff header's `[▲, ▼, ✕]` button rects, right-aligned.
    pub(super) fn diff_header_buttons(panel: Rect) -> [Rect; 3] {
        let y = panel.origin.y + 12.0;
        let right = panel.origin.x + panel.size.x - PAD;
        // `from_right` 0 = ✕ (rightmost), 1 = ▼, 2 = ▲.
        let slot = |from_right: f32| Rect {
            origin: Point2D::new(
                right - (from_right + 1.0) * DIFF_BTN - from_right * DIFF_BTN_GAP,
                y,
            ),
            size: Point2D::new(DIFF_BTN, DIFF_BTN),
        };
        [slot(2.0), slot(1.0), slot(0.0)]
    }

    /// Number of diff lines visible in the body at once.
    fn diff_visible_lines() -> usize {
        let body_h = DIFF_VIEW_HEIGHT - DIFF_BODY_TOP - FOOTER_H - PAD;
        ((body_h / DIFF_LINE_H).floor() as usize).max(1)
    }

    /// Lines a single ▲/▼ press scrolls — one body-page less a small
    /// overlap so context carries across the jump.
    pub fn diff_page_step() -> usize {
        Self::diff_visible_lines().saturating_sub(2).max(1)
    }

    /// Largest valid scroll offset for the open diff (0 when none).
    pub fn diff_max_scroll(&self) -> usize {
        match &self.state.diff {
            Some(view) => view
                .lines
                .len()
                .saturating_sub(Self::diff_visible_lines()),
            None => 0,
        }
    }

    /// Paint the scrollable unified-diff view that replaces the
    /// status / action body while `GitPanelState::diff` is set.
    pub(super) fn paint_diff(&self, cx: &mut PaintCx<'_>, rect: Rect, view: &GitDiffView) {
        let left = rect.origin.x + PAD;
        let top = rect.origin.y;

        // Header — title + ▲ / ▼ / ✕ buttons.
        let title = truncate(&format!("Diff · {}", view.title), 64);
        self.text(cx, &title, left, top + HEADER_BASELINE, 13.0, self.theme.foreground);
        let [up, down, close] = Self::diff_header_buttons(rect);
        self.paint_glyph_button(cx, up, "▲");
        self.paint_glyph_button(cx, down, "▼");
        self.paint_glyph_button(cx, close, "✕");
        self.divider(cx, left, top + 42.0, rect.size.x);

        // Body — a visible window of diff lines, per-line coloured.
        let visible = Self::diff_visible_lines();
        let max_scroll = self.diff_max_scroll();
        let scroll = view.scroll.min(max_scroll);
        let max_chars = ((rect.size.x - PAD * 2.0) / 6.0) as usize;
        if view.lines.is_empty() {
            self.text(
                cx,
                "No changes.",
                left,
                top + DIFF_BODY_TOP,
                12.0,
                self.theme.muted_foreground,
            );
        }
        for (row, line) in view.lines.iter().skip(scroll).take(visible).enumerate() {
            let baseline = top + DIFF_BODY_TOP + row as f32 * DIFF_LINE_H;
            self.text(
                cx,
                &truncate(line, max_chars),
                left,
                baseline,
                11.0,
                self.diff_line_color(line),
            );
        }

        // Footer — scroll position + close hint.
        let total = view.lines.len();
        let shown_end = (scroll + visible).min(total);
        let start = if total == 0 { 0 } else { scroll + 1 };
        self.text(
            cx,
            &format!("lines {start}–{shown_end} of {total} · ✕ to close"),
            left,
            top + self.height() - PAD,
            10.0,
            self.theme.muted_foreground,
        );
    }

    /// Per-line colour for a unified-diff line.
    fn diff_line_color(&self, line: &str) -> Color {
        let green = Color { r: 0.32, g: 0.73, b: 0.42, a: 1.0 };
        let red = Color { r: 0.92, g: 0.38, b: 0.38, a: 1.0 };
        let blue = Color { r: 0.38, g: 0.63, b: 0.93, a: 1.0 };
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
    pub(super) fn paint_glyph_button(&self, cx: &mut PaintCx<'_>, rect: Rect, glyph: &str) {
        cx.backend.fill_round_rect(rect, 5.0, self.theme.muted);
        cx.backend.stroke_round_rect(rect, 5.0, self.theme.border, 1.0);
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
