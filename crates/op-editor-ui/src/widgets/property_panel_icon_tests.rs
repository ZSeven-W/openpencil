//! Focused tests for icon/size behavior that is easier to keep
//! separate from the broad property-panel snapshot suite.

use super::property_panel::{PropertyPanel, PropertyPanelAction, SectionCapabilities};
use super::property_panel_sections as sections;
use crate::{Point2D, Rect};
use op_editor_core::{EditorState, NodeId};

fn state_from(src: &str) -> EditorState {
    let doc = jian_ops_schema::load_str(src)
        .expect("property-panel fixture parses")
        .value;
    EditorState::from_document(doc)
}

fn visible_for(panel: &PropertyPanel) -> sections::VisibleSections {
    let caps = SectionCapabilities::for_kind(&panel.snapshot.kind_variant);
    sections::VisibleSections {
        create_component: caps.create_component && panel.snapshot.can_create_component,
        component_button: crate::widgets::property_panel_visibility::ComponentButtonState::Create,
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

#[test]
fn text_size_section_does_not_emit_clip_content_action() {
    let mut state = EditorState::sample();
    state.set_single_selection(NodeId::new("n11"));
    let panel = PropertyPanel::for_selection(&state).expect("text panel");
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 1200.0),
    };

    let actions = sections::action_button_rects_with_fill_picker(
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
        actions
            .iter()
            .all(|(action, _)| !matches!(action, PropertyPanelAction::ToggleSizeClipContent)),
        "Text nodes do not support clipContent, so the panel must not expose a dead checkbox"
    );
}

#[test]
fn icon_font_selection_exposes_icon_picker_action() {
    let mut state = state_from(
        r##"{ "version": "0.8.0", "children": [
              {"type":"icon_font","id":"icon","name":"Search",
               "x":40,"y":40,"width":24,"height":24,
               "iconFontName":"search","iconFontFamily":"lucide"}
        ]}"##,
    );
    state.set_single_selection(NodeId::new("icon"));
    let panel = PropertyPanel::for_selection(&state).expect("icon panel");
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 1200.0),
    };

    let action_rect = sections::action_button_rects_with_fill_picker(
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
    )
    .into_iter()
    .find(|(action, _)| matches!(action, PropertyPanelAction::OpenSelectedIconPicker))
    .map(|(_, r)| r)
    .expect("icon section emits picker action");
    let center = Point2D::new(
        action_rect.origin.x + action_rect.size.x / 2.0,
        action_rect.origin.y + action_rect.size.y / 2.0,
    );

    assert_eq!(
        panel.hit_test_action(rect, center),
        Some(PropertyPanelAction::OpenSelectedIconPicker)
    );
}
