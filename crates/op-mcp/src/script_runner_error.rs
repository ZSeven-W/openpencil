//! Typed failures for the sandboxed QuickJS script→program runner
//! (`script_runner.rs`).
//!
//! Style follows `ProgramError` / `op_orchestrator::OrchestratorError`: a
//! plain enum plus a hand-written `Display`, without `thiserror`. Existing
//! variants retain the exact sentence the
//! stringly-typed runner produced, because those sentences ship verbatim to
//! the model (the `batch_design(script)` tool surfaces them as the
//! `InvalidArgument` payload) and the orchestrator's retry ladder logs them.
//!
//! What the enum buys over `String` is the CLASSIFICATION: a caller can now
//! tell "the model sent nothing runnable" (`EmptySource` / `NoOperations`)
//! from "the sandbox itself could not start" (`RuntimeInit` / `ContextInit` /
//! `BindHostFn` / `SetGlobal` — host faults, never the model's fault) from
//! "the script threw" (`Threw` / reference failures), without matching prose.

use jian_core::expression::builtins::default_builtins;
use std::fmt;

/// Everything `run_script_to_program` can refuse on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptError {
    /// `strip_fences` left nothing runnable (the model emitted only prose
    /// or an empty fence).
    EmptySource,
    /// The source exceeds `MAX_SCRIPT_BYTES`.
    SourceTooLarge { bytes: usize, max: usize },
    /// Several alternative programs were supplied without an explicit choice.
    AmbiguousCodeBlocks { count: usize },
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
    /// A Preview expression was evaluated as generation-time JavaScript.
    RuntimeBindingReference { scope: String },
    /// An undefined JS identifier also names a registered Preview builtin.
    /// Name alone cannot distinguish a call from an intended local handle.
    RuntimeBuiltinReference { name: String },
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
            ScriptError::AmbiguousCodeBlocks { count } => write!(
                f,
                "script response contains {count} code blocks without a clear final program; return one complete JavaScript program"
            ),
            ScriptError::NoOperations => {
                f.write_str("script emitted no I(...), K(...), or U(...) operations")
            }
            ScriptError::RuntimeInit(detail) => write!(f, "js runtime: {detail}"),
            ScriptError::ContextInit(detail) => write!(f, "js context: {detail}"),
            ScriptError::BindHostFn { name, detail } => write!(f, "bind {name}: {detail}"),
            ScriptError::SetGlobal { name, detail } => write!(f, "set {name}: {detail}"),
            ScriptError::Prelude(detail) => write!(f, "prelude: {detail}"),
            ScriptError::Threw(message) => write!(f, "script error: {message}"),
            ScriptError::RuntimeBindingReference { scope } => write!(
                f,
                "script error: {scope} is not defined. {scope} is a Preview runtime scope, \
                 not a JavaScript variable in the generation sandbox. Quote the ENTIRE \
                 runtime expression, for example bindings: {{content: \"'Selected: ' + $app.choice\"}}. \
                 Declare typed state with an intentional default; event set values are also \
                 expression strings (a string literal needs inner quotes). Preserve the requested \
                 behavior while repairing syntax."
            ),
            ScriptError::RuntimeBuiltinReference { name } => write!(
                f,
                "script error: {name} is not defined. This name is also a registered Jian \
                 Preview builtin, not an installed JavaScript global. If you intended a \
                 binding/action expression, quote the ENTIRE expression: \
                 bindings: {{content: \"to_str($app.qty)\"}}. Calling to_str($app.qty) or \
                 to_str(\"$app.qty\") in the builder is still wrong; quoting only the \
                 argument is insufficient. For generation-time conversion use JavaScript \
                 String/Number/Boolean. If it is a JavaScript node handle or helper, \
                 declare it in this complete script; a builtin name alone does not imply \
                 an old node id. Preserve the requested state, bindings and events."
            ),
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

impl ScriptError {
    /// Classify only exact Preview scopes or names in its actual registry.
    /// A builtin-name collision remains ambiguous; its diagnostic covers both
    /// expression quotation and an intended local JS handle/helper declaration.
    pub fn runtime_reference(name: &str) -> Option<Self> {
        if matches!(name, "$state" | "$app") {
            Some(Self::RuntimeBindingReference {
                scope: name.to_string(),
            })
        } else if default_builtins().contains_key(name) {
            Some(Self::RuntimeBuiltinReference {
                name: name.to_string(),
            })
        } else {
            None
        }
    }

    /// Parent contract failures must not be salvaged as a successful prefix.
    pub fn is_invalid_insertion_parent(&self) -> bool {
        matches!(self, Self::Threw(message) if message.contains("OP_SCRIPT_PARENT_UNDEFINED")
            || message.contains("OP_SCRIPT_PARENT_UNRESOLVED"))
    }
}

/// Boundary bridge for the callers that still report `String` —
/// `batch_design::expand_script_arg` forwards the message into a
/// `ToolOutcome::Err` payload. `Display` reproduces the exact sentence, so
/// the text the model receives is unchanged.
impl From<ScriptError> for String {
    fn from(error: ScriptError) -> String {
        error.to_string()
    }
}
