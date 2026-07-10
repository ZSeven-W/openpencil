//! Widget facade for shell-core (Step 1b §1.4 — widget logic lives here so
//! both shell-native and shell-web reuse it; only RenderBackend / DOM event
//! mapping / accesskit DOM mirror are platform-owned).
//!
//! Two layers in one module:
//!
//! - **Primitives** (Phase B1+B2): the remaining OP-local building blocks
//!   `TreeWidget` / `PropertyRow`.
//! - **Compositions** (Step 2): `LayerPanel` / `PropertyPanel` /
//!   `Toolbar` — view models built from an `op_editor_core::EditorState`
//!   that compose the primitives into the actual editor UI surface.
//!   These were briefly housed in a `chrome/` submodule; the name
//!   collided with the higher-level "OP chrome = openpencil-shell"
//!   architectural term, so they live alongside the primitives now
//!   (every entry here `impl Widget`, primitives + compositions
//!   alike).

use crate::{Point2D, Rect, RenderBackend};

/// Minimum width (in CSS / physical px) below which the editor-UI
/// host paints the Toolbar only and skips the LayerPanel /
/// CanvasViewport / PropertyPanel rails. Single canonical
/// definition consumed by both `openpencil-shell-web::WidgetHost`
/// and `openpencil-shell-native::WidgetHostNative` so they stay in
/// lock-step (codex Step 3 R1 BLOCK fix — was duplicated as
/// `const MIN_RAIL_WIDTH` in each host).
pub const MIN_RAIL_WIDTH: f32 = 80.0;

// Phase B primitives.
pub(crate) mod button;
pub mod prop_row;
pub mod tree;

// Step 2 compositions (built on top of the primitives, driven by
// `op_editor_core::EditorState`).
pub(crate) mod font_picker_cache;
pub mod layer_context_menu;
pub mod layer_panel;
pub(crate) mod layer_panel_cache;
mod layer_panel_hit;
mod layer_panel_paint;
#[cfg(test)]
mod layer_panel_tests;
mod layer_panel_walkers;
pub mod path_anchor_context_menu;
pub mod property_panel;
pub mod property_panel_action;
pub mod property_panel_code;
pub mod property_panel_effects;
pub mod property_panel_export;
pub mod property_panel_fill;
mod property_panel_fill_body;
mod property_panel_fill_image_body;
mod property_panel_fill_picker;
#[cfg(test)]
mod property_panel_fill_tests;
pub mod property_panel_flex;
pub mod property_panel_icon;
#[cfg(test)]
mod property_panel_icon_tests;
pub mod property_panel_image_assets;
#[cfg(test)]
mod property_panel_image_assets_tests;
pub mod property_panel_image_fill;
#[cfg(test)]
mod property_panel_image_fill_tests;
pub mod property_panel_image_node;
pub mod property_panel_image_popovers;
mod property_panel_image_preview;
pub mod property_panel_input_layout;
pub mod property_panel_inputs;
#[cfg(test)]
mod property_panel_instance_tests;
pub mod property_panel_layer;
pub mod property_panel_layout;
#[cfg(test)]
mod property_panel_multi_select_tests;
mod property_panel_overlay_hit;
#[cfg(test)]
mod property_panel_press_tests;
pub mod property_panel_sections;
pub mod property_panel_snapshot;
pub mod property_panel_stroke;
#[cfg(test)]
mod property_panel_stroke_tests;
#[cfg(test)]
mod property_panel_tab_interact_tests;
#[cfg(test)]
mod property_panel_test_support;
#[cfg(test)]
mod property_panel_tests;
pub mod property_panel_text;
pub(crate) mod property_panel_text_input;
#[cfg(test)]
mod property_panel_text_tests;
pub mod property_panel_typography;
pub mod property_panel_visibility;
#[cfg(test)]
mod property_panel_wash_tests;
pub mod property_panel_widget;
#[cfg(test)]
mod property_panel_widget_tests;
pub mod text_input;
pub(crate) mod text_input_backend;
mod text_selection;
pub mod toolbar;

// Step 3 — center canvas that renders document nodes as actual
// visual primitives (frame fills, rect strokes, text strings).
mod canvas_agent_cursor;
#[cfg(test)]
mod canvas_agent_cursor_tests;
mod canvas_frame_labels;
pub mod canvas_layout_transition;
pub mod canvas_overlay_transform;
mod canvas_path_overlay;
mod canvas_selection_overlay;
pub mod canvas_text_edit;
pub mod canvas_viewport;
mod canvas_viewport_grid;
// `pub` for the remote-image miss-queue API (`take_remote_image_requests`
// / `store_remote_image_bytes` / …) the desktop host drains per frame;
// the paint entry point itself stays `pub(super)`.
pub mod canvas_viewport_image;
pub mod canvas_viewport_overlay;
pub mod canvas_viewport_paint;
#[cfg(test)]
mod canvas_viewport_paint_pop_tests;
mod canvas_viewport_text;
mod canvas_viewport_widget;

// Phase 6 — shell-core-side `theme()` / `t()` derivations over
// `op_editor_core::EditorUiState` for widgets ported off `Document`.
pub mod editor_state_ext;

// Step 4 — icon glyph drawer for editor chrome (lucide-flavored line art).
pub mod icon_catalog;
pub mod icons;
#[cfg(test)]
mod icons_tests;
// Lucide d-string data — extracted as a sibling so `icons.rs` stays
// under the 800-line cap as more first-party glyphs are added.
mod icons_data;

// Step 5 — brand logos for agent provider cards (filled SVG paths,
// not lucide stroke art). Sourced verbatim from `apps/web/src/
// components/icons/*-logo.tsx`.
pub mod brand_icons;

// Step 4 — extra editor-chrome widgets (TS app parity).
pub mod agent_settings_acp;
mod agent_settings_acp_helpers;
pub mod agent_settings_builtin;
mod agent_settings_builtin_layout;
mod agent_settings_builtin_parts;
#[cfg(test)]
mod agent_settings_builtin_tests;
mod agent_settings_caret;
#[cfg(test)]
mod agent_settings_compact_action_tests;
#[cfg(test)]
mod agent_settings_connect_tests;
#[cfg(test)]
mod agent_settings_form_action_tests;
mod agent_settings_form_actions;
mod agent_settings_header_action;
pub mod agent_settings_i18n;
pub mod agent_settings_images;
mod agent_settings_images_parts;
#[cfg(test)]
mod agent_settings_images_profile_tests;
pub mod agent_settings_mcp;
pub mod agent_settings_panel;
mod agent_settings_panel_card;
mod agent_settings_panel_geometry;
#[cfg(test)]
mod agent_settings_panel_tests;
mod agent_settings_switch;
#[cfg(test)]
mod agent_settings_switch_style_tests;
pub mod agent_settings_system;
mod ai_chat_checklist;
mod ai_chat_hit;
pub(crate) mod ai_chat_input_text;
pub mod ai_chat_model_picker;
#[cfg(test)]
mod ai_chat_model_picker_tests;
pub mod ai_chat_panel;
pub mod ai_chat_panel_controls;
mod ai_chat_panel_footer;
pub(crate) mod ai_chat_panel_header;
mod ai_chat_panel_hit;
pub mod ai_chat_panel_paint;
pub mod ai_chat_transcript;
#[cfg(test)]
mod ai_chat_transcript_apply_tests;
pub(crate) mod ai_chat_transcript_cache;
pub(crate) mod ai_chat_transcript_completion;
pub(crate) mod ai_chat_transcript_design;
pub(crate) mod ai_chat_transcript_hit;
pub(crate) mod ai_chat_transcript_paint_parts;
pub(crate) mod ai_chat_transcript_selection;
pub(crate) mod ai_chat_transcript_steps;
pub(crate) mod ai_chat_transcript_text;
pub(crate) mod ai_chat_transcript_tools;
pub mod align_toolbar;
pub mod color_picker;
mod component_browser_kits;
pub mod component_browser_panel;
pub(crate) mod design_md_line_cache;
pub mod design_md_markdown;
pub mod design_md_panel;
#[cfg(test)]
mod design_md_panel_tests;
pub mod export_dialog;
pub mod figma_import;
pub mod figma_import_progress;
pub mod file_drop_overlay;
pub mod file_menu;
pub mod git_panel;
mod git_panel_clone;
mod git_panel_diff;
mod git_panel_empty;
mod git_panel_hit;
mod git_panel_menus;
mod git_panel_ready;
mod git_panel_remotes;
mod git_panel_resolve;
mod git_panel_ssh_keys;
mod git_panel_status;
#[cfg(test)]
mod git_panel_tests;
mod git_panel_text;
mod git_panel_tracked_picker;
#[cfg(test)]
mod git_panel_tracked_picker_tests;
pub mod icon_picker_panel;
pub mod ime_preedit_overlay;
pub mod locale_picker;
mod property_panel_color_variables;
pub mod shape_picker;
pub mod status_bar;
pub mod top_bar;
mod top_bar_paint;
#[cfg(test)]
mod top_bar_tests;
pub mod variables_panel;
mod variables_preset_menu;

pub use prop_row::PropertyRow;
pub use text_input::TextInputWidget;
pub use tree::{TreeItem, TreeWidget};

pub use layer_panel::{LayerItem, LayerPanel};
pub use property_panel::{EffectAddMenuHit, FontWeightChoice, PropertyPanel, PropertyPanelAction};
pub use toolbar::Toolbar;

pub use canvas_layout_transition::{CanvasLayoutTransition, CANVAS_LAYOUT_TRANSITION_MS};
pub use canvas_viewport::{
    arc_handle_positions, path_handle_positions, rotate_point, rotation_corner_at_point,
    selection_handle_at_point, ArcHandle, CanvasNodeDragOverlay, CanvasViewport, SelectionHandle,
};
pub use canvas_viewport_paint::paint_scene_page;
pub use canvas_viewport_widget::widget_text_inset_left;

pub use icons::{draw_icon, draw_icon_catalog_entry, draw_icon_data, Icon, IconPathData};

pub use ai_chat_hit::{AIChatHit, ChatCursorProbe, ChatResizeEdge};
pub use ai_chat_panel::{
    AIChatPlaceholder, AI_CHAT_COLLAPSED_HEIGHT, AI_CHAT_COLLAPSED_WIDTH, AI_CHAT_HEIGHT,
    AI_CHAT_MAX_RATIO, AI_CHAT_MIN_HEIGHT, AI_CHAT_MIN_WIDTH, AI_CHAT_WIDTH,
};
pub use ai_chat_transcript_design::parse_design_json_nodes;
pub use align_toolbar::{AlignToolbar, AlignToolbarHit, ALIGN_TOOLBAR_HEIGHT, ALIGN_TOOLBAR_WIDTH};
pub use component_browser_panel::{
    ComponentBrowserHit, ComponentBrowserPanel, COMPONENT_BROWSER_PANEL_H,
    COMPONENT_BROWSER_PANEL_W,
};
pub use design_md_panel::{DesignMdHit, DesignMdPanel, DESIGN_MD_PANEL_H, DESIGN_MD_PANEL_W};
pub use export_dialog::{ExportDialog, ExportDialogHit, ExportFormat};
pub use git_panel::{GitPanel, GitPanelHit, GIT_PANEL_INSET, GIT_PANEL_WIDTH};
pub use icon_picker_panel::{
    IconPickerHit, IconPickerPanel, ICONIFY_LOAD_MORE_LIMIT, ICON_PICKER_PANEL_H,
    ICON_PICKER_PANEL_W,
};
pub use locale_picker::{LocalePicker, LOCALE_PICKER_WIDTH};
pub use shape_picker::{ShapeChoice, ShapePicker, SHAPE_PICKER_WIDTH};
pub use status_bar::{StatusBar, STATUS_BAR_HEIGHT, STATUS_BAR_WIDTH};
pub use top_bar::{TopBar, TopBarHit, WindowControl, TOP_BAR_HEIGHT};
// Re-export panel/toolbar width constants + hit enums so the host
// can size them consistently and route hits.
pub use layer_panel::{DropPosition, DropTarget, LayerPanelHit, LAYER_PANEL_WIDTH};
pub use property_panel::PROPERTY_PANEL_WIDTH;
pub use toolbar::{ToolbarAction, ToolbarHit, TOOLBAR_WIDTH};
pub use variables_preset_menu::{PresetMenuHit, ThemePresetMenu};

/// Stable identifier assigned by the widget host. Used by accesskit
/// (`accesskit::NodeId(WidgetId.0)`), the DOM mirror, and event routing.
///
/// `WidgetId(0)` is reserved for the root host node — see
/// [`ROOT_WIDGET_ID`]. Use [`WidgetId::new`] to construct non-root ids
/// with a debug-time check; the tuple constructor stays public so
/// `const`-context callers (e.g. test fixtures) and pattern matches keep
/// working.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetId(pub u64);

/// The reserved root-host id. Phase C tree routing skips this id when
/// dispatching to widgets (the root is the implicit host frame). Made a
/// named constant so the convention is compiler-visible — see codex
/// Phase B1 review NIT-7.
pub const ROOT_WIDGET_ID: WidgetId = WidgetId(0);

impl WidgetId {
    /// Constructs a non-root `WidgetId`. In debug builds, panics if the
    /// caller tries to allocate id 0 (reserved for [`ROOT_WIDGET_ID`]);
    /// in release the value is accepted as-is so production paths are
    /// not punished for a host bug. Phase C tree routing should use this
    /// constructor for any id derived from widget allocation.
    #[inline]
    pub const fn new(id: u64) -> Self {
        debug_assert!(
            id != 0,
            "WidgetId::new(0) — id 0 is reserved for ROOT_WIDGET_ID"
        );
        Self(id)
    }
}

/// Result of a `Widget::layout` call — the absolute rectangle the widget
/// occupies in its parent frame. Phase B keeps this minimal (no taffy yet);
/// Phase C / D may extend with taffy-style intrinsic sizing once the four
/// inspector widgets shake out their layout needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutBox {
    pub rect: Rect,
}

/// Frame-scoped paint context. Holds the active `RenderBackend` so widgets
/// can issue draw calls; the `&mut dyn` indirection lets shell-native +
/// shell-web share the same widget code without monomorphising over the
/// concrete backend type.
pub struct PaintCx<'a> {
    pub backend: &'a mut dyn RenderBackend,
}

/// Layout-time context. Phase B passes the available width + the host's
/// dpi scale; later phases may add font metrics / theme tokens.
#[derive(Debug, Clone, Copy)]
pub struct LayoutCx {
    pub available_width: f32,
    pub dpi: f32,
}

/// The widget facade. Step 1b widgets are static — `paint` takes `&self`
/// and only sees the host-provided rect; mutable per-widget state lives in
/// dedicated `*State` structs (see B2). Future phases may add a `&mut self`
/// `event` method for input handling; Phase C wires DOM events in
/// shell-web and the trait surface gets extended in lockstep.
pub trait Widget {
    /// Stable identifier (assigned by the host).
    fn id(&self) -> WidgetId;

    /// Compute the widget's layout in the given context. Pure — no
    /// rendering side effects.
    fn layout(&self, cx: &LayoutCx) -> LayoutBox;

    /// Paint the widget into `rect` via `cx.backend`. The host is
    /// responsible for placing the rect; the widget only paints relative
    /// to it.
    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect);

    /// Generate the accesskit Node for this widget. Used by both the
    /// shell-native accesskit_winit adapter (Step 1a) and the shell-web
    /// DOM mirror (Phase D). The host assigns NodeIds from `WidgetId`.
    fn access_node(&self) -> accesskit::Node;
}

/// Convenience constructor used by tests + B2 widget impls.
pub fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect {
        origin: Point2D::new(x, y),
        size: Point2D::new(width, height),
    }
}
