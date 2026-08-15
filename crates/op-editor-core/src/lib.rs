//! OpenPencil editor core.
//!
//! Owns the editor's runtime state — `EditorState` — built on the
//! canonical `.op` document model (`jian_ops_schema::PenDocument`).
//! There is no second document model: the node tree, pages, variables
//! and themes all live on `PenDocument`; this crate adds only the
//! editor-only state (selection, tool, viewport, history, transient
//! UI drafts).

pub mod account_state;
pub mod acp_agent_presets;
pub mod agent_indicators;
mod agent_indicators_tests;
mod agent_provider_wire;
pub mod agent_reveals;
pub mod agent_settings;
pub mod agent_settings_acp_connection;
pub mod agent_settings_builtin_models;
pub mod agent_settings_builtin_presets;
pub mod agent_settings_button_state;
pub mod agent_settings_connection;
pub mod align;
pub mod align_guides;
pub mod auth_routes;
pub mod blank_starter;
pub mod bridge_protocol;
pub mod button_press_state;
mod catalog_toml;
pub mod chat;
pub mod chat_activity;
pub mod chat_button_state;
mod chat_design_apply;
mod chat_model_mutators;
pub mod chat_sessions;
mod chat_title;
pub mod clipboard;
pub mod codegen;
pub mod collab_admission_ui;
pub mod collab_gate;
pub mod collab_notice_ui;
pub mod collab_owner_confirm_ui;
pub mod collab_panel_hover;
pub mod collab_public_ui;
pub mod collab_routes;
mod collab_ui_debug;
pub mod collab_ui_state;
pub mod collab_wire;
pub mod color_picker;
pub mod color_picker_edit;
mod color_picker_snapshot;
pub mod command;
pub mod command_apply;
mod command_apply_legacy;
pub mod command_authored_subtree;
pub mod command_batch;
pub mod command_font_replace;
pub mod command_layout_prop;
pub mod command_node;
pub mod command_node_attrs;
pub mod command_promote;
pub mod command_refine;
mod command_root_replace;
pub mod command_style_replace;
mod component_backing;
pub mod component_browser_state;
pub mod components;
pub mod compositing;
pub mod conversion;
pub mod design_md;
pub mod design_md_button_state;
pub mod document_install;
pub mod drag_mutators;
pub mod edit_transaction;
pub mod editor_toast;
pub mod size_class;
// Runtime-fetched product assets for the browser bundle (native embeds them).
pub mod editor_ui_state;
pub mod export_batch;
pub mod export_dialog_state;
pub mod export_name;
pub mod export_quick_menu_state;
pub mod figma_import_state;
pub mod fill_order;
pub mod fills;
pub mod font_catalog;
pub mod geometry;
pub mod git_button_state;
pub mod grouping;
pub mod history;
pub mod history_snapshot;
pub mod hoist_app_state;
pub mod host_drag_state;
pub mod host_drag_transitions;
pub mod host_escape_transitions;
pub mod host_image_panel_transitions;
pub mod host_keyboard_transitions;
pub mod host_preset_name_draft;
pub mod host_press_transitions;
pub mod host_settings_commit;
pub mod host_support;
mod host_support_allocator;
#[cfg(test)]
mod host_support_tests;
pub mod host_ui_transitions;
pub mod host_variables_commit;
pub mod host_variables_transitions;
pub mod html_import_diagnostics;
pub mod icon_picker_state;
pub mod id_allocator;
pub mod image_aspect;
pub mod image_crop;
pub mod image_drop;
mod image_fill_upload;
pub mod image_node_props;
pub mod image_panel_state;
pub mod ime_state;
mod instance_child_override;
pub mod instance_override;
pub mod missing_fonts;
pub mod mutators;
pub mod node_defaults;
pub mod node_id;
pub mod page_mutators;
pub mod path_bounds;
pub mod path_edit;
pub mod pen;
pub mod pen_node_ext;
pub mod preview_slideshow;
pub mod prompt_center_catalog;
pub mod prompt_center_keyboard;
pub mod property_edit_mutators;
pub mod property_panel_state;
pub mod ref_resolve;
pub mod rename;
pub mod render_backend;
pub mod request_snapshot;
pub mod scene_template_append;
pub mod scene_template_catalog;
pub mod scene_template_keyboard;
pub mod scene_template_palette;
pub mod scene_template_prompt;
pub mod scene_vars;
pub mod selection;
pub mod selection_resolve;
pub mod share_routes;
pub mod state;
pub mod statusbar_state;
pub mod svg_import;
pub mod svg_path_bounds;
mod svg_path_data;
pub mod sync_gate;
pub mod user_scene_templates;
pub mod web_assets;

/// Tight source-coordinate bounds for an SVG path-data string.
pub fn svg_path_data_bounds(d: &str) -> Option<(f32, f32, f32, f32)> {
    let bounds = svg_path_data::svg_path_bounds(d)?;
    Some((
        bounds.x as f32,
        bounds.y as f32,
        bounds.w as f32,
        bounds.h as f32,
    ))
}
pub mod text_edit;
pub mod text_input_focus;
pub mod text_script;
mod text_script_tests;
pub mod theme_presets;
pub mod tool;
pub mod toolbar_state;
pub mod topbar_state;
pub mod ui_draft;
pub mod uikit;
mod uikit_allocator;
pub mod uikit_io;
pub mod uikit_shadcn;
pub mod variables;
pub mod variables_panel_state;
pub mod variables_resolve;
pub mod viewport;
pub mod walkers;
pub mod web_sync;

#[cfg(test)]
mod command_allocator_tests;
#[cfg(test)]
mod command_app_state_tests;
#[cfg(test)]
mod command_attr_tests;
#[cfg(test)]
mod command_authored_subtree_tests;
#[cfg(test)]
mod command_batch_page_tests;
#[cfg(test)]
mod command_batch_tests;
#[cfg(test)]
mod command_component_tests;
#[cfg(test)]
mod command_delete_tests;
#[cfg(test)]
mod command_insert_tests;
#[cfg(test)]
mod command_promote_tests;
#[cfg(test)]
mod command_refine_tests;
#[cfg(test)]
mod command_reparent_tests;
#[cfg(test)]
mod command_replace_tests;
#[cfg(test)]
mod command_stroke_tests;
#[cfg(test)]
mod command_style_replace_tests;
#[cfg(test)]
mod command_subtree_tests;
#[cfg(test)]
mod command_tests;
#[cfg(test)]
mod command_update_tests;
#[cfg(test)]
mod command_widget_tests;
#[cfg(test)]
mod conversion_tests;
#[cfg(test)]
mod dirty_tests;
#[cfg(test)]
mod document_install_tests;
#[cfg(test)]
mod edit_transaction_tests;
#[cfg(test)]
mod fills_tests;
#[cfg(test)]
mod history_bench_tests;
#[cfg(test)]
mod history_snapshot_tests;
#[cfg(test)]
mod prompt_center_catalog_tests;
#[cfg(test)]
mod property_task9_tests;
#[cfg(test)]
mod svg_import_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests_agent_settings;
#[cfg(test)]
mod tests_agent_settings_draft;
#[cfg(test)]
mod tests_agent_settings_model_catalog;
#[cfg(test)]
mod tests_drag_mutators;
#[cfg(test)]
mod tests_geometry;
#[cfg(test)]
mod tests_mutators;
#[cfg(test)]
mod tests_pages;
#[cfg(test)]
mod translate_equivalence_tests;

pub use account_state::{
    AccountMenuRow, AccountState, LoginFlowError, LoginFlowStatus, LoginModalButton,
};
pub use acp_agent_presets::{
    acp_agent_preset, matches_preset_transport, AcpAgentPreset, AcpPresetAvailability,
    ACP_AGENT_PRESETS,
};
pub use agent_settings::{
    normalize_builtin_models, AcpAgentConfig, AcpAgentConnectOutcome, AcpAgentConnectPhase,
    AcpAgentConnectRequest, AcpAgentConnection, AcpAgentField, AcpConnectionType, AgentSettings,
    AgentSettingsDrag, AgentSettingsTab, BuiltinAgentConfig, BuiltinAgentField, BuiltinAgentKind,
    BuiltinModelCatalog, BuiltinModelCatalogPhase, BuiltinModelCatalogRefreshOutcome,
    BuiltinModelCatalogRefreshRequest, BuiltinModelCatalogTarget, BuiltinModelOption,
    ImageGenField, ImageGenProfile, ImageGenProvider, ImageSearchField, ImageTestStatus, McpCli,
    McpServer, ProviderConnectOutcome, ProviderConnectPhase, ProviderConnection, SettingsFocus,
    MAX_BUILTIN_AGENT_MODELS, MAX_BUILTIN_MODEL_CHARS,
};
pub use agent_settings_builtin_presets::{
    builtin_agent_preset, infer_builtin_agent_preset, normalize_builtin_agent_preset,
    BuiltinAgentPreset, BuiltinAgentPresetKey, BUILTIN_AGENT_PRESETS,
};
pub use agent_settings_button_state::AgentSettingsButton;
pub use agent_settings_connection::{
    local_daemon_origin, local_mcp_url, missing_models_connect_error, DEFAULT_LOCAL_DAEMON_ORIGIN,
    DEFAULT_MCP_PORT,
};
pub use align::AlignAction;
pub use button_press_state::ButtonPressTarget;
pub use chat::{
    AgentProvider, ChatAnchor, ChatImage, ChatMessage, ChatRole, ChatState, ChatToolCall,
    ModelEntry,
};
pub use chat_activity::{ChatActivity, ChatActivityStatus, ChatCompletion, PendingSubtaskRetry};
pub use chat_button_state::{ChatFooterButton, ChatHeaderButton};
pub use chat_sessions::{adjust_running_tab_after_close, ChatSessions};
pub use collab_admission_ui::{
    CollabAdmissionRequestKey, PendingCollabAdmissionUi, MAX_COLLAB_ADMISSION_REQUEST_KEY_BYTES,
    MAX_COLLAB_PENDING_ADMISSIONS,
};
pub use collab_gate::{
    CollabApplyError, CollabDocumentMutation, CollabEditSource, CollabGateAction, CollabGatePolicy,
    CollabGateReason, CollabNodeField, CollabUnsupportedFeature,
};
pub use collab_owner_confirm_ui::{
    CollabOwnerIdentityUi, PendingOwnerConfirmationUi, MAX_COLLAB_OWNER_DISPLAY_NAME_CHARS,
    MAX_COLLAB_OWNER_IDENTIFIER_CHARS,
};
pub use collab_panel_hover::CollabPanelHover;
pub use collab_public_ui::{
    CollabConnectErrorUi, CollabConnectionPathUi, CollabInviteCode, CollabPublicSessionUi,
    CollabRelayRegion, MAX_COLLAB_INVITE_CODE_CHARS,
};
pub use collab_ui_state::{
    AuthenticatedCollabSession, CollabAvailability, CollabCanvasPoint, CollabConnectionPhase,
    CollabDiscardedEditUi, CollabNotice, CollabNoticeKind, CollabPanelState, CollabPanelView,
    CollabParticipantUi, CollabPendingEditUi, CollabRejectUiCode, CollabShareEndpoint,
    CollabUiAction, CollabUiRole, CollabUiState, DiscoveredCollabEndpoint, RemotePresenceUi,
    COLLAB_PRESENCE_FRAME_INTERVAL_MS, MAX_COLLAB_DISCARDED_FIELDS,
    MAX_COLLAB_DISCARDED_NODE_LABEL_CHARS, MAX_COLLAB_DISPLAY_NAME_CHARS,
    MAX_COLLAB_SHARE_ENDPOINT_CHARS, MAX_COLLAB_UI_PARTICIPANTS, MAX_COLLAB_UI_SELECTION_IDS,
};
pub use color_picker::{hsv_to_rgb, parse_hex_alpha, parse_hex_rgb, rgb_to_hex, rgb_to_hsv};
pub use command::{
    BatchInsertItem, EditorCommand, EffectField, LayoutPropValue, NodeFlag, StrokeSide,
    StylePropValue, StylePropertyReplacement, VariableScalarPayload,
};
pub use command_node_attrs::{WidgetNumberField, WidgetTextField};
pub use command_promote::PromoteResult;
pub use command_style_replace::StyleReplaceError;
pub use component_browser_state::ComponentBrowserButton;
pub use components::{Component, ComponentLibrary, ComponentOption};
pub use compositing::{fill_blend_mode_at, node_blend_mode, node_mask_type};
pub use design_md::{extract_design_md_from_document, generate_design_md, parse_design_md};
pub use design_md_button_state::DesignMdButton;
pub use document_install::{DocumentInstallError, DocumentInstallReport, PreparedDocument};
pub use edit_transaction::{
    CompletedLocalEdit, EditOrigin, LocalEditCapture, LocalEditError, LocalEditOutcome,
};
pub use editor_ui_state::{
    AssetCenterTab, BooleanOp, CloneField, CloneFormState, CommitDiffPatch, CommitDiffSummary,
    CommitDiffView, CompositingPickerTarget, CustomPrompt, DesignMdPanelState, DesignMdRequest,
    EditorUiState, EmbedHost, ExportFormat, FileAction, FillType, FlexLayout, FontPickerPurpose,
    GitBranchPickerMode, GitCandidateFile, GitCommitSummary, GitDiffTarget, GitDiffView,
    GitFileEntry, GitOverflowView, GitPanelAction, GitPanelState, ImageAdjustmentField,
    ImageFillMode, LayerContextMenuState, LeftPanelTab, Locale, MergeConflictRow, MergeResolveFile,
    MergeResolveState, MissingFontSurface, PaddingEditMode, PageRenameState, PencilCursorStyle,
    PreviewDeviceKind, PreviewState, PromptCenterFocus, PromptCenterState, PromptFilter,
    PropertyTab, RecentFile, SceneFilter, SceneTemplateCenterState, SceneTemplateFocus,
    SizeToggleState, SlidesDrag, SlidesPanelState, SlidesPanelTarget, StyleImportState, ThemeMode,
    UpdateStatus, VariableRowFocus,
};
pub use export_dialog_state::ExportDialogButton;
pub use export_quick_menu_state::ExportQuickRow;
pub use figma_import_state::{FigmaImportButton, FigmaImportPage, FigmaImportSelection};
pub use fill_order::move_fill;
pub use fills::{
    first_fill_type, first_image_fill_summary, first_solid_fill_hex, first_solid_fill_opacity,
    first_solid_stroke_hex, node_effects, ImageFillSummary,
};
pub use geometry::{aggregate_bounds, own_bounds, union_aggregate_bounds, DocRect};
pub use git_button_state::GitButton;
pub use history::{EditorSnapshot, History, HISTORY_CAP};
pub use history_snapshot::{SharedComponents, SharedDoc};
pub use hoist_app_state::{hoist_app_state, UNPLANNED_APP_STATE_IDX};
pub use icon_picker_state::{IconPickerRemoteIcon, IconPickerRemoteState, IconifyLoadMoreRequest};
pub use id_allocator::{
    collect_document_ids, next_namespaced_counter, next_sequential_counter, DocumentIdAllocator,
    IdAllocError, IdAllocator, NamespacedIdAllocator, PeerNamespace, SequentialIdAllocator,
    MAX_PEER_NAMESPACE_LEN,
};
pub use image_aspect::aspect_matched_height;
pub use image_crop::{
    image_fill_body_is_crop, primary_image_fill_is_crop_editable, primary_image_fill_transform,
    translate_primary_image_crop,
};
pub use image_node_props::image_node_summary;
pub use instance_override::{
    apply_instance_override, resolve_instance_display_node,
    resolve_instance_display_node_for_anchor, split_instance_child_anchor, InstanceWriteScope,
    INSTANCE_DIRECT_PROPS,
};
pub use jian_ops_schema::node::MaskType;
pub use jian_ops_schema::style::BlendMode;
pub use jian_ops_schema::{DesignMdColor, DesignMdSpec, DesignMdTypography, PenDocument};
pub use mutators::EditorStateInvariant;
pub use node_defaults::{
    default_leaf_node_size, widget_default_size, DEFAULT_LEAF_NODE_SIZE, DEFAULT_TEXT_NODE_HEIGHT,
    DEFAULT_TEXT_NODE_WIDTH,
};
pub use node_id::NodeId;
pub use pen_node_ext::PenNodeExt;
pub use render_backend::*;
pub use selection::SelectionState;
pub use state::EditorState;
pub use statusbar_state::StatusBarButton;
pub use theme_presets::ThemePreset;
pub use tool::Tool;
pub use toolbar_state::{ToolbarAction, ToolbarHover};
pub use topbar_state::TopBarButton;
pub use ui_draft::{
    ColorPickerDrag, ColorPickerState, ColorTarget, LayerContextTarget, LayerRenameState,
    PathAnchorMenuState, PropertyFocus, UiDraftState, VariableUiState,
};
pub use uikit::{builtin_kits, ComponentCategory, KitComponent, UIKit};
pub use uikit_io::KitIoRequest;
pub use variables_panel_state::VariablesPanelButton;
pub use viewport::Viewport;
pub use walkers::ReorderDirection;
pub use web_sync::WebSyncError;
