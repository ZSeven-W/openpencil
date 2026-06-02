//! OpenPencil editor core.
//!
//! Owns the editor's runtime state — `EditorState` — built on the
//! canonical `.op` document model (`jian_ops_schema::PenDocument`).
//! There is no second document model: the node tree, pages, variables
//! and themes all live on `PenDocument`; this crate adds only the
//! editor-only state (selection, tool, viewport, history, transient
//! UI drafts).

pub mod agent_settings;
pub mod agent_settings_builtin_presets;
pub mod align;
pub mod align_guides;
pub mod chat;
mod chat_design_apply;
pub mod clipboard;
pub mod color_picker;
pub mod command;
pub mod command_apply;
pub mod command_layout_prop;
pub mod command_node;
pub mod command_node_attrs;
pub mod components;
pub mod design_md;
pub mod editor_ui_state;
pub mod fills;
pub mod geometry;
pub mod grouping;
pub mod history;
pub mod host_support;
pub mod icon_picker_state;
pub mod image_node_props;
pub mod mutators;
pub mod node_id;
pub mod page_mutators;
pub mod path_bounds;
pub mod pen;
pub mod pen_node_ext;
pub mod property_edit_mutators;
pub mod property_panel_state;
pub mod rename;
pub mod render_backend;
pub mod selection;
pub mod state;
pub mod svg_import;
pub mod svg_path_bounds;
mod svg_path_data;
pub mod tool;
pub mod toolbar_state;
pub mod ui_draft;
pub mod uikit;
pub mod variables;
pub mod viewport;
pub mod walkers;

#[cfg(test)]
mod command_attr_tests;
#[cfg(test)]
mod command_subtree_tests;
#[cfg(test)]
mod command_tests;
#[cfg(test)]
mod svg_import_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests_agent_settings;
#[cfg(test)]
mod tests_agent_settings_draft;
#[cfg(test)]
mod tests_geometry;
#[cfg(test)]
mod tests_mutators;
#[cfg(test)]
mod tests_pages;

pub use agent_settings::{
    AcpAgentConfig, AcpAgentField, AcpConnectionType, AgentSettings, AgentSettingsDrag,
    AgentSettingsTab, BuiltinAgentConfig, BuiltinAgentField, BuiltinAgentKind, ImageGenField,
    ImageGenProfile, ImageGenProvider, ImageSearchField, ImageTestStatus, McpCli, McpServer,
    SettingsFocus,
};
pub use agent_settings_builtin_presets::{
    builtin_agent_preset, infer_builtin_agent_preset, normalize_builtin_agent_preset,
    BuiltinAgentPreset, BuiltinAgentPresetKey, BUILTIN_AGENT_PRESETS,
};
pub use align::AlignAction;
pub use chat::{
    AgentProvider, ChatAnchor, ChatImage, ChatMessage, ChatRole, ChatState, ChatToolCall,
    ModelEntry,
};
pub use color_picker::{hsv_to_rgb, parse_hex_alpha, parse_hex_rgb, rgb_to_hex, rgb_to_hsv};
pub use command::{
    BatchInsertItem, EditorCommand, EffectField, LayoutPropValue, NodeFlag, VariableScalarPayload,
};
pub use components::{Component, ComponentLibrary};
pub use design_md::{extract_design_md_from_document, generate_design_md, parse_design_md};
pub use editor_ui_state::{
    BooleanOp, CloneField, CloneFormState, DesignMdRequest, EditorUiState, ExportFormat,
    FileAction, FileMenuChoice, FillType, FlexLayout, GitBranchPickerMode, GitCommitSummary,
    GitDiffTarget, GitDiffView, GitFileEntry, GitOverflowView, GitPanelAction, GitPanelState,
    ImageAdjustmentField, ImageFillMode, LayerContextMenuState, Locale, MergeConflictRow,
    MergeResolveFile, MergeResolveState, PaddingEditMode, PageRenameState, PropertyTab, RecentFile,
    ShapeChoice, ThemeMode, UpdateStatus, VariableRowFocus,
};
pub use fills::{
    first_fill_type, first_image_fill_summary, first_solid_fill_hex, first_solid_fill_opacity,
    first_solid_stroke_hex, node_effects, ImageFillSummary,
};
pub use geometry::{aggregate_bounds, own_bounds, union_aggregate_bounds, DocRect};
pub use history::{EditorSnapshot, History, HISTORY_CAP};
pub use icon_picker_state::{IconPickerRemoteIcon, IconPickerRemoteState, IconifyLoadMoreRequest};
pub use image_node_props::image_node_summary;
pub use jian_ops_schema::{DesignMdColor, DesignMdSpec, DesignMdTypography};
pub use node_id::NodeId;
pub use pen_node_ext::PenNodeExt;
pub use render_backend::*;
pub use selection::SelectionState;
pub use state::EditorState;
pub use tool::Tool;
pub use toolbar_state::{ToolbarAction, ToolbarHover};
pub use ui_draft::{
    ColorPickerDrag, ColorPickerState, ColorTarget, LayerContextTarget, LayerRenameState,
    PropertyFocus, UiDraftState, VariableUiState,
};
pub use uikit::{builtin_kits, ComponentCategory, KitComponent, UIKit};
pub use viewport::Viewport;
pub use walkers::ReorderDirection;
