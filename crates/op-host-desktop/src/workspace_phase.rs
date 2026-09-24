//! The pure decisions behind the generation workspace's phase pump.
//!
//! `app_handler/redraw.rs` observes the "everything is done" edge every
//! frame. The verdicts themselves moved to `op_editor_core::workspace_run`
//! so the mobile engine's chat pump settles its runs by the same rules;
//! this module keeps the desktop's import path stable.

pub(crate) use op_editor_core::workspace_run::{
    assistant_streaming, awaiting_launch, last_assistant_failed, produced_board_count,
};
