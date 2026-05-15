//! MCP (Model Context Protocol) request / response types.
//! Mirrors the wire shape `packages/pen-mcp` uses for its stdio +
//! HTTP server. v1 scope: protocol types + tool registry trait.
//! Real stdio listener + HTTP server land in `openpencil-desktop`
//! (or a dedicated `openpencil-mcp` binary) once the routing
//! decisions are made; the data shape here lets that work proceed
//! without redesign.

use std::collections::BTreeMap;

pub mod json_serializer;
pub mod parser;
pub mod tools;
pub mod extra_read_tools;
#[cfg(test)] mod extra_read_tools_tests;
#[cfg(test)] mod tools_tests;
pub mod write_tools;
pub mod batch_design;
pub mod scalar_vars;
pub mod component_tools;
#[cfg(test)] mod component_tools_tests;
pub mod node_attr_tools;
#[cfg(test)] mod node_attr_tools_tests;
pub mod selected_ops_tools;
#[cfg(test)] mod selected_ops_tools_tests;
#[cfg(test)] mod write_tools_tests;
#[cfg(test)] mod copy_node_tests;
#[cfg(test)] mod replace_node_tests;
#[cfg(test)] mod batch_design_tests;
#[cfg(test)] mod scalar_vars_tests;

// Re-export the public surface of submodules so callers can keep
// using `mcp::parse_tool_call` / `mcp::GetDocumentInfo` after the
// split. Mirrors the `widgets::*` re-export pattern.
pub use parser::parse_tool_call;
pub use tools::{
    count_nodes_snapshot, document_info_snapshot, find_node_by_name_snapshot,
    get_active_theme_snapshot, get_canvas_bounds_snapshot, get_component_snapshot,
    get_history_depth_snapshot, get_node_parent_snapshot, get_node_snapshot,
    get_selection_set_snapshot, get_viewport_snapshot, list_components_snapshot,
    list_node_kinds_snapshot, list_pages_snapshot, list_variables_snapshot,
    selection_snapshot, snapshot_layout_snapshot, CountNodes, FindNodeByName,
    GetActiveTheme, GetCanvasBounds, GetComponent, GetDocumentInfo, GetHistoryDepth,
    GetNode, GetNodeParent, GetSelection, GetSelectionSet, GetViewport, ListComponents,
    ListNodeKinds, ListPages, ListVariables, NodeRecord, SnapshotLayout, VariableRecord,
};
pub use write_tools::{
    copy_node_snapshot, delete_node_snapshot, insert_node_snapshot, move_node_snapshot,
    replace_node_snapshot, set_active_axis_value_snapshot, set_variable_color_snapshot,
    update_node_snapshot, CopyNode, DeleteNode, InsertNode, MoveNode, ReplaceNode,
    SetActiveAxisValue, SetVariableColor, UpdateNode,
};
pub use component_tools::{
    add_page_snapshot, clear_selection_snapshot, create_component_snapshot,
    cycle_active_axis_value_snapshot, delete_component_snapshot, delete_page_snapshot,
    duplicate_page_snapshot, instantiate_component_snapshot, redo_snapshot,
    rename_component_snapshot, rename_page_snapshot, reorder_page_snapshot,
    set_active_page_snapshot, set_active_tool_snapshot, set_node_collapsed_snapshot,
    set_node_hidden_snapshot, set_node_locked_snapshot, set_selection_set_snapshot,
    set_selection_snapshot, set_viewport_snapshot, toggle_node_selection_snapshot,
    undo_snapshot, AddPage, ClearSelection, CreateComponent, CycleActiveAxisValue,
    DeleteComponent, DeletePage, DuplicatePage, InstantiateComponent,
    Redo, RenameComponent, RenamePage, ReorderPage, SetActivePage, SetActiveTool,
    SetNodeCollapsed, SetNodeHidden, SetNodeLocked, SetSelection, SetSelectionSet,
    SetViewport, ToggleNodeSelection, Undo,
};
pub use node_attr_tools::{
    set_node_corner_radius_snapshot, set_node_fill_hex_snapshot,
    set_node_font_size_snapshot, set_node_font_weight_snapshot,
    set_node_name_snapshot, set_node_rotation_snapshot,
    set_node_stroke_hex_snapshot, set_node_stroke_width_snapshot,
    set_node_text_snapshot, SetNodeCornerRadius, SetNodeFillHex, SetNodeFontSize,
    SetNodeFontWeight, SetNodeName, SetNodeRotation, SetNodeStrokeHex,
    SetNodeStrokeWidth, SetNodeText,
};
pub use selected_ops_tools::{
    align_selected_snapshot, copy_selected_snapshot, cut_selected_snapshot,
    delete_selected_snapshot, duplicate_selected_snapshot, group_selected_snapshot,
    nudge_selected_snapshot, paste_clipboard_snapshot, reorder_selected_snapshot,
    ungroup_selected_snapshot, AlignSelected, CopySelected, CutSelected, DeleteSelected,
    DuplicateSelected, GroupSelected, NudgeSelected, PasteClipboard, ReorderSelected,
    UngroupSelected,
};
pub use batch_design::{
    batch_design_snapshot, design_content_snapshot, design_refine_snapshot,
    design_skeleton_snapshot, BatchDesign, DesignContent, DesignRefine, DesignSkeleton,
};
pub use extra_read_tools::{
    get_node_children_snapshot, ChildRecord, GetNodeChildren,
};
pub use json_serializer::response_to_json;
pub use scalar_vars::{
    create_variable_snapshot, delete_variable_snapshot, rename_variable_snapshot,
    set_variable_boolean_snapshot, set_variable_number_snapshot,
    set_variable_string_snapshot, CreateVariable, DeleteVariable, RenameVariable,
    SetVariableBoolean, SetVariableNumber, SetVariableString,
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
        command: Option<McpCommand>,
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
/// wrong id (codex BLOCK: passing `&ToolCall` to tools left id
/// preservation as a convention only; this shape enforces it
/// structurally).
///
/// `OkWithCommand` carries a `McpCommand` the host applies AFTER
/// dispatch: the tool stays `&self` (so the registry doesn't
/// need `Arc<Mutex<Document>>`), but write tools can still
/// describe their intent + the host serializes the mutation
/// against the live document.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolOutcome {
    Ok(BTreeMap<String, String>),
    OkWithCommand(BTreeMap<String, String>, McpCommand),
    Err(ToolErrorCode, String),
}

/// Document mutation a tool wants the host to apply. The tool
/// validates its arguments (`call(&self, ...)`) and returns
/// `OkWithCommand(result, command)`; the host then calls
/// `Document::apply_mcp_command(command)`. This pattern keeps
/// the `McpTool` trait `&self` (so trait objects + the registry
/// stay simple) while still admitting write tools.
///
/// Variants extend as write tools land. Today:
/// - `SetVariableColor { name, hex }` — routes through
///   `VariableTable::set_color_hex` with its full correctness
///   chain (subset match / no default clobber / no other-axis
///   shadow / history snapshot).
/// - `SetActiveAxisValue { axis, value }` — pins a theme axis
///   directly (vs. `cycle_active_axis_value` which advances).
/// - `InsertNode { ... }` — creates a fresh node on the active
///   page with the supplied bounds + fill. The applier
///   allocates an id past `max_node_id()` so it can't collide
///   with existing nodes even after deletes.
#[derive(Debug, Clone, PartialEq)]
pub enum McpCommand {
    SetVariableColor {
        name: String,
        hex: String,
    },
    SetActiveAxisValue {
        axis: String,
        value: String,
    },
    /// Advance the active value for `axis` to the next entry in
    /// its `values` list (wrapping back to the first). When the
    /// axis has no current selection the call seeds it to the
    /// first value. Rejects unknown axes and axes with an empty
    /// values list. Mirrors `Document::theme()` axis cycling so
    /// LLM-driven theme demos don't need to know each value name.
    CycleActiveAxisValue {
        axis: String,
    },
    InsertNode {
        kind: String,
        name: String,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        fill_hex: Option<String>,
    },
    /// Patch fields on an existing node. Every field is optional;
    /// `None` leaves the live value unchanged. Bounds writes
    /// replace coordinates piecemeal — caller can move (x, y)
    /// without resizing or resize (w, h) without moving.
    UpdateNode {
        node_id: u64,
        x: Option<i32>,
        y: Option<i32>,
        width: Option<i32>,
        height: Option<i32>,
        name: Option<String>,
        fill_hex: Option<String>,
    },
    /// Remove a node + all its descendants from its parent. The
    /// applier walks pages to find the parent vec containing the
    /// id and `retain`s it out. Returns false when the id doesn't
    /// resolve OR points at a Page root (use page mutators for
    /// page deletion).
    DeleteNode { node_id: u64 },
    /// Reparent a node. `target_parent_id == 0` reparents to the
    /// page root of the currently-active page. Non-zero ids must
    /// resolve to an existing node; the applier rejects moves
    /// where the target would create a cycle (target is a
    /// descendant of the moved node).
    MoveNode {
        node_id: u64,
        target_parent_id: u64,
    },
    /// Deep-clone a node + every descendant under a new parent.
    /// `target_parent_id == 0` puts the copy at the active page
    /// root. Fresh ids are allocated by the applier starting past
    /// `max_node_id()` so the clone can't collide with any live
    /// node. The wire result is `{"wrote": "true"}` only — clone
    /// ids aren't surfaced today because `ToolOutcome` is built
    /// before apply runs; callers that need the new ids must
    /// re-query (e.g. `list_pages` + walk). A future patch may
    /// thread the allocator back to the tool to surface
    /// `new_root_id`, but the current contract is fire-and-forget.
    CopyNode {
        node_id: u64,
        target_parent_id: u64,
    },
    /// Replace an existing node with a freshly-built one at the
    /// same parent slot + same index. Captures the same shape as
    /// `InsertNode` (kind / name / bounds / fill_hex) plus the
    /// target `node_id` to swap. Useful when the LLM wants to
    /// change kind (rect → ellipse) or radically alter the node
    /// in one atomic op rather than via incremental `UpdateNode`
    /// patches. The new node gets a fresh id past `max_node_id()`
    /// so the wire response is still fire-and-forget (callers
    /// re-query to learn the new id).
    ///
    /// Bounded scope: today only the leaf-style fields land on
    /// the replacement node. A future patch may grow this to
    /// carry children / a full subtree once a JSON Node parser
    /// lives on the host. The current contract matches what TS
    /// `replace_node` accepts for primitives, minus children.
    ///
    /// **Destructive on containers**: replacing a Frame / Group
    /// / node-with-children drops every descendant of the old
    /// node. To prevent silent data loss the applier REFUSES the
    /// swap when the target has children unless `drop_children`
    /// is set to `true`. Callers that genuinely want to discard
    /// the subtree must opt in; otherwise use `update_node` to
    /// patch a container in place.
    ReplaceNode {
        node_id: u64,
        kind: String,
        name: String,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        fill_hex: Option<String>,
        /// Required confirmation when the target has children.
        /// `false` (the default) makes the applier refuse a
        /// destructive swap; `true` is explicit consent.
        drop_children: bool,
    },
    /// Insert N leaf nodes on the active page in one atomic
    /// shot. Mirrors TS `batch_design` for the leaf subset
    /// (frame / group / rect / ellipse / polygon / line / text /
    /// path). Apply path validates EVERY descriptor before any
    /// mutation; a single bad entry rejects the whole batch so
    /// callers never see a partial design. Each emitted node
    /// gets a fresh non-colliding id from `next_node_id_seed`,
    /// advanced for the batch.
    BatchInsert {
        items: Vec<BatchInsertItem>,
    },
    /// Set a non-color scalar variable's value (Number / String /
    /// Boolean). Mirrors `SetVariableColor` for the other three
    /// `VariableKind`s. The applier routes through
    /// `VariableTable::set_scalar` which honors active-theme
    /// routing identically to set_color_hex.
    SetVariableScalar {
        name: String,
        scalar: VariableScalarPayload,
    },
    /// Create a new theme-agnostic scalar variable. `kind` is one
    /// of `"color"` / `"number"` / `"boolean"` / `"string"`;
    /// `default_value` is parsed per kind (hex for color, decimal
    /// for number, `"true"`/`"false"` for boolean, free text for
    /// string). Rejects empty / duplicate names + bad defaults.
    CreateVariable {
        name: String,
        kind: String,
        default_value: String,
    },
    /// Delete a variable by name. Also drops any node `$ref`s
    /// pointing at it. Rejects unknown names.
    DeleteVariable {
        name: String,
    },
    /// Rename a variable + rewrite every node `$ref` that points
    /// at it. Rejects unknown `old_name`, empty `new_name`, or a
    /// `new_name` that collides with a different variable.
    RenameVariable {
        old_name: String,
        new_name: String,
    },
    /// Instantiate a registered component on the active page. The
    /// applier deep-clones the component's root subtree with fresh
    /// ids past `max_node_id()` and appends it to the active page's
    /// top-level children. Mirrors TS's drag-from-Components-panel
    /// insertion. Returns false at apply time when the component
    /// id is unknown.
    InstantiateComponent {
        component_id: u64,
    },
    /// Promote an existing Frame / Group node to a registered
    /// component. `node_id` must resolve on the active page +
    /// must be a Frame or Group (the applier rejects other
    /// kinds). The component is keyed by the node's id; instances
    /// later spawn via `InstantiateComponent`.
    CreateComponent {
        node_id: u64,
        name: String,
    },
    /// Remove a component from the registry by id. Live instances
    /// already dropped on the page are NOT affected — they're
    /// independent clones at apply time.
    DeleteComponent {
        component_id: u64,
    },
    /// Rename a registered component. The applier rejects unknown
    /// ids and empty / whitespace-only names.
    RenameComponent {
        component_id: u64,
        name: String,
    },
    /// Switch the active page. Subsequent inserts / batch_design /
    /// design_* commands target the new active page.
    SetActivePage {
        index: u32,
    },
    /// Append a fresh empty page to the document + switch active
    /// to it. Mirrors TS `addPage()`. Applier returns false on
    /// id-space exhaustion (same guard as the rest of the write
    /// tools).
    AddPage,
    /// Set a page's display name. Rejects out-of-range indices
    /// and empty / whitespace-only names.
    RenamePage {
        index: u32,
        name: String,
    },
    /// Remove a page by index. The applier reuses the existing
    /// `Document::remove_page` which keeps the active page valid.
    DeletePage {
        index: u32,
    },
    /// Duplicate the page at `index` (clone + insert right after).
    /// Switches active page to the clone. Mirrors TS
    /// `duplicatePage(idx)`.
    DuplicatePage {
        index: u32,
    },
    /// Move the page at `from` to `to`. `to` is clamped into
    /// `[0, page_count)`. The applier keeps `active_page_index`
    /// tracking the moved page.
    ReorderPage {
        from: u32,
        to: u32,
    },
    /// Clear the current multi-select. After this command,
    /// `get_selection` reports id=0 / kind="none". Cheap reset
    /// for LLM workflows that need to start clean before a
    /// targeted selection.
    ClearSelection,
    /// Set the selection to a single node by id. Mirrors a click
    /// on the canvas. Rejects unknown ids; the applier walks
    /// every page to find the target so this works across
    /// active-page boundaries.
    SetSelection { node_id: u64 },
    /// Set canvas pan + zoom. Any subset of pan_x / pan_y /
    /// zoom_percent can be `None` to leave that axis untouched.
    /// Zoom is `int * 100`-scaled (zoom_percent=100 == 1.0×);
    /// applier clamps to a sane visual range so the canvas
    /// can't be zoomed to a degenerate size.
    SetViewport {
        pan_x: Option<i32>,
        pan_y: Option<i32>,
        zoom_percent: Option<i32>,
    },
    /// Flip a single boolean flag on a node. The applier walks
    /// every page to find the target. Rejects unknown ids.
    SetNodeFlag {
        node_id: u64,
        flag: NodeFlag,
        value: bool,
    },
    /// Set the active canvas tool (select / rect / ellipse / etc).
    /// Mirrors a click on the left toolbar. Unknown tool strings
    /// reject at apply time.
    SetActiveTool {
        tool: String,
    },
    /// Pop the last history snapshot off the undo stack.
    /// Returns false when the past stack is empty.
    Undo,
    /// Push the last undone snapshot back. Returns false when
    /// the redo stack is empty.
    Redo,
    /// Duplicate the currently-selected node + select the clone.
    /// `offset_px` shifts the clone by that many doc-px (defaults
    /// to 10 if 0 is passed — Document::duplicate_selected uses
    /// the value literally, so 0 == "same position"). Returns
    /// false when nothing is selected.
    DuplicateSelected {
        offset_px: i32,
    },
    /// Delete the currently-selected node. Returns false when
    /// nothing is selected. The applier pushes a history snapshot
    /// before the mutation so undo restores the node.
    DeleteSelected,
    /// Translate the currently-selected node by (dx, dy) doc-px.
    /// Mirrors the arrow-key nudge. Returns false when nothing
    /// is selected.
    NudgeSelected {
        dx: i32,
        dy: i32,
    },
    /// Cmd+G equivalent — wrap the multi-selected siblings in a
    /// new Group node. Returns false at apply time when the
    /// selection is empty / single-node or spans across parents.
    GroupSelected,
    /// Cmd+Shift+G equivalent — replace a selected Group with
    /// its children (in place). Returns false when the anchor
    /// is not a Group.
    UngroupSelected,
    /// Move the currently-selected node up or down in z-order.
    /// `direction` is "up" (forward) or "down" (back). Mirrors
    /// the layer-panel [/] shortcut + the "Bring forward / Send
    /// backward" right-click action.
    ReorderSelected {
        direction: String,
    },
    /// Set the rotation (in degrees) on `node_id`. Mirrors the
    /// PropertyPanel rotation input. The applier walks every
    /// page to find the target; rejects unknown ids.
    SetNodeRotation {
        node_id: u64,
        degrees: f32,
    },
    /// Set the text content on a Text-kind node. Rejects when
    /// the target's kind isn't Text. Other kinds keep their
    /// `text` field untouched.
    SetNodeText {
        node_id: u64,
        text: String,
    },
    /// Set corner-radius (doc-px) on a node. Honored at paint
    /// time for Rect / Frame; other kinds accept the write but
    /// the radius is invisible. Rejects negative values + nan.
    SetNodeCornerRadius {
        node_id: u64,
        radius: f32,
    },
    /// Set the font size (doc-px) on a Text-kind node. Rejects
    /// non-Text kinds and non-positive sizes.
    SetNodeFontSize {
        node_id: u64,
        font_size: f32,
    },
    /// Set the font weight (OpenType range 1..=1000) on a Text-kind
    /// node. Rejects non-Text kinds and out-of-range weights so the
    /// per-codepoint typeface cache lookup stays well-formed.
    SetNodeFontWeight {
        node_id: u64,
        font_weight: u16,
    },
    /// Set the stroke color on a node by id. If the node has an
    /// existing stroke the color is overwritten in-place; otherwise
    /// a fresh `Stroke { color, width: 1.0 }` is attached so the
    /// node paints with a visible 1 doc-px outline.
    SetNodeStrokeHex {
        node_id: u64,
        hex: String,
    },
    /// Set the stroke width (doc-px) on a node by id. Width = 0
    /// clears the stroke (matches the property panel's "remove
    /// stroke" behavior). Width > 0 on a node with no stroke
    /// attaches a fresh black-default stroke at that width so the
    /// caller doesn't have to issue a paired `set_node_stroke_hex`
    /// first.
    SetNodeStrokeWidth {
        node_id: u64,
        width: f32,
    },
    /// Apply alignment / distribution to the current selection.
    /// `action` is one of `"left"`, `"center_h"`, `"right"`,
    /// `"top"`, `"center_v"`, `"bottom"`, `"distribute_h"`,
    /// `"distribute_v"`. Distribute variants silently no-op for
    /// fewer than 3 selected nodes; align variants on a single
    /// top-level node also no-op (no useful reference frame).
    AlignSelected {
        action: String,
    },
    /// Set the fill color on a node by id. Mirrors the existing
    /// `set_node_stroke_hex` shape (LLM ergonomic: one-call color
    /// change, no need to pass the other `update_node` fields).
    /// `update_node` keeps doing piecemeal multi-field writes.
    SetNodeFillHex {
        node_id: u64,
        hex: String,
    },
    /// Rename a node by id. Mirrors `update_node` but takes only
    /// the name so the LLM doesn't have to thread the other
    /// optional fields.
    SetNodeName {
        node_id: u64,
        name: String,
    },
    /// Cmd+C parity — deep-clone the current selection into the
    /// document's internal clipboard. False at apply time when
    /// nothing is selected.
    CopySelected,
    /// Cmd+X parity — copy the selection, then delete it.
    /// History snapshot follows the existing `delete_selected`
    /// path so undo restores both clipboard and tree.
    CutSelected,
    /// Cmd+V parity — paste the document clipboard as top-level
    /// siblings on the active page, offset by `offset_px` doc-px.
    /// Mints fresh ids past `max_node_id()`. Replaces selection
    /// with the new ids. Apply-time false when the clipboard is
    /// empty or id-space is exhausted.
    PasteClipboard {
        offset_px: i32,
    },
    /// Shift-click semantics — toggle a single node's membership
    /// in the multi-selection set. Mirrors `Document::toggle_selection`:
    /// if `node_id` is already selected it's removed (and the
    /// anchor reassigned to the last surviving id); otherwise it's
    /// added as the new anchor. Rejects unknown ids.
    ToggleNodeSelection {
        node_id: u64,
    },
    /// Replace the multi-selection (`Document.selected_set`) with
    /// the supplied list of node ids. The applier walks every
    /// page to resolve each id; unknown ids are dropped silently
    /// (per Electron parity — a stale id from a panel click that
    /// raced a delete should not cause the whole call to fail).
    /// Empty list clears the selection.
    SetSelectionSet {
        node_ids: Vec<u64>,
    },
}

/// Which boolean property `SetNodeFlag` should write. Three
/// LLM-visible flags map 1:1 to the existing `Node` fields:
/// `hidden` (layer panel eye toggle), `locked` (layer panel
/// padlock toggle), `collapsed` (layer panel disclosure toggle).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeFlag {
    Hidden,
    Locked,
    Collapsed,
}

/// Wire-friendly value payload for `McpCommand::SetVariableScalar`.
/// Mirrors `document::VariableScalar` but stays a plain enum so
/// the wire layer doesn't reach into shell-core's document API.
#[derive(Debug, Clone, PartialEq)]
pub enum VariableScalarPayload {
    Number(f64),
    String(String),
    Boolean(bool),
}

/// Per-item descriptor for `McpCommand::BatchInsert`. Same shape
/// as `InsertNode`'s args; carried in a Vec so the applier can
/// validate-then-mutate the whole set atomically.
#[derive(Debug, Clone, PartialEq)]
pub struct BatchInsertItem {
    pub kind: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub fill_hex: Option<String>,
}

/// Trait every MCP tool implements. The MCP server walks its
/// `ToolRegistry`, looks up the requested tool, and forwards the
/// arguments. Tools return a `ToolOutcome`; the registry wraps it
/// with the originating request id to produce a `ToolResponse`.
pub trait McpTool: Send + Sync {
    fn name(&self) -> &str;
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome;
}

/// Registry — owned by the MCP server. v1 is a plain HashMap; a
/// future version may add priority / per-tool auth / rate limits.
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
        // makes id mismatch structurally impossible (codex BLOCK:
        // passing the id to tools left enforcement as convention).
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
            },
            ToolOutcome::OkWithCommand(result, command) => ToolResponse::Ok {
                id: call.id,
                result,
                command: Some(command),
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
/// `writer`. Loops until EOF or a write error. Pure stdlib I/O —
/// works on top of stdin/stdout, a TCP stream, or in-memory bufs
/// for tests. The actual `openpencil-mcp` binary just wraps this
/// with `BufReader::new(stdin())` + `stdout()`.
///
/// **Read-only path** — when a tool returns `ToolOutcome::
/// OkWithCommand`, this function REJECTS it as `ToolErrorCode::
/// Internal` so the client never sees a misleading success for
/// a mutation that was never applied (codex stop-gate: previously
/// the response was written without applying the command, so
/// clients saw "wrote: true" on a no-op). Hosts that need write
/// tools should use [`run_stdio_with_applier`] which threads in
/// a closure for command application.
pub fn run_stdio<R: std::io::BufRead, W: std::io::Write>(
    registry: &ToolRegistry,
    reader: &mut R,
    writer: &mut W,
) -> std::io::Result<()> {
    run_stdio_with_applier(registry, reader, writer, |_| {
        // No applier → write tools are unsupported on this path.
        // Returning false demotes ToolResponse::Ok to an Err so
        // the client doesn't see a fake success.
        false
    })
}

/// Variant of [`run_stdio`] that accepts an applier closure for
/// MCP write commands. The applier returns `true` when the
/// command actually mutated the document (caller may push undo
/// snapshots etc.) and `false` when it rejected — typically
/// because the document's state drifted between tool validation
/// (snapshotted) and command application (live).
///
/// Wire behaviour:
/// - read tools (no command) → response written verbatim.
/// - write tools, applier returns `true` → response written with
///   `result` carrying the tool's payload; the host has already
///   applied the mutation.
/// - write tools, applier returns `false` → demoted to
///   `ToolErrorCode::Internal` with message describing which
///   command rejected, so the client knows the mutation didn't
///   land.
pub fn run_stdio_with_applier<R, W, F>(
    registry: &ToolRegistry,
    reader: &mut R,
    writer: &mut W,
    mut apply: F,
) -> std::io::Result<()>
where
    R: std::io::BufRead,
    W: std::io::Write,
    F: FnMut(&McpCommand) -> bool,
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
            // Codex stop-gate: structured-arg rejection (or any
            // other parse failure) used to silently `continue`,
            // leaving JSON-RPC clients waiting on a correlated
            // response that never came. Surface a typed error
            // with the request id when we can recover it so the
            // client can fail fast. If even the id is missing
            // (e.g. wire-level malformed JSON, no id field at
            // all) we drop the line — there's nothing to
            // correlate against.
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
        writeln!(writer, "{}", response_to_json(&response))?;
        writer.flush()?;
    }
}

