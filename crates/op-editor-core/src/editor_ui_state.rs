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
//! module — for example `ExportFormat` in `widgets/export_dialog.rs`.
//! They are data/state enums, not rendering code, so their type
//! definitions belong in the state layer. The widget *painting /
//! hit-test* code stays in shell-core untouched.
//!
//! All types here are plain data (enums + structs of primitives /
//! strings / ids), so `op-editor-core` stays wasm32-clean.
//!
//! ### Module layout
//!
//! This file is the public spine: the [`EditorUiState`] struct itself
//! plus the shared re-exports. Everything else lives in sibling
//! submodules (per the 800-line-per-file ceiling) and is re-exported
//! here, so every existing `editor_ui_state::*` import path still
//! resolves:
//!
//! - [`chrome`] — theme / embed host / file actions / recent files /
//!   theme-preset IO / update status / Design-MD request / pencil cursor
//! - [`pickers`] — picker purposes, canvas overlay geometry, layer
//!   context menu + page rename, variable-row / effect-param focus
//! - [`git_panel`] — the whole in-app Git panel data model
//! - [`groups`] — grouped sub-states (preview / size toggles /
//!   Design-MD panel) carved out of the flat field list
//! - `defaults` — `impl Default for EditorUiState`
//! - `methods` — `impl EditorUiState`

pub mod chrome;
mod defaults;
pub mod git_panel;
pub mod groups;
mod methods;
pub mod pickers;
#[cfg(test)]
mod tests;

pub use chrome::{
    DesignMdRequest, EmbedHost, FileAction, PencilCursorStyle, RecentFile, ThemeMode,
    ThemePresetIo, UpdateStatus, RECENT_FILE_CAP,
};
pub use git_panel::{
    CloneField, CloneFormState, CommitDiffPatch, CommitDiffSummary, CommitDiffView,
    GitBranchPickerMode, GitCandidateFile, GitCommitSummary, GitDiffTarget, GitDiffView,
    GitFileEntry, GitOverflowView, GitPanelAction, GitPanelState, MergeConflictRow,
    MergeResolveFile, MergeResolveState,
};
pub use groups::{
    CustomPrompt, DesignMdPanelState, PreviewState, PromptCenterFocus, PromptCenterState,
    PromptFilter, SceneFilter, SceneTemplateCenterState, SizeToggleState,
};
pub use pickers::{
    CanvasDropIndicator, CanvasOverlayLine, CanvasOverlayRect, CompositingPickerTarget,
    EffectParamFocus, FontPickerPurpose, LayerContextMenuState, MissingFontSurface,
    PageRenameState, PreviewDeviceKind, VariableRowFocus,
};

use crate::node_id::NodeId;
use crate::tool::Tool;
use std::collections::HashSet;

// `Locale` is the i18n locale enum — dependency-free + wasm-clean, so
// it lives in `op-i18n` and re-exports cleanly into the state layer.
pub use op_i18n::Locale;

pub use crate::property_panel_state::{
    BooleanOp, ExportFormat, FillType, FlexLayout, ImageAdjustmentField, ImageFillMode,
    PaddingEditMode, PropertyTab,
};

// What the LayerPanel right-click context menu is acting on — the
// canonical definition is `ui_draft::LayerContextTarget` (it backs
// the inline-rename draft too). Re-exported so UI code that
// references a context target has one import path.
pub use crate::ui_draft::LayerContextTarget;

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
    /// TopBar Globe dropdown state.
    pub locale_picker: jian_widgets::components::select::SelectState,
    /// User's last-set ⚡Nx parallel-agents team size — an app-level
    /// preference (persisted via `settings_io`), NOT the per-tab
    /// `ChatState::agent_team_size` it seeds. `ChatSessions::new_tab`
    /// carries the ACTIVE tab's current value forward for continuity
    /// within a session; this field is what re-seeds tab 0's value across
    /// a full app restart, where no "active tab" from a prior session
    /// exists to carry forward from. Old `settings.json` files predating
    /// this field default to `1` (serde default), matching
    /// `ChatState::default().agent_team_size`.
    pub preferred_agent_team_size: u32,

    // --- Collaboration ---------------------------------------------
    /// Sanitized collaboration display state shared by native and web
    /// widgets. Transport handles, tickets, stable subjects, and device ids
    /// deliberately never enter this paint-state projection.
    pub collab: crate::collab_ui_state::CollabUiState,

    // --- File menu --------------------------------------------------
    /// File-menu dropdown open (anchored under folder + chevron).
    pub file_menu_open: bool,
    /// Shared file-menu interaction state; `hover = None` means no
    /// actionable row hovered.
    pub file_menu: jian_widgets::components::menu::MenuState,
    /// Pending file-menu action for the host runner to handle.
    pub pending_file_action: Option<FileAction>,
    /// Recent files (head = newest, cap 10).
    pub recent_files: Vec<RecentFile>,
    /// TopBar display name; `None` = "Untitled".
    pub file_name_display: Option<String>,
    /// Derived from `EditorState::revision != saved_revision`; painted
    /// by the TopBar only, never serialized.
    pub document_dirty: bool,

    // --- Modals -----------------------------------------------------
    /// Raster export scale (1.0 / 2.0 / 3.0). Default 2.0.
    pub export_scale: f32,
    pub export_dialog_open: bool,
    /// Which modal export-dialog button the cursor is over — drives the
    /// per-button `theme.button_hover` wash. Updated by the host on
    /// cursor-move while the dialog is open.
    pub export_dialog_hover: Option<crate::export_dialog_state::ExportDialogButton>,
    pub export_format: ExportFormat,
    /// Property-panel Export section: the scale dropdown's inline
    /// select popup is open. Mutually exclusive with
    /// `export_format_picker_open` — opening one closes the other.
    pub export_scale_picker_open: bool,
    /// Property-panel Export section: the format dropdown's inline
    /// select popup is open.
    pub export_format_picker_open: bool,
    /// Row index the cursor is over in the open Export select popup
    /// (drives the row hover highlight). `None` when no popup is
    /// open or the cursor is off every row.
    pub export_picker_hover: Option<usize>,
    /// Index into the PropertyPanel's `action_button_rects_with_fill_picker`
    /// walker of the action button the cursor is over — drives its
    /// `theme.button_hover` wash. Stable per frame (same VisibleSections
    /// + fill-picker state feed paint + the host hover update).
    pub property_action_hover: Option<usize>,
    /// PropertyPanel Design / Code tab currently under the cursor. Drives a
    /// visible pill on inactive tabs.
    pub property_tab_hover: Option<PropertyTab>,
    /// Vertical scroll offset of the right-rail PropertyPanel, in px
    /// (≥ 0). A wheel / trackpad pan over the inspector advances it;
    /// paint + hit-test shift the section content up by this amount
    /// so a tall inspector (many effects, etc.) stays reachable.
    pub property_panel_scroll: jian_core::scroll::ScrollState,
    /// Vertical scroll offset (px, ≥ 0) of the LayerPanel's 页面
    /// (Pages) section — that section has a bounded height, so a
    /// long page list scrolls within it.
    pub layer_pages_scroll: jian_core::scroll::ScrollState,
    /// Vertical scroll offset (px, ≥ 0) of the LayerPanel's 图层
    /// (Layers) section row viewport.
    pub layer_layers_scroll: jian_core::scroll::ScrollState,
    /// Horizontal scroll offset (px, ≥ 0) of the LayerPanel's 页面
    /// row content. The row chrome stays fixed; only tree content shifts.
    pub layer_pages_h_scroll: jian_core::scroll::ScrollState,
    /// Horizontal scroll offset (px, ≥ 0) of the LayerPanel's 图层
    /// tree content. Needed for deeply nested layer trees.
    pub layer_layers_h_scroll: jian_core::scroll::ScrollState,
    /// Top-bar import dropdown (`从 Figma 导入` / `从 HTML 导入`).
    pub import_menu_open: bool,
    /// Shared `Select` scroll / hover state for that dropdown.
    pub import_menu: jian_widgets::components::select::SelectState,
    /// Import modal (shared by the Figma and HTML sources).
    pub figma_import_open: bool,
    /// Which source that modal is importing.
    pub import_source: crate::figma_import_state::ImportSource,
    /// Which Figma-import target the cursor is over (close / drop-zone)
    /// — drives the `theme.button_hover` wash. Host updates on cursor-move.
    pub figma_import_hover: Option<crate::figma_import_state::FigmaImportButton>,
    /// Page summaries shown after desktop has prepared a multi-page
    /// `.fig`. Empty while the modal is in its initial file-picker mode.
    pub figma_import_pages: Vec<crate::figma_import_state::FigmaImportPage>,
    /// Shared Select hover/pressed/scroll state for the page list.
    pub figma_import_page_select: jian_widgets::components::select::SelectState,
    /// True while a `.fig` is being parsed on a worker thread. Paint
    /// uses this to show a "正在解析 Figma 文件…" overlay so the user
    /// gets feedback during the multi-second parse (a 2-3 MB .fig with
    /// hundreds of nodes can take a couple of seconds to walk the
    /// Kiwi schema, build the tree, and convert every node). The
    /// desktop runner sets it when spawning the worker and clears it
    /// when the result lands.
    pub figma_import_in_progress: bool,
    /// True while a file is being dragged over the window (between the
    /// platform's `HoveredFile` and `HoveredFileCancelled` / drop). Drives
    /// the full-canvas drop overlay so the user sees a clear drop target.
    pub file_drop_active: bool,
    /// Node the hovering file would fill if released now (image files over a
    /// frame / rectangle / … — see `image_drop`). Drives the drop-target ring
    /// painted by the same overlay. `None` when the drop would open a document
    /// or insert a standalone node instead. Never serialized.
    pub file_drop_target: Option<crate::NodeId>,
    /// Imported Figma documents parsed in Preserve mode already carry
    /// authored parent-local geometry. The scene builder can use this
    /// flag to skip the expensive flex/text layout pass.
    pub preserve_authored_geometry: bool,
    // --- Canvas preview (Play) mode -------------------------------
    /// Preview (Play) mode flag + device / screen switcher state.
    pub preview: PreviewState,
    /// Floating `Cmd+,` agent-settings modal open.
    pub agent_settings_open: bool,
    pub agent_settings: crate::agent_settings::AgentSettings,
    pub agent_settings_drag: Option<crate::agent_settings::AgentSettingsDrag>,
    /// Draft, caret, selection, and blink state for the focused
    /// settings-modal input.
    pub settings_input: jian_core::text_input::TextInputState,

    // --- Account (platform + zseven-sso user system) ---------------
    /// Runtime release gate for the account experience. Hosts set this
    /// at startup when a working auth backend is linked (the
    /// `op-auth-bridge` prebuilt library, or the dev fake-login env);
    /// stub builds and the wasm host leave it false, hiding every
    /// account entry point exactly like the old compile-time gate.
    pub account_ui_available: bool,
    /// Signed-in / signed-out identity, fed by the zseven-sso device
    /// login flow (browser pairing + WebSocket push) — see
    /// `AccountState::dev_fake_signed_in` for the dev-only fast path.
    pub account: crate::account_state::AccountState,
    /// TopBar avatar-button dropdown (signed-in state) open.
    pub account_menu_open: bool,
    /// Which account-dropdown row the cursor is over.
    pub account_menu_hover: Option<crate::account_state::AccountMenuRow>,
    /// Sign-in modal (signed-out state) open.
    pub login_modal_open: bool,
    /// Which login-modal control the cursor is over.
    pub login_modal_hover: Option<crate::account_state::LoginModalButton>,
    /// Set after the production "Sign in with browser" button is
    /// clicked in a build without an auth backend — reveals the honest
    /// "coming soon" note instead of pretending a flow ran. Untouched
    /// by the dev fake-login and real device-login paths.
    pub login_modal_stub_hint_shown: bool,
    /// Progress note for an in-flight browser device-login, painted in
    /// the sign-in modal. `None` when no flow is running.
    pub login_modal_status: Option<crate::account_state::LoginFlowStatus>,

    // --- Toolbar shape slot ----------------------------------------
    /// Toolbar shape-tool dropdown state.
    pub shape_picker: jian_widgets::components::select::SelectState,
    /// Toolbar button currently hovered — drives the per-button
    /// `theme.button_hover` wash on the vertical tool column.
    pub toolbar_hover: Option<crate::toolbar_state::ToolbarHover>,
    /// Last-selected shape tool — drives the toolbar shape slot's
    /// icon. Always one of Rect / Ellipse / Polygon / Line / Pen.
    pub shape_tool: Tool,
    /// Toolbar/property icon picker interaction state.
    pub icon_picker: jian_widgets::components::select::SelectState,
    /// True when the icon picker should replace the selected icon
    /// instead of inserting a new icon at the canvas centre.
    pub icon_picker_replace_selection: bool,
    /// Top-left corner of the floating Icon picker in logical px.
    /// `None` until first dragged/opened, then reused across opens.
    pub icon_picker_panel_pos: Option<(f32, f32)>,
    /// Live text filter for the native Lucide icon picker.
    pub icon_picker_search: String,
    /// True after Cmd/Ctrl+A in the icon search box. The next edit
    /// replaces the whole search query.
    pub icon_picker_select_all: bool,
    /// Remote Iconify search results appended by the desktop host.
    pub icon_picker_remote: crate::icon_picker_state::IconPickerRemoteState,
    /// Queued "load more" request drained asynchronously by desktop.
    pub icon_picker_load_more_request: Option<crate::icon_picker_state::IconifyLoadMoreRequest>,

    // --- AI chat model picker --------------------------------------
    /// AI chat model-picker dropdown interaction state.
    pub chat_model_picker: jian_widgets::components::select::SelectState,
    /// Text filter, caret, selection, and blink state for the chat
    /// model-picker search box.
    pub chat_model_picker_input: jian_core::text_input::TextInputState,
    /// Request seam raised every time the model picker OPENS: a host
    /// with local CLI access re-discovers the external providers'
    /// catalogs so a CLI that shipped new models mid-session is listed
    /// without an app restart. Drained by the desktop pump
    /// (`drain_model_catalog_refresh`), which applies its own TTL
    /// debounce; hosts without subprocess access simply clear it.
    pub pending_model_catalog_refresh: bool,
    /// Hovered chat design JSON card `(message_index, block_index)`;
    /// drives the TS-style hover reveal of the card's copy affordance.
    pub chat_design_block_hover: Option<(usize, usize)>,
    /// Index of the empty-state quick action card under the cursor.
    pub chat_example_hover: Option<usize>,
    /// Which bare chat header button (chevron / maximize / new-chat)
    /// the cursor is over — drives their `theme.button_hover` wash.
    pub chat_header_hover: Option<crate::chat_button_state::ChatHeaderButton>,
    /// Which bottom-toolbar chat control the cursor is over.
    pub chat_footer_hover: Option<crate::chat_button_state::ChatFooterButton>,
    /// Index of the tab-row tab the cursor is over — drives the × close glyph
    /// and hover wash on inactive tabs. `None` when not hovering any tab.
    pub chat_tab_hover: Option<usize>,
    /// Index into `AgentProvider::ALL` of the agent driving the chat.
    pub chat_selected_agent: usize,
    /// Whether the Parallel Agents picker dropdown is open.
    /// Set by `toggle_parallel_agents_picker`; cleared on outside-click or row select.
    pub parallel_agents_picker_open: bool,
    /// Which row (1–6) the cursor is over inside the Parallel Agents picker —
    /// drives the hover-highlight wash. `None` = no hover / picker closed.
    pub parallel_agents_picker_hover: Option<u32>,

    /// Primary-pointer pressed button target. Button feedback is exclusive
    /// across chrome families, so one field covers toolbar / topbar /
    /// statusbar / chat buttons without duplicating every hover field.
    pub pressed_button: Option<crate::button_press_state::ButtonPressTarget>,

    /// True after Cmd/Ctrl+A in the component-browser search box. The
    /// next edit replaces the whole search query.
    pub component_browser_select_all: bool,

    // --- Window chrome ---------------------------------------------
    /// Cursor is over the TopBar's window-control (traffic-light)
    /// cluster — reveals the close / minimise / maximise glyphs.
    pub topbar_traffic_hover: bool,
    /// Which TopBar chrome button the cursor is over — drives the
    /// `theme.button_hover` wash on the sidebar / file-menu / figma /
    /// theme / locale / fullscreen / git / agent-chip buttons.
    pub topbar_button_hover: Option<crate::topbar_state::TopBarButton>,
    /// Which floating status-bar control the cursor is over — drives
    /// the `theme.button_hover` wash on the search / zoom-out / zoom-in
    /// controls.
    pub statusbar_hover: Option<crate::statusbar_state::StatusBarButton>,
    /// Window is in fullscreen. macOS hides the native traffic
    /// lights then, so the TopBar drops its left-edge reservation.
    pub window_fullscreen: bool,
    /// Raised when the TopBar fullscreen button is clicked. The host
    /// runner (which owns the window) consumes it next frame to toggle
    /// the actual window fullscreen, then clears it.
    pub pending_fullscreen_toggle: bool,
    /// Raised when the user clicks a chat tab's close-× (MT.3
    /// `AIChatHit::CloseTab`). Carries the tab index to remove. The host
    /// runner drains it next frame: closing a tab can need to abort an
    /// in-flight run bound to it (a `current_chat` / `current_design`
    /// session the widget layer cannot reach), so the actual `close_tab` +
    /// run-binding fix-up runs host-side, then this clears. `None` = idle.
    pub pending_close_chat_tab: Option<usize>,
    /// Which embedding container the editor chrome renders inside.
    pub embed: EmbedHost,

    // --- Alignment toolbar -----------------------------------------
    /// Align-toolbar button currently hovered.
    pub align_toolbar_hover: Option<crate::align::AlignAction>,

    // --- Property panel: tabs + layout toggles ---------------------
    /// Active PropertyPanel tab — toggled by `Cmd+Shift+C`.
    pub property_tab: PropertyTab,
    /// Active flex-layout mode for the property panel's row.
    pub flex_layout: FlexLayout,
    /// Padding-section edit mode pinned via the gear popover. `None`
    /// re-derives the mode from the node's values each frame (TS
    /// default); `Some(_)` keeps the user's pick. The pin is scoped to
    /// [`Self::padding_edit_mode_anchor`] so it never leaks into the
    /// next selection — the panel ignores it once the anchor differs.
    pub padding_edit_mode: Option<PaddingEditMode>,
    /// Node id (anchor) the [`Self::padding_edit_mode`] pin was set for.
    /// Empty when unset. The property panel honours the pin only while
    /// the current selection anchor still matches, so selecting another
    /// node falls back to deriving the mode from that node's values.
    pub padding_edit_mode_anchor: String,
    /// Whether the padding-mode gear popover is open.
    pub padding_mode_popover_open: bool,
    /// Index (into `PaddingEditMode::ALL`) of the popover row under the
    /// cursor while the gear popover is open — drives the hover wash.
    pub padding_mode_popover_hover: Option<usize>,
    /// Stroke-width edit mode pinned via the stroke-section gear.
    /// Scoped to [`Self::stroke_edit_mode_anchor`] like padding.
    pub stroke_edit_mode: Option<PaddingEditMode>,
    /// Node id (anchor) the stroke edit-mode pin was set for.
    pub stroke_edit_mode_anchor: String,
    /// Whether the stroke-mode gear popover is open.
    pub stroke_mode_popover_open: bool,
    /// Index (into `PaddingEditMode::ALL`) of the stroke popover row
    /// under the cursor while the gear popover is open.
    pub stroke_mode_popover_hover: Option<usize>,
    /// PropertyPanel Size-section fill / hug / clip toggles.
    pub size_toggles: SizeToggleState,
    /// Fill-type dropdown state in the PropertyPanel.
    pub fill_type_picker: jian_widgets::components::select::SelectState,
    /// Which fill row the open fill-type dropdown targets. The Fill
    /// section stacks one row per fill, each with its own type
    /// dropdown, so `fill_type_picker.open` plus this index identify
    /// the row whose picker is showing. Meaningless when the picker is
    /// closed; defaults to `0`.
    pub fill_type_picker_index: usize,
    /// Shared dropdown interaction state for node blend, node mask, and
    /// per-fill blend-mode options.
    pub compositing_picker: jian_widgets::components::select::SelectState,
    /// Property row that owns the open compositing picker.
    pub compositing_picker_target: Option<CompositingPickerTarget>,
    /// Whether the selected Ref's inline component-target list is open.
    pub instance_component_picker_open: bool,
    /// Selection anchor that owns the open component-target list.
    /// Scoping prevents a picker opened on one Ref from leaking to the next.
    pub instance_component_picker_anchor: String,
    /// Whether the Position section shows the per-corner 2×2 radius grid.
    pub corner_expand_open: bool,
    /// Whether the Effects section's three-kind "+" add-menu is open.
    pub effect_add_picker_open: bool,
    /// Row index hovered in the Effects add-menu (`None` = none), so the
    /// popover highlights the row under the cursor like the other
    /// property-panel dropdowns.
    pub effect_add_menu_hover: Option<usize>,
    /// Whether the Interactions section's Navigate/Back/Remove popover
    /// is open. `false` = closed.
    pub interaction_menu_open: bool,
    /// Row index hovered in the open Interactions popover (`None` =
    /// none).
    pub interaction_menu_hover: Option<usize>,
    /// Fill/stroke colour-variable dropdown currently open in the
    /// PropertyPanel; `None` means closed.
    pub property_color_variable_picker_open: Option<crate::ui_draft::ColorTarget>,
    /// Scroll offset of that dropdown's own list. The popup is height
    /// capped, so a document with many colour variables scrolls inside
    /// it rather than stretching the inspector. Reset when it opens.
    pub property_color_variable_picker_scroll: jian_core::scroll::ScrollState,
    /// Row slot hovered in that dropdown (`None` = none). It indexes the
    /// popup's laid-out row list, so slot 0 is the leading unbind row
    /// whenever a variable is bound. Cleared whenever the popup closes.
    pub property_color_variable_picker_hover: Option<usize>,
    /// Whether the image-fill editor popover is open.
    pub image_fill_popover_open: bool,
    /// Image-fill node currently in Figma-style crop editing mode.
    /// `None` keeps ordinary canvas drags moving the node itself.
    pub image_crop_editing: Option<NodeId>,
    /// Text font-family picker select state.
    pub font_picker: jian_widgets::components::select::SelectState,
    pub font_picker_purpose: Option<FontPickerPurpose>,
    /// Whether the cursor is over the picker's bottom "Import font…"
    /// (`ImportAction`) row — drives its hover wash, like `font_picker.hover`
    /// does for entry rows. The host tracks it on cursor-move when the picker
    /// is open; reset whenever `font_picker.hover` is (close / search edit).
    pub font_picker_import_hover: bool,
    /// Live type-ahead filter for the font-family picker (TS
    /// FontPicker search input).
    pub font_picker_search: String,
    /// Host-enumerated system font families (sorted, deduped against
    /// the bundled set). Empty until a host enumerates; the picker
    /// then falls back to the TS `FALLBACK_SYSTEM_FONTS` list.
    pub system_font_families: std::sync::Arc<Vec<String>>,
    /// App-shipped font families that the active renderer actually
    /// registered. Hosts without bundled font blobs leave this empty.
    pub bundled_font_families: std::sync::Arc<Vec<String>>,
    /// User-imported font families (from the host `FontStore` /
    /// `jian-skia` registry). Threaded in by the host exactly like
    /// `system_font_families`; the picker paints these first, above
    /// the bundled + system groups. The host refreshes this whenever
    /// the imported-font generation changes (import / remove), so no
    /// separate "loaded" flag is needed.
    pub imported_font_families: std::sync::Arc<Vec<String>>,
    /// Whether this host can import / remove user fonts. Desktop and the
    /// CanvasKit web host drain these requests; unsupported hosts leave it
    /// `false` so the picker omits dead import/remove controls.
    pub font_import_supported: bool,
    /// Whether this host can export a whole frame set in one action —
    /// it needs a directory picker plus the offscreen raster exporter.
    /// Desktop sets it; hosts that leave it `false` omit the File-menu
    /// row entirely rather than paint a dead one.
    pub batch_frame_export_supported: bool,
    /// Whether a host already ran font enumeration (so an empty list
    /// is "machine has none" rather than "not loaded yet").
    pub system_fonts_loaded: bool,
    /// Missing-font data shared by the one-shot prompt and Settings Fonts tab.
    /// `None` means no missing families have been detected.
    pub missing_fonts_prompt: Option<crate::missing_fonts::MissingFontsPrompt>,
    /// Whether the one-shot modal is visible. Dismissal keeps prompt data so
    /// the Settings Fonts tab can continue to expose unresolved rows.
    pub missing_fonts_modal_open: bool,
    /// Vertical scroll offset of the one-shot missing-font rows. The header
    /// and dismiss action stay fixed while long family lists scroll.
    pub missing_fonts_scroll: jian_core::scroll::ScrollState,
    /// Detection was requested before system-font enumeration completed.
    pub missing_fonts_pending_detect: bool,
    /// Whether the deferred detection may open the one-shot modal when font
    /// enumeration completes. History navigation sets this to `false` so an
    /// undo/redo refresh cannot resurrect a dismissed prompt.
    pub missing_fonts_pending_open_modal: bool,
    /// Row whose choose-file action is waiting for a platform import drain.
    pub missing_fonts_import_row: Option<usize>,
    /// Hovered missing-fonts control (modal button / settings row) —
    /// cursor-move updates it, paint tints from it.
    pub missing_fonts_hover: Option<crate::missing_fonts::MissingFontsHover>,
    /// Raised by `PropertyPanelAction::ImportFont` — the desktop host
    /// drains it to open a native font-file dialog + `FontStore::import`.
    pub pending_font_import: bool,
    /// Raised by `PropertyPanelAction::RemoveImportedFont` — carries the
    /// resolved family; the desktop host drains it to `FontStore::remove`.
    pub pending_font_remove: Option<String>,
    /// Image-node section Search / Generate popover state.
    pub image_panel: crate::image_panel_state::ImagePanelState,
    /// Whether the typography font-weight dropdown is open.
    pub font_weight_picker_open: bool,
    /// Index (into `FontWeightChoice::ALL`) of the weight-dropdown row
    /// under the cursor while it's open — drives the hover wash.
    pub font_weight_picker_hover: Option<usize>,
    /// Active-theme axis whose value picker is open; `None` = closed.
    pub axis_dropdown_open: Option<String>,
    /// Whether the Variables right-rail panel is explicitly open.
    pub variables_panel_open: bool,
    pub variables_preset_menu_open: bool,
    /// Which row inside the open theme-preset dropdown the cursor is over —
    /// drives the per-row hover wash.
    pub variables_preset_menu_hover: Option<crate::variables_panel_state::PresetMenuButton>,
    pub variables_add_menu_open: bool,
    /// Whether the preset dropdown's save-as-name input is showing
    /// (TS `showPresetNameInput`). This legacy preset-name draft still
    /// lives in `UiDraftState::property_input_draft`; variables panel
    /// row/header edits use the text-input states below.
    pub variables_preset_name_focus: bool,
    /// Pending `.optheme` import / export the desktop host must run.
    pub pending_theme_preset_io: Option<ThemePresetIo>,
    /// Theme axis currently shown in the floating variables panel.
    /// Separate from `ui.variables.active_theme`, which stores the
    /// concrete value selected for each axis.
    pub variables_current_axis: Option<String>,
    /// Theme-axis tab whose rename/delete menu is open.
    pub variables_theme_menu_axis: Option<String>,
    /// Variant column whose rename/delete menu is open.
    pub variables_variant_menu_value: Option<String>,
    /// Theme-axis name currently being edited in the VariablesPanel.
    pub variables_theme_rename_axis: Option<String>,
    /// Variant column value currently being edited in the VariablesPanel.
    pub variables_variant_rename_value: Option<String>,
    /// Shared text state for VariablesPanel theme-axis / variant header renames.
    pub variables_header_input: jian_core::text_input::TextInputState,
    /// Which variables-panel target the cursor is over (row / axis chip
    /// / dropdown item) — drives the `theme.button_hover` wash.
    pub variables_panel_hover: Option<crate::variables_panel_state::VariablesPanelButton>,
    /// Editor focus for a non-color variable row (Number / String).
    pub variable_row_focus: Option<VariableRowFocus>,
    /// Text state for focused VariablesPanel row/value cells.
    pub variable_row_input: jian_core::text_input::TextInputState,
    /// Live search filter for the variables panel rows (TS
    /// `variables-panel.tsx` search box). Case-insensitive substring
    /// match on the variable name; transient, never serialized.
    pub variables_search: String,
    /// Whether the variables-panel search box owns the keyboard.
    pub variables_search_focus: bool,
    /// Vertical scroll offset (px) of the variables row list.
    pub variables_scroll: jian_core::scroll::ScrollState,
    /// Variable row whose `⋯` overflow menu (Rename / Delete) is open.
    /// Indexes the UNFILTERED `doc.variables` order.
    pub variables_row_menu: Option<usize>,
    /// User-resized panel size; `None` = the 820x480 default. Mirrors
    /// TS's transient React state (resets each session, not persisted).
    pub variables_panel_size: Option<(f32, f32)>,
    /// Editor focus on an effect-parameter value (Effects section).
    /// Shares `UiDraftState.property_input` with property-panel fields.
    pub effect_param_focus: Option<EffectParamFocus>,

    /// Legacy in-flight IME composition cache. The native host clears
    /// this defensively but no longer paints a separate preedit bubble.
    pub ime_preedit: Option<crate::ime_state::ImePreedit>,

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
    /// Last LayerPanel click target + ms; 400 ms re-press → rename.
    pub last_layer_click: Option<(LayerContextTarget, u64)>,
    /// Deepest canvas hit + ms for the first half of a possible
    /// double-click. A same-hit re-press within 400 ms drills exactly
    /// one hierarchy level (or enters inline edit when no child level
    /// remains on a Text node).
    pub last_canvas_click: Option<(NodeId, u64)>,
    /// Last VariablesPanel name-cell click + ms; 400 ms same-row
    /// re-press promotes to variable rename.
    pub last_variable_name_click: Option<(usize, u64)>,
    /// Smart-guide lines to paint during the current node drag —
    /// computed each `apply_cursor_move` by `align_guides`, cleared on
    /// drag release. View-only transient state: never serialized,
    /// never part of the undo snapshot.
    pub active_guides: Vec<crate::align_guides::AlignmentGuide>,
    /// Which pencil-cursor silhouette the generation overlay draws.
    /// User-selectable in Settings > System; persisted across launches.
    pub pencil_cursor_style: PencilCursorStyle,
    /// Ghost of the blank starter frame `(x, y, w, h)` in doc px. Set when a
    /// design prompt clears the pristine starter; the canvas keeps painting
    /// it until the generated design's sized root lands (or the turn ends),
    /// so sending a prompt never flashes an empty canvas. Transient chrome
    /// state — never persisted.
    pub starter_ghost: Option<[f32; 4]>,
    /// Drop-target preview painted during canvas node dragging.
    /// View-only transient state: never serialized, never part of
    /// the undo snapshot.
    pub canvas_drop_indicator: Option<CanvasDropIndicator>,

    // --- Auto-update ------------------------------------------------
    /// Latest result of the desktop host's background update probe.
    /// Transient: never serialized, rebuilt each launch.
    pub update_status: UpdateStatus,

    // --- In-app Git -------------------------------------------------
    /// Floating Git panel snapshot — filled by the desktop host from
    /// its `GitSession`. Transient: never serialized.
    pub git_panel: GitPanelState,

    // --- Design-MD panel --------------------------------------------
    /// Floating Design-MD panel — open flag, hover target, position,
    /// expanded-section bitmask, scroll, and the queued host request.
    pub design_md_panel: DesignMdPanelState,

    // --- Prompt Center ----------------------------------------------
    /// Floating prompt catalogue, search, save form, and custom entries.
    pub prompt_center: PromptCenterState,
    /// Floating Scene Template Center panel.
    pub scene_template_center: SceneTemplateCenterState,

    // --- Component browser ------------------------------------------
    /// Whether the floating Component-Browser panel is shown.
    pub component_browser_open: bool,
    /// Current hierarchy focus under the cursor (Select tool, no
    /// drag). Canvas paint outlines the focus solid and all of its
    /// direct visible children dashed.
    pub canvas_hover_node: Option<NodeId>,
    /// Sibling scope entered by a one-level canvas double-click. While
    /// set, the scope's direct children are the current single-click
    /// targets and their children are the next drill candidates.
    /// Escape and selecting outside the scope exit it. Transient:
    /// never serialized.
    pub entered_container: Option<NodeId>,
    /// Top-left corner of the Component-Browser panel in logical px;
    /// `None` until first opened — the host then centres it.
    pub component_browser_pos: Option<(f32, f32)>,
    /// Live search filter — names + tags substring-match against this.
    pub component_browser_search: String,
    /// Active category pill (`None` = all categories).
    pub component_browser_category: Option<crate::uikit::ComponentCategory>,
    /// Which component-browser target the cursor is over (close /
    /// category pill / card) — drives the `theme.button_hover` wash.
    pub component_browser_hover: Option<crate::component_browser_state::ComponentBrowserButton>,
    /// Active kit filter (`None` = every loaded kit). Mirrors the TS
    /// `uikit-store.ts` `activeKitId`.
    pub component_browser_kit_id: Option<String>,
    /// Whether the kit-filter dropdown popover is open (the TS panel
    /// uses a native `<select>`; the Rust panel paints a popover).
    pub component_browser_kit_picker_open: bool,
    /// Imported-kit id awaiting delete confirmation — the TS panel's
    /// `confirmDeleteKitId` (Trash press arms it; Delete / Cancel
    /// resolve it). Transient: never serialized.
    pub component_browser_confirm_delete_kit: Option<String>,
    /// A queued kit Import / Export request — set by a header-button
    /// press, drained by the desktop host (which owns the native file
    /// dialogs). Transient: never serialized.
    pub component_browser_kit_request: Option<crate::uikit_io::KitIoRequest>,
    /// Persistence-dirty flag: raised by `import_kit` / `remove_kit`,
    /// drained by the desktop host into `uikits.json` (the TS
    /// `uikit-store.persist()` counterpart).
    pub ui_kits_changed: bool,
    /// A queued component-instantiate request — `(kit_id, comp_id)`,
    /// set by a card click, drained by the desktop host so it can
    /// run the instantiate against the viewport's centre.
    pub component_browser_pending_insert: Option<(String, String)>,
}
