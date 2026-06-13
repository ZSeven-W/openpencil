//! Status-line text + commit-input / merge-banner painters for
//! [`GitPanel`].
//!
//! Split out of `git_panel.rs` to keep that file under the repo's
//! 800-line cap. These paint the bound-repo status view's secondary
//! widgets: the working-tree status line (text + colour), the
//! commit-message input box, and the merge-in-progress banner that
//! replaces the commit input while a merge is underway.

use crate::widgets::git_panel::GitPanel;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};
use jian_widgets::components::text_area::TextArea;

impl GitPanel<'_> {
    /// The working-tree status line text + colour.
    pub(super) fn status_line(&self) -> (String, Color) {
        let blue = Color {
            r: 0.23,
            g: 0.51,
            b: 0.96,
            a: 1.0,
        };
        if self.state.pulling {
            return (self.t("git.panel.pulling").to_string(), blue);
        }
        if self.state.pushing {
            return (self.t("git.panel.pushing").to_string(), blue);
        }
        if self.state.conflicted_count > 0 {
            (
                self.t("git.panel.changedConflicted")
                    .replace("{{changed}}", &self.state.dirty_count.to_string())
                    .replace("{{conflicted}}", &self.state.conflicted_count.to_string()),
                Color {
                    r: 0.94,
                    g: 0.27,
                    b: 0.27,
                    a: 1.0,
                },
            )
        } else if self.state.dirty_count > 0 {
            (
                self.t("git.panel.changed")
                    .replace("{{count}}", &self.state.dirty_count.to_string()),
                Color {
                    r: 0.96,
                    g: 0.62,
                    b: 0.04,
                    a: 1.0,
                },
            )
        } else {
            (
                self.t("git.panel.clean").to_string(),
                Color {
                    r: 0.22,
                    g: 0.78,
                    b: 0.42,
                    a: 1.0,
                },
            )
        }
    }

    /// Paint the merge-in-progress banner into the action-input slot
    /// (the commit input is replaced by it during a merge).
    pub(super) fn paint_merge_banner(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        let amber = Color {
            r: 0.96,
            g: 0.62,
            b: 0.04,
            a: 1.0,
        };
        cx.backend.fill_round_rect(rect, 6.0, self.theme.muted);
        cx.backend.stroke_round_rect(rect, 6.0, amber, 1.0);
        let baseline = rect.origin.y + rect.size.y / 2.0 + 4.0;
        self.text(
            cx,
            self.t("git.panel.mergeInProgress"),
            rect.origin.x + 8.0,
            baseline,
            12.0,
            amber,
        );
    }

    /// Paint the commit-message input box.
    pub(super) fn paint_input(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_round_rect(rect, 6.0, self.theme.muted);
        let border = if self.state.commit_focused {
            self.theme.primary
        } else {
            self.theme.border
        };
        cx.backend.stroke_round_rect(rect, 6.0, border, 1.0);

        let placeholder = if self.state.commit_focused {
            ""
        } else {
            self.t("git.panel.commitPlaceholder")
        };
        TextArea {
            state: &self.state.commit_input,
            placeholder,
            focused: self.state.commit_focused,
            font_size: 12.0,
            now_ms: self.now_ms,
            pad_x: 8.0,
            max_visible_lines: 1,
        }
        .paint(
            cx.backend,
            Rect {
                origin: rect.origin,
                size: Point2D::new(rect.size.x, rect.size.y),
            },
            &self.widget_tokens(),
        );
    }
}
