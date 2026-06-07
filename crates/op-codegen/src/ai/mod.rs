//! AI-driven code-generation pipeline. A pull-based, transport-agnostic
//! state machine: it emits `PendingRequest`s (skill names + user message
//! + params) and consumes streamed text deltas via `on_delta`/
//! `on_complete`/`on_error`, advancing through planning → chunk →
//! assembly. Hosts supply the streaming transport; the pipeline owns the
//! logic. Ported from the TS `code-generation-pipeline.ts`.

pub mod assets;
pub mod bundle;
pub mod parse;
pub mod pipeline;
pub mod prompts;
pub mod types;
