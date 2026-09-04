//! OpenPencil AI chat layer.
//!
//! This crate carries the transport-free data shapes for the editor's
//! chat / agent integration — extracted out of `openpencil-shell-core`
//! in the Phase 7 strangler reorg:
//!
//! - [`chat_provider`] — the `ChatProvider` trait + provider-category
//!   types (`CliName`, `Provider`, …) and the `EchoProvider` test
//!   double. Real transports (tokio / reqwest / process-spawn) live
//!   desktop-side in `openpencil-desktop`.
//! - [`chat_models`] — the `ModelEntry` model-catalog type.
//! - [`chat_sse`] — transport-free Anthropic / OpenAI-compatible SSE
//!   payload → `ChatDelta` parsers shared by the desktop/daemon
//!   transports (`op-host-services`) and the mobile FFI chat pump
//!   (`op-engine-ffi`).
//! - [`agent_settings_state`] — state types for the Cmd+, settings
//!   modal (`AgentSettingsTab`, `AgentProvider`, …).
//! - [`design_md`] — the shared design.md generation system prompt +
//!   LLM-output cleanup helpers used by the desktop / web / serve-web
//!   design.md generators.
//!
//! The crate is transport-free (serde_json only) and wasm32-clean so
//! both the native and web shells can build against it.

pub mod agent_settings_state;
pub mod chat_history;
pub mod chat_models;
pub mod chat_provider;
pub mod chat_sse;
pub mod chat_tool_sse;
pub mod design_md;
