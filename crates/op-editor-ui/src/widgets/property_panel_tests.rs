//! Tests for `widgets::property_panel` — moved to a sibling file to
//! keep `property_panel.rs` under the 800-line cap.
//!
//! Phase 6: the panel builds from `op_editor_core::EditorState`, so
//! the fixtures construct `EditorState` values.

use super::property_panel::{PropertyPanel, PropertyPanelAction};
use super::property_panel_sections as sections;
use super::property_panel_test_support::{state_from, visible_for};
use crate::widgets::{PaintCx, Widget};
use crate::{Color, Point2D, Rect, TextLayout};
use jian_ops_schema::variable::{VariableKind, VariableScalar};
use op_editor_core::{EditorState, NodeId, PropertyTab};

#[derive(Default)]
struct RoundFillBackend {
    fills: Vec<(Rect, Color)>,
}

impl crate::RenderBackend for RoundFillBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, rect: Rect, _: f32, color: Color) {
        self.fills.push((rect, color));
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

fn color_eq(a: Color, b: Color) -> bool {
    (a.r - b.r).abs() < 0.001
        && (a.g - b.g).abs() < 0.001
        && (a.b - b.b).abs() < 0.001
        && (a.a - b.a).abs() < 0.001
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
fn for_selection_code_tab_builds_panel_without_selection() {
    // The Code tab is selection-independent (TS falls back to the active
    // page's children), so the panel must stay alive with no selection.
    let mut state = EditorState::sample();
    state.clear_selection();
    state.editor_ui.property_tab = PropertyTab::Code;
    let panel =
        PropertyPanel::for_selection(&state).expect("Code tab panel survives empty selection");
    assert!(matches!(panel.tab, PropertyTab::Code));
    // The idle node-count label reads the LIVE generation targets — with
    // an empty selection that is every active-page child.
    assert_eq!(
        panel.codegen.selection_snapshot.len(),
        state.active_children().len()
    );
    // Design input rows are never clickable under the Code body.
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 700.0),
    };
    assert!(panel.hit_test(rect, Point2D::new(140.0, 120.0)).is_none());
}

#[test]
fn for_selection_design_tab_still_hides_panel_without_selection() {
    let mut state = EditorState::sample();
    state.clear_selection();
    state.editor_ui.property_tab = PropertyTab::Design;
    assert!(PropertyPanel::for_selection(&state).is_none());
}

#[test]
fn inactive_property_tab_hover_paints_pill_background() {
    let mut state = EditorState::sample();
    state.editor_ui.property_tab = PropertyTab::Code;
    state.editor_ui.property_tab_hover = Some(PropertyTab::Design);
    let panel = PropertyPanel::for_selection(&state).expect("sample doc has a selection");
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 700.0),
    };
    let mut backend = RoundFillBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        panel.paint(&mut cx, rect);
    }

    let muted_pills = backend
        .fills
        .iter()
        .filter(|(_, color)| color_eq(*color, panel.theme.muted))
        .count();
    assert!(
        muted_pills >= 2,
        "active Code tab and hovered inactive Design tab should both paint a visible pill"
    );
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
    let sides_rect =
        sections::editable_input_rects(rect, visible_for(&panel), &panel.snapshot.fills)
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
    let rects = sections::editable_input_rects(rect, visible_for(&panel), &panel.snapshot.fills);
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
fn effects_add_menu_hits_shadow_and_blur_rows() {
    use crate::widgets::EffectAddMenuHit;
    let mut state = EditorState::sample();
    state.set_single_selection(NodeId::new("n10"));
    // Open the add-menu, then rebuild the panel so it reflects the flag.
    state.editor_ui.toggle_effect_add_picker();
    assert!(state.editor_ui.effect_add_picker_open);
    let panel = PropertyPanel::for_selection(&state).expect("frame panel");
    assert!(panel.effect_add_picker_open);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 1600.0),
    };
    let add_rect = panel
        .effect_add_button_rect(panel.scrolled_rect(rect))
        .expect("effects header emits an AddEffect '+' rect");
    let menu = crate::widgets::property_panel_effects::effect_add_menu_rect(add_rect);
    let rows = crate::widgets::property_panel_effects::effect_add_menu_row_rects(menu);
    assert_eq!(rows.len(), 2, "menu has Drop Shadow + Layer Blur rows");
    // Hit-testing each row centre resolves to the matching add action.
    for (expected, row) in rows {
        let center = Point2D::new(
            row.origin.x + row.size.x / 2.0,
            row.origin.y + row.size.y / 2.0,
        );
        assert_eq!(
            panel.effect_add_menu_hit(rect, center),
            EffectAddMenuHit::Row(expected)
        );
    }
    // A click well outside the menu dismisses.
    assert_eq!(
        panel.effect_add_menu_hit(rect, Point2D::new(5.0, 5.0)),
        EffectAddMenuHit::Outside
    );
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
        &panel.snapshot.fills,
        false,
        0,
        false,
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
fn color_variables_add_fill_and_stroke_binding_buttons() {
    let mut state = state_from(
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"rect","name":"Rect",
               "x":40,"y":40,"width":160,"height":100,
               "fill":[{"type":"solid","color":"#ffffff"}],
               "stroke":{"thickness":1,"fill":[{"type":"solid","color":"#374151"}]}}
        ]}"##,
    );
    state.set_single_selection(NodeId::new("rect"));
    assert!(state.create_variable(
        "color-1",
        VariableKind::Color,
        VariableScalar::Str("#000000".into()),
    ));
    let panel = PropertyPanel::for_selection(&state).expect("rectangle panel");
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 1200.0),
    };

    let rects = sections::action_button_rects_with_fill_picker(
        rect,
        visible_for(&panel),
        &panel.snapshot.effects,
        &panel.snapshot.fills,
        false,
        0,
        false,
        false,
        false,
        false,
        false,
    );

    assert!(
        rects.iter().any(|(action, _)| matches!(
            action,
            PropertyPanelAction::ToggleColorVariablePicker(op_editor_core::ColorTarget::Fill)
        )),
        "solid fill row should expose a color-variable picker button"
    );
    assert!(
        rects.iter().any(|(action, _)| matches!(
            action,
            PropertyPanelAction::ToggleColorVariablePicker(op_editor_core::ColorTarget::Stroke)
        )),
        "stroke row should expose a color-variable picker button"
    );
}

#[test]
fn color_variable_picker_emits_bind_and_unbind_rows() {
    let mut state = state_from(
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"rect","name":"Rect",
               "x":40,"y":40,"width":160,"height":100,
               "fill":[{"type":"solid","color":"#ffffff"}],
               "stroke":{"thickness":1,"fill":[{"type":"solid","color":"#374151"}]}}
        ]}"##,
    );
    state.set_single_selection(NodeId::new("rect"));
    assert!(state.create_variable(
        "color-1",
        VariableKind::Color,
        VariableScalar::Str("#000000".into()),
    ));
    state.editor_ui.property_color_variable_picker_open = Some(op_editor_core::ColorTarget::Fill);
    let panel = PropertyPanel::for_selection(&state).expect("rectangle panel");
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 1200.0),
    };
    let rects = sections::action_button_rects_with_fill_picker(
        rect,
        visible_for(&panel),
        &panel.snapshot.effects,
        &panel.snapshot.fills,
        false,
        0,
        false,
        false,
        false,
        false,
        false,
    );
    assert!(
        rects.iter().any(|(action, _)| matches!(
            action,
            PropertyPanelAction::BindColorVariable {
                target: op_editor_core::ColorTarget::Fill,
                index: 0,
            }
        )),
        "open color-variable picker should expose variable rows"
    );

    assert!(state.bind_selected_color_variable(op_editor_core::ColorTarget::Fill, "color-1"));
    let panel = PropertyPanel::for_selection(&state).expect("bound rectangle panel");
    let rects = sections::action_button_rects_with_fill_picker(
        rect,
        visible_for(&panel),
        &panel.snapshot.effects,
        &panel.snapshot.fills,
        false,
        0,
        false,
        false,
        false,
        false,
        false,
    );
    assert!(
        rects.iter().any(|(action, _)| matches!(
            action,
            PropertyPanelAction::UnbindColorVariable(op_editor_core::ColorTarget::Fill)
        )),
        "bound color field should expose an unbind row"
    );
}

#[test]
fn fill_width_hides_the_w_input_but_keeps_h_and_row_height() {
    use op_editor_core::PropertyFocus;
    let fill = {
        let mut s = state_from(
            r##"{ "version": "0.8.0", "children": [
                  {"type":"frame","id":"ff","name":"Frame",
                   "x":40,"y":40,"width":"fill_container","height":240,
                   "layout":"vertical","children":[]}
            ]}"##,
        );
        s.set_single_selection(NodeId::new("ff"));
        PropertyPanel::for_selection(&s).expect("fill-width frame panel")
    };
    assert!(fill.snapshot.size_fill_width, "width sizing should be fill");
    assert!(
        !fill.snapshot.size_fill_height,
        "height stays a concrete number"
    );
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 1200.0),
    };
    let fill_rects = sections::editable_input_rects(rect, visible_for(&fill), &fill.snapshot.fills);
    // W is omitted (fill); H remains (numeric).
    assert!(
        !fill_rects.iter().any(|(f, _)| *f == PropertyFocus::SizeW),
        "SizeW must be hidden when width is fill"
    );
    let fill_h = fill_rects
        .iter()
        .find(|(f, _)| *f == PropertyFocus::SizeH)
        .map(|(_, r)| *r)
        .expect("SizeH must remain");

    // A fixed-width frame keeps both — and SizeH sits at the SAME y, so
    // hiding W never collapses the row / shifts later sections.
    let fixed = {
        let mut s = state_from(
            r##"{ "version": "0.8.0", "children": [
                  {"type":"frame","id":"ff","name":"Frame",
                   "x":40,"y":40,"width":360,"height":240,
                   "layout":"vertical","children":[]}
            ]}"##,
        );
        s.set_single_selection(NodeId::new("ff"));
        PropertyPanel::for_selection(&s).expect("fixed-width frame panel")
    };
    let fixed_rects =
        sections::editable_input_rects(rect, visible_for(&fixed), &fixed.snapshot.fills);
    assert!(
        fixed_rects.iter().any(|(f, _)| *f == PropertyFocus::SizeW),
        "fixed width keeps SizeW"
    );
    let fixed_w = fixed_rects
        .iter()
        .find(|(f, _)| *f == PropertyFocus::SizeW)
        .map(|(_, r)| *r)
        .expect("SizeW present for fixed width");
    let fixed_h = fixed_rects
        .iter()
        .find(|(f, _)| *f == PropertyFocus::SizeH)
        .map(|(_, r)| *r)
        .expect("SizeH present for fixed width");
    assert!(
        (fill_h.origin.y - fixed_h.origin.y).abs() < 0.01,
        "hiding W must not move H's row (row height preserved)"
    );
    // With W hidden, H reflows into the (now-empty) LEFT slot.
    assert!(
        (fill_h.origin.x - fixed_w.origin.x).abs() < 0.01,
        "H must slide into the left slot when W is hidden"
    );
}

#[test]
fn both_dimensions_fill_collapses_the_size_input_row() {
    use op_editor_core::PropertyFocus;
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 1200.0),
    };
    let panel_for = |w: &str, h: &str| {
        let json = format!(
            r##"{{ "version": "0.8.0", "children": [
                  {{"type":"frame","id":"ff","name":"Frame",
                   "x":40,"y":40,"width":{w},"height":{h},
                   "layout":"vertical","children":[]}}
            ]}}"##
        );
        let mut s = state_from(&json);
        s.set_single_selection(NodeId::new("ff"));
        PropertyPanel::for_selection(&s).expect("frame panel")
    };
    // The size checkboxes sit BELOW the W/H input row. When both
    // dimensions are fill, the whole input row collapses, so the first
    // checkbox (填充宽度) shifts up by exactly one input row.
    let chk_y = |p: &PropertyPanel| {
        sections::action_button_rects_with_fill_picker(
            rect,
            visible_for(p),
            &p.snapshot.effects,
            &p.snapshot.fills,
            false,
            0,
            false,
            false,
            false,
            false,
            false,
        )
        .into_iter()
        .find(|(a, _)| matches!(a, PropertyPanelAction::ToggleSizeFillWidth))
        .map(|(_, r)| r.origin.y)
        .expect("fill-width checkbox rect")
    };
    // One dimension numeric (row present) vs both fill (row collapsed).
    let one = panel_for("\"fill_container\"", "240");
    let both = panel_for("\"fill_container\"", "\"fill_container\"");
    assert!(one.snapshot.size_fill_width && both.snapshot.size_fill_height);
    // INPUT_HEIGHT (30) + 10 gap = 40 px of collapse.
    let delta = chk_y(&one) - chk_y(&both);
    assert!(
        (delta - 40.0).abs() < 0.01,
        "both-hidden must collapse the input row (~40px up), got {delta}"
    );
    // Neither W nor H emits a focus rect when both are hidden.
    let both_inputs =
        sections::editable_input_rects(rect, visible_for(&both), &both.snapshot.fills);
    assert!(
        !both_inputs
            .iter()
            .any(|(f, _)| matches!(f, PropertyFocus::SizeW | PropertyFocus::SizeH)),
        "no W/H hit-rect when both dimensions are fill"
    );
}

#[test]
fn padding_mode_derives_from_values_and_drives_input_count() {
    use op_editor_core::{PaddingEditMode, PropertyFocus};
    // from_values mirrors TS parsePaddingValues.
    assert_eq!(
        PaddingEditMode::from_values(10.0, 10.0, 10.0, 10.0),
        PaddingEditMode::Single
    );
    assert_eq!(
        PaddingEditMode::from_values(10.0, 20.0, 10.0, 20.0),
        PaddingEditMode::Axis
    );
    assert_eq!(
        PaddingEditMode::from_values(8.0, 24.0, 32.0, 24.0),
        PaddingEditMode::Individual
    );

    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 1200.0),
    };
    let padding_rects = |padding: &str| {
        let json = format!(
            r##"{{ "version": "0.8.0", "children": [
                  {{"type":"frame","id":"f","name":"F","x":0,"y":0,
                   "width":300,"height":200,"layout":"vertical",
                   "padding":{padding},"children":[]}}
            ]}}"##
        );
        let mut s = state_from(&json);
        s.set_single_selection(NodeId::new("f"));
        let panel = PropertyPanel::for_selection(&s).expect("frame panel");
        sections::editable_input_rects(rect, visible_for(&panel), &panel.snapshot.fills)
            .into_iter()
            .filter(|(f, _)| {
                matches!(
                    f,
                    PropertyFocus::PaddingTop
                        | PropertyFocus::PaddingRight
                        | PropertyFocus::PaddingBottom
                        | PropertyFocus::PaddingLeft
                )
            })
            .count()
    };
    // Single → 1 input, Axis → 2, Individual → 4.
    assert_eq!(padding_rects("12"), 1, "uniform padding → 1 input");
    assert_eq!(padding_rects("[10, 20]"), 2, "axis padding → 2 inputs");
    assert_eq!(
        padding_rects("[8, 24, 32, 24]"),
        4,
        "individual padding → 4 inputs"
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
        &panel.snapshot.fills,
        false,
        0,
        false,
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
    let padding_top = sections::editable_input_rects(rect, visible, &panel.snapshot.fills)
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
    state.editor_ui.font_picker.open = true;
    // Type-ahead narrows the overlay to one row (TS search filter) —
    // "geor" leaves only the fallback-system "Georgia".
    state.editor_ui.font_picker_search = "geor".to_string();
    let panel = PropertyPanel::for_selection(&state).expect("text panel");
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 1200.0),
    };
    let entries = panel.font_picker_entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].family, "Georgia");
    let layout = super::property_panel_typography::font_picker_layout(
        rect,
        panel.visible_sections_for_test(),
        &entries,
        panel.font_import_supported,
        0.0,
    )
    .expect("picker layout");
    let georgia = layout
        .rows
        .iter()
        .find_map(|(row, r)| {
            matches!(
                row,
                super::property_panel_typography::FontPickerRow::Entry(0)
            )
            .then_some(*r)
        })
        .expect("Georgia font row");
    let center = Point2D::new(
        georgia.origin.x + georgia.size.x / 2.0,
        georgia.origin.y + georgia.size.y / 2.0,
    );
    assert!(matches!(
        panel.hit_test_action(rect, center),
        Some(PropertyPanelAction::SetFontFamilyIndex(0))
    ));
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
        &panel.snapshot.fills,
        false,
        0,
        false,
        false,
        true,
        false,
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
fn no_stroke_swatch_defaults_to_slate_not_black() {
    // Regression: clicking the stroke hex used to seed #000000 while the
    // swatch painted slate. Paint and the edit-seed now read ONE source
    // (`stroke_swatch_color`), whose no-stroke default is `#374151`.
    use crate::widgets::property_panel_inputs::format_color_hex;
    use crate::widgets::property_panel_snapshot::NodeSnapshot;
    let hex = format_color_hex(NodeSnapshot::DEFAULT_STROKE_SWATCH);
    assert_eq!(hex, "#374151");
    assert_ne!(hex, "#000000");
}

// ④ fit-content hover-wash tests (`action_wash_rect`) live in the sibling
// `property_panel_wash_tests.rs` to keep this file under the 800-line cap.
