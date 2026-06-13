use crate::widgets::git_panel::GitPanel;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use jian_widgets::components::select::{SelectHit, SelectState};
use op_editor_core::{EditorState, GitCandidateFile, GitOverflowView, GitPanelState};

#[derive(Default)]
struct RoundFillBackend {
    fills: Vec<(Rect, f32, Color)>,
}

impl RenderBackend for RoundFillBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        self.fills.push((rect, radius, color));
    }
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

fn color_close(a: Color, b: Color) -> bool {
    (a.r - b.r).abs() < 1e-6
        && (a.g - b.g).abs() < 1e-6
        && (a.b - b.b).abs() < 1e-6
        && (a.a - b.a).abs() < 1e-6
}

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

#[test]
fn pressed_tracked_picker_row_uses_shared_select_feedback() {
    let mut select = SelectState::default();
    select.pressed = Some(1);
    let state = state_with(GitPanelState {
        overflow_open: true,
        overflow_view: GitOverflowView::TrackedPicker,
        candidate_files: vec![candidate("a.op"), candidate("b.op")],
        tracked_picker: select,
        ..open_repo()
    });
    let panel = GitPanel::for_editor(&state).unwrap();
    let rect = panel_rect(&panel);
    let rows = panel.tracked_picker_row_rects(rect);
    let theme = crate::widgets::editor_state_ext::theme_for(&state.editor_ui);
    let expected = theme.button_hover.with_alpha(theme.button_hover.a * 1.8);
    let mut backend = RoundFillBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint_tracked_picker(&mut cx, rect);

    assert!(
        backend.fills.iter().any(|(fill, radius, color)| {
            *fill == rows[1] && (*radius - 8.0).abs() < 0.01 && color_close(*color, expected)
        }),
        "pressed tracked-picker row should paint the shared pressed feedback token"
    );
}
