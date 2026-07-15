use op_editor_core::{EditorState, NodeId};
use op_editor_ui::widgets::{LayerPanel, LayerPanelHit, PaintCx, Widget};
use op_editor_ui::{Color, Point2D, Rect, RenderBackend, TextLayout};

const PANEL_RECT: Rect = Rect::xywh(0.0, 0.0, 180.0, 168.0);
const LAYER_ROW_HEIGHT: f32 = 28.0;
const ROW_FONT: f32 = 13.0;
const MEASURED_CHAR_WIDTH: f32 = 5.2;
const BRAND_LABEL: &str = "Google Brand Mark";

const FIXTURE: &str = r#"
{
  "version": "1.0.0",
  "children": [
    {
      "type": "frame",
      "id": "login",
      "name": "Google Login",
      "width": 120,
      "height": 80,
      "children": [
        {
          "type": "rectangle",
          "id": "brand",
          "name": "Google Brand Mark",
          "width": 40,
          "height": 24
        }
      ]
    }
  ]
}
"#;

#[derive(Debug, Clone, PartialEq)]
struct CapturedText {
    content: String,
    origin: Point2D,
    active_clip: Option<Rect>,
}

#[derive(Debug, Clone, Copy)]
struct CapturedStroke {
    top_left: Point2D,
    size: f32,
    active_clip: Option<Rect>,
}

#[derive(Debug, Clone, Copy)]
struct CapturedRoundFill {
    rect: Rect,
    radius: f32,
    active_clip: Option<Rect>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct RecordingState {
    translation: Point2D,
    active_clip: Option<Rect>,
}

#[derive(Default)]
struct RecordingBackend {
    state: RecordingState,
    saved_states: Vec<RecordingState>,
    texts: Vec<CapturedText>,
    clips: Vec<Rect>,
    fill_rects: Vec<Rect>,
    round_fills: Vec<CapturedRoundFill>,
    stroke_svg_paths: Vec<CapturedStroke>,
}

impl RecordingBackend {
    fn translated_point(&self, point: Point2D) -> Point2D {
        Point2D::new(
            point.x + self.state.translation.x,
            point.y + self.state.translation.y,
        )
    }

    fn translated_rect(&self, rect: Rect) -> Rect {
        Rect {
            origin: self.translated_point(rect.origin),
            size: rect.size,
        }
    }
}

impl RenderBackend for RecordingBackend {
    fn begin_frame(&mut self) {}

    fn end_frame(&mut self) {}

    fn fill_rect(&mut self, rect: Rect, _: Color) {
        self.fill_rects.push(self.translated_rect(rect));
    }

    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}

    fn draw_text(&mut self, layout: &TextLayout, origin: Point2D) {
        for run in layout.runs() {
            self.texts.push(CapturedText {
                content: run.content.clone(),
                origin: self.translated_point(Point2D::new(
                    origin.x + run.origin.x,
                    origin.y + run.origin.y,
                )),
                active_clip: self.state.active_clip,
            });
        }
    }

    fn clip_rect(&mut self, rect: Rect) {
        let clip = self.translated_rect(rect);
        self.clips.push(clip);
        self.state.active_clip = Some(clip);
    }

    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}

    fn fill_round_rect(&mut self, rect: Rect, radius: f32, _: Color) {
        self.round_fills.push(CapturedRoundFill {
            rect: self.translated_rect(rect),
            radius,
            active_clip: self.state.active_clip,
        });
    }

    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}

    fn stroke_svg_path(&mut self, _: &str, top_left: Point2D, size: f32, _: Color, _: f32) {
        self.stroke_svg_paths.push(CapturedStroke {
            top_left: self.translated_point(top_left),
            size,
            active_clip: self.state.active_clip,
        });
    }

    fn save(&mut self) {
        self.saved_states.push(self.state);
    }

    fn restore(&mut self) {
        self.state = self
            .saved_states
            .pop()
            .expect("paint restore must match a save");
    }

    fn translate(&mut self, offset: Point2D) {
        self.state.translation += offset;
    }

    fn resize(&mut self, _: u32, _: u32) {}

    fn dpi_scale(&self) -> f32 {
        1.0
    }

    fn measure_text_family(&mut self, text: &str, _: f32, _: &str) -> f32 {
        text.chars().count() as f32 * MEASURED_CHAR_WIDTH
    }
}

fn fixture_state(hover_brand: bool) -> EditorState {
    let doc: op_editor_core::PenDocument =
        serde_json::from_str(FIXTURE).expect("layer-panel fixture parses");
    let mut state = EditorState::from_document(doc);
    state.editor_ui.layer_layers_h_scroll.offset = 28.0;
    if hover_brand {
        state.editor_ui.hovered_layer_id = Some(NodeId::new("brand"));
    }
    state
}

fn paint(panel: &LayerPanel) -> RecordingBackend {
    let mut backend = RecordingBackend::default();
    panel.paint(
        &mut PaintCx {
            backend: &mut backend,
        },
        PANEL_RECT,
    );
    assert!(
        backend.saved_states.is_empty(),
        "paint must balance save/restore"
    );
    assert_eq!(
        backend.state,
        RecordingState::default(),
        "paint must restore translation and clip state"
    );
    backend
}

fn child_row(panel: &LayerPanel) -> Rect {
    let child_index = panel
        .items
        .iter()
        .position(|item| item.node_id == NodeId::new("brand"))
        .expect("brand row is visible");
    let regions = panel.regions(PANEL_RECT);
    Rect::xywh(
        PANEL_RECT.origin.x + 6.0,
        regions.layers_rows_top - regions.layers.offset
            + child_index as f32 * LAYER_ROW_HEIGHT
            + 2.0,
        PANEL_RECT.size.x - 12.0,
        LAYER_ROW_HEIGHT - 4.0,
    )
}

fn rect_right(rect: Rect) -> f32 {
    rect.origin.x + rect.size.x
}

fn close(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() < 1e-4
}

fn point_close(actual: Point2D, expected: Point2D) -> bool {
    close(actual.x, expected.x) && close(actual.y, expected.y)
}

fn child_content_clip(backend: &RecordingBackend, row: Rect) -> Rect {
    backend
        .clips
        .iter()
        .copied()
        .find(|clip| {
            point_close(clip.origin, row.origin)
                && close(clip.size.y, row.size.y)
                && clip.size.x <= row.size.x
        })
        .expect("child row content clip paints")
}

fn label_starting_with<'a>(backend: &'a RecordingBackend, prefix: &str) -> &'a CapturedText {
    backend
        .texts
        .iter()
        .find(|text| text.content.starts_with(prefix))
        .unwrap_or_else(|| panic!("text starting with {prefix:?} paints"))
}

fn has_stroke_at(backend: &RecordingBackend, top_left: Point2D, size: f32) -> bool {
    backend
        .stroke_svg_paths
        .iter()
        .any(|stroke| point_close(stroke.top_left, top_left) && close(stroke.size, size))
}

fn assert_longest_measured_ellipsis(
    backend: &mut RecordingBackend,
    original: &str,
    rendered: &str,
    available_w: f32,
) {
    let prefix = rendered
        .strip_suffix('…')
        .expect("truncated label ends with an ellipsis");
    let remainder = original
        .strip_prefix(prefix)
        .expect("rendered prefix comes from the original label");
    let next_scalar = remainder
        .chars()
        .next()
        .expect("ellipsized label omits at least one Unicode scalar");

    let rendered_w = backend.measure_text_family(rendered, ROW_FONT, "system-ui");
    assert!(
        rendered_w <= available_w + f32::EPSILON,
        "rendered candidate {rendered:?} width {rendered_w} exceeds budget {available_w}"
    );

    let next_candidate = format!("{prefix}{next_scalar}…");
    let next_w = backend.measure_text_family(&next_candidate, ROW_FONT, "system-ui");
    assert!(
        next_w > available_w,
        "rendered candidate is not maximal: next candidate {next_candidate:?} width {next_w} \
         still fits budget {available_w}"
    );
}

#[test]
fn normal_and_hovered_rows_share_one_action_gutter() {
    let brand = NodeId::new("brand");
    let normal_panel = LayerPanel::from_editor(&fixture_state(false));
    let hovered_panel = LayerPanel::from_editor(&fixture_state(true));
    let regions = normal_panel.regions(PANEL_RECT);
    let child = normal_panel
        .items
        .iter()
        .find(|item| item.node_id == brand)
        .expect("brand item exists");
    assert_eq!(child.depth, 1);
    assert!(close(regions.layers.horizontal_offset, 28.0));

    let row = child_row(&normal_panel);
    assert_eq!(row, Rect::xywh(6.0, 134.0, 168.0, 24.0));
    let trailing_right = row.origin.x + row.size.x - 8.0;
    let lock_x = trailing_right - 14.0;
    let eye_x = lock_x - 22.0;
    let intended_gutter_left = eye_x - 8.0;
    assert!(close(trailing_right, 166.0));
    assert!(close(lock_x, 152.0));
    assert!(close(eye_x, 130.0));
    assert!(close(intended_gutter_left, 122.0));

    let mut normal = paint(&normal_panel);
    let hovered = paint(&hovered_panel);
    let normal_label = label_starting_with(&normal, "Google Brand").clone();
    let hovered_label = label_starting_with(&hovered, "Google Brand").clone();
    assert!(normal_label.content.ends_with('…'));
    assert_eq!(
        normal_label, hovered_label,
        "hover must not reflow the label"
    );

    let normal_clip = child_content_clip(&normal, row);
    let hovered_clip = child_content_clip(&hovered, row);
    assert_eq!(normal_clip, hovered_clip, "hover must not move the clip");
    let backing = hovered
        .fill_rects
        .iter()
        .copied()
        .find(|fill| {
            close(fill.origin.y, row.origin.y)
                && close(fill.size.y, row.size.y)
                && close(rect_right(*fill), rect_right(row))
        })
        .expect("hover action backing paints");

    let action_y = row.origin.y + 7.0;
    assert!(has_stroke_at(&hovered, Point2D::new(eye_x, action_y), 12.0));
    assert!(has_stroke_at(
        &hovered,
        Point2D::new(lock_x, action_y),
        12.0
    ));
    let action_center_y = action_y + 6.0;
    assert_eq!(
        hovered_panel.hit_test(PANEL_RECT, Point2D::new(eye_x + 6.0, action_center_y)),
        Some(LayerPanelHit::ToggleHidden(brand.clone()))
    );
    assert_eq!(
        hovered_panel.hit_test(PANEL_RECT, Point2D::new(lock_x + 6.0, action_center_y)),
        Some(LayerPanelHit::ToggleLocked(brand))
    );

    let measured_label_right = normal_label.origin.x
        + normal.measure_text_family(&normal_label.content, ROW_FONT, "system-ui");
    let clip_right = rect_right(normal_clip);
    let backing_left = backing.origin.x;
    assert_longest_measured_ellipsis(
        &mut normal,
        BRAND_LABEL,
        &normal_label.content,
        backing_left - normal_label.origin.x,
    );
    assert!(
        close(clip_right, backing_left)
            && close(backing_left, intended_gutter_left)
            && measured_label_right <= backing_left + f32::EPSILON,
        "content/action gutter conflict: clip right={clip_right}, backing left={backing_left}, \
         intended gutter={intended_gutter_left}, measured label right={measured_label_right}"
    );
}

#[test]
fn drag_ghost_clips_and_measured_truncates_at_the_action_gutter() {
    let state = fixture_state(false);
    let brand = NodeId::new("brand");
    let ghost = LayerPanel::ghost_item_for(&state, &brand).expect("brand ghost exists");
    let mut panel = LayerPanel::from_editor_with_drag_source(&state, &brand);
    panel.drag_ghost = Some((ghost, 30.0));

    let ghost_row = Rect::xywh(6.0, 16.0, 168.0, 24.0);
    let intended_gutter_left = 122.0;
    let mut backend = paint(&panel);
    let ghost_label = backend
        .texts
        .iter()
        .find(|text| close(text.origin.y, 33.0) && text.content.starts_with("Google"))
        .expect("ghost label paints")
        .clone();
    assert!(ghost_label.content.starts_with("Google"));
    assert!(close(ghost_label.origin.y, 33.0));

    let ghost_clip = backend
        .clips
        .iter()
        .copied()
        .find(|clip| {
            point_close(clip.origin, ghost_row.origin) && close(clip.size.y, ghost_row.size.y)
        })
        .expect("ghost content clip paints");
    let ghost_background = backend
        .round_fills
        .iter()
        .find(|fill| fill.rect == ghost_row && close(fill.radius, 6.0))
        .expect("ghost rounded background paints");
    assert_eq!(
        ghost_background.active_clip, None,
        "ghost background must paint before the content clip"
    );
    assert_eq!(
        ghost_label.active_clip,
        Some(ghost_clip),
        "ghost label must paint inside the canonical content clip"
    );
    let ghost_icon = backend
        .stroke_svg_paths
        .iter()
        .find(|stroke| {
            point_close(stroke.top_left, Point2D::new(36.0, 22.0)) && close(stroke.size, 14.0)
        })
        .expect("14px ghost icon paints");
    assert_eq!(
        ghost_icon.active_clip,
        Some(ghost_clip),
        "ghost icon must paint inside the canonical content clip"
    );
    let measured_label_right = ghost_label.origin.x
        + backend.measure_text_family(&ghost_label.content, ROW_FONT, "system-ui");
    let clip_right = rect_right(ghost_clip);
    assert_longest_measured_ellipsis(
        &mut backend,
        BRAND_LABEL,
        &ghost_label.content,
        intended_gutter_left - ghost_label.origin.x,
    );
    assert!(
        close(clip_right, intended_gutter_left)
            && ghost_label.content.ends_with('…')
            && measured_label_right <= intended_gutter_left + f32::EPSILON,
        "drag ghost must clip and truncate at {intended_gutter_left}: clip right={clip_right}, \
         label={:?}, measured label right={measured_label_right}",
        ghost_label.content
    );
}
