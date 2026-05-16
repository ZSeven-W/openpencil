//! OpenPencil editor core.
//!
//! Owns the editor's runtime state — `EditorState` — built on the
//! canonical `.op` document model (`jian_ops_schema::PenDocument`).
//! There is no second document model: the node tree, pages, variables
//! and themes all live on `PenDocument`; this crate adds only the
//! editor-only state (selection, tool, viewport, history, transient
//! UI drafts).

pub mod history;
pub mod node_id;
pub mod render_backend;
pub mod selection;
pub mod state;
pub mod tool;
pub mod ui_draft;
pub mod viewport;

pub use history::{EditorSnapshot, History, HISTORY_CAP};
pub use node_id::NodeId;
pub use render_backend::*;
pub use selection::SelectionState;
pub use state::EditorState;
pub use tool::Tool;
pub use ui_draft::{
    ColorTarget, LayerContextTarget, LayerRenameState, PropertyFocus, UiDraftState, VariableUiState,
};
pub use viewport::Viewport;
