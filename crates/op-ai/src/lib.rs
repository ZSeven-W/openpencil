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
//! - [`agent_settings_state`] — state types for the Cmd+, settings
//!   modal (`AgentSettingsTab`, `AgentProvider`, …).
//!
//! The crate is dependency-free and wasm32-clean so both the native
//! and web shells can build against it.

pub mod agent_settings_state;
pub mod chat_models;
pub mod chat_provider;
