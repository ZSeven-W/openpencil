//! Typed failure domain for the `run_design_agent` MCP tool.
//!
//! Every variant is a pre-flight or transport failure — a loop that starts
//! always drains to exhaustion, and tool-level defects inside the loop are
//! the model's to correct, not errors of this tool.

use std::fmt;

/// Why a `run_design_agent` call could not produce a landed design.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesignAgentRunError {
    /// The `brief` argument was missing or blank.
    EmptyBrief,
    /// No enabled builtin provider with an API key + saved model exists in
    /// the daemon's persisted agent settings.
    NoConfiguredProvider,
    /// `provider_id` named no persisted builtin provider.
    UnknownProvider(String),
    /// The named provider exists but is disabled or missing key/model.
    ProviderNotReady(String),
    /// `model` is not one of the named provider's saved model ids.
    ModelNotSaved { provider: String, model: String },
    /// The wall-clock budget elapsed before the loop finished; the live
    /// document is untouched.
    Timeout { seconds: u64 },
    /// The provider stream ended in an error delta without a usable design.
    Loop(String),
    /// The loop finished but left no top-level frames to land; the live
    /// document is untouched.
    EmptyResult,
}

impl fmt::Display for DesignAgentRunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyBrief => f.write_str("brief must be a non-empty design request"),
            Self::NoConfiguredProvider => f.write_str(
                "no builtin agent provider is configured: save an enabled provider with an \
                 API key and at least one model in the daemon's agent settings",
            ),
            Self::UnknownProvider(id) => {
                write!(f, "provider_id names no saved builtin provider: {id}")
            }
            Self::ProviderNotReady(id) => write!(
                f,
                "builtin provider {id} is not ready (needs enabled + api_key + a saved model)"
            ),
            Self::ModelNotSaved { provider, model } => write!(
                f,
                "model {model} is not saved on builtin provider {provider}"
            ),
            Self::Timeout { seconds } => write!(
                f,
                "design agent loop exceeded its {seconds}s budget and was cancelled; \
                 the document is unchanged"
            ),
            Self::Loop(message) => write!(f, "design agent loop failed: {message}"),
            Self::EmptyResult => f.write_str(
                "design agent loop finished without authoring any top-level frame; \
                 the document is unchanged",
            ),
        }
    }
}

impl std::error::Error for DesignAgentRunError {}
