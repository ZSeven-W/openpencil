//! Tests for `widgets::git_panel` — moved to a sibling file to keep
//! `git_panel.rs` under the 800-line cap.

use crate::widgets::git_panel::*;
use crate::{Point2D, Rect};
use op_editor_core::{
    CloneField, CloneFormState, EditorState, GitBranchPickerMode, GitCandidateFile,
    GitCommitSummary, GitDiffView, GitFileEntry, GitOverflowView, GitPanelState, MergeConflictRow,
    MergeResolveFile, MergeResolveState,
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
                time_label: "now".into(),
                is_initial: false,
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
            time_label: "now".into(),
            is_initial: false,
        }],
        ..open_repo()
    });
    assert_eq!(
        GitPanel::for_editor(&empty).unwrap().height(),
        GitPanel::for_editor(&one).unwrap().height(),
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
fn commit_rows_open_a_commit_diff() {
    let s = state_with(GitPanelState {
        recent_commits: vec![
            GitCommitSummary {
                short_hash: "aaa1111".into(),
                summary: "first".into(),
                author: "Ada".into(),
                time_label: "now".into(),
                is_initial: false,
            },
            GitCommitSummary {
                short_hash: "bbb2222".into(),
                summary: "second".into(),
                author: "Bo".into(),
                time_label: "now".into(),
                is_initial: false,
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
fn expanded_commit_card_maps_restore_and_copy_and_shifts_later_rows() {
    // Row 0 expanded → its inline detail card (里程碑详情) sits between
    // rows 0 and 1, exposing 恢复 / 复制哈希 buttons and pushing row 1
    // down by the card height.
    let commits = vec![
        GitCommitSummary {
            short_hash: "aaa1111".into(),
            summary: "first".into(),
            author: "Ada".into(),
            time_label: "now".into(),
            is_initial: false,
        },
        GitCommitSummary {
            short_hash: "bbb2222".into(),
            summary: "second".into(),
            author: "Bo".into(),
            time_label: "now".into(),
            is_initial: false,
        },
    ];
    let collapsed = state_with(GitPanelState {
        branch: Some("main".to_string()),
        recent_commits: commits.clone(),
        ..open_repo()
    });
    let cp = GitPanel::for_editor(&collapsed).unwrap();
    let crect = panel_rect(&cp);
    let row1_collapsed = cp.ready_commit_row_rects(crect)[1].origin.y;
    // Same state, but row 0 expanded.
    let expanded = state_with(GitPanelState {
        branch: Some("main".to_string()),
        recent_commits: commits,
        expanded_commit: Some(0),
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&expanded).unwrap();
    let rect = panel_rect(&panel);
    // Card buttons exist and map to the expanded row's index.
    let (restore, copy) = panel.ready_commit_card_buttons(rect).unwrap();
    assert_eq!(
        panel.hit_test(rect, centre(restore)),
        Some(GitPanelHit::RestoreCommit(0))
    );
    assert_eq!(
        panel.hit_test(rect, centre(copy)),
        Some(GitPanelHit::CopyCommitHash(0))
    );
    // Row 1 shifted down by exactly the card height; the panel grew too.
    // (No diff loaded in this state → base card height, no patch rows.)
    let row1_expanded = panel.ready_commit_row_rects(rect)[1].origin.y;
    assert!((row1_expanded - row1_collapsed - 104.0).abs() < 0.5);
    assert!(panel.height() > cp.height());
    // The expanded card sits below row 0's click target.
    let row0 = panel.ready_commit_row_rects(rect)[0];
    assert!(restore.origin.y > row0.origin.y);
}

#[test]
fn ready_view_maps_each_header_and_commit_region() {
    // A clean bound repo → the TS ready layout. Its header exposes
    // the branch picker + pull/push + overflow; the commit box is a
    // focus target and its button commits a non-empty message.
    let mut state = open_repo();
    state.commit_input.set_text("ship it");
    let s = state_with(GitPanelState {
        branch: Some("main".to_string()),
        // A remote + commits-ahead so pull/push are enabled (they now
        // disable when there's no remote / nothing to push, TS parity).
        remotes: vec!["origin → https://example.com/r.git".to_string()],
        ahead: 1,
        ..state
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
fn branch_picker_submodes_map_create_input_and_cancel() {
    let mut create_state = open_repo();
    create_state.branch_create_input.set_text("feature/new");
    let s = state_with(GitPanelState {
        branch: Some("main".to_string()),
        branches: vec!["feature".to_string(), "main".to_string()],
        branch_picker_open: true,
        branch_picker_mode: GitBranchPickerMode::Create,
        ..create_state
    });
    let panel = GitPanel::for_editor(&s).unwrap();
    let rect = panel_rect(&panel);
    let picker = panel.branch_picker_panel(rect);
    let input_point = Point2D::new(picker.origin.x + 16.0, picker.origin.y + 34.0);
    assert_eq!(
        panel.hit_test(rect, input_point),
        Some(GitPanelHit::BranchCreateInput)
    );
    let submit = Rect {
        origin: Point2D::new(
            picker.origin.x + picker.size.x - 18.0 - 64.0,
            picker.origin.y + 54.0,
        ),
        size: Point2D::new(64.0, 24.0),
    };
    assert_eq!(
        panel.hit_test(rect, centre(submit)),
        Some(GitPanelHit::BranchCreateSubmit)
    );

    let s = state_with(GitPanelState {
        branch: Some("main".to_string()),
        branches: vec!["feature".to_string(), "main".to_string()],
        branch_picker_open: true,
        branch_picker_mode: GitBranchPickerMode::Merge,
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&s).unwrap();
    let rect = panel_rect(&panel);
    let picker = panel.branch_picker_panel(rect);
    let cancel_point = Point2D::new(
        picker.origin.x + picker.size.x / 2.0,
        picker.origin.y + picker.size.y - 12.0,
    );
    assert_eq!(
        panel.hit_test(rect, cancel_point),
        Some(GitPanelHit::BranchPickerCancel)
    );
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
    // TS 5-item menu: switch-tracked / clear-author / remote-settings /
    // ssh-keys / close-repo (with two dividers between groups).
    assert_eq!(rows.len(), 5);
    assert_eq!(
        panel.hit_test(rect, centre(rows[0])),
        Some(GitPanelHit::OverflowSwitchTracked)
    );
    assert_eq!(
        panel.hit_test(rect, centre(rows[1])),
        Some(GitPanelHit::OverflowClearAuthor)
    );
    assert_eq!(
        panel.hit_test(rect, centre(rows[2])),
        Some(GitPanelHit::OverflowRemoteSettings)
    );
    assert_eq!(
        panel.hit_test(rect, centre(rows[3])),
        Some(GitPanelHit::OverflowSshKeys)
    );
    assert_eq!(
        panel.hit_test(rect, centre(rows[4])),
        Some(GitPanelHit::OverflowCloseRepo)
    );
}

#[test]
fn git_menus_use_shared_menu_state_protocol() {
    use jian_widgets::components::menu::{MenuHit, MenuState};

    let s = state_with(GitPanelState {
        branch: Some("main".to_string()),
        overflow_open: true,
        overflow_menu: MenuState { hover: Some(2) },
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&s).unwrap();
    let rect = panel_rect(&panel);
    let rows = panel.overflow_row_rects(rect);
    assert_eq!(panel.state.overflow_menu.hover, Some(2));
    assert_eq!(
        panel.overflow_menu_hit(rect, centre(rows[2])),
        MenuHit::Row(2)
    );
    assert_eq!(
        panel.overflow_menu_hit(rect, Point2D::new(rows[2].origin.x, rows[2].origin.y - 4.0)),
        MenuHit::Inside
    );

    let s = state_with(GitPanelState {
        branch: Some("main".to_string()),
        branches: vec!["main".to_string(), "feature".to_string()],
        branch_picker_open: true,
        branch_picker_menu: MenuState { hover: Some(1) },
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&s).unwrap();
    let rect = panel_rect(&panel);
    let rows = panel.branch_picker_row_rects(rect);
    assert_eq!(panel.state.branch_picker_menu.hover, Some(1));
    assert_eq!(
        panel.branch_picker_menu_hit(rect, centre(rows[1])),
        MenuHit::Row(1)
    );
    assert_eq!(
        panel.branch_picker_menu_hit(rect, Point2D::new(rows[0].origin.x, rows[0].origin.y - 4.0)),
        MenuHit::Inside
    );
}

#[test]
fn tracked_picker_maps_rows_and_actions() {
    let s = state_with(GitPanelState {
        branch: Some("main".to_string()),
        overflow_open: true,
        overflow_view: GitOverflowView::TrackedPicker,
        candidate_files: vec![
            GitCandidateFile {
                path: "/r/a.op".into(),
                relative_path: "a.op".into(),
                milestone_count: 2,
                last_commit_time: "1h".into(),
                last_commit_message: Some("hi".into()),
            },
            GitCandidateFile {
                path: "/r/b.op".into(),
                relative_path: "b.op".into(),
                milestone_count: 0,
                last_commit_time: String::new(),
                last_commit_message: None,
            },
        ],
        tracked_picker_selected: Some(0),
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&s).unwrap();
    let rect = panel_rect(&panel);
    let rows = panel.tracked_picker_row_rects(rect);
    assert_eq!(rows.len(), 2);
    assert_eq!(
        panel.hit_test(rect, centre(rows[1])),
        Some(GitPanelHit::TrackedPickerRow(1))
    );
    // With a selection, both bind buttons are live; Back always is.
    let (back, bind, open) = panel.tracked_picker_footer_rects(rect);
    assert_eq!(
        panel.hit_test(rect, centre(back)),
        Some(GitPanelHit::TrackedPickerBack)
    );
    assert_eq!(
        panel.hit_test(rect, centre(bind)),
        Some(GitPanelHit::TrackedPickerBind)
    );
    assert_eq!(
        panel.hit_test(rect, centre(open)),
        Some(GitPanelHit::TrackedPickerBindOpen)
    );
}

#[test]
fn overflow_remote_settings_subview_maps_inputs_and_back() {
    let s = state_with(GitPanelState {
        branch: Some("main".to_string()),
        overflow_open: true,
        overflow_view: GitOverflowView::RemoteSettings,
        remotes: vec!["origin → https://example.com/r.git".to_string()],
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&s).unwrap();
    let rect = panel_rect(&panel);
    let (back, url, set) = panel.remote_settings_rects(rect);
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
    // The TS remote-settings has no HTTPS-credential input — fetch is the
    // next interactive element (a remote is configured in this state).
    let fetch = panel.remote_settings_fetch_rect(rect);
    assert_eq!(
        panel.hit_test(rect, centre(fetch)),
        Some(GitPanelHit::FetchRemote)
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

#[test]
fn clone_form_takes_over_and_maps_each_target() {
    // With `clone_form` set the panel switches to the clone view; each
    // field / button hit-tests to its own action regardless of repo
    // state (the wizard opens from the no-repo empty state).
    let st = state_with(GitPanelState {
        open: true,
        in_repo: false,
        clone_form: Some(CloneFormState {
            url_input: jian_core::text_input::TextInputState::with_text(
                "https://github.com/owner/repo.git",
            ),
            dest_input: jian_core::text_input::TextInputState::with_text("/tmp/repo"),
            focus: Some(CloneField::Url),
            ..Default::default()
        }),
        ..GitPanelState::default()
    });
    let panel = GitPanel::for_editor(&st).unwrap();
    let rect = panel_rect(&panel);
    // The view sizes to the clone layout (positive, finite height).
    assert!(panel.height() > 0.0);
    let layout = panel.clone_layout(rect);
    assert_eq!(
        panel.hit_test(rect, centre(layout.url_input)),
        Some(GitPanelHit::CloneUrlInput)
    );
    assert_eq!(
        panel.hit_test(rect, centre(layout.dest_input)),
        Some(GitPanelHit::CloneDestInput)
    );
    assert_eq!(
        panel.hit_test(rect, centre(layout.dest_pick)),
        Some(GitPanelHit::CloneDestPick)
    );
    assert_eq!(
        panel.hit_test(rect, centre(layout.submit)),
        Some(GitPanelHit::CloneSubmit)
    );
    assert_eq!(
        panel.hit_test(rect, centre(layout.cancel)),
        Some(GitPanelHit::CloneCancel)
    );
    // The dest input + pick button must not overlap.
    assert!(
        layout.dest_input.origin.x + layout.dest_input.size.x <= layout.dest_pick.origin.x + 0.01,
        "dest input + pick button overlap"
    );
}

#[test]
fn clone_view_locks_to_cancel_only_while_cloning() {
    // Mid-clone the form is locked: only Cancel acts (it abandons the
    // job); the URL / destination / pick / submit controls are greyed
    // and must swallow clicks instead of mutating a running clone.
    let st = state_with(GitPanelState {
        open: true,
        in_repo: false,
        clone_form: Some(CloneFormState {
            url_input: jian_core::text_input::TextInputState::with_text(
                "https://github.com/owner/repo.git",
            ),
            dest_input: jian_core::text_input::TextInputState::with_text("/tmp/repo"),
            cloning: true,
            ..Default::default()
        }),
        ..GitPanelState::default()
    });
    let panel = GitPanel::for_editor(&st).unwrap();
    let rect = panel_rect(&panel);
    let layout = panel.clone_layout(rect);
    assert_eq!(
        panel.hit_test(rect, centre(layout.cancel)),
        Some(GitPanelHit::CloneCancel)
    );
    assert_eq!(
        panel.hit_test(rect, centre(layout.submit)),
        Some(GitPanelHit::Inside)
    );
    assert_eq!(
        panel.hit_test(rect, centre(layout.url_input)),
        Some(GitPanelHit::Inside)
    );
    assert_eq!(
        panel.hit_test(rect, centre(layout.dest_pick)),
        Some(GitPanelHit::Inside)
    );
}

#[test]
fn dirty_bound_repo_still_shows_the_ready_view() {
    // TS parity: a bound, non-merging repo shows the ready view whether
    // the working tree is clean OR dirty (there is no per-file staging
    // view in TS; the commit-milestone flow handles dirty changes).
    let mut state = open_repo();
    state.changed_files = vec![GitFileEntry {
        path: "x.op".into(),
        staged: false,
        status: 'M',
    }];
    state.dirty_count = 1;
    let editor = state_with(state);
    let panel = GitPanel::for_editor(&editor).expect("open repo => panel");
    assert!(
        panel.is_ready_state(),
        "a dirty bound repo must still show the ready view",
    );
}
