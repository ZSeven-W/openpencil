//! Build a codegen-pipeline `CodegenInput` from the editor's current
//! selection.
//!
//! The AI codegen pipeline runs against the SELECTED node subtrees, not
//! the whole document. This module turns `EditorState`'s selection into a
//! `CodegenInput`: the selected subtrees serialized to canonical JSON, the
//! active framework, and the document's design variables (when present).
//!
//! Asset sanitization (replacing embedded data-URLs) happens later, inside
//! the pipeline — so the RAW selected-nodes JSON returned here is also kept
//! for the export bundle. At this stage the raw JSON equals `nodes_json`.

use jian_ops_schema::node::PenNode;
use op_ai::chat_provider::{EffortLevel, ThinkingMode};
use op_codegen::ai::types::CodegenInput;
use op_editor_core::state::EditorState;
use op_editor_core::walkers::find_node;

/// Default per-request token cap for the pipeline input. The per-phase
/// prompt builders override this, so it is only a fallback default.
// Consumed by the host codegen session in a later P3 task.
#[allow(dead_code)]
const DEFAULT_MAX_OUTPUT_TOKENS: u32 = 4096;

/// Build pipeline input from the current selection, falling back to the
/// ACTIVE PAGE's children when nothing is selected (TS `getTargetNodes`,
/// code-panel.tsx:137-142). Returns the input plus the RAW nodes JSON
/// (before asset sanitization) for the bundle. `None` when there is
/// nothing to generate from: an empty page with no selection, or a
/// selection whose ids resolve to no live node (TS no-ops on
/// `nodes.length === 0`).
pub fn build_codegen_input(state: &EditorState) -> Option<(CodegenInput, String)> {
    if state.selection.is_empty() {
        // Whole-page fallback: every top-level child of the active page.
        let children = state.active_children();
        if children.is_empty() {
            return None;
        }
        let nodes: Vec<&PenNode> = children.iter().collect();
        return Some(input_from_nodes(
            &nodes,
            state.codegen.framework,
            state.doc.variables.as_ref(),
        ));
    }

    // Resolve each selected id to its full subtree. The forest is the page
    // children when the document is multi-page, else the root `children`.
    // `find_node` recurses, so a selected nested node resolves too.
    let mut selected: Vec<&PenNode> = Vec::with_capacity(state.selection.set.len());
    if let Some(pages) = state.doc.pages.as_ref() {
        for id in &state.selection.set {
            if let Some(node) = pages.iter().find_map(|page| find_node(&page.children, id)) {
                selected.push(node);
            }
        }
    } else {
        for id in &state.selection.set {
            if let Some(node) = find_node(&state.doc.children, id) {
                selected.push(node);
            }
        }
    }

    // A selection of ids that resolve to no live node yields no input.
    if selected.is_empty() {
        return None;
    }

    Some(input_from_nodes(
        &selected,
        state.codegen.framework,
        state.doc.variables.as_ref(),
    ))
}

/// Pure core: build a `CodegenInput` from already-resolved nodes, the target
/// framework and the document's optional variables. Separated from selection
/// resolution so it can be unit-tested with hand-built nodes. The second
/// tuple element is the RAW nodes JSON (identical to `input.nodes_json` here;
/// asset sanitization is a later, in-pipeline step).
fn input_from_nodes(
    nodes: &[&PenNode],
    framework: op_editor_core::codegen::Framework,
    variables: Option<
        &std::collections::BTreeMap<String, jian_ops_schema::variable::VariableDefinition>,
    >,
) -> (CodegenInput, String) {
    let nodes_json = serde_json::to_string(nodes).unwrap_or_else(|_| "[]".to_string());
    let variables_json = variables
        .filter(|vars| !vars.is_empty())
        .map(|vars| serde_json::to_string(vars).unwrap_or_else(|_| "{}".to_string()));

    let input = CodegenInput {
        nodes_json: nodes_json.clone(),
        framework,
        variables_json,
        max_output_tokens: DEFAULT_MAX_OUTPUT_TOKENS,
        thinking: ThinkingMode::Adaptive,
        effort: EffortLevel::Low,
    };
    (input, nodes_json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jian_ops_schema::node::{ContainerProps, FrameNode, PenNode, PenNodeBase, RectangleNode};
    use jian_ops_schema::sizing::SizingBehavior;
    use jian_ops_schema::variable::{
        VariableDefinition, VariableKind, VariableScalar, VariableValue,
    };
    use op_editor_core::EditorState;
    use op_editor_core::NodeId;

    fn frame(id: &str, children: Vec<PenNode>) -> PenNode {
        PenNode::Frame(FrameNode {
            base: PenNodeBase {
                id: id.to_string(),
                name: Some(id.to_string()),
                x: Some(0.0),
                y: Some(0.0),
                ..Default::default()
            },
            container: ContainerProps {
                width: Some(SizingBehavior::Number(100.0)),
                height: Some(SizingBehavior::Number(100.0)),
                ..Default::default()
            },
            children: Some(children),
            image_search_query: None,
            reusable: None,
            slot: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        })
    }

    fn rect(id: &str) -> PenNode {
        PenNode::Rectangle(RectangleNode {
            base: PenNodeBase {
                id: id.to_string(),
                name: Some(id.to_string()),
                x: Some(10.0),
                y: Some(10.0),
                ..Default::default()
            },
            container: ContainerProps {
                width: Some(SizingBehavior::Number(40.0)),
                height: Some(SizingBehavior::Number(40.0)),
                ..Default::default()
            },
            children: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        })
    }

    #[test]
    fn build_from_single_selected_frame() {
        let mut state = EditorState::new();
        state.doc.children = vec![frame("n1", vec![rect("n2")])];
        state.set_single_selection(NodeId::new("n1"));

        let (input, raw) = build_codegen_input(&state).expect("some");
        // The selected subtree (and its child) is serialized into the JSON.
        assert!(input.nodes_json.contains("n1"));
        assert!(input.nodes_json.contains("n2"));
        assert!(raw.contains("n1"));
        assert_eq!(input.framework, state.codegen.framework);
        // Defaults.
        assert_eq!(input.max_output_tokens, DEFAULT_MAX_OUTPUT_TOKENS);
        assert_eq!(input.thinking, ThinkingMode::Adaptive);
        assert_eq!(input.effort, EffortLevel::Low);
        // No variables on this doc.
        assert!(input.variables_json.is_none());
        // Raw equals the serialized nodes JSON at this stage.
        assert_eq!(raw, input.nodes_json);
    }

    #[test]
    fn empty_selection_on_empty_page_is_none() {
        // No selection AND no page children → nothing to generate from.
        let state = EditorState::new();
        assert!(build_codegen_input(&state).is_none());
    }

    #[test]
    fn empty_selection_falls_back_to_active_page_children() {
        // TS getTargetNodes: no selection → ALL active-page children.
        let mut state = EditorState::new();
        state.doc.children = vec![frame("n1", vec![rect("n2")]), frame("n3", vec![])];
        state.clear_selection();

        let (input, raw) = build_codegen_input(&state).expect("page fallback");
        assert!(input.nodes_json.contains("n1"));
        assert!(input.nodes_json.contains("n2"));
        assert!(input.nodes_json.contains("n3"));
        assert_eq!(raw, input.nodes_json);
        assert_eq!(input.framework, state.codegen.framework);
    }

    #[test]
    fn unresolvable_selection_is_none() {
        let mut state = EditorState::new();
        state.doc.children = vec![frame("n1", vec![])];
        // Select an id that no longer exists in the tree.
        state.set_single_selection(NodeId::new("ghost"));
        assert!(build_codegen_input(&state).is_none());
    }

    #[test]
    fn variables_are_serialized_when_present() {
        let mut state = EditorState::new();
        state.doc.children = vec![frame("n1", vec![])];
        state
            .doc
            .variables
            .get_or_insert_with(Default::default)
            .insert(
                "color-1".to_string(),
                VariableDefinition {
                    kind: VariableKind::Color,
                    value: VariableValue::Scalar(VariableScalar::Str("#ff0000".to_string())),
                },
            );
        state.set_single_selection(NodeId::new("n1"));

        let (input, _raw) = build_codegen_input(&state).expect("some");
        let vars_json = input.variables_json.expect("variables");
        assert!(vars_json.contains("color-1"));
    }

    #[test]
    fn multi_selection_serializes_each_subtree() {
        let mut state = EditorState::new();
        state.doc.children = vec![frame("n1", vec![]), frame("n3", vec![])];
        state.selection.set = vec![NodeId::new("n1"), NodeId::new("n3")];
        state.selection.anchor = NodeId::new("n3");

        let (input, _raw) = build_codegen_input(&state).expect("some");
        assert!(input.nodes_json.contains("n1"));
        assert!(input.nodes_json.contains("n3"));
    }
}
