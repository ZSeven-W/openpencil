//! Hoist node-level `state` to one document-root `MergeAppState`.
//!
//! Generated nodes may declare `state` (the `$app.*` store). Document
//! state is global, so we strip every node's `state` and emit a single
//! [`EditorCommand::MergeAppState`] per subtask, tagged with the
//! subtask's plan index. The additive, `plan_idx`-keyed apply
//! (`op-editor-core`) then resolves cross-subtask conflicts
//! deterministically regardless of concurrent replay order.

use jian_ops_schema::node::PenNode;
use jian_ops_schema::state::StateSchema;
use op_editor_core::EditorCommand;

/// Take (strip) a node's own `state`, returning the drained schema.
fn take_node_state(node: &mut PenNode) -> Option<StateSchema> {
    match node {
        PenNode::Frame(n) => n.state.take(),
        PenNode::Group(n) => n.state.take(),
        PenNode::Rectangle(n) => n.state.take(),
        PenNode::Ellipse(n) => n.state.take(),
        PenNode::Line(n) => n.state.take(),
        PenNode::Polygon(n) => n.state.take(),
        PenNode::Path(n) => n.state.take(),
        PenNode::Text(n) => n.state.take(),
        PenNode::TextInput(n) => n.state.take(),
        PenNode::TextArea(n) => n.state.take(),
        PenNode::Select(n) => n.state.take(),
        PenNode::Switch(n) => n.state.take(),
        PenNode::Checkbox(n) => n.state.take(),
        PenNode::Slider(n) => n.state.take(),
        PenNode::RadioGroup(n) => n.state.take(),
        PenNode::NumberInput(n) => n.state.take(),
        PenNode::Progress(n) => n.state.take(),
        PenNode::Tabs(n) => n.state.take(),
        PenNode::Image(n) => n.state.take(),
        PenNode::IconFont(n) => n.state.take(),
        PenNode::Ref(n) => n.state.take(),
    }
}

/// Return mutable children of any container-style node.
///
/// Only variants that carry an `Option<Vec<PenNode>>` children field are
/// matched; leaf nodes return `None` so the recursive walk skips them.
fn children_mut(node: &mut PenNode) -> Option<&mut Vec<PenNode>> {
    match node {
        PenNode::Frame(n) => n.children.as_mut(),
        PenNode::Group(n) => n.children.as_mut(),
        PenNode::Rectangle(n) => n.children.as_mut(),
        PenNode::Tabs(n) => n.children.as_mut(),
        PenNode::Ref(n) => n.children.as_mut(),
        _ => None,
    }
}

/// Recursively walk `nodes`, drain each node's `state` into `acc`.
///
/// When the same key appears in multiple nodes within one subtask, the first
/// occurrence wins (the deeper cross-subtask conflict is resolved later by
/// `plan_idx` ordering in `op-editor-core`).
fn hoist_in_slice(nodes: &mut [PenNode], acc: &mut StateSchema) {
    for node in nodes.iter_mut() {
        if let Some(schema) = take_node_state(node) {
            for (k, v) in schema {
                acc.entry(k).or_insert(v);
            }
        }
        // Recurse into any container children that exist.
        if let Some(kids) = children_mut(node) {
            hoist_in_slice(kids, acc);
        }
    }
}

/// Strip every node's `state` from `forest` and return one
/// [`EditorCommand::MergeAppState`] tagged with `plan_idx`.
///
/// Callers should skip emitting the command when the returned
/// `MergeAppState.state` is empty (no nodes declared state).
pub fn hoist_app_state(forest: &mut [PenNode], plan_idx: usize) -> EditorCommand {
    let mut state = StateSchema::new();
    hoist_in_slice(forest, &mut state);
    EditorCommand::MergeAppState { plan_idx, state }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_nodes(json: &str) -> Vec<PenNode> {
        serde_json::from_str(json).expect("parse nodes")
    }

    #[test]
    fn hoist_strips_node_state_and_emits_one_command() {
        let mut forest = parse_nodes(
            r#"[{"type":"frame","id":"root","state":{"count":{"type":"int","default":0}}}]"#,
        );
        let cmd = hoist_app_state(&mut forest, 2);
        // Node state must be stripped.
        let PenNode::Frame(f) = &forest[0] else {
            panic!("expected Frame")
        };
        assert!(f.state.is_none(), "node state must be stripped");
        // Command carries the key and the correct plan index.
        let EditorCommand::MergeAppState { plan_idx, state } = cmd else {
            panic!("expected MergeAppState")
        };
        assert_eq!(plan_idx, 2);
        assert!(state.contains_key("count"));
    }

    #[test]
    fn hoist_recurses_into_nested_nodes() {
        // Outer frame has no state; inner frame carries state.
        let mut forest = parse_nodes(
            r#"[{"type":"frame","id":"outer","children":[
                {"type":"frame","id":"inner","state":{"score":{"type":"int","default":0}}}
            ]}]"#,
        );
        let cmd = hoist_app_state(&mut forest, 0);
        // Inner node state must be drained.
        let PenNode::Frame(outer) = &forest[0] else {
            panic!()
        };
        let inner_node = outer.children.as_ref().unwrap().first().unwrap();
        let PenNode::Frame(inner_f) = inner_node else {
            panic!()
        };
        assert!(
            inner_f.state.is_none(),
            "nested node state must be stripped"
        );
        let EditorCommand::MergeAppState { state, .. } = cmd else {
            panic!("expected MergeAppState")
        };
        assert!(state.contains_key("score"), "nested key must be hoisted");
    }

    #[test]
    fn hoist_merges_keys_from_multiple_nodes_first_wins() {
        // Two sibling frames declare the same key with different defaults; first wins.
        let mut forest = parse_nodes(
            r#"[
                {"type":"frame","id":"a","state":{"x":{"type":"int","default":1}}},
                {"type":"frame","id":"b","state":{"x":{"type":"int","default":99}}}
            ]"#,
        );
        let EditorCommand::MergeAppState { state, .. } = hoist_app_state(&mut forest, 0) else {
            panic!()
        };
        let entry = state.get("x").unwrap();
        let default_val = entry
            .default
            .as_ref()
            .and_then(|v| v.as_i64())
            .expect("default must be an integer");
        assert_eq!(default_val, 1, "first declaration of key x must win");
    }

    #[test]
    fn hoist_empty_forest_yields_empty_state() {
        let mut forest: Vec<PenNode> = Vec::new();
        let EditorCommand::MergeAppState { state, plan_idx } = hoist_app_state(&mut forest, 5)
        else {
            panic!()
        };
        assert_eq!(plan_idx, 5);
        assert!(state.is_empty(), "empty forest must produce empty state");
    }

    #[test]
    fn hoist_group_children_walked() {
        // A group container with a child frame that carries state.
        let mut forest = parse_nodes(
            r#"[{"type":"group","id":"grp","children":[
                {"type":"frame","id":"leaf","state":{"toggle":{"type":"bool","default":false}}}
            ]}]"#,
        );
        let EditorCommand::MergeAppState { state, .. } = hoist_app_state(&mut forest, 1) else {
            panic!()
        };
        assert!(
            state.contains_key("toggle"),
            "group child state must be hoisted"
        );
    }
}
