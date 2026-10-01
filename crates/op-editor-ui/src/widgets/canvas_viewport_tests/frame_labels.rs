//! Root-frame label paint tests (name text, generating chip, hover/selection tint).
//!
//! Split out of `canvas_viewport_tests.rs` to keep every file under
//! the repository's 800-line cap. Shared fixtures (`RecordingBackend`,
//! scene builders, transform-replay helpers) stay in that spine.

use super::*;

#[test]
fn generating_label_text_preserves_plain_names_when_idle() {
    assert_eq!(generating_label_text("Checkout", false), "Checkout");
}

#[test]
fn generating_label_text_prefixes_active_frame_names() {
    assert_eq!(
        generating_label_text("Checkout", true),
        "Generating: Checkout"
    );
    assert_eq!(
        generating_label_text("移动端首页", true),
        "Generating: 移动端首页"
    );
}

#[test]
fn frame_label_paint_uses_top_level_scene_nodes() {
    let _guard = crate::agent_indicator_test_support::lock();
    op_editor_core::agent_indicators::clear();

    let mut roots = Vec::new();
    let mut labels = Vec::new();
    for i in 0..18 {
        let mut child = SceneNode::leaf(format!("child-{i}"), NodeKind::Rect);
        child.bounds = Rect::xywh(8.0, 8.0, 12.0, 12.0);
        let mut frame = SceneNode::leaf(format!("frame-{i}"), NodeKind::Frame);
        frame.bounds = Rect::xywh(i as f32 * 140.0, 0.0, 120.0, 80.0);
        frame.children = vec![child];
        labels.push(FrameLabel::new(
            format!("frame-{i}"),
            format!("Frame {i}"),
            Color {
                r: 0.6,
                g: 0.6,
                b: 0.6,
                a: 1.0,
            },
            false,
        ));
        roots.push(frame);
    }
    let scene = LayoutScene {
        pages: vec![ScenePage {
            id: "p".into(),
            name: "p".into(),
            children: roots,
        }],
        active_page_index: 0,
    };
    let mut state = EditorState::new();
    state.viewport.zoom = 1.0;
    let mut viewport = CanvasViewport::from_editor(&state, &scene);
    viewport.frame_labels = labels;

    crate::layout_scene::reset_find_visit_count();
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 2_800.0, 300.0));
    }

    assert_eq!(
        crate::layout_scene::find_visit_count(),
        0,
        "frame labels belong to top-level roots and should not deep-search the scene"
    );
}

#[test]
fn generating_frame_label_paints_vector_icon_and_keeps_full_hit_area() {
    let mut frame = SceneNode::leaf("n1", NodeKind::Frame);
    frame.bounds = Rect::xywh(40.0, 40.0, 120.0, 80.0);
    let color = Color {
        r: 0.0,
        g: 0.48,
        b: 1.0,
        a: 1.0,
    };
    let labels = vec![FrameLabel::new("n1", "Generating: Frame", color, true)];
    let mut state = EditorState::new();
    state.viewport.zoom = 1.0;
    let clip = Rect::xywh(0.0, 0.0, 400.0, 300.0);
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        crate::widgets::canvas_frame_labels::paint_frame_labels(
            &mut cx,
            std::slice::from_ref(&frame),
            &labels,
            &[],
            Point2D::ZERO,
            &state.viewport,
            clip,
        );
    }

    assert_eq!(backend.texts, vec!["Generating: Frame"]);
    assert_eq!(
        backend.strokes,
        crate::widgets::icons::Icon::Sparkles.paths().len()
    );
    // Far along the label's width (past the icon and into the text) and
    // vertically mid-band: the frame's top edge is y = 40, so the label box
    // spans y = 19 .. 37. Probing the middle keeps this a test of the FULL
    // hit area rather than of one band edge.
    assert_eq!(
        crate::widgets::canvas_frame_labels::frame_label_at_point(
            std::slice::from_ref(&frame),
            &labels,
            Point2D::ZERO,
            &state.viewport,
            clip,
            Point2D::new(170.0, 28.0),
        ),
        Some("n1".into())
    );

    let idle_labels = vec![FrameLabel::new("n1", "Frame", color, false)];
    let mut idle_backend = RecordingBackend::default();
    let mut cx = PaintCx {
        backend: &mut idle_backend,
    };
    crate::widgets::canvas_frame_labels::paint_frame_labels(
        &mut cx,
        &[frame],
        &idle_labels,
        &[],
        Point2D::ZERO,
        &state.viewport,
        clip,
    );
    assert_eq!(idle_backend.strokes, 0);
}

#[test]
fn selected_root_frame_label_uses_primary_active_color() {
    let scene = sample_scene();
    let mut state = EditorState::new();
    state.doc.children = vec![named_frame_node("n1", "Frame")];
    state.set_single_selection(op_editor_core::NodeId::new("n1"));
    let viewport = CanvasViewport::from_editor(&state, &scene);

    let color = viewport
        .frame_labels
        .iter()
        .find(|label| label.id == "n1")
        .map(|label| label.color)
        .expect("root frame label should be collected");

    assert_eq!(color, viewport.theme.primary);
}

#[test]
fn hovered_root_frame_label_uses_primary_active_color() {
    let _guard = crate::agent_indicator_test_support::lock();
    op_editor_core::agent_indicators::clear();
    let scene = sample_scene();
    let mut state = EditorState::new();
    state.doc.children = vec![named_frame_node("n1", "Frame")];
    state.editor_ui.canvas_hover_node = Some(op_editor_core::NodeId::new("n1"));
    let viewport = CanvasViewport::from_editor(&state, &scene);
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 800.0, 600.0));
    }

    let color = backend
        .texts
        .iter()
        .zip(backend.text_colors.iter())
        .find_map(|(text, color)| (text == "Frame").then_some(*color))
        .expect("root frame label should paint");

    assert_eq!(color, viewport.theme.primary.to_jian());
}

#[test]
fn reader_mode_collects_no_root_frame_labels() {
    let scene = sample_scene();
    let mut state = EditorState::new();
    state.doc.children = vec![named_frame_node("n1", "01 首页")];
    state.editor_ui.touch = true;
    state.editor_ui.workspace.visible = true;
    assert!(state.editor_ui.works_reader_visible());

    let viewport = CanvasViewport::from_editor(&state, &scene);

    assert!(viewport.frame_labels.is_empty());
}
