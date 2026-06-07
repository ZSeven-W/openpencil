//! Desktop codegen session — drives the pull-based `CodegenPipeline` on a
//! worker thread and streams progress into `editor_state.codegen`. Mirrors
//! `design_session.rs` (single progress channel; no document mutation).
//! The worker driver + pump + launch land in the next tasks; this file
//! defines the types so the crate compiles.

use std::sync::mpsc::Receiver;

use op_codegen::ai::types::AssetFile;
use op_editor_core::codegen::CodeGenProgress;

/// Streamed from the worker to the UI pump.
// fields wired in P3 Tasks 3-5
#[allow(dead_code)]
pub enum CodegenDelta {
    Progress(CodeGenProgress),
    Done {
        code: String,
        degraded: bool,
        assets: Vec<AssetFile>,
    },
    Failed(String),
}

/// An in-flight generation. The UI pump drains `rx` each frame.
// fields wired in P3 Tasks 3-5
#[allow(dead_code)]
pub struct CodegenSession {
    pub(crate) rx: Receiver<CodegenDelta>,
    pub(crate) finished: bool,
}

/// The completed result kept HOST-SIDE for Download / Export Bundle — asset
/// bytes are not carried in the wasm-clean `editor_state`.
// fields wired in P3 Tasks 3-5
#[allow(dead_code)]
#[derive(Default, Clone)]
pub struct CodegenResult {
    pub code: String,
    /// File extension for the active framework (e.g. "tsx", "vue", "html").
    pub framework_ext: String,
    pub assets: Vec<AssetFile>,
    /// Raw (pre-asset-sanitization) selected-nodes JSON, for the bundle.
    pub raw_nodes_json: String,
    /// Sanitized selected-nodes JSON (asset data-URLs replaced), for the bundle.
    pub sanitized_nodes_json: String,
}
