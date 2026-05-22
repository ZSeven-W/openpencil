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

/// Auto-update status surfaced in the settings modal's System tab.
///
/// The desktop host runs a background probe against the GitHub
/// releases API and writes the outcome here; the System tab paints
/// from it. `Idle` is the pre-probe state, `Checking` while the
/// request is in flight. `Available` carries the newer release tag
/// so the tab can name the version the user can upgrade to.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UpdateStatus {
    /// No check has run yet (also the web build's permanent state).
    #[default]
    Idle,
    /// A release-API request is in flight.
    Checking,
    /// The running build is the latest published release.
    UpToDate,
    /// A newer release exists — carries its version string.
    Available { version: String },
    /// The probe failed (offline, rate-limited, parse error).
    Error,
}

/// One commit row shown in the Git panel — plain data snapshotted
/// by the desktop host from its git session. The platform-free
/// widget layer only paints it; it never calls git itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitCommitSummary {
    /// Abbreviated commit hash.
    pub short_hash: String,
    /// First line of the commit message.
    pub summary: String,
    /// Author display name.
    pub author: String,
}

/// One changed file in the Git panel's staging list — plain data
/// snapshotted by the desktop host from `git status`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitFileEntry {
    /// Repo-relative path.
    pub path: String,
    /// Whether the change is staged in the index.
    pub staged: bool,
    /// Single-char status: `M` / `A` / `D` / `R` / `?` / `U`.
    pub status: char,
}

/// What a Git-panel diff request should diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitDiffTarget {
    /// The whole working tree's unstaged changes (`git diff`).
    WorkingTree,
    /// One repo-relative path's working-tree changes (`git diff -- <path>`).
    Path(String),
    /// The full patch a commit introduced (`git show <rev>`).
    Commit(String),
}

/// A unified-diff view open inside the Git panel — filled by the
/// desktop host from a background `git diff` / `git show` job.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitDiffView {
    /// Human label for the diff (a path, or a commit summary).
    pub title: String,
    /// The diff text split into lines for per-line colouring.
    pub lines: Vec<String>,
    /// Index of the first visible line — paged by the ▲ / ▼ buttons
    /// and the mouse wheel.
    pub scroll: usize,
    /// First visible character column — long lines scroll sideways
    /// with the ◀ / ▶ buttons.
    pub h_scroll: usize,
    /// Repo-relative path when this diff is a single working-tree
    /// file that supports per-hunk staging — `None` for a commit
    /// diff or the whole-tree diff.
    pub stage_path: Option<String>,
}

/// One node conflict in the interactive merge-resolution view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeConflictRow {
    /// The conflicting node's `id`.
    pub id: String,
    /// Display label — the node's name / type.
    pub label: String,
    /// Human label for the conflict kind (e.g. "both modified").
    pub kind: String,
    /// Whether "theirs" is a selectable resolution — `false` for a
    /// structural conflict, which can only be resolved to "ours".
    pub theirs_allowed: bool,
    /// The chosen side: `false` = keep ours, `true` = take theirs.
    pub take_theirs: bool,
}

/// One conflicted `.op` file in the merge-resolution view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeResolveFile {
    /// Repo-relative path.
    pub path: String,
    /// The three merge-stage blobs — kept so "Apply" can re-run the
    /// structured merge with the user's per-node choices.
    pub base: String,
    pub ours: String,
    pub theirs: String,
    /// The file's node conflicts.
    pub conflicts: Vec<MergeConflictRow>,
}

/// Interactive merge-conflict-resolution state — set when a branch
/// merge conflicts entirely in structured `.op` files. The panel
/// shows each conflicting node with an ours/theirs choice.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MergeResolveState {
    /// The branch being merged in.
    pub branch: String,
    /// The conflicted `.op` files.
    pub files: Vec<MergeResolveFile>,
}

impl MergeResolveState {
    /// Total conflict count across every file.
    pub fn total(&self) -> usize {
        self.files.iter().map(|f| f.conflicts.len()).sum()
    }

    /// Every conflict row, flattened in file order — the order the
    /// panel paints and hit-tests.
    pub fn rows(&self) -> Vec<&MergeConflictRow> {
        self.files.iter().flat_map(|f| &f.conflicts).collect()
    }

    /// Set the choice of the flat-indexed conflict row. A `theirs`
    /// choice on a structural conflict falls back to "ours".
    pub fn set_choice(&mut self, flat_index: usize, take_theirs: bool) {
        let mut i = 0;
        for file in &mut self.files {
            for row in &mut file.conflicts {
                if i == flat_index {
                    row.take_theirs = take_theirs && row.theirs_allowed;
                    return;
                }
                i += 1;
            }
        }
    }
}

/// An interactive action requested from the Git panel. The desktop
/// host drains it from [`GitPanelState::pending_action`] and runs it
/// against its `GitSession` (the widget layer never calls git).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitPanelAction {
    /// Re-read repository state into the panel.
    Refresh,
    /// Pull the current branch's upstream.
    Pull,
    /// Push the current branch to its upstream.
    Push,
    /// Stage + commit the tracked document with the panel's
    /// `commit_message`.
    Commit,
    /// Switch the working tree to the named branch.
    SwitchBranch(String),
    /// Add / re-point the `origin` remote to the given URL.
    SetRemote(String),
    /// Generate (or reuse) an SSH key for the `origin` host and bind
    /// it as that host's stored credential.
    SetupSshAuth,
    /// Store an HTTPS credential for the `origin` host — the payload
    /// is the `username:token` text typed into the Remotes section.
    SetHttpsAuth(String),
    /// Merge the named branch into the current one through an
    /// isolated worktree (the live tree is never marked up).
    MergeBranch(String),
    /// Abort an in-progress merge, restoring the pre-merge state.
    AbortMerge,
    /// Finalize an in-progress merge once its conflicts are resolved.
    CompleteMerge,
    /// Compute a unified diff and open it in the panel's diff view.
    ShowDiff(GitDiffTarget),
    /// Toggle whether the named changed file is staged in the index.
    ToggleStageFile(String),
    /// Stage a single hunk of the open diff — `(path, hunk_index)`.
    StageHunk(String, usize),
    /// Re-run the branch merge applying the per-node ours/theirs
    /// choices the user picked in the merge-resolution view.
    ApplyMergeResolution,
}

/// Git panel state — a plain-data snapshot the desktop host fills
/// from its `GitSession`. The widget layer reads it to paint the
/// floating Git panel; it carries no git handles, so it stays
/// wasm-clean. Refreshed whenever the panel is opened.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitPanelState {
    /// Whether the floating Git panel is currently shown.
    pub open: bool,
    /// Whether the open document lives inside a git repository.
    pub in_repo: bool,
    /// Current branch name of that repository.
    pub branch: Option<String>,
    /// All local branch names, sorted — the panel lists them for
    /// one-click switching.
    pub branches: Vec<String>,
    /// Number of changed (dirty) files in the working tree.
    pub dirty_count: usize,
    /// Number of files with unresolved merge conflicts.
    pub conflicted_count: usize,
    /// Whether a merge is in progress — drives the panel's conflict
    /// mode (conflicted-file list + Abort / Complete actions).
    pub merging: bool,
    /// Repo-relative paths with unresolved merge conflicts.
    pub conflicted_files: Vec<String>,
    /// Changed files in the working tree — the per-file staging list.
    pub changed_files: Vec<GitFileEntry>,
    /// Configured remotes as display strings — `name → url`.
    pub remotes: Vec<String>,
    /// Draft URL typed into the Remotes section's input box.
    pub remote_draft: String,
    /// Whether the remote-URL input holds keyboard focus.
    pub remote_focused: bool,
    /// Draft `username:token` typed into the HTTPS-credential input.
    pub https_draft: String,
    /// Whether the HTTPS-credential input holds keyboard focus.
    pub https_focused: bool,
    /// Most-recent commits, newest first.
    pub recent_commits: Vec<GitCommitSummary>,
    /// Commit-message draft typed into the panel's input box.
    pub commit_message: String,
    /// Whether the commit-message input holds keyboard focus.
    pub commit_focused: bool,
    /// Interactive action requested by a panel click / Enter —
    /// drained and executed by the desktop host.
    pub pending_action: Option<GitPanelAction>,
    /// Whether a background `git pull` is currently in flight — the
    /// panel shows a "Pulling…" status and disables the Pull button.
    pub pulling: bool,
    /// Whether a background `git push` is currently in flight — the
    /// panel shows a "Pushing…" status and disables the Push button.
    pub pushing: bool,
    /// Whether the panel is awaiting its first repository snapshot
    /// after opening / a repo switch. While `true` the panel shows a
    /// "Loading…" state instead of the (possibly stale) prior data.
    pub loading: bool,
    /// Open diff view — `Some` puts the panel into diff mode, showing
    /// a scrollable unified diff instead of the status / action area.
    /// Closed by the diff view's ✕ button.
    pub diff: Option<GitDiffView>,
    /// Interactive merge-conflict-resolution view — `Some` puts the
    /// panel into resolution mode, listing each conflicting node with
    /// an ours/theirs choice. Cleared on Apply / Cancel.
    pub merge_resolve: Option<MergeResolveState>,
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
    /// "Import from Figma" modal.
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
    /// Vertical scroll offset of the model-picker dropdown, in px.
    /// Non-zero only when the connected catalog is taller than the
    /// picker's capped height; the host clamps it on wheel input.
    pub chat_model_picker_scroll: f32,
    /// Index into `chat.available_models` of the model row the cursor
    /// is over, or `None`. Drives the picker's hover-row tint.
    pub chat_model_picker_hover: Option<usize>,
    /// Index into `AgentProvider::ALL` of the agent driving the chat.
    pub chat_selected_agent: usize,

    // --- Window chrome ---------------------------------------------
    /// Cursor is over the TopBar's window-control (traffic-light)
    /// cluster — reveals the close / minimise / maximise glyphs.
    pub topbar_traffic_hover: bool,
    /// Window is in fullscreen. macOS hides the native traffic
    /// lights then, so the TopBar drops its left-edge reservation.
    pub window_fullscreen: bool,

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
    /// Smart-guide lines to paint during the current node drag —
    /// computed each `apply_cursor_move` by `align_guides`, cleared on
    /// drag release. View-only transient state: never serialized,
    /// never part of the undo snapshot.
    pub active_guides: Vec<crate::align_guides::AlignmentGuide>,

    // --- Auto-update ------------------------------------------------
    /// Latest result of the desktop host's background update probe.
    /// Transient: never serialized, rebuilt each launch.
    pub update_status: UpdateStatus,

    // --- In-app Git -------------------------------------------------
    /// Floating Git panel snapshot — filled by the desktop host from
    /// its `GitSession`. Transient: never serialized.
    pub git_panel: GitPanelState,

    // --- Design-MD panel --------------------------------------------
    /// Whether the floating Design-MD panel is shown.
    pub design_md_panel_open: bool,
    /// Top-left corner of the Design-MD panel in logical px. `None`
    /// until first opened — the host then centres it on the viewport.
    pub design_md_panel_pos: Option<(f32, f32)>,
    /// Bitmask of expanded Design-MD sections (bit 0 = theme, 1 =
    /// colors, 2 = typography, 3 = components, 4 = layout, 5 = notes).
    /// Defaults to theme + colors + typography expanded.
    pub design_md_expanded: u8,
    /// A queued Design-MD import / export request — set by a panel
    /// click, drained by the desktop host (which owns the native file
    /// dialog). Transient: never serialized.
    pub design_md_request: Option<DesignMdRequest>,

    // --- Component browser ------------------------------------------
    /// Whether the floating Component-Browser panel is shown.
    pub component_browser_open: bool,
    /// Top-left corner of the Component-Browser panel in logical px;
    /// `None` until first opened — the host then centres it.
    pub component_browser_pos: Option<(f32, f32)>,
    /// Live search filter — names + tags substring-match against this.
    pub component_browser_search: String,
    /// Active category pill (`None` = all categories).
    pub component_browser_category: Option<crate::uikit::ComponentCategory>,
    /// Active kit filter (`None` = every loaded kit). Kept for the
    /// future imported-kits surface; v1 ships one built-in kit.
    pub component_browser_kit_id: Option<String>,
    /// A queued component-instantiate request — `(kit_id, comp_id)`,
    /// set by a card click, drained by the desktop host so it can
    /// run the instantiate against the viewport's centre.
    pub component_browser_pending_insert: Option<(String, String)>,
}

/// A Design-MD panel action that needs the desktop host's native
/// file dialog — set by the widget layer, drained by the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesignMdRequest {
    /// Pick a `.md` file, parse it, and set `design_md`.
    Import,
    /// Write the current `design_md` to a `.md` file.
    Export,
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
            chat_model_picker_scroll: 0.0,
            chat_model_picker_hover: None,
            chat_selected_agent: 0,
            topbar_traffic_hover: false,
            window_fullscreen: false,
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
            active_guides: Vec::new(),
            update_status: UpdateStatus::Idle,
            git_panel: GitPanelState::default(),
            design_md_panel_open: false,
            design_md_panel_pos: None,
            design_md_expanded: 0b0000_0111,
            design_md_request: None,
            component_browser_open: false,
            component_browser_pos: None,
            component_browser_search: String::new(),
            component_browser_category: None,
            component_browser_kit_id: None,
            component_browser_pending_insert: None,
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
