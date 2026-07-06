//! Selected-count chip layout, paint, and hit-test tests.

use super::*;
use crate::widgets::ai_chat_hit::AIChatHit;

#[derive(Default)]
struct SelectedChipPaintBackend {
    texts: Vec<(String, f32, jian_core::scene::Color, Point2D)>,
}

impl crate::RenderBackend for SelectedChipPaintBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: crate::Color) {}
    fn stroke_rect(&mut self, _: Rect, _: crate::Color, _: f32) {}
    fn draw_text(&mut self, layout: &crate::TextLayout, origin: Point2D) {
        if let Some(run) = layout.runs().first() {
            self.texts
                .push((run.content.clone(), run.font_size, run.color, origin));
        }
    }
    fn clip_rect(&mut self, _: Rect) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: crate::Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: crate::Color) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: crate::Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: crate::Color, _: f32) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

fn panel_with_selection(count: usize) -> (EditorState, Rect) {
    let mut state = EditorState::new();
    state.selection.set = (0..count)
        .map(|idx| op_editor_core::NodeId::new(format!("n{idx}")))
        .collect();
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    (state, rect)
}

fn panel_with_named_selection() -> (EditorState, Rect) {
    let mut state = EditorState::new();
    state.doc.children = vec![jian_ops_schema::node::PenNode::Rectangle(
        jian_ops_schema::node::RectangleNode {
            base: jian_ops_schema::node::PenNodeBase {
                id: "album".to_string(),
                name: Some("Album Art".to_string()),
                ..Default::default()
            },
            container: jian_ops_schema::node::ContainerProps::default(),
            children: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        },
    )];
    state.set_single_selection(op_editor_core::NodeId::new("album"));
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    (state, rect)
}

fn paint_panel(panel: &AIChatPlaceholder<'_>, rect: Rect) -> SelectedChipPaintBackend {
    let mut backend = SelectedChipPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    panel.paint(&mut cx, rect);
    backend
}

#[test]
fn selected_count_zero_has_no_chip_row() {
    let (state, rect) = panel_with_selection(0);
    let panel = AIChatPlaceholder::from_editor(&state);

    assert_eq!(panel.selection_chip_row_h(), 0.0);
    assert_eq!(panel.input_height(), INPUT_BASE_HEIGHT);

    let backend = paint_panel(&panel, rect);
    assert!(!backend
        .texts
        .iter()
        .any(|(text, _, _, _)| { text.contains("selected") || text.contains("已选择") }));
}

#[test]
fn selected_count_three_paints_chip_text() {
    let (state, rect) = panel_with_selection(3);
    let panel = AIChatPlaceholder::from_editor(&state);

    assert_eq!(panel.selection_chip_row_h(), SELECTION_CHIP_ROW_HEIGHT);
    assert_eq!(
        panel.input_height(),
        INPUT_BASE_HEIGHT + SELECTION_CHIP_ROW_HEIGHT
    );

    let backend = paint_panel(&panel, rect);
    assert!(backend
        .texts
        .iter()
        .any(|(text, size, _, _)| text.contains('3') && (*size - 11.0).abs() < 1e-4));
}

#[test]
fn single_selection_chip_paints_node_name_instead_of_count() {
    let (state, rect) = panel_with_named_selection();
    let panel = AIChatPlaceholder::from_editor(&state);

    let backend = paint_panel(&panel, rect);
    assert!(backend
        .texts
        .iter()
        .any(|(text, size, _, _)| text == "Album Art" && (*size - 11.0).abs() < 1e-4));
    assert!(!backend
        .texts
        .iter()
        .any(|(text, _, _, _)| text == "1 selected"));
}

#[test]
fn selected_count_chip_clear_hit_returns_clear_selection() {
    let (state, rect) = panel_with_selection(3);
    let panel = AIChatPlaceholder::from_editor(&state);
    let chip = panel.selection_chip_rect(panel.input_rect(rect)).unwrap();
    let point = Point2D::new(
        chip.origin.x + chip.size.x - 9.0,
        chip.origin.y + chip.size.y / 2.0,
    );

    assert_eq!(panel.hit_test(rect, point), Some(AIChatHit::ClearSelection));
}

#[test]
fn selected_count_chip_uses_zh_cn_template() {
    let (mut state, rect) = panel_with_selection(3);
    state.editor_ui.locale = op_editor_core::Locale::ZhCn;
    let panel = AIChatPlaceholder::from_editor(&state);

    let backend = paint_panel(&panel, rect);
    assert!(backend
        .texts
        .iter()
        .any(|(text, _, _, _)| text == "已选择 3 个"));
}
