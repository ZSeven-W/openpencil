//! Floating-menu + colour-variable tests — the Effects "+" add-menu
//! (hit routing and paint ordering over the row it covers), the
//! Export dropdown toggles, and the fill/stroke variable bindings.
//!
//! Split out of `property_panel_tests.rs` to keep both files under
//! the openpencil 800-line cap.

use super::{color_eq, RoundFillBackend};
use crate::widgets::property_panel::{PropertyPanel, PropertyPanelAction};
use crate::widgets::property_panel_color_variables::ColorVariableRow;
use crate::widgets::property_panel_sections as sections;
use crate::widgets::property_panel_test_support::{state_from, visible_for};
use crate::widgets::{PaintCx, Widget};
use crate::{Point2D, Rect};
use jian_ops_schema::variable::{VariableKind, VariableScalar};
use op_editor_core::{EditorState, NodeId};

#[test]
fn effects_add_menu_hits_all_three_effect_kinds() {
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
    assert_eq!(
        rows.len(),
        3,
        "menu has Shadow + Layer Blur + Background Blur rows"
    );
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
fn effects_add_menu_owns_feedback_above_the_covered_interaction_row() {
    use crate::widgets::EffectAddMenuHit;

    let mut state = EditorState::sample();
    state.set_single_selection(NodeId::new("n10"));
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 1600.0),
    };

    let closed = PropertyPanel::for_selection(&state).expect("frame panel");
    let action_rects = sections::action_button_rects_with_fill_picker(
        rect,
        visible_for(&closed),
        &closed.snapshot.effects,
        &closed.snapshot.fills,
        &closed.snapshot.interactions,
        false,
        0,
        false,
        false,
        false,
        false,
        false,
    );
    let (interaction_index, interaction_rect) = action_rects
        .iter()
        .enumerate()
        .find_map(|(index, (action, rect))| {
            matches!(action, PropertyPanelAction::ToggleInteractionMenu(_))
                .then_some((index, *rect))
        })
        .expect("frame panel has an Add interaction action");

    // Reproduce the stale lower hover that existed when the Effects menu was
    // opened while the cursor crossed into its overlap with Interactions.
    state.editor_ui.property_action_hover = Some(interaction_index);
    state.editor_ui.toggle_effect_add_picker();
    let mut panel = PropertyPanel::for_selection(&state).expect("open effects menu panel");
    assert_eq!(
        panel.action_hover, None,
        "an owning popup must not carry a body-action hover into paint"
    );

    let menu = panel
        .effect_add_menu_rect(rect)
        .expect("open effects menu bounds");
    let rows = crate::widgets::property_panel_effects::effect_add_menu_row_rects(menu);
    let (popup_action, overlap_point) = rows
        .iter()
        .find_map(|(action, popup_row)| {
            let left = popup_row.origin.x.max(interaction_rect.origin.x);
            let top = popup_row.origin.y.max(interaction_rect.origin.y);
            let right = (popup_row.origin.x + popup_row.size.x)
                .min(interaction_rect.origin.x + interaction_rect.size.x);
            let bottom = (popup_row.origin.y + popup_row.size.y)
                .min(interaction_rect.origin.y + interaction_rect.size.y);
            (right > left && bottom > top).then_some((
                action.clone(),
                Point2D::new((left + right) / 2.0, (top + bottom) / 2.0),
            ))
        })
        .expect("downward Effects menu overlaps Add interaction");

    assert!(matches!(
        panel.effect_add_menu_hit(rect, overlap_point),
        EffectAddMenuHit::Row(_)
    ));
    assert!(panel.effect_add_menu_contains(rect, overlap_point));
    assert_eq!(
        panel.hit_test_action(rect, overlap_point),
        Some(popup_action),
        "popup hit testing must win over the covered body action"
    );

    // Even if a caller injects a stale body hover into the immutable panel,
    // popup chrome is painted later and therefore remains visually top-most.
    panel.action_hover = Some(interaction_index);
    let theme = panel.theme;
    let mut backend = RoundFillBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        panel.paint(&mut cx, rect);
    }
    let body_hover_paint = backend
        .fills
        .iter()
        .position(|(painted, color)| {
            *painted == interaction_rect && color_eq(*color, theme.button_hover)
        })
        .expect("injected body hover paints its feedback wash");
    let popup_background_paint = backend
        .fills
        .iter()
        .position(|(painted, color)| *painted == menu && color_eq(*color, theme.popover))
        .expect("effects popup paints its background");
    assert!(
        body_hover_paint < popup_background_paint,
        "popup background must composite after the covered body hover"
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
        &panel.snapshot.interactions,
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
        r##"{ "version": "1.0.0", "children": [
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
        &panel.snapshot.interactions,
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
        r##"{ "version": "1.0.0", "children": [
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
    let layout = panel
        .color_variable_picker_layout(rect)
        .expect("open picker should lay out");
    let unbound_rows = layout.rows.clone();
    assert_eq!(
        layout.row_at(row_center(&unbound_rows, 0)),
        Some(ColorVariableRow::Variable(0)),
        "open color-variable picker should expose variable rows"
    );
    assert!(
        matches!(
            panel.color_variable_picker_action_at(rect, row_center(&unbound_rows, 0)),
            Some(PropertyPanelAction::BindColorVariable {
                target: op_editor_core::ColorTarget::Fill,
                index: 0,
            })
        ),
        "clicking a variable row should still bind that variable"
    );

    assert!(state.bind_selected_color_variable(op_editor_core::ColorTarget::Fill, "color-1"));
    let panel = PropertyPanel::for_selection(&state).expect("bound rectangle panel");
    let layout = panel
        .color_variable_picker_layout(rect)
        .expect("open picker should lay out");
    assert_eq!(
        layout.rows.first().map(|(row, _)| *row),
        Some(ColorVariableRow::Unbind),
        "bound color field should expose an unbind row"
    );
    assert!(
        matches!(
            panel.color_variable_picker_action_at(rect, row_center(&layout.rows, 0)),
            Some(PropertyPanelAction::UnbindColorVariable(
                op_editor_core::ColorTarget::Fill
            ))
        ),
        "clicking the unbind row should still unbind"
    );
}

fn row_center(rows: &[(ColorVariableRow, Rect)], index: usize) -> Point2D {
    let (_, rect) = rows[index];
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}
