//! Instance inspection / override editing host tests (GAP #10 + #22
//! + #26): property-panel commits on a Ref anchor route through the
//!
//! instance-write redirect, the panel's component lifecycle actions
//! dispatch, context-menu detach rows work, and remote icon inserts
//! bake their SVG `d`.

use super::WidgetHostNative;
use jian_ops_schema::node::PenNode;
use op_editor_core::{NodeId, PenNodeExt};

const COMPONENT_DOC: &str = r##"{
  "version":"0.8.0",
  "children":[
    {"type":"frame","id":"card","name":"Card","reusable":true,"x":0,"y":0,"width":200,"height":100,
     "fill":[{"type":"solid","color":"#222222"}],
     "children":[{"type":"text","id":"title","name":"Title","content":"Hello"}]},
    {"type":"ref","id":"inst1","ref":"card","x":300,"y":50}
  ]
}"##;

fn seeded_host() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    let doc = jian_ops_schema::load_str(COMPONENT_DOC)
        .expect("fixture JSON parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("inst1"));
    host
}

fn ref_node(host: &WidgetHostNative) -> &jian_ops_schema::node::RefNode {
    match op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new("inst1"),
    ) {
        Some(PenNode::Ref(r)) => r,
        other => panic!("inst1 must stay a Ref, got {other:?}"),
    }
}

#[test]
fn fill_hex_commit_on_instance_lands_in_descendants() {
    let mut host = seeded_host();
    host.editor_state_mut().ui.property_focus = Some(op_editor_core::PropertyFocus::FillHex);
    host.editor_state_mut()
        .ui
        .property_input
        .set_text("#ff0000");
    host.commit_property_focus_if_any();
    let over = ref_node(&host)
        .descendants
        .as_ref()
        .and_then(|d| d.get("card"))
        .expect("fill override routed under descendants[card]");
    // The host hex commit path re-cases through `color_to_hex`
    // (uppercase) — compare case-insensitively.
    assert_eq!(
        over.pointer("/fill/0/color")
            .and_then(|v| v.as_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("#ff0000")
    );
}

#[test]
fn position_commit_on_instance_writes_the_ref_base() {
    let mut host = seeded_host();
    host.editor_state_mut().ui.property_focus = Some(op_editor_core::PropertyFocus::PositionX);
    host.editor_state_mut().ui.property_input.set_text("400");
    host.commit_property_focus_if_any();
    let r = ref_node(&host);
    assert_eq!(r.base.x, Some(400.0), "x is an INSTANCE_DIRECT_PROP");
    assert!(r.descendants.is_none(), "no override for a direct prop");
}

#[test]
fn detach_instance_panel_action_materializes_the_ref() {
    let mut host = seeded_host();
    host.apply_property_action(op_editor_ui::widgets::PropertyPanelAction::DetachInstance);
    let anchor = host.editor_state().selection.anchor.clone();
    assert_ne!(anchor.as_str(), "inst1", "detach selects the new subtree");
    let detached =
        op_editor_core::walkers::find_node(host.editor_state().active_children(), &anchor)
            .expect("detached node on the page");
    assert!(
        matches!(detached, PenNode::Frame(_)),
        "the Ref materialized into the component's Frame"
    );
    assert!(
        op_editor_core::walkers::find_node(
            host.editor_state().active_children(),
            &NodeId::new("inst1")
        )
        .is_none(),
        "the Ref slot was replaced"
    );
}

#[test]
fn go_to_component_panel_action_selects_the_master() {
    let mut host = seeded_host();
    host.apply_property_action(op_editor_ui::widgets::PropertyPanelAction::GoToComponent);
    assert_eq!(host.editor_state().selection.anchor.as_str(), "card");
}

#[test]
fn context_menu_swaps_create_component_for_detach_rows() {
    use op_editor_core::editor_ui_state::{LayerContextMenuState, LayerContextTarget};
    use op_editor_ui::widgets::layer_context_menu::{LayerContextAction, LayerContextMenu};
    let host = seeded_host();
    let menu_for = |id: &str| {
        LayerContextMenu::for_state(
            host.editor_state(),
            LayerContextMenuState {
                target: LayerContextTarget::Layer(NodeId::new(id)),
                anchor_x: 0.0,
                anchor_y: 0.0,
                menu: Default::default(),
            },
        )
    };
    // Instance row → Detach Instance replaces Create Component.
    let menu = menu_for("inst1");
    let hits: Vec<_> = (0..10)
        .filter_map(|i| menu.hit_test(op_editor_ui::Point2D::new(4.0, 6.0 + 1.0 + i as f32 * 32.0)))
        .collect();
    assert!(hits.contains(&LayerContextAction::DetachInstance));
    assert!(!hits.contains(&LayerContextAction::CreateComponent));
    // Component row → Detach Component replaces Create Component.
    let menu = menu_for("card");
    let hits: Vec<_> = (0..10)
        .filter_map(|i| menu.hit_test(op_editor_ui::Point2D::new(4.0, 6.0 + 1.0 + i as f32 * 32.0)))
        .collect();
    assert!(hits.contains(&LayerContextAction::DetachComponent));
    assert!(!hits.contains(&LayerContextAction::CreateComponent));
}

#[test]
fn context_menu_detach_instance_dispatch_materializes() {
    use op_editor_core::ui_draft::LayerContextTarget;
    use op_editor_ui::widgets::layer_context_menu::LayerContextAction;
    let mut host = seeded_host();
    host.dispatch_layer_context_action(
        LayerContextAction::DetachInstance,
        LayerContextTarget::Layer(NodeId::new("inst1")),
    );
    assert!(
        op_editor_core::walkers::find_node(
            host.editor_state().active_children(),
            &NodeId::new("inst1")
        )
        .is_none(),
        "context-menu detach replaced the Ref"
    );
}

#[test]
fn context_menu_detach_component_sheds_reusable_flag() {
    use op_editor_core::ui_draft::LayerContextTarget;
    use op_editor_ui::widgets::layer_context_menu::LayerContextAction;
    let mut host = seeded_host();
    host.dispatch_layer_context_action(
        LayerContextAction::DetachComponent,
        LayerContextTarget::Layer(NodeId::new("card")),
    );
    match op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new("card"),
    ) {
        Some(PenNode::Frame(f)) => assert_eq!(f.reusable, None, "reusable flag shed"),
        other => panic!("card must stay a Frame, got {other:?}"),
    }
}

#[test]
fn remote_icon_insert_bakes_svg_d_as_path_node() {
    let mut host = seeded_host();
    let state = host.editor_state_mut();
    state.editor_ui.icon_picker.open = true;
    state.editor_ui.icon_picker_replace_selection = false;
    state.editor_ui.icon_picker_panel_pos = Some((0.0, 0.0));
    state.editor_ui.icon_picker_search = "zwxq".to_string();
    state.editor_ui.icon_picker_remote = op_editor_core::IconPickerRemoteState {
        query: "zwxq".to_string(),
        icons: vec![op_editor_core::IconPickerRemoteIcon {
            collection: "mdi".to_string(),
            name: "zwxq-home".to_string(),
            width: 24.0,
            height: 24.0,
            style: "fill".to_string(),
            d: "M3 9l9-7 9 7v11h-6v-7H9v7H3z".to_string(),
        }],
        loading: false,
        next_start: 1,
        total: 1,
        ..Default::default()
    };

    // Find the remote row's hit point through the panel's own
    // hit-test so the test can't drift from the row math.
    let panel_rect = host
        .icon_picker_panel_rect(1200.0, 800.0)
        .expect("picker open");
    let panel = op_editor_ui::widgets::IconPickerPanel::for_editor(host.editor_state())
        .expect("panel builds");
    let mut hit_point = None;
    let mut y = panel_rect.origin.y + 2.0;
    while y < panel_rect.origin.y + panel_rect.size.y {
        if let Some(op_editor_ui::widgets::IconPickerHit::SelectIcon { collection, name }) = panel
            .hit_test(
                panel_rect,
                op_editor_ui::Point2D::new(panel_rect.origin.x + 4.0, y),
            )
        {
            if collection == "mdi" && name == "zwxq-home" {
                hit_point = Some((panel_rect.origin.x + 4.0, y));
                break;
            }
        }
        y += 2.0;
    }
    let (x, y) = hit_point.expect("remote icon row is hit-testable");
    let before = host.editor_state().active_children().len();
    assert!(host.dispatch_icon_picker_press(x, y, 1200.0, 800.0));

    let children = host.editor_state().active_children();
    assert_eq!(children.len(), before + 1, "insert landed");
    // The icon inserts ABOVE the selected node (`inst1`), so it is no
    // longer the last child — locate the baked path by its icon id.
    let path_idx = children
        .iter()
        .position(|n| matches!(n, PenNode::Path(p) if p.icon_id.as_deref() == Some("mdi:zwxq-home")))
        .expect("baked remote icon path was inserted");
    let inst1_idx = children
        .iter()
        .position(|n| n.id_str() == "inst1")
        .expect("selection still present");
    assert!(
        path_idx < inst1_idx,
        "the inserted icon sits above the selected node"
    );
    let PenNode::Path(p) = &children[path_idx] else {
        unreachable!("path_idx points at the matched Path");
    };
    assert_eq!(
        p.d.as_deref(),
        Some("M3 9l9-7 9 7v11h-6v-7H9v7H3z"),
        "the remote icon's d is baked (no fallback dot)"
    );
    assert_eq!(p.icon_id.as_deref(), Some("mdi:zwxq-home"));
}
