//! Shared code-generation input builders.

use jian_ops_schema::node::PenNode;
use op_ai::chat_provider::{EffortLevel, ThinkingMode};
use op_codegen::ai::types::CodegenInput;
use op_editor_core::state::EditorState;
use op_editor_core::walkers::find_node;

/// Default per-request token cap for the pipeline input. The per-phase
/// prompt builders override this, so it is only a fallback default.
pub const DEFAULT_MAX_OUTPUT_TOKENS: u32 = 4096;

/// Build pipeline input from the current selection, falling back to the
/// active page's children when nothing is selected. Returns the input plus
/// the raw selected-nodes JSON used by desktop export bundling.
pub fn build_codegen_input(state: &EditorState) -> Option<(CodegenInput, String)> {
    if state.selection.is_empty() {
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

    if selected.is_empty() {
        return None;
    }

    Some(input_from_nodes(
        &selected,
        state.codegen.framework,
        state.doc.variables.as_ref(),
    ))
}

/// Web hosts do not need the raw selected-nodes JSON because their bundle is
/// assembled from live selection state.
pub fn build_codegen_input_value(state: &EditorState) -> Option<CodegenInput> {
    build_codegen_input(state).map(|(input, _raw)| input)
}

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

/// File extension for the active framework's generated component file.
pub fn framework_ext(fw: op_editor_core::codegen::Framework) -> &'static str {
    use op_editor_core::codegen::Framework;
    match fw {
        Framework::React | Framework::ReactNative => "tsx",
        Framework::Vue => "vue",
        Framework::Svelte => "svelte",
        Framework::Html => "html",
        Framework::Flutter => "dart",
        Framework::SwiftUi => "swift",
        Framework::Compose => "kt",
    }
}
