//! Editor-UI overlay + panel state for `EditorState`.
//!
//! This module ports the ~30 widget-layer UI fields that
//! `openpencil-shell-core::document::UiState` carries beyond the
//! editor-state subset already modelled by [`crate::ui_draft`]:
//!
//!   - menu / dropdown open flags + their hover targets
//!     (file menu, locale picker, shape picker, fill-type picker, …)
//!   - modal open flags (export dialog, figma import, agent settings)
//!   - the agent-settings modal struct (see [`crate::agent_settings`])
//!   - panel widths, theme mode, locale
//!   - layer / page hover + right-click context menu + page-rename
//!   - the property-panel tab + flex-layout + size toggles
//!   - export scale + format, recent files, pending file action
//!
//! ### Move STATE, not RENDER code
//!
//! Many of these types are *declared* under shell-core's `widgets/`
//! module — `ExportFormat` in `widgets/export_dialog.rs`,
//! `FileMenuChoice` in `widgets/file_menu.rs`, `ShapeChoice` in
//! `widgets/shape_picker.rs`. They are data/state enums, not rendering
//! code, so their type definitions belong in the state layer. The
//! widget *painting / hit-test* code stays in shell-core untouched.
//!
//! All types here are plain data (enums + structs of primitives /
//! strings / ids), so `op-editor-core` stays wasm32-clean.

use crate::node_id::NodeId;
use crate::tool::Tool;
use std::collections::HashSet;

// `Locale` is the i18n locale enum — dependency-free + wasm-clean, so
// it lives in `op-i18n` and re-exports cleanly into the state layer.
pub use op_i18n::Locale;

/// Light / dark UI theme switch.
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

/// Which PropertyPanel tab is active — toggled by `Cmd+Shift+C`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyTab {
    Design,
    Code,
}

/// Variants the Fill section's type-selector pill exposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillType {
    Solid,
    LinearGradient,
    RadialGradient,
    Image,
}

/// Three flex-layout modes the property panel exposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexLayout {
    Free,
    Vertical,
    Horizontal,
}

/// Path boolean ops — TS parity with Paper.js (Ctrl+Alt+U/S/I/X).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanOp {
    Union,
    Subtract,
    Intersect,
    Exclude,
}

/// Raster export format. State enum ported from shell-core's
/// `widgets/export_dialog::ExportFormat` (the widget render code stays
/// in shell-core).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Png,
    Jpeg,
    Webp,
    Svg,
    Pdf,
}

impl ExportFormat {
    pub const ALL: [ExportFormat; 5] = [
        ExportFormat::Png,
        ExportFormat::Jpeg,
        ExportFormat::Webp,
        ExportFormat::Svg,
        ExportFormat::Pdf,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ExportFormat::Png => "PNG",
            ExportFormat::Jpeg => "JPEG",
            ExportFormat::Webp => "WEBP",
            ExportFormat::Svg => "SVG",
            ExportFormat::Pdf => "PDF",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Png => "png",
            ExportFormat::Jpeg => "jpg",
            ExportFormat::Webp => "webp",
            ExportFormat::Svg => "svg",
            ExportFormat::Pdf => "pdf",
        }
    }

    /// Whether the format has a working export backend.
    pub fn is_implemented(self) -> bool {
        true
    }
}

/// File-menu choices. State enum ported from shell-core's
/// `widgets/file_menu::FileMenuChoice`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMenuChoice {
    NewFile,
    OpenFile,
    Save,
    SaveAs,
    ExportImage,
    OpenRecent(usize),
    ClearRecent,
}

/// File-menu actions the host runner has to handle (rfd dialogs +
/// serde live host-side, not here). `ExportImage` opens the picker;
/// `ExportImageConfirm` commits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileAction {
    New,
    Open,
    Save,
    SaveAs,
    ExportImage,
    ExportImageConfirm,
    ImportFigma,
    OpenRecent(usize),
    ClearRecent,
}

/// File-menu "Recent files" entry — host persists via settings IO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentFile {
    pub path: String,
    /// Unix seconds when last touched.
    pub modified_at: u64,
}

/// Toolbar shape-slot dropdown choice. State enum ported from
/// shell-core's `widgets/shape_picker::ShapeChoice`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeChoice {
    /// Pick a shape tool (Rect / Ellipse / Polygon / Line / Pen).
    Tool(Tool),
    /// Open the icon picker (host concern; this only reports intent).
    OpenIconPicker,
    /// Open a file dialog to import an image / SVG.
    ImportImageOrSvg,
}

// What the LayerPanel right-click context menu is acting on — the
// canonical definition is `ui_draft::LayerContextTarget` (it backs
// the inline-rename draft too). Re-exported so UI code that
// references a context target has one import path.
pub use crate::ui_draft::LayerContextTarget;

/// Right-click context-menu state.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerContextMenuState {
    pub target: LayerContextTarget,
    pub anchor_x: f32,
    pub anchor_y: f32,
    /// Hovered row index for the menu paint; `None` = no row hovered.
    pub hovered_row: Option<u8>,
}

/// Inline-rename state for a page row (double-click → rename).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRenameState {
    pub page_index: usize,
    pub draft: String,
}

/// Editor focus for a non-color variable row in the VariablesPanel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableRowFocus {
    Number(usize),
    String(usize),
}

/// Editor-UI overlay + panel state — the widget-layer toggles, hover
/// targets, menu / modal open flags and panel metrics that the ~30
/// editor widgets paint from. Faithful superset of the UI subset
/// of shell-core's `UiState`.
///
/// The *editor-state* subset of `UiState` (focused property field +
/// drafts, pen-tool path, color picker, text-edit drafts, variable
/// caches, active page index) lives on [`crate::ui_draft::UiDraftState`]
/// and is not duplicated here.
#[derive(Debug, Clone)]
pub struct EditorUiState {
    // --- Sidebar + panel metrics -----------------------------------
    pub sidebar_open: bool,
    pub layer_panel_width: f32,
    pub property_panel_width: f32,

    // --- Theme + locale --------------------------------------------
    /// Active UI theme — TopBar Sun icon flips it.
    pub theme_mode: ThemeMode,
    /// UI locale — TopBar Globe cycles.
    pub locale: Locale,
    /// TopBar Globe dropdown open.
    pub locale_picker_open: bool,
    /// Locale row currently hovered while the locale picker is open.
    pub locale_picker_hover: Option<Locale>,

    // --- File menu --------------------------------------------------
    /// File-menu dropdown open (anchored under folder + chevron).
    pub file_menu_open: bool,
    /// File-menu row currently hovered — drives the per-row tint.
    pub file_menu_hover: Option<FileMenuChoice>,
    /// Pending file-menu action for the host runner to handle.
    pub pending_file_action: Option<FileAction>,
    /// Recent files (head = newest, cap 10).
    pub recent_files: Vec<RecentFile>,
    /// TopBar display name; `None` = "Untitled".
    pub file_name_display: Option<String>,

    // --- Modals -----------------------------------------------------
    /// Raster export scale (1.0 / 2.0 / 3.0). Default 2.0.
    pub export_scale: f32,
    pub export_dialog_open: bool,
    pub export_format: ExportFormat,
    /// Modal "从 Figma 导入".
    pub figma_import_open: bool,
    /// Floating `Cmd+,` agent-settings modal open.
    pub agent_settings_open: bool,
    pub agent_settings: crate::agent_settings::AgentSettings,
    pub agent_settings_drag: Option<crate::agent_settings::AgentSettingsDrag>,
    /// Draft for the focused settings-modal input (e.g. MCP port).
    pub settings_input_draft: String,

    // --- Toolbar shape slot ----------------------------------------
    /// Whether the Toolbar shape-tool dropdown is open.
    pub shape_picker_open: bool,
    /// Shape-picker row currently hovered.
    pub shape_picker_hover: Option<ShapeChoice>,
    /// Last-selected shape tool — drives the toolbar shape slot's
    /// icon. Always one of Rect / Ellipse / Polygon / Line / Pen.
    pub shape_tool: Tool,

    // --- AI chat model picker --------------------------------------
    /// AI chat model-picker dropdown open.
    pub chat_model_picker_open: bool,
    /// Index into `AgentProvider::ALL` of the agent driving the chat.
    pub chat_selected_agent: usize,

    // --- Alignment toolbar -----------------------------------------
    /// Align-toolbar button currently hovered.
    pub align_toolbar_hover: Option<crate::align::AlignAction>,

    // --- Property panel: tabs + layout toggles ---------------------
    /// Active PropertyPanel tab — toggled by `Cmd+Shift+C`.
    pub property_tab: PropertyTab,
    /// Active flex-layout mode for the property panel's row.
    pub flex_layout: FlexLayout,
    pub size_fill_width: bool,
    pub size_fill_height: bool,
    pub size_hug_width: bool,
    pub size_hug_height: bool,
    pub size_clip_content: bool,
    /// Whether the fill-type dropdown is open.
    pub fill_type_picker_open: bool,
    /// Active-theme axis whose value picker is open; `None` = closed.
    pub axis_dropdown_open: Option<String>,
    /// Editor focus for a non-color variable row (Number / String).
    pub variable_row_focus: Option<VariableRowFocus>,

    // --- Layer / page hover + context menu -------------------------
    /// Currently-hovered LayerPanel row, or `None`.
    pub hovered_layer_id: Option<NodeId>,
    /// Page-row hover state for the Pages section, or `None`.
    pub hovered_page_index: Option<usize>,
    /// Open layer-row context menu, or `None`.
    pub layer_context_menu: Option<LayerContextMenuState>,
    /// Node ids whose children are collapsed in the LayerPanel.
    /// Editor-only UI state — the canonical `PenNodeBase` has no
    /// `collapsed` field, so the collapse flag lives here rather than
    /// on the node.
    ///
    /// Layer-collapse is view-only UI state: deliberately excluded from
    /// the undo snapshot and from file persistence. It is transient —
    /// rebuilt on load, never serialized, and toggling it never pushes
    /// a history entry.
    pub collapsed_layers: HashSet<NodeId>,
    /// Caret-blink anchor (ms) for an inline layer / page rename.
    pub rename_caret_anchor_ms: u64,
    /// Last LayerPanel click target + ms; 400 ms re-press → rename.
    pub last_layer_click: Option<(LayerContextTarget, u64)>,
    /// Last canvas left-click target + ms; 400 ms same-node re-press
    /// on a Text node promotes to inline text edit.
    pub last_canvas_click: Option<(NodeId, u64)>,
}

impl Default for EditorUiState {
    fn default() -> Self {
        Self {
            sidebar_open: true,
            layer_panel_width: 240.0,
            property_panel_width: 280.0,
            theme_mode: ThemeMode::Dark,
            locale: Locale::ZhCn,
            locale_picker_open: false,
            locale_picker_hover: None,
            file_menu_open: false,
            file_menu_hover: None,
            pending_file_action: None,
            recent_files: Vec::new(),
            file_name_display: None,
            export_scale: 2.0,
            export_dialog_open: false,
            export_format: ExportFormat::Png,
            figma_import_open: false,
            agent_settings_open: false,
            agent_settings: crate::agent_settings::AgentSettings::default(),
            agent_settings_drag: None,
            settings_input_draft: String::new(),
            shape_picker_open: false,
            shape_picker_hover: None,
            shape_tool: Tool::Rect,
            chat_model_picker_open: false,
            chat_selected_agent: 0,
            align_toolbar_hover: None,
            property_tab: PropertyTab::Design,
            flex_layout: FlexLayout::Free,
            size_fill_width: false,
            size_fill_height: false,
            size_hug_width: false,
            size_hug_height: false,
            size_clip_content: false,
            fill_type_picker_open: false,
            axis_dropdown_open: None,
            variable_row_focus: None,
            hovered_layer_id: None,
            hovered_page_index: None,
            layer_context_menu: None,
            collapsed_layers: HashSet::new(),
            rename_caret_anchor_ms: 0,
            last_layer_click: None,
            last_canvas_click: None,
        }
    }
}

impl EditorUiState {
    /// A fresh UI state — sidebar open, dark theme, no menus open.
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_editor_ui_is_quiescent() {
        let c = EditorUiState::new();
        assert!(c.sidebar_open);
        assert_eq!(c.theme_mode, ThemeMode::Dark);
        assert_eq!(c.locale, Locale::ZhCn);
        assert!(!c.file_menu_open);
        assert!(!c.export_dialog_open);
        assert!(!c.agent_settings_open);
        assert_eq!(c.shape_tool, Tool::Rect);
        assert_eq!(c.property_tab, PropertyTab::Design);
        assert_eq!(c.flex_layout, FlexLayout::Free);
        assert!(c.recent_files.is_empty());
        assert!(c.collapsed_layers.is_empty());
    }

    #[test]
    fn theme_mode_flips() {
        assert_eq!(ThemeMode::Dark.flipped(), ThemeMode::Light);
        assert_eq!(ThemeMode::Light.flipped(), ThemeMode::Dark);
    }

    #[test]
    fn export_format_metadata() {
        assert_eq!(ExportFormat::ALL.len(), 5);
        assert_eq!(ExportFormat::Png.extension(), "png");
        assert_eq!(ExportFormat::Jpeg.extension(), "jpg");
    }
}
