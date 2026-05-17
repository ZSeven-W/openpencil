//! OpenPencil editor core.
//!
//! Owns the editor's runtime state — `EditorState` — built on the
//! canonical `.op` document model (`jian_ops_schema::PenDocument`).
//! There is no second document model: the node tree, pages, variables
//! and themes all live on `PenDocument`; this crate adds only the
//! editor-only state (selection, tool, viewport, history, transient
//! UI drafts).

pub mod agent_settings;
pub mod align;
pub mod chat;
pub mod clipboard;
pub mod color_picker;
pub mod command;
pub mod command_apply;
pub mod command_node;
pub mod command_node_attrs;
pub mod components;
pub mod editor_ui_state;
pub mod fills;
pub mod geometry;
pub mod grouping;
pub mod history;
pub mod host_support;
pub mod mutators;
pub mod node_id;
pub mod page_mutators;
pub mod pen;
pub mod pen_node_ext;
pub mod rename;
pub mod render_backend;
pub mod selection;
pub mod state;
pub mod svg_import;
pub mod svg_path_bounds;
pub mod tool;
pub mod ui_draft;
pub mod variables;
pub mod viewport;
pub mod walkers;

#[cfg(test)]
mod command_tests;
#[cfg(test)]
mod svg_import_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests_geometry;
#[cfg(test)]
mod tests_mutators;
#[cfg(test)]
mod tests_pages;

pub use agent_settings::{
    AgentSettings, AgentSettingsDrag, AgentSettingsTab, McpCli, McpServer, SettingsFocus,
};
pub use align::AlignAction;
pub use chat::{AgentProvider, ChatAnchor, ChatMessage, ChatRole, ChatState, ModelEntry};
pub use color_picker::{hsv_to_rgb, parse_hex_rgb, rgb_to_hex, rgb_to_hsv};
pub use command::{BatchInsertItem, EditorCommand, NodeFlag, VariableScalarPayload};
pub use components::{Component, ComponentLibrary};
pub use editor_ui_state::{
    BooleanOp, EditorUiState, ExportFormat, FileAction, FileMenuChoice, FillType, FlexLayout,
    LayerContextMenuState, Locale, PageRenameState, PropertyTab, RecentFile, ShapeChoice,
    ThemeMode, VariableRowFocus,
};
pub use fills::{first_fill_type, first_solid_fill_hex, first_solid_stroke_hex};
pub use geometry::{aggregate_bounds, own_bounds, union_aggregate_bounds, DocRect};
pub use history::{EditorSnapshot, History, HISTORY_CAP};
pub use node_id::NodeId;
pub use pen_node_ext::PenNodeExt;
pub use render_backend::*;
pub use selection::SelectionState;
pub use state::EditorState;
pub use tool::Tool;
pub use ui_draft::{
    ColorPickerDrag, ColorPickerState, ColorTarget, LayerContextTarget, LayerRenameState,
    PropertyFocus, UiDraftState, VariableUiState,
};
pub use viewport::Viewport;
pub use walkers::ReorderDirection;
