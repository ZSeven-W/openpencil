//! Chat model catalog types.
//!
//! shell-core is wasm32-clean and transport-free, so it owns only
//! the [`ModelEntry`] *type* — it never queries a CLI. The desktop
//! host (`openpencil-desktop`) discovers real models from the
//! connected CLIs (Codex's `models_cache.json`, `opencode models`,
//! the Claude SDK, …) and writes them into
//! `Document.chat.available_models`.

use crate::agent_settings_state::AgentProvider;

/// One selectable model in the chat model picker.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelEntry {
    /// Which CLI agent backs this model — also picks the chat
    /// transport (`chat_session::provider_for_agent`).
    pub provider: AgentProvider,
    /// Wire id passed to the CLI (e.g. `gpt-5.5`, `claude-sonnet-4-6`).
    pub value: String,
    /// Human label shown in the picker (e.g. `GPT-5.5`).
    pub display_name: String,
}

impl ModelEntry {
    pub fn new(
        provider: AgentProvider,
        value: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        Self {
            provider,
            value: value.into(),
            display_name: display_name.into(),
        }
    }
}
