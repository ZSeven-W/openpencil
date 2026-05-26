//! Tests for `widgets::property_panel` — moved to a sibling file to
//! keep `property_panel.rs` under the 800-line cap.
//!
//! Phase 6: the panel builds from `op_editor_core::EditorState`, so
//! the fixtures construct `EditorState` values.

use super::property_panel::{PropertyPanel, PropertyPanelAction, SectionCapabilities};
use super::property_panel_sections as sections;
use crate::widgets::{PaintCx, Widget};
use crate::{Color, ImageDrawMode, Point2D, Rect};
use op_editor_core::{EditorState, NodeId};

/// Build an `EditorState` from a canonical `.op` JSON string.
fn state_from(src: &str) -> EditorState {
    let doc = jian_ops_schema::load_str(src)
        .expect("property-panel fixture parses")
        .value;
    EditorState::from_document(doc)
}

#[test]
fn for_selection_with_real_node_builds_snapshot() {
    let state = EditorState::sample();
    let panel = PropertyPanel::for_selection(&state).expect("sample doc has a selection");
    assert_eq!(panel.snapshot.kind, "Text");
    assert_eq!(panel.snapshot.name, "Title");
    // Title node bounds: (60, 60, 240, 28).
    assert_eq!(panel.snapshot.x, 60);
    assert_eq!(panel.snapshot.y, 60);
    assert_eq!(panel.snapshot.width, 240);
    assert_eq!(panel.snapshot.height, 28);
}

#[test]
fn for_selection_without_selection_returns_none() {
    let state = EditorState::new();
    assert!(PropertyPanel::for_selection(&state).is_none());
}

#[test]
fn for_selection_with_stale_selection_returns_none() {
    let mut state = EditorState::sample();
    state.set_single_selection(NodeId::new("n9999"));
    assert!(PropertyPanel::for_selection(&state).is_none());
}

#[test]
fn access_node_advertises_group_with_kind_label() {
    let state = EditorState::sample();
    let panel = PropertyPanel::for_selection(&state).unwrap();
    let node = panel.access_node();
    assert_eq!(node.role(), accesskit::Role::Group);
    assert_eq!(node.label(), Some("Text"));
}

#[test]
fn group_snapshot_aggregates_child_bounds() {
    // A Group has no own bounds, so `from_node` must derive W/H
    // from children — else the panel shows "0 × 0" for a container.
    let mut state = EditorState::sample();
    state.set_single_selection(NodeId::new("n12"));
    let panel = PropertyPanel::for_selection(&state).unwrap();
    assert_eq!(panel.snapshot.kind, "Group");
    assert_eq!(panel.snapshot.x, 60);
    assert_eq!(panel.snapshot.y, 130);
    assert!(panel.snapshot.width > 0);
    assert!(panel.snapshot.height > 0);
}

/// Build a `VisibleSections` from a panel's per-kind capabilities.
fn visible_for(panel: &PropertyPanel) -> sections::VisibleSections {
    let caps = SectionCapabilities::for_kind(&panel.snapshot.kind_variant);
    sections::VisibleSections {
        create_component: caps.create_component && panel.snapshot.can_create_component,
        flex_layout: caps.flex_layout,
        flex_layout_mode: panel.snapshot.flex_layout,
        layout_justify: panel.snapshot.layout_justify,
        layout_align: panel.snapshot.layout_align,
        size_options: caps.size_options,
        clip_content: panel.snapshot.can_clip_content,
        text: caps.text && panel.snapshot.text.is_some(),
        icon: panel.snapshot.icon.is_some(),
        image: caps.image && panel.snapshot.is_image_node,
        opacity: caps.opacity,
        corner_radius: panel.snapshot.has_corner_radius,
        polygon_sides: panel.snapshot.polygon_sides.is_some(),
        ellipse_arc: panel.snapshot.ellipse_arc.is_some(),
        fill: caps.fill,
        stroke: caps.stroke,
        effects: caps.effects,
        export: caps.export,
        fill_type: panel.fill_type,
        gradient_stop_count: panel.snapshot.gradient_stops.len(),
    }
}

#[test]
fn polygon_selection_exposes_sides_layer_input() {
    let mut state = state_from(
        r##"{ "version": "0.8.0", "children": [
              {"type":"polygon","id":"poly","name":"Hex",
               "x":40,"y":40,"width":120,"height":120,
               "polygonCount":6}
        ]}"##,
    );
    state.set_single_selection(NodeId::new("poly"));
    let panel = PropertyPanel::for_selection(&state).expect("polygon panel");

    assert_eq!(panel.snapshot.polygon_sides, Some(6));

    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 1200.0),
    };
    let sides_rect = sections::editable_input_rects(rect, visible_for(&panel))
        .into_iter()
        .find(|(focus, _)| *focus == op_editor_core::PropertyFocus::PolygonSides)
        .map(|(_, r)| r)
        .expect("polygon side input rect");
    let center = Point2D::new(
        sides_rect.origin.x + sides_rect.size.x / 2.0,
        sides_rect.origin.y + sides_rect.size.y / 2.0,
    );
    assert_eq!(
        panel.hit_test(rect, center),
        Some(op_editor_core::PropertyFocus::PolygonSides)
    );
}

#[test]
fn ellipse_selection_exposes_arc_layer_inputs() {
    let mut state = state_from(
        r##"{ "version": "0.8.0", "children": [
              {"type":"ellipse","id":"ell","name":"Arc",
               "x":40,"y":40,"width":120,"height":100,
               "startAngle":30,"sweepAngle":270,"innerRadius":0.25}
        ]}"##,
    );
    state.set_single_selection(NodeId::new("ell"));
    let panel = PropertyPanel::for_selection(&state).expect("ellipse panel");

    let arc = panel.snapshot.ellipse_arc.expect("ellipse arc snapshot");
    assert_eq!(arc.start_deg, 30.0);
    assert_eq!(arc.sweep_deg, 270.0);
    assert_eq!(arc.inner_percent, 25.0);

    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 1200.0),
    };
    let rects = sections::editable_input_rects(rect, visible_for(&panel));
    for focus in [
        op_editor_core::PropertyFocus::EllipseStart,
        op_editor_core::PropertyFocus::EllipseSweep,
        op_editor_core::PropertyFocus::EllipseInnerRadius,
    ] {
        let target = rects
            .iter()
            .find(|(f, _)| *f == focus)
            .map(|(_, r)| *r)
            .expect("ellipse arc input rect");
        let center = Point2D::new(
            target.origin.x + target.size.x / 2.0,
            target.origin.y + target.size.y / 2.0,
        );
        assert_eq!(panel.hit_test(rect, center), Some(focus));
    }
}

#[test]
fn hit_test_action_export_section_returns_picker_toggles() {
    // Single-frame selection paints every section + Export.
    let mut state = EditorState::sample();
    state.set_single_selection(NodeId::new("n10"));
    let panel = PropertyPanel::for_selection(&state).expect("frame panel");
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 1600.0),
    };
    let rects = sections::action_button_rects_with_fill_picker(
        rect,
        visible_for(&panel),
        &panel.snapshot.effects,
        false,
        false,
        false,
        false,
    );
    // The Export section emits a scale-dropdown + a format-dropdown
    // toggle rect — clicking neither opens the Export modal.
    let scale_rect = rects
        .iter()
        .find(|(a, _)| matches!(a, PropertyPanelAction::ToggleExportScalePicker))
        .map(|(_, r)| *r)
        .expect("export section must emit a scale-dropdown rect");
    let format_rect = rects
        .iter()
        .find(|(a, _)| matches!(a, PropertyPanelAction::ToggleExportFormatPicker))
        .map(|(_, r)| *r)
        .expect("export section must emit a format-dropdown rect");
    let scale_center = Point2D::new(
        scale_rect.origin.x + scale_rect.size.x / 2.0,
        scale_rect.origin.y + scale_rect.size.y / 2.0,
    );
    assert!(
        matches!(
            panel.hit_test_action(rect, scale_center),
            Some(PropertyPanelAction::ToggleExportScalePicker)
        ),
        "click on the scale dropdown should toggle the scale picker",
    );
    let format_center = Point2D::new(
        format_rect.origin.x + format_rect.size.x / 2.0,
        format_rect.origin.y + format_rect.size.y / 2.0,
    );
    assert!(
        matches!(
            panel.hit_test_action(rect, format_center),
            Some(PropertyPanelAction::ToggleExportFormatPicker)
        ),
        "click on the format dropdown should toggle the format picker",
    );
}

#[test]
fn flex_advanced_rows_do_not_overlap_gap_modes() {
    let mut state = state_from(
        r##"{ "version": "0.8.0", "children": [
              {"type":"frame","id":"f","name":"Frame",
               "x":40,"y":40,"width":360,"height":240,
               "layout":"horizontal","gap":0,
               "children":[]}
        ]}"##,
    );
    state.set_single_selection(NodeId::new("f"));
    let panel = PropertyPanel::for_selection(&state).expect("frame panel");
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 1200.0),
    };
    let visible = visible_for(&panel);
    let actions = sections::action_button_rects_with_fill_picker(
        rect,
        visible,
        &panel.snapshot.effects,
        false,
        false,
        false,
        false,
    );
    let last_gap_mode = actions
        .iter()
        .find(|(action, _)| {
            matches!(
                action,
                PropertyPanelAction::SetLayoutJustify(
                    super::property_panel::LayoutJustifyValue::SpaceAround
                )
            )
        })
        .map(|(_, r)| *r)
        .expect("space-around hit rect");
    let padding_top = sections::editable_input_rects(rect, visible)
        .into_iter()
        .find(|(focus, _)| *focus == op_editor_core::PropertyFocus::PaddingTop)
        .map(|(_, r)| r)
        .expect("padding top input rect");

    assert!(
        padding_top.origin.y >= last_gap_mode.origin.y + last_gap_mode.size.y + 18.0,
        "padding inputs must start below the full gap-mode column"
    );
}

#[test]
fn font_family_picker_rows_are_clickable() {
    let mut state = EditorState::sample();
    state.set_single_selection(NodeId::new("n11"));
    state.editor_ui.font_family_picker_open = true;
    let panel = PropertyPanel::for_selection(&state).expect("text panel");
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 1200.0),
    };
    let poppins = panel
        .font_family_picker_row_rect(rect, "Poppins")
        .expect("Poppins font row");
    let center = Point2D::new(
        poppins.origin.x + poppins.size.x / 2.0,
        poppins.origin.y + poppins.size.y / 2.0,
    );
    assert!(matches!(
        panel.hit_test_action(rect, center),
        Some(PropertyPanelAction::SetFontFamily(family)) if family == "Poppins"
    ));
}

#[test]
fn font_family_options_include_promoted_system_fonts() {
    let system_fonts = vec![
        "Zapfino".to_string(),
        "PingFang SC".to_string(),
        "Hiragino Sans GB".to_string(),
    ];
    let system_fonts =
        super::property_panel_font_picker::prepare_system_font_families(system_fonts);
    let options =
        super::property_panel_font_picker::font_family_options(&system_fonts, "My Brand Font");
    assert_eq!(options.first().map(String::as_str), Some("My Brand Font"));
    let pingfang = options
        .iter()
        .position(|family| family == "PingFang SC")
        .expect("system CJK font should be visible");
    let zapfino = options
        .iter()
        .position(|family| family == "Zapfino")
        .expect("regular system font should be visible");
    assert!(pingfang < zapfino, "CJK fonts should be promoted");
}

#[test]
fn prepared_system_fonts_are_sorted_once_for_smooth_picker_scroll() {
    let system_fonts = vec![
        "Zapfino".to_string(),
        "  ".to_string(),
        "PingFang SC".to_string(),
        "zapfino".to_string(),
        "Bodoni 72".to_string(),
    ];
    let prepared = super::property_panel_font_picker::prepare_system_font_families(system_fonts);

    assert_eq!(
        prepared,
        vec![
            "PingFang SC".to_string(),
            "Bodoni 72".to_string(),
            "Zapfino".to_string()
        ]
    );
}

#[test]
fn font_family_picker_scroll_hits_later_system_fonts() {
    let mut state = EditorState::sample();
    state.set_single_selection(NodeId::new("n11"));
    state.editor_ui.font_family_picker_open = true;
    state.editor_ui.system_font_families =
        (0..120).map(|i| format!("System Font {i:03}")).collect();
    state.editor_ui.font_family_picker_scroll = 56.0 * 20.0;

    let panel = PropertyPanel::for_selection(&state).expect("text panel");
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 900.0),
    };

    assert!(panel.font_family_picker_max_scroll(rect) > 0.0);
    let target = panel
        .font_family_picker_row_rect(rect, "System Font 030")
        .expect("scrolled system font row");
    let center = Point2D::new(
        target.origin.x + target.size.x / 2.0,
        target.origin.y + target.size.y / 2.0,
    );

    assert!(matches!(
        panel.hit_test_action(rect, center),
        Some(PropertyPanelAction::SetFontFamily(family)) if family == "System Font 030"
    ));
}

#[test]
fn font_family_options_include_all_system_fonts_after_scroll_support() {
    let mut system_fonts: Vec<String> = (0..400).map(|i| format!("System Font {i}")).collect();
    system_fonts.push("PingFang SC".to_string());
    system_fonts.push("宋体".to_string());

    let options =
        super::property_panel_font_picker::font_family_options(&system_fonts, "Brand Font");

    assert_eq!(options.first().map(String::as_str), Some("Brand Font"));
    assert!(options.iter().any(|family| family == "PingFang SC"));
    assert!(options.iter().any(|family| family == "宋体"));
    assert!(options.iter().any(|family| family == "System Font 399"));
}

#[test]
fn export_scale_picker_open_emits_option_rows() {
    let mut state = EditorState::sample();
    state.set_single_selection(NodeId::new("n10"));
    // Opening the scale picker makes the option rows part of the
    // panel's hit surface.
    state.editor_ui.export_scale_picker_open = true;
    let panel = PropertyPanel::for_selection(&state).expect("frame panel");
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 1600.0),
    };
    let rects = sections::action_button_rects_with_fill_picker(
        rect,
        visible_for(&panel),
        &panel.snapshot.effects,
        false,
        false,
        true,
        false,
    );
    let rows: Vec<_> = rects
        .iter()
        .filter(|(a, _)| matches!(a, PropertyPanelAction::SetExportScale(_)))
        .collect();
    assert_eq!(rows.len(), 3, "open scale picker emits 1x/2x/3x rows");
    // A click on an option row wins over the dropdown toggle it
    // overlaps — `hit_test_action` walks the rects in `rev()`.
    let row = rows[0].1;
    let row_center = Point2D::new(
        row.origin.x + row.size.x / 2.0,
        row.origin.y + row.size.y / 2.0,
    );
    assert!(
        matches!(
            panel.hit_test_action(rect, row_center),
            Some(PropertyPanelAction::SetExportScale(_))
        ),
        "click on a picker row resolves to SetExportScale",
    );
}

#[test]
fn format_color_hex_pads_to_six_chars() {
    use crate::widgets::property_panel_inputs::format_color_hex;
    assert_eq!(format_color_hex(Color::WHITE), "#FFFFFF");
    assert_eq!(format_color_hex(Color::BLACK), "#000000");
    assert_eq!(format_color_hex(Color::RED), "#FF0000");
}

#[test]
fn multi_selection_panel_shows_union_bounds_and_is_inert() {
    let mut state = EditorState::sample();
    state.set_single_selection(NodeId::new("n11"));
    state.toggle_selection(NodeId::new("n12"));
    assert_eq!(state.selection_count(), 2);

    let panel = PropertyPanel::for_selection(&state).expect("multi-select must paint");
    assert!(panel.is_multi);
    assert_eq!(panel.snapshot.kind, "2 items");
    assert_eq!(panel.snapshot.x, 60);
    assert_eq!(panel.snapshot.y, 60);
    // Union spans Title (y 60..88) + Button group (y 130..166) →
    // x=60, w=240, h≈106.
    assert!(panel.snapshot.width >= 240);
    assert!(panel.snapshot.height >= 100);
    assert!(panel.focus.is_none());
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 600.0),
    };
    assert!(panel.hit_test(rect, Point2D::new(140.0, 100.0)).is_none());
    assert!(panel
        .hit_test_action(rect, Point2D::new(140.0, 100.0))
        .is_none());
}

/// Minimal `RenderBackend` that counts paint ops.
#[derive(Default)]
struct CountingBackend {
    text: usize,
    texts: Vec<String>,
    round_rects: usize,
    images: Vec<(Rect, u64, usize)>,
    image_modes: Vec<ImageDrawMode>,
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

#[test]
fn multi_select_paint_diverges_from_full_section_paint() {
    let mut state = EditorState::sample();
    state.set_single_selection(NodeId::new("n11"));
    state.toggle_selection(NodeId::new("n12"));
    let panel_multi = PropertyPanel::for_selection(&state).expect("multi");
    state.set_single_selection(NodeId::new("n10"));
    let panel_frame = PropertyPanel::for_selection(&state).expect("frame");
    assert!(!panel_frame.is_multi);

    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 1200.0),
    };
    let multi = paint_and_count(&panel_multi, rect);
    let frame = paint_and_count(&panel_frame, rect);
    assert_ne!(multi, frame, "multi must paint fewer ops than single-Frame");
    assert!(multi.0 > 5 && multi.1 > 0, "Size section must paint");
}

fn paint_and_count(panel: &PropertyPanel, rect: Rect) -> (usize, usize) {
    let mut backend = CountingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        panel.paint(&mut cx, rect);
    }
    (backend.text, backend.round_rects)
}

#[test]
fn multi_select_caps_keep_size_hide_fill_and_stroke() {
    let mut state = EditorState::sample();
    state.set_single_selection(NodeId::new("n11"));
    state.toggle_selection(NodeId::new("n12"));
    let panel = PropertyPanel::for_selection(&state).expect("multi-select panel");
    assert!(panel.is_multi);
    let caps = panel.capabilities();
    assert!(caps.size_options, "multi-select must paint W/H");
    assert!(!caps.fill, "multi-select must hide fill section");
    assert!(!caps.stroke, "multi-select must hide stroke section");
    assert!(!caps.flex_layout, "multi-select hides flex");
    // A Rect selection routes through `for_kind`, exposing fill/stroke.
    state.set_single_selection(NodeId::new("n13"));
    let single = PropertyPanel::for_selection(&state).expect("single-select panel");
    let caps_single = single.capabilities();
    assert!(caps_single.fill, "single Rect must paint fill");
    assert!(caps_single.stroke, "single Rect must paint stroke");
}

#[test]
fn multi_select_panel_shows_even_when_all_zero_size() {
    // Symmetry with single-select: a 0x0 node still shows the panel.
    let mut state = state_from(
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"n50","name":"A"},
              {"type":"rectangle","id":"n51","name":"B"}
        ]}"##,
    );
    state.set_single_selection(NodeId::new("n50"));
    state.toggle_selection(NodeId::new("n51"));
    assert_eq!(state.selection_count(), 2);
    let panel = PropertyPanel::for_selection(&state).expect("0x0 multi-select must paint");
    assert!(panel.is_multi);
    assert_eq!(panel.snapshot.width, 0);
    assert_eq!(panel.snapshot.height, 0);
}

fn image_fill_state_with_url(url: &str) -> EditorState {
    let mut state = state_from(&format!(
        r##"{{ "version": "0.8.0", "children": [
              {{"type":"rectangle","id":"n60","name":"Photo fill",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{{"type":"image","url":"{}","mode":"fill",
                 "exposure":0,"contrast":0,"saturation":0,
                 "temperature":0,"tint":0,"highlights":0,"shadows":0}}]}}
        ]}}"##,
        url
    ));
    state.set_single_selection(NodeId::new("n60"));
    state
}

fn image_fill_state() -> EditorState {
    image_fill_state_with_url("")
}

#[test]
fn image_fill_body_click_opens_the_image_popover() {
    let state = image_fill_state();
    let panel = PropertyPanel::for_selection(&state).expect("image fill panel");
    let rect = Rect {
        origin: Point2D::new(320.0, 24.0),
        size: Point2D::new(280.0, 900.0),
    };
    let rects = sections::action_button_rects_with_fill_picker(
        rect,
        visible_for(&panel),
        &panel.snapshot.effects,
        false,
        false,
        false,
        false,
    );
    let body = rects
        .iter()
        .find(|(a, _)| matches!(a, PropertyPanelAction::ToggleImageFillPopover))
        .map(|(_, r)| *r)
        .expect("image fill body emits popover toggle action");
    let center = Point2D::new(
        body.origin.x + body.size.x / 2.0,
        body.origin.y + body.size.y / 2.0,
    );
    assert!(
        matches!(
            panel.hit_test_action(rect, center),
            Some(PropertyPanelAction::ToggleImageFillPopover)
        ),
        "image fill body click should open the image editor popover",
    );
}

#[test]
fn open_image_fill_popover_paints_selected_image_preview() {
    const PNG_DATA_URL: &str =
        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII=";
    let mut state = image_fill_state_with_url(PNG_DATA_URL);
    state.editor_ui.image_fill_popover_open = true;
    let panel = PropertyPanel::for_selection(&state).expect("image fill panel");
    assert_eq!(
        panel
            .snapshot
            .image_fill
            .as_ref()
            .unwrap()
            .image_url
            .as_deref(),
        Some(PNG_DATA_URL),
    );

    let mut backend = CountingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        let rect = Rect {
            origin: Point2D::new(320.0, 24.0),
            size: Point2D::new(280.0, 900.0),
        };
        panel.paint(&mut cx, rect);
        panel.paint_overlays(&mut cx, rect);
    }
    assert!(
        backend.images.iter().any(|(_, _, bytes)| *bytes > 0),
        "selected image data URL should be decoded and painted in the upload well",
    );
}

#[test]
fn image_fill_body_paints_selected_image_thumbnail_with_mode() {
    const PNG_DATA_URL: &str =
        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII=";
    let mut state = image_fill_state_with_url(PNG_DATA_URL);
    assert!(state.set_selected_image_fill_mode(op_editor_core::ImageFillMode::Tile));
    let panel = PropertyPanel::for_selection(&state).expect("image fill panel");

    let mut backend = CountingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        panel.paint(
            &mut cx,
            Rect {
                origin: Point2D::new(320.0, 24.0),
                size: Point2D::new(280.0, 900.0),
            },
        );
    }

    assert!(
        backend.image_modes.contains(&ImageDrawMode::Tile),
        "fill body thumbnail should paint the selected image using the current image mode",
    );
}

#[test]
fn image_fill_adjustment_reset_label_uses_i18n() {
    let mut state = image_fill_state();
    state.editor_ui.image_fill_popover_open = true;
    state.editor_ui.locale = op_editor_core::Locale::ZhCn;
    assert!(
        state.set_selected_image_adjustment(op_editor_core::ImageAdjustmentField::Exposure, 36.0)
    );
    let panel = PropertyPanel::for_selection(&state).expect("image fill panel");

    let mut backend = CountingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        panel.paint_overlays(
            &mut cx,
            Rect {
                origin: Point2D::new(320.0, 24.0),
                size: Point2D::new(280.0, 900.0),
            },
        );
    }
    assert!(backend.texts.iter().any(|s| s == "重置"));
    assert!(!backend.texts.iter().any(|s| s == "Reset"));
}

#[test]
fn open_image_fill_popover_routes_upload_and_mode_actions() {
    let mut state = image_fill_state();
    state.editor_ui.image_fill_popover_open = true;
    let panel = PropertyPanel::for_selection(&state).expect("image fill panel");
    let rect = Rect {
        origin: Point2D::new(320.0, 24.0),
        size: Point2D::new(280.0, 900.0),
    };
    let popup_rects =
        sections::image_fill_popover_action_rects(rect, visible_for(&panel), &panel.snapshot);
    let upload = popup_rects
        .iter()
        .find(|(a, _)| matches!(a, PropertyPanelAction::PickFillImage))
        .map(|(_, r)| *r)
        .expect("open image popover exposes an upload hit rect");
    let upload_center = Point2D::new(
        upload.origin.x + upload.size.x / 2.0,
        upload.origin.y + upload.size.y / 2.0,
    );
    assert!(
        matches!(
            panel.hit_test_action(rect, upload_center),
            Some(PropertyPanelAction::PickFillImage)
        ),
        "upload well should trigger the image file picker",
    );
    let crop = popup_rects
        .iter()
        .find(|(a, _)| {
            matches!(
                a,
                PropertyPanelAction::SetImageFillMode(op_editor_core::ImageFillMode::Crop)
            )
        })
        .map(|(_, r)| *r)
        .expect("open image popover exposes fit-mode hit rects");
    let crop_center = Point2D::new(
        crop.origin.x + crop.size.x / 2.0,
        crop.origin.y + crop.size.y / 2.0,
    );
    assert!(
        matches!(
            panel.hit_test_action(rect, crop_center),
            Some(PropertyPanelAction::SetImageFillMode(
                op_editor_core::ImageFillMode::Crop
            ))
        ),
        "fit-mode chips should dispatch mode updates",
    );
}

#[test]
fn image_fill_popover_internal_gap_is_consumed_without_action() {
    let mut state = image_fill_state();
    state.editor_ui.image_fill_popover_open = true;
    let panel = PropertyPanel::for_selection(&state).expect("image fill panel");
    let rect = Rect {
        origin: Point2D::new(320.0, 24.0),
        size: Point2D::new(280.0, 900.0),
    };
    let popup_rects =
        sections::image_fill_popover_action_rects(rect, visible_for(&panel), &panel.snapshot);
    let upload = popup_rects
        .iter()
        .find(|(a, _)| matches!(a, PropertyPanelAction::PickFillImage))
        .map(|(_, r)| *r)
        .expect("upload rect exists");
    let gap = Point2D::new(upload.origin.x + 20.0, upload.origin.y - 5.0);

    assert_eq!(panel.hit_test_action(rect, gap), None);
    assert!(
        panel.image_fill_popover_contains(rect, gap),
        "clicks in non-interactive popover gaps must be consumed so the popover stays open",
    );
}
