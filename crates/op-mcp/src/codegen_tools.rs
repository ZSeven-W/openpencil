//! TS-compatible codegen MCP tools for file-backed Rust MCP.
//!
//! The TS `packages/pen-mcp` codegen tools forward to the running web
//! app's `/api/mcp/codegen/*` endpoints because the pipeline state is
//! stored in app memory. The file-backed Rust MCP server does not own
//! that live app state yet, so these tools mirror TS standalone
//! behavior: plan/submit/assemble report the live-canvas requirement,
//! and clean is idempotent with `{ ok: true, deleted: false }`.

use std::collections::BTreeMap;

use super::{McpTool, ToolErrorCode, ToolOutcome};

pub struct CodegenPlan;
pub struct CodegenSubmitChunk;
pub struct CodegenAssemble;
pub struct CodegenClean;

impl McpTool for CodegenPlan {
    fn name(&self) -> &str {
        "codegen_plan"
    }

    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        ToolOutcome::Err(
            ToolErrorCode::ToolFailed,
            "codegen_plan requires a running App (live canvas mode). Pipeline state is stored in App memory.".into(),
        )
    }
}

impl McpTool for CodegenSubmitChunk {
    fn name(&self) -> &str {
        "codegen_submit_chunk"
    }

    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        ToolOutcome::Err(
            ToolErrorCode::ToolFailed,
            "codegen_submit_chunk requires a running App (live canvas mode)".into(),
        )
    }
}

impl McpTool for CodegenAssemble {
    fn name(&self) -> &str {
        "codegen_assemble"
    }

    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        ToolOutcome::Err(
            ToolErrorCode::ToolFailed,
            "codegen_assemble requires a running App (live canvas mode)".into(),
        )
    }
}

impl McpTool for CodegenClean {
    fn name(&self) -> &str {
        "codegen_clean"
    }

    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        out.insert("ok".into(), "true".into());
        out.insert("deleted".into(), "false".into());
        ToolOutcome::Ok(out)
    }
}

pub fn codegen_plan_snapshot() -> CodegenPlan {
    CodegenPlan
}

pub fn codegen_submit_chunk_snapshot() -> CodegenSubmitChunk {
    CodegenSubmitChunk
}

pub fn codegen_assemble_snapshot() -> CodegenAssemble {
    CodegenAssemble
}

pub fn codegen_clean_snapshot() -> CodegenClean {
    CodegenClean
}
