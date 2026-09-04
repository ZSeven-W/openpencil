//! Typed failures for the sandboxed QuickJS script→program runner
//! (`script_runner.rs`).
//!
//! Style follows `ProgramError` / `op_orchestrator::OrchestratorError`: a
//! plain enum plus a hand-written `Display`, no `thiserror` and no new
//! dependency. Every variant's `Display` reproduces the exact sentence the
//! stringly-typed runner produced, because those sentences ship verbatim to
//! the model (the `batch_design(script)` tool surfaces them as the
//! `InvalidArgument` payload) and the orchestrator's retry ladder logs them.
//!
//! What the enum buys over `String` is the CLASSIFICATION: a caller can now
//! tell "the model sent nothing runnable" (`EmptySource` / `NoOperations`)
//! from "the sandbox itself could not start" (`RuntimeInit` / `ContextInit` /
//! `BindHostFn` / `SetGlobal` — host faults, never the model's fault) from
//! "the script threw" (`Threw` / `StaleReference`), without matching prose.

use std::fmt;

/// Everything `run_script_to_program` can refuse on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptError {
    /// `strip_fences` left nothing runnable (the model emitted only prose
    /// or an empty fence).
    EmptySource,
    /// The source exceeds `MAX_SCRIPT_BYTES`.
    SourceTooLarge { bytes: usize, max: usize },
    /// The script ran to completion but never called `I(...)`.
    NoOperations,
    /// The QuickJS runtime could not be created — a host fault.
    RuntimeInit(String),
    /// The QuickJS context could not be created — a host fault.
    ContextInit(String),
    /// A host recorder function could not be built. `name` is the JS-visible
    /// binding (`__record` / `__recordK`).
    BindHostFn { name: &'static str, detail: String },
    /// A host recorder function could not be installed on `globalThis`.
    SetGlobal { name: &'static str, detail: String },
    /// The sandbox prelude failed to evaluate — a host fault.
    Prelude(String),
    /// The script threw and nothing had been recorded before the throw.
    /// The payload is the JS exception message (or the thrown value's
    /// string form).
    Threw(String),
    /// The script referenced a binding that only existed in an earlier
    /// batch's sandbox. `message` is the raw `x is not defined` text and
    /// `name` the identifier, so the explanation can name it back.
    StaleReference { message: String, name: String },
    /// The script threw a value carrying no message.
    UncaughtException,
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScriptError::EmptySource => f.write_str("script is empty after stripping fences"),
            ScriptError::SourceTooLarge { bytes, max } => {
                write!(f, "script too large: {bytes} bytes (max {max})")
            }
            ScriptError::NoOperations => {
                f.write_str("script emitted no I(...), K(...), or U(...) operations")
            }
            ScriptError::RuntimeInit(detail) => write!(f, "js runtime: {detail}"),
            ScriptError::ContextInit(detail) => write!(f, "js context: {detail}"),
            ScriptError::BindHostFn { name, detail } => write!(f, "bind {name}: {detail}"),
            ScriptError::SetGlobal { name, detail } => write!(f, "set {name}: {detail}"),
            ScriptError::Prelude(detail) => write!(f, "prelude: {detail}"),
            ScriptError::Threw(message) => write!(f, "script error: {message}"),
            ScriptError::StaleReference { message, name } => write!(
                f,
                "script error: {message}. Each script runs in a FRESH sandbox — a variable \
                 from an earlier batch no longer exists. Reference nodes created in an \
                 earlier batch by their id STRING instead: I(\"n12\", {{…}}), not I({name}, {{…}})."
            ),
            ScriptError::UncaughtException => f.write_str("script error: uncaught JS exception"),
        }
    }
}

impl std::error::Error for ScriptError {}

/// Boundary bridge for the callers that still report `String` —
/// `batch_design::expand_script_arg` forwards the message into a
/// `ToolOutcome::Err` payload. `Display` reproduces the exact sentence, so
/// the text the model receives is unchanged.
impl From<ScriptError> for String {
    fn from(error: ScriptError) -> String {
        error.to_string()
    }
}
