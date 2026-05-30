//! Tests for `widgets::git_panel` — moved to a sibling file to keep
//! `git_panel.rs` under the 800-line cap.

use crate::widgets::git_panel::*;
use crate::{Point2D, Rect};
use op_editor_core::{
    EditorState, GitCommitSummary, GitDiffView, GitFileEntry, GitOverflowView, GitPanelState,
    MergeConflictRow, MergeResolveFile, MergeResolveState,
};

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

/// A bound repo with a dirty working tree — leaves the TS ready view
/// (which only shows for a clean tree) and exercises the classic
/// status body: branch rows, the Commit/Refresh/Pull/Push row, the
/// working-tree status line, and the Remotes section.
fn dirty_repo() -> GitPanelState {
    GitPanelState {
        changed_files: vec![GitFileEntry {
            path: "dirty.op".into(),
            staged: false,
            status: 'M',
        }],
        dirty_count: 1,
        ..open_repo()
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
    // A dirty tree keeps the classic status body (the TS ready view
    // only paints for a clean tree); that is where the 4-button row
    // + commit input live.
    let s = state_with(dirty_repo());
    let panel = GitPanel::for_editor(&s).unwrap();
    let rect = panel_rect(&panel);
    let rects = GitPanel::action_rects(rect, false);
    // Normal mode: Commit / Refresh / Pull / Push + the commit input.
    assert_eq!(rects.buttons.len(), 4);
    assert_eq!(
        panel.hit_test(rect, centre(rects.input)),
        Some(GitPanelHit::CommitInput)
    );
    assert_eq!(
        panel.hit_test(rect, centre(rects.buttons[0])),
        Some(GitPanelHit::Commit)
    );
    assert_eq!(
        panel.hit_test(rect, centre(rects.buttons[1])),
        Some(GitPanelHit::Refresh)
    );
    assert_eq!(
        panel.hit_test(rect, centre(rects.buttons[2])),
        Some(GitPanelHit::Pull)
    );
    assert_eq!(
        panel.hit_test(rect, centre(rects.buttons[3])),
        Some(GitPanelHit::Push)
    );
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
        ..dirty_repo()
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
    assert_eq!(
        panel.hit_test(rect, centre(rows[1])),
        Some(GitPanelHit::Inside)
    );
}

#[test]
fn branch_row_merge_button_dispatches_a_merge() {
    let s = state_with(GitPanelState {
        branch: Some("main".to_string()),
        branches: vec!["feature".to_string(), "main".to_string()],
        ..dirty_repo()
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
    assert_eq!(
        panel.hit_test(rect, centre(current_btn)),
        Some(GitPanelHit::Inside)
    );
}

#[test]
fn changed_file_rows_toggle_staging() {
    let s = state_with(GitPanelState {
        changed_files: vec![
            GitFileEntry {
                path: "a.op".into(),
                staged: false,
                status: 'M',
            },
            GitFileEntry {
                path: "b.op".into(),
                staged: true,
                status: 'A',
            },
        ],
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&s).unwrap();
    let rect = panel_rect(&panel);
    let rows = panel.list_row_rects(rect);
    // A dirty tree → the list slot shows the staging list. The
    // left-edge checkbox toggles staging; the row body opens the
    // file's diff (where hunks can be staged).
    assert_eq!(rows.len(), 2);
    let checkbox = |r: Rect| Point2D::new(r.origin.x + 10.0, centre(r).y);
    assert_eq!(
        panel.hit_test(rect, checkbox(rows[0])),
        Some(GitPanelHit::ToggleStageFile(0))
    );
    assert_eq!(
        panel.hit_test(rect, centre(rows[1])),
        Some(GitPanelHit::ShowChangedFileDiff(1))
    );
}

#[test]
fn remotes_section_maps_the_input_and_set_button() {
    let s = state_with(GitPanelState {
        remotes: vec!["origin → git@host:org/repo.git".into()],
        ..dirty_repo()
    });
    let panel = GitPanel::for_editor(&s).unwrap();
    let rect = panel_rect(&panel);
    let layout = panel.remotes_layout(rect);
    assert_eq!(
        panel.hit_test(rect, centre(layout.input)),
        Some(GitPanelHit::RemoteInput)
    );
    assert_eq!(
        panel.hit_test(rect, centre(layout.set_button)),
        Some(GitPanelHit::SetRemote)
    );
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
    let rects = GitPanel::action_rects(rect, true);
    // Merge mode: 3 buttons — Abort / Refresh / Complete.
    assert_eq!(rects.buttons.len(), 3);
    assert_eq!(
        panel.hit_test(rect, centre(rects.buttons[0])),
        Some(GitPanelHit::AbortMerge)
    );
    assert_eq!(
        panel.hit_test(rect, centre(rects.buttons[1])),
        Some(GitPanelHit::Refresh)
    );
    assert_eq!(
        panel.hit_test(rect, centre(rects.input)),
        Some(GitPanelHit::Inside)
    );
    // Complete slot — inert while conflicts remain.
    assert_eq!(
        panel.hit_test(rect, centre(rects.buttons[2])),
        Some(GitPanelHit::Inside)
    );

    // Conflicts resolved — Complete becomes actionable.
    let ready = state_with(GitPanelState {
        merging: true,
        conflicted_files: vec![],
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&ready).unwrap();
    let rect = panel_rect(&panel);
    let rects = GitPanel::action_rects(rect, true);
    assert_eq!(
        panel.hit_test(rect, centre(rects.buttons[2])),
        Some(GitPanelHit::CompleteMerge)
    );
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
    // A clean tree → the TS ready view, which has no working-tree
    // status line at all, so no click anywhere yields a working diff.
    let clean = state_with(open_repo());
    let panel = GitPanel::for_editor(&clean).unwrap();
    let rect = panel_rect(&panel);
    let clean_hit = panel.hit_test(rect, centre(panel.status_rect(rect)));
    assert_ne!(clean_hit, Some(GitPanelHit::ShowWorkingDiff));

    // A dirty tree → the classic body's status line opens the diff.
    let dirty = state_with(GitPanelState {
        dirty_count: 3,
        ..dirty_repo()
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
    // A clean tree shows the TS ready view; its history rows map to
    // a commit's diff.
    let rows = panel.ready_commit_row_rects(rect);
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
fn ready_view_maps_each_header_and_commit_region() {
    // A clean bound repo → the TS ready layout. Its header exposes
    // the branch picker + pull/push + overflow; the commit box is a
    // focus target and its button commits a non-empty message.
    let s = state_with(GitPanelState {
        branch: Some("main".to_string()),
        commit_message: "ship it".to_string(),
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&s).unwrap();
    let rect = panel_rect(&panel);
    let (pull, push, overflow) = panel.ready_header_buttons(rect);
    assert_eq!(
        panel.hit_test(rect, centre(panel.ready_branch_rect(rect))),
        Some(GitPanelHit::BranchPicker)
    );
    assert_eq!(panel.hit_test(rect, centre(pull)), Some(GitPanelHit::Pull));
    assert_eq!(panel.hit_test(rect, centre(push)), Some(GitPanelHit::Push));
    assert_eq!(
        panel.hit_test(rect, centre(overflow)),
        Some(GitPanelHit::Overflow)
    );
    // With a non-empty message the Save-milestone button fires (it
    // saves the live design + commits, so no pre-staged file needed).
    assert_eq!(
        panel.hit_test(rect, centre(panel.ready_commit_btn(rect))),
        Some(GitPanelHit::CommitMilestone)
    );
    // The box body away from the button focuses the input.
    let box_r = panel.ready_commit_box(rect);
    let top_left = Point2D::new(box_r.origin.x + 6.0, box_r.origin.y + 6.0);
    assert_eq!(
        panel.hit_test(rect, top_left),
        Some(GitPanelHit::CommitInput)
    );
}

#[test]
fn ready_commit_button_is_inert_without_a_message() {
    // An empty commit message → the button is not a commit target;
    // the click falls through to the box's focus instead.
    let s = state_with(GitPanelState {
        branch: Some("main".to_string()),
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&s).unwrap();
    let rect = panel_rect(&panel);
    assert_eq!(
        panel.hit_test(rect, centre(panel.ready_commit_btn(rect))),
        Some(GitPanelHit::CommitInput)
    );
}

#[test]
fn branch_picker_dropdown_switches_and_dismisses() {
    let s = state_with(GitPanelState {
        branch: Some("main".to_string()),
        branches: vec!["feature".to_string(), "main".to_string()],
        branch_picker_open: true,
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&s).unwrap();
    let rect = panel_rect(&panel);
    let rows = panel.branch_picker_row_rects(rect);
    assert_eq!(rows.len(), 2);
    // Row 0 = feature (not current) → switch; row 1 = main (current) → no-op.
    assert_eq!(
        panel.hit_test(rect, centre(rows[0])),
        Some(GitPanelHit::SwitchBranch(0))
    );
    assert_eq!(
        panel.hit_test(rect, centre(rows[1])),
        Some(GitPanelHit::Inside)
    );
    // A click outside the dropdown (but inside the panel) dismisses it.
    let outside = Point2D::new(rect.origin.x + rect.size.x / 2.0, rect.origin.y + 8.0);
    assert_eq!(
        panel.hit_test(rect, outside),
        Some(GitPanelHit::DismissPopover)
    );
    // An open popover is modal: a click FAR OUTSIDE the panel (e.g. on
    // the canvas) also dismisses it rather than returning None (which
    // would leave the popover stuck open).
    let far = Point2D::new(rect.origin.x - 200.0, rect.origin.y + 400.0);
    assert_eq!(panel.hit_test(rect, far), Some(GitPanelHit::DismissPopover));
}

#[test]
fn ready_long_branch_never_eats_the_overflow_button() {
    // A long branch name must not push the pull/push cluster over the
    // right-anchored `…` overflow button (the branch rect is clamped).
    let s = state_with(GitPanelState {
        branch: Some("feature/a-very-long-branch-name-indeed".to_string()),
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&s).unwrap();
    let rect = panel_rect(&panel);
    let (_, _, overflow) = panel.ready_header_buttons(rect);
    assert_eq!(
        panel.hit_test(rect, centre(overflow)),
        Some(GitPanelHit::Overflow)
    );
    // The branch button must not overlap the pull button either.
    let branch = panel.ready_branch_rect(rect);
    let (pull, _, _) = panel.ready_header_buttons(rect);
    assert!(
        branch.origin.x + branch.size.x <= pull.origin.x,
        "branch button overruns the pull icon"
    );
}

#[test]
fn overflow_menu_maps_its_entries() {
    let s = state_with(GitPanelState {
        branch: Some("main".to_string()),
        overflow_open: true,
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&s).unwrap();
    let rect = panel_rect(&panel);
    let rows = panel.overflow_row_rects(rect);
    assert_eq!(rows.len(), 2);
    assert_eq!(
        panel.hit_test(rect, centre(rows[0])),
        Some(GitPanelHit::OverflowRemoteSettings)
    );
    assert_eq!(
        panel.hit_test(rect, centre(rows[1])),
        Some(GitPanelHit::OverflowSshKeys)
    );
}

#[test]
fn overflow_remote_settings_subview_maps_inputs_and_back() {
    let s = state_with(GitPanelState {
        branch: Some("main".to_string()),
        overflow_open: true,
        overflow_view: GitOverflowView::RemoteSettings,
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&s).unwrap();
    let rect = panel_rect(&panel);
    let (back, url, set, https, login) = panel.remote_settings_rects(rect);
    assert_eq!(
        panel.hit_test(rect, centre(back)),
        Some(GitPanelHit::OverflowBack)
    );
    assert_eq!(
        panel.hit_test(rect, centre(url)),
        Some(GitPanelHit::RemoteInput)
    );
    assert_eq!(
        panel.hit_test(rect, centre(set)),
        Some(GitPanelHit::SetRemote)
    );
    assert_eq!(
        panel.hit_test(rect, centre(https)),
        Some(GitPanelHit::HttpsInput)
    );
    assert_eq!(
        panel.hit_test(rect, centre(login)),
        Some(GitPanelHit::SetHttpsAuth)
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
fn merge_resolution_view_maps_choices_and_actions() {
    let conflict = |id: &str| MergeConflictRow {
        id: id.into(),
        label: format!("Node {id}"),
        kind: "both modified".into(),
        theirs_allowed: true,
        take_theirs: false,
    };
    let s = state_with(GitPanelState {
        merge_resolve: Some(MergeResolveState {
            branch: "feature".into(),
            files: vec![MergeResolveFile {
                path: "doc.op".into(),
                base: "{}".into(),
                ours: "{}".into(),
                theirs: "{}".into(),
                conflicts: vec![conflict("n1"), conflict("n2")],
            }],
        }),
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&s).unwrap();
    let rect = panel_rect(&panel);
    let layout = panel.resolve_layout(rect);
    assert_eq!(layout.rows.len(), 2);
    let (ours0, theirs0) = layout.rows[0];
    assert_eq!(
        panel.hit_test(rect, centre(ours0)),
        Some(GitPanelHit::MergeChoiceOurs(0))
    );
    assert_eq!(
        panel.hit_test(rect, centre(theirs0)),
        Some(GitPanelHit::MergeChoiceTheirs(0))
    );
    let (_, theirs1) = layout.rows[1];
    assert_eq!(
        panel.hit_test(rect, centre(theirs1)),
        Some(GitPanelHit::MergeChoiceTheirs(1))
    );
    assert_eq!(
        panel.hit_test(rect, centre(layout.apply)),
        Some(GitPanelHit::ApplyMergeResolution)
    );
    assert_eq!(
        panel.hit_test(rect, centre(layout.cancel)),
        Some(GitPanelHit::CancelMergeResolution)
    );
}

#[test]
fn merge_resolve_set_choice_clamps_structural_to_ours() {
    let mut state = MergeResolveState {
        branch: "feature".into(),
        files: vec![MergeResolveFile {
            path: "doc.op".into(),
            base: "{}".into(),
            ours: "{}".into(),
            theirs: "{}".into(),
            conflicts: vec![
                MergeConflictRow {
                    id: "n1".into(),
                    label: "Node n1".into(),
                    kind: "both modified".into(),
                    theirs_allowed: true,
                    take_theirs: false,
                },
                MergeConflictRow {
                    id: "n2".into(),
                    label: "Node n2".into(),
                    kind: "added on remote".into(),
                    theirs_allowed: false,
                    take_theirs: false,
                },
            ],
        }],
    };
    // A prop conflict honours "theirs".
    state.set_choice(0, true);
    assert!(state.rows()[0].take_theirs);
    // A structural conflict clamps a "theirs" choice back to "ours".
    state.set_choice(1, true);
    assert!(!state.rows()[1].take_theirs);
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
            h_scroll: 0,
            stage_path: None,
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
            h_scroll: 0,
            stage_path: None,
        }),
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&diffing).unwrap();
    let rect = panel_rect(&panel);
    let [left, right, up, down, close] = GitPanel::diff_header_buttons(rect);
    assert_eq!(
        panel.hit_test(rect, centre(left)),
        Some(GitPanelHit::DiffScrollLeft)
    );
    assert_eq!(
        panel.hit_test(rect, centre(right)),
        Some(GitPanelHit::DiffScrollRight)
    );
    assert_eq!(
        panel.hit_test(rect, centre(up)),
        Some(GitPanelHit::DiffScrollUp)
    );
    assert_eq!(
        panel.hit_test(rect, centre(down)),
        Some(GitPanelHit::DiffScrollDown)
    );
    assert_eq!(
        panel.hit_test(rect, centre(close)),
        Some(GitPanelHit::CloseDiff)
    );
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
            h_scroll: 0,
            stage_path: None,
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
            h_scroll: 0,
            stage_path: None,
        }),
        ..open_repo()
    });
    assert!(GitPanel::for_editor(&long).unwrap().diff_max_scroll() > 0);
    assert!(GitPanel::diff_page_step() >= 1);
}

#[test]
fn empty_state_cards_map_to_actions() {
    // No repo + saved doc → Init enabled, all three cards act.
    let saved = state_with(GitPanelState {
        open: true,
        in_repo: false,
        has_saved_file: true,
        ..GitPanelState::default()
    });
    let panel = GitPanel::for_editor(&saved).unwrap();
    let rect = panel_rect(&panel);
    let cards = panel.empty_state_rects(rect);
    assert_eq!(
        panel.hit_test(rect, centre(cards[0])),
        Some(GitPanelHit::EmptyInit)
    );
    assert_eq!(
        panel.hit_test(rect, centre(cards[1])),
        Some(GitPanelHit::EmptyOpen)
    );
    assert_eq!(
        panel.hit_test(rect, centre(cards[2])),
        Some(GitPanelHit::EmptyClone)
    );

    // Unsaved doc → Init card is inert (swallowed), Open still acts.
    let unsaved = state_with(GitPanelState {
        open: true,
        in_repo: false,
        has_saved_file: false,
        ..GitPanelState::default()
    });
    let panel = GitPanel::for_editor(&unsaved).unwrap();
    let rect = panel_rect(&panel);
    let cards = panel.empty_state_rects(rect);
    assert_eq!(
        panel.hit_test(rect, centre(cards[0])),
        Some(GitPanelHit::Inside)
    );
    assert_eq!(
        panel.hit_test(rect, centre(cards[1])),
        Some(GitPanelHit::EmptyOpen)
    );
}
