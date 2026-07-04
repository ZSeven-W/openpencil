//! MCP (Model Context Protocol) request / response types.
//! Mirrors the wire shape `packages/pen-mcp` uses for its stdio +
//! HTTP server. v1 scope: protocol types + tool registry trait.
// Some ported element builders nest `json!` subtrees deeply enough to
// exceed the default macro recursion limit (128).
#![recursion_limit = "512"]
//!
//! ## op-editor-core port (Phase 5 Task 5.1)
//!
//! The MCP server was ported off shell-core's `Document` onto
//! `op_editor_core::EditorState` / `op_editor_core::EditorCommand`.
//! Read tools now snapshot an `EditorState` (canonical `PenDocument`);
//! write tools emit an `op_editor_core::EditorCommand` the host applies
//! via `EditorState::apply`.
//!
//! `set_node_collapsed` (`NodeFlag::Collapsed`) remains editor-chrome
//! state with no canonical schema field, so that tool returns a clean
//! `ToolFailed` instead of queueing a command `EditorState::apply`
//! would reject. See `component_tools.rs`.

use std::collections::BTreeMap;

pub mod batch_design;
#[cfg(test)]
mod batch_design_ref_tests;
pub mod batch_design_result;
#[cfg(test)]
mod batch_design_tests;
mod batch_direct_ops;
pub mod batch_get;
#[cfg(test)]
mod batch_get_tests;
mod batch_layered;
#[cfg(test)]
mod batch_layered_tests;
mod batch_page;
mod batch_program;
#[cfg(test)]
mod batch_program_tests;
pub mod bulk_vars;
#[cfg(test)]
mod bulk_vars_tests;
mod codegen_plan_store;
pub mod codegen_tools;
#[cfg(test)]
mod codegen_tools_tests;
pub mod component_tools;
#[cfg(test)]
mod component_tools_tests;
pub mod conversion_status;
#[cfg(test)]
mod conversion_status_tests;
pub mod conversion_tools;
#[cfg(test)]
mod conversion_tools_tests;
#[cfg(test)]
mod copy_node_tests;
#[cfg(feature = "debug-tools")]
pub mod debug_tools;
pub mod design_md_tools;
#[cfg(test)]
mod design_md_tools_tests;
pub mod design_prompt;
#[cfg(test)]
mod design_prompt_tests;
pub mod design_refine_result;
pub mod document_save;
#[cfg(test)]
mod document_save_tests;
pub mod editor_state_tool;
pub mod element_tools;
pub mod extra_read_tools;
#[cfg(test)]
mod extra_read_tools_tests;
pub mod guidelines_tools;
mod insert_node_args;
mod insert_node_data;
pub mod json_serializer;
pub mod lint_tools;
#[cfg(test)]
mod lint_tools_tests;
pub mod node_attr_tools;
#[cfg(test)]
mod node_attr_tools_tests;
mod node_shape_defaults;
pub mod open_document;
#[cfg(test)]
mod open_document_tests;
pub mod page_tools;
pub mod parser;
pub mod read_nodes;
#[cfg(test)]
mod read_nodes_tests;
pub mod read_tools;
pub mod read_tools_extra;
pub mod reparent_tools;
#[cfg(test)]
mod replace_node_tests;
pub mod scalar_vars;
#[cfg(test)]
mod scalar_vars_tests;
#[cfg(feature = "script")]
pub mod script_runner;
pub mod selected_ops_tools;
#[cfg(test)]
mod selected_ops_tools_tests;
#[cfg(test)]
mod selection_tools_tests;
pub mod style_guide_tools;
#[cfg(test)]
mod style_guide_tools_tests;
pub mod style_ops_tools;
#[cfg(test)]
mod style_ops_tools_tests;
#[cfg(test)]
pub mod test_fixtures;
pub mod theme_presets;
#[cfg(test)]
mod theme_presets_tests;
pub mod tools;
#[cfg(test)]
mod tools_tests;
mod update_node_data;
#[cfg(test)]
mod update_node_data_tests;
pub mod write_tools;
#[cfg(test)]
mod write_tools_tests;
// Cross-cutting tests for the crate spine — stdio dispatch + parser
// invariants + a few read-tool registry round-trips.
#[cfg(test)]
mod mcp_read_tests;
#[cfg(test)]
mod mcp_tests;

// The MCP command DTO is now `op_editor_core::EditorCommand` — the
// faithful port of the old shell-core `McpCommand`. Re-exported here
// under both names so existing `mcp::McpCommand` / `mcp::NodeFlag`
// call sites keep resolving while the wire layer speaks the new type.
pub use op_editor_core::{
    BatchInsertItem, EditorCommand, EditorCommand as McpCommand, NodeFlag, VariableScalarPayload,
};

// Re-export the public surface of submodules so callers can keep
// using `mcp::parse_tool_call` / `mcp::GetDocumentInfo` after the
// split. Mirrors the `widgets::*` re-export pattern.
pub use batch_design::{
    design_content_snapshot, design_skeleton_snapshot, DesignContent, DesignSkeleton,
};
pub use batch_design_result::{batch_design_snapshot, BatchDesign};
pub use batch_get::{batch_get_snapshot, BatchGet};
pub use bulk_vars::{
    get_variables_snapshot, set_themes_snapshot, set_variables_snapshot, GetVariables, SetThemes,
    SetVariables,
};
pub use codegen_tools::{
    codegen_assemble_snapshot, codegen_clean_snapshot, codegen_plan_snapshot,
    codegen_submit_chunk_snapshot, CodegenAssemble, CodegenClean, CodegenPlan, CodegenSubmitChunk,
};
pub use component_tools::{
    create_component_snapshot, delete_component_snapshot, instantiate_component_snapshot,
    rename_component_snapshot, set_node_collapsed_snapshot, CreateComponent, DeleteComponent,
    InstantiateComponent, RenameComponent, SetNodeCollapsed,
};
pub use conversion_status::{conversion_status_snapshot, ConversionStatus};
pub use conversion_tools::{
    upsert_component_snapshot, upsert_screen_snapshot, upsert_variables_snapshot,
    UpsertComponentTool, UpsertScreenTool, UpsertVariablesTool,
};
#[cfg(feature = "debug-tools")]
pub use debug_tools::{
    debug_logs_tail_snapshot, debug_screenshot_snapshot, debug_tools_enabled,
    debug_validation_report_snapshot, parse_screenshot_args, DebugLogsTail, DebugScreenshot,
    DebugValidationReport, ScreenshotRequest, ScreenshotTarget,
};
#[cfg(not(feature = "debug-tools"))]
pub fn debug_tools_enabled() -> bool {
    false
}
pub use design_md_tools::{
    export_design_md_snapshot, get_design_md_snapshot, set_design_md_snapshot, ExportDesignMd,
    GetDesignMd, SetDesignMd,
};
pub use design_prompt::{get_design_prompt_snapshot, GetDesignPrompt};
pub use design_refine_result::{design_refine_snapshot, DesignRefine};
pub use document_save::{save_document_snapshot, SaveDocument};
pub use extra_read_tools::{get_node_children_snapshot, ChildRecord, GetNodeChildren};
pub use guidelines_tools::{get_guidelines_snapshot, GetGuidelines};
pub use json_serializer::{response_to_json, tool_response_to_json};
pub use lint_tools::{lint_document_snapshot, LintDocument};
pub use node_attr_tools::{
    add_node_effect_snapshot, remove_node_effect_snapshot, set_ellipse_arc_snapshot,
    set_node_corner_radius_snapshot, set_node_fill_hex_snapshot, set_node_flip_snapshot,
    set_node_font_size_snapshot, set_node_font_weight_snapshot, set_node_name_snapshot,
    set_node_rotation_snapshot, set_node_stroke_hex_snapshot, set_node_stroke_side_width_snapshot,
    set_node_stroke_width_snapshot, set_node_text_snapshot, AddNodeEffect, RemoveNodeEffect,
    SetEllipseArc, SetNodeCornerRadius, SetNodeFillHex, SetNodeFlip, SetNodeFontSize,
    SetNodeFontWeight, SetNodeName, SetNodeRotation, SetNodeStrokeHex, SetNodeStrokeSideWidth,
    SetNodeStrokeWidth, SetNodeText,
};
pub use open_document::{open_document_snapshot, OpenDocument};
pub use page_tools::{
    add_page_snapshot, clear_selection_snapshot, cycle_active_axis_value_snapshot,
    delete_page_snapshot, duplicate_page_snapshot, redo_snapshot, remove_page_snapshot,
    rename_page_snapshot, reorder_page_snapshot, set_active_page_snapshot,
    set_active_tool_snapshot, set_node_hidden_snapshot, set_node_locked_snapshot,
    set_selection_set_snapshot, set_selection_snapshot, set_viewport_snapshot,
    toggle_node_selection_snapshot, undo_snapshot, AddPage, ClearSelection, CycleActiveAxisValue,
    DeletePage, DuplicatePage, Redo, RenamePage, ReorderPage, SetActivePage, SetActiveTool,
    SetNodeHidden, SetNodeLocked, SetSelection, SetSelectionSet, SetViewport, ToggleNodeSelection,
    Undo,
};
pub use parser::parse_tool_call;
pub use read_nodes::{read_nodes_snapshot, ReadNodes};
pub use reparent_tools::{copy_node_snapshot, move_node_snapshot, CopyNode, MoveNode};
pub use scalar_vars::{
    create_variable_snapshot, delete_variable_snapshot, rename_variable_snapshot,
    set_variable_boolean_snapshot, set_variable_number_snapshot, set_variable_string_snapshot,
    CreateVariable, DeleteVariable, RenameVariable, SetVariableBoolean, SetVariableNumber,
    SetVariableString,
};
pub use selected_ops_tools::{
    align_selected_snapshot, copy_selected_snapshot, cut_selected_snapshot,
    delete_selected_snapshot, duplicate_selected_snapshot, group_selected_snapshot,
    nudge_selected_snapshot, paste_clipboard_snapshot, reorder_selected_snapshot,
    ungroup_selected_snapshot, AlignSelected, CopySelected, CutSelected, DeleteSelected,
    DuplicateSelected, GroupSelected, NudgeSelected, PasteClipboard, ReorderSelected,
    UngroupSelected,
};
pub mod spawn_agents_tool;
pub use spawn_agents_tool::{spawn_agents_snapshot, SpawnAgents, SpawnSpec};
pub mod tool_search;
pub use editor_state_tool::{get_editor_state_snapshot, GetEditorState};
pub use style_guide_tools::{
    get_style_guide_snapshot, get_style_guide_tags_snapshot, GetStyleGuide, GetStyleGuideTags,
};
pub use style_ops_tools::{
    replace_all_matching_properties_snapshot, search_all_unique_properties_snapshot,
    ReplaceAllMatchingProperties, SearchAllUniqueProperties,
};
pub use theme_presets::{
    list_theme_presets_snapshot, load_theme_preset_snapshot, save_theme_preset_snapshot,
    ListThemePresets, LoadThemePreset, SaveThemePreset,
};
pub use tool_search::{tool_search, tool_search_snapshot, ToolSearch};
pub use tools::{
    count_nodes_snapshot, document_info_snapshot, find_empty_space_snapshot,
    find_node_by_name_snapshot, get_active_theme_snapshot, get_canvas_bounds_snapshot,
    get_component_snapshot, get_history_depth_snapshot, get_node_parent_snapshot,
    get_node_snapshot, get_selection_set_snapshot, get_viewport_snapshot, list_components_snapshot,
    list_node_kinds_snapshot, list_pages_snapshot, list_variables_snapshot, selection_snapshot,
    snapshot_layout_snapshot, CountNodes, FindEmptySpace, FindNodeByName, GetActiveTheme,
    GetCanvasBounds, GetComponent, GetDocumentInfo, GetHistoryDepth, GetNode, GetNodeParent,
    GetSelection, GetSelectionSet, GetViewport, ListComponents, ListNodeKinds, ListPages,
    ListVariables, NodeRecord, SnapshotLayout, VariableRecord,
};
pub use write_tools::{
    delete_node_snapshot, import_svg_snapshot, insert_node_snapshot, replace_node_snapshot,
    set_active_axis_value_snapshot, set_variable_color_snapshot, update_node_snapshot, DeleteNode,
    ImportSvg, InsertNode, ReplaceNode, SetActiveAxisValue, SetVariableColor, UpdateNode,
};

/// JSON-RPC-style request id. Strings + integers both supported by
/// the spec; we accept either over the wire.
#[derive(Debug, Clone, PartialEq)]
pub enum RequestId {
    Str(String),
    Num(i64),
}

/// Inbound tool invocation. `tool` is the registered tool name
/// (`insert_node`, `batch_design`, `design_skeleton`, etc); `arguments`
/// is the JSON object the tool expects.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    pub id: RequestId,
    pub tool: String,
    pub arguments: BTreeMap<String, String>,
}

/// Tool response — either a structured result object or an error.
/// Errors are typed enough for the LLM client to recover (e.g.
/// `MissingArgument` vs `InvalidArgument` vs `ToolFailed`).
#[derive(Debug, Clone, PartialEq)]
pub enum ToolResponse {
    Ok {
        id: RequestId,
        result: BTreeMap<String, String>,
        /// Optional mutation the host should apply to the live
        /// document. Write tools return one of these via
        /// `ToolOutcome::OkWithCommand`; read tools return None.
        /// The registry surfaces it so callers don't need to
        /// re-walk the tool list.
        command: Option<EditorCommand>,
        /// Pre-serialized nested-JSON result (from
        /// `ToolOutcome::OkJson`). When `Some`, the serializers emit it
        /// verbatim as the wire `result` instead of encoding `result`
        /// (the flat map) — so read tools match TS's arbitrary-JSON
        /// shapes byte-for-byte. `None` ⇒ use the flat-map encoding.
        json: Option<String>,
    },
    Err {
        id: RequestId,
        code: ToolErrorCode,
        message: String,
    },
}

/// Tool failure kind — matches JSON-RPC error categories. The MCP
/// server maps these to standard codes when serialising.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolErrorCode {
    MissingArgument,
    InvalidArgument,
    ToolFailed,
    UnknownTool,
    Internal,
}

/// Result of a tool's work — content + payload only. The
/// `ToolRegistry::dispatch` wrapper attaches the originating
/// `RequestId` so a misbehaving tool literally can't mint a
/// wrong id.
///
/// `OkWithCommand` carries an `EditorCommand` the host applies AFTER
/// dispatch: the tool stays `&self` (so the registry doesn't need
/// `Arc<Mutex<EditorState>>`), but write tools can still describe
/// their intent + the host serializes the mutation against the live
/// editor state via `EditorState::apply`.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolOutcome {
    Ok(BTreeMap<String, String>),
    OkWithCommand(BTreeMap<String, String>, EditorCommand),
    /// A read tool returning a TS-identical nested-JSON result. The
    /// `String` is the already-serialized JSON object (e.g.
    /// `serde_json::to_string(&value)`); it rides verbatim into the wire
    /// `result` (and the `content[].text` of the tools/call envelope),
    /// bypassing the flat string-map encoding. Used where TS `pen-mcp`
    /// returns arbitrary nested JSON (snapshot_layout, batch_get, …) so
    /// the two stacks serialize byte-identical tool results.
    OkJson(String),
    /// A write tool returning a TS-identical nested-JSON result AND a command
    /// the host applies (like `OkWithCommand`, but the wire `result` is the
    /// verbatim JSON instead of the flat string-map). Used by design tools
    /// (`design_refine`, `batch_design`) whose TS handlers return rich nested
    /// results (`fixes[]`, `layoutSnapshot`, `results[]`, …) while still
    /// mutating the document. The host applies `command`; the client sees `json`.
    OkJsonWithCommand(String, EditorCommand),
    Err(ToolErrorCode, String),
}

/// Trait every MCP tool implements. The MCP server walks its
/// `ToolRegistry`, looks up the requested tool, and forwards the
/// arguments. Tools return a `ToolOutcome`; the registry wraps it
/// with the originating request id to produce a `ToolResponse`.
pub trait McpTool: Send + Sync {
    fn name(&self) -> &str;
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome;
}

/// Registry — owned by the MCP server. v1 is a plain map; a future
/// version may add priority / per-tool auth / rate limits.
#[derive(Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, Box<dyn McpTool>>,
}

impl ToolRegistry {
    pub fn register(&mut self, tool: Box<dyn McpTool>) {
        let name = tool.name().to_string();
        self.tools.insert(name, tool);
    }
    pub fn dispatch(&self, call: ToolCall) -> ToolResponse {
        // The registry — not the tool — stamps the response id. Tools
        // never see the id; their `ToolOutcome` is content-only. This
        // makes id mismatch structurally impossible.
        let Some(tool) = self.tools.get(&call.tool) else {
            return ToolResponse::Err {
                id: call.id,
                code: ToolErrorCode::UnknownTool,
                message: format!("unknown tool: {}", call.tool),
            };
        };
        match tool.call(&call.arguments) {
            ToolOutcome::Ok(result) => ToolResponse::Ok {
                id: call.id,
                result,
                command: None,
                json: None,
            },
            ToolOutcome::OkWithCommand(result, command) => ToolResponse::Ok {
                id: call.id,
                result,
                command: Some(command),
                json: None,
            },
            ToolOutcome::OkJson(json) => ToolResponse::Ok {
                id: call.id,
                result: BTreeMap::new(),
                command: None,
                json: Some(json),
            },
            ToolOutcome::OkJsonWithCommand(json, command) => ToolResponse::Ok {
                id: call.id,
                result: BTreeMap::new(),
                command: Some(command),
                json: Some(json),
            },
            ToolOutcome::Err(code, message) => ToolResponse::Err {
                id: call.id,
                code,
                message,
            },
        }
    }
    pub fn names(&self) -> Vec<&str> {
        self.tools.keys().map(String::as_str).collect()
    }
    pub fn len(&self) -> usize {
        self.tools.len()
    }
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

/// Read line-delimited JSON-RPC from `reader`, dispatch each request
/// through `registry`, write each response (followed by `\n`) to
/// `writer`. Loops until EOF or a write error. Pure stdlib I/O.
///
/// **Read-only path** — when a tool returns a command-bearing response
/// (`ToolOutcome::OkWithCommand` or `OkJsonWithCommand`, i.e. any
/// `ToolResponse::Ok { command: Some(_), .. }`), this function REJECTS it as
/// `ToolErrorCode::Internal` so the client never sees a misleading success
/// for a mutation that was never applied. Hosts that need write tools should
/// use [`run_stdio_with_applier`].
pub fn run_stdio<R: std::io::BufRead, W: std::io::Write>(
    registry: &ToolRegistry,
    reader: &mut R,
    writer: &mut W,
) -> std::io::Result<()> {
    run_stdio_with_applier(registry, reader, writer, |_| {
        // No applier → write tools are unsupported on this path.
        false
    })
}

/// Variant of [`run_stdio`] that accepts an applier closure for MCP
/// write commands. The applier returns `true` when the command
/// actually mutated the editor state and `false` when it rejected.
///
/// Wire behaviour:
/// - read tools (no command) → response written verbatim.
/// - write tools, applier returns `true` → response written with
///   `result` carrying the tool's payload.
/// - write tools, applier returns `false` → demoted to
///   `ToolErrorCode::Internal` with a message describing which
///   command rejected.
pub fn run_stdio_with_applier<R, W, F>(
    registry: &ToolRegistry,
    reader: &mut R,
    writer: &mut W,
    mut apply: F,
) -> std::io::Result<()>
where
    R: std::io::BufRead,
    W: std::io::Write,
    F: FnMut(&EditorCommand) -> bool,
{
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            return Ok(()); // EOF
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some(call) = parse_tool_call(trimmed) else {
            // Surface a typed error with the request id when we can
            // recover it so the client can fail fast. If even the id
            // is missing we drop the line — nothing to correlate.
            if let Some(id) = parser::extract_request_id(trimmed) {
                let err = ToolResponse::Err {
                    id,
                    code: ToolErrorCode::InvalidArgument,
                    message: "malformed tool call: unparseable or structured arguments".into(),
                };
                writeln!(writer, "{}", response_to_json(&err))?;
                writer.flush()?;
            }
            continue;
        };
        let mut response = registry.dispatch(call);
        // Apply any queued command before reporting success. If
        // application fails (applier returns false), demote the
        // ToolResponse::Ok to a typed Err so the client sees the
        // mutation didn't land.
        if let ToolResponse::Ok {
            id,
            command: Some(cmd),
            ..
        } = &response
        {
            if !apply(cmd) {
                let id = id.clone();
                response = ToolResponse::Err {
                    id,
                    code: ToolErrorCode::Internal,
                    message: format!("host rejected command: {cmd:?}"),
                };
            }
        }
        // Tool results use the MCP-spec `content[]` envelope (with
        // `isError` for tool-level failures); only transport/parse errors
        // above use the JSON-RPC `error` shape.
        writeln!(writer, "{}", tool_response_to_json(&response))?;
        writer.flush()?;
    }
}
