use jian_ops_schema::node::{ContainerProps, FrameNode, PenNode, PenNodeBase, RectangleNode};
use jian_ops_schema::sizing::SizingBehavior;
use jian_ops_schema::variable::{VariableDefinition, VariableKind, VariableScalar, VariableValue};
use op_ai::chat_provider::{EffortLevel, ThinkingMode};
use op_editor_core::{EditorState, NodeId};
use op_editor_host_core::codegen::{
    build_codegen_input, build_codegen_input_value, framework_ext, DEFAULT_MAX_OUTPUT_TOKENS,
};

fn two_rect_state() -> EditorState {
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0.0","children":[
            {"type":"rectangle","id":"n1","name":"A","x":0,"y":0,"width":10,"height":10},
            {"type":"rectangle","id":"n2","name":"B","x":20,"y":0,"width":10,"height":10}
        ]}"#,
    )
    .expect("fixture parses")
    .value;
    EditorState::from_document(doc)
}

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
        screen: None,
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
fn selected_subtree_input_keeps_raw_json() {
    let mut state = two_rect_state();
    state.set_single_selection(NodeId::new("n1"));
    let (input, raw) = build_codegen_input(&state).expect("input");
    assert!(input.nodes_json.contains("n1"));
    assert!(!input.nodes_json.contains("n2"));
    assert_eq!(raw, input.nodes_json);
    assert_eq!(input.max_output_tokens, DEFAULT_MAX_OUTPUT_TOKENS);
    assert_eq!(input.thinking, ThinkingMode::Adaptive);
    assert_eq!(input.effort, EffortLevel::Low);
    assert!(input.variables_json.is_none());
}

#[test]
fn selected_frame_serializes_child_subtree() {
    let mut state = EditorState::new();
    state.doc.children = vec![frame("n1", vec![rect("n2")])];
    state.set_single_selection(NodeId::new("n1"));

    let (input, raw) = build_codegen_input(&state).expect("some");
    assert!(input.nodes_json.contains("n1"));
    assert!(input.nodes_json.contains("n2"));
    assert!(raw.contains("n1"));
    assert_eq!(raw, input.nodes_json);
}

#[test]
fn value_helper_drops_raw_json_for_web() {
    let mut state = two_rect_state();
    state.clear_selection();
    let input = build_codegen_input_value(&state).expect("page fallback");
    assert!(input.nodes_json.contains("n1"));
    assert!(input.nodes_json.contains("n2"));
}

#[test]
fn empty_and_unresolvable_inputs_are_none() {
    assert!(build_codegen_input(&EditorState::new()).is_none());
    let mut ghost = two_rect_state();
    ghost.set_single_selection(NodeId::new("ghost"));
    assert!(build_codegen_input(&ghost).is_none());
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

#[test]
fn framework_extensions_match_desktop_and_web() {
    use op_editor_core::codegen::Framework;
    assert_eq!(framework_ext(Framework::React), "tsx");
    assert_eq!(framework_ext(Framework::ReactNative), "tsx");
    assert_eq!(framework_ext(Framework::Vue), "vue");
    assert_eq!(framework_ext(Framework::Svelte), "svelte");
    assert_eq!(framework_ext(Framework::Html), "html");
    assert_eq!(framework_ext(Framework::Flutter), "dart");
    assert_eq!(framework_ext(Framework::SwiftUi), "swift");
    assert_eq!(framework_ext(Framework::Compose), "kt");
}
