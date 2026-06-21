use crate::widgets::property_panel::{PropertyPanel, PropertyPanelAction};
use crate::widgets::property_panel_sections as sections;
use crate::widgets::property_panel_test_support::{state_from, visible_for};
use crate::{Point2D, Rect};
use op_editor_core::{NodeId, PropertyFocus};

fn panel_rect() -> Rect {
    Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 1200.0),
    }
}

fn sided_stroke_state() -> op_editor_core::EditorState {
    let mut state = state_from(
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"rect","name":"Rect",
               "x":40,"y":40,"width":160,"height":100,
               "fill":[{"type":"solid","color":"#ffffff"}],
               "stroke":{"thickness":{"top":1,"right":2,"bottom":0,"left":4},
                         "fill":[{"type":"solid","color":"#374151"}]}}
        ]}"##,
    );
    state.set_single_selection(NodeId::new("rect"));
    state
}

#[test]
fn sided_stroke_snapshot_preserves_edge_widths() {
    let state = sided_stroke_state();
    let panel = PropertyPanel::for_selection(&state).expect("rectangle panel");
    let stroke = panel.snapshot.stroke.expect("stroke");

    assert_eq!(stroke.width, 4.0);
    assert_eq!(stroke.sides, Some([1.0, 2.0, 0.0, 4.0]));
}

#[test]
fn stroke_section_emits_edge_width_inputs() {
    let state = sided_stroke_state();
    let panel = PropertyPanel::for_selection(&state).expect("rectangle panel");

    let focuses: Vec<_> =
        sections::editable_input_rects(panel_rect(), visible_for(&panel), &panel.snapshot.fills)
            .into_iter()
            .map(|(focus, _)| focus)
            .collect();

    assert!(focuses.contains(&PropertyFocus::StrokeTopWidth));
    assert!(focuses.contains(&PropertyFocus::StrokeRightWidth));
    assert!(focuses.contains(&PropertyFocus::StrokeBottomWidth));
    assert!(focuses.contains(&PropertyFocus::StrokeLeftWidth));
}

#[test]
fn stroke_mode_popover_emits_mode_actions() {
    let mut state = sided_stroke_state();
    state.editor_ui.stroke_mode_popover_open = true;
    let panel = PropertyPanel::for_selection(&state).expect("rectangle panel");

    let actions: Vec<_> = sections::action_button_rects_with_fill_picker(
        panel_rect(),
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
    .map(|(action, _)| action)
    .collect();

    assert!(actions.contains(&PropertyPanelAction::ToggleStrokeModePopover));
    assert!(actions.contains(&PropertyPanelAction::SetStrokeMode(
        op_editor_core::PaddingEditMode::Single
    )));
    assert!(actions.contains(&PropertyPanelAction::SetStrokeMode(
        op_editor_core::PaddingEditMode::Axis
    )));
    assert!(actions.contains(&PropertyPanelAction::SetStrokeMode(
        op_editor_core::PaddingEditMode::Individual
    )));
}
