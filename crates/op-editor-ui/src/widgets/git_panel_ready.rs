//! Ready-state (clean bound-repo) view for [`GitPanel`] — a port of
//! the TS `GitPanelReady`: a compact header (branch button + pull/push
//! + overflow `…`), a commit textarea, and the recent-commit history.
//!
//! Split out of `git_panel.rs` to keep that file under the repo's
//! 800-line cap. The merge / loading / not-repo states keep the classic
//! header + body in `git_panel.rs`; only the clean bound-repo view is
//! the TS popover layout here.
//!
//! Geometry is shared by paint + hit-test through the `ready_*` rect
//! helpers, and the branch button uses a char-width heuristic (not text
//! measurement) so the pure-geometry hit-test agrees with paint.

use crate::widgets::git_panel::{truncate, GitPanel, GitPanelHit, PAD};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};
use jian_widgets::components::text_area::TextArea;
use op_editor_core::{CommitDiffSummary, CommitDiffView, GitButton};

/// Header bar height (TS `px-2.5 py-1.5` around a 28 px `icon-sm`
/// button = 6 + 28 + 6 = 40 px row).
const HEADER_H: f32 = 40.0;
/// 28 px ghost icon button (TS `size="icon-sm"` = `h-7 w-7`).
const ICON_BTN: f32 = 28.0;
/// Horizontal inset for the ready view — tighter than the classic
/// body's `PAD` (16) to match the TS header `px-2.5` (10 px). Shared
/// by the header, commit box, and history so the column lines up.
const READY_PAD: f32 = 10.0;
/// Commit-box geometry — the TS `p-3` outer wrapper (12 px) around a
/// `rounded-lg` card. The card = a 2-row `text-xs` `leading-relaxed`
/// textarea (`pt-2.5`+`pb-1` ≈ 53 px) over the `justify-end` button row
/// (`h-6` 24 px + `pb-1.5` 6 px = 30 px) ≈ 83 px.
const COMMIT_TOP: f32 = HEADER_H + 12.0;
const COMMIT_H: f32 = 83.0;
/// Height of the commit-signature form (`提交署名`) when it replaces the
/// commit box — heading + subheading + name + email inputs + button row.
const AUTHOR_FORM_H: f32 = 168.0;
const ROW_H: f32 = 26.0;
const MAX_COMMITS: usize = 8;
/// Extra empty body height appended below the recent-commit history so the
/// ready panel reads roomy (TS fixed-height feel) instead of hugging a short
/// history. Constant, so the panel still grows with more commits.
const READY_FILL: f32 = 200.0;
const SUMMARY_MAX: usize = 34;
/// Base height of the inline commit-detail card (里程碑详情 title + one
/// status/summary line + a restore/copy-hash button row) under an expanded
/// commit row (TS `HistoryMilestoneRow` detail block). Grows by one
/// [`PATCH_ROW_H`] per rendered patch line. The extra height over the button
/// row (anchored at `height - 52`) is bottom padding so the next commit row
/// stays well clear of the buttons.
const CARD_BASE_H: f32 = 104.0;
/// Per-patch-line height in the expanded diff list.
const PATCH_ROW_H: f32 = 14.0;
/// Cap on patch lines drawn in the card (TS scrolls a `max-h-24` list;
/// the summary counts still report the full totals above the list).
const MAX_PATCH_ROWS: usize = 6;
/// Per-char advance heuristic for the branch label (keeps paint +
/// hit-test aligned without measuring text).
const BRANCH_CHAR_W: f32 = 7.5;

impl GitPanel<'_> {
    /// `true` when the panel shows the TS ready layout — a clean,
    /// bound repository with no merge in progress and no diff /
    /// resolve takeover. Dirty working trees keep the classic status
    /// + staging body so per-file staging stays reachable.
    pub(super) fn is_ready_state(&self) -> bool {
        // TS parity: a bound, non-merging repo always shows the ready view
        // — whether the working tree is clean OR dirty. TS has no per-file
        // staging view; the commit-milestone flow saves + commits dirty
        // changes. (`changed_files` no longer gates this.)
        self.state.in_repo
            && !self.state.loading
            && !self.state.merging
            && self.state.diff.is_none()
            && self.state.merge_resolve.is_none()
    }

    /// Y of the first history row — below the commit box, or below the taller
    /// commit-signature form when it has replaced the box.
    fn history_first(&self) -> f32 {
        let box_h = if self.state.author_prompt {
            AUTHOR_FORM_H
        } else {
            COMMIT_H
        };
        COMMIT_TOP + box_h + 24.0
    }

    /// The panel height for the ready view — header + commit box (or signature
    /// form) + the recent-commit rows, plus [`READY_FILL`] body space.
    pub(super) fn ready_height(&self) -> f32 {
        let rows = self.state.recent_commits.len().clamp(1, MAX_COMMITS);
        self.history_first() + rows as f32 * ROW_H + PAD + self.expanded_card_extra() + READY_FILL
    }

    /// Number of patch lines the expanded card will draw (0 unless the diff
    /// is `Ready`, capped at [`MAX_PATCH_ROWS`]).
    fn card_patch_rows(&self) -> usize {
        match &self.state.expanded_commit_diff {
            Some(CommitDiffView::Ready(s)) => s.patches.len().min(MAX_PATCH_ROWS),
            _ => 0,
        }
    }

    /// Total height of the open card — base chrome plus one row per drawn
    /// patch line. Shared by paint + the hit-test walk so they agree.
    fn expanded_card_height(&self) -> f32 {
        CARD_BASE_H + self.card_patch_rows() as f32 * PATCH_ROW_H
    }

    /// Extra height contributed by an open inline commit-detail card —
    /// [`Self::expanded_card_height`] when a valid row is expanded, else 0.
    /// Shared by [`GitPanel::ready_height`] + the history paint / hit-test
    /// walk so they stay in lockstep.
    fn expanded_card_extra(&self) -> f32 {
        let n = self.state.recent_commits.len().min(MAX_COMMITS);
        match self.state.expanded_commit {
            Some(e) if e < n => self.expanded_card_height(),
            _ => 0.0,
        }
    }

    /// Vertical offset inserted before commit row `i` by a detail card
    /// open under an earlier row.
    fn expand_offset_before(&self, i: usize) -> f32 {
        let n = self.state.recent_commits.len().min(MAX_COMMITS);
        match self.state.expanded_commit {
            Some(e) if e < i && e < n => self.expanded_card_height(),
            _ => 0.0,
        }
    }

    /// `(恢复, 复制哈希)` button rects for the inline card whose top edge
    /// is `card_top`. Backend-free fixed widths keep paint + hit aligned.
    /// The button row is bottom-anchored so a growing patch list pushes it
    /// down in lockstep with paint.
    fn commit_card_button_rects(&self, rect: Rect, card_top: f32) -> (Rect, Rect) {
        // Button row sits at content position `52 + patches`; the rest of the
        // card height (CARD_BASE_H - 52 = 52px) is bottom padding so the next
        // commit row keeps well clear of the buttons.
        let btn_y = card_top + self.expanded_card_height() - 52.0;
        let h = 24.0;
        let x = rect.origin.x + 40.0; // align with the message column (`pl-10`)
        let restore = Rect {
            origin: Point2D::new(x, btn_y),
            size: Point2D::new(64.0, h),
        };
        let copy = Rect {
            origin: Point2D::new(x + 64.0 + 8.0, btn_y),
            size: Point2D::new(84.0, h),
        };
        (restore, copy)
    }

    /// `(恢复, 复制哈希)` rects for the currently-expanded card, or `None`
    /// when nothing is expanded. Mirrors the history paint y-walk so the
    /// hit-test lands exactly where paint drew the buttons.
    pub(super) fn ready_commit_card_buttons(&self, rect: Rect) -> Option<(Rect, Rect)> {
        let n = self.state.recent_commits.len().min(MAX_COMMITS);
        let e = self.state.expanded_commit.filter(|&e| e < n)?;
        // Row `e`'s text baseline (no prior card offsets — only one card
        // can be open) then `+ ROW_H` to the card top, matching paint.
        let card_top = rect.origin.y + self.history_first() + (e as f32 + 1.0) * ROW_H - 6.0;
        Some(self.commit_card_button_rects(rect, card_top))
    }

    /// Resolved branch label (`detached HEAD` fallback).
    fn ready_branch_label(&self) -> String {
        self.state
            .branch
            .clone()
            .unwrap_or_else(|| self.t("git.panel.detachedHead").to_string())
    }

    /// Branch-button rect — `⎇ <branch> ▾`, left-aligned in the header.
    /// Width is clamped so a long branch name can never push the
    /// pull / push / overflow icon cluster past the right edge.
    pub(super) fn ready_branch_rect(&self, rect: Rect) -> Rect {
        let label = self.ready_branch_label();
        let raw_w = 18.0 + label.chars().count() as f32 * BRANCH_CHAR_W + 16.0;
        // Reserve room for pull + push + overflow (3 ICON_BTN) + the
        // inter-button gaps (2px ×3) + both READY_PAD insets.
        let reserved = 3.0 * ICON_BTN + 6.0 + READY_PAD * 2.0;
        let w = raw_w.min((rect.size.x - reserved).max(40.0));
        Rect {
            origin: Point2D::new(
                rect.origin.x + READY_PAD,
                rect.origin.y + (HEADER_H - ICON_BTN) / 2.0,
            ),
            size: Point2D::new(w, ICON_BTN),
        }
    }

    /// The three header icon buttons: (pull `↓`, push `↑`, overflow `…`).
    pub(super) fn ready_header_buttons(&self, rect: Rect) -> (Rect, Rect, Rect) {
        let y = rect.origin.y + (HEADER_H - ICON_BTN) / 2.0;
        let branch = self.ready_branch_rect(rect);
        let pull_x = branch.origin.x + branch.size.x + 2.0;
        let push_x = pull_x + ICON_BTN + 2.0;
        let overflow_x = rect.origin.x + rect.size.x - READY_PAD - ICON_BTN;
        let mk = |x: f32| Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(ICON_BTN, ICON_BTN),
        };
        (mk(pull_x), mk(push_x), mk(overflow_x))
    }

    /// The bordered commit textarea box.
    pub(super) fn ready_commit_box(&self, rect: Rect) -> Rect {
        Rect {
            origin: Point2D::new(rect.origin.x + READY_PAD, rect.origin.y + COMMIT_TOP),
            size: Point2D::new(rect.size.x - READY_PAD * 2.0, COMMIT_H),
        }
    }

    /// The `保存为里程碑` button anchored bottom-right inside the box.
    pub(super) fn ready_commit_btn(&self, rect: Rect) -> Rect {
        let b = self.ready_commit_box(rect);
        // Wider than the label alone so the leading milestone icon fits.
        let w = 108.0;
        let h = 24.0;
        Rect {
            origin: Point2D::new(
                b.origin.x + b.size.x - w - 6.0,
                b.origin.y + b.size.y - h - 6.0,
            ),
            size: Point2D::new(w, h),
        }
    }

    /// Whether the "Save milestone" button can fire — a non-empty
    /// message (TS `canSubmit = commitMessage.trim().length > 0`). The
    /// ready-view commit saves the live design to the tracked `.op` and
    /// commits it in one step, so it does NOT require a pre-staged file
    /// the way the classic staging body's Commit does.
    fn ready_can_commit(&self) -> bool {
        !self.state.commit_input.text().trim().is_empty()
    }

    /// Whether Pull can fire — a configured remote and no in-flight
    /// pull / push (TS `pullDisabled = !hasRemote || busy`).
    fn pull_enabled(&self) -> bool {
        !self.state.remotes.is_empty() && !self.state.pulling && !self.state.pushing
    }

    /// Whether Push can fire — Pull's conditions plus `ahead > 0` (TS
    /// also disables Push when up-to-date, `ahead === 0`).
    fn push_enabled(&self) -> bool {
        self.pull_enabled() && self.state.ahead > 0
    }

    /// Paint the ready view.
    /// `(name_input, email_input, save, cancel)` rects of the commit-signature
    /// form, shared by paint + hit-test.
    pub(super) fn author_form_rects(&self, rect: Rect) -> (Rect, Rect, Rect, Rect) {
        let left = rect.origin.x + READY_PAD;
        let inner_w = rect.size.x - READY_PAD * 2.0;
        let top = rect.origin.y + COMMIT_TOP;
        let name_input = Rect {
            origin: Point2D::new(left, top + 58.0),
            size: Point2D::new(inner_w, 28.0),
        };
        let email_input = Rect {
            origin: Point2D::new(left, top + 102.0),
            size: Point2D::new(inner_w, 28.0),
        };
        let btn_y = top + 138.0;
        let w = 56.0;
        let save = Rect {
            origin: Point2D::new(rect.origin.x + rect.size.x - READY_PAD - w, btn_y),
            size: Point2D::new(w, 26.0),
        };
        let cancel = Rect {
            origin: Point2D::new(save.origin.x - 8.0 - w, btn_y),
            size: Point2D::new(w, 26.0),
        };
        (name_input, email_input, save, cancel)
    }

    /// Paint the commit-signature form (`提交署名`) in the commit-box area.
    fn paint_author_form(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        let t = self.theme;
        let left = rect.origin.x + READY_PAD;
        let top = rect.origin.y + COMMIT_TOP;
        self.text(
            cx,
            self.t("git.author.heading"),
            left,
            top + 16.0,
            13.0,
            t.foreground,
        );
        self.text(
            cx,
            self.t("git.author.subheading"),
            left,
            top + 36.0,
            11.0,
            t.muted_foreground,
        );
        let (name_input, email_input, save, cancel) = self.author_form_rects(rect);
        self.text(
            cx,
            self.t("git.author.nameLabel"),
            left,
            top + 54.0,
            11.0,
            t.muted_foreground,
        );
        self.paint_menu_input(
            cx,
            name_input,
            &self.state.author_name_input,
            self.t("git.author.namePlaceholder"),
            self.state.author_name_focused,
        );
        self.text(
            cx,
            self.t("git.author.emailLabel"),
            left,
            top + 98.0,
            11.0,
            t.muted_foreground,
        );
        self.paint_menu_input(
            cx,
            email_input,
            &self.state.author_email_input,
            self.t("git.author.emailPlaceholder"),
            self.state.author_email_focused,
        );
        let can_save = !self.state.author_name_input.text().trim().is_empty()
            && self.state.author_email_input.text().contains('@');
        self.paint_button_with_hit(
            cx,
            cancel,
            self.t("git.author.cancel"),
            true,
            false,
            Some(GitPanelHit::AuthorCancel),
        );
        self.paint_button_with_hit(
            cx,
            save,
            self.t("git.author.submit"),
            can_save,
            true,
            Some(GitPanelHit::AuthorSave),
        );
    }

    pub(super) fn paint_ready(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        let t = self.theme;
        let top = rect.origin.y;
        let width = rect.size.x;

        // ── Header bar (TS `border-b border-border/60 bg-card/40`) ──
        cx.backend.fill_rect(
            Rect {
                origin: rect.origin,
                size: Point2D::new(width, HEADER_H),
            },
            alpha(t.card, 0.40),
        );
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(rect.origin.x, top + HEADER_H),
                size: Point2D::new(width, 1.0),
            },
            alpha(t.border, 0.60),
        );

        // Branch button — `⎇ <branch> ▾`, with a `hover:bg-accent` wash
        // (TS the branch trigger is a ghost button).
        let branch_r = self.ready_branch_rect(rect);
        if self.state.branch_button_hovered {
            cx.backend.fill_round_rect(branch_r, 6.0, t.accent);
        }
        let cy = top + HEADER_H / 2.0;
        draw_icon(
            cx.backend,
            Icon::GitBranch,
            Point2D::new(branch_r.origin.x, cy - 6.0),
            12.0,
            t.foreground,
            1.5,
        );
        // Truncate to the clamped button width (label area = button
        // width minus the 18px icon gutter and the 12px chevron).
        let max_chars = ((branch_r.size.x - 30.0) / BRANCH_CHAR_W).max(1.0) as usize;
        let label = truncate(&self.ready_branch_label(), max_chars);
        self.text(
            cx,
            &label,
            branch_r.origin.x + 18.0,
            cy + 4.0,
            12.0,
            t.foreground,
        );
        draw_icon(
            cx.backend,
            Icon::ChevronDown,
            Point2D::new(branch_r.origin.x + branch_r.size.x - 14.0, cy - 6.0),
            12.0,
            t.muted_foreground,
            1.5,
        );

        // Pull / Push / Overflow icon buttons.
        let (pull_r, push_r, overflow_r) = self.ready_header_buttons(rect);
        self.paint_ready_icon(
            cx,
            pull_r,
            Icon::ArrowDown,
            self.pull_enabled(),
            self.state.button_hover == Some(GitButton::Pull),
            self.pressed == Some(GitButton::Pull),
        );
        self.paint_ready_icon(
            cx,
            push_r,
            Icon::ArrowUp,
            self.push_enabled(),
            self.state.button_hover == Some(GitButton::Push),
            self.pressed == Some(GitButton::Push),
        );
        // Overflow `…` — TS colors this `text-muted-foreground` at size 13,
        // dimmer than the pull / push glyphs (git-panel-header.tsx:127-129).
        crate::widgets::button::paint_ghost_button_feedback(
            cx.backend,
            &self.theme,
            overflow_r,
            self.state.button_hover == Some(GitButton::Overflow),
            self.pressed == Some(GitButton::Overflow),
        );
        let overflow_s = 13.0;
        draw_icon(
            cx.backend,
            Icon::MoreHorizontal,
            Point2D::new(
                overflow_r.origin.x + (overflow_r.size.x - overflow_s) / 2.0,
                overflow_r.origin.y + (overflow_r.size.y - overflow_s) / 2.0,
            ),
            overflow_s,
            t.muted_foreground,
            1.5,
        );

        // ── Commit box / signature form ──
        if self.state.author_prompt {
            self.paint_author_form(cx, rect);
        } else {
            let box_r = self.ready_commit_box(rect);
            cx.backend.fill_round_rect(box_r, 8.0, t.card);
            let border = if self.state.commit_focused {
                alpha(t.primary, 0.50)
            } else {
                alpha(t.border, 0.70)
            };
            cx.backend.stroke_round_rect(box_r, 8.0, border, 1.0);
            let text_r = Rect {
                origin: box_r.origin,
                size: Point2D::new(box_r.size.x, (box_r.size.y - 38.0).max(0.0)),
            };
            let placeholder = if self.state.commit_focused {
                ""
            } else {
                self.t("git.commit.placeholder")
            };
            {
                // jian TextArea paints glyphs top-relative; wrap the backend so
                // the run sits on its baseline (matches the chat commit input).
                let tokens = self.widget_tokens();
                let mut baselined = crate::widgets::text_input_backend::BaselineAdjustingBackend {
                    inner: cx.backend,
                    baseline_delta_y: 12.0,
                };
                TextArea {
                    state: &self.state.commit_input,
                    placeholder,
                    focused: self.state.commit_focused,
                    font_size: 12.0,
                    now_ms: self.now_ms,
                    pad_x: 12.0,
                    max_visible_lines: 2,
                }
                .paint(&mut baselined, text_r, &tokens);
            }
            let btn = self.ready_commit_btn(rect);
            self.paint_milestone_button(
                cx,
                btn,
                self.ready_can_commit(),
                self.state.button_hover == Some(GitButton::CommitMilestone),
                self.pressed == Some(GitButton::CommitMilestone),
            );
            // "未检测到变更" hint — shown to the left of the button after a
            // milestone save was skipped for having no changes (TS-style guard).
            if self.state.commit_no_changes {
                self.text(
                    cx,
                    self.t("git.history.diff.noChanges"),
                    box_r.origin.x + 4.0,
                    btn.origin.y + btn.size.y / 2.0 + 4.0,
                    11.0,
                    alpha(t.destructive, 0.90),
                );
            }
        }
        // Divider below the commit box / form, just above the history.
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(rect.origin.x, rect.origin.y + self.history_first() - 18.0),
                size: Point2D::new(width, 1.0),
            },
            alpha(t.border, 0.60),
        );

        // ── Recent-commit history (TS `git-panel-history-list`) ──
        // No section header — the timeline / empty line sits directly
        // under the commit-box divider.
        let mut y = top + self.history_first();
        if self.state.recent_commits.is_empty() {
            // Empty log → a single centered `git.history.empty` line
            // (TS `flex items-center justify-center p-6 text-xs
            // text-muted-foreground`). +14 gives the `p-6`-style top
            // breathing room so it doesn't crowd the commit-box divider.
            let label = self.t("git.history.empty");
            let tw = cx.backend.measure_text(label, 12.0);
            self.text(
                cx,
                label,
                rect.origin.x + (rect.size.x - tw) / 2.0,
                y + 14.0,
                12.0,
                t.muted_foreground,
            );
        } else {
            // TS `git-panel-history-list`: a timeline with a 1px rail down
            // the dot column (`left-5`), a 7px milestone dot per row, the
            // message (`text-[12px] text-foreground`) left, and the author
            // (`font-mono text-[10px] text-muted-foreground/80`) right.
            let rail_x = rect.origin.x + 20.0;
            let msg_x = rect.origin.x + 40.0;
            let n = self.state.recent_commits.len().min(MAX_COMMITS);
            cx.backend.fill_rect(
                Rect {
                    origin: Point2D::new(rail_x, y - 8.0),
                    size: Point2D::new(1.0, n as f32 * ROW_H + self.expanded_card_extra()),
                },
                alpha(t.border, 0.60),
            );
            for (i, commit) in self
                .state
                .recent_commits
                .iter()
                .take(MAX_COMMITS)
                .enumerate()
            {
                cx.backend.fill_round_rect(
                    Rect {
                        origin: Point2D::new(rail_x - 3.5, y - 7.5),
                        size: Point2D::new(7.0, 7.0),
                    },
                    3.5,
                    t.foreground,
                );
                // Right meta — `<author-first-token> · <relative-time>`
                // (TS `{authorShort} · {timeAgo}`).
                let meta = format!(
                    "{} · {}",
                    author_first_token(&commit.author),
                    commit.time_label,
                );
                let author_w = cx.backend.measure_text(&meta, 10.0);
                let author_x = rect.origin.x + width - READY_PAD - author_w;
                // Truncate the message to the space left of the meta.
                let msg_w = (author_x - msg_x - 8.0).max(0.0);
                let chars = (msg_w / 7.0) as usize;
                self.text(
                    cx,
                    &truncate(&commit.summary, chars.min(SUMMARY_MAX)),
                    msg_x,
                    y,
                    12.0,
                    t.foreground,
                );
                self.text(
                    cx,
                    &meta,
                    author_x,
                    y,
                    10.0,
                    alpha(t.muted_foreground, 0.80),
                );
                y += ROW_H;
                // Inline detail card under the expanded row (TS
                // `HistoryMilestoneRow` detail block). Captures the card
                // top right after the row advance so the hit-test helper
                // `ready_commit_card_buttons` lands on the same geometry.
                if self.state.expanded_commit == Some(i) {
                    let card_top = y - 6.0;
                    self.paint_commit_card(cx, rect, card_top);
                    y += self.expanded_card_height();
                }
            }
        }
    }

    /// Paint the inline commit-detail card (里程碑详情) — a muted band with
    /// the detail title, the semantic diff (TS `GitPanelHistoryDiff`: a
    /// summary row + an `op nodeId` patch list, or a loading / initial /
    /// no-changes / error line), and a `恢复` / `复制哈希` button row.
    fn paint_commit_card(&self, cx: &mut PaintCx<'_>, rect: Rect, card_top: f32) {
        let t = self.theme;
        let h = self.expanded_card_height();
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(rect.origin.x, card_top),
                size: Point2D::new(rect.size.x, h),
            },
            alpha(t.muted, 0.30),
        );
        let body_x = rect.origin.x + 40.0;
        // Title — `text-[11px] font-medium`.
        self.text(
            cx,
            self.t("git.history.milestoneDetailTitle"),
            body_x,
            card_top + 18.0,
            11.0,
            t.foreground,
        );
        // Diff body (TS `GitPanelHistoryDiff` states).
        let status_y = card_top + 38.0;
        match &self.state.expanded_commit_diff {
            None | Some(CommitDiffView::Loading) => {
                self.text(
                    cx,
                    self.t("git.history.diff.loading"),
                    body_x,
                    status_y,
                    10.0,
                    alpha(t.muted_foreground, 0.85),
                );
            }
            Some(CommitDiffView::Initial) => {
                self.text(
                    cx,
                    self.t("git.history.diff.initialCommit"),
                    body_x,
                    status_y,
                    10.0,
                    alpha(t.muted_foreground, 0.85),
                );
            }
            Some(CommitDiffView::NoChanges) => {
                self.text(
                    cx,
                    self.t("git.history.diff.noChanges"),
                    body_x,
                    status_y,
                    10.0,
                    alpha(t.muted_foreground, 0.85),
                );
            }
            Some(CommitDiffView::Error(msg)) => {
                let label = self.t("git.history.diff.error").replace("{{message}}", msg);
                self.text(cx, &label, body_x, status_y, 10.0, t.destructive);
            }
            Some(CommitDiffView::Ready(summary)) => {
                self.paint_diff_summary(cx, summary, body_x, status_y);
                // Patch list — one `op nodeId` line each (TS font-mono).
                for (k, p) in summary.patches.iter().take(MAX_PATCH_ROWS).enumerate() {
                    let py = card_top + 54.0 + k as f32 * PATCH_ROW_H;
                    self.text(cx, &p.op, body_x, py, 10.0, t.foreground);
                    let opw = cx.backend.measure_text(&p.op, 10.0);
                    self.text(
                        cx,
                        &p.node_id,
                        body_x + opw + 6.0,
                        py,
                        10.0,
                        alpha(t.muted_foreground, 0.70),
                    );
                }
            }
        }
        let (restore, copy) = self.commit_card_button_rects(rect, card_top);
        if let Some(e) = self.state.expanded_commit {
            self.wash_if_hovered(cx, restore, 6.0, GitPanelHit::RestoreCommit(e));
            self.wash_if_hovered(cx, copy, 6.0, GitPanelHit::CopyCommitHash(e));
        }
        // 恢复 — outline button.
        cx.backend.stroke_round_rect(restore, 6.0, t.border, 1.0);
        self.center_label(
            cx,
            self.t("git.history.restoreButton"),
            restore,
            t.foreground,
        );
        // 复制哈希 — ghost button (no border).
        self.center_label(
            cx,
            self.t("git.history.copyHashButton"),
            copy,
            alpha(t.foreground, 0.80),
        );
    }

    /// Draw an 11px label horizontally + vertically centred in `r`.
    fn center_label(&self, cx: &mut PaintCx<'_>, label: &str, r: Rect, color: Color) {
        let tw = cx.backend.measure_text(label, 11.0);
        self.text(
            cx,
            label,
            r.origin.x + (r.size.x - tw) / 2.0,
            r.origin.y + r.size.y / 2.0 + 4.0,
            11.0,
            color,
        );
    }

    /// Paint the diff summary row — coloured `framesChanged` / `+added` /
    /// `-removed` / `~modified` segments left-to-right (TS `GitPanelHistoryDiff`
    /// summary spans). Only non-zero counts render.
    fn paint_diff_summary(&self, cx: &mut PaintCx<'_>, s: &CommitDiffSummary, x: f32, y: f32) {
        let t = self.theme;
        // Build the (label, colour) segments first so the draw loop borrows
        // `self` only through `self.text` / the backend measure.
        let mut segments: Vec<(String, Color)> = Vec::new();
        if s.frames_changed > 0 {
            segments.push((
                self.plural(
                    "git.history.diff.framesChanged_one",
                    "git.history.diff.framesChanged_other",
                    s.frames_changed,
                ),
                t.muted_foreground,
            ));
        }
        if s.nodes_added > 0 {
            segments.push((
                format!(
                    "+{}",
                    self.plural(
                        "git.history.diff.nodesAdded_one",
                        "git.history.diff.nodesAdded_other",
                        s.nodes_added,
                    )
                ),
                t.primary,
            ));
        }
        if s.nodes_removed > 0 {
            segments.push((
                format!(
                    "-{}",
                    self.plural(
                        "git.history.diff.nodesRemoved_one",
                        "git.history.diff.nodesRemoved_other",
                        s.nodes_removed,
                    )
                ),
                t.destructive,
            ));
        }
        if s.nodes_modified > 0 {
            segments.push((
                format!(
                    "~{}",
                    self.plural(
                        "git.history.diff.nodesModified_one",
                        "git.history.diff.nodesModified_other",
                        s.nodes_modified,
                    )
                ),
                t.muted_foreground,
            ));
        }
        let mut cur = x;
        for (label, color) in &segments {
            self.text(cx, label, cur, y, 10.0, *color);
            cur += cx.backend.measure_text(label, 10.0) + 10.0;
        }
    }

    /// Pick the `_one` / `_other` plural form (TS i18next English rule: 1 →
    /// one) and substitute `{{count}}`. Both keys are `&'static` so they
    /// satisfy [`GitPanel::t`]'s static-key contract.
    fn plural(&self, one_key: &'static str, other_key: &'static str, count: u32) -> String {
        let key = if count == 1 { one_key } else { other_key };
        self.t(key).replace("{{count}}", &count.to_string())
    }

    /// One ghost icon button — a faint rounded slot + a centred glyph,
    /// dimmed when disabled.
    fn paint_ready_icon(
        &self,
        cx: &mut PaintCx<'_>,
        rect: Rect,
        icon: Icon,
        enabled: bool,
        hovered: bool,
        pressed: bool,
    ) {
        // Ghost icon button — a `theme.button_hover` wash signals the
        // cursor is over it (matches the TS `hover:bg-accent` controls).
        if enabled {
            crate::widgets::button::paint_ghost_button_feedback(
                cx.backend,
                &self.theme,
                rect,
                hovered,
                pressed,
            );
        }
        // TS: enabled = currentColor (foreground), disabled = full
        // text-muted-foreground — no half-alpha (git-panel-remote-controls.tsx).
        let color = if enabled {
            self.theme.foreground
        } else {
            self.theme.muted_foreground
        };
        let s = 12.0;
        let c = Point2D::new(
            rect.origin.x + (rect.size.x - s) / 2.0,
            rect.origin.y + (rect.size.y - s) / 2.0,
        );
        draw_icon(cx.backend, icon, c, s, color, 1.5);
    }

    /// Paint the `保存为里程碑` milestone button — a port of the TS
    /// commit-input button (`variant="default"`, a primary-blue fill with
    /// a leading `Milestone` icon). Unlike the generic
    /// [`GitPanel::paint_button`], the disabled state stays primary-blue
    /// at half opacity (TS `disabled:opacity-50`) instead of going grey,
    /// and the signpost icon sits to the left of the centred label.
    fn paint_milestone_button(
        &self,
        cx: &mut PaintCx<'_>,
        rect: Rect,
        enabled: bool,
        hovered: bool,
        pressed: bool,
    ) {
        let t = self.theme;
        let factor = if enabled { 1.0 } else { 0.5 };
        cx.backend
            .fill_round_rect(rect, 6.0, alpha(t.primary, factor));
        // Brighten the primary fill on hover (TS `hover:bg-primary/90`
        // reads as a subtle lift); only while actionable.
        if enabled {
            crate::widgets::button::paint_ghost_button_feedback(
                cx.backend,
                &self.theme,
                rect,
                hovered,
                pressed,
            );
        }

        let label = self.t("git.commit.submitButton");
        let icon_s = 11.0;
        let gap = 4.0;
        let label_w = cx.backend.measure_text(label, 11.0);
        let content_w = icon_s + gap + label_w;
        let start_x = rect.origin.x + (rect.size.x - content_w).max(6.0) / 2.0;
        let color = alpha(t.primary_foreground, factor);

        cx.backend.save();
        cx.backend.clip_rect(rect);
        draw_icon(
            cx.backend,
            Icon::Milestone,
            Point2D::new(start_x, rect.origin.y + (rect.size.y - icon_s) / 2.0),
            icon_s,
            color,
            2.0,
        );
        self.text(
            cx,
            label,
            start_x + icon_s + gap,
            rect.origin.y + rect.size.y / 2.0 + 4.0,
            11.0,
            color,
        );
        cx.backend.restore();
    }

    /// One clickable rect per displayed recent-commit row. Shared by
    /// [`GitPanel::paint_ready`] + [`GitPanel::ready_hit`] so paint +
    /// hit-test stay aligned.
    pub(super) fn ready_commit_row_rects(&self, rect: Rect) -> Vec<Rect> {
        // +9 centres the 26px click target on the 12px row text whose
        // baseline is `history_first() + i*ROW_H` (was +4, bottom-biased).
        let first = rect.origin.y + self.history_first() - ROW_H + 9.0;
        (0..self.state.recent_commits.len().min(MAX_COMMITS))
            .map(|i| Rect {
                origin: Point2D::new(
                    rect.origin.x,
                    first + i as f32 * ROW_H + self.expand_offset_before(i),
                ),
                size: Point2D::new(rect.size.x, ROW_H),
            })
            .collect()
    }

    /// Map a press inside the ready view onto a [`GitPanelHit`].
    pub(super) fn ready_hit(&self, rect: Rect, point: Point2D) -> Option<GitPanelHit> {
        let (pull, push, overflow) = self.ready_header_buttons(rect);
        // While the signature form is up, the commit box is replaced by it —
        // its fields/buttons own the clicks in that region (header still works).
        if self.state.author_prompt {
            let (name, email, save, cancel) = self.author_form_rects(rect);
            if name.contains(point) {
                return Some(GitPanelHit::AuthorNameInput);
            }
            if email.contains(point) {
                return Some(GitPanelHit::AuthorEmailInput);
            }
            if save.contains(point) {
                return Some(GitPanelHit::AuthorSave);
            }
            if cancel.contains(point) {
                return Some(GitPanelHit::AuthorCancel);
            }
        } else if self.ready_can_commit() && self.ready_commit_btn(rect).contains(point) {
            // Save-milestone button (it sits inside the commit box).
            return Some(GitPanelHit::CommitMilestone);
        }
        // Overflow is the right-anchored fixed element — test it before
        // pull/push so the always-present `…` menu wins any residual
        // pixel overlap (defense-in-depth on top of the branch clamp).
        if overflow.contains(point) {
            return Some(GitPanelHit::Overflow);
        }
        if self.ready_branch_rect(rect).contains(point) {
            return Some(GitPanelHit::BranchPicker);
        }
        if self.pull_enabled() && pull.contains(point) {
            return Some(GitPanelHit::Pull);
        }
        if self.push_enabled() && push.contains(point) {
            return Some(GitPanelHit::Push);
        }
        if !self.state.author_prompt && self.ready_commit_box(rect).contains(point) {
            return Some(GitPanelHit::CommitInput);
        }
        // Expanded detail-card buttons win over the rows they sit between.
        if let (Some((restore, copy)), Some(e)) = (
            self.ready_commit_card_buttons(rect),
            self.state.expanded_commit,
        ) {
            if restore.contains(point) {
                return Some(GitPanelHit::RestoreCommit(e));
            }
            if copy.contains(point) {
                return Some(GitPanelHit::CopyCommitHash(e));
            }
        }
        for (i, row) in self.ready_commit_row_rects(rect).iter().enumerate() {
            if row.contains(point) {
                return Some(GitPanelHit::ShowCommitDiff(i));
            }
        }
        rect.contains(point).then_some(GitPanelHit::Inside)
    }
}

/// A colour at `factor` of its current alpha (Tailwind `/NN`).
fn alpha(c: Color, factor: f32) -> Color {
    crate::util::alpha(c, factor)
}

/// The first whitespace-delimited token of an author name — TS
/// `commit.author.name.split(/\s+/)[0]` ("Ada Lovelace" → "Ada",
/// "Kayshen-X" stays whole).
fn author_first_token(author: &str) -> &str {
    author.split_whitespace().next().unwrap_or(author)
}
