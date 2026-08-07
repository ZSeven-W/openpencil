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

use op_editor_core::PreviewDeviceKind;
use op_editor_ui::widgets::SelectionHandle;
use op_editor_ui::{Rect, Theme};

mod a11y;
mod account_press;
#[cfg(test)]
mod account_press_tests;
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
#[cfg(test)]
mod canvas_drag_transition_tests;
mod canvas_pan_cache;
mod canvas_scene_patch;
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
#[cfg(test)]
mod codegen_framework_tests;
mod color_picker_press;
mod component_browser_press;
#[cfg(test)]
mod deferred_press_tests;
mod design_md_press;
#[cfg(test)]
mod design_md_press_tests;
#[cfg(test)]
mod document_epoch_tests;
mod figma_import_scroll;
#[cfg(test)]
mod figma_import_tests;
#[cfg(test)]
mod font_generation_scene_tests;
mod font_picker_dispatch;
#[cfg(test)]
mod font_picker_keyboard_tests;
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
mod image_crop_drag;
#[cfg(test)]
mod image_crop_drag_tests;
mod image_panel_dispatch;
#[cfg(test)]
mod image_panel_overlay_tests;
mod image_panel_selection;
#[cfg(test)]
mod image_panel_selection_tests;
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
mod missing_fonts_dispatch;
mod mode_transition_host;
#[cfg(test)]
mod overlay_cursor_tests;
mod overlay_rects;
#[cfg(test)]
mod page_switch_center_tests;
mod paint;
#[cfg(test)]
mod pan_cache_tests;
#[cfg(test)]
mod panel_history_tests;
mod pen_press;
#[cfg(test)]
mod pen_press_tests;
mod press;
mod press_helpers;
mod preview_edge_swipe;
#[cfg(all(test, not(target_os = "windows")))]
mod preview_edge_swipe_tests;
mod preview_frame;
#[cfg(test)]
mod preview_frame_geometry_tests;
#[cfg(all(test, not(target_os = "windows")))]
mod preview_frame_tests;
#[cfg(test)]
mod property_compositing_tests;
mod property_dispatch;
mod property_layout_dispatch;
#[cfg(test)]
mod property_panel_interactions_tests;
#[cfg(test)]
mod property_panel_press_tests;
mod property_popovers;
mod release_feedback;
mod screen_switcher;
#[cfg(all(test, not(target_os = "windows")))]
mod screen_switcher_tests;
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
// Windows CI DirectWrite/Skia text layout aborts inside these text-edit
// fixtures before Rust can report an assertion. macOS + Linux keep coverage.
#[cfg(all(test, not(target_os = "windows")))]
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
    /// Paint-only interpolation from the previous layout scene to the
    /// current one. Used by canvas node drag/reorder previews.
    pub(in crate::widget_host) layout_transition:
        Option<op_editor_ui::widgets::CanvasLayoutTransition>,
    /// Skips the `layout_scene` rebuild when the document / active theme /
    /// active page are unchanged. `editor_state_dirty` fires on nearly every
    /// interaction (hover, scroll, selection, caret drafts, chat streaming),
    /// but most leave the scene inputs identical — this guards the rebuild.
    pub(in crate::widget_host) scene_cache: op_pen_loader::SceneBuildCache,
    /// Set whenever `editor_state` is mutated. Drives the lazy
    /// rebuild of `layout_scene` — `refresh_layout_scene()` rebuilds
    /// + clears the flag, so a sequence of mutations re-derives once.
    pub(in crate::widget_host) editor_state_dirty: bool,
    /// Monotonic counter bumped every time the WHOLE `editor_state` is
    /// replaced (Open / New / import) — never on an in-place edit or a
    /// save. Async work captured against a document (e.g. a clipboard
    /// paste decode on a worker thread) reads this at dispatch and
    /// re-checks it before applying its result, so a result decoded
    /// for a document that has since been replaced is dropped instead
    /// of landing in the wrong document.
    pub(in crate::widget_host) document_epoch: u64,
    /// The `jian_skia` font-registry generation the current `layout_scene`
    /// was built against. A runtime font import/removal bumps that
    /// generation WITHOUT dirtying `editor_state`, so `refresh_layout_scene`
    /// watches this to force a rebuild — otherwise an already-open document
    /// keeps its stale fallback-font layout until an unrelated dirty event.
    pub(in crate::widget_host) layout_scene_font_generation: u64,
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
    /// Active bitmap pan while the selected image fill is in crop edit mode.
    pub(in crate::widget_host) image_crop_drag: Option<image_crop_drag::ImageCropDragState>,
    pub(in crate::widget_host) effect_radius_drag: Option<usize>,
    /// Active generated-code preview text selection drag.
    pub(in crate::widget_host) code_selection_drag: Option<CodeSelectionDragState>,
    /// Active chat input text selection drag.
    pub(in crate::widget_host) chat_input_selection_drag: Option<ChatInputSelectionDragState>,
    /// Active Search / Generate popover input selection drag.
    pub(in crate::widget_host) image_input_selection_drag:
        Option<image_panel_selection::ImageInputSelectionDragState>,
    /// Latest Search / Generate input geometry measured by the real painter.
    pub(in crate::widget_host) image_input_geometry:
        Option<op_editor_ui::widgets::property_panel_image_assets::ImagePopoverInputGeometry>,
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
    /// Per-gesture container geometry/index used by canvas drop hit-testing.
    /// Built lazily and discarded whenever a live tree mutation reflows the
    /// scene, avoiding repeated whole-document DFS work on cursor frames.
    pub(in crate::widget_host) canvas_drop_index: Option<canvas_select_drag::CanvasDropIndex>,
    /// Original selected ids for an active Option-drag clone move.
    /// Drop hit-testing skips these so a fresh clone does not
    /// immediately reparent back into the source it overlaps.
    pub(in crate::widget_host) option_drag_source_ids: Vec<op_editor_core::NodeId>,
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
    /// Millisecond deadline until which a pan/zoom gesture counts as
    /// live. While `now_ms` is before it, the canvas paints in
    /// interactive-degrade mode (effect layers + sub-pixel leaves
    /// skip); the animation scheduler wakes at the deadline so the
    /// gesture-end frame repaints at full quality.
    pub(in crate::widget_host) interaction_hot_until_ms: u64,
    /// Offscreen canvas layer serving pure-pan frames during a live
    /// gesture. See `widget_host/canvas_pan_cache.rs`.
    pub(in crate::widget_host) pan_cache: Option<canvas_pan_cache::CanvasPanCache>,
    /// Progressive gesture-end quality restore over `pan_cache`.
    pub(in crate::widget_host) pan_cache_restore: Option<canvas_pan_cache::PanCacheRestore>,
    /// Frames served by a pan-cache blit (test observability).
    pub(in crate::widget_host) pan_cache_blits: u64,
    /// In-place scroll refreshes performed (test observability).
    pub(in crate::widget_host) pan_cache_scrolls: u64,
    /// Full expanded-layer builds performed (test observability).
    pub(in crate::widget_host) pan_cache_builds: u64,
    /// Whether the most recent gesture tick was a zoom — zoom frames
    /// never build the pan cache (each tick would invalidate it).
    pub(in crate::widget_host) last_gesture_was_zoom: bool,
    /// Whether the shift key is currently held. Runners update
    /// this via `set_modifier_shift` on every modifier change.
    /// Drives shift+click multi-select in `apply_press`.
    pub(in crate::widget_host) shift_held: bool,
    /// Whether Alt/Option is currently held. Node dragging uses this
    /// to duplicate the current selection before moving it.
    pub(in crate::widget_host) alt_held: bool,
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
    pub(in crate::widget_host) preview_device_frame: Option<preview_frame::DeviceFrame>,
    pub(in crate::widget_host) preview_scroll_y: f32,
    pub(in crate::widget_host) preview_manual_pick: Option<PreviewDeviceKind>,
    pub(in crate::widget_host) preview_surface_capture: Option<preview_frame::PreviewSurface>,
    /// Track M-1: an in-flight canvas ↔ device-frame merge animation —
    /// `Some(Enter)` for the brief window right after `enter_preview`
    /// installed `self.preview`; `Some(Exit)` for the window between
    /// `exit_preview` being CALLED and the runtime actually dropping
    /// (see `exit_preview`'s doc — the drop is deferred to
    /// `settle_mode_transition` so the merge-back animation has
    /// something live to paint). `None` the rest of the time.
    pub(in crate::widget_host) preview_mode_transition: Option<crate::preview::ModeTransition>,
    /// Live preview pointer-drag state: `true` between a canvas Down
    /// and its Up, so cursor moves dispatch as drags (slider knob)
    /// instead of hovers.
    pub(in crate::widget_host) preview_press_active: bool,
    /// Last preview pointer position in DOCUMENT space — the release
    /// dispatches its Up here (the OS reports release without coords).
    pub(in crate::widget_host) preview_last_doc: Option<(f32, f32)>,
    /// Track C-4: the SCREEN-space x a preview press started at, when
    /// that press began within the edge-swipe dead zone (device-frame
    /// content-local x < 24px) — the iOS-style "swipe from the left
    /// edge to go back" candidate. `None` when no candidate is being
    /// tracked (press started elsewhere, or preview isn't in App Mode /
    /// there's nowhere to pop to).
    pub(in crate::widget_host) preview_edge_swipe_start_x: Option<f32>,
    /// Stable, process-unique id scoping this host's chat-panel transcript
    /// cache. Allocated once at construction and stamped onto every
    /// `AIChatPlaceholder` this host builds (`.owned_by`), so the display-frame
    /// cursor-shape hint (`hit_test_current_build`) only reads a build THIS
    /// panel resolved — never one a different panel left in the thread-local
    /// slot after a tab/host switch.
    pub(in crate::widget_host) chat_panel_owner: u64,
    /// Stable, process-unique id scoping this host's LayerPanel row-model
    /// cache. Stamped onto the per-frame paint build (`from_editor_owned`)
    /// so the thread-local slot is owned by this panel and never
    /// cross-served to another host on a revision-counter collision. Page
    /// switches don't rotate (`active_page_index` is in the key), but
    /// whole-document replacements do — see
    /// `force_rotate_layer_panel_owner` (revision counters restart at 0,
    /// so the key alone can alias across documents).
    pub(in crate::widget_host) layer_panel_owner: u64,
    /// Active chat-session (tab) index observed at the last owner rotation.
    /// When the active session changes, [`Self::rotate_chat_owner_if_session_changed`]
    /// rotates `chat_panel_owner` so the new tab's transcript never reads the
    /// previous tab's cached geometry via the 0-hash cursor-shape hint (it reads
    /// `None` — the default arrow — until the new tab's first paint re-stores the
    /// slot under the rotated owner).
    pub(in crate::widget_host) last_chat_session_index: usize,
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
    /// Cursor-following resolved bounds for same-container flex
    /// drags. Paint hides the in-flow selected node and draws a
    /// floating copy at these bounds.
    pub(in crate::widget_host) overlay_bounds: Option<Rect>,
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
    /// Authored parent-relative origin at press time. Kept separate from the
    /// resolved absolute scene bounds so left/top drags of nested free nodes
    /// do not jump into document coordinates.
    pub(in crate::widget_host) start_authored_x: Option<f64>,
    pub(in crate::widget_host) start_authored_y: Option<f64>,
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
        // Read the font generation BEFORE building the initial scene. An
        // import landing during construction then leaves this stale-low, so
        // the next `refresh_layout_scene` rebuilds — reading it AFTER the
        // build would record the new generation against the pre-import scene
        // and skip the rebuild until an unrelated dirty event (same race we
        // fixed in `FontResolver` / `SkiaMeasure`).
        let layout_scene_font_generation = jian_skia::font_generation();
        // Seed the render scene once up front; subsequent frames
        // re-derive only when `editor_state_dirty` is set.
        let layout_scene = op_pen_loader::editor_state_to_active_page_layout_scene(&editor_state);
        let last_chat_session_index = editor_state.chat.active_index();
        Self {
            editor_state,
            layout_scene,
            layout_transition: None,
            scene_cache: op_pen_loader::SceneBuildCache::new(),
            editor_state_dirty: false,
            document_epoch: 0,
            layout_scene_font_generation,
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
            image_crop_drag: None,
            effect_radius_drag: None,
            code_selection_drag: None,
            chat_input_selection_drag: None,
            image_input_selection_drag: None,
            image_input_geometry: None,
            chat_text_selection_drag: None,
            text_edit_selection_drag: None,
            text_measure: None,
            panel_resize: None,
            variables_resize: None,
            node_drag: None,
            canvas_drop_index: None,
            option_drag_source_ids: Vec::new(),
            path_anchor_drag: None,
            arc_handle_drag: None,
            handle_drag: None,
            rotate_drag: None,
            create_drag: None,
            marquee_drag: None,
            layer_drag: None,
            next_node_id: 100,
            now_ms: 0,
            interaction_hot_until_ms: 0,
            pan_cache: None,
            pan_cache_restore: None,
            pan_cache_blits: 0,
            pan_cache_scrolls: 0,
            pan_cache_builds: 0,
            last_gesture_was_zoom: false,
            shift_held: false,
            alt_held: false,
            last_viewport_w: 0.0,
            last_viewport_h: 0.0,
            preview: None,
            preview_device_frame: None,
            preview_scroll_y: 0.0,
            preview_manual_pick: None,
            preview_surface_capture: None,
            preview_mode_transition: None,
            preview_press_active: false,
            preview_last_doc: None,
            preview_edge_swipe_start_x: None,
            chat_panel_owner: op_editor_ui::widgets::AIChatPlaceholder::next_owner(),
            layer_panel_owner: op_editor_ui::widgets::LayerPanel::next_layer_panel_owner(),
            last_chat_session_index,
        }
    }

    /// Rotate the chat-panel transcript-cache owner when the active chat session
    /// (tab) changed since the last call. A fresh owner means the new tab's
    /// display-frame cursor hint reads `None` (the slot still belongs to the old
    /// owner) until this tab's next paint re-resolves and re-stamps it — the
    /// documented one-frame isolation. Called at the top of the paint / probe
    /// entry points so the very next resolve stores under the rotated owner.
    pub(in crate::widget_host) fn rotate_chat_owner_if_session_changed(&mut self) {
        let active = self.editor_state.chat.active_index();
        if active != self.last_chat_session_index {
            self.last_chat_session_index = active;
            self.chat_panel_owner = op_editor_ui::widgets::AIChatPlaceholder::next_owner();
        }
    }

    /// Force a chat-panel transcript-cache owner rotation NOW, unconditionally —
    /// even when `chat.active_index()` is unchanged. Called synchronously at each
    /// host session-mutation site (tab switch / new tab). A tab switch changes the
    /// active session but a `CursorMoved` can arrive before the next paint and run
    /// the event-time cursor-shape hint (`geometry::cursor_hint` →
    /// `hit_test_current_build`), which would otherwise pair the previous session's
    /// cached geometry with the new session's live messages. Rotating here means
    /// that hint reads `None` (default arrow) until the new session's first paint
    /// re-stamps the slot. Rotating unconditionally also covers same-index session
    /// replacement — closing active tab 0 installs the next session at index 0,
    /// closing the sole tab replaces it in place — which the index-only poll in
    /// [`Self::rotate_chat_owner_if_session_changed`] misses.
    ///
    /// Public because some session mutations run outside this crate: the desktop
    /// runner's ⌘T `new_chat_tab` and tab-close `close_chat_tab` mutate
    /// `chat` directly and must rotate synchronously for the same reason.
    pub fn force_rotate_chat_owner(&mut self) {
        self.chat_panel_owner = op_editor_ui::widgets::AIChatPlaceholder::next_owner();
        self.last_chat_session_index = self.editor_state.chat.active_index();
    }

    /// Force a LayerPanel row-model-cache owner rotation NOW. The cache key is
    /// `(document_revision, active_page_index, collapsed_fp, rename_fp)`, and a
    /// freshly loaded / imported / MCP-replaced document restarts its revision
    /// at 0 and its active page index at 0 — so a WHOLE-DOCUMENT replacement
    /// leaves the key byte-identical to the previous document's, while the
    /// owner never rotates on its own. The paint path would then serve the
    /// PREVIOUS document's cached rows indefinitely. Rotating the owner at every
    /// replacement seam makes the next owned paint resolve miss the stale slot
    /// and rebuild against the new document. (Page/tab switches WITHIN a live
    /// document need no rotation — they change `active_page_index`, which is in
    /// the key.)
    ///
    /// Public because some replacement seams run outside this crate: the desktop
    /// runner replaces `editor_state` on Open / New (`persistence.rs`) and MCP
    /// `ReplaceDocument` (`mcp_runtime.rs`) and must rotate synchronously.
    pub fn force_rotate_layer_panel_owner(&mut self) {
        self.layer_panel_owner = op_editor_ui::widgets::LayerPanel::next_layer_panel_owner();
    }

    /// Build the Layer panel through this host's owner-scoped row cache.
    ///
    /// Paint and event-time hit tests must share this path: page/layer presses,
    /// hover, scrolling, and accessibility otherwise rebuild and reallocate the
    /// whole active-page row model independently on large documents.
    pub(in crate::widget_host) fn layer_panel(&self) -> op_editor_ui::widgets::LayerPanel {
        op_editor_ui::widgets::LayerPanel::from_editor_owned(
            &self.editor_state,
            self.layer_panel_owner,
        )
    }

    /// Push the host's current shift-key state. Runners call this
    /// on every modifier-change event so `apply_press` can branch
    /// on shift+click semantics.
    pub fn set_modifier_shift(&mut self, held: bool) {
        self.shift_held = held;
    }

    pub fn set_modifier_alt(&mut self, held: bool) {
        self.alt_held = held;
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

    /// Center the canvas viewport on a scene-space `rect`, keeping zoom
    /// unchanged. Used by APP MODE preview to keep the mounted screen
    /// framed on entry and after a screen-switch reconcile. `viewport_w`
    /// / `viewport_h` are the values the paint path already threads
    /// (the host has no cached viewport-size field of its own —
    /// `last_viewport_w/h` is refreshed only on press, so it can be
    /// stale for a frame at enter time).
    fn center_canvas_on(&mut self, rect: op_editor_ui::Rect, viewport_w: f32, viewport_h: f32) {
        let (_cx0, _cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        let vp = &mut self.editor_state.viewport;
        vp.pan_x = cw / 2.0 - (rect.origin.x + rect.size.x / 2.0) * vp.zoom;
        vp.pan_y = ch / 2.0 - (rect.origin.y + rect.size.y / 2.0) * vp.zoom;
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

    /// Track C-6 (`Cmd+P`): [`Self::toggle_preview`] using the host's own
    /// cached viewport (`last_viewport_w/h`) instead of a caller-supplied
    /// size — the keyboard shortcut's entry point (`op-host-desktop`,
    /// which has no access to this crate's private canvas-region math)
    /// is otherwise identical to the TopBar Play button's `press.rs` call
    /// site. `last_viewport_w/h` can be a frame stale right after a
    /// resize (same caveat `center_canvas_on`'s doc already accepts for
    /// this cache), which only affects entry centering, never whether
    /// preview toggles.
    pub fn toggle_preview_with_cached_viewport(&mut self) -> bool {
        let (_cx0, _cy0, cw, ch) = self.canvas_region(self.last_viewport_w, self.last_viewport_h);
        self.toggle_preview((cw, ch))
    }

    /// Resize hook called from the desktop runner's `Resized` handler.
    /// Preview layout is now derived per-root from the document (not the
    /// canvas region), so resizing only changes the paint transform, not
    /// the flex solve — `PreviewSession::resize` is itself a no-op.
    /// Returns early when not in preview.
    pub fn preview_resize(&mut self, viewport_w: f32, viewport_h: f32) {
        self.cache_preview_viewport(viewport_w, viewport_h);
        if self.preview.is_none() {
            return;
        }
        let (_cx0, _cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        if let Some(preview) = self.preview.as_mut() {
            preview.resize((cw, ch));
        }
        self.recompute_device_frame(viewport_w, viewport_h);
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
        if self.device_mode_active() {
            // Device mode fails closed: a missing frame must never fall
            // through to the editor viewport inverse.
            return self.device_preview_doc_point(screen_x, screen_y);
        }
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
        // Track M-1: the canvas/device-frame rect is physically moving
        // mid-merge — same "discard, don't queue" call
        // `PreviewSession::transition_active` makes for a screen-switch
        // slide, for the same reason (no stable target to land on).
        if self.mode_transition_active() {
            return false;
        }
        let Some(doc) = self.preview_doc_point(screen_x, screen_y, viewport_w, viewport_h) else {
            return false;
        };
        self.capture_device_preview_surface(screen_x, screen_y);
        self.preview_press_active = true;
        self.preview_last_doc = Some((doc.x, doc.y));
        // Track C-4: arm an edge-swipe candidate when this press started
        // in the device frame's left-edge dead zone. Forwarding Down to
        // the runtime as usual is deliberate: a genuine edge-swipe moves
        // well past the tap-gesture's own same-spot Down/Up tolerance
        // before the pop fires, so it never completes as a stray tap.
        self.arm_edge_swipe_candidate(screen_x);
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
        // Track C-4: a held drag that crosses the edge-swipe threshold
        // fires `pop` and cancels the underlying gesture instead of
        // completing as a normal Move — checked BEFORE updating
        // `preview_last_doc` so the cancel dispatches at the gesture's
        // last real position, not this frame's.
        if self.preview_press_active && self.maybe_fire_edge_swipe(screen_x) {
            self.cancel_preview_gesture_for_edge_swipe();
            self.mark_dirty();
            return true;
        }
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
        self.preview_surface_capture = None;
        self.disarm_edge_swipe();
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

    /// Route a wheel / trackpad-pan scroll into the preview runtime;
    /// `false` (not consumed — no `onScroll` node under the cursor)
    /// lets the caller fall back to canvas pan/zoom so the user can
    /// still navigate while previewing. Mouse wheels carry only
    /// `delta_y`; two-finger trackpad pans carry both axes.
    pub fn preview_dispatch_wheel(
        &mut self,
        screen_x: f32,
        screen_y: f32,
        delta_x: f32,
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
            .is_some_and(|p| p.dispatch_wheel(doc.x, doc.y, delta_x, delta_y));
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
        // A runtime font import/removal advances the font-registry generation
        // without dirtying `editor_state`, so watch it directly — otherwise
        // the early-out below skips the rebuild and the open document keeps
        // its stale fallback-font layout. Reuses the same generation the
        // scene cache measures against, so the two stay consistent.
        let font_generation = jian_skia::font_generation();
        let font_changed = font_generation != self.layout_scene_font_generation;
        if self.editor_state_dirty || font_changed {
            let active_page_index = self
                .editor_state
                .ui
                .active_page_index
                .min(self.editor_state.page_count().saturating_sub(1));
            let active_page_changed = active_page_index != self.layout_scene.active_page_index;
            if active_page_changed {
                // A page switch builds a disjoint render tree. Release the
                // previous transition + scene before the loader allocates the
                // new payload and scene so both page trees do not overlap at
                // the switch's peak. Same-page document/font rebuilds retain
                // the old scene until the replacement is ready.
                self.layout_transition = None;
                drop(std::mem::take(&mut self.layout_scene));
            }
            // Only re-derive when the scene inputs (doc / theme / active page /
            // font generation) actually changed — most `editor_state_dirty`
            // marks (hover, scroll, selection, caret drafts, chat streaming)
            // leave them identical, and the scene carries no editor state, so
            // the rebuild would be a no-op.
            if let Some(scene) = self.scene_cache.maybe_rebuild(&self.editor_state) {
                self.layout_scene = scene;
                // A rebuilt scene invalidates the pan bitmap cache
                // (covers the font-generation path, which bypasses
                // `mark_dirty`).
                self.drop_pan_cache();
            }
            self.editor_state_dirty = false;
            self.layout_scene_font_generation = font_generation;
        }
    }

    pub(in crate::widget_host) fn start_layout_transition_from_scene(
        &mut self,
        before: op_editor_ui::layout_scene::LayoutScene,
    ) {
        self.refresh_layout_scene();
        self.layout_transition = op_editor_ui::widgets::CanvasLayoutTransition::between(
            &before,
            &self.layout_scene,
            self.now_ms,
        );
    }

    pub(in crate::widget_host) fn start_layout_transition_from_scene_excluding(
        &mut self,
        before: op_editor_ui::layout_scene::LayoutScene,
        excluded_id: &op_editor_core::NodeId,
    ) {
        self.refresh_layout_scene();
        self.layout_transition = op_editor_ui::widgets::CanvasLayoutTransition::between_excluding(
            &before,
            &self.layout_scene,
            self.now_ms,
            Some(excluded_id.as_str()),
        );
    }

    pub(in crate::widget_host) fn start_layout_transition_from_bounds(
        &mut self,
        node_id: &op_editor_core::NodeId,
        bounds: Rect,
    ) {
        let mut starts = std::collections::HashMap::new();
        starts.insert(node_id.as_str().to_string(), bounds);
        self.layout_transition =
            op_editor_ui::widgets::CanvasLayoutTransition::from_start_bounds(starts, self.now_ms);
    }

    /// The layout-resolved render scene for the live `EditorState`.
    /// Rebuilt on demand when the state changed since the last
    /// derive. The `CanvasViewport` paint + the host's canvas
    /// hit-test both read through this.
    pub fn layout_scene(&mut self) -> &op_editor_ui::layout_scene::LayoutScene {
        self.refresh_layout_scene();
        &self.layout_scene
    }

    /// Borrow the canonical state mutably together with the scene snapshot
    /// resolved immediately before that borrow. Background enrichers may use
    /// it to derive one coherent pre-mutation validation view; it must not be
    /// treated as current after a state mutation, and callers must mark the
    /// host dirty when they mutate the state.
    pub fn editor_state_mut_and_layout_scene(
        &mut self,
    ) -> (
        &mut op_editor_core::EditorState,
        &op_editor_ui::layout_scene::LayoutScene,
    ) {
        self.refresh_layout_scene();
        (&mut self.editor_state, &self.layout_scene)
    }

    /// Mark `editor_state` as mutated so the next `refresh_layout_scene()`
    /// re-derives the render scene. Call after any direct mutation of
    /// `self.editor_state`.
    pub(in crate::widget_host) fn mark_dirty(&mut self) {
        self.editor_state_dirty = true;
        // A mutated document / UI invalidates the pan bitmap cache —
        // a blitted frame must never show stale content.
        self.drop_pan_cache();
    }

    /// Test-only: flag the render scene stale after a test mutated
    /// `editor_state` directly through `editor_state_mut()`.
    #[cfg(test)]
    pub(in crate::widget_host) fn mark_paint_dirty_for_test(&mut self) {
        self.editor_state_dirty = true;
        self.drop_pan_cache();
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

    /// The current document epoch — bumped on every whole-document
    /// replacement (Open / New / import), never on save or in-place
    /// edit. Async work captures this at dispatch and re-checks it
    /// before applying, so a result decoded for a since-replaced
    /// document is dropped. See [`Self::document_epoch`] field docs.
    pub fn document_epoch(&self) -> u64 {
        self.document_epoch
    }

    /// Replace the whole editor state (Open / New) and bump the
    /// document epoch. Use this instead of assigning through
    /// `editor_state_mut()` whenever a fresh document supersedes the
    /// current one, so epoch-guarded async work can detect the swap.
    /// (`install_imported_state` is the import-specific analogue and
    /// bumps the epoch itself.)
    pub fn replace_editor_state(&mut self, state: op_editor_core::EditorState) {
        self.editor_state = state;
        self.document_epoch = self.document_epoch.wrapping_add(1);
        self.scene_cache.invalidate();
        self.editor_state_dirty = true;
    }

    /// Install a Figma-imported editor state. The worker only parses
    /// into canonical data; layout scene construction stays on the
    /// normal host path so the worker never touches Skia / FontMgr.
    pub fn install_imported_state(&mut self, state: op_editor_core::EditorState) {
        self.install_imported_state_with_drop_hook(state, || {});
    }

    /// Import-specific replacement with a callback that runs after the old
    /// state and scene finish dropping on the background worker. Desktop uses
    /// this to schedule allocator pressure relief at the correct lifetime
    /// boundary without blocking the UI thread; other native callers keep the
    /// no-op callback above.
    pub fn install_imported_state_with_drop_hook<F>(
        &mut self,
        mut state: op_editor_core::EditorState,
        after_drop: F,
    ) where
        F: FnOnce() + Send + 'static,
    {
        let imported_document_dirty = state.editor_ui.document_dirty;
        let mut preserved = self.editor_state.editor_ui.clone();
        preserved.figma_import_in_progress = false;
        preserved.file_name_display = state.editor_ui.file_name_display.take();
        preserved.preserve_authored_geometry = state.editor_ui.preserve_authored_geometry;
        // Dirty/saved state belongs to the incoming document. The rest of the
        // live shell UI is intentionally retained, but inheriting this flag
        // from the replaced editor would make a saved import appear dirty (or
        // an unsaved import appear clean).
        preserved.document_dirty = imported_document_dirty;
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
        // Whole-document replacement — bump the epoch so any async
        // work captured against the previous document (e.g. a pending
        // clipboard paste decode) is dropped instead of applied here.
        self.document_epoch = self.document_epoch.wrapping_add(1);
        let old_scene = std::mem::take(&mut self.layout_scene);
        std::thread::Builder::new()
            .name("op-import-drop".into())
            .spawn(move || {
                drop(old_state);
                drop(old_scene);
                after_drop();
            })
            .expect("spawn op-import-drop worker");

        // The imported document restarts at revision 0 / page 0, so its
        // LayerPanel row-model-cache key aliases the replaced document's.
        // Rotate the owner here — the single funnel for the Figma-import path
        // (figma_import_session) — so the next owned paint resolve rebuilds
        // instead of serving the previous document's cached rows.
        self.force_rotate_layer_panel_owner();

        // The scene was just taken (left empty) and is rebuilt lazily on the next
        // `refresh_layout_scene`. Invalidate the build cache so that rebuild is
        // NOT skipped even if the imported document happens to match the last
        // build's inputs — otherwise the canvas would stay blank.
        self.scene_cache.invalidate();
        self.editor_state_dirty = true;
        self.arm_missing_fonts_detection();
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
        self.refresh_missing_fonts_after_document_change();
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

    /// Record a live canvas pan gesture tick: the canvas paints
    /// interactive-degraded until `INTERACTION_HOT_MS` after the
    /// last tick, then the scheduler-driven repaint restores quality.
    pub(in crate::widget_host) fn note_viewport_gesture(&mut self) {
        self.interaction_hot_until_ms = self.now_ms.saturating_add(INTERACTION_HOT_MS);
        self.last_gesture_was_zoom = false;
    }

    /// Record a live canvas ZOOM gesture tick. Same degrade window as
    /// a pan, but the pan bitmap cache must NOT rebuild per tick — the
    /// zoom invalidates it every frame, so building (2× a plain frame)
    /// would be pure loss; zoom frames paint direct in degrade mode.
    pub(in crate::widget_host) fn note_viewport_zoom_gesture(&mut self) {
        self.interaction_hot_until_ms = self.now_ms.saturating_add(INTERACTION_HOT_MS);
        self.last_gesture_was_zoom = true;
    }

    /// Whether the current frame should paint in interactive-degrade
    /// mode (a pan/zoom gesture ticked within the hot window).
    pub(in crate::widget_host) fn fast_interaction_active(&self) -> bool {
        self.now_ms < self.interaction_hot_until_ms
    }

    /// Canvas-only low-cost paint mode for direct manipulation. Unlike
    /// `fast_interaction_active`, this does not make the pan bitmap cache
    /// eligible: edited geometry changes on every frame.
    pub(in crate::widget_host) fn canvas_fast_interaction_active(&self) -> bool {
        self.fast_interaction_active()
            || self.node_drag.as_ref().is_some_and(|drag| drag.moved)
            || self.handle_drag.is_some()
            || self.rotate_drag.is_some()
            || self.create_drag.is_some()
    }

    /// Next millisecond at which the host should wake to repaint
    /// the caret blink phase. `None` = no animation pending.
    pub fn next_animation_deadline_ms(&self) -> Option<u64> {
        let mut next = op_editor_core::agent_indicators::next_reveal_deadline_ms(self.now_ms);
        // Gesture-end full-quality repaint: wake once the
        // interactive-degrade window closes. Quantized UP to a 50 ms
        // grid so consecutive gesture ticks report the SAME deadline —
        // the desktop runner's waker dedups identical instants, and a
        // per-tick sliding deadline re-armed the OS timer every frame.
        if self.fast_interaction_active() {
            let deadline = self.interaction_hot_until_ms.div_ceil(50) * 50;
            next = Some(next.map_or(deadline, |current| current.min(deadline)));
        }
        // Progressive quality restore: one tile per frame until the
        // visible region is sharp again.
        if self.pan_cache_restore.is_some() {
            let deadline = self.now_ms.saturating_add(16);
            next = Some(next.map_or(deadline, |current| current.min(deadline)));
        }
        if let Some(deadline) =
            op_editor_core::agent_indicators::next_generation_scan_deadline_ms(self.now_ms)
        {
            next = Some(next.map_or(deadline, |current| current.min(deadline)));
        }
        if let Some(transition) = self.layout_transition.as_ref() {
            if let Some(deadline) = transition.next_deadline_ms(self.now_ms) {
                next = Some(next.map_or(deadline, |current| current.min(deadline)));
            }
        }
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

/// How long after the last pan/zoom tick the canvas keeps painting in
/// interactive-degrade mode. Long enough to cover trackpad event gaps,
/// short enough that full quality returns imperceptibly after release.
const INTERACTION_HOT_MS: u64 = 150;

impl Default for WidgetHostNative {
    fn default() -> Self {
        Self::new()
    }
}
