//! Script-gen subagent parse path — THE default generation protocol.
//!
//! The sub-agent writes a real JavaScript program (`I(parent, {...})` +
//! loops); expansion, sandboxing, limits, and truncation repair live in
//! `op_mcp::script_runner` (shared with external `batch_design(script)` calls).
//! Generation defers foreground repair until the complete forest has passed
//! structural repair; direct MCP calls retain insertion-time contrast checks.

use crate::program_gen::ProgramGenError;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::state::StateSchema;

/// Failure of [`parse_script`]. Both variants render VERBATIM (no prefix):
/// the sub-agent transcript shows the underlying script-runner / program
/// message exactly as it did when this was a bare `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptGenError {
    /// `op_mcp::script_runner::run_generation_script_to_program` rejected the emitted
    /// JS. Carried as its display text because the runner lives in a crate
    /// this module does not own.
    ScriptRun(String),
    /// The recorded `batch_design` program failed to build a forest.
    Program(ProgramGenError),
}

impl std::fmt::Display for ScriptGenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ScriptRun(message) => write!(f, "{message}"),
            Self::Program(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ScriptGenError {}

impl From<ProgramGenError> for ScriptGenError {
    fn from(error: ProgramGenError) -> Self {
        Self::Program(error)
    }
}

/// Keeps `?`-into-`Result<_, String>` call sites compiling unchanged.
impl From<ScriptGenError> for String {
    fn from(error: ScriptGenError) -> Self {
        error.to_string()
    }
}

/// Expand the emitted JS into a `batch_design` program and run it to a
/// section forest via the shared executor. Returns the forest plus any
/// doc-root `state` the program's nodes hoisted (see `program_gen`'s module
/// doc for why the caller needs this separately from the forest).
pub fn parse_script(text: &str) -> Result<(Vec<PenNode>, StateSchema), ScriptGenError> {
    // `to_string()` (not `?`-with-`From`) so this stays source-compatible
    // whether the script runner returns a `String` or its own typed error.
    let program = op_mcp::script_runner::run_generation_script_to_program(text)
        .map_err(|error| ScriptGenError::ScriptRun(error.to_string()))?;
    Ok(crate::program_gen::run_program_to_forest(&program)?)
}

#[cfg(test)]
#[path = "script_gen_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "script_gen_style_replay_tests.rs"]
mod style_replay_tests;
