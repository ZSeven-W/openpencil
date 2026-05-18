//! Tests for `widgets::git_panel` — moved to a sibling file to keep
//! `git_panel.rs` under the 800-line cap.

use crate::widgets::git_panel::*;
use crate::{Point2D, Rect};
use op_editor_core::{EditorState, GitCommitSummary, GitDiffView, GitPanelState};

fn state_with(panel: GitPanelState) -> EditorState {
    let mut s = EditorState::new();
    s.editor_ui.git_panel = panel;
    s
}

fn open_repo() -> GitPanelState {
    GitPanelState {
        open: true,
        in_repo: true,
        ..GitPanelState::default()
    }
}

fn centre(r: Rect) -> Point2D {
    Point2D::new(r.origin.x + r.size.x / 2.0, r.origin.y + r.size.y / 2.0)
}

/// A panel rect sized to the panel's current mode.
fn panel_rect(panel: &GitPanel<'_>) -> Rect {
    Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(panel.panel_width(), panel.height()),
    }
}

#[test]
fn closed_panel_yields_none() {
    let s = state_with(GitPanelState::default());
    assert!(GitPanel::for_editor(&s).is_none());
}

#[test]
fn open_panel_height_grows_with_commits() {
    let base = state_with(open_repo());
    let h0 = GitPanel::for_editor(&base).unwrap().height();
    let with_commits = state_with(GitPanelState {
        recent_commits: vec![
            GitCommitSummary {
                short_hash: "abc1234".into(),
                summary: "first".into(),
                author: "Ada".into(),
            };
            3
        ],
        ..open_repo()
    });
    let h3 = GitPanel::for_editor(&with_commits).unwrap().height();
    assert!(h3 > h0, "more commits → taller panel");
}

#[test]
fn empty_history_reserves_a_placeholder_row() {
    let empty = state_with(open_repo());
    let one = state_with(GitPanelState {
        recent_commits: vec![GitCommitSummary {
            short_hash: "abc1234".into(),
            summary: "only".into(),
            author: "Ada".into(),
        }],
        ..open_repo()
    });
    assert_eq!(
        GitPanel::for_editor(&empty).unwrap().height(),
        GitPanel::for_editor(&one).unwrap().height(),
    );
}

#[test]
fn hit_test_maps_each_action_region() {
    let s = state_with(open_repo());
    let panel = GitPanel::for_editor(&s).unwrap();
    let rect = panel_rect(&panel);
    let rects = GitPanel::action_rects(rect);
    assert_eq!(panel.hit_test(rect, centre(rects.input)), Some(GitPanelHit::CommitInput));
    assert_eq!(panel.hit_test(rect, centre(rects.commit)), Some(GitPanelHit::Commit));
    assert_eq!(panel.hit_test(rect, centre(rects.refresh)), Some(GitPanelHit::Refresh));
    assert_eq!(panel.hit_test(rect, centre(rects.pull)), Some(GitPanelHit::Pull));
    // Header area → swallowed, not an action.
    assert_eq!(
        panel.hit_test(rect, Point2D::new(20.0, 8.0)),
        Some(GitPanelHit::Inside)
    );
    // Outside the panel entirely → None.
    assert_eq!(
        panel.hit_test(rect, Point2D::new(GIT_PANEL_WIDTH + 50.0, 8.0)),
        None
    );
}

#[test]
fn branch_rows_switch_to_non_current_branch() {
    let s = state_with(GitPanelState {
        branch: Some("main".to_string()),
        branches: vec!["feature".to_string(), "main".to_string()],
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&s).unwrap();
    let rect = panel_rect(&panel);
    let (_, rows) = panel.branch_layout(rect);
    assert_eq!(rows.len(), 2);
    // Row 0 is `feature` (not current) → a switch.
    assert_eq!(
        panel.hit_test(rect, centre(rows[0])),
        Some(GitPanelHit::SwitchBranch(0))
    );
    // Row 1 is `main` (the current branch) → a no-op click.
    assert_eq!(panel.hit_test(rect, centre(rows[1])), Some(GitPanelHit::Inside));
}

#[test]
fn branch_row_merge_button_dispatches_a_merge() {
    let s = state_with(GitPanelState {
        branch: Some("main".to_string()),
        branches: vec!["feature".to_string(), "main".to_string()],
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&s).unwrap();
    let rect = panel_rect(&panel);
    let (_, rows) = panel.branch_layout(rect);
    // Row 0 = `feature` (not current). Its right-edge button merges;
    // the row body switches.
    let merge_btn = GitPanel::branch_merge_button(rows[0]);
    assert_eq!(
        panel.hit_test(rect, centre(merge_btn)),
        Some(GitPanelHit::MergeBranch(0))
    );
    // A point on the row's far left (clear of the button) → switch.
    let left_of_row = Point2D::new(rows[0].origin.x + 8.0, centre(rows[0]).y);
    assert_eq!(
        panel.hit_test(rect, left_of_row),
        Some(GitPanelHit::SwitchBranch(0))
    );
    // The current branch's row has no merge button — its whole row
    // (button area included) is a no-op.
    let current_btn = GitPanel::branch_merge_button(rows[1]);
    assert_eq!(panel.hit_test(rect, centre(current_btn)), Some(GitPanelHit::Inside));
}

#[test]
fn merge_mode_remaps_the_action_buttons() {
    // Conflicts still present — Complete is disabled, so its slot
    // dispatches nothing (a swallowed `Inside`).
    let blocked = state_with(GitPanelState {
        merging: true,
        conflicted_files: vec!["doc.op".to_string()],
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&blocked).unwrap();
    let rect = panel_rect(&panel);
    let rects = GitPanel::action_rects(rect);
    // Slot 0 = Abort, slot 1 = Refresh; the commit input is inert.
    assert_eq!(panel.hit_test(rect, centre(rects.commit)), Some(GitPanelHit::AbortMerge));
    assert_eq!(panel.hit_test(rect, centre(rects.refresh)), Some(GitPanelHit::Refresh));
    assert_eq!(panel.hit_test(rect, centre(rects.input)), Some(GitPanelHit::Inside));
    // Complete slot — inert while conflicts remain.
    assert_eq!(panel.hit_test(rect, centre(rects.pull)), Some(GitPanelHit::Inside));

    // Conflicts resolved — Complete becomes actionable.
    let ready = state_with(GitPanelState {
        merging: true,
        conflicted_files: vec![],
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&ready).unwrap();
    let rect = panel_rect(&panel);
    let rects = GitPanel::action_rects(rect);
    assert_eq!(panel.hit_test(rect, centre(rects.pull)), Some(GitPanelHit::CompleteMerge));
}

#[test]
fn non_repo_panel_has_no_action_targets() {
    let s = state_with(GitPanelState {
        open: true,
        in_repo: false,
        ..GitPanelState::default()
    });
    let panel = GitPanel::for_editor(&s).unwrap();
    let rect = panel_rect(&panel);
    // Any in-bounds click is just swallowed.
    assert_eq!(
        panel.hit_test(rect, Point2D::new(40.0, 40.0)),
        Some(GitPanelHit::Inside)
    );
}

#[test]
fn truncate_caps_long_summaries() {
    assert_eq!(truncate("short", 38), "short");
    let long = "x".repeat(50);
    let t = truncate(&long, 38);
    assert_eq!(t.chars().count(), 38);
    assert!(t.ends_with('…'));
}

// --- Diff view ----------------------------------------------------

#[test]
fn dirty_status_line_opens_the_working_diff() {
    // A clean tree → the status line is inert.
    let clean = state_with(open_repo());
    let panel = GitPanel::for_editor(&clean).unwrap();
    let rect = panel_rect(&panel);
    let clean_hit = panel.hit_test(rect, centre(panel.status_rect(rect)));
    assert_eq!(clean_hit, Some(GitPanelHit::Inside));

    // A dirty tree → clicking the status line opens the diff.
    let dirty = state_with(GitPanelState {
        dirty_count: 3,
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&dirty).unwrap();
    let rect = panel_rect(&panel);
    assert_eq!(
        panel.hit_test(rect, centre(panel.status_rect(rect))),
        Some(GitPanelHit::ShowWorkingDiff)
    );
}

#[test]
fn commit_rows_open_a_commit_diff() {
    let s = state_with(GitPanelState {
        recent_commits: vec![
            GitCommitSummary {
                short_hash: "aaa1111".into(),
                summary: "first".into(),
                author: "Ada".into(),
            },
            GitCommitSummary {
                short_hash: "bbb2222".into(),
                summary: "second".into(),
                author: "Bo".into(),
            },
        ],
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&s).unwrap();
    let rect = panel_rect(&panel);
    let rows = panel.list_row_rects(rect);
    assert_eq!(rows.len(), 2);
    assert_eq!(
        panel.hit_test(rect, centre(rows[0])),
        Some(GitPanelHit::ShowCommitDiff(0))
    );
    assert_eq!(
        panel.hit_test(rect, centre(rows[1])),
        Some(GitPanelHit::ShowCommitDiff(1))
    );
}

#[test]
fn conflict_rows_open_a_file_diff_in_merge_mode() {
    let s = state_with(GitPanelState {
        merging: true,
        conflicted_files: vec!["a.op".into(), "b.op".into()],
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&s).unwrap();
    let rect = panel_rect(&panel);
    let rows = panel.list_row_rects(rect);
    assert_eq!(rows.len(), 2);
    assert_eq!(
        panel.hit_test(rect, centre(rows[1])),
        Some(GitPanelHit::ShowFileDiff(1))
    );
}

#[test]
fn diff_mode_widens_the_panel_and_fixes_its_height() {
    let normal = state_with(open_repo());
    let normal_w = GitPanel::for_editor(&normal).unwrap().panel_width();
    assert_eq!(normal_w, GIT_PANEL_WIDTH);

    let diffing = state_with(GitPanelState {
        diff: Some(GitDiffView {
            title: "Working tree".into(),
            lines: vec!["+a".into(), "-b".into()],
            scroll: 0,
        }),
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&diffing).unwrap();
    assert_eq!(panel.panel_width(), GIT_DIFF_PANEL_WIDTH);
    // Diff mode is a tall fixed-height view.
    assert!(panel.height() > 400.0);
}

#[test]
fn diff_header_buttons_map_to_scroll_and_close() {
    let diffing = state_with(GitPanelState {
        diff: Some(GitDiffView {
            title: "Working tree".into(),
            lines: (0..200).map(|i| format!("+line {i}")).collect(),
            scroll: 0,
        }),
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&diffing).unwrap();
    let rect = panel_rect(&panel);
    let [up, down, close] = GitPanel::diff_header_buttons(rect);
    assert_eq!(panel.hit_test(rect, centre(up)), Some(GitPanelHit::DiffScrollUp));
    assert_eq!(panel.hit_test(rect, centre(down)), Some(GitPanelHit::DiffScrollDown));
    assert_eq!(panel.hit_test(rect, centre(close)), Some(GitPanelHit::CloseDiff));
    // The diff body itself swallows clicks.
    assert_eq!(
        panel.hit_test(rect, Point2D::new(40.0, 200.0)),
        Some(GitPanelHit::Inside)
    );
}

#[test]
fn diff_scroll_metrics_clamp_to_the_line_count() {
    // A short diff fits in one page → nothing to scroll.
    let short = state_with(GitPanelState {
        diff: Some(GitDiffView {
            title: "t".into(),
            lines: vec!["+a".into(), "+b".into()],
            scroll: 0,
        }),
        ..open_repo()
    });
    assert_eq!(GitPanel::for_editor(&short).unwrap().diff_max_scroll(), 0);

    // A long diff → a positive max scroll, and a non-zero page step.
    let long = state_with(GitPanelState {
        diff: Some(GitDiffView {
            title: "t".into(),
            lines: (0..500).map(|i| format!("+{i}")).collect(),
            scroll: 0,
        }),
        ..open_repo()
    });
    assert!(GitPanel::for_editor(&long).unwrap().diff_max_scroll() > 0);
    assert!(GitPanel::diff_page_step() >= 1);
}
