//! Pipeline-internal types. Ported from packages/pen-types/src/codegen.ts.
//! Shared UI-facing types come from op-editor-core; param enums from op-ai.

use op_ai::chat_provider::{EffortLevel, ThinkingMode};
use op_editor_core::codegen::Framework;
use serde::{Deserialize, Serialize};

/// Stable id correlating a dispatched request with its streamed deltas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequestId(pub u64);

/// Which pipeline phase a request belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestKind {
    Planning,
    Chunk { chunk_id: String },
    Assembly,
}

/// Step-1 planner output (no node data). Parity with TS `CodePlanFromAI`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CodePlan {
    #[serde(default)]
    pub chunks: Vec<PlannedChunk>,
    #[serde(default, rename = "sharedStyles")]
    pub shared_styles: Vec<SharedStyle>,
    #[serde(rename = "rootLayout")]
    pub root_layout: RootLayout,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SharedStyle {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RootLayout {
    #[serde(default)]
    pub direction: String,
    #[serde(default)]
    pub gap: f64,
    #[serde(default)]
    pub responsive: bool,
}

/// Parity with TS `PlannedChunk`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PlannedChunk {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "nodeIds")]
    pub node_ids: Vec<String>,
    #[serde(default)]
    pub role: String,
    #[serde(default, rename = "suggestedComponentName")]
    pub suggested_component_name: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

/// Hydrated chunk with execution order. Parity with TS `ExecutableChunk`
/// (we carry the planned chunk + order; the prompt builder reads node data
/// from `CodegenInput`).
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutableChunk {
    pub plan: PlannedChunk,
    pub order: usize,
}

/// A prop a chunk component exposes. Parity with TS `PropDef`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct PropDef {
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "type")]
    pub ty: String,
    #[serde(default)]
    pub required: bool,
}

/// A slot a chunk component exposes. Parity with TS `SlotDef`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct SlotDef {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
}

/// An import a chunk component declares. Parity with TS `ImportDef`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct ImportDef {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub specifiers: Vec<String>,
}

/// Structured metadata each chunk emits. Parity with TS `ChunkContract`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct ChunkContract {
    #[serde(default, rename = "chunkId")]
    pub chunk_id: String,
    #[serde(default, rename = "componentName")]
    pub component_name: String,
    #[serde(default, rename = "exportedProps")]
    pub exported_props: Vec<PropDef>,
    #[serde(default)]
    pub slots: Vec<SlotDef>,
    #[serde(default, rename = "cssClasses")]
    pub css_classes: Vec<String>,
    #[serde(default, rename = "cssVariables")]
    pub css_variables: Vec<String>,
    #[serde(default)]
    pub imports: Vec<ImportDef>,
}

/// A chunk's generated code + parsed contract. Parity with TS `ChunkResult`.
#[derive(Debug, Clone, PartialEq)]
pub struct ChunkResult {
    pub chunk_id: String,
    pub code: String,
    pub contract: ChunkContract,
}

/// An extracted embedded image. Bytes stay here; the host maps to
/// `op_editor_core::codegen::AssetMeta` for the UI. Parity with TS
/// `CodegenAssetFile`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetFile {
    pub id: String,
    pub relative_path: String,
    pub zip_path: String,
    pub mime_type: String,
    pub bytes: Vec<u8>,
    pub source_node_id: String,
}

/// Inputs to construct a pipeline run.
#[derive(Debug, Clone)]
pub struct CodegenInput {
    /// Selected node subtree as canonical JSON (already sanitized of
    /// embedded asset data-URLs by `assets::extract_codegen_assets`).
    pub nodes_json: String,
    pub framework: Framework,
    /// Design variables as JSON (None if the doc has none).
    pub variables_json: Option<String>,
    /// Per-request token cap.
    pub max_output_tokens: u32,
    pub thinking: ThinkingMode,
    pub effort: EffortLevel,
}

/// A logical request the host must execute against the model. Skill NAMES
/// (not text) + the pipeline-owned user message + params. The transport
/// expands the skills into the final `op_ai::ChatRequest`.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingRequest {
    pub id: RequestId,
    pub kind: RequestKind,
    pub skills: Vec<&'static str>,
    pub user_message: String,
    pub max_output_tokens: u32,
    pub thinking: ThinkingMode,
    pub effort: EffortLevel,
}

/// What the host should do next. Returned by `CodegenPipeline::step`.
#[derive(Debug, Clone, PartialEq)]
pub enum PipelineStep {
    Dispatch(Vec<PendingRequest>),
    Waiting,
    Done {
        code: String,
        degraded: bool,
        assets: Vec<AssetFile>,
    },
    Failed {
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_plan_deserializes_ts_shape() {
        let json = r#"{
            "chunks": [
                {"id":"a","name":"Nav","nodeIds":["1"],"role":"nav",
                 "suggestedComponentName":"Navbar","dependencies":[]}
            ],
            "sharedStyles": [],
            "rootLayout": {"direction":"column","gap":16,"responsive":true}
        }"#;
        let plan: CodePlan = serde_json::from_str(json).unwrap();
        assert_eq!(plan.chunks.len(), 1);
        assert_eq!(plan.chunks[0].suggested_component_name, "Navbar");
        assert_eq!(plan.root_layout.direction, "column");
        assert!(plan.root_layout.responsive);
    }

    #[test]
    fn chunk_contract_defaults_missing_arrays() {
        let c: ChunkContract = serde_json::from_str(r#"{"componentName":"Card"}"#).unwrap();
        assert_eq!(c.component_name, "Card");
        assert!(c.css_classes.is_empty());
        assert!(c.exported_props.is_empty());
        assert!(c.slots.is_empty());
        assert!(c.imports.is_empty());
    }

    #[test]
    fn chunk_contract_deserializes_props_slots_imports() {
        let json = r#"{
            "componentName": "Card",
            "exportedProps": [
                {"name": "title", "type": "string", "required": true},
                {"name": "subtitle", "type": "string", "required": false}
            ],
            "slots": [{"name": "footer", "description": "card footer area"}],
            "imports": [{"source": "react", "specifiers": ["useState"]}]
        }"#;
        let c: ChunkContract = serde_json::from_str(json).unwrap();
        assert_eq!(c.exported_props.len(), 2);
        assert_eq!(c.exported_props[0].name, "title");
        assert_eq!(c.exported_props[0].ty, "string");
        assert!(c.exported_props[0].required);
        assert!(!c.exported_props[1].required);
        assert_eq!(c.slots.len(), 1);
        assert_eq!(c.slots[0].name, "footer");
        assert_eq!(c.slots[0].description, "card footer area");
        assert_eq!(c.imports.len(), 1);
        assert_eq!(c.imports[0].source, "react");
        assert_eq!(c.imports[0].specifiers, vec!["useState"]);
    }
}
