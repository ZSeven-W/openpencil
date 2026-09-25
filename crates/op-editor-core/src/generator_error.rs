//! Typed failures for canvas generator nodes (`generator.rs`).
//!
//! One enum for the whole feature: spec parsing, parameter validation,
//! program execution (reported by whichever host runner is installed),
//! output limits, and the final apply. `Display` is the sentence the
//! property panel and the MCP tools show verbatim, so every variant names
//! what went wrong AND what the document looks like afterwards (always:
//! unchanged — a generator failure never half-applies).

use std::fmt;

/// Everything a generator operation can refuse on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeneratorError {
    /// No node with this id exists in the document.
    NodeNotFound { id: String },
    /// The node exists but carries no generator spec.
    NotAGenerator { id: String },
    /// Generators are frames; this node kind cannot own generated children.
    NotAFrame { id: String },
    /// The stored spec could not be parsed or failed validation.
    InvalidSpec(String),
    /// The spec names an engine this build does not know.
    UnsupportedEngine(String),
    /// A parameter name the spec does not declare.
    UnknownParam(String),
    /// A parameter index past the end of the declared list.
    ParamIndexOutOfRange { index: usize, count: usize },
    /// A parameter value that does not match its declared kind.
    InvalidParamValue { name: String, reason: String },
    /// The program source exceeds [`crate::generator::MAX_PROGRAM_BYTES`].
    ProgramTooLarge { bytes: usize, max: usize },
    /// This host has no generator runtime installed (the browser bundle).
    RuntimeUnavailable,
    /// The program threw, failed to parse, or emitted a rejected operation.
    Program(String),
    /// The program ran past its wall-clock budget and was interrupted.
    TimeLimit { budget_ms: u64 },
    /// The program emitted more nodes than a generator may own.
    TooManyNodes { count: usize, max: usize },
    /// The program emitted a tree nested deeper than allowed.
    TooDeep { depth: usize, max: usize },
    /// A derived child id already belongs to a node outside the generator.
    IdCollision { id: String },
    /// The editor refused the final document write.
    ApplyRejected,
    /// A live collaboration session cannot carry generator writes yet.
    CollaborationUnsupported,
}

impl fmt::Display for GeneratorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GeneratorError::NodeNotFound { id } => write!(f, "node not found: {id}"),
            GeneratorError::NotAGenerator { id } => {
                write!(f, "node {id} is not a generator")
            }
            GeneratorError::NotAFrame { id } => {
                write!(f, "node {id} is not a frame; only frames can be generators")
            }
            GeneratorError::InvalidSpec(detail) => write!(f, "invalid generator spec: {detail}"),
            GeneratorError::UnsupportedEngine(engine) => {
                write!(f, "unsupported generator engine: {engine}")
            }
            GeneratorError::UnknownParam(name) => write!(f, "unknown generator parameter: {name}"),
            GeneratorError::ParamIndexOutOfRange { index, count } => write!(
                f,
                "generator parameter index {index} out of range ({count} parameters)"
            ),
            GeneratorError::InvalidParamValue { name, reason } => {
                write!(f, "invalid value for parameter {name}: {reason}")
            }
            GeneratorError::ProgramTooLarge { bytes, max } => {
                write!(f, "generator program too large: {bytes} bytes (max {max})")
            }
            GeneratorError::RuntimeUnavailable => f.write_str(
                "this app cannot run generator programs; the saved children are kept as-is",
            ),
            GeneratorError::Program(detail) => write!(f, "generator program failed: {detail}"),
            GeneratorError::TimeLimit { budget_ms } => write!(
                f,
                "generator program stopped after {budget_ms} ms (time limit); document unchanged"
            ),
            GeneratorError::TooManyNodes { count, max } => write!(
                f,
                "generator produced {count} nodes (max {max}); document unchanged"
            ),
            GeneratorError::TooDeep { depth, max } => write!(
                f,
                "generator output is nested {depth} levels deep (max {max}); document unchanged"
            ),
            GeneratorError::IdCollision { id } => write!(
                f,
                "generated id {id} is already used by another node; document unchanged"
            ),
            GeneratorError::ApplyRejected => {
                f.write_str("the editor rejected the generated children; document unchanged")
            }
            GeneratorError::CollaborationUnsupported => {
                f.write_str("generators cannot be edited during a live collaboration session")
            }
        }
    }
}

impl std::error::Error for GeneratorError {}
