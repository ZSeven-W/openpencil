#![cfg(test)]

use crate::fills::node_stroke_width;
use crate::node_id::NodeId;
use crate::walkers::find_node;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::{PenFill, PenStroke, SidedThickness, StrokeThickness};

fn three_fill_state() -> crate::EditorState {
    let src = r##"{
      "version":"1.0.0",
      "children":[{
        "type":"rectangle","id":"n1","name":"R",
        "x":0,"y":0,"width":10,"height":10,
        "fill":[
          {"type":"solid","color":"#111111"},
          {"type":"solid","color":"#222222"},
          {"type":"solid","color":"#333333"}
        ]
      }]
    }"##;
    let doc = jian_ops_schema::load_str(src)
        .expect("three-fill fixture parses")
        .value;
    let mut state = crate::EditorState::from_document(doc);
    state.set_single_selection(NodeId::new("n1"));
    state
}

fn selected_fill_colors(state: &crate::EditorState) -> Vec<String> {
    let node = find_node(state.active_children(), &NodeId::new("n1")).expect("n1 exists");
    crate::fills::node_fills(node)
        .expect("rectangle has fills")
        .iter()
        .map(|fill| match fill {
            PenFill::Solid(body) => body.color.clone(),
            other => panic!("fixture fill must stay solid, got {other:?}"),
        })
        .collect()
}

#[test]
fn move_fill_primitive_moves_two_to_zero() {
    let mut state = three_fill_state();
    let node = crate::walkers::find_node_mut(state.active_children_mut(), &NodeId::new("n1"))
        .expect("n1 exists");

    assert!(crate::move_fill(node, 2, 0));
    assert_eq!(
        selected_fill_colors(&state),
        ["#333333", "#111111", "#222222"]
    );
}

#[test]
fn move_fill_primitive_rejects_invalid_and_same_indices_without_mutating() {
    for (from, to) in [(0, 0), (3, 0), (0, 3)] {
        let mut state = three_fill_state();
        let before = serde_json::to_value(&state.doc).expect("document serializes");
        let node = crate::walkers::find_node_mut(state.active_children_mut(), &NodeId::new("n1"))
            .expect("n1 exists");

        assert!(!crate::move_fill(node, from, to), "case {from}->{to}");
        assert_eq!(
            serde_json::to_value(&state.doc).expect("document serializes"),
            before,
            "case {from}->{to} must preserve the full document"
        );
    }
}

#[test]
fn move_selected_fill_is_one_undoable_history_entry() {
    let mut state = three_fill_state();

    assert!(state.move_selected_fill(2, 0));
    assert_eq!(state.history.past.len(), 1);
    assert_eq!(
        selected_fill_colors(&state),
        ["#333333", "#111111", "#222222"]
    );

    assert!(state.undo());
    assert_eq!(
        selected_fill_colors(&state),
        ["#111111", "#222222", "#333333"]
    );
}

#[test]
fn move_selected_fill_invalid_and_same_indices_do_not_push_history() {
    for (from, to) in [(0, 0), (3, 0), (0, 3)] {
        let mut state = three_fill_state();
        let before = serde_json::to_value(&state.doc).expect("document serializes");

        assert!(!state.move_selected_fill(from, to), "case {from}->{to}");
        assert_eq!(state.history.past.len(), 0, "case {from}->{to}");
        assert_eq!(
            serde_json::to_value(&state.doc).expect("document serializes"),
            before,
            "case {from}->{to} must preserve the full document"
        );
    }
}

#[test]
fn moving_across_primary_keeps_ref_with_its_fill_and_clears_primary_cache() {
    let mut state = three_fill_state();
    assert!(state.set_selected_fill_hex_at(0, "$brand"));
    state
        .ui
        .variables
        .fill_refs
        .insert(NodeId::new("n1"), "brand".to_string());

    assert!(state.move_selected_fill(2, 0));

    assert_eq!(
        selected_fill_colors(&state),
        ["#333333", "$brand", "#222222"],
        "the authored variable token must travel with its original fill"
    );
    assert!(
        !state
            .ui
            .variables
            .fill_refs
            .contains_key(&NodeId::new("n1")),
        "the node-level primary cache must not bind the new fill at index 0"
    );

    assert!(state.undo());
    assert_eq!(
        selected_fill_colors(&state),
        ["$brand", "#222222", "#333333"]
    );
    assert_eq!(
        state
            .ui
            .variables
            .fill_refs
            .get(&NodeId::new("n1"))
            .map(String::as_str),
        Some("brand"),
        "undo must restore the primary-fill cache with the authored token"
    );
}

/// Removing a fill must also drop the node's `fill_refs` variable binding.
/// Otherwise the scene resolver's `fill_for` (a registered fill ref wins
/// over `container.fill`) keeps painting the variable colour after the
/// fill row is gone — the "deleted the fill but the colour stays" bug on
/// token-based (old .op) designs.
#[test]
fn remove_selected_fill_clears_the_variable_ref() {
    use crate::node_id::NodeId;
    use crate::test_support::{rect, state_with};
    use crate::walkers::{find_node, find_node_mut};

    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    s.set_single_selection(NodeId::new("n1"));
    // Mirror a `$ref` fill: bind writes `$name` into fill[0] + fill_refs.
    let node = find_node_mut(s.active_children_mut(), &NodeId::new("n1")).unwrap();
    crate::fills::set_primary_fill_hex(node, "$color-info-bg");
    s.ui.variables
        .fill_refs
        .insert(NodeId::new("n1"), "color-info-bg".to_string());

    assert!(s.remove_selected_fill(0), "remove must report success");

    let node = find_node(s.active_children(), &NodeId::new("n1")).unwrap();
    assert!(
        crate::fills::node_fills(node)
            .map(|f| f.is_empty())
            .unwrap_or(true),
        "container.fill must be cleared"
    );
    assert!(
        !s.ui.variables.fill_refs.contains_key(&NodeId::new("n1")),
        "fill_ref must clear too, else fill_for keeps painting the variable colour"
    );
}

#[test]
fn node_stroke_width_reads_max_sided_edge() {
    let mut node = rect_node();
    let PenNode::Rectangle(r) = &mut node else {
        panic!("rectangle");
    };
    r.container.stroke = Some(PenStroke {
        thickness: StrokeThickness::Sided(SidedThickness {
            bottom: Some(3.0),
            ..Default::default()
        }),
        align: None,
        join: None,
        cap: None,
        dash_pattern: None,
        dash_offset: None,
        fill: None,
    });

    assert_eq!(node_stroke_width(&node), Some(3.0));
}

fn rect_node() -> PenNode {
    let src = r#"{"version":"1.0.0","children":[
        {"type":"rectangle","id":"r1","name":"R",
         "x":0,"y":0,"width":10,"height":10}
    ]}"#;
    jian_ops_schema::load_str(src)
        .expect("fixture parses")
        .value
        .children
        .into_iter()
        .next()
        .expect("one node")
}
