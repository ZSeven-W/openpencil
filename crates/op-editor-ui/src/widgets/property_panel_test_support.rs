//! Shared fixtures for the property-panel test modules
//! (`property_panel_tests`, `property_panel_multi_select_tests`,
//! `property_panel_image_fill_tests`). Kept in one place so the topic
//! test files don't duplicate the `EditorState` builder, the visible-
//! section projection, or the paint-op-counting `RenderBackend`.

use super::property_panel::{PropertyPanel, SectionCapabilities};
use super::property_panel_sections as sections;
use crate::widgets::{PaintCx, Widget};
use crate::{Color, ImageDrawMode, Point2D, Rect};
use op_editor_core::EditorState;

/// Build an `EditorState` from a canonical-schema JSON fixture.
pub(super) fn state_from(src: &str) -> EditorState {
    let doc = jian_ops_schema::load_str(src)
        .expect("property-panel fixture parses")
        .value;
    EditorState::from_document(doc)
}

/// Project the panel's snapshot onto the `VisibleSections` the layout
/// walkers consume — mirrors what `PropertyPanel::paint` derives.
pub(super) fn visible_for(panel: &PropertyPanel) -> sections::VisibleSections {
    let caps = SectionCapabilities::for_kind(&panel.snapshot.kind_variant);
    sections::VisibleSections {
        create_component: caps.create_component && panel.snapshot.can_create_component,
        component_button: if panel.snapshot.is_instance {
            crate::widgets::property_panel_visibility::ComponentButtonState::Instance
        } else if panel.snapshot.is_reusable {
            crate::widgets::property_panel_visibility::ComponentButtonState::DetachComponent
        } else {
            crate::widgets::property_panel_visibility::ComponentButtonState::Create
        },
        flex_layout: caps.flex_layout,
        flex_layout_mode: panel.snapshot.flex_layout,
        padding_edit_mode: op_editor_core::PaddingEditMode::from_values(
            panel.snapshot.layout_padding.top,
            panel.snapshot.layout_padding.right,
            panel.snapshot.layout_padding.bottom,
            panel.snapshot.layout_padding.left,
        ),
        layout_justify: panel.snapshot.layout_justify,
        layout_align: panel.snapshot.layout_align,
        size_options: caps.size_options,
        size_fill_width: panel.snapshot.size_fill_width,
        size_fill_height: panel.snapshot.size_fill_height,
        size_hug_width: panel.snapshot.size_hug_width,
        size_hug_height: panel.snapshot.size_hug_height,
        clip_content: panel.snapshot.can_clip_content,
        text: caps.text && panel.snapshot.text.is_some(),
        icon: panel.snapshot.icon.is_some(),
        widget: panel.snapshot.widget.as_ref().map(|w| w.kind),
        widget_checked: panel.snapshot.widget.as_ref().is_some_and(|w| w.checked),
        image: caps.image && panel.snapshot.is_image_node,
        image_warning: false,
        opacity: caps.opacity,
        corner_radius: panel.snapshot.has_corner_radius,
        polygon_sides: panel.snapshot.polygon_sides.is_some(),
        ellipse_arc: panel.snapshot.ellipse_arc.is_some(),
        fill: caps.fill,
        stroke: caps.stroke,
        color_variable_count: panel.color_variable_count,
        fill_variable_bound: panel.fill_variable_ref.is_some(),
        stroke_variable_bound: panel.stroke_variable_ref.is_some(),
        color_variable_picker_open: panel.color_variable_picker_open,
        effects: caps.effects,
        export: caps.export,
        fill_type: panel.fill_type,
        gradient_stop_count: panel.snapshot.gradient_stops.len(),
    }
}

/// Minimal `RenderBackend` that counts paint ops + records what was
/// drawn, so paint tests can assert on the output without a real GPU.
#[derive(Default)]
pub(super) struct CountingBackend {
    pub(super) text: usize,
    pub(super) texts: Vec<String>,
    pub(super) round_rects: usize,
    pub(super) images: Vec<(Rect, u64, usize)>,
    pub(super) image_modes: Vec<ImageDrawMode>,
}
impl crate::RenderBackend for CountingBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, layout: &crate::TextLayout, _: Point2D) {
        self.text += 1;
        if let Some(run) = layout.runs().first() {
            self.texts.push(run.content.clone());
        }
    }
    fn clip_rect(&mut self, _: Rect) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {
        self.round_rects += 1;
    }
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn draw_image(&mut self, rect: Rect, image_id: u64, encoded: &[u8]) {
        self.images.push((rect, image_id, encoded.len()));
    }
    fn draw_image_with_mode(
        &mut self,
        rect: Rect,
        image_id: u64,
        encoded: &[u8],
        mode: ImageDrawMode,
    ) {
        self.images.push((rect, image_id, encoded.len()));
        self.image_modes.push(mode);
    }
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

/// Paint `panel` into a `CountingBackend` and return (text-ops,
/// round-rect-ops).
pub(super) fn paint_and_count(panel: &PropertyPanel, rect: Rect) -> (usize, usize) {
    let mut backend = CountingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        panel.paint(&mut cx, rect);
    }
    (backend.text, backend.round_rects)
}
