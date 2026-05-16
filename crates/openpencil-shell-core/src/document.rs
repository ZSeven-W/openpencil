//! `Document` model for the Rust shell — single source of truth
//! for pages / nodes / selection / chrome UI state. Single-page
//! sample fixture for Step 2; multi-page + variables + components
//! arrive in later steps. wasm32-clean (no platform imports).

/// Stable node identity — a string id matching the canonical `.op`
/// schema's `PenNodeBase.id`. The empty string is the `NONE`
/// sentinel meaning "no node"; `NodeId::new` rejects it.
///
/// Editor-minted ids follow the `n{N}` convention (`n1`, `n2`, …);
/// canonical-schema loads keep whatever arbitrary string the file
/// authored. Ordering is intentionally NOT derived — a node tree
/// has no meaningful id order, only insertion order in `children`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct NodeId(String);

impl NodeId {
    /// The "no node" sentinel — an empty-string id. A `const` over
    /// an owned `String` is feasible because `String::new()` is a
    /// const fn, so call sites keep using `NodeId::NONE` unchanged.
    pub const NONE: NodeId = NodeId(String::new());

    /// True when this id is not the `NONE` sentinel.
    #[inline]
    pub fn is_real(&self) -> bool {
        !self.0.is_empty()
    }

    /// Construct a real (non-sentinel) id. Panics on an empty
    /// string (reserved for `NodeId::NONE`).
    #[inline]
    pub fn new(id: impl Into<String>) -> Self {
        let id = id.into();
        if id.is_empty() {
            panic!("NodeId::new(\"\") — the empty id is reserved for NodeId::NONE");
        }
        Self(id)
    }

    /// Non-panicking construction. Returns `None` for an empty
    /// string (the NONE sentinel). Used by code that accepts an
    /// arbitrary id from the wire (MCP write tools) and needs to
    /// reject the sentinel without panicking.
    #[inline]
    pub fn new_opt(id: impl Into<String>) -> Option<Self> {
        let id = id.into();
        if id.is_empty() {
            None
        } else {
            Some(Self(id))
        }
    }

    /// Bridge an MCP-wire numeric id to a model `NodeId`. The MCP
    /// protocol still carries node ids as `u64`; `0` is the wire's
    /// "no node" value and maps to `None`, any other `N` maps to
    /// the editor-minted `n{N}` string. Confines the numeric ⇄
    /// string conversion to the one MCP-apply seam.
    #[inline]
    pub fn from_mcp_u64(id: u64) -> Option<Self> {
        if id == 0 {
            None
        } else {
            Some(Self(format!("n{id}")))
        }
    }

    /// Inner string id.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Inner string id (alias of [`NodeId::as_str`], kept for the
    /// pre-string-rewrite call sites that named it `raw`).
    #[inline]
    pub fn raw(&self) -> &str {
        &self.0
    }

    /// Convert to a `WidgetId`. The string id is hashed into the
    /// `u64` `WidgetId` space — accesskit / DOM-mirror routing only
    /// needs a stable per-node integer, not the id text.
    #[inline]
    pub fn to_widget_id(&self) -> crate::widgets::WidgetId {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.0.hash(&mut h);
        let v = h.finish();
        crate::widgets::WidgetId(if v == 0 { 1 } else { v })
    }
}

impl From<NodeId> for String {
    fn from(id: NodeId) -> String {
        id.0
    }
}

impl From<&NodeId> for String {
    fn from(id: &NodeId) -> String {
        id.0.clone()
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Node kinds covered by the editor. `Other` round-trips unknown
/// kinds so the host never errors on an unfamiliar serialized node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Frame,
    Group,
    Rect,
    Ellipse,
    Polygon,
    Line,
    /// Pen-tool polyline. Geometry in `Node.points`.
    Path,
    Text,
    Other(String),
}

impl NodeKind {
    /// Human-facing label for LayerPanel + PropertyPanel.
    pub fn label(&self) -> &str {
        match self {
            NodeKind::Frame => "Frame",
            NodeKind::Group => "Group",
            NodeKind::Rect => "Rect",
            NodeKind::Ellipse => "Ellipse",
            NodeKind::Polygon => "Polygon",
            NodeKind::Line => "Line",
            NodeKind::Path => "Path",
            NodeKind::Text => "Text",
            NodeKind::Other(s) => s.as_str(),
        }
    }
}

/// Solid outline descriptor for [`Node::stroke`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    pub color: crate::Color,
    pub width: f32,
}

/// Drop-shadow effect — offset + blur + colour, doc-px units.
/// Painted behind the node's fill. Mirrors the TS `PenEffect`
/// shadow variant (`offsetX` / `offsetY` / `blur` / `color`);
/// `spread` is omitted until the renderer honors it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropShadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub color: crate::Color,
}

/// A visual effect on a node. v1 ships drop shadow (the common
/// case + what the property panel's 效果 section needs); layer
/// blur lands once its render path does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Effect {
    DropShadow(DropShadow),
}

/// Document tree node. `bounds = Rect::ZERO` for containers that
/// derive size from children. `text` is populated for Text kind.
#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    pub name: String,
    /// Doc-space rect; ZERO = container (use children's bounds).
    pub bounds: crate::Rect,
    pub fill: Option<crate::Color>,
    pub stroke: Option<Stroke>,
    pub text: Option<String>,
    /// Rotation radians, clockwise about bounds center.
    pub rotation: f32,
    pub hidden: bool,
    pub locked: bool,
    pub collapsed: bool,
    pub fill_type: FillType,
    pub corner_radius: f32,
    pub points: Vec<crate::Point2D>,
    /// Text font size in doc-px (0 = default 13). Text-only.
    pub font_size: f32,
    pub font_weight: u16,
    pub text_wrap: bool,
    /// Visual effects (drop shadows) painted with the node. Empty
    /// for most nodes; the property panel's 效果 section edits it.
    pub effects: Vec<Effect>,
    pub children: Vec<Node>,
}

impl Node {
    pub fn leaf(id: impl Into<String>, kind: NodeKind, name: impl Into<String>) -> Self {
        Self {
            id: NodeId::new(id),
            kind,
            name: name.into(),
            bounds: crate::Rect::ZERO,
            fill: None,
            stroke: None,
            text: None,
            rotation: 0.0,
            hidden: false,
            locked: false,
            collapsed: false,
            fill_type: FillType::Solid,
            corner_radius: 0.0,
            points: Vec::new(),
            font_size: 0.0,
            font_weight: 0,
            text_wrap: false,
            effects: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn with_children(
        id: impl Into<String>,
        kind: NodeKind,
        name: impl Into<String>,
        children: Vec<Node>,
    ) -> Self {
        Self {
            id: NodeId::new(id),
            kind,
            name: name.into(),
            bounds: crate::Rect::ZERO,
            fill: None,
            stroke: None,
            text: None,
            rotation: 0.0,
            hidden: false,
            locked: false,
            collapsed: false,
            fill_type: FillType::Solid,
            corner_radius: 0.0,
            points: Vec::new(),
            font_size: 0.0,
            font_weight: 0,
            text_wrap: false,
            effects: Vec::new(),
            children,
        }
    }

    /// Builder: set bounds.
    pub fn with_bounds(mut self, bounds: crate::Rect) -> Self {
        self.bounds = bounds;
        self
    }

    /// Builder: set solid fill.
    pub fn with_fill(mut self, fill: crate::Color) -> Self {
        self.fill = Some(fill);
        self
    }

    /// Builder: set stroke (color + width).
    pub fn with_stroke(mut self, color: crate::Color, width: f32) -> Self {
        self.stroke = Some(Stroke { color, width });
        self
    }

    /// Builder: set the Text node's rendered string.
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Find a descendant by id, including self.
    pub fn find(&self, id: &NodeId) -> Option<&Node> {
        if &self.id == id {
            return Some(self);
        }
        for child in &self.children {
            if let Some(hit) = child.find(id) {
                return Some(hit);
            }
        }
        None
    }

    /// Effective bounds: own `bounds` if non-zero, else children's union.
    pub fn aggregate_bounds(&self) -> crate::Rect {
        if self.bounds.size.x > 0.0 || self.bounds.size.y > 0.0 {
            return self.bounds;
        }
        let mut iter = self
            .children
            .iter()
            .map(Node::aggregate_bounds)
            .filter(|r| r.size.x > 0.0 || r.size.y > 0.0);
        let Some(first) = iter.next() else {
            return crate::Rect::ZERO;
        };
        let (mut min_x, mut min_y) = (first.origin.x, first.origin.y);
        let (mut max_x, mut max_y) = (first.origin.x + first.size.x, first.origin.y + first.size.y);
        for r in iter {
            min_x = min_x.min(r.origin.x);
            min_y = min_y.min(r.origin.y);
            max_x = max_x.max(r.origin.x + r.size.x);
            max_y = max_y.max(r.origin.y + r.size.y);
        }
        crate::Rect::xywh(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

/// A document page — id + name + top-level node children.
#[derive(Debug, Clone)]
pub struct Page {
    pub id: NodeId,
    pub name: String,
    pub children: Vec<Node>,
}

impl Page {
    pub fn new(id: impl Into<String>, name: impl Into<String>, children: Vec<Node>) -> Self {
        Self {
            id: NodeId::new(id),
            name: name.into(),
            children,
        }
    }

    /// Find a descendant by id (does not match page id itself).
    pub fn find(&self, id: &NodeId) -> Option<&Node> {
        for child in &self.children {
            if let Some(hit) = child.find(id) {
                return Some(hit);
            }
        }
        None
    }
}

/// A document — the editor's subject. Holds pages, the active
/// page index, and a selection sentinel; the rest of the document
/// model (variables, components, styles, artboards) is Step 3+.
///
/// Selection is **page-scoped** by design (codex Step 2 R1
/// CONCERN-1): the editor UI only ever shows one page at a time
/// (`active_page_index`), so a "selection" outside that page is
/// effectively no selection. This keeps LayerPanel and
/// PropertyPanel pinned to the same content.
#[derive(Debug, Clone)]
pub struct Document {
    pub pages: Vec<Page>,
    /// Index into `pages` for the page currently shown in the editor.
    pub active_page_index: usize,
    /// Anchor selection — last entry of `selected_set`, or `NONE`.
    pub selected: NodeId,
    /// Full selection set (anchor invariant: `selected` = last).
    pub selected_set: Vec<NodeId>,
    /// Cross-action clipboard buffer. Cut/copy fill it; paste drains.
    pub clipboard: Vec<Node>,
    pub tool: Tool,
    pub viewport: Viewport,
    pub chat: ChatState,
    pub ui: UiState,
    /// Undo / redo stacks. Push BEFORE a transactional mutation.
    pub history: History,
    pub var_table: VariableTable,
    pub components: ComponentLibrary,
}

/// Document undo / redo stacks. Snapshot = deep copy of the
/// editable subset (pages + selection + viewport).
#[derive(Debug, Clone, Default)]
pub struct History {
    pub past: std::collections::VecDeque<DocumentSnapshot>,
    pub future: std::collections::VecDeque<DocumentSnapshot>,
}

/// Snapshot of the document state covered by undo/redo.
#[derive(Debug, Clone)]
pub struct DocumentSnapshot {
    pub pages: Vec<Page>,
    pub active_page_index: usize,
    pub selected: NodeId,
    pub selected_set: Vec<NodeId>,
    /// Variable table at snapshot time. Covers Variable definitions
    /// + themed entries + active_theme + fill_refs / stroke_refs.
    /// Without this, ColorPicker variable-mode edits would push an
    /// undo entry that couldn't restore the variable (codex stop-
    /// gate: "variable edits push undo entries that cannot restore
    /// the variable").
    pub var_table: VariableTable,
}

/// Chrome-level UI state — toggles for collapsible chrome surfaces.
#[derive(Debug, Clone)]
pub struct UiState {
    pub sidebar_open: bool,
    pub layer_panel_width: f32,
    pub property_panel_width: f32,
    /// Property-panel input with keyboard focus; `None` = muted.
    pub property_focus: Option<PropertyFocus>,
    /// Draft for the focused property input; committed on Enter,
    /// discarded on Escape.
    pub property_input_draft: String,
    /// Draft for the focused settings-modal input (e.g. MCP port).
    /// Lives on UiState because `AgentSettings` is `Copy` and can't
    /// hold a `String` — the focus discriminator stays on
    /// `AgentSettings::focus` so its lifecycle is tied to the modal.
    pub settings_input_draft: String,
    /// Caret-blink anchor for the focused property-panel input —
    /// reset on focus + every keystroke (mirrors `chat.caret_anchor_ms`).
    pub property_caret_anchor_ms: u64,
    /// Select-all-on-focus flag — next keystroke clears the seeded draft.
    pub property_draft_select_all: bool,
    /// Active theme — TopBar Sun icon flips it.
    pub theme_mode: ThemeMode,
    /// UI locale — TopBar Globe cycles.
    pub locale: Locale,
    /// TopBar Globe dropdown open.
    pub locale_picker_open: bool,
    /// AI chat model-picker dropdown open (the "Default ▴" chip in
    /// the chat panel's bottom toolbar). Lists CLI agents that are
    /// connected per `agent_settings.connected`.
    pub chat_model_picker_open: bool,
    /// Index into `AgentProvider::ALL` of the agent driving the AI
    /// chat. Default 0 = Claude Code. The picker only *selects*
    /// among connected agents, but the stored index is absolute so
    /// it survives connect/disconnect churn.
    pub chat_selected_agent: usize,
    /// File-menu dropdown anchored under folder+chevron.
    pub file_menu_open: bool,
    /// Row currently hovered while the file menu is open — drives
    /// the per-row tint so the user can see which action will fire.
    pub file_menu_hover: Option<crate::widgets::file_menu::FileMenuChoice>,
    pub locale_picker_hover: Option<Locale>,
    pub shape_picker_hover: Option<crate::widgets::shape_picker::ShapeChoice>,
    pub align_toolbar_hover: Option<AlignAction>,
    /// Raster export scale (1.0 / 2.0 / 3.0). Default 2.0.
    pub export_scale: f32,
    pub export_dialog_open: bool,
    pub export_format: crate::widgets::export_dialog::ExportFormat,
    /// Pending file-menu action.
    pub pending_file_action: Option<FileAction>,
    /// Recent files (head = newest, cap 10).
    pub recent_files: Vec<RecentFile>,
    /// TopBar display name; None = "Untitled".
    pub file_name_display: Option<String>,
    /// Modal "从 Figma 导入" — opens on Figma logo click in TopBar.
    pub figma_import_open: bool,
    /// Whether the Toolbar shape-tool dropdown is open. The shape
    /// slot in the toolbar shows the icon for `shape_tool`; click
    /// it to toggle this picker. Picker rows: Rectangle / Ellipse /
    /// Polygon / Line / Icon / Import Image or SVG / Pen.
    pub shape_picker_open: bool,
    /// Active-theme axis whose value picker is open. `None` =
    /// closed. Click on the chip toggles open; click on a value
    /// row sets `var_table.active_theme[axis] = value` + closes;
    /// click outside the panel closes silently.
    pub axis_dropdown_open: Option<String>,
    /// Editor focus for a non-color variable row (Number / String).
    /// Reuses `property_input_draft` as the buffer so a single
    /// caret-blink anchor + input handler tier serves both.
    pub variable_row_focus: Option<VariableRowFocus>,
    /// Last-selected shape tool — drives the toolbar shape slot's
    /// icon. Defaults to Rect. Always one of Rect / Ellipse /
    /// Polygon / Line / Pen (Icon + Import are one-shot actions
    /// that don't promote to the active tool).
    pub shape_tool: Tool,
    /// Active flex-layout mode for the property panel's "弹性布局"
    /// row. Lives on UiState until the Node schema grows a real
    /// LayoutSettings field; visual-only for now.
    pub flex_layout: FlexLayout,
    /// "尺寸" checkboxes — TS app stores these per-node; we
    /// surface them as document-level toggles until Node grows a
    /// SizeOptions field. Visual + interactive only.
    pub size_fill_width: bool,
    pub size_fill_height: bool,
    pub size_hug_width: bool,
    pub size_hug_height: bool,
    pub size_clip_content: bool,
    /// Whether the fill-type dropdown is open. The actual
    /// `FillType` value moved to `Node.fill_type` on 2026-05-11
    /// so each node remembers its own picker choice (codex
    /// full-audit CONCERN fix). The picker-open flag stays in
    /// UI state since it's transient overlay state, not a
    /// per-node property.
    pub fill_type_picker_open: bool,
    /// Currently-hovered LayerPanel row, or `None` when the
    /// cursor isn't over a row. Drives the hover-reveal of the
    /// eye + lock icons (TS parity: icons only appear on
    /// hovered / active rows). Host updates this on every
    /// cursor-move event.
    pub hovered_layer_id: Option<NodeId>,
    /// Page-row hover state for the Pages section. `None` when the
    /// cursor isn't over a page row. Drives the hover-reveal of
    /// the trailing × delete button on each page row.
    pub hovered_page_index: Option<usize>,
    /// Open layer-row context menu (right-click target + anchor
    /// position in panel-local px). `None` when no menu is open.
    pub layer_context_menu: Option<LayerContextMenuState>,
    /// Inline rename in progress on a layer or page row.
    pub layer_rename: Option<LayerRenameState>,
    pub rename_caret_anchor_ms: u64,
    /// Last LayerPanel click target + ms; 400 ms re-press → rename.
    pub last_layer_click: Option<(LayerContextTarget, u64)>,
    /// Canvas Text node in inline text-edit mode.
    pub text_editing: Option<NodeId>,
    pub text_edit_caret_anchor_ms: u64,
    /// Last canvas left-click target + timestamp; 400 ms same-node
    /// re-press on a Text node promotes to text edit.
    pub last_canvas_click: Option<(NodeId, u64)>,
    /// Deferred history snapshot for canvas text-edit. Set on
    /// `start_text_edit`, flushed onto the past stack by the first
    /// keystroke that actually mutates the node, dropped on commit
    /// — so undo only sees a snapshot for sessions that produced a
    /// change.
    pub pending_text_edit_history: Option<DocumentSnapshot>,
    /// Last text-edit keystroke ms — pauses > 500 ms split bursts.
    pub text_edit_last_ms: u64,
    /// Active PropertyPanel tab — toggled by `Cmd+Shift+C`.
    pub property_tab: PropertyTab,
    /// Floating Cmd+, settings modal state.
    pub agent_settings_open: bool,
    pub agent_settings: AgentSettings,
    pub agent_settings_drag: Option<AgentSettingsDrag>,
    /// In-progress Pen-tool path. `Some(id)` while the user is
    /// click-adding anchors; cleared on Enter / Escape / tool change.
    pub pen_in_progress: Option<NodeId>,
    /// Cursor position in document coords for the Pen-tool rubber
    /// band — drives the preview segment from the last anchor.
    pub pen_cursor_doc: Option<crate::Point2D>,
    /// Pre-pen snapshot — flushed to history when the path is
    /// committed with at least one segment; dropped on cancel.
    pub pending_pen_history: Option<DocumentSnapshot>,
    /// Floating color picker state. `Some(target)` while the picker
    /// is open; the picker mutates the corresponding Fill or Stroke
    /// colour as the user drags inside.
    pub color_picker: Option<ColorPickerState>,
}

/// Which colour the picker is targeting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorTarget {
    Fill,
    Stroke,
}

/// Which PropertyPanel tab is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyTab {
    Design,
    Code,
}

pub use crate::agent_settings_state::{
    AgentProvider, AgentSettings, AgentSettingsDrag, AgentSettingsTab, McpCli, McpServer,
    SettingsFocus,
};

/// File-menu "Recent files" entry — runner persists via `settings_io`.
#[derive(Debug, Clone)]
pub struct RecentFile {
    pub path: String,
    /// Unix seconds when last touched.
    pub modified_at: u64,
}

/// File-menu choices the desktop runner has to handle (rfd dialogs
/// + serde live there, not in shell-core).
/// File-menu choices. ExportImage opens the picker; ExportImageConfirm commits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileAction {
    New, Open, Save, SaveAs, ExportImage, ExportImageConfirm,
    ImportFigma, OpenRecent(usize), ClearRecent,
}

/// Path boolean ops — TS parity with Paper.js (Ctrl+Alt+U/S/I/X).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanOp { Union, Subtract, Intersect, Exclude }

/// Floating colour-picker state. HSV stays anchored across the
/// committed-RGB rounding cycle.
#[derive(Debug, Clone)]
pub struct ColorPickerState {
    pub target: ColorTarget,
    pub hue: f32,
    pub sat: f32,
    pub val: f32,
    pub pre_snap: Option<DocumentSnapshot>,
    pub drag: Option<ColorPickerDrag>,
    /// Viewport-y of the click that opened the picker.
    pub anchor_y: f32,
    /// When `Some`, the picker edits the named Color variable
    /// instead of the selected node's `target` channel. The
    /// commit path routes through `VariableTable::set_color_hex`.
    /// `target` is still set (to a sensible default like Fill)
    /// because callers + paint code currently key off it; keeping
    /// the new field parallel avoids touching every ColorTarget
    /// pattern match.
    pub variable: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorPickerDrag {
    SvBox,
    HueSlider,
}

/// Inline rename in progress on a layer or page row.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerRenameState {
    pub target: LayerContextTarget,
    pub draft: String,
}

/// What the LayerPanel right-click context menu is acting on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerContextTarget {
    Layer(NodeId),
    Page(usize),
}

/// Right-click context-menu state.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerContextMenuState {
    pub target: LayerContextTarget,
    pub anchor_x: f32,
    pub anchor_y: f32,
    /// Hovered row index for the menu paint. None when no row is hovered.
    pub hovered_row: Option<u8>,
}

/// Inline-rename state for a page row (double-click → rename).
#[derive(Debug, Clone, PartialEq)]
pub struct PageRenameState {
    pub page_index: usize,
    pub draft: String,
}

/// Variants the Fill section's type-selector pill exposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillType {
    Solid,
    LinearGradient,
    RadialGradient,
    Image,
}

/// Three flex-layout modes the property panel exposes — Free
/// (no layout), Vertical, Horizontal. Mirrors the TS app's
/// `layout.flexLayout` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexLayout {
    Free,
    Vertical,
    Horizontal,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            sidebar_open: true,
            layer_panel_width: 240.0,
            property_panel_width: 280.0,
            property_focus: None,
            property_input_draft: String::new(),
            settings_input_draft: String::new(),
            property_caret_anchor_ms: 0,
            property_draft_select_all: false,
            theme_mode: ThemeMode::Dark,
            locale: Locale::ZhCn,
            locale_picker_open: false,
            chat_model_picker_open: false,
            chat_selected_agent: 0,
            file_menu_open: false,
            file_menu_hover: None,
            locale_picker_hover: None,
            shape_picker_hover: None,
            align_toolbar_hover: None,
            export_scale: 2.0,
            export_dialog_open: false,
            export_format: crate::widgets::export_dialog::ExportFormat::Png,
            pending_file_action: None,
            recent_files: Vec::new(),
            file_name_display: None,
            figma_import_open: false,
            shape_picker_open: false,
            axis_dropdown_open: None,
            variable_row_focus: None,
            shape_tool: Tool::Rect,
            flex_layout: FlexLayout::Free,
            size_fill_width: false,
            size_fill_height: false,
            size_hug_width: false,
            size_hug_height: false,
            size_clip_content: false,
            fill_type_picker_open: false,
            hovered_layer_id: None,
            hovered_page_index: None,
            layer_context_menu: None,
            layer_rename: None,
            rename_caret_anchor_ms: 0,
            last_layer_click: None,
            text_editing: None,
            text_edit_caret_anchor_ms: 0,
            last_canvas_click: None,
            pending_text_edit_history: None,
            text_edit_last_ms: 0,
            property_tab: PropertyTab::Design,
            agent_settings_open: false,
            agent_settings: AgentSettings::default(),
            agent_settings_drag: None,
            pen_in_progress: None,
            pen_cursor_doc: None,
            pending_pen_history: None,
            color_picker: None,
        }
    }
}

/// Light/dark switch — light palette is stubbed in
/// [`crate::theme::Theme::light`] and ready to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

impl ThemeMode {
    pub fn flipped(self) -> Self {
        match self {
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
        }
    }
}

// Phase 3 strangler reorg: the `Locale` enum + its impls moved into the
// `op-i18n` crate (alongside the 15-locale string tables). Re-exported here
// so existing `document::Locale` / `crate::document::Locale` paths resolve.
pub use op_i18n::Locale;

/// Identifier for a property-panel input row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyFocus {
    PositionX,
    PositionY,
    Rotation,
    PositionR,
    SizeW,
    SizeH,
    Opacity,
    FillHex,
    StrokeHex,
    StrokeWidth,
}

/// Editor focus for a non-color variable row in the VariablesPanel.
/// `Number(idx)` allows digits + leading minus + one `.`; `String`
/// allows any non-control character (free-form text). Boolean
/// variables don't need a focus — their row click toggles the
/// value directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableRowFocus {
    Number(usize),
    String(usize),
}

mod chat;
pub use chat::{ChatAnchor, ChatMessage, ChatRole, ChatState};

/// Pan + zoom state. Pan = canvas-local px; zoom = multiplier.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub pan_x: f32,
    pub pan_y: f32,
    pub zoom: f32,
}

impl Viewport {
    /// Identity viewport — origin pan, 100% zoom.
    pub const IDENTITY: Viewport = Viewport {
        pan_x: 0.0,
        pan_y: 0.0,
        zoom: 1.0,
    };
    pub const MIN_ZOOM: f32 = 0.1;
    pub const MAX_ZOOM: f32 = 8.0;

    /// Wheel zoom anchored at `cursor` (canvas-local).
    pub fn zoom_at(&mut self, cursor: crate::Point2D, delta: f32) {
        let prev_zoom = self.zoom;
        let factor = (delta * 0.0015).exp();
        let new_zoom = (prev_zoom * factor).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        // Recover the document-space point at cursor BEFORE zoom,
        // then re-anchor pan so it stays at cursor AFTER zoom.
        let doc_x = (cursor.x - self.pan_x) / prev_zoom;
        let doc_y = (cursor.y - self.pan_y) / prev_zoom;
        self.zoom = new_zoom;
        self.pan_x = cursor.x - doc_x * new_zoom;
        self.pan_y = cursor.y - doc_y * new_zoom;
    }

    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.pan_x += dx;
        self.pan_y += dy;
    }

    /// Canvas-local → document space.
    pub fn to_document(&self, p: crate::Point2D) -> crate::Point2D {
        crate::Point2D::new(
            (p.x - self.pan_x) / self.zoom,
            (p.y - self.pan_y) / self.zoom,
        )
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Editor tool — what primary mouse drag does on the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Rect,
    Ellipse,
    Polygon,
    Line,
    Pen,
    Text,
    Frame,
    Hand,
}

impl Tool {
    /// All tools, in toolbar display order. Single source of truth
    /// for the toolbar build path.
    pub const ALL: [Tool; 9] = [
        Tool::Select, Tool::Rect, Tool::Ellipse, Tool::Polygon, Tool::Line,
        Tool::Pen, Tool::Text, Tool::Frame, Tool::Hand,
    ];

    /// Stable accesskit / DOM id token (lowercase ASCII).
    pub fn ident(self) -> &'static str {
        match self {
            Tool::Select => "select", Tool::Rect => "rect", Tool::Ellipse => "ellipse",
            Tool::Polygon => "polygon", Tool::Line => "line", Tool::Pen => "pen",
            Tool::Text => "text", Tool::Frame => "frame", Tool::Hand => "hand",
        }
    }

    /// True when this tool sits inside the Toolbar's shape-slot
    /// dropdown — paints whichever variant is currently active.
    pub fn is_shape(self) -> bool {
        matches!(self, Tool::Rect | Tool::Ellipse | Tool::Polygon | Tool::Line | Tool::Pen)
    }
}

pub mod align;
mod clipboard;
mod color_picker;
mod components;
mod grouping;
mod mcp_apply;
mod mcp_apply_node_attrs;
mod mcp_apply_selected;
mod mutators;
mod page_mutators;
mod pen;
mod variables;
#[cfg(test)] mod variables_tests;
mod walkers;
pub use align::AlignAction;
pub use components::{Component, ComponentLibrary};
pub use variables::{ThemeAxis, ThemedValue, Variable, VariableKind, VariableScalar, VariableTable, VariableValue};
pub use walkers::ReorderDirection;

#[cfg(test)] mod tests_geometry;
#[cfg(test)] mod tests_mutators;
