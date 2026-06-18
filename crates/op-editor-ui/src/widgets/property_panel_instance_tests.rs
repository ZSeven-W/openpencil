//! Instance inspection tests (GAP #10): a selected `Ref` builds its
//! panel snapshot from the merged display node so the FULL section
//! set shows, carries the `is_instance` flag for badging, and emits
//! the Go-to-component / Detach-instance action rects. Plus the
//! layer-panel component/instance badging flags.

use super::property_panel::{PropertyPanel, PropertyPanelAction};
use super::property_panel_test_support::state_from;
use crate::widgets::layer_panel::LayerPanel;
use crate::widgets::property_panel_visibility::ComponentButtonState;
use crate::{Point2D, Rect};
use op_editor_core::NodeId;

const COMPONENT_DOC: &str = r##"{
  "version":"0.8.0",
  "children":[
    {"type":"frame","id":"card","name":"Card","reusable":true,"x":0,"y":0,"width":200,"height":100,
     "fill":[{"type":"solid","color":"#222222"}],
     "children":[{"type":"text","id":"title","name":"Title","content":"Hello"}]},
    {"type":"ref","id":"inst1","ref":"card","x":300,"y":50,
     "descendants":{"card":{"fill":[{"type":"solid","color":"#ff8800"}]}}}
  ]
}"##;

fn panel_for(id: &str) -> PropertyPanel {
    let mut state = state_from(COMPONENT_DOC);
    state.set_single_selection(NodeId::new(id));
    PropertyPanel::for_selection(&state).expect("panel builds")
}

#[test]
fn instance_snapshot_merges_display_node_and_flags_instance() {
    let panel = panel_for("inst1");
    let snap = &panel.snapshot;
    assert!(snap.is_instance, "Ref selection flags is_instance");
    assert!(!snap.is_reusable);
    // Merged display node: component geometry + instance position.
    assert_eq!(snap.x, 300);
    assert_eq!(snap.width, 200);
    // descendants[card] top-level fill override shows in the panel.
    let fill = snap.fill.expect("merged fill present");
    assert!(fill.r > 0.9 && fill.g > 0.4 && fill.b < 0.1, "#ff8800");
    // The merged node takes the component's Frame kind — the FULL
    // section set (fill / stroke / layout) is exposed, not the old
    // near-empty `Other("ref")` mask.
    assert!(matches!(
        snap.kind_variant,
        crate::layout_scene::NodeKind::Frame
    ));
}

#[test]
fn reusable_component_snapshot_flags_is_reusable() {
    let panel = panel_for("card");
    assert!(panel.snapshot.is_reusable);
    assert!(!panel.snapshot.is_instance);
}

#[test]
fn registered_group_component_snapshot_flags_is_reusable() {
    let mut state = state_from(
        r##"{
          "version":"0.8.0",
          "children":[
            {"type":"group","id":"text_group","name":"Text Group",
             "children":[{"type":"text","id":"label","name":"Label","content":"Hello"}]}
          ]
        }"##,
    );
    state.set_single_selection(NodeId::new("text_group"));
    assert!(state.create_component_from_node_name(&NodeId::new("text_group")));

    let panel = PropertyPanel::for_selection(&state).expect("group panel builds");
    assert!(panel.snapshot.is_reusable);
    assert!(!panel.snapshot.is_instance);
    assert_eq!(
        panel.visible_sections().component_button,
        ComponentButtonState::DetachComponent
    );
}

#[test]
fn instance_panel_emits_go_to_component_and_detach_rows() {
    let panel = panel_for("inst1");
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 2000.0),
    };
    let actions: Vec<PropertyPanelAction> = super::property_panel_layout::action_button_rects(
        rect,
        super::property_panel_test_support::visible_for(&panel),
        &panel.snapshot.effects,
    )
    .into_iter()
    .map(|(a, _)| a)
    .collect();
    assert!(actions.contains(&PropertyPanelAction::GoToComponent));
    assert!(actions.contains(&PropertyPanelAction::DetachInstance));
    assert!(!actions.contains(&PropertyPanelAction::CreateComponent));
}

#[test]
fn reusable_panel_emits_detach_component_action() {
    let panel = panel_for("card");
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 2000.0),
    };
    let actions: Vec<PropertyPanelAction> = super::property_panel_layout::action_button_rects(
        rect,
        super::property_panel_test_support::visible_for(&panel),
        &panel.snapshot.effects,
    )
    .into_iter()
    .map(|(a, _)| a)
    .collect();
    assert!(actions.contains(&PropertyPanelAction::DetachComponent));
    assert!(!actions.contains(&PropertyPanelAction::CreateComponent));
}

#[test]
fn instance_block_height_matches_two_rows() {
    use crate::widgets::property_panel_inputs::{
        create_component_block_height, CREATE_COMPONENT_BLOCK_H, CREATE_COMPONENT_BTN_H,
        CREATE_COMPONENT_ROW_GAP,
    };
    assert_eq!(
        create_component_block_height(ComponentButtonState::Create),
        CREATE_COMPONENT_BLOCK_H
    );
    assert_eq!(
        create_component_block_height(ComponentButtonState::Instance),
        CREATE_COMPONENT_BLOCK_H + CREATE_COMPONENT_BTN_H + CREATE_COMPONENT_ROW_GAP
    );
}

#[test]
fn layer_panel_rows_flag_components_and_instances() {
    let state = state_from(COMPONENT_DOC);
    let panel = LayerPanel::from_editor(&state);
    let card = panel
        .items
        .iter()
        .find(|i| i.node_id.as_str() == "card")
        .expect("card row");
    assert!(card.is_reusable && !card.is_instance);
    assert!(matches!(card.icon, crate::widgets::icons::Icon::Diamond));
    let inst = panel
        .items
        .iter()
        .find(|i| i.node_id.as_str() == "inst1")
        .expect("instance row");
    assert!(inst.is_instance && !inst.is_reusable);
    assert!(matches!(inst.icon, crate::widgets::icons::Icon::Diamond));
}
