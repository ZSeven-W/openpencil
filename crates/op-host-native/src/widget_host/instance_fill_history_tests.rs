//! Fill-order and compound instance-history tests split from the broader
//! instance panel suite to keep each source file under the repository limit.

use super::WidgetHostNative;
use jian_ops_schema::node::container::{AlignItems, JustifyContent};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::PenFill;
use op_editor_core::NodeId;

#[test]
fn native_move_fill_action_dispatches_as_one_undoable_edit() {
    let mut host = WidgetHostNative::new();
    let doc = jian_ops_schema::load_str(
        r##"{"version":"0.8.0","children":[{
          "type":"rectangle","id":"rect","name":"Rect",
          "x":0,"y":0,"width":10,"height":10,
          "fill":[
            {"type":"solid","color":"#111111"},
            {"type":"solid","color":"#222222"},
            {"type":"solid","color":"#333333"}
          ]
        }]}"##,
    )
    .expect("fixture parses")
    .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("rect"));

    host.apply_property_action(op_editor_ui::widgets::PropertyPanelAction::MoveFill {
        from: 2,
        to: 0,
    });

    let node = op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new("rect"),
    )
    .expect("rect exists");
    let colors: Vec<_> = op_editor_core::fills::node_fills(node)
        .expect("fills exist")
        .iter()
        .map(|fill| match fill {
            PenFill::Solid(body) => body.color.as_str(),
            other => panic!("expected solid, got {other:?}"),
        })
        .collect();
    assert_eq!(colors, ["#333333", "#111111", "#222222"]);
    assert_eq!(host.editor_state().history.past.len(), 1);
}

#[test]
fn native_instance_move_fill_undo_restores_the_original_ref() {
    let mut host = WidgetHostNative::new();
    let doc = jian_ops_schema::load_str(
        r##"{"version":"0.8.0","children":[
          {"type":"rectangle","id":"master","name":"Master","reusable":true,
           "x":0,"y":0,"width":10,"height":10,
           "fill":[
             {"type":"solid","color":"#111111"},
             {"type":"solid","color":"#222222"},
             {"type":"solid","color":"#333333"}
           ]},
          {"type":"ref","id":"inst","ref":"master","x":20,"y":0}
        ]}"##,
    )
    .expect("fixture parses")
    .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("inst"));

    host.apply_property_action(op_editor_ui::widgets::PropertyPanelAction::MoveFill {
        from: 2,
        to: 0,
    });
    assert_eq!(host.editor_state().history.past.len(), 1);
    assert!(host.editor_state_mut().undo());

    let node = op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new("inst"),
    )
    .expect("instance exists after undo");
    let PenNode::Ref(reference) = node else {
        panic!("undo must restore a Ref, got {node:?}");
    };
    assert!(
        reference.descendants.is_none(),
        "undo must remove the fill-order override"
    );

    assert!(host.editor_state_mut().redo());
    let node = op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new("inst"),
    )
    .expect("instance exists after redo");
    let display = op_editor_core::resolve_instance_display_node(&host.editor_state().doc, node)
        .expect("instance resolves after redo");
    let colors: Vec<_> = op_editor_core::fills::node_fills(&display)
        .expect("display fills")
        .iter()
        .map(|fill| match fill {
            PenFill::Solid(body) => body.color.as_str(),
            other => panic!("expected solid, got {other:?}"),
        })
        .collect();
    assert_eq!(colors, ["#333333", "#111111", "#222222"]);
    assert!(
        !host.editor_state_mut().redo(),
        "single edit has no ghost redo"
    );
}

fn resolved_instance_alignment(
    host: &WidgetHostNative,
    id: &str,
) -> (Option<JustifyContent>, Option<AlignItems>) {
    let node =
        op_editor_core::walkers::find_node(host.editor_state().active_children(), &NodeId::new(id))
            .expect("instance exists");
    let display = op_editor_core::resolve_instance_display_node(&host.editor_state().doc, node)
        .expect("instance resolves");
    let PenNode::Frame(frame) = display else {
        panic!("instance must resolve to a frame");
    };
    (frame.container.justify_content, frame.container.align_items)
}

#[test]
fn native_compound_instance_alignment_undo_redo_preserves_each_history_state() {
    use op_editor_ui::widgets::property_panel::{LayoutAlignValue, LayoutJustifyValue};
    use op_editor_ui::widgets::PropertyPanelAction;

    let mut host = WidgetHostNative::new();
    let doc = jian_ops_schema::load_str(
        r##"{"version":"0.8.0","children":[
          {"type":"frame","id":"master","name":"Master","reusable":true,
           "x":0,"y":0,"width":100,"height":100,"layout":"vertical",
           "justifyContent":"start","alignItems":"start","children":[]},
          {"type":"ref","id":"inst","ref":"master","x":120,"y":0}
        ]}"##,
    )
    .expect("fixture parses")
    .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("inst"));

    host.apply_property_action(PropertyPanelAction::SetLayoutAlignment {
        justify: LayoutJustifyValue::Center,
        align: LayoutAlignValue::End,
    });
    assert_eq!(host.editor_state().history.past.len(), 2);
    assert_eq!(
        resolved_instance_alignment(&host, "inst"),
        (Some(JustifyContent::Center), Some(AlignItems::End))
    );

    assert!(host.editor_state_mut().undo());
    assert_eq!(
        resolved_instance_alignment(&host, "inst"),
        (Some(JustifyContent::Center), Some(AlignItems::Start)),
        "first undo must preserve the first half of the compound action"
    );
    assert!(host.editor_state_mut().undo());
    assert_eq!(
        resolved_instance_alignment(&host, "inst"),
        (Some(JustifyContent::Start), Some(AlignItems::Start))
    );
    assert!(!host.editor_state_mut().undo(), "no ghost undo entry");

    assert!(host.editor_state_mut().redo());
    assert_eq!(
        resolved_instance_alignment(&host, "inst"),
        (Some(JustifyContent::Center), Some(AlignItems::Start))
    );
    assert!(host.editor_state_mut().redo());
    assert_eq!(
        resolved_instance_alignment(&host, "inst"),
        (Some(JustifyContent::Center), Some(AlignItems::End))
    );
    assert!(!host.editor_state_mut().redo(), "no ghost redo entry");
}
