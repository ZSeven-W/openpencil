//! Script-gen subagent parse path — THE default generation protocol.
//!
//! The sub-agent writes a real JavaScript program (`I(parent, {...})` +
//! loops); expansion, sandboxing, limits, and truncation repair live in
//! `op_mcp::script_runner` (shared verbatim with external
//! `batch_design(script)` calls). This module only bridges the recorded
//! program into the section-forest executor.

use jian_ops_schema::node::PenNode;

/// Expand the emitted JS into a `batch_design` program and run it to a
/// section forest via the shared executor.
pub fn parse_script(text: &str) -> Result<Vec<PenNode>, String> {
    let program = op_mcp::script_runner::run_script_to_program(text)?;
    crate::program_gen::run_program_to_forest(&program)
}

#[cfg(test)]
#[path = "script_gen_tests.rs"]
mod tests;
