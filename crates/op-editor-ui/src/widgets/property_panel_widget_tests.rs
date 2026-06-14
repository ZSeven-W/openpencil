//! Focused tests for the form-widget section in the property panel.

use super::property_panel::{PropertyPanel, WidgetKind};
use super::property_panel_sections as sections;
use super::property_panel_test_support::{state_from, visible_for};
use crate::{Point2D, Rect};
use op_editor_core::{NodeId, PropertyFocus};

fn panel_rect() -> Rect {
    Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 1200.0),
    }
}

#[test]
fn text_input_selection_exposes_widget_text_rows() {
    let mut state = state_from(
        r##"{ "version": "0.8.0", "children": [
              {"type":"text_input","id":"email","name":"Email",
               "x":24,"y":32,"width":220,"height":40,
               "placeholder":"Email address","value":"kai@example.com"}
        ]}"##,
    );
    state.set_single_selection(NodeId::new("email"));
    let panel = PropertyPanel::for_selection(&state).expect("text input panel");
    let widget = panel.snapshot.widget.as_ref().expect("widget summary");

    assert_eq!(widget.kind, WidgetKind::TextInput);
    assert_eq!(widget.placeholder, "Email address");
    assert_eq!(widget.value, "kai@example.com");

    let visible = visible_for(&panel);
    assert_eq!(visible.widget, Some(WidgetKind::TextInput));
    let focuses: Vec<_> = sections::editable_input_rects(panel_rect(), visible)
        .into_iter()
        .map(|(focus, _)| focus)
        .collect();
    assert!(focuses.contains(&PropertyFocus::WidgetPlaceholder));
    assert!(focuses.contains(&PropertyFocus::WidgetValue));
}

#[test]
fn slider_selection_exposes_widget_range_rows() {
    let mut state = state_from(
        r##"{ "version": "0.8.0", "children": [
              {"type":"slider","id":"volume","name":"Volume",
               "x":24,"y":32,"width":220,"height":24,
               "min":0,"max":100,"step":5,"value":50}
        ]}"##,
    );
    state.set_single_selection(NodeId::new("volume"));
    let panel = PropertyPanel::for_selection(&state).expect("slider panel");
    let widget = panel.snapshot.widget.as_ref().expect("widget summary");

    assert_eq!(widget.kind, WidgetKind::Slider);
    assert_eq!(widget.min, "0");
    assert_eq!(widget.max, "100");
    assert_eq!(widget.step, "5");

    let visible = visible_for(&panel);
    assert_eq!(visible.widget, Some(WidgetKind::Slider));
    let focuses: Vec<_> = sections::editable_input_rects(panel_rect(), visible)
        .into_iter()
        .map(|(focus, _)| focus)
        .collect();
    assert!(focuses.contains(&PropertyFocus::WidgetMin));
    assert!(focuses.contains(&PropertyFocus::WidgetMax));
    assert!(focuses.contains(&PropertyFocus::WidgetStep));
}

// (a) capability gating — a non-widget kind hides the section. ----------------

#[test]
fn frame_selection_hides_widget_section() {
    let mut state = state_from(
        r##"{ "version": "0.8.0", "children": [
              {"type":"frame","id":"f1","name":"Frame","x":0,"y":0,"width":200,"height":100}
        ]}"##,
    );
    state.set_single_selection(NodeId::new("f1"));
    let panel = PropertyPanel::for_selection(&state).expect("frame panel");
    assert!(panel.snapshot.widget.is_none(), "frame is not a widget");
    let visible = visible_for(&panel);
    assert!(visible.widget.is_none(), "frame hides the Widget section");
    // No widget focuses leak into the input walker.
    let focuses: Vec<_> = sections::editable_input_rects(panel_rect(), visible)
        .into_iter()
        .map(|(focus, _)| focus)
        .collect();
    assert!(!focuses.contains(&PropertyFocus::WidgetPlaceholder));
    assert!(!focuses.contains(&PropertyFocus::WidgetMin));
}

#[test]
fn checkbox_selection_emits_toggle_checked_action() {
    use super::property_panel::PropertyPanelAction;
    let mut state = state_from(
        r##"{ "version": "0.8.0", "children": [
              {"type":"checkbox","id":"cb","name":"Agree","x":0,"y":0,"width":18,"height":18,
               "label":"Accept","checked":false}
        ]}"##,
    );
    state.set_single_selection(NodeId::new("cb"));
    let panel = PropertyPanel::for_selection(&state).expect("checkbox panel");
    let actions: Vec<PropertyPanelAction> =
        sections::action_button_rects(panel_rect(), visible_for(&panel), &panel.snapshot.effects)
            .into_iter()
            .map(|(a, _)| a)
            .collect();
    // Current `checked` is false → the toggle action carries `true`.
    assert!(actions.contains(&PropertyPanelAction::ToggleWidgetChecked(true)));
}

// (b) field commit — editing placeholder writes the node. ---------------------

#[test]
fn committing_placeholder_updates_text_input_node() {
    use jian_ops_schema::node::PenNode;
    let mut state = state_from(
        r##"{ "version": "0.8.0", "children": [
              {"type":"text_input","id":"email","name":"Email",
               "x":0,"y":0,"width":220,"height":40,"placeholder":"Email"}
        ]}"##,
    );
    state.set_single_selection(NodeId::new("email"));
    // The host commit path routes a focused WidgetPlaceholder draft
    // through this op-editor-core mutator; exercise it end-to-end.
    assert!(
        state.set_selected_widget_text(op_editor_core::WidgetTextField::Placeholder, "Your email")
    );
    let PenNode::TextInput(ti) = &state.active_children()[0] else {
        panic!("expected a TextInput node");
    };
    assert_eq!(ti.placeholder.as_deref(), Some("Your email"));
}

#[test]
fn committing_slider_max_updates_node() {
    use jian_ops_schema::node::PenNode;
    let mut state = state_from(
        r##"{ "version": "0.8.0", "children": [
              {"type":"slider","id":"sl","name":"Vol","x":0,"y":0,"width":220,"height":24,
               "min":0,"max":100,"step":1,"value":50}
        ]}"##,
    );
    state.set_single_selection(NodeId::new("sl"));
    assert!(state.commit_property_edit(PropertyFocus::WidgetMax, 250.0));
    let PenNode::Slider(sl) = &state.active_children()[0] else {
        panic!("expected a Slider node");
    };
    assert_eq!(sl.max, Some(250.0));
}
