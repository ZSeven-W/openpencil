use crate::widgets::git_panel::GitPanel;
use crate::{Point2D, Rect};
use jian_widgets::components::select::SelectHit;
use op_editor_core::{EditorState, GitCandidateFile, GitOverflowView, GitPanelState};

fn state_with(panel: GitPanelState) -> EditorState {
    let mut state = EditorState::new();
    state.editor_ui.git_panel = panel;
    state
}

fn open_repo() -> GitPanelState {
    GitPanelState {
        open: true,
        in_repo: true,
        branch: Some("main".to_string()),
        ..GitPanelState::default()
    }
}

fn panel_rect(panel: &GitPanel<'_>) -> Rect {
    Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(panel.panel_width(), panel.height()),
    }
}

fn candidate(path: &str) -> GitCandidateFile {
    GitCandidateFile {
        path: format!("/repo/{path}"),
        relative_path: path.to_string(),
        milestone_count: 0,
        last_commit_time: String::new(),
        last_commit_message: None,
    }
}

#[test]
fn tracked_picker_rows_use_shared_select_hit_protocol() {
    let state = state_with(GitPanelState {
        overflow_open: true,
        overflow_view: GitOverflowView::TrackedPicker,
        candidate_files: vec![candidate("a.op"), candidate("b.op")],
        tracked_picker_selected: Some(0),
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&state).unwrap();
    let rect = panel_rect(&panel);
    let rows = panel.tracked_picker_row_rects(rect);

    assert_eq!(
        panel.tracked_picker_select_hit(
            rect,
            Point2D::new(rows[1].origin.x + 8.0, rows[1].origin.y + 8.0)
        ),
        SelectHit::Row(1)
    );
    assert_eq!(
        panel.tracked_picker_select_hit(
            rect,
            Point2D::new(rows[0].origin.x - 1.0, rows[0].origin.y)
        ),
        SelectHit::Outside
    );
}
