//! Design memory — a port of `pen-ai-skills/src/memory`.
//!
//! Two stateless helpers the resolution engine consults:
//!  - [`document_context`] — accumulated design facts for one document;
//!  - [`generation_history`] — the anti-repetition feedback log.
//!
//! The TS originals call `new Date().toISOString()` internally; the
//! Rust ports take an explicit timestamp argument instead, keeping the
//! crate dependency-free and the functions deterministic for tests.

pub mod document_context;
pub mod generation_history;

pub use document_context::{
    context_to_prompt_string, create_design_context, extract_design_context, merge_preference,
    OrchestratorPlan, PlanStyleGuide, PlanSubtask,
};
pub use generation_history::{
    create_history_entry, get_recent_entries, history_to_prompt_string, HistoryEntryParams,
};
