//! `EditorCommand` — the editor-mutation DTO.
//!
//! A serde-free description of one editor mutation. The MCP server is
//! one producer (a tool validates its arguments, then emits an
//! `EditorCommand` for the host to apply); a keyboard handler, a CLI,
//! or a test is another. The DTO + [`EditorState::apply`] live here so
//! every producer goes through the same pre-validate-then-mutate apply
//! path, and so a future `op-mcp` crate can depend on `op-editor-core`
//! one-way (no dependency cycle).
//!
//! Ported from `openpencil-shell-core::mcp::McpCommand`. Two adaptations
//! to the `op-editor-core` model:
//!
//!   - Node targets are [`NodeId`] (the string newtype) rather than the
//!     `u64` the shell-core wire layer carried. There is no
//!     `from_mcp_u64` round-trip: the canonical `.op` schema's ids are
//!     strings, so commands carry them directly.
//!   - Node payloads describe leaf geometry with plain scalars; the
//!     applier builds the canonical `jian_ops_schema::PenNode` variant.
//!
use crate::node_id::NodeId;
use crate::walkers::ReorderDirection;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::variable::VariableDefinition;
use jian_ops_schema::DesignMdSpec;
use std::collections::BTreeMap;

/// Which boolean property [`EditorCommand::SetNodeFlag`] writes. The
/// canonical `PenNodeBase` carries `visible` + `locked`; `Collapsed` is
/// an editor-UI-only flag with no schema field, so the applier
/// rejects it (documented in the apply module).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeFlag {
    Hidden,
    Locked,
    Collapsed,
}

/// Which scalar parameter [`EditorCommand::SetEffectParam`] writes.
/// `OffsetX` / `OffsetY` / `Blur` / `Spread` target a Shadow effect;
/// `Radius` targets a Blur / BackgroundBlur effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectField {
    OffsetX,
    OffsetY,
    Blur,
    Spread,
    Radius,
}

/// Wire-friendly value payload for [`EditorCommand::SetVariableScalar`]
/// — a non-color scalar variable's new value.
#[derive(Debug, Clone, PartialEq)]
pub enum VariableScalarPayload {
    Number(f64),
    String(String),
    Boolean(bool),
}

/// Value payload for [`EditorCommand::SetNodeLayoutProp`] — covers the
/// full shape set used by layout / text properties that do not have
/// their own typed `EditorCommand` variants.
///
/// Variants:
/// - `Number(f64)` — plain numeric value (gap, letterSpacing, lineHeight,
///   opacity, and sizing numbers for width/height).
/// - `Keyword(String)` — a string keyword (textAlign, textGrowth,
///   alignItems, justifyContent, and sizing keywords like "fit_content").
/// - `NumberArray(Vec<f64>)` — padding as a number array.
/// - `Bool(bool)` — boolean layout flags such as `clipContent`.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutPropValue {
    Number(f64),
    Keyword(String),
    NumberArray(Vec<f64>),
    Bool(bool),
}

/// Value payload used by batch style-property replacement commands.
#[derive(Debug, Clone, PartialEq)]
pub enum StylePropValue {
    String(String),
    Number(f64),
    NumberArray(Vec<f64>),
}

/// One `replace_all_matching_properties` rule after MCP parsing.
#[derive(Debug, Clone, PartialEq)]
pub struct StylePropertyReplacement {
    pub property: String,
    pub from: StylePropValue,
    pub to: StylePropValue,
}

/// Per-item descriptor for [`EditorCommand::BatchInsert`]. Same shape as
/// `InsertNode`'s args; carried in a Vec so the applier can
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

/// One editor mutation. The applier validates every argument BEFORE
/// touching the document so a bad arg never half-mutates it.
#[derive(Debug, Clone, PartialEq)]
pub enum EditorCommand {
    /// Set a color variable's value by name. Routes through the active-
    /// theme discipline (`set_variable_color`).
    SetVariableColor { name: String, hex: String },
    /// Pin a theme axis to a specific value.
    SetActiveAxisValue { axis: String, value: String },
    /// Advance a theme axis to its next declared value (wrapping).
    CycleActiveAxisValue { axis: String },
    /// Create a fresh leaf node on the active page.
    InsertNode {
        kind: String,
        name: String,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        fill_hex: Option<String>,
        /// `NodeId::NONE` inserts at the active/requested page root.
        target_parent: NodeId,
        /// `None` inserts on the active page; `Some` targets a page id
        /// or legacy page index.
        page_id: Option<String>,
    },
    /// Patch fields on an existing node — every field optional.
    UpdateNode {
        node_id: NodeId,
        x: Option<i32>,
        y: Option<i32>,
        width: Option<i32>,
        height: Option<i32>,
        name: Option<String>,
        fill_hex: Option<String>,
        /// `None` updates on the active page; `Some` targets a page id
        /// or legacy page index.
        page_id: Option<String>,
    },
    /// Shallow-merge TS-style JSON fields onto an existing node.
    PatchNodeData {
        node_id: NodeId,
        patch_json: String,
        /// `None` updates on the active page; `Some` targets a page id
        /// or legacy page index.
        page_id: Option<String>,
    },
    /// Remove a node + all its descendants.
    DeleteNode {
        node_id: NodeId,
        /// `None` deletes on the active page; `Some` targets a page id
        /// or legacy page index.
        page_id: Option<String>,
    },
    /// Reparent a node. A `NONE` target reparents to the active page
    /// root; a real id must resolve + must not create a cycle.
    MoveNode {
        node_id: NodeId,
        target_parent: NodeId,
        /// `None` moves on the active page; `Some` targets a page id
        /// or legacy page index.
        page_id: Option<String>,
        /// Optional insertion index within the target parent/root.
        index: Option<usize>,
    },
    /// Deep-clone a node + subtree under a new parent (`NONE` = active
    /// page root). Fresh ids minted past the id space.
    CopyNode {
        node_id: NodeId,
        target_parent: NodeId,
        /// Optional TS-style shallow overrides JSON applied to the cloned
        /// root. The clone's freshly-generated id is preserved.
        overrides_json: Option<String>,
        /// `None` copies on the active page; `Some` targets a page id
        /// or legacy page index.
        page_id: Option<String>,
    },
    /// Replace an existing node with a freshly-built leaf at the same
    /// slot. `drop_children` is the destructive-swap guard: replacing a
    /// node WITH children requires `drop_children == true`, else the
    /// applier refuses so a container can't silently lose its subtree.
    ReplaceNode {
        node_id: NodeId,
        kind: String,
        name: String,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        fill_hex: Option<String>,
        drop_children: bool,
        /// `None` replaces on the active page; `Some` targets a page id
        /// or legacy page index.
        page_id: Option<String>,
    },
    /// Replace an existing node with a pre-built canonical subtree at
    /// the same parent slot. Incoming ids are remapped before insertion.
    ReplaceSubtree {
        node_id: NodeId,
        node: Box<PenNode>,
        drop_children: bool,
        /// `None` replaces on the active page; `Some` targets a page id
        /// or legacy page index.
        page_id: Option<String>,
    },
    /// Insert N leaf nodes on the active page atomically — one bad
    /// descriptor rejects the whole batch.
    BatchInsert {
        items: Vec<BatchInsertItem>,
        /// `None` inserts on the active page; `Some` targets a page id
        /// or legacy page index.
        page_id: Option<String>,
    },

    /// Insert one or more pre-built canonical `PenNode` subtrees under a
    /// parent. Unlike `InsertNode` / `BatchInsert` (flat leaves), this
    /// carries fully-nested nodes — frames with children, layout, text.
    /// Every incoming node id is remapped to a fresh editor id, so the
    /// caller's ids are structural placeholders only.
    InsertSubtree {
        nodes: Vec<PenNode>,
        /// `NodeId::NONE` → active page root.
        parent_id: NodeId,
        /// `None` inserts on the active page; `Some` targets a page id
        /// or legacy page index.
        page_id: Option<String>,
    },
    /// Insert one or more already-id-authored subtrees. Used by the
    /// layered design skeleton workflow, which must return root/section
    /// ids that remain valid for later `design_content` calls.
    InsertAuthoredSubtree {
        nodes: Vec<PenNode>,
        /// `NodeId::NONE` → active page root.
        parent_id: NodeId,
        /// `None` inserts on the active page; `Some` targets a page id
        /// or legacy page index.
        page_id: Option<String>,
    },
    /// Run deterministic post-generation cleanup for a layered design
    /// root. Unlike most write commands, a valid root with no needed
    /// edits is still accepted so `design_refine` can be idempotent.
    RefineDesign {
        root_id: NodeId,
        canvas_width: Option<i32>,
        /// `None` refines on the active page; `Some` targets a page id
        /// or legacy page index.
        page_id: Option<String>,
    },
    /// Set a non-color scalar variable's value.
    SetVariableScalar {
        name: String,
        scalar: VariableScalarPayload,
    },
    /// Create a new theme-agnostic scalar variable. `kind` is one of
    /// `"color"` / `"number"` / `"boolean"` / `"string"`.
    CreateVariable {
        name: String,
        kind: String,
        default_value: String,
    },
    /// Delete a variable by name.
    DeleteVariable { name: String },
    /// Rename a variable.
    RenameVariable { old_name: String, new_name: String },
    /// Merge or replace the full document variables map.
    SetVariables {
        variables: BTreeMap<String, VariableDefinition>,
        replace: bool,
    },
    /// Merge or replace the full document theme axes map.
    SetThemes {
        themes: BTreeMap<String, Vec<String>>,
        replace: bool,
    },
    /// Merge a theme preset payload into document variables + theme axes.
    MergeThemePreset {
        variables: BTreeMap<String, VariableDefinition>,
        themes: BTreeMap<String, Vec<String>>,
    },
    /// Replace the document's persisted design.md spec.
    SetDesignMd { spec: Box<DesignMdSpec> },
    /// Instantiate a registered component onto the active page.
    InstantiateComponent { component_id: NodeId },
    /// Promote an active-page Frame / Group / Rectangle to a component.
    CreateComponent { node_id: NodeId, name: String },
    /// Remove a component registration.
    DeleteComponent { component_id: NodeId },
    /// Rename a component registration.
    RenameComponent { component_id: NodeId, name: String },
    /// Instantiate a UIKit component onto the active page. The kit /
    /// component lookup happens against [`crate::EditorState::ui_kits`]
    /// at apply time; `(doc_x, doc_y)` default to `(0, 0)` when the
    /// caller does not specify a drop point.
    InstantiateKitComponent {
        kit_id: String,
        component_id: String,
        doc_x: Option<f64>,
        doc_y: Option<f64>,
    },
    /// Switch the active page.
    SetActivePage { index: u32 },
    /// Append a fresh page + switch to it. When `children` is omitted,
    /// the page gets the default blank frame; when supplied, external
    /// child ids are remapped before insertion.
    AddPage {
        name: Option<String>,
        children: Option<Vec<PenNode>>,
    },
    /// Set a page's display name.
    RenamePage { index: u32, name: String },
    /// Remove a page by index.
    DeletePage { index: u32 },
    /// Duplicate the page at `index` (clone + insert after).
    DuplicatePage { index: u32, name: Option<String> },
    /// Move the page at `from` to `to`.
    ReorderPage { from: u32, to: u32 },
    /// Clear the multi-selection.
    ClearSelection,
    /// Select a single node by id.
    SetSelection { node_id: NodeId },
    /// Replace the multi-selection with the supplied ids; unknown ids
    /// are dropped silently.
    SetSelectionSet { node_ids: Vec<NodeId> },
    /// Toggle one node's membership in the multi-selection.
    ToggleNodeSelection { node_id: NodeId },
    /// Set canvas pan + zoom. Any axis `None` is left untouched.
    SetViewport {
        pan_x: Option<i32>,
        pan_y: Option<i32>,
        zoom_percent: Option<i32>,
    },
    /// Flip a single boolean flag on a node.
    SetNodeFlag {
        node_id: NodeId,
        flag: NodeFlag,
        value: bool,
    },
    /// Set the horizontal / vertical mirror flags on a node. Either
    /// axis `None` is left untouched. Writes `PenNodeBase::flip_x` /
    /// `flip_y`.
    SetNodeFlip {
        node_id: NodeId,
        flip_x: Option<bool>,
        flip_y: Option<bool>,
    },
    /// Set the arc geometry on an Ellipse node — `start_angle` /
    /// `sweep_angle` (degrees) carve a pie / arc, `inner_radius`
    /// (0.0..=1.0 fraction of the radius) carves a donut hole. Any
    /// field `None` is left untouched; rejects non-Ellipse kinds.
    SetEllipseArc {
        node_id: NodeId,
        start_angle: Option<f64>,
        sweep_angle: Option<f64>,
        inner_radius: Option<f64>,
    },
    /// Append a visual effect to a node. `kind` is one of `"shadow"`
    /// / `"blur"` / `"background_blur"`; the effect lands with sensible
    /// default parameters.
    AddNodeEffect { node_id: NodeId, kind: String },
    /// Remove the effect at `index` from a node's effect list.
    RemoveNodeEffect { node_id: NodeId, index: u32 },
    /// Set one scalar parameter of the effect at `index` on a node.
    SetEffectParam {
        node_id: NodeId,
        index: u32,
        field: EffectField,
        value: f32,
    },
    /// Replace the colour of the Shadow effect at `index`. `hex` is
    /// canonical `#RRGGBB` or `#RRGGBBAA`. No-op for Blur kinds.
    SetEffectColor {
        node_id: NodeId,
        index: u32,
        hex: String,
    },
    /// Set the active canvas tool.
    SetActiveTool { tool: String },
    /// Undo the last change.
    Undo,
    /// Redo the last undone change.
    Redo,
    /// Duplicate the selection + select the clone.
    DuplicateSelected { offset_px: i32 },
    /// Delete the selection.
    DeleteSelected,
    /// Translate the selection by `(dx, dy)` doc-px.
    NudgeSelected { dx: i32, dy: i32 },
    /// `Cmd+G` — wrap the multi-selected siblings in a new Group.
    GroupSelected,
    /// `Cmd+Shift+G` — replace a selected Group with its children.
    UngroupSelected,
    /// Move the selection up / down in z-order.
    ReorderSelected { direction: ReorderDirection },
    /// Set rotation (degrees) on a node.
    SetNodeRotation { node_id: NodeId, degrees: f32 },
    /// Set the text content on a Text-kind node.
    SetNodeText { node_id: NodeId, text: String },
    /// Set corner-radius (doc-px) on a node.
    SetNodeCornerRadius { node_id: NodeId, radius: f32 },
    /// Set the font size (doc-px) on a Text-kind node.
    SetNodeFontSize { node_id: NodeId, font_size: f32 },
    /// Set the font weight (1..=1000) on a Text-kind node.
    SetNodeFontWeight { node_id: NodeId, font_weight: u16 },
    /// Set the stroke color on a node by id.
    SetNodeStrokeHex { node_id: NodeId, hex: String },
    /// Set the stroke width (doc-px) on a node by id.
    SetNodeStrokeWidth { node_id: NodeId, width: f32 },
    /// Apply alignment / distribution to the selection.
    AlignSelected { action: String },
    /// Set the fill color on a node by id.
    SetNodeFillHex { node_id: NodeId, hex: String },
    /// Rename a node by id.
    SetNodeName { node_id: NodeId, name: String },
    /// `Cmd+C` — deep-clone the selection into the clipboard.
    CopySelected,
    /// `Cmd+X` — copy the selection, then delete it.
    CutSelected,
    /// `Cmd+V` — paste the clipboard as top-level siblings.
    PasteClipboard { offset_px: i32 },
    /// Parse an SVG document + insert the resulting nodes on the
    /// active page, offset by `(x, y)` doc-px.
    ImportSvg {
        svg: String,
        x: i32,
        y: i32,
        /// `NodeId::NONE` inserts at the active page root.
        target_parent: NodeId,
        /// `None` inserts on the active page; `Some` targets a page id
        /// or legacy page index.
        page_id: Option<String>,
    },
    /// Set a layout / text property on a node by name + value.
    ///
    /// Covers the remaining 17-property whitelist entries that do not
    /// have their own typed command variants: `gap`, `padding`,
    /// `letterSpacing`, `lineHeight`, `opacity`, `textAlign`,
    /// `textGrowth`, `alignItems`, `justifyContent`, and sizing
    /// keywords for `width` / `height` (`"fit_content"` /
    /// `"fill_container"`).
    ///
    /// Unknown / unsupported (property, value) combinations return
    /// `false` at apply time — the same convention as all other
    /// commands.
    SetNodeLayoutProp {
        node_id: NodeId,
        property: String,
        value: LayoutPropValue,
    },
    /// Replace all matching style property values under one or more
    /// parent ids. Mirrors the TS MCP `replace_all_matching_properties`
    /// operation.
    ReplaceAllMatchingProperties {
        page_id: Option<String>,
        parent_ids: Vec<NodeId>,
        replacements: Vec<StylePropertyReplacement>,
    },
    /// Apply a sequence of sub-commands atomically: every sub-command
    /// must succeed, or the editor state is rolled back to the
    /// pre-batch snapshot and the whole batch reports `false`. Emitted
    /// by the MCP `batch_design` multi-op program executor so a mixed
    /// DSL program rides the host applier as ONE command.
    ///
    /// RESTRICTED to snapshot-covered sub-commands — those whose
    /// mutations the rollback snapshot (document, selection, active
    /// page index, components) can genuinely restore. Commands whose
    /// primary state lives outside it — `SetActiveTool` (tool),
    /// `SetViewport` (viewport), `CopySelected` / `CutSelected` /
    /// `PasteClipboard` (clipboard), `Undo` / `Redo` (the history
    /// stacks themselves), `SetActiveAxisValue` /
    /// `CycleActiveAxisValue` (transient active-theme pin) — and
    /// nested `Batch`es are rejected up front at apply time: the whole
    /// batch fails with zero state change before any sub-command runs.
    /// See `command_batch.rs` for the classification.
    Batch { commands: Vec<EditorCommand> },
}
