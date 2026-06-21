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

/// The Widget section is an experimental surface, hidden unless opted in.
/// These tests exercise that section, so build state with the gate ON.
fn widget_state(src: &str) -> op_editor_core::EditorState {
    let mut state = state_from(src);
    state.editor_ui.agent_settings.experimental_features_enabled = true;
    state
}

#[test]
fn widget_section_hidden_unless_experimental_enabled() {
    let src = r##"{ "version": "0.8.0", "children": [
              {"type":"text_input","id":"email","name":"Email",
               "x":24,"y":32,"width":220,"height":40,"placeholder":"Email"}
        ]}"##;

    // Default (gate off): the Widget section is suppressed.
    let mut off = state_from(src);
    off.set_single_selection(NodeId::new("email"));
    let panel = PropertyPanel::for_selection(&off).expect("panel");
    assert!(
        panel.snapshot.widget.is_none(),
        "Widget section must be hidden when experimental features are off"
    );

    // Gate on: the Widget summary is populated.
    let mut on = widget_state(src);
    on.set_single_selection(NodeId::new("email"));
    let panel = PropertyPanel::for_selection(&on).expect("panel");
    assert!(
        panel.snapshot.widget.is_some(),
        "Widget section must appear when experimental features are on"
    );
}

#[test]
fn text_input_selection_exposes_widget_text_rows() {
    let mut state = widget_state(
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
    let focuses: Vec<_> =
        sections::editable_input_rects(panel_rect(), visible, &panel.snapshot.fills)
        .into_iter()
        .map(|(focus, _)| focus)
        .collect();
    assert!(focuses.contains(&PropertyFocus::WidgetPlaceholder));
    assert!(focuses.contains(&PropertyFocus::WidgetValue));
}

#[test]
fn property_panel_text_input_fields_expose_icon_and_bind_rows() {
    let mut state = widget_state(
        r##"{ "version": "0.8.0", "children": [
              {"type":"text_input","id":"email","name":"Email",
               "x":24,"y":32,"width":220,"height":40,
               "leadingIcon":"mail","trailingIcon":"eye",
               "placeholder":"Email address",
               "bindings":{"bind:value":"$state.email"}}
        ]}"##,
    );
    state.set_single_selection(NodeId::new("email"));
    let panel = PropertyPanel::for_selection(&state).expect("text input panel");
    let widget = panel.snapshot.widget.as_ref().expect("widget summary");

    // The snapshot surfaces the current icon names + bind key (prefix
    // stripped) so the rows paint the live values.
    assert_eq!(widget.leading_icon, "mail");
    assert_eq!(widget.trailing_icon, "eye");
    assert_eq!(widget.bind_key, "email");

    let visible = visible_for(&panel);
    assert_eq!(visible.widget, Some(WidgetKind::TextInput));
    let focuses: Vec<_> =
        sections::editable_input_rects(panel_rect(), visible, &panel.snapshot.fills)
        .into_iter()
        .map(|(focus, _)| focus)
        .collect();
    assert!(focuses.contains(&PropertyFocus::WidgetLeadingIcon));
    assert!(focuses.contains(&PropertyFocus::WidgetTrailingIcon));
    assert!(focuses.contains(&PropertyFocus::WidgetBindKey));
}

#[test]
fn checkbox_selection_hides_icon_and_bind_rows() {
    // Icon + bind editing is Phase-1 scoped to the input kinds; a
    // Checkbox widget shows neither.
    let mut state = widget_state(
        r##"{ "version": "0.8.0", "children": [
              {"type":"checkbox","id":"cb","name":"Agree","x":0,"y":0,"width":18,"height":18,
               "label":"Accept","checked":false}
        ]}"##,
    );
    state.set_single_selection(NodeId::new("cb"));
    let panel = PropertyPanel::for_selection(&state).expect("checkbox panel");
    let visible = visible_for(&panel);
    let focuses: Vec<_> =
        sections::editable_input_rects(panel_rect(), visible, &panel.snapshot.fills)
        .into_iter()
        .map(|(focus, _)| focus)
        .collect();
    assert!(!focuses.contains(&PropertyFocus::WidgetLeadingIcon));
    assert!(!focuses.contains(&PropertyFocus::WidgetBindKey));
}

#[test]
fn slider_selection_exposes_widget_range_rows() {
    let mut state = widget_state(
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
    let focuses: Vec<_> =
        sections::editable_input_rects(panel_rect(), visible, &panel.snapshot.fills)
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
    let mut state = widget_state(
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
    let focuses: Vec<_> =
        sections::editable_input_rects(panel_rect(), visible, &panel.snapshot.fills)
        .into_iter()
        .map(|(focus, _)| focus)
        .collect();
    assert!(!focuses.contains(&PropertyFocus::WidgetPlaceholder));
    assert!(!focuses.contains(&PropertyFocus::WidgetMin));
}

#[test]
fn checkbox_selection_emits_toggle_checked_action() {
    use super::property_panel::PropertyPanelAction;
    let mut state = widget_state(
        r##"{ "version": "0.8.0", "children": [
              {"type":"checkbox","id":"cb","name":"Agree","x":0,"y":0,"width":18,"height":18,
               "label":"Accept","checked":false}
        ]}"##,
    );
    state.set_single_selection(NodeId::new("cb"));
    let panel = PropertyPanel::for_selection(&state).expect("checkbox panel");
    let actions: Vec<PropertyPanelAction> = sections::action_button_rects(
        panel_rect(),
        visible_for(&panel),
        &panel.snapshot.effects,
        &panel.snapshot.fills,
    )
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
    let mut state = widget_state(
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
    let mut state = widget_state(
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
