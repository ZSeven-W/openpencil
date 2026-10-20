//! Editor-UI overlay + panel state for `EditorState`.
//!
//! This module owns the widget-layer state beyond [`crate::ui_draft`]:
//! menus, modals, panels, preferences, hover targets, and pending host
//! actions. Painting and hit-testing stay in the UI layer. Its plain-data
//! types keep `op-editor-core` wasm32-clean.
//!
//! This public spine re-exports [`EditorUiState`] and the shared surface;
//! implementations live in sibling modules without changing import paths:
//!
//! Public submodules group chrome, picker, Git, panel, and slide-navigation
//! data. Private `defaults`, `methods`, `state`, and `tests` modules hold
//! the struct definition, its implementations, and regression coverage.

pub mod chrome;
mod defaults;
mod exports;
pub mod git_panel;
pub mod groups;
pub mod home;
pub mod home_example_draft;
mod methods;
pub mod pickers;
pub mod slides_panel_state;
mod state;
#[cfg(test)]
mod tests;
pub mod workspace;

pub use exports::*;
pub use state::EditorUiState;
