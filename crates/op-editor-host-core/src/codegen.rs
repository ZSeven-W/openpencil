//! Shared code-generation input builders.

use std::io::{self, Write};

use jian_ops_schema::node::PenNode;
use op_ai::chat_provider::{EffortLevel, ThinkingMode};
use op_codegen::ai::types::CodegenInput;
use op_editor_core::state::EditorState;
use op_editor_core::walkers::find_node;

/// Default hard per-request token cap. Phase builders stay within it while
/// selecting planning/chunk/assembly budgets of 6k/12k/16k respectively.
pub const DEFAULT_MAX_OUTPUT_TOKENS: u32 = 16_000;

/// A bounded live-export serialization failed before returning a JSON copy.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(crate) enum CodegenNodesJsonError {
    #[error("Codegen nodes JSON is too large ({input_bytes} bytes observed; maximum is {max_bytes} bytes)")]
    TooLarge {
        input_bytes: usize,
        max_bytes: usize,
    },
    #[error("Could not serialize codegen nodes JSON: {message}")]
    Serialization { message: String },
}

/// Build pipeline input from the current selection, falling back to the
/// active page's children when nothing is selected. Returns the input plus
/// the raw selected-nodes JSON used by desktop export bundling.
pub fn build_codegen_input(state: &EditorState) -> Option<(CodegenInput, String)> {
    let selected = codegen_target_nodes(state)?;
    Some(input_from_nodes(
        &selected,
        state.codegen.framework,
        state.doc.variables.as_ref(),
    ))
}

/// Serialize only the selected/page nodes for live bundle export, stopping
/// before the output can exceed `max_bytes`. Unlike `build_codegen_input`,
/// this returns exactly one owned JSON buffer and never serializes variables.
pub(crate) fn build_codegen_nodes_json_limited(
    state: &EditorState,
    max_bytes: usize,
) -> Result<Option<String>, CodegenNodesJsonError> {
    let Some(nodes) = codegen_target_nodes(state) else {
        return Ok(None);
    };
    let mut writer = LimitedJsonWriter::new(max_bytes);
    let result = serde_json::to_writer(&mut writer, &nodes);
    if let Some(input_bytes) = writer.overflow_at {
        return Err(CodegenNodesJsonError::TooLarge {
            input_bytes,
            max_bytes,
        });
    }
    result.map_err(|error| CodegenNodesJsonError::Serialization {
        message: error.to_string(),
    })?;
    String::from_utf8(writer.bytes).map(Some).map_err(|error| {
        CodegenNodesJsonError::Serialization {
            message: error.utf8_error().to_string(),
        }
    })
}

/// Web hosts do not need the raw selected-nodes JSON because their bundle is
/// assembled from live selection state.
pub fn build_codegen_input_value(state: &EditorState) -> Option<CodegenInput> {
    build_codegen_input(state).map(|(input, _raw)| input)
}

fn codegen_target_nodes(state: &EditorState) -> Option<Vec<&PenNode>> {
    if state.selection.is_empty() {
        let children = state.active_children();
        return (!children.is_empty()).then(|| children.iter().collect());
    }

    let mut selected = Vec::with_capacity(state.selection.set.len());
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
    (!selected.is_empty()).then_some(selected)
}

struct LimitedJsonWriter {
    bytes: Vec<u8>,
    max_bytes: usize,
    overflow_at: Option<usize>,
}

impl LimitedJsonWriter {
    fn new(max_bytes: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(max_bytes.min(64 * 1024)),
            max_bytes,
            overflow_at: None,
        }
    }
}

impl Write for LimitedJsonWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let input_bytes = self.bytes.len().saturating_add(buffer.len());
        if input_bytes > self.max_bytes {
            self.overflow_at.get_or_insert(input_bytes);
            return Err(io::Error::other("codegen nodes JSON limit exceeded"));
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> EditorState {
        let doc = jian_ops_schema::load_str(
            r#"{"version":"1.0.0","children":[
                {"type":"rectangle","id":"n1","name":"A","width":10,"height":10},
                {"type":"rectangle","id":"n2","name":"B","width":10,"height":10}
            ]}"#,
        )
        .expect("fixture parses")
        .value;
        EditorState::from_document(doc)
    }

    #[test]
    fn limited_json_matches_generation_raw_json_without_a_second_copy() {
        let mut state = state();
        state.set_single_selection(op_editor_core::NodeId::new("n1"));
        let (_, generation_raw) = build_codegen_input(&state).expect("generation input");
        let limited = build_codegen_nodes_json_limited(&state, generation_raw.len())
            .expect("bounded serialization")
            .expect("live nodes");
        assert_eq!(limited, generation_raw);
    }

    #[test]
    fn limited_json_returns_a_typed_error_as_soon_as_the_cap_is_crossed() {
        let error = build_codegen_nodes_json_limited(&state(), 32)
            .expect_err("fixture exceeds the tiny cap");
        assert!(matches!(
            error,
            CodegenNodesJsonError::TooLarge {
                input_bytes,
                max_bytes: 32,
            } if input_bytes > 32
        ));
    }

    #[test]
    fn limited_writer_never_appends_the_over_limit_chunk() {
        let mut writer = LimitedJsonWriter::new(4);
        writer.write_all(b"1234").expect("at limit");
        assert!(writer.write_all(b"5").is_err());
        assert_eq!(writer.bytes, b"1234");
        assert_eq!(writer.overflow_at, Some(5));
    }
}
