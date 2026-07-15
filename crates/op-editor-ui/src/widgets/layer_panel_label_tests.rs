//! Focused paint + interaction regressions for layer-row labels.

use super::layer_panel::{LayerPanel, LayerPanelHit, LAYER_PANEL_WIDTH, LAYER_ROW_HEIGHT};
use super::layer_panel_paint::{
    approx_text_width, layer_action_gutter_left, layer_trailing_icon_xs, paint_rename_input,
    ROW_FONT,
};
use super::{PaintCx, Widget};
use crate::theme::Theme;
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use jian_core::text_input::TextInputState;
use op_editor_core::{EditorState, NodeId};

const LONG_NAME: &str = "CodexLiveCanvasSmoke-1783-Long-Layer-Name";
const WIDE_NAME: &str = "设计系统图层名称设计系统图层名称设计系统图层名称";

#[derive(Debug)]
struct CapturedText {
    content: String,
    origin: Point2D,
}

#[derive(Default)]
struct LayerLabelBackend {
    translation: Point2D,
    saved_translations: Vec<Point2D>,
    texts: Vec<CapturedText>,
    fills: Vec<Rect>,
    strokes: Vec<(Point2D, f32)>,
    clips: Vec<Rect>,
}

impl RenderBackend for LayerLabelBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, rect: Rect, _: Color) {
        self.fills.push(Rect {
            origin: Point2D::new(
                rect.origin.x + self.translation.x,
                rect.origin.y + self.translation.y,
            ),
            size: rect.size,
        });
    }
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}

    fn draw_text(&mut self, layout: &TextLayout, origin: Point2D) {
        for run in layout.runs() {
            self.texts.push(CapturedText {
                content: run.content.clone(),
                origin: Point2D::new(
                    origin.x + self.translation.x + run.origin.x,
                    origin.y + self.translation.y + run.origin.y,
                ),
            });
        }
    }

    fn clip_rect(&mut self, rect: Rect) {
        self.clips.push(Rect {
            origin: Point2D::new(
                rect.origin.x + self.translation.x,
                rect.origin.y + self.translation.y,
            ),
            size: rect.size,
        });
    }
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, top_left: Point2D, size: f32, _: Color, _: f32) {
        self.strokes.push((
            Point2D::new(
                top_left.x + self.translation.x,
                top_left.y + self.translation.y,
            ),
            size,
        ));
    }

    fn save(&mut self) {
        self.saved_translations.push(self.translation);
    }

    fn restore(&mut self) {
        self.translation = self
            .saved_translations
            .pop()
            .expect("paint restore should match a save");
    }

    fn translate(&mut self, offset: Point2D) {
        self.translation.x += offset.x;
        self.translation.y += offset.y;
    }

    fn resize(&mut self, _: u32, _: u32) {}

    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

fn state_with_layer_name(name: &str) -> EditorState {
    let source = format!(
        r#"{{"version":"0.8.0","children":[{{"type":"rectangle","id":"n1","name":"{name}","width":10,"height":10}}]}}"#
    );
    let doc = jian_ops_schema::load_str(&source)
        .expect("layer-label fixture parses")
        .value;
    EditorState::from_document(doc)
}

fn renaming_state(hovered: bool) -> EditorState {
    let mut state = state_with_layer_name("Original");
    assert!(state.start_rename_layer(NodeId::new("n1")));
    state
        .ui
        .layer_rename
        .as_mut()
        .expect("rename is active")
        .input
        .set_text(LONG_NAME);
    if hovered {
        state.editor_ui.hovered_layer_id = Some(NodeId::new("n1"));
    }
    state
}

fn panel_rect(panel: &LayerPanel) -> Rect {
    Rect::xywh(0.0, 0.0, LAYER_PANEL_WIDTH, panel.intrinsic_height())
}

fn paint(panel: &LayerPanel, rect: Rect) -> LayerLabelBackend {
    let mut backend = LayerLabelBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    panel.paint(&mut cx, rect);
    backend
}

fn captured_layer_label(backend: &LayerLabelBackend) -> &CapturedText {
    backend
        .texts
        .iter()
        .find(|text| text.content.starts_with("CodexLiveCanvas"))
        .expect("layer label should paint")
}

fn first_layer_row(panel: &LayerPanel, rect: Rect) -> Rect {
    Rect::xywh(
        rect.origin.x + 6.0,
        panel.regions(rect).layers_rows_top + 2.0,
        rect.size.x - 12.0,
        LAYER_ROW_HEIGHT - 4.0,
    )
}

fn approx_point(a: Point2D, b: Point2D) -> bool {
    (a.x - b.x).abs() < 1e-4 && (a.y - b.y).abs() < 1e-4
}

fn has_clip(backend: &LayerLabelBackend, expected: Rect) -> bool {
    backend.clips.iter().any(|clip| {
        approx_point(clip.origin, expected.origin) && approx_point(clip.size, expected.size)
    })
}

#[test]
fn long_layer_name_ellipsizes_before_stable_action_gutter() {
    let state = state_with_layer_name(LONG_NAME);
    let panel = LayerPanel::from_editor(&state);
    let rect = panel_rect(&panel);
    let backend = paint(&panel, rect);
    let label = captured_layer_label(&backend);

    assert!(
        label.content.ends_with('…'),
        "long layer label should be ellipsized, painted {:?}",
        label.content
    );

    let row = first_layer_row(&panel, rect);
    let gutter_left = layer_action_gutter_left(row);
    let content_clip = Rect::xywh(
        row.origin.x,
        row.origin.y,
        gutter_left - row.origin.x,
        row.size.y,
    );
    assert!(
        has_clip(&backend, content_clip),
        "layer content should be hard-clipped before the fixed action gutter"
    );
    let label_right = label.origin.x + approx_text_width(&label.content, ROW_FONT);
    assert!(
        label_right <= gutter_left + f32::EPSILON,
        "layer label right edge {label_right} should stay before action gutter at {gutter_left}"
    );

    let mut hovered_state = state;
    hovered_state.editor_ui.hovered_layer_id = Some(NodeId::new("n1"));
    let hovered_panel = LayerPanel::from_editor(&hovered_state);
    let hovered_backend = paint(&hovered_panel, rect);
    let hovered_label = captured_layer_label(&hovered_backend);
    assert_eq!(
        hovered_label.content, label.content,
        "hover should not change the label width budget"
    );
    assert!(
        approx_point(hovered_label.origin, label.origin),
        "hover should not move the label"
    );
}

#[test]
fn horizontal_scroll_expands_label_budget_to_fixed_action_edge() {
    let unscrolled_panel = LayerPanel::from_editor(&state_with_layer_name(LONG_NAME));
    let rect = panel_rect(&unscrolled_panel);
    let unscrolled_backend = paint(&unscrolled_panel, rect);
    let unscrolled_label = captured_layer_label(&unscrolled_backend);

    let mut scrolled_state = state_with_layer_name(LONG_NAME);
    scrolled_state.editor_ui.layer_layers_h_scroll.offset = 80.0;
    let scrolled_panel = LayerPanel::from_editor(&scrolled_state);
    let regions = scrolled_panel.regions(rect);
    assert_eq!(regions.layers.horizontal_offset, 80.0);
    let scrolled_backend = paint(&scrolled_panel, rect);
    let scrolled_label = captured_layer_label(&scrolled_backend);

    assert!(
        scrolled_label.content.chars().count() > unscrolled_label.content.chars().count(),
        "scrolling content left should expose more of the label without moving fixed actions"
    );
    let row = first_layer_row(&scrolled_panel, rect);
    let gutter_left = layer_action_gutter_left(row);
    let label_right =
        scrolled_label.origin.x + approx_text_width(&scrolled_label.content, ROW_FONT);
    assert!(
        label_right <= gutter_left + f32::EPSILON,
        "translated label right edge {label_right} should stay before action gutter at \
         {gutter_left}"
    );
}

#[test]
fn wide_layer_name_ellipsis_fits_backend_measured_width() {
    let panel = LayerPanel::from_editor(&state_with_layer_name(WIDE_NAME));
    let rect = panel_rect(&panel);
    let mut backend = paint(&panel, rect);
    let (content, origin) = backend
        .texts
        .iter()
        .find(|text| text.content.starts_with("设计系统"))
        .map(|text| (text.content.clone(), text.origin))
        .expect("wide layer label should paint");
    assert!(
        content.ends_with('…'),
        "wide layer label should end in a Unicode ellipsis"
    );

    let row = first_layer_row(&panel, rect);
    let available_w = layer_action_gutter_left(row) - origin.x;
    let measured_w = backend.measure_text_family(&content, ROW_FONT, "system-ui");
    assert!(
        measured_w <= available_w + f32::EPSILON,
        "backend-measured label width {measured_w} should fit budget {available_w}: {content:?}"
    );
}

#[test]
fn renaming_layer_paints_full_text_input_draft() {
    let state = renaming_state(false);
    let panel = LayerPanel::from_editor(&state);
    assert_eq!(
        panel.rename_input.as_ref().expect("rename input").text(),
        LONG_NAME
    );

    let rect = panel_rect(&panel);
    let backend = paint(&panel, rect);
    assert!(
        backend.texts.iter().any(|text| text.content == LONG_NAME),
        "inline rename must paint the full TextInputState value"
    );
}

#[test]
fn renaming_layer_underline_ends_inside_row() {
    let panel = LayerPanel::from_editor(&renaming_state(false));
    let rect = panel_rect(&panel);
    let row = first_layer_row(&panel, rect);
    let backend = paint(&panel, rect);
    let content_clip = Rect::xywh(row.origin.x, row.origin.y, row.size.x - 8.0, row.size.y);
    assert!(
        has_clip(&backend, content_clip),
        "rename content should be clipped at the row's 8px right inset"
    );
    let underline_y = row.origin.y + row.size.y - 1.0;
    let underline = backend
        .fills
        .iter()
        .find(|fill| (fill.origin.y - underline_y).abs() < 1e-4 && (fill.size.y - 1.0).abs() < 1e-4)
        .expect("rename underline should paint");

    assert!(
        underline.origin.x + underline.size.x <= row.origin.x + row.size.x - 8.0 + f32::EPSILON,
        "rename underline right edge {} should stay inside row at {}",
        underline.origin.x + underline.size.x,
        row.origin.x + row.size.x - 8.0
    );
}

#[test]
fn zero_width_rename_input_paints_nothing_until_horizontal_scroll_reveals_space() {
    let input = TextInputState::with_text(LONG_NAME);
    let mut backend = LayerLabelBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    paint_rename_input(&mut cx, &Theme::dark(), &input, 240.0, 20.0, 0.0, 0);

    assert!(
        !backend.texts.iter().any(|text| text.content == LONG_NAME),
        "zero remaining width should defer input glyph paint"
    );
    assert!(
        !backend
            .fills
            .iter()
            .any(|fill| (fill.origin.y - 43.0).abs() < 1e-4 && fill.size.y == 1.0),
        "zero remaining width should not paint an out-of-bounds underline"
    );
    assert_eq!(
        input.text(),
        LONG_NAME,
        "deferring paint must not alter the live rename draft"
    );
}

#[test]
fn renaming_layer_underline_does_not_force_width_past_narrow_row() {
    let panel = LayerPanel::from_editor(&renaming_state(false));
    let rect = Rect::xywh(0.0, 0.0, 100.0, panel.intrinsic_height());
    let row = first_layer_row(&panel, rect);
    let backend = paint(&panel, rect);
    let underline_y = row.origin.y + row.size.y - 1.0;
    let underline = backend
        .fills
        .iter()
        .find(|fill| (fill.origin.y - underline_y).abs() < 1e-4 && (fill.size.y - 1.0).abs() < 1e-4)
        .expect("rename underline should paint");

    assert!(
        underline.origin.x + underline.size.x <= row.origin.x + row.size.x - 8.0 + f32::EPSILON,
        "narrow rename underline right edge {} should stay inside row at {}",
        underline.origin.x + underline.size.x,
        row.origin.x + row.size.x - 8.0
    );
}

#[test]
fn hovered_renaming_layer_paints_no_trailing_actions() {
    let panel = LayerPanel::from_editor(&renaming_state(true));
    let rect = panel_rect(&panel);
    let row = first_layer_row(&panel, rect);
    let (eye_x, lock_x) = layer_trailing_icon_xs(row);
    let action_y = row.origin.y + 7.0;
    let backend = paint(&panel, rect);

    for action_origin in [
        Point2D::new(eye_x, action_y),
        Point2D::new(lock_x, action_y),
    ] {
        assert!(
            !backend.strokes.iter().any(|(top_left, size)| {
                approx_point(*top_left, action_origin) && (*size - 12.0).abs() < 1e-4
            }),
            "renaming row should not paint trailing action at {action_origin:?}"
        );
    }
}

#[test]
fn hovered_renaming_layer_action_points_fall_back_to_layer_hit() {
    let panel = LayerPanel::from_editor(&renaming_state(true));
    let rect = panel_rect(&panel);
    let row = first_layer_row(&panel, rect);
    let (eye_x, lock_x) = layer_trailing_icon_xs(row);
    let action_center_y = row.origin.y + 12.0;

    for point in [
        Point2D::new(eye_x + 6.0, action_center_y),
        Point2D::new(lock_x + 6.0, action_center_y),
    ] {
        assert_eq!(
            panel.hit_test(rect, point),
            Some(LayerPanelHit::Layer(NodeId::new("n1"))),
            "renaming action position should remain a row hit"
        );
    }
}
