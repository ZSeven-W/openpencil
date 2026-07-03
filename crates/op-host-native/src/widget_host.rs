//! Step 4 native widget glue — the only file in shell-native allowed
//! to call into `op_editor_ui::widgets`. Mirrors the
//! shell-web `widget_host.rs` so the editor-UI composition is
//! cross-platform: same widget code, same paint output.
//!
//! Layout matches `apps/web/src/components/editor/editor-layout.tsx`
//! — TopBar / LayerPanel / Toolbar (vertical floating) / Canvas
//! (fills) / RightPanel (only with selection) / StatusBar (floating
//! bottom-right) / AIChatPlaceholder (floating bottom-center).
//!
//! ### Mobile (iOS / Android) — Step 1f path
//!
//! Spec §11 — shell-native is desktop-gated (`backend` /
//! `widget_host` modules cfg-gated to `macos | linux | windows`).
//! Mobile widget rendering lands in Step 1f via `context::EaglProvider`
//! / `context::AndroidEglProvider`. Per the 2026-05-10 directive
//! (Android and iOS need no IPC / local CLI — only a custom provider):
//! mobile rendering is a custom-provider plugin point on the
//! `GlContextProvider` trait, not a separate IPC / CLI pipeline.
//!
//! ### Module layout
//!
//! This file is the public spine. Implementation methods are split
//! across sibling submodules (per the 800-line-per-file ceiling):
//! - `NativeFrameBackend` (`RenderBackend` impl) — moved to `crate::backend`
//! - [`helpers`] — hex parsing + resize-bounds math + constants
//! - [`geometry`] — canvas region / cursor hint / picker rect helpers
//! - [`input`] — cursor-move / release / panel-resize input handlers
//! - [`scroll`] — wheel + trackpad-pan (zoom / canvas + diff scroll)
//! - [`press`] — `apply_press` + new-node spawn (largest method)
//! - [`git_press`] — Git-panel press dispatch
//! - [`paint`] — full editor-UI composition paint pass

use op_editor_ui::widgets::SelectionHandle;
use op_editor_ui::{Rect, Theme};

mod a11y;
#[cfg(test)]
mod agent_settings_acp_tests;
#[cfg(test)]
mod agent_settings_compact_press_tests;
mod agent_settings_draft_dispatch;
#[cfg(test)]
mod agent_settings_form_press_tests;
#[cfg(test)]
mod agent_settings_image_gen_tests;
#[cfg(test)]
mod agent_settings_tests;
mod ai_chat_geometry;
mod arc_drag;
mod blur_inputs;
#[cfg(test)]
mod blur_inputs_tests;
mod canvas_select_drag;
#[cfg(test)]
mod canvas_select_drag_tests;
mod chat_design_apply;
#[cfg(test)]
mod chat_design_apply_tests;
mod chat_design_hover;
#[cfg(test)]
mod chat_design_hover_tests;
mod chat_model_picker_caret;
#[cfg(test)]
mod chat_model_picker_caret_tests;
#[cfg(test)]
mod chat_send_tests;
mod click;
mod color_picker_press;
mod component_browser_press;
#[cfg(test)]
mod deferred_press_tests;
mod design_md_press;
#[cfg(test)]
mod design_md_press_tests;
#[cfg(test)]
mod figma_import_tests;
mod font_picker_dispatch;
mod geometry;
mod geometry_settings_hover;
#[cfg(test)]
mod git_panel_placement_tests;
mod git_press;
mod helpers;
mod history_guard;
mod icon_picker_press;
#[cfg(test)]
mod icon_picker_press_tests;
mod image_panel_dispatch;
mod ime;
mod input;
#[cfg(test)]
mod input_clipboard_tests;
#[cfg(test)]
mod input_drag_tests;
#[cfg(test)]
mod input_tests;
#[cfg(test)]
mod instance_panel_tests;
mod keyboard;
#[cfg(test)]
mod overlay_cursor_tests;
mod overlay_rects;
mod paint;
#[cfg(test)]
mod panel_history_tests;
mod pen_press;
#[cfg(test)]
mod pen_press_tests;
mod press;
mod press_helpers;
mod property_dispatch;
mod property_layout_dispatch;
#[cfg(test)]
mod property_panel_press_tests;
mod property_popovers;
mod release_feedback;
mod scroll;
#[cfg(test)]
mod scroll_tests;
mod settings_caret;
#[cfg(test)]
mod settings_caret_tests;
mod settings_dispatch;
mod shape_picker_press;
#[cfg(test)]
mod shortcut_surface_tests;
mod shortcuts;
mod text_edit_press;
#[cfg(test)]
mod text_edit_press_tests;
#[cfg(test)]
mod theme_tests;
mod toolbar_actions;
mod toolbar_hover;
#[cfg(test)]
mod variables_panel_add_tests;
mod variables_panel_commit;
mod variables_panel_geometry;
mod variables_panel_press;
mod variables_panel_row_press;
#[cfg(test)]
mod variables_panel_tests;
#[cfg(test)]
mod variables_panel_ux_tests;
mod variables_preset_press;
mod viewport_fit;

/// Cursor affordance the host suggests for a given screen point — re-exported
/// from `jian-core` so widgets (via `cursor_at`) and hosts share one vocabulary.
/// The runner maps each variant to its native cursor (`CursorIcon` on desktop,
/// CSS `cursor:` string on web). Domain/canvas cursor decisions (active tool,
/// selection handles, resize gutters) stay host-side — see `cursor_for_handle`
/// and `geometry::cursor_hint`.
pub use jian_core::CursorHint;

/// Map a selection handle to its resize cursor. Stays host-side because it
/// depends on the OP `SelectionHandle` type, which is not a jian-atomic concern.
pub(in crate::widget_host) fn cursor_for_handle(h: SelectionHandle) -> CursorHint {
    match h {
        SelectionHandle::Left | SelectionHandle::Right => CursorHint::ResizeEw,
        SelectionHandle::Top | SelectionHandle::Bottom => CursorHint::ResizeNs,
        SelectionHandle::TopLeft | SelectionHandle::BottomRight => CursorHint::ResizeNwse,
        SelectionHandle::TopRight | SelectionHandle::BottomLeft => CursorHint::ResizeNesw,
    }
}

/// Native counterpart of shell-web's `WidgetHost`. Owns the
/// canonical-model `EditorState` as its single source of truth +
/// composes the editor UI per frame in the TS-equivalent layout.
pub struct WidgetHostNative {
    /// **The host's single source of truth.** All input handlers
    /// mutate this; paint + the input hit-test read the derived
    /// `layout_scene` rebuilt from it (see `refresh_layout_scene`).
    pub(in crate::widget_host) editor_state: op_editor_core::EditorState,
    /// Derived paint-only `LayoutScene` of `editor_state` — the
    /// layout-resolved render tree the `CanvasViewport` paints AND
    /// the host's canvas hit-test queries. Rebuilt lazily by
    /// `refresh_layout_scene()` whenever `editor_state_dirty` is set.
    pub(in crate::widget_host) layout_scene: op_editor_ui::layout_scene::LayoutScene,
    /// Skips the `layout_scene` rebuild when the document / active theme /
    /// active page are unchanged. `editor_state_dirty` fires on nearly every
    /// interaction (hover, scroll, selection, caret drafts, chat streaming),
    /// but most leave the scene inputs identical — this guards the rebuild.
    pub(in crate::widget_host) scene_cache: op_pen_loader::SceneBuildCache,
    /// Set whenever `editor_state` is mutated. Drives the lazy
    /// rebuild of `layout_scene` — `refresh_layout_scene()` rebuilds
    /// + clears the flag, so a sequence of mutations re-derives once.
    pub(in crate::widget_host) editor_state_dirty: bool,
    pub(in crate::widget_host) theme: Theme,
    /// Active canvas pan-drag state — left-button press → motion
    /// → release.
    pub(in crate::widget_host) drag: Option<DragState>,
    /// True while Space is held — transient pan mode (TS parity):
    /// canvas presses pan regardless of the active tool.
    pub(in crate::widget_host) space_pan: bool,
    /// Last cursor position the canvas-hover hit-test ran at —
    /// sub-3px moves skip the tree walk (cost guard).
    pub(in crate::widget_host) last_hover_probe: Option<(f32, f32)>,
    /// Active chat-panel drag state — present while the user
    /// drags the floating AI chat panel by its header. Holds the
    /// transient panel top-left position so paint can place the
    /// panel at the cursor instead of its anchor; on release the
    /// host computes the nearest corner via `ChatAnchor::nearest`
    /// and snaps.
    pub(in crate::widget_host) chat_drag: Option<ChatDragState>,
    /// Active chat-panel resize — mirrors the TS panel's invisible
    /// edge/corner handles.
    pub(in crate::widget_host) chat_resize: Option<ChatResizeState>,
    /// Active Design-MD panel drag — present while the user drags the
    /// floating panel by its header bar. The live top-left is written
    /// straight back into `editor_ui.design_md_panel_pos`.
    pub(in crate::widget_host) design_md_drag: Option<DesignMdDragState>,
    /// Active Component-Browser panel drag.
    pub(in crate::widget_host) component_browser_drag: Option<ComponentBrowserDragState>,
    /// Active Icon-picker panel drag.
    pub(in crate::widget_host) icon_picker_drag: Option<IconPickerDragState>,
    /// Active image-fill adjustment slider drag in the floating
    /// property popover.
    pub(in crate::widget_host) image_adjustment_drag: Option<op_editor_core::ImageAdjustmentField>,
    /// Active generated-code preview text selection drag.
    pub(in crate::widget_host) code_selection_drag: Option<CodeSelectionDragState>,
    /// Active chat input text selection drag.
    pub(in crate::widget_host) chat_input_selection_drag: Option<ChatInputSelectionDragState>,
    /// Active chat transcript text selection drag.
    pub(in crate::widget_host) chat_text_selection_drag: Option<ChatTextSelectionDragState>,
    /// Active inline canvas text-edit selection drag — press inside
    /// the edited Text node placed the caret; dragging extends the
    /// selection from the press offset.
    pub(in crate::widget_host) text_edit_selection_drag: Option<TextEditSelectionDragState>,
    /// Lazily-created measure-only Skia backend for text-edit
    /// hit-testing OUTSIDE the paint pass (press / drag / arrow-key
    /// line mapping). Same `measure_text_weighted` implementation the
    /// paint backend uses, so hit geometry matches painted glyphs.
    pub(in crate::widget_host) text_measure: Option<crate::NativeBackend>,
    /// Active panel-resize drag — set when the cursor is pressed
    /// within the resize gutter of LayerPanel's right edge or
    /// PropertyPanel's left edge.
    pub(in crate::widget_host) panel_resize: Option<PanelResize>,
    /// Active floating-VariablesPanel resize drag (right / bottom /
    /// corner edge, TS pointer-capture handles). The live size is
    /// written into `editor_ui.variables_panel_size`.
    pub(in crate::widget_host) variables_resize:
        Option<op_editor_ui::widgets::variables_panel::VariablesResizeEdge>,
    /// Active node-drag — set when the user presses on a node in
    /// the canvas with the Select tool. Tracks the document-space
    /// cursor anchor so each `apply_cursor_move` translates the
    /// selected node by the delta.
    pub(in crate::widget_host) node_drag: Option<NodeDragState>,
    /// Active handle-drag — set when the user pressed on one of
    /// the 8 selection handles. Carries the start screen anchor +
    /// the original bounds so each move computes a fresh
    /// `new_bounds = start_bounds + delta`.
    pub(in crate::widget_host) handle_drag: Option<HandleDragState>,
    /// Active rotation drag — set when the user pressed in the
    /// rotation ring just outside one of the four corners.
    pub(in crate::widget_host) rotate_drag: Option<RotateDragState>,
    /// Active shape-create drag — set when the user presses
    /// empty canvas with a shape tool selected.
    pub(in crate::widget_host) create_drag: Option<CreateDragState>,
    /// Active marquee rect-select drag — set when the user
    /// presses empty canvas with the Select tool. On release,
    /// every top-level node whose bounds intersect the marquee
    /// joins the selection (replaces if `additive == false`,
    /// toggles each if `additive == true`).
    pub(in crate::widget_host) marquee_drag: Option<MarqueeDragState>,
    /// Active layer drag-to-reorder gesture — set when the user
    /// presses a LayerPanel row with the press-y exceeding a
    /// small threshold during move. Carries the source NodeId and
    /// the live cursor y in panel-local space. Resolved on
    /// release into `Document::reorder_before` /
    /// `reorder_after` via `LayerPanel::drop_target_at`.
    pub(in crate::widget_host) layer_drag: Option<LayerDragState>,
    /// Active path-anchor drag — set when the user presses on a
    /// per-anchor handle of a selected Path node with the Pen tool.
    /// Each `apply_cursor_move` snaps the anchor to the current
    /// document-space cursor; release commits a history snapshot.
    pub(in crate::widget_host) path_anchor_drag: Option<PathAnchorDragState>,
    /// Active ellipse arc-handle drag — set when the user presses on
    /// a start / sweep / inner-radius handle of a selected Ellipse.
    /// Each move re-applies `SetEllipseArc`; release commits history.
    pub(in crate::widget_host) arc_handle_drag: Option<ArcHandleDragState>,
    /// Counter for minting fresh `NodeId`s for newly-created nodes.
    /// Bumped past the highest sample id so new + sample nodes
    /// never collide on the same key.
    pub(in crate::widget_host) next_node_id: u64,
    /// Host-supplied frame timestamp in milliseconds. Focused
    /// `TextInputState`s use this for caret blink. The
    /// inspector_window runner refreshes this once per
    /// `RedrawRequested` from a single `Instant` start anchor;
    /// any other host (mobile / browser) installs its own clock.
    pub(in crate::widget_host) now_ms: u64,
    /// Whether the shift key is currently held. Runners update
    /// this via `set_modifier_shift` on every modifier change.
    /// Drives shift+click multi-select in `apply_press`.
    pub(in crate::widget_host) shift_held: bool,
    /// Last viewport size seen by paint/press. Used by handlers
    /// that don't receive viewport dims (e.g. apply_cursor_move
    /// driving the color-picker drag).
    pub(in crate::widget_host) last_viewport_w: f32,
    pub(in crate::widget_host) last_viewport_h: f32,
    /// Live canvas Preview (Play) session — `Some` while
    /// `editor_state.editor_ui.preview_mode` is set. Owns the jian
    /// `Runtime` (host-local; `!Send`). Built on enter from the
    /// document JSON, dropped on exit so the document stays untouched.
    pub(in crate::widget_host) preview: Option<crate::preview::PreviewSession>,
    /// Live preview pointer-drag state: `true` between a canvas Down
    /// and its Up, so cursor moves dispatch as drags (slider knob)
    /// instead of hovers.
    pub(in crate::widget_host) preview_press_active: bool,
    /// Last preview pointer position in DOCUMENT space — the release
    /// dispatches its Up here (the OS reports release without coords).
    pub(in crate::widget_host) preview_last_doc: Option<(f32, f32)>,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct DragState {
    pub(in crate::widget_host) last_x: f32,
    pub(in crate::widget_host) last_y: f32,
}

/// Which panel edge is being dragged, plus the press anchor +
/// the panel width at press time. Live width is computed as
/// `start_width + (live_x - start_x) * sign`.
#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct PanelResize {
    pub(in crate::widget_host) kind: PanelResizeKind,
    pub(in crate::widget_host) start_x: f32,
    pub(in crate::widget_host) start_width: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelResizeKind {
    LayerRight,
    PropertyLeft,
}

/// Active node-drag — tracks the previous cursor position in
/// SCREEN coordinates. Each `apply_cursor_move` divides the
/// screen-space delta by the active zoom to get a doc-space
/// translation, which sidesteps canvas_region offset math (the
/// offset cancels for incremental deltas).
#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct NodeDragState {
    pub(in crate::widget_host) last_screen_x: f32,
    pub(in crate::widget_host) last_screen_y: f32,
    /// Press point in screen px — the drag-threshold anchor.
    pub(in crate::widget_host) press_screen_x: f32,
    pub(in crate::widget_host) press_screen_y: f32,
    /// Latches true once the cursor travels past `NODE_DRAG_THRESHOLD_PX`
    /// so a pure click with sub-pixel jitter never moves anything.
    pub(in crate::widget_host) moved: bool,
    /// Net cursor travel since the press in DOC px (`(cursor - press)
    /// / zoom`, refreshed each move). Flex-flow children never
    /// doc-translate during the drag, so the release commit adds this
    /// to their scene bounds to find the dropped position.
    pub(in crate::widget_host) total_dx: f64,
    pub(in crate::widget_host) total_dy: f64,
}

/// Active handle-drag — captures the press cursor anchor + the
/// node's starting bounds. Each `apply_cursor_move` computes the
/// new bounds from a screen-space delta inverted by zoom and
/// writes it via `Document::set_selected_bounds`.
#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct HandleDragState {
    pub(in crate::widget_host) handle: SelectionHandle,
    pub(in crate::widget_host) start_screen_x: f32,
    pub(in crate::widget_host) start_screen_y: f32,
    pub(in crate::widget_host) start_bounds: Rect,
}

/// Active rotation drag — `center_screen` is the screen-space
/// centre of the selected node's bounds at press time; the
/// rotation angle is `atan2(y - cy, x - cx)`. We snapshot the
/// initial cursor angle and the node's starting rotation so each
/// move computes `rotation = start_rotation + (cursor_angle -
/// start_cursor_angle)`.
#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct RotateDragState {
    pub(in crate::widget_host) center_screen_x: f32,
    pub(in crate::widget_host) center_screen_y: f32,
    pub(in crate::widget_host) start_cursor_angle: f32,
    pub(in crate::widget_host) start_rotation: f32,
}

/// Active shape-create drag — set when the user presses an empty
/// canvas point with a shape tool (Rect / Ellipse / Polygon /
/// Line / Pen / Frame / Text) selected. The new node is created
/// at press time and resized on each move; the document's
/// selection still points at it, so we don't need to carry the
/// id on the drag state itself.
#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct CreateDragState {
    pub(in crate::widget_host) start_doc_x: f32,
    pub(in crate::widget_host) start_doc_y: f32,
}

/// Active marquee rect-select drag. Endpoints are in SCREEN
/// coordinates so paint can draw the rect without re-deriving
/// the canvas→screen transform; release converts to doc space
/// once to ask the document which nodes overlap.
#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct MarqueeDragState {
    pub(in crate::widget_host) start_screen_x: f32,
    pub(in crate::widget_host) start_screen_y: f32,
    pub(in crate::widget_host) current_screen_x: f32,
    pub(in crate::widget_host) current_screen_y: f32,
    /// Whether shift was held at press time. Drives whether
    /// release REPLACES the selection or toggles each
    /// intersecting node into / out of the existing set.
    pub(in crate::widget_host) additive: bool,
}

/// Active LayerPanel drag-to-reorder gesture.
#[derive(Debug, Clone)]
pub(in crate::widget_host) struct LayerDragState {
    /// NodeId of the row the user pressed on — what gets moved on
    /// release.
    pub(in crate::widget_host) source: op_editor_core::NodeId,
    /// Cursor y at press time, panel-local. Used to suppress drag
    /// activation until the cursor has moved a few pixels (avoids
    /// promoting a regular click into a drag).
    pub(in crate::widget_host) start_y: f32,
    /// Live cursor x / y for the drop-target hit-test.
    pub(in crate::widget_host) current_x: f32,
    pub(in crate::widget_host) current_y: f32,
    /// Whether the cursor has moved past the activation threshold.
    /// False = still a candidate click; True = committed drag (paint
    /// the drop indicator).
    pub(in crate::widget_host) active: bool,
}

/// What a path-anchor drag is editing — the anchor body itself, or
/// one of its two bezier control handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::widget_host) enum AnchorDragTarget {
    /// The anchor point — drag moves the whole anchor.
    Anchor,
    /// A bezier control handle.
    Handle(op_editor_core::pen::PathHandleSide),
}

/// Path-anchor drag — tracks which anchor (or which of its bezier
/// handles) of which Path node is being dragged (Pen or Select
/// tool). Move dispatches apply the TS-style cumulative cursor delta
/// (`movePathControl`, `path-editing.ts:66-114` — the grab offset is
/// preserved, no snap); release commits a history snapshot ONLY when
/// it actually moved (codex CONCERN: a press-release without motion
/// pushed a no-op snapshot that polluted the undo stack).
#[derive(Debug, Clone)]
pub(in crate::widget_host) struct PathAnchorDragState {
    pub(in crate::widget_host) node_id: op_editor_core::NodeId,
    pub(in crate::widget_host) anchor_index: usize,
    /// Whether the anchor body or a handle is being dragged.
    pub(in crate::widget_host) target: AnchorDragTarget,
    /// The dragged anchor's absolute doc position, fixed at press —
    /// handle drags compute their offset relative to it.
    pub(in crate::widget_host) anchor_doc: op_editor_ui::Point2D,
    /// Press cursor doc point (un-rotated into the node's local frame
    /// for rotated paths) — base of the cumulative drag delta and the
    /// did-it-move gate.
    pub(in crate::widget_host) start_doc: op_editor_ui::Point2D,
    /// The grabbed handle's offset at press. `Some` = an existing
    /// handle, edited with TS `movePathControl` semantics; `None` for
    /// the anchor body or a Pen-tool ghost mint (deliberate Rust
    /// superset — TS cannot grab an unset handle).
    pub(in crate::widget_host) grab_offset: Option<op_editor_ui::Point2D>,
    /// Shift held at press — a ghost-handle MINT with Shift produces
    /// independent (broken) handles instead of mirrored ones.
    pub(in crate::widget_host) shift: bool,
    /// Set to true on the first cursor-move that mutates the target.
    pub(in crate::widget_host) moved: bool,
    /// Snapshot captured at drag-start; pushed only if `moved`.
    pub(in crate::widget_host) pre_drag_snapshot: op_editor_core::EditorSnapshot,
}

/// Ellipse arc-handle drag — tracks which arc handle of which
/// Ellipse is being dragged. Move re-applies `SetEllipseArc`;
/// release commits a history snapshot only when the arc changed.
#[derive(Debug, Clone)]
pub(in crate::widget_host) struct ArcHandleDragState {
    pub(in crate::widget_host) node_id: op_editor_core::NodeId,
    pub(in crate::widget_host) handle: op_editor_ui::widgets::ArcHandle,
    /// Press cursor doc point — the move handler gates `moved` on
    /// real motion from here so a press-release pushes no undo entry.
    pub(in crate::widget_host) start_doc: op_editor_ui::Point2D,
    /// Set true on the first cursor-move that actually moves the arc.
    pub(in crate::widget_host) moved: bool,
    /// Snapshot captured at drag-start; pushed only if `moved`.
    pub(in crate::widget_host) pre_drag_snapshot: op_editor_core::EditorSnapshot,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct CodeSelectionDragState {
    pub(in crate::widget_host) anchor: usize,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct ChatInputSelectionDragState {
    pub(in crate::widget_host) anchor: usize,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct ChatTextSelectionDragState {
    pub(in crate::widget_host) message_index: usize,
    pub(in crate::widget_host) anchor: usize,
}

/// Inline canvas text-edit selection drag — `anchor` is the byte
/// offset placed by the press; cursor moves extend `anchor..focus`.
#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct TextEditSelectionDragState {
    pub(in crate::widget_host) anchor: usize,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct ChatDragState {
    /// Pointer offset within the panel rect when the drag began.
    /// Subtracting from the live cursor position gives the panel
    /// top-left, so the panel doesn't visually jump on press.
    pub(in crate::widget_host) grab_dx: f32,
    pub(in crate::widget_host) grab_dy: f32,
    /// Live panel top-left (logical px, viewport-relative).
    pub(in crate::widget_host) pos_x: f32,
    pub(in crate::widget_host) pos_y: f32,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct ChatResizeState {
    pub(in crate::widget_host) edge: op_editor_ui::widgets::ChatResizeEdge,
    pub(in crate::widget_host) start_x: f32,
    pub(in crate::widget_host) start_y: f32,
    pub(in crate::widget_host) start_rect: Rect,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct DesignMdDragState {
    /// Pointer offset within the panel rect when the drag began —
    /// subtracting from the live cursor gives the panel top-left.
    pub(in crate::widget_host) grab_dx: f32,
    pub(in crate::widget_host) grab_dy: f32,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct ComponentBrowserDragState {
    pub(in crate::widget_host) grab_dx: f32,
    pub(in crate::widget_host) grab_dy: f32,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct IconPickerDragState {
    pub(in crate::widget_host) grab_dx: f32,
    pub(in crate::widget_host) grab_dy: f32,
}

impl WidgetHostNative {
    pub fn new() -> Self {
        // A fresh launch opens with a single empty starter Frame —
        // see `EditorState::starter`.
        let editor_state = op_editor_core::EditorState::starter();
        // Seed the render scene once up front; subsequent frames
        // re-derive only when `editor_state_dirty` is set.
        let layout_scene = op_pen_loader::editor_state_to_layout_scene(&editor_state);
        Self {
            editor_state,
            layout_scene,
            scene_cache: op_pen_loader::SceneBuildCache::new(),
            editor_state_dirty: false,
            theme: Theme::dark(),
            drag: None,
            space_pan: false,
            last_hover_probe: None,
            chat_drag: None,
            chat_resize: None,
            design_md_drag: None,
            component_browser_drag: None,
            icon_picker_drag: None,
            image_adjustment_drag: None,
            code_selection_drag: None,
            chat_input_selection_drag: None,
            chat_text_selection_drag: None,
            text_edit_selection_drag: None,
            text_measure: None,
            panel_resize: None,
            variables_resize: None,
            node_drag: None,
            path_anchor_drag: None,
            arc_handle_drag: None,
            handle_drag: None,
            rotate_drag: None,
            create_drag: None,
            marquee_drag: None,
            layer_drag: None,
            next_node_id: 100,
            now_ms: 0,
            shift_held: false,
            last_viewport_w: 0.0,
            last_viewport_h: 0.0,
            preview: None,
            preview_press_active: false,
            preview_last_doc: None,
        }
    }

    /// Push the host's current shift-key state. Runners call this
    /// on every modifier-change event so `apply_press` can branch
    /// on shift+click semantics.
    pub fn set_modifier_shift(&mut self, held: bool) {
        self.shift_held = held;
    }

    /// Push the host's monotonic millisecond timestamp into the
    /// host. Drives caret blink + any future time-based
    /// animations via `jian_core::anim`. Also forwarded to the live
    /// preview runtime (caret blink in Preview mode).
    pub fn set_now_ms(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
        if let Some(preview) = self.preview.as_mut() {
            preview.set_now_ms(now_ms);
        }
    }

    /// Whether the canvas is currently in Preview (Play) mode with a
    /// live runtime.
    pub fn preview_active(&self) -> bool {
        self.preview.is_some() && self.editor_state.editor_ui.preview_mode
    }

    /// Enter Preview (Play) mode: flip the editor flag + build a live
    /// jian runtime from the current document (which is NOT mutated).
    /// Layout is solved per-root from each root frame's own authored
    /// size (mirroring the design canvas), so `canvas_size` no longer
    /// drives the flex solve — it is retained only for API
    /// compatibility; the visible viewport affects paint transform
    /// (pan / zoom / clip), not layout. On a build failure the editor
    /// stays in design mode and the error is recorded in
    /// `preview_warnings`. Returns `true` on success.
    pub fn enter_preview(&mut self, canvas_size: (f32, f32)) -> bool {
        if self.preview.is_some() {
            return true;
        }
        match crate::preview::PreviewSession::enter(
            &self.editor_state.doc,
            canvas_size,
            &self.editor_state.ui.variables.active_theme,
            self.editor_state.ui.active_page_index,
        ) {
            Ok(mut session) => {
                session.set_now_ms(self.now_ms);
                self.editor_state.editor_ui.enter_preview();
                self.editor_state.editor_ui.preview_warnings = session.warnings().to_vec();
                self.preview = Some(session);
                self.mark_dirty();
                true
            }
            Err(message) => {
                // Stay in design mode; surface the failure.
                self.editor_state.editor_ui.preview_mode = false;
                self.editor_state.editor_ui.preview_warnings = vec![format!("preview: {message}")];
                self.mark_dirty();
                false
            }
        }
    }

    /// Exit Preview mode: drop the runtime + clear the editor flag. The
    /// document is byte-identical to before entering (the runtime never
    /// touched it). Idempotent.
    pub fn exit_preview(&mut self) {
        self.preview = None;
        self.preview_press_active = false;
        self.preview_last_doc = None;
        self.editor_state.editor_ui.exit_preview();
        self.mark_dirty();
    }

    /// Toggle Preview mode. `canvas_size` is the logical canvas region
    /// (used only on enter). Returns the new state (`true` = in preview).
    pub fn toggle_preview(&mut self, canvas_size: (f32, f32)) -> bool {
        if self.preview_active() {
            self.exit_preview();
            false
        } else {
            self.enter_preview(canvas_size)
        }
    }

    /// Resize hook called from the desktop runner's `Resized` handler.
    /// Preview layout is now derived per-root from the document (not the
    /// canvas region), so resizing only changes the paint transform, not
    /// the flex solve — `PreviewSession::resize` is itself a no-op.
    /// Returns early when not in preview.
    pub fn preview_resize(&mut self, viewport_w: f32, viewport_h: f32) {
        if self.preview.is_none() {
            return;
        }
        let (_cx0, _cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        if let Some(preview) = self.preview.as_mut() {
            preview.resize((cw, ch));
        }
    }

    /// Route a printable character into the live preview runtime.
    /// Returns `true` when consumed by a focused widget. No-op (false)
    /// when not in preview.
    pub fn preview_dispatch_text(&mut self, text: &str) -> bool {
        let consumed = self.preview.as_mut().is_some_and(|p| p.dispatch_text(text));
        if consumed {
            self.mark_dirty();
        }
        consumed
    }

    /// Screen → document-space point when preview is active and the
    /// point is inside the canvas region; `None` otherwise.
    fn preview_doc_point(
        &self,
        screen_x: f32,
        screen_y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<op_editor_ui::Point2D> {
        self.preview.as_ref()?;
        let (cx0, cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        if screen_x < cx0 || screen_x > cx0 + cw || screen_y < cy0 || screen_y > cy0 + ch {
            return None;
        }
        let canvas_local = op_editor_ui::Point2D::new(screen_x - cx0, screen_y - cy0);
        Some(self.editor_state.viewport.to_document(canvas_local))
    }

    /// Route a screen-space press into the live preview runtime as a
    /// pointer Down; the matching Up arrives via
    /// [`Self::preview_dispatch_release`], with Moves in between so
    /// drags (slider knobs) work. No-op (false) when not in preview or
    /// the press is outside the canvas region.
    pub fn preview_dispatch_press(
        &mut self,
        screen_x: f32,
        screen_y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        use jian_core::gesture::pointer::PointerPhase;
        let Some(doc) = self.preview_doc_point(screen_x, screen_y, viewport_w, viewport_h) else {
            return false;
        };
        self.preview_press_active = true;
        self.preview_last_doc = Some((doc.x, doc.y));
        let handled = self
            .preview
            .as_mut()
            .is_some_and(|p| p.dispatch_pointer_phase(doc.x, doc.y, PointerPhase::Down));
        self.mark_dirty();
        handled
    }

    /// Route a cursor move into the live preview runtime — a drag
    /// (`Move`) while the preview press is held, a `Hover` otherwise.
    /// Returns `true` (move consumed) only when the point is on-canvas
    /// AND not over a floating overlay panel, so top-bar hover and
    /// floating-panel hover still work while previewing. (Other
    /// floating chrome — e.g. the chat panel — is already
    /// non-interactive in preview: `press.rs` swallows all non-topbar
    /// presses, so preview owning its hover is consistent.)
    pub fn preview_dispatch_move(&mut self, screen_x: f32, screen_y: f32) -> bool {
        use jian_core::gesture::pointer::PointerPhase;
        let (vw, vh) = (self.last_viewport_w, self.last_viewport_h);
        if self.over_topmost_panel(screen_x, screen_y, vw, vh) {
            return false;
        }
        let Some(doc) = self.preview_doc_point(screen_x, screen_y, vw, vh) else {
            return false;
        };
        let phase = if self.preview_press_active {
            PointerPhase::Move
        } else {
            PointerPhase::Hover
        };
        self.preview_last_doc = Some((doc.x, doc.y));
        let emitted = self
            .preview
            .as_mut()
            .is_some_and(|p| p.dispatch_pointer_phase(doc.x, doc.y, phase));
        if emitted || self.preview_press_active {
            self.mark_dirty();
        }
        true
    }

    /// Complete a preview drag: pointer Up at the last known document
    /// point. Returns `true` when a preview press was in flight (the
    /// release is consumed).
    pub fn preview_dispatch_release(&mut self) -> bool {
        use jian_core::gesture::pointer::PointerPhase;
        if !self.preview_press_active {
            return false;
        }
        self.preview_press_active = false;
        if let Some((x, y)) = self.preview_last_doc {
            if let Some(p) = self.preview.as_mut() {
                p.dispatch_pointer_phase(x, y, PointerPhase::Up);
            }
        }
        self.mark_dirty();
        true
    }

    /// Route a wheel into the preview runtime; `false` (not consumed —
    /// no `onScroll` node under the cursor) lets the caller fall back
    /// to canvas pan/zoom so the user can still navigate while
    /// previewing.
    pub fn preview_dispatch_wheel(
        &mut self,
        screen_x: f32,
        screen_y: f32,
        delta_y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        let Some(doc) = self.preview_doc_point(screen_x, screen_y, viewport_w, viewport_h) else {
            return false;
        };
        let consumed = self
            .preview
            .as_mut()
            .is_some_and(|p| p.dispatch_wheel(doc.x, doc.y, 0.0, delta_y));
        if consumed {
            self.mark_dirty();
        }
        consumed
    }

    /// Advance focus to the next (`shift=false`) / previous
    /// (`shift=true`) focusable widget in the live preview runtime —
    /// Tab / Shift+Tab. Lets the user reach a text input without
    /// clicking (the desktop runner otherwise drops Tab as a control
    /// char). Returns `true` while in preview so the caret repaints.
    pub fn preview_focus(&mut self, shift: bool) -> bool {
        let Some(preview) = self.preview.as_mut() else {
            return false;
        };
        if shift {
            preview.focus_previous();
        } else {
            preview.focus_next();
        }
        self.mark_dirty();
        true
    }

    /// Route a named key into the live preview runtime. `shift` drives
    /// Shift+Tab focus traversal + selection. Returns `true` when the
    /// runtime emitted any semantic event.
    pub fn preview_dispatch_key(&mut self, key: &str, shift: bool) -> bool {
        use jian_core::gesture::pointer::Modifiers;
        let mods = if shift {
            Modifiers::SHIFT
        } else {
            Modifiers::empty()
        };
        let handled = self
            .preview
            .as_mut()
            .is_some_and(|p| p.dispatch_key(key, mods));
        // Tab traversal / text edits change focus or content even when
        // no semantic event fires, so always repaint while in preview.
        if self.preview.is_some() {
            self.mark_dirty();
        }
        handled
    }

    /// Refresh host-level theme tokens from the canonical editor UI
    /// state. Most widgets derive their theme directly, but a few
    /// paint-layer affordances still read this host cache.
    pub(in crate::widget_host) fn sync_theme_from_editor(&mut self) {
        self.theme =
            op_editor_ui::widgets::editor_state_ext::theme_for(&self.editor_state.editor_ui);
    }

    /// Run a path boolean op on the active selection (Union /
    /// Subtract / Intersect / Exclude). Backed by skia's `Path::op`.
    /// Returns true when the op committed (≥ 2 Path nodes were
    /// selected + the result yielded a non-empty polyline).
    pub fn apply_boolean_op(&mut self, op: op_editor_core::BooleanOp) -> bool {
        // Codex stop-gate: boolean op shortcuts (Cmd+Alt+U/S/I/X)
        // mutate the document — commit any pending variable-row
        // edit first so the dirty draft lands before this op runs.
        self.commit_variable_row_focus_if_any();
        // The skia `Path::op` math runs against the layout-resolved
        // `LayoutScene` + the editor selection; the result polyline
        // is committed back through an `EditorState` mutator so the
        // host never edits the canonical tree directly.
        self.refresh_layout_scene();
        let selected: Vec<String> = self
            .editor_state
            .selection
            .set
            .iter()
            .map(|id| id.as_str().to_string())
            .collect();
        let outcome = crate::boolean_ops::compute_boolean_op(&self.layout_scene, &selected, op);
        let Some(result) = outcome else {
            return false;
        };
        // Scene ids are the canonical `.op` ids — wrap straight into
        // `op_editor_core::NodeId`.
        let source_ids: Vec<op_editor_core::NodeId> = result
            .source_ids
            .iter()
            .map(op_editor_core::NodeId::new)
            .collect();
        let pre = self.editor_state.snapshot_for_history();
        let new_id = self.editor_state.replace_paths_with_polyline(
            &source_ids,
            &result.contours,
            &mut self.next_node_id,
        );
        match new_id {
            Some(id) => {
                self.editor_state.history_push_past(pre);
                self.editor_state.set_single_selection(id);
                self.mark_dirty();
                true
            }
            None => false,
        }
    }

    /// Rebuild the layout-resolved `LayoutScene` from `editor_state`
    /// if `editor_state_dirty` is set; clear the flag. Cheap no-op
    /// when the scene is already current. The input hit-test + the
    /// paint pass both call this before reading `layout_scene`.
    pub(in crate::widget_host) fn refresh_layout_scene(&mut self) {
        if self.editor_state_dirty {
            // Only re-derive when the scene inputs (doc / theme / active page)
            // actually changed — most `editor_state_dirty` marks (hover, scroll,
            // selection, caret drafts, chat streaming) leave them identical, and
            // the scene carries no editor state, so the rebuild would be a no-op.
            if let Some(scene) = self.scene_cache.maybe_rebuild(&self.editor_state) {
                self.layout_scene = scene;
            }
            self.editor_state_dirty = false;
        }
    }

    /// The layout-resolved render scene for the live `EditorState`.
    /// Rebuilt on demand when the state changed since the last
    /// derive. The `CanvasViewport` paint + the host's canvas
    /// hit-test both read through this.
    pub fn layout_scene(&mut self) -> &op_editor_ui::layout_scene::LayoutScene {
        self.refresh_layout_scene();
        &self.layout_scene
    }

    /// Mark `editor_state` as mutated so the next `refresh_layout_scene()`
    /// re-derives the render scene. Call after any direct mutation of
    /// `self.editor_state`.
    pub(in crate::widget_host) fn mark_dirty(&mut self) {
        self.editor_state_dirty = true;
    }

    /// Test-only: flag the render scene stale after a test mutated
    /// `editor_state` directly through `editor_state_mut()`.
    #[cfg(test)]
    pub(in crate::widget_host) fn mark_paint_dirty_for_test(&mut self) {
        self.editor_state_dirty = true;
    }

    /// Borrow the canonical-model editor state — the host's single
    /// source of truth.
    pub fn editor_state(&self) -> &op_editor_core::EditorState {
        &self.editor_state
    }

    /// Mutable borrow of the canonical-model editor state. Callers
    /// that mutate through this MUST call `mark_editor_state_dirty()`
    /// afterwards, else the paint snapshot goes stale.
    pub fn editor_state_mut(&mut self) -> &mut op_editor_core::EditorState {
        &mut self.editor_state
    }

    /// Public dirty-flag — desktop-side code that mutates
    /// `editor_state` through `editor_state_mut()` (settings I/O,
    /// `.op` load, chat streaming, model discovery) calls this so the
    /// next paint re-derives the snapshot.
    pub fn mark_editor_state_dirty(&mut self) {
        self.editor_state_dirty = true;
    }

    /// Install a Figma-imported editor state. The worker only parses
    /// into canonical data; layout scene construction stays on the
    /// normal host path so the worker never touches Skia / FontMgr.
    pub fn install_imported_state(&mut self, mut state: op_editor_core::EditorState) {
        let mut preserved = self.editor_state.editor_ui.clone();
        preserved.figma_import_in_progress = false;
        preserved.file_name_display = state.editor_ui.file_name_display.take();
        preserved.preserve_authored_geometry = state.editor_ui.preserve_authored_geometry;
        // The imported document replaces the previous one, so an in-flight
        // clone wizard belongs to a document that no longer exists — drop
        // it. The host's `poll_git_clone_job` then abandons the job (it
        // only binds while a `cloning` form is live). Without this the
        // clone could bind a repo onto the freshly-imported untitled
        // document, which the path-based origin check can't catch (both
        // documents are untitled → the same `None` path).
        preserved.git_panel.clone_form = None;
        state.editor_ui = preserved;

        let old_state = std::mem::replace(&mut self.editor_state, state);
        let old_scene = std::mem::take(&mut self.layout_scene);
        std::thread::Builder::new()
            .name("op-import-drop".into())
            .spawn(move || {
                drop(old_state);
                drop(old_scene);
            })
            .expect("spawn op-import-drop worker");

        // The scene was just taken (left empty) and is rebuilt lazily on the next
        // `refresh_layout_scene`. Invalidate the build cache so that rebuild is
        // NOT skipped even if the imported document happens to match the last
        // build's inputs — otherwise the canvas would stay blank.
        self.scene_cache.invalidate();
        self.editor_state_dirty = true;
    }

    /// Drain a queued Component-Browser insert: place the chosen
    /// UIKit component at the viewport's centre (top-left = centre −
    /// half the component's size) and call
    /// [`EditorState::instantiate_kit_component`]. Returns `true`
    /// when an instantiate landed (the desktop runner schedules a
    /// repaint on `true`).
    pub fn drain_component_browser_insert(&mut self, viewport_w: f32, viewport_h: f32) -> bool {
        let Some((kit_id, comp_id)) = self
            .editor_state
            .editor_ui
            .component_browser_pending_insert
            .take()
        else {
            return false;
        };
        let dims = self
            .editor_state
            .ui_kits
            .iter()
            .find(|k| k.id == kit_id)
            .and_then(|k| k.components.iter().find(|c| c.id == comp_id))
            .map(|c| (c.width as f64, c.height as f64));
        let Some((cw_comp, ch_comp)) = dims else {
            return false;
        };
        let (_cx0, _cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        let canvas_local = op_editor_ui::Point2D::new(cw / 2.0, ch / 2.0);
        let doc = self.editor_state.viewport.to_document(canvas_local);
        let dx = doc.x as f64 - cw_comp / 2.0;
        let dy = doc.y as f64 - ch_comp / 2.0;
        if self
            .editor_state
            .instantiate_kit_component(&kit_id, &comp_id, dx, dy)
            .is_some()
        {
            self.mark_dirty();
            true
        } else {
            false
        }
    }

    /// Insert nodes parsed from the Figma clipboard, centred on the
    /// viewport, with fresh ids, batched undo, and the pasted roots
    /// selected — mirrors TS `use-figma-paste.ts:67-100`.
    pub fn paste_figma_nodes(
        &mut self,
        nodes: Vec<jian_ops_schema::node::PenNode>,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        use op_editor_core::PenNodeExt;
        if nodes.is_empty() {
            return false;
        }
        // Union of the incoming roots' own bounds — the paste centres
        // this box on the canvas viewport centre.
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for node in &nodes {
            let b = op_editor_core::own_bounds(node);
            min_x = min_x.min(b.x);
            min_y = min_y.min(b.y);
            max_x = max_x.max(b.x + b.w);
            max_y = max_y.max(b.y + b.h);
        }
        if min_x > max_x {
            min_x = 0.0;
            min_y = 0.0;
            max_x = 0.0;
            max_y = 0.0;
        }
        let (_cx0, _cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        let canvas_local = op_editor_ui::Point2D::new(cw / 2.0, ch / 2.0);
        let centre = self.editor_state.viewport.to_document(canvas_local);
        let dx = centre.x as f64 - (min_x + max_x) / 2.0;
        let dy = centre.y as f64 - (min_y + max_y) / 2.0;

        let snap = self.editor_state.snapshot_for_history();
        let mut taken = self.editor_state.collect_node_ids();
        let mut new_ids = Vec::with_capacity(nodes.len());
        for node in &nodes {
            let mut clone = op_editor_core::walkers::deep_clone_with_new_ids(
                node,
                &mut self.next_node_id,
                &mut taken,
            );
            op_editor_core::walkers::translate_subtree(&mut clone, dx, dy);
            new_ids.push(op_editor_core::NodeId::new(clone.base().id.clone()));
            self.editor_state.active_children_mut().push(clone);
        }
        if let Some(anchor) = new_ids.first().cloned() {
            self.editor_state.set_single_selection(anchor);
            for id in new_ids.into_iter().skip(1) {
                self.editor_state.toggle_selection(id);
            }
        }
        self.editor_state.history_push_past(snap);
        self.mark_dirty();
        true
    }

    /// Commit any in-progress settings-modal input draft (currently
    /// the MCP port). Used by the desktop runner before persisting
    /// settings on quick-quit so a focused-but-uncommitted port edit
    /// isn't silently dropped.
    pub fn flush_settings_input(&mut self) {
        self.commit_settings_focus_if_any();
    }

    /// Whether the chat input is focused — runner uses this to
    /// decide whether to schedule a periodic wake-up for caret
    /// blink.
    pub fn chat_focused(&self) -> bool {
        self.editor_state.chat.focused
    }

    /// Next millisecond at which the host should wake to repaint
    /// the caret blink phase. `None` = no animation pending.
    pub fn next_animation_deadline_ms(&self) -> Option<u64> {
        let mut next = op_editor_core::agent_indicators::next_reveal_deadline_ms(self.now_ms);
        // While previewing, keep the loop ticking (~30 fps) so the live
        // runtime's caret blink + any time-driven widget state animates.
        if self.preview.is_some() {
            let deadline = self.now_ms.saturating_add(33);
            next = Some(next.map_or(deadline, |current| current.min(deadline)));
        }
        if let Some(input) = self.editor_state.active_text_input() {
            let deadline = input.next_blink_flip_ms(self.now_ms);
            next = Some(next.map_or(deadline, |current| current.min(deadline)));
        }
        // While a `git clone` runs, keep the loop ticking so
        // `poll_git_clone_job` drains the worker's result later.
        if let Some(form) = &self.editor_state.editor_ui.git_panel.clone_form {
            if form.cloning {
                let deadline = self.now_ms.saturating_add(100);
                next = Some(next.map_or(deadline, |current| current.min(deadline)));
            }
        }
        next
    }
}

impl Default for WidgetHostNative {
    fn default() -> Self {
        Self::new()
    }
}
