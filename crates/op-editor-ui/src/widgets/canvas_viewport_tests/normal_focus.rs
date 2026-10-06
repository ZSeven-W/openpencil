//! Normal single-page painting must isolate the work, including overlapping roots.

use super::*;
use op_editor_core::{HomeFamily, WorkspaceView};

fn fixture(overlap: bool) -> (EditorState, LayoutScene) {
    let mut state = EditorState::new();
    state.doc.children = vec![
        named_frame_node("first", "First page"),
        named_frame_node("second", "Second page"),
    ];
    state
        .editor_ui
        .workspace
        .open_for_reading(HomeFamily::KnowledgeCards, 2);
    state.viewport.zoom = 1.0;
    let mut first = leaf(
        "first",
        NodeKind::Frame,
        Rect::xywh(80.0, 80.0, 320.0, 200.0),
        Some(Color::BLUE),
    );
    first.children.push(leaf(
        "content",
        NodeKind::Rect,
        Rect::xywh(100.0, 100.0, 40.0, 40.0),
        Some(Color::WHITE),
    ));
    let second = leaf(
        "second",
        NodeKind::Frame,
        Rect::xywh(if overlap { 80.0 } else { 450.0 }, 80.0, 320.0, 200.0),
        Some(Color::RED),
    );
    let scene = LayoutScene {
        pages: vec![ScenePage {
            id: "p".into(),
            name: "Page".into(),
            children: vec![first, second],
        }],
        active_page_index: 0,
    };
    (state, scene)
}

fn paint(state: &EditorState, scene: &LayoutScene) -> RecordingBackend {
    let canvas = CanvasViewport::from_editor(state, scene);
    let mut backend = RecordingBackend::default();
    canvas.paint(
        &mut PaintCx {
            backend: &mut backend,
        },
        Rect::xywh(0.0, 0.0, 1000.0, 700.0),
    );
    backend
}

#[test]
fn single_page_excludes_neighbor_fill_and_label_even_when_roots_overlap() {
    for overlap in [false, true] {
        let (mut state, scene) = fixture(overlap);
        state.editor_ui.workspace.view = WorkspaceView::Single { index: 0 };
        let result = paint(&state, &scene);
        assert!(result.fill_colors.contains(&Color::BLUE));
        assert!(
            result.fill_colors.contains(&Color::WHITE),
            "selected subtree is intact"
        );
        assert!(!result.fill_colors.contains(&Color::RED));
        assert!(result.texts.iter().any(|text| text == "First page"));
        assert!(!result.texts.iter().any(|text| text == "Second page"));
    }
}

#[test]
fn paging_and_pan_zoom_do_not_reveal_other_boards_or_mutate_the_work() {
    let (mut state, scene) = fixture(false);
    let original = state.doc.clone();
    for (index, visible, hidden) in [(0, Color::BLUE, Color::RED), (1, Color::RED, Color::BLUE)] {
        state.editor_ui.workspace.view = WorkspaceView::Single { index };
        state.editor_ui.workspace.selected = index;
        for (pan_x, pan_y, zoom) in [(0.0, 0.0, 1.0), (15.0, 35.0, 0.6), (-20.0, -10.0, 1.2)] {
            state.viewport = DocViewport { pan_x, pan_y, zoom };
            let result = paint(&state, &scene);
            assert!(result.fill_colors.contains(&visible));
            assert!(!result.fill_colors.contains(&hidden));
            let label = if index == 0 {
                "First page"
            } else {
                "Second page"
            };
            assert!(result.texts.iter().any(|text| text == label));
        }
    }
    assert_eq!(state.doc, original);
}

#[test]
fn all_overview_professional_and_sdk_viewer_keep_the_full_scene() {
    let (mut state, scene) = fixture(false);
    for view in [WorkspaceView::AllBoards, WorkspaceView::Overview] {
        state.editor_ui.workspace.view = view;
        let result = paint(&state, &scene);
        assert!(result.fill_colors.contains(&Color::BLUE));
        assert!(result.fill_colors.contains(&Color::RED));
    }
    state.editor_ui.workspace.view = WorkspaceView::Single { index: 0 };
    state.editor_ui.workspace.enter_professional();
    let result = paint(&state, &scene);
    assert!(result.fill_colors.contains(&Color::RED));
    let viewer = CanvasViewport::from_scene(&scene, state.viewport, Theme::default());
    let mut result = RecordingBackend::default();
    viewer.paint(
        &mut PaintCx {
            backend: &mut result,
        },
        Rect::xywh(0.0, 0.0, 1000.0, 700.0),
    );
    assert!(result.fill_colors.contains(&Color::BLUE));
    assert!(result.fill_colors.contains(&Color::RED));
}

#[test]
fn touch_reader_still_uses_its_selected_page_in_every_desktop_view() {
    let (mut state, scene) = fixture(true);
    state.editor_ui.touch = true;
    state.editor_ui.size_class = op_editor_core::size_class::EditorSizeClass::Compact;
    state.editor_ui.workspace.selected = 1;
    for view in [
        WorkspaceView::AllBoards,
        WorkspaceView::Single { index: 0 },
        WorkspaceView::LongPage,
    ] {
        state.editor_ui.workspace.view = view;
        let result = paint(&state, &scene);
        assert!(result.fill_colors.contains(&Color::RED));
        assert!(!result.fill_colors.contains(&Color::BLUE));
        assert!(!result
            .texts
            .iter()
            .any(|text| text == "First page" || text == "Second page"));
    }
}

#[test]
fn single_page_label_hits_and_stale_selection_are_limited_to_the_visible_root() {
    let (mut state, scene) = fixture(false);
    let rect = Rect::xywh(0.0, 0.0, 1000.0, 700.0);
    state.editor_ui.workspace.view = WorkspaceView::AllBoards;
    let all = CanvasViewport::from_editor(&state, &scene);
    assert_eq!(
        all.frame_label_at_point(rect, Point2D::new(460.0, 65.0)),
        Some("second".into())
    );
    state.editor_ui.workspace.view = WorkspaceView::Single { index: 0 };
    state.selection.set = vec![
        op_editor_core::NodeId::new("first"),
        op_editor_core::NodeId::new("second"),
    ];
    let canvas = CanvasViewport::from_editor(&state, &scene);
    assert!(canvas
        .frame_label_at_point(rect, Point2D::new(460.0, 65.0))
        .is_none());
    let result = paint(&state, &scene);
    assert!(!result
        .stroke_rects
        .iter()
        .any(|(rect, _)| rect.origin.x == 450.0));
    state.editor_ui.workspace.view = WorkspaceView::Single { index: usize::MAX };
    let result = paint(&state, &scene);
    assert!(
        result.fill_colors.contains(&Color::BLUE),
        "invalid camera indices fall back to the first board"
    );
    assert!(!result.fill_colors.contains(&Color::RED));
}
